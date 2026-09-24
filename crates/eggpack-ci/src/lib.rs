#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Provider-neutral CI planning and deterministic GitHub Actions generation.

use eggpack_contract::DistributionContract;
use eggpack_core::{
    cargo_command, BuildBindingsV1, BuildStrategy, CompatibilityFloor, HostArch, HostOs,
    HostRequirement, LogicalOutputSelector, PlannedAssetForm, PlannedTarget, Qualification,
    ReleasePlan, SupportTier,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt, path::Path, time::Duration};

const MAX_CI_PLAN_JSON: usize = 1_000_000;
const MAX_OUTPUTS: usize = 4_096;
const MAX_WORKFLOW_BYTES: usize = 8_000_000;

/// Bounded failure from CI planning, policy validation, rendering, or drift checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiError(String);
impl fmt::Display for CiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CiError {}
fn fail(message: &str) -> CiError {
    CiError(message.to_owned())
}

/// Provider-neutral schema-v1 release workflow graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CIPlan {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Opaque release id copied from ReleasePlan.
    pub release_id: String,
    /// Opaque source revision copied from ReleasePlan.
    pub source_revision: String,
    /// Required target build jobs in canonical target order.
    pub targets: Vec<TargetJob>,
    /// Required build job IDs that later aggregation must depend on.
    pub required_aggregation_dependencies: Vec<String>,
}

/// One provider-neutral target build node and its unresolved qualification intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetJob {
    /// Original resolved target policy and contract-derived form.
    pub planned: PlannedTarget,
    /// Stable provider-neutral job identifier.
    pub job_id: String,
    /// Whether this build is required to gate later aggregation.
    pub required: bool,
    /// Qualification remains intent and is never reported as passed by M001.
    pub qualification: QualificationIntent,
    /// Explicit Cargo build outputs mapped to contract logical slots.
    pub outputs: Vec<BuildOutput>,
}

/// Qualification intent retained without claiming execution or success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationIntent {
    /// Planned classification.
    pub classification: Qualification,
    /// Optional separately configured qualification host.
    pub host: Option<HostRequirement>,
    /// M001 never executes qualification.
    pub state: QualificationState,
}

/// Qualification execution state represented by M001.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationState {
    /// Execution is deferred to a later milestone.
    Unresolved,
}

/// Explicit logical candidate output with Cargo source and M002-derived command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildOutput {
    /// Contract logical output selector.
    pub selector: LogicalOutputSelector,
    /// Explicit Cargo package source.
    pub package: String,
    /// Explicit Cargo binary target source.
    pub binary: String,
    /// Deterministic internal artifact handoff name.
    pub handoff_name: String,
    /// Shell-free executable name from M002 command construction.
    pub executable: String,
    /// Ordered arguments from M002 command construction.
    pub args: Vec<String>,
}

/// Project a contract-validated ReleasePlan and M002 bindings to a provider-neutral graph.
pub fn project_ci_plan(
    contract: &DistributionContract,
    release: &ReleasePlan,
    bindings: &BuildBindingsV1,
) -> Result<CIPlan, CiError> {
    validate_release(release)?;
    let canonical_config = eggpack_core::PackConfig {
        schema_version: 1,
        targets: release
            .targets
            .iter()
            .map(|target| target.policy.clone())
            .collect(),
    };
    let selected: Vec<_> = release
        .targets
        .iter()
        .map(|target| target.target.clone())
        .collect();
    let resolved = canonical_config
        .resolve(
            contract,
            &release.release_id,
            &release.source_revision,
            &selected,
        )
        .map_err(|_| fail("ReleasePlan does not resolve canonically against its contract"))?;
    if resolved != *release {
        return Err(fail(
            "ReleasePlan differs from canonical contract resolution",
        ));
    }
    bindings
        .validate_for(contract, release)
        .map_err(|_| fail("build bindings do not match the resolved release plan and contract"))?;

    let mut targets = Vec::with_capacity(release.targets.len());
    let mut job_ids = BTreeSet::new();
    let mut dependencies = Vec::new();
    for planned in &release.targets {
        let job_id = target_job_id(&planned.target);
        if !job_ids.insert(job_id.clone()) {
            return Err(fail("canonical target job identifier collision"));
        }
        let required = planned.policy.support == SupportTier::Required;
        if required {
            dependencies.push(job_id.clone());
        }
        let target_bindings = bindings
            .targets
            .get(&planned.target)
            .ok_or_else(|| fail("target build bindings are missing"))?;
        let mut outputs = Vec::with_capacity(target_bindings.len());
        for binding in target_bindings {
            let command = cargo_command(
                planned,
                binding,
                Path::new("$GITHUB_WORKSPACE"),
                Path::new("$CARGO_TARGET_DIR"),
                Duration::from_secs(30 * 60),
            )
            .map_err(|_| fail("M002 rejected target Cargo command intent"))?;
            let handoff_name = handoff_name(&planned.target, &binding.selector);
            outputs.push(BuildOutput {
                selector: binding.selector.clone(),
                package: binding.package.clone(),
                binary: binding.binary.clone(),
                handoff_name,
                executable: command.executable,
                args: command.args,
            });
        }
        outputs.sort_by(|a, b| a.selector.cmp(&b.selector));
        targets.push(TargetJob {
            planned: planned.clone(),
            job_id,
            required,
            qualification: QualificationIntent {
                classification: planned.policy.qualification,
                host: planned.policy.qualification_host,
                state: QualificationState::Unresolved,
            },
            outputs,
        });
    }
    targets.sort_by(|a, b| a.planned.target.cmp(&b.planned.target));
    dependencies.sort();
    let graph = CIPlan {
        schema_version: 1,
        release_id: release.release_id.clone(),
        source_revision: release.source_revision.clone(),
        targets,
        required_aggregation_dependencies: dependencies,
    };
    graph.validate()?;
    Ok(graph)
}

