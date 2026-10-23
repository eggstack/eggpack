//! The build workflow renderer and the rendering helpers it shares with the release renderers.

use crate::drift::MAX_WORKFLOW_BYTES;
use crate::error::{fail, CiError};
use crate::planning::CIPlan;
use crate::policy::{
    validate_sha256_hex, zig_archive_name, zig_download_url, zig_expected_digest,
    CrossToolProvisioningV1, GitHubPolicy, WorkflowTrigger,
};
use eggpack_core::{BuildStrategy, PlannedTarget};

pub(crate) fn contains_unsafe_dispatch_expression_in_run(workflow: &str) -> bool {
    let lines: Vec<&str> = workflow.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let indentation = line.len() - line.trim_start().len();
        let Some(script) = line.trim_start().strip_prefix("run:") else {
            continue;
        };
        let is_block = matches!(script.trim(), "|" | ">" | "|-" | ">-");
        if script.contains("${{ inputs.release_tag }}")
            || script.contains("${{ github.event.inputs.release_tag }}")
        {
            return true;
        }
        if is_block {
            for body_line in lines.iter().skip(index + 1) {
                let body_indentation = body_line.len() - body_line.trim_start().len();
                if !body_line.trim().is_empty() && body_indentation <= indentation {
                    break;
                }
                if body_line.contains("${{ inputs.release_tag }}")
                    || body_line.contains("${{ github.event.inputs.release_tag }}")
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Exact cross-tool versions required by a CargoZigbuild target policy.
pub(crate) fn cross_tool_versions(planned: &PlannedTarget) -> Result<(&str, &str), CiError> {
    let cargo_zigbuild = planned
        .policy
        .toolchain
        .cargo_zigbuild
        .as_deref()
        .ok_or_else(|| fail("cargo-zigbuild version missing from ReleasePlan"))?;
    let zig = planned
        .policy
        .toolchain
        .zig
        .as_deref()
        .ok_or_else(|| fail("Zig version missing from ReleasePlan"))?;
    Ok((cargo_zigbuild, zig))
}

/// Render the `PreinstalledVerified` cross-tool check: both exact configured
/// versions are verified, but nothing is installed.
pub(crate) fn verified_cross_tools_step(planned: &PlannedTarget) -> Result<String, CiError> {
    let (cargo_zigbuild, zig) = cross_tool_versions(planned)?;
    let mut step = String::from(
        "      - name: Verify Rust and cross tools\n        shell: bash\n        run: |\n          cargo +",
    );
    step.push_str(&shell_quote(&planned.policy.toolchain.rust));
    step.push_str(" --version\n          actual=\"$(cargo zigbuild --version)\"\n          test \"$actual\" = ");
    step.push_str(&shell_quote(&format!("cargo-zigbuild {cargo_zigbuild}")));
    step.push_str("\n          actual_zig=\"$(zig version)\"\n          test \"$actual_zig\" = ");
    step.push_str(&shell_quote(zig));
    step.push('\n');
    Ok(step)
}

/// Render deterministic `Provisioned` cross-tool setup for a CargoZigbuild job.
///
/// Order: install exact cargo-zigbuild into an invocation-private
/// `CARGO_INSTALL_ROOT` (only its `bin` joins PATH, exact version required),
/// then download the official Zig archive over HTTPS with bounded timeouts,
/// verify the checked-in SHA-256 before extraction, validate the single
/// top-level layout, extract into a private directory, require a regular
/// non-symlink executable, bind it via `CARGO_ZIGBUILD_ZIG_PATH`, isolate
/// `CARGO_ZIGBUILD_CACHE_DIR`, and require exact Zig version equality.
/// No package manager, mirror, arbitrary URL, or arbitrary command exists.
pub(crate) fn provision_cross_tools_steps(
    provisioning: &CrossToolProvisioningV1,
    planned: &PlannedTarget,
) -> Result<String, CiError> {
    let (cargo_zigbuild, zig_version) = cross_tool_versions(planned)?;
    let host_os = planned.policy.host_os;
    let host_arch = planned.policy.host_arch;
    let archive = zig_archive_name(host_os, host_arch, zig_version)?;
    let url = zig_download_url(zig_version, host_arch)?;
    let digest = zig_expected_digest(provisioning, host_os, host_arch)?;
    validate_sha256_hex(digest)?;
    let mut step = String::from("      - name: Install exact cargo-zigbuild\n        shell: bash\n        timeout-minutes: ");
    step.push_str(&provisioning.cargo_install_timeout_minutes.to_string());
    step.push_str("\n        run: |\n          set -euo pipefail\n          install_root=\"${{ runner.temp }}/eggpack/cargo-install/${{ github.run_id }}-${{ github.run_attempt }}\"\n          mkdir -p \"$install_root\"\n          CARGO_INSTALL_ROOT=\"$install_root\" cargo install cargo-zigbuild --version ");
    step.push_str(&shell_quote(cargo_zigbuild));
    step.push_str(" --locked\n          echo \"CARGO_INSTALL_ROOT=$install_root\" >> \"$GITHUB_ENV\"\n          echo \"$install_root/bin\" >> \"$GITHUB_PATH\"\n          export PATH=\"$install_root/bin:$PATH\"\n          test -x \"$install_root/bin/cargo-zigbuild\"\n");
    step.push_str("      - name: Provision verified Zig ");
    step.push_str(zig_version);
    step.push_str("\n        shell: bash\n        run: |\n          set -euo pipefail\n          zig_version=");
    step.push_str(&shell_quote(zig_version));
    step.push_str("\n          zig_archive=");
    step.push_str(&shell_quote(&archive));
    step.push_str("\n          zig_url=");
    step.push_str(&shell_quote(&url));
    step.push_str("\n          zig_sha=");
    step.push_str(&shell_quote(digest));
    step.push_str("\n          work=\"${{ runner.temp }}/eggpack/zig/${{ github.run_id }}-${{ github.run_attempt }}\"\n          mkdir -p \"$work\"\n          archive=\"$work/$zig_archive\"\n          curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location --proto-redir '=https' --connect-timeout ");
    step.push_str(&provisioning.zig_connect_timeout_secs.to_string());
    step.push_str(" --max-time ");
    step.push_str(&provisioning.zig_max_time_secs.to_string());
    step.push_str(" -o \"$archive\" \"$zig_url\"\n          printf '%s  %s\\n' \"$zig_sha\" \"$archive\" | sha256sum --check -\n          entries=\"$(tar -tf \"$archive\")\"\n          test -n \"$entries\"\n          if printf '%s\\n' \"$entries\" | grep -Eq '(^/)|(^|/)\\.\\.(/|$)'; then echo 'rejected Zig archive path' >&2; exit 1; fi\n          top=\"$(printf '%s\\n' \"$entries\" | cut -d/ -f1 | sort -u)\"\n          test -n \"$top\"\n          test \"$(printf '%s\\n' \"$top\" | wc -l)\" = \"1\"\n          printf '%s\\n' \"$entries\" | grep -Fxq \"$top/zig\"\n          rm -rf \"$work/extract\"\n          mkdir -p \"$work/extract\"\n          tar -xf \"$archive\" -C \"$work/extract\"\n          test ! -L \"$work/extract/$top/zig\"\n          test -f \"$work/extract/$top/zig\"\n          test -s \"$work/extract/$top/zig\"\n          test -x \"$work/extract/$top/zig\"\n          zig_bin=\"$work/extract/$top/zig\"\n          test \"$(\"$zig_bin\" version)\" = ");
    step.push_str(&shell_quote(zig_version));
    step.push_str("\n          cache=\"${{ runner.temp }}/eggpack/zigbuild-cache/${{ github.run_id }}-${{ github.run_attempt }}\"\n          mkdir -p \"$cache\"\n          echo \"CARGO_ZIGBUILD_ZIG_PATH=$zig_bin\" >> \"$GITHUB_ENV\"\n          echo \"CARGO_ZIGBUILD_CACHE_DIR=$cache\" >> \"$GITHUB_ENV\"\n          echo \"$work/extract/$top\" >> \"$GITHUB_PATH\"\n");
    Ok(step)
}

/// Extra `env` lines binding the product build to the verified Zig path.
///
/// `CARGO_ZIGBUILD_ZIG_PATH` is the build authority; PATH only carries the
/// verified directory for direct diagnostics. The cache directory is
/// invocation-private scratch state, never a shared correctness cache.
pub(crate) fn provisioned_build_env() -> &'static str {
    "\n          CARGO_ZIGBUILD_ZIG_PATH: ${{ env.CARGO_ZIGBUILD_ZIG_PATH }}\n          CARGO_ZIGBUILD_CACHE_DIR: ${{ env.CARGO_ZIGBUILD_CACHE_DIR }}"
}

/// Render the cross-tool section for one CargoZigbuild job: either the
/// `PreinstalledVerified` exact check or the `Provisioned` install sequence.
pub(crate) fn cross_tools_section(
    policy: &GitHubPolicy,
    planned: &PlannedTarget,
) -> Result<String, CiError> {
    match &policy.cross_tools {
        None => verified_cross_tools_step(planned),
        Some(provisioning) => provision_cross_tools_steps(provisioning, planned),
    }
}

/// Deterministically render a read-only GitHub Actions build workflow.
pub fn render_github(plan: &CIPlan, policy: &GitHubPolicy) -> Result<String, CiError> {
    policy.validate(plan)?;
    let mut out = String::from("name: Eggpack candidate builds\n'on':\n");
    for trigger in &policy.triggers {
        match trigger {
            WorkflowTrigger::Push => out.push_str("  push:\n"),
            WorkflowTrigger::WorkflowDispatch => out.push_str("  workflow_dispatch:\n"),
        }
    }
    out.push_str("permissions:\n  contents: read\nconcurrency:\n  group: eggpack-${{ github.workflow }}-${{ github.ref }}\n  cancel-in-progress: ");
    out.push_str(if policy.cancel_in_progress {
        "true\n"
    } else {
        "false\n"
    });
    out.push_str("jobs:\n  preflight:\n    runs-on: ");
    out.push_str(&yaml_scalar(&policy.preflight_runner));
    out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n      - name: Check out source\n        uses: ");
    out.push_str(&yaml_scalar(&policy.checkout.reference));
    out.push_str("\n      - name: Check Cargo availability\n        shell: bash\n        run: cargo --version\n");

    for job in &plan.targets {
        let runner = policy
            .runners
            .iter()
            .find(|mapping| {
                mapping.os == job.planned.policy.host_os
                    && mapping.arch == job.planned.policy.host_arch
            })
            .ok_or_else(|| fail("GitHub runner mapping missing for build host"))?;
        out.push_str("  ");
        out.push_str(&job.job_id);
        out.push_str(":\n    needs: preflight\n    runs-on: ");
        out.push_str(&yaml_scalar(&runner.label));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    continue-on-error: ");
        out.push_str(if job.required { "false\n" } else { "true\n" });
        out.push_str("    steps:\n      - name: Check out source\n        uses: ");
        out.push_str(&yaml_scalar(&policy.checkout.reference));
        out.push_str("\n      - name: Set up Rust toolchain\n        uses: ");
        out.push_str(&yaml_scalar(&policy.rust_toolchain.reference));
        out.push_str("\n        with:\n          toolchain: ");
        out.push_str(&yaml_scalar(&job.planned.policy.toolchain.rust));
        out.push_str("\n          targets: ");
        out.push_str(&yaml_scalar(&job.planned.target));
        out.push('\n');
        match job.planned.policy.strategy {
            BuildStrategy::NativeCargo => {
                out.push_str("      - name: Verify Rust toolchain\n        shell: bash\n        run: cargo +");
                out.push_str(&shell_quote(&job.planned.policy.toolchain.rust));
                out.push_str(" --version\n");
            }
            BuildStrategy::CargoZigbuild => {
                out.push_str(&cross_tools_section(policy, &job.planned)?);
            }
        }
        let provisioned = policy.cross_tools.is_some()
            && job.planned.policy.strategy == BuildStrategy::CargoZigbuild;
        for output in &job.outputs {
            out.push_str("      - name: Build ");
            out.push_str(&yaml_scalar(&format!(
                "{} / {}",
                job.planned.target, output.binary
            )));
            out.push_str("\n        shell: bash\n        env:\n          CARGO_TARGET_DIR: \"${{ runner.temp }}/eggpack/${{ github.run_id }}-${{ github.run_attempt }}\"");
            if provisioned {
                out.push_str(provisioned_build_env());
            }
            out.push_str("\n        run: ");
            let mut command = Vec::with_capacity(output.args.len() + 1);
            command.push(output.executable.as_str());
            command.extend(output.args.iter().map(String::as_str));
            out.push_str(&yaml_scalar(
                &command
                    .iter()
                    .map(|arg| shell_quote(arg))
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
            out.push('\n');
            let mut path = format!(
                "${{{{ runner.temp }}}}/eggpack/${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}/{}/release/{}",
                job.planned.target, output.binary
            );
            if job.planned.target.contains("-windows-") {
                path.push_str(".exe");
            }
            out.push_str("      - name: Hand off internal candidate bytes\n        uses: ");
            out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&output.handoff_name));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&path));
            out.push_str("\n          if-no-files-found: error\n          retention-days: ");
            out.push_str(&policy.artifact_retention_days.to_string());
            out.push('\n');
        }
    }
    if out.len() > MAX_WORKFLOW_BYTES {
        return Err(fail("rendered workflow exceeds size bound"));
    }
    Ok(out)
}

pub(crate) fn yaml_scalar(value: &str) -> String {
    serde_json::to_string(value).expect("string JSON serialization cannot fail")
}

pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