fn validate_release(release: &ReleasePlan) -> Result<(), CiError> {
    if release.schema_version != 1
        || !safe_metadata(&release.release_id, 256)
        || !safe_metadata(&release.source_revision, 256)
        || release.targets.is_empty()
        || release.targets.len() > 256
    {
        return Err(fail("invalid or out-of-bounds ReleasePlan"));
    }
    let mut previous: Option<&str> = None;
    let mut targets = BTreeSet::new();
    for target in &release.targets {
        if !safe_target(&target.target)
            || !targets.insert(target.target.as_str())
            || previous.is_some_and(|p| p >= target.target.as_str())
            || target.policy.target != target.target
        {
            return Err(fail(
                "ReleasePlan targets must be unique, canonical, and ordered",
            ));
        }
        previous = Some(&target.target);
    }
    Ok(())
}

impl CIPlan {
    /// Serialize a validated graph as compact deterministic JSON.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("CIPlan serialization failed"))
    }

    /// Parse and validate a bounded schema-v1 graph document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_CI_PLAN_JSON {
            return Err(fail("CIPlan document exceeds size bound"));
        }
        let plan: Self = serde_json::from_str(text).map_err(|_| fail("invalid CIPlan JSON"))?;
        plan.validate()?;
        Ok(plan)
    }

    /// Validate graph shape, canonical ordering, and M002-derived build commands.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1
            || !safe_metadata(&self.release_id, 256)
            || !safe_metadata(&self.source_revision, 256)
            || self.targets.is_empty()
            || self.targets.len() > 256
        {
            return Err(fail("invalid or out-of-bounds CIPlan"));
        }
        let mut job_ids = BTreeSet::new();
        let mut handoffs = BTreeSet::new();
        let mut dependencies = Vec::new();
        let mut prior: Option<&str> = None;
        let mut output_count = 0usize;
        for job in &self.targets {
            let target = &job.planned;
            let policy = &target.policy;
            let zig_version_valid = policy
                .toolchain
                .cargo_zigbuild
                .as_deref()
                .is_some_and(safe_tool_version);
            let toolchain_valid = safe_tool_version(&policy.toolchain.rust)
                && ((policy.strategy == BuildStrategy::CargoZigbuild && zig_version_valid)
                    || (policy.strategy == BuildStrategy::NativeCargo
                        && policy.toolchain.cargo_zigbuild.is_none()));
            let floor_valid = match policy.floor {
                CompatibilityFloor::None => true,
                CompatibilityFloor::Glibc { .. } => target.target.contains("-linux-gnu"),
                CompatibilityFloor::Macos { .. } => target.target.ends_with("-apple-darwin"),
            };
            let native_qualification_valid = if policy.qualification == Qualification::Native {
                policy.strategy == BuildStrategy::NativeCargo
                    && host_matches_target(
                        policy.qualification_host.unwrap_or(HostRequirement {
                            os: policy.host_os,
                            arch: policy.host_arch,
                        }),
                        &target.target,
                    )
            } else {
                true
            };
            if !safe_target(&target.target)
                || target.policy.target != target.target
                || !toolchain_valid
                || !floor_valid
                || !native_qualification_valid
                || (policy.strategy == BuildStrategy::CargoZigbuild
                    && matches!(policy.floor, CompatibilityFloor::Macos { .. }))
                || prior.is_some_and(|p| p >= target.target.as_str())
                || job.job_id != target_job_id(&target.target)
                || !job_ids.insert(job.job_id.as_str())
                || job.outputs.is_empty()
                || job.outputs.len() > 256
                || job.required != (target.policy.support == SupportTier::Required)
                || job.qualification.classification != target.policy.qualification
                || job.qualification.host != target.policy.qualification_host
                || job.qualification.state != QualificationState::Unresolved
            {
                return Err(fail("invalid, unordered, or inconsistent target job"));
            }
            prior = Some(&target.target);
            output_count = output_count
                .checked_add(job.outputs.len())
                .ok_or_else(|| fail("CIPlan output count overflow"))?;
            if output_count > MAX_OUTPUTS {
                return Err(fail("CIPlan exceeds output count bound"));
            }
            if job.required {
                dependencies.push(job.job_id.clone());
            }
            let mut previous_selector: Option<&LogicalOutputSelector> = None;
            for output in &job.outputs {
                if previous_selector.is_some_and(|p| p >= &output.selector)
                    || !safe_identifier(&output.package)
                    || !safe_identifier(&output.binary)
                    || !selector_matches_form(&output.selector, target.artifact_form)
                    || output.handoff_name != handoff_name(&target.target, &output.selector)
                    || !handoffs.insert(output.handoff_name.as_str())
                {
                    return Err(fail("invalid or duplicate logical build output"));
                }
                previous_selector = Some(&output.selector);
                let binding = eggpack_core::BuildBinding {
                    selector: output.selector.clone(),
                    package: output.package.clone(),
                    binary: output.binary.clone(),
                };
                let expected = cargo_command(
                    target,
                    &binding,
                    Path::new("$GITHUB_WORKSPACE"),
                    Path::new("$CARGO_TARGET_DIR"),
                    Duration::from_secs(30 * 60),
                )
                .map_err(|_| fail("M002 rejected target Cargo command intent"))?;
                if output.executable != expected.executable || output.args != expected.args {
                    return Err(fail("build command differs from M002 command semantics"));
                }
            }
        }
        dependencies.sort();
        if self.required_aggregation_dependencies != dependencies {
            return Err(fail(
                "required aggregation dependencies do not match target gating",
            ));
        }
        Ok(())
    }
}

fn safe_target(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn safe_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn safe_metadata(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
fn safe_tool_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".+-_".contains(&b))
}
fn host_matches_target(host: HostRequirement, target: &str) -> bool {
    let arch_matches = match host.arch {
        HostArch::X86_64 => target.starts_with("x86_64-"),
        HostArch::Aarch64 => target.starts_with("aarch64-"),
        HostArch::Armv7 => target.starts_with("armv7-"),
    };
    let os_matches = match host.os {
        HostOs::Linux => target.contains("-linux-"),
        HostOs::Macos => target.ends_with("-apple-darwin"),
        HostOs::Windows => target.contains("-windows-"),
    };
    arch_matches && os_matches
}
fn selector_matches_form(selector: &LogicalOutputSelector, form: PlannedAssetForm) -> bool {
    match (selector, form) {
        (LogicalOutputSelector::Direct, PlannedAssetForm::Direct)
        | (LogicalOutputSelector::BundleEntry { .. }, PlannedAssetForm::Bundle) => true,
        (LogicalOutputSelector::ArchiveMember { source }, PlannedAssetForm::Archive) => {
            !source.is_empty()
                && source.len() <= 255
                && !source.starts_with('/')
                && source
                    .split('/')
                    .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
        }
        _ => false,
    }
}
fn target_job_id(target: &str) -> String {
    let mut id = String::from("build_");
    for b in target.bytes() {
        if b.is_ascii_alphanumeric() {
            id.push((b as char).to_ascii_lowercase());
        } else {
            id.push('_');
        }
    }
    id
}
fn handoff_name(target: &str, selector: &LogicalOutputSelector) -> String {
    let slot = match selector {
        LogicalOutputSelector::Direct => "direct".to_owned(),
        LogicalOutputSelector::BundleEntry { index } => format!("bundle-{index}"),
        LogicalOutputSelector::ArchiveMember { source } => {
            let mut digest = Sha256::new();
            digest.update(source.as_bytes());
            let hex: String = digest
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            format!("archive-{hex}")
        }
    };
    format!("eggpack-{target}-{slot}")
}

/// A full immutable GitHub action commit reference for one fixed action repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionPin {
    /// Full `owner/repository@40-hex-sha` reference.
    pub reference: String,
}

/// One provider policy runner label for a provider-neutral host capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerMapping {
    /// Required host operating system family.
    pub os: HostOs,
    /// Required host architecture family.
    pub arch: HostArch,
    /// GitHub runner label selected by provider policy.
    pub label: String,
    /// The selected runner image has a preinstalled cargo-zigbuild executable.
    pub cargo_zigbuild: bool,
    /// The selected runner image has a preinstalled Zig executable.
    pub zig: bool,
}

/// Finite workflow trigger supported by the M001 renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTrigger {
    /// Generate on pushes.
    Push,
    /// Allow deliberate manual generation.
    WorkflowDispatch,
}

/// Bounded GitHub Actions policy supplied separately from CIPlan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubPolicy {
    /// Runner label used for lightweight graph preflight.
    pub preflight_runner: String,
    /// Exact host capability to GitHub label mappings.
    pub runners: Vec<RunnerMapping>,
    /// Immutable pin for `actions/checkout`.
    pub checkout: ActionPin,
    /// Immutable pin for `dtolnay/rust-toolchain`.
    pub rust_toolchain: ActionPin,
    /// Immutable pin for `actions/upload-artifact`.
    pub upload_artifact: ActionPin,
    /// Finite workflow event set.
    pub triggers: Vec<WorkflowTrigger>,
    /// Per-job timeout in minutes, from 1 to 360.
    pub timeout_minutes: u16,
    /// Whether a newer ref run cancels an in-progress run in the same group.
    pub cancel_in_progress: bool,
    /// Internal candidate artifact retention in days, from 1 to 90.
    pub artifact_retention_days: u8,
}

impl GitHubPolicy {
    /// Validate action identity, immutable SHA syntax, runner uniqueness, and bounds.
    pub fn validate(&self, plan: &CIPlan) -> Result<(), CiError> {
        plan.validate()?;
        if !safe_runner_label(&self.preflight_runner)
            || self.timeout_minutes == 0
            || self.timeout_minutes > 360
            || self.artifact_retention_days == 0
            || self.artifact_retention_days > 90
            || self.triggers.is_empty()
            || self.triggers.len() > 2
            || self.runners.len() > 128
        {
            return Err(fail("invalid or out-of-bounds GitHub policy"));
        }
        validate_pin(&self.checkout, "actions/checkout")?;
        validate_pin(&self.rust_toolchain, "dtolnay/rust-toolchain")?;
        validate_pin(&self.upload_artifact, "actions/upload-artifact")?;
        let mut hosts = BTreeSet::new();
        for mapping in &self.runners {
            if !safe_runner_label(&mapping.label)
                || !hosts.insert((os_code(mapping.os), arch_code(mapping.arch)))
            {
                return Err(fail("invalid or duplicate GitHub runner mapping"));
            }
        }
        let mut triggers = BTreeSet::new();
        for trigger in &self.triggers {
            let code = match trigger {
                WorkflowTrigger::Push => 0,
                WorkflowTrigger::WorkflowDispatch => 1,
            };
            if !triggers.insert(code) {
                return Err(fail("duplicate workflow trigger"));
            }
        }
        for job in &plan.targets {
            let host = &job.planned.policy;
            let mapping = self
                .runners
                .iter()
                .find(|mapping| mapping.os == host.host_os && mapping.arch == host.host_arch)
                .ok_or_else(|| fail("GitHub runner mapping missing for build host"))?;
            if job.planned.policy.strategy == BuildStrategy::CargoZigbuild
                && (!mapping.cargo_zigbuild || !mapping.zig)
            {
                return Err(fail(
                    "cross-build runner lacks preinstalled cargo-zigbuild or Zig",
                ));
            }
        }
        Ok(())
    }
}

fn validate_pin(pin: &ActionPin, expected_repo: &str) -> Result<(), CiError> {
    let Some((repo, sha)) = pin.reference.split_once('@') else {
        return Err(fail(
            "GitHub action pin must include an immutable commit SHA",
        ));
    };
    if repo != expected_repo || sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(fail(
            "GitHub action pin is not an approved repository commit SHA",
        ));
    }
    Ok(())
}
fn safe_runner_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 64
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn os_code(os: HostOs) -> u8 {
    match os {
        HostOs::Linux => 1,
        HostOs::Macos => 2,
        HostOs::Windows => 3,
    }
}
fn arch_code(arch: HostArch) -> u8 {
    match arch {
        HostArch::X86_64 => 1,
        HostArch::Aarch64 => 2,
        HostArch::Armv7 => 3,
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
                let expected = job
                    .planned
                    .policy
                    .toolchain
                    .cargo_zigbuild
                    .as_deref()
                    .ok_or_else(|| fail("cargo-zigbuild version missing from ReleasePlan"))?;
                out.push_str("      - name: Verify Rust and cross tools\n        shell: bash\n        run: |\n          cargo +");
                out.push_str(&shell_quote(&job.planned.policy.toolchain.rust));
                out.push_str(" --version\n          actual=\"$(cargo zigbuild --version)\"\n          test \"$actual\" = ");
                out.push_str(&shell_quote(&format!("cargo-zigbuild {expected}")));
                out.push_str("\n          zig version\n");
            }
        }
        for output in &job.outputs {
            out.push_str("      - name: Build ");
            out.push_str(&yaml_scalar(&format!(
                "{} / {}",
                job.planned.target, output.binary
            )));
            out.push_str("\n        shell: bash\n        env:\n          CARGO_TARGET_DIR: \"${{ runner.temp }}/eggpack/${{ github.run_id }}-${{ github.run_attempt }}\"\n        run: ");
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

fn yaml_scalar(value: &str) -> String {
    serde_json::to_string(value).expect("string JSON serialization cannot fail")
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// Deterministic check result; original workflow bytes are never returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    /// Whether normalized existing bytes match rendered output.
    pub matches: bool,
    /// Expected byte count after newline normalization.
    pub expected_bytes: usize,
    /// Existing byte count after newline normalization.
    pub actual_bytes: usize,
    /// First differing byte offset, if any.
    pub first_difference: Option<usize>,
}

/// Compare existing workflow bytes with deterministic output without writing files.
pub fn check_github(
    plan: &CIPlan,
    policy: &GitHubPolicy,
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    let expected = render_github(plan, policy)?;
    if existing.len() > MAX_WORKFLOW_BYTES {
        return Err(fail("existing workflow exceeds size bound"));
    }
    let expected = normalize_newlines(expected.as_bytes());
    let existing = normalize_newlines(existing);
    let matches = expected == existing;
    let first_difference = if matches {
        None
    } else {
        Some(
            expected
                .iter()
                .zip(&existing)
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| expected.len().min(existing.len())),
        )
    };
    Ok(DriftReport {
        matches,
        expected_bytes: expected.len(),
        actual_bytes: existing.len(),
        first_difference,
    })
}
fn normalize_newlines(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            out.push(b'\n');
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggpack_core::{
        CompatibilityFloor, HostArch, HostOs, PackConfig, TargetPolicy, ToolchainRequirement,
    };

    fn contract() -> DistributionContract {
        DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap()
    }
    fn plan(strategy: BuildStrategy, support: SupportTier) -> (DistributionContract, ReleasePlan) {
        let contract = contract();
        let target = "x86_64-unknown-linux-gnu";
        let policy = TargetPolicy {
            target: target.into(),
            strategy,
            host_os: HostOs::Linux,
            host_arch: HostArch::X86_64,
            qualification_host: Some(HostRequirement {
                os: HostOs::Macos,
                arch: HostArch::Aarch64,
            }),
            toolchain: ToolchainRequirement {
                rust: "1.89.0".into(),
                cargo_zigbuild: (strategy == BuildStrategy::CargoZigbuild).then(|| "0.20.0".into()),
            },
            floor: CompatibilityFloor::Glibc {
                major: 2,
                minor: 17,
            },
            qualification: Qualification::DeferredNative,
            support,
        };
        let config = PackConfig {
            schema_version: 1,
            targets: vec![policy],
        };
        let release = config
            .resolve(
                &contract,
                "1.2.3",
                "0123456789abcdef",
                &["linux-x64".into()],
            )
            .unwrap();
        (contract, release)
    }
    fn bindings(release: &ReleasePlan) -> BuildBindingsV1 {
        let target = &release.targets[0].target;
        BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                target.clone(),
                vec![eggpack_core::BuildBinding {
                    selector: LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "eggsact".into(),
                }],
            )]
            .into_iter()
            .collect(),
        }
    }
    fn policy() -> GitHubPolicy {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        GitHubPolicy {
            preflight_runner: "ubuntu-latest".into(),
            runners: vec![RunnerMapping {
                os: HostOs::Linux,
                arch: HostArch::X86_64,
                label: "ubuntu-latest".into(),
                cargo_zigbuild: true,
                zig: true,
            }],
            checkout: ActionPin {
                reference: format!("actions/checkout@{sha}"),
            },
            rust_toolchain: ActionPin {
                reference: format!("dtolnay/rust-toolchain@{sha}"),
            },
            upload_artifact: ActionPin {
                reference: format!("actions/upload-artifact@{sha}"),
            },
            triggers: vec![WorkflowTrigger::Push, WorkflowTrigger::WorkflowDispatch],
            timeout_minutes: 60,
            cancel_in_progress: true,
            artifact_retention_days: 7,
        }
    }
    fn graph(strategy: BuildStrategy, support: SupportTier) -> CIPlan {
        let (contract, release) = plan(strategy, support);
        project_ci_plan(&contract, &release, &bindings(&release)).unwrap()
    }

    #[test]
    fn rendering_is_deterministic_parseable_read_only_and_hands_off_candidate() {
        let graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
        assert_eq!(graph.targets[0].planned.target, "x86_64-unknown-linux-gnu");
        assert_eq!(
            graph.targets[0].qualification.host,
            Some(HostRequirement {
                os: HostOs::Macos,
                arch: HostArch::Aarch64,
            })
        );
        let output = render_github(&graph, &policy()).unwrap();
        assert_eq!(output, render_github(&graph, &policy()).unwrap());
        assert_eq!(output, include_str!("../tests/fixtures/native-direct.yml"));
        let encoded = graph.to_json().unwrap();
        assert_eq!(CIPlan::from_json(&encoded).unwrap(), graph);
        assert!(CIPlan::from_json(
            &encoded.replace("\"schema_version\":1", "\"schema_version\":2")
        )
        .is_err());
        let parsed: serde_yaml::Value = serde_yaml::from_str(&output).unwrap();
        assert!(parsed.get("jobs").is_some());
        let permissions = parsed["permissions"].as_mapping().unwrap();
        assert_eq!(permissions.len(), 1);
        assert_eq!(permissions.get("contents").unwrap().as_str(), Some("read"));
        let jobs = parsed["jobs"].as_mapping().unwrap();
        for (_, job) in jobs {
            let permissions = job["permissions"].as_mapping().unwrap();
            assert_eq!(permissions.len(), 1);
            assert_eq!(permissions.get("contents").unwrap().as_str(), Some("read"));
        }
        assert!(output.contains("permissions:\n  contents: read"));
        assert!(output.contains("eggpack-x86_64-unknown-linux-gnu-direct"));
        assert!(!output.contains("eggsact-1.2.3"));
        assert!(output.contains("'cargo' '+1.89.0' 'build' '--release' '--locked'"));
        assert!(!output.contains("contents: write"));
        assert!(!output.contains("id-token: write"));
        assert!(!output.to_ascii_lowercase().contains("qualify"));
        assert!(!output.contains("publish"));
        let mut secret_metadata = graph.clone();
        secret_metadata.source_revision = "secret-token-that-must-not-render".into();
        secret_metadata.release_id = "secret-release-id".into();
        let rendered = render_github(&secret_metadata, &policy()).unwrap();
        assert!(!rendered.contains("secret-token-that-must-not-render"));
        assert!(!rendered.contains("secret-release-id"));
    }

    #[test]
    fn zigbuild_reuses_core_floor_syntax_and_keeps_qualification_unresolved() {
        let graph = graph(BuildStrategy::CargoZigbuild, SupportTier::Experimental);
        let output = render_github(&graph, &policy()).unwrap();
        assert!(output.contains("x86_64-unknown-linux-gnu.2.17"));
        assert!(output.contains("cargo-zigbuild 0.20.0"));
        assert!(output.contains("continue-on-error: true"));
        assert_eq!(
            graph.targets[0].qualification.state,
            QualificationState::Unresolved
        );
        assert!(!graph.targets[0].required);
    }

    #[test]
    fn drift_check_normalizes_only_crlf_and_detects_manual_edits() {
        let graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
        let text = render_github(&graph, &policy()).unwrap();
        assert!(
            check_github(&graph, &policy(), text.as_bytes())
                .unwrap()
                .matches
        );
        let crlf = text.replace('\n', "\r\n");
        assert!(
            check_github(&graph, &policy(), crlf.as_bytes())
                .unwrap()
                .matches
        );
        let changed = text.replacen("contents: read", "contents: write", 1);
        let report = check_github(&graph, &policy(), changed.as_bytes()).unwrap();
        assert!(!report.matches);
        assert!(report.first_difference.is_some());
        assert!(check_github(&graph, &policy(), &vec![b'x'; MAX_WORKFLOW_BYTES + 1]).is_err());
    }

    #[test]
    fn rejects_mutable_malformed_wrong_repo_and_duplicate_policy() {
        let direct_graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
        let mut p = policy();
        p.checkout.reference = "actions/checkout@v4".into();
        assert!(p.validate(&direct_graph).is_err());
        let mut p = policy();
        p.checkout.reference = "somebody/checkout@0123456789abcdef0123456789abcdef01234567".into();
        assert!(p.validate(&direct_graph).is_err());
        let mut p = policy();
        p.upload_artifact.reference = "actions/upload-artifact@not-a-commit".into();
        assert!(p.validate(&direct_graph).is_err());
        let mut p = policy();
        p.runners.push(p.runners[0].clone());
        assert!(p.validate(&direct_graph).is_err());
        let mut p = policy();
        p.runners.clear();
        assert!(p.validate(&direct_graph).is_err());
        let mut p = policy();
        let duplicate = p.runners[0].clone();
        p.runners.resize(129, duplicate);
        assert!(p.validate(&direct_graph).is_err());
        let mut p = policy();
        p.timeout_minutes = 999;
        assert!(p.validate(&direct_graph).is_err());
        let cross_graph = graph(BuildStrategy::CargoZigbuild, SupportTier::Required);
        let mut p = policy();
        p.runners[0].cargo_zigbuild = false;
        assert!(p.validate(&cross_graph).is_err());
    }

    #[test]
    fn rejects_invalid_graph_and_command_drift_from_m002() {
        let mut invalid_command = graph(BuildStrategy::NativeCargo, SupportTier::Required);
        invalid_command.targets[0].outputs[0].args[1] = "test".into();
        assert!(invalid_command.validate().is_err());
        let mut invalid_gate = graph(BuildStrategy::NativeCargo, SupportTier::Required);
        invalid_gate.required_aggregation_dependencies.clear();
        assert!(invalid_gate.validate().is_err());
        let encoded = graph(BuildStrategy::NativeCargo, SupportTier::Required)
            .to_json()
            .unwrap()
            .replace("unresolved", "passed");
        assert!(CIPlan::from_json(&encoded).is_err());
    }

    #[test]
    fn archive_handoff_identity_is_bounded_deterministic_and_slot_distinct() {
        let source = "bin/tool/run".to_owned();
        let archive = LogicalOutputSelector::ArchiveMember { source };
        let name = handoff_name("x86_64-unknown-linux-gnu", &archive);
        assert!(name.starts_with("eggpack-x86_64-unknown-linux-gnu-archive-"));
        assert_eq!(name, handoff_name("x86_64-unknown-linux-gnu", &archive));
        assert_ne!(
            name,
            handoff_name("x86_64-unknown-linux-gnu", &LogicalOutputSelector::Direct)
        );
    }

    #[test]
    fn canonical_multi_target_native_and_cross_jobs_are_sorted_and_explicit() {
        let contract = DistributionContract::parse_toml_str(include_str!(
            "../tests/fixtures/mixed-direct-targets.toml"
        ))
        .unwrap();
        let config = PackConfig {
            schema_version: 1,
            targets: vec![
                TargetPolicy {
                    target: "x86_64-unknown-linux-gnu".into(),
                    strategy: BuildStrategy::NativeCargo,
                    host_os: HostOs::Linux,
                    host_arch: HostArch::X86_64,
                    qualification_host: Some(HostRequirement {
                        os: HostOs::Macos,
                        arch: HostArch::Aarch64,
                    }),
                    toolchain: ToolchainRequirement {
                        rust: "stable".into(),
                        cargo_zigbuild: None,
                    },
                    floor: CompatibilityFloor::None,
                    qualification: Qualification::DeferredNative,
                    support: SupportTier::Required,
                },
                TargetPolicy {
                    target: "aarch64-unknown-linux-gnu".into(),
                    strategy: BuildStrategy::CargoZigbuild,
                    host_os: HostOs::Linux,
                    host_arch: HostArch::X86_64,
                    qualification_host: None,
                    toolchain: ToolchainRequirement {
                        rust: "1.89.0".into(),
                        cargo_zigbuild: Some("0.20.0".into()),
                    },
                    floor: CompatibilityFloor::Glibc {
                        major: 2,
                        minor: 17,
                    },
                    qualification: Qualification::Structural,
                    support: SupportTier::NonGating,
                },
            ],
        };
        let release = config
            .resolve(
                &contract,
                "1.2.3",
                "0123456789abcdef",
                &["linux-arm64".into(), "linux-x64".into()],
            )
            .unwrap();
        let mut binding_map = std::collections::BTreeMap::new();
        for target in &release.targets {
            binding_map.insert(
                target.target.clone(),
                vec![eggpack_core::BuildBinding {
                    selector: LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "eggsact".into(),
                }],
            );
        }
        let bindings = BuildBindingsV1 {
            schema_version: 1,
            targets: binding_map,
        };
        let graph = project_ci_plan(&contract, &release, &bindings).unwrap();
        assert_eq!(graph.targets.len(), 2);
        assert_eq!(graph.targets[0].planned.target, "aarch64-unknown-linux-gnu");
        assert_eq!(graph.targets[1].planned.target, "x86_64-unknown-linux-gnu");
        assert_eq!(
            graph.targets[0].planned.policy.strategy,
            BuildStrategy::CargoZigbuild
        );
        assert_eq!(
            graph.targets[1].planned.policy.strategy,
            BuildStrategy::NativeCargo
        );
        assert!(graph
            .required_aggregation_dependencies
            .contains(&graph.targets[1].job_id));
        assert!(!graph
            .required_aggregation_dependencies
            .contains(&graph.targets[0].job_id));
        assert_eq!(
            graph.targets[1].qualification.host.unwrap().os,
            HostOs::Macos
        );
        assert!(graph.targets[0].outputs[0]
            .args
            .iter()
            .any(|arg| { arg == "aarch64-unknown-linux-gnu.2.17" }));
        assert!(graph.targets[1].outputs[0]
            .args
            .iter()
            .any(|arg| arg == &graph.targets[1].planned.target));
        assert!(graph.validate().is_ok());
        let github = policy();
        let workflow = render_github(&graph, &github).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
        assert!(parsed.get("jobs").is_some());
        assert_eq!(
            workflow,
            include_str!("../tests/fixtures/native-direct-multitarget.yml")
        );
    }

    #[test]
    fn bundle_and_archive_logical_slots_survive_projection_without_filename_inference() {
        for (fixture, product, sources) in [
            (
                include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
                "codegg",
                vec![
                    LogicalOutputSelector::BundleEntry { index: 0 },
                    LogicalOutputSelector::BundleEntry { index: 1 },
                    LogicalOutputSelector::BundleEntry { index: 2 },
                ],
            ),
            (
                include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
                "egress",
                vec![
                    LogicalOutputSelector::ArchiveMember {
                        source: "egress".into(),
                    },
                    LogicalOutputSelector::ArchiveMember {
                        source: "bin/egress-helper".into(),
                    },
                ],
            ),
        ] {
            let contract = DistributionContract::parse_toml_str(fixture).unwrap();
            let target = "x86_64-unknown-linux-gnu";
            let config = PackConfig {
                schema_version: 1,
                targets: vec![TargetPolicy {
                    target: target.into(),
                    strategy: BuildStrategy::NativeCargo,
                    host_os: HostOs::Linux,
                    host_arch: HostArch::X86_64,
                    qualification_host: None,
                    toolchain: ToolchainRequirement {
                        rust: "stable".into(),
                        cargo_zigbuild: None,
                    },
                    floor: CompatibilityFloor::None,
                    qualification: Qualification::DeferredNative,
                    support: SupportTier::Required,
                }],
            };
            let release = config
                .resolve(&contract, "2.0.0", "revision", &["linux-x64".into()])
                .unwrap();
            let outputs = sources
                .iter()
                .enumerate()
                .map(|(index, selector)| eggpack_core::BuildBinding {
                    selector: selector.clone(),
                    package: product.into(),
                    binary: format!("bin{index}"),
                })
                .collect();
            let bindings = BuildBindingsV1 {
                schema_version: 1,
                targets: [(target.into(), outputs)].into_iter().collect(),
            };
            let graph = project_ci_plan(&contract, &release, &bindings).unwrap();
            assert_eq!(
                graph.targets[0].planned.artifact_form,
                release.targets[0].artifact_form
            );
            let expected_slots: BTreeSet<_> = sources.iter().cloned().collect();
            assert_eq!(
                graph.targets[0]
                    .outputs
                    .iter()
                    .map(|output| output.selector.clone())
                    .collect::<BTreeSet<_>>(),
                expected_slots
            );
            assert_eq!(
                graph.targets[0]
                    .outputs
                    .iter()
                    .map(|output| output.package.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                1
            );
            assert_eq!(
                graph.targets[0]
                    .outputs
                    .iter()
                    .map(|output| output.handoff_name.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                sources.len()
            );
        }
    }
}
