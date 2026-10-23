//! The release workflow renderers and the emission helpers that build their bytes.

use crate::consumer_validator::CONSUMER_EVIDENCE_FILE;
use crate::drift::MAX_WORKFLOW_BYTES;
use crate::error::{fail, CiError};
use crate::identity::{
    ReleaseWorkflowShapeV1, RUNTIME_CI_PLAN, RUNTIME_GITHUB_POLICY, RUNTIME_IDENTITY_ARTIFACT,
    RUNTIME_IDENTITY_DIR, RUNTIME_RELEASE_PLAN, UNRESOLVED_RELEASE_ID, UNRESOLVED_SOURCE_REVISION,
};
use crate::planning::project_ci_plan;
use crate::policy::{
    validate_pin, ActionPin, EggpackToolPolicy, GitHubPolicy, GitHubReleaseInputsV1,
    ReleaseIdentityMode, StagingTagSource, WorkflowTrigger, BUILD_HANDOFF_FILE, CANDIDATES_DIR,
    GATE_OUTCOME_FILE, QUALIFICATION_EVIDENCE_FILE,
};
use crate::release_plan::{
    consumer_evidence_handoff_name, consumer_job_id, project_release_plan_with_consumer,
    with_github_draft_staging, ReleaseCIPlanV1, StagingProvider,
};
use crate::render_github::{
    contains_unsafe_dispatch_expression_in_run, cross_tools_section, provisioned_build_env,
    shell_quote, yaml_scalar,
};
use crate::runner::RunnerCommand;
use eggpack_contract::DistributionContract;
use eggpack_core::{BuildStrategy, HostArch, HostOs, Qualification};

/// Emit a private working-directory creation step for generated release jobs.
///
/// Directory names are renderer constants drawn from the same job's CLI argv;
/// no caller input ever reaches the shell. The assertion documents that only
/// fixed safe relative paths are representable here.
fn mkdir_step(out: &mut String, name: &str, dir: &str) {
    assert!(
        !dir.is_empty()
            && dir.len() <= 128
            && dir
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"./-_".contains(&b))
            && !dir.starts_with('/')
            && !dir.contains(".."),
        "generated mkdir directory is not a fixed safe relative path"
    );
    out.push_str("      - name: ");
    out.push_str(name);
    out.push_str("\n        shell: bash\n        run: ");
    out.push_str(&yaml_scalar(&format!("mkdir -p '{dir}'")));
    out.push('\n');
}

pub(crate) fn tool_install_snippet(tool: &EggpackToolPolicy) -> String {
    format!(
        "      - name: Install pinned Eggpack tool\n        shell: bash\n        timeout-minutes: {timeout}\n        run: |\n          cargo install --git {repo} --rev {rev} --locked {package}\n          eggpack --version\n",
        timeout = tool.install_timeout_minutes,
        repo = tool.repo,
        rev = tool.revision,
        package = tool.package
    )
}

fn checkout_snippet(out: &mut String, policy: &GitHubPolicy, source_sha: Option<&str>) {
    out.push_str("      - name: Check out source\n        uses: ");
    out.push_str(&yaml_scalar(&policy.checkout.reference));
    if let Some(reference) = source_sha {
        out.push_str("\n        with:\n          ref: ");
        out.push_str(&yaml_scalar(reference));
        out.push_str("\n      - name: Verify frozen source checkout\n        shell: bash\n        env:\n          EGGPACK_EXPECTED_SOURCE_SHA: ");
        out.push_str(&yaml_scalar(reference));
        out.push_str("\n        run: test \"$(git rev-parse --verify HEAD^{commit})\" = \"$EGGPACK_EXPECTED_SOURCE_SHA\"\n");
        // Frozen SHA checkouts never carry tags (`fetch --no-tags`), but the
        // runtime identity and source-verification commands require the exact
        // release tag locally. Fetch only that tag and bind it to the frozen
        // revision; any peel mismatch fails the job before Eggpack runs.
        if let Some(staging) = policy.staging.as_ref() {
            let tag_expression = match staging.tag_source {
                StagingTagSource::RefName => "${{ github.ref_name }}",
                StagingTagSource::DispatchInput => "${{ inputs.release_tag }}",
            };
            out.push_str("      - name: Fetch release tag for local verification\n        shell: bash\n        env:\n          EGGPACK_RELEASE_TAG: ");
            out.push_str(&yaml_scalar(tag_expression));
            out.push_str("\n          EGGPACK_EXPECTED_SOURCE_SHA: ");
            out.push_str(&yaml_scalar(reference));
            out.push_str("\n        run: ");
            out.push_str(&yaml_scalar(&release_tag_fetch_script()));
            out.push('\n');
        }
    }
    out.push('\n');
}

/// Validate the dispatch/push tag shape, fetch exactly that tag ref, and
/// assert it peels to the frozen source revision. The tag value arrives only
/// via `EGGPACK_RELEASE_TAG`; no workflow expression may appear in this body.
fn release_tag_fetch_script() -> String {
    String::from(
        "set -euo pipefail\ntag=\"$EGGPACK_RELEASE_TAG\"\nif [ -z \"$tag\" ] || [ \"${#tag}\" -gt 128 ]; then\n  echo 'release tag is empty or oversized' >&2\n  exit 1\nfi\nref=\"refs/tags/$tag\"\ngit check-ref-format \"$ref\"\ngit fetch --no-tags --depth=1 origin \"$ref:$ref\"\npeeled=\"$(git rev-parse --verify \"$ref^{commit}\")\"\nif [ \"$peeled\" != \"$EGGPACK_EXPECTED_SOURCE_SHA\" ]; then\n  echo 'release tag does not peel to the frozen source revision' >&2\n  exit 1\nfi\n",
    )
}

fn frozen_source_sha_expression(runtime: bool) -> &'static str {
    if runtime {
        "${{ needs.resolve.outputs.source_sha }}"
    } else {
        "${{ needs.preflight.outputs.source_sha }}"
    }
}

fn source_job_id(runtime: bool) -> &'static str {
    if runtime {
        "resolve"
    } else {
        "preflight"
    }
}

fn emit_source_needs(out: &mut String, existing: &[String], staging: bool, runtime: bool) {
    if !staging {
        out.push_str(&existing.join(", "));
        return;
    }
    let source_job = source_job_id(runtime);
    let mut needs = existing.to_vec();
    if !needs.iter().any(|need| need == source_job) {
        needs.push(source_job.to_owned());
    }
    out.push_str("[ ");
    out.push_str(&needs.join(", "));
    out.push_str(" ]");
}

fn release_tag_resolution_script(mapped_semver: bool, expected_sha: Option<&str>) -> String {
    let mut script = String::from(
        "set -euo pipefail\ntag=\"$EGGPACK_RELEASE_TAG\"\nif [ \"${#tag}\" -gt 255 ] || [ -z \"$tag\" ]; then\n  echo 'release tag is empty or oversized' >&2\n  exit 1\nfi\n",
    );
    if mapped_semver {
        script.push_str(
            "if [ \"${#tag}\" -gt 64 ] || [[ ! \"$tag\" =~ ^v(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$ ]]; then\n  echo 'release_tag must be a canonical vX.Y.Z stable tag' >&2\n  exit 1\nfi\nexport LC_ALL=C\nversion=\"${tag#v}\"\nIFS='.' read -r major minor patch <<< \"$version\"\nfor component in \"$major\" \"$minor\" \"$patch\"; do\n  if [ \"${#component}\" -gt 20 ] || { [ \"${#component}\" -eq 20 ] && [[ \"$component\" > 18446744073709551615 ]]; }; then\n    echo 'release_tag numeric component is out of range' >&2\n    exit 1\n  fi\ndone\n",
        );
    }
    script.push_str(
        "ref=\"refs/tags/$tag\"\ngit check-ref-format \"$ref\"\naskpass=\"$RUNNER_TEMP/eggpack-tag-askpass\"\nverify_repo=\"$RUNNER_TEMP/eggpack-tag-verify.git\"\ntrap 'rm -f -- \"$askpass\"' EXIT\ncat >\"$askpass\" <<'ASKPASS'\n#!/bin/sh\ncase \"$1\" in\n  *Username*) printf '%s\\n' x-access-token ;;\n  *Password*) printf '%s\\n' \"$EGGPACK_GIT_TOKEN\" ;;\n  *) exit 1 ;;\nesac\nASKPASS\nchmod 700 \"$askpass\"\nremote=\"$GITHUB_SERVER_URL/$GITHUB_REPOSITORY.git\"\nrefs=\"$(GIT_ASKPASS=\"$askpass\" GIT_TERMINAL_PROMPT=0 EGGPACK_GIT_TOKEN=\"$GITHUB_TOKEN\" git ls-remote --exit-code \"$remote\" \"$ref\" \"$ref^{}\")\"\ndirect=\npeeled=\nwhile IFS=\"$(printf '\\t')\" read -r oid name; do\n  case \"$name\" in\n    \"$ref\") [ -z \"$direct\" ] || { echo 'duplicate tag ref result' >&2; exit 1; }; direct=\"$oid\" ;;\n    \"$ref^{}\") [ -z \"$peeled\" ] || { echo 'duplicate peeled ref result' >&2; exit 1; }; peeled=\"$oid\" ;;\n    *) echo 'unexpected tag resolution result' >&2; exit 1 ;;\n  esac\ndone <<< \"$refs\"\nsource_sha=\"${peeled:-$direct}\"\nif [[ ! \"$source_sha\" =~ ^[0-9a-f]{40}$ ]]; then\n  echo 'tag did not resolve to one full object OID' >&2\n  exit 1\nfi\ngit init --bare \"$verify_repo\" >/dev/null\nGIT_ASKPASS=\"$askpass\" GIT_TERMINAL_PROMPT=0 EGGPACK_GIT_TOKEN=\"$GITHUB_TOKEN\" git --git-dir=\"$verify_repo\" fetch --no-tags --depth=1 \"$remote\" \"$ref\" >/dev/null\nfetched_commit=\"$(git --git-dir=\"$verify_repo\" rev-parse --verify 'FETCH_HEAD^{commit}')\"\nif [ \"$fetched_commit\" != \"$source_sha\" ]; then\n  echo 'tag did not resolve to the advertised commit object' >&2\n  exit 1\nfi\n"
    );
    if expected_sha.is_some() {
        script.push_str(
            "if [ \"$source_sha\" != \"$EGGPACK_EXPECTED_SOURCE_SHA\" ]; then\n  echo 'release tag moved after preflight resolution' >&2\n  exit 1\nfi\n",
        );
    } else {
        script.push_str("printf 'source_sha=%s\\n' \"$source_sha\" >> \"$GITHUB_OUTPUT\"\n");
    }
    script
}

fn emit_tag_resolution_step(
    out: &mut String,
    tag_source: StagingTagSource,
    mapped_semver: bool,
    expected_sha: Option<&str>,
    id: &str,
) {
    let tag_expression = match tag_source {
        StagingTagSource::RefName => "${{ github.ref_name }}",
        StagingTagSource::DispatchInput => "${{ inputs.release_tag }}",
    };
    out.push_str("      - name: Resolve and validate immutable release source\n        id: ");
    out.push_str(id);
    out.push_str("\n        shell: bash\n        env:\n          EGGPACK_RELEASE_TAG: ");
    out.push_str(&yaml_scalar(tag_expression));
    out.push_str("\n          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n");
    if let Some(expected_sha) = expected_sha {
        out.push_str("          EGGPACK_EXPECTED_SOURCE_SHA: ");
        out.push_str(&yaml_scalar(expected_sha));
        out.push('\n');
    }
    out.push_str("        run: ");
    out.push_str(&yaml_scalar(&release_tag_resolution_script(
        mapped_semver,
        expected_sha,
    )));
    out.push('\n');
}

fn source_verify_snippet(
    out: &mut String,
    inputs: &GitHubReleaseInputsV1,
    staging: bool,
    policy: &GitHubPolicy,
) {
    if !staging {
        return;
    }
    let mapped = policy.release_identity_mode == Some(ReleaseIdentityMode::VPrefixedStableSemver);
    let mapped_staging = if mapped {
        policy.staging.as_ref()
    } else {
        None
    };
    let command = RunnerCommand::VerifySource {
        release_plan: inputs.release_plan.clone(),
        github_policy: mapped_staging.map(|staging| staging.inputs.github_policy.clone()),
        expected_tag: mapped_staging.map(|_| "$release_tag".to_owned()),
        source_root: mapped_staging.map(|_| "${{ github.workspace }}".to_owned()),
    };
    out.push_str("      - name: Verify checked-out release source\n        shell: bash\n");
    if let Some(staging) = mapped_staging {
        let expression = match staging.tag_source {
            StagingTagSource::RefName => "${{ github.ref_name }}",
            StagingTagSource::DispatchInput => "${{ inputs.release_tag }}",
        };
        out.push_str("        env:\n          EGGPACK_RELEASE_TAG: ");
        out.push_str(&yaml_scalar(expression));
        out.push('\n');
    }
    out.push_str("        run: ");
    out.push_str(&yaml_scalar(&command.to_shell()));
    out.push('\n');
}

/// Download the runtime identity preflight artifact (reusable mode only).
///
/// Every release-producing job downloads the invocation-local ReleasePlan,
/// ReleaseCIPlan, and GitHubDraftPolicy before its `_verify-source` step.
/// Exact mode emits nothing here (M003c byte-compatible).
fn runtime_identity_download_step(out: &mut String, download_pin: &ActionPin, runtime: bool) {
    if !runtime {
        return;
    }
    out.push_str("      - name: Download runtime release identity\n        uses: ");
    out.push_str(&yaml_scalar(&download_pin.reference));
    out.push_str("\n        with:\n          name: ");
    out.push_str(&yaml_scalar(RUNTIME_IDENTITY_ARTIFACT));
    out.push_str("\n          path: ");
    out.push_str(&yaml_scalar(&format!("./{RUNTIME_IDENTITY_DIR}")));
    out.push_str("\n          if-no-files-found: error\n");
}

/// Deterministically render build -> qualify -> gate -> aggregate GitHub workflow.
///
/// M002a execution wiring: every generated CLI invocation carries all required
/// explicit inputs; build jobs create the exact canonical artifact that
/// qualification jobs consume; qualification artifacts carry build handoff +
/// evidence + candidate bytes; gate and aggregate consume the canonical
/// per-target input layout. No cross-job absolute paths are serialized and no
/// repository discovery is introduced.
pub fn render_release_github(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
) -> Result<String, CiError> {
    render_release_github_inner(graph, policy, None)
}

/// Runtime render configuration for reusable workflows (M003d section 4D).
struct RuntimeRender {
    /// Complete `_resolve-release` command for the resolve job, with the
    /// workflow tag expression and the `$head_sha` revision marker.
    resolve: RunnerCommand,
}

/// Deterministically render a reusable release workflow from static shape.
///
/// The checked-in bytes work for any future tag: no release_id,
/// source_revision, exact tag, or digest is embedded (rendering fails if the
/// unresolved placeholders leak). At runtime the `resolve` job checks out
/// the event-selected exact tag, derives HEAD, resolves invocation-local
/// ReleasePlan/ReleaseCIPlan/GitHubDraftPolicy into workflow-private
/// storage, and uploads them as an internal preflight artifact that every
/// later job downloads before its `_verify-source` step.
pub fn render_reusable_release_github(
    contract: &DistributionContract,
    shape: &ReleaseWorkflowShapeV1,
    policy: &GitHubPolicy,
) -> Result<String, CiError> {
    shape.validate()?;
    let staging_intent = shape
        .staging
        .as_ref()
        .ok_or_else(|| fail("reusable release rendering requires a staging intent"))?;
    let static_inputs = policy
        .release_inputs
        .as_ref()
        .ok_or_else(|| fail("reusable rendering requires explicit release input paths"))?;
    let pack_config_path = static_inputs
        .pack_config
        .clone()
        .ok_or_else(|| fail("reusable rendering requires a PackConfig path"))?;
    let template_path = static_inputs
        .draft_template
        .clone()
        .ok_or_else(|| fail("reusable rendering requires a draft template path"))?;
    if !shape.consumer_validators.is_empty() && static_inputs.consumer_validators.is_none() {
        return Err(fail(
            "reusable consumer validators require an explicit validator map path",
        ));
    }
    // Project the static graph structure against a dummy identity. The
    // renderer never serializes release identity (proven by the leak
    // assertion below); the dummy only drives job/handoff derivation.
    let pack = shape.pack_config();
    let dummy_plan = pack
        .resolve(
            contract,
            UNRESOLVED_RELEASE_ID,
            UNRESOLVED_SOURCE_REVISION,
            &shape.selected_aliases,
        )
        .map_err(|_| fail("workflow shape does not resolve against its contract"))?;
    let dummy_ci = project_ci_plan(contract, &dummy_plan, &shape.build_bindings)?;
    let base = project_release_plan_with_consumer(
        &dummy_ci,
        &shape.qualification_bindings,
        &shape.build_bindings,
        &dummy_plan,
        shape.consumer_validators.clone(),
    )?;
    let graph = with_github_draft_staging(base, staging_intent.required)?;
    // Rewrite identity-carrying input paths to workflow-private storage.
    // Static checked-in paths (contract, bindings, template, PackConfig,
    // presentation, validator map) stay repository-relative.
    let mut runtime_policy = policy.clone();
    {
        let runtime_inputs = runtime_policy
            .release_inputs
            .as_mut()
            .ok_or_else(|| fail("reusable rendering requires explicit release input paths"))?;
        runtime_inputs.release_plan = RUNTIME_RELEASE_PLAN.to_owned();
        runtime_inputs.ci_plan = RUNTIME_CI_PLAN.to_owned();
        let runtime_staging = runtime_policy
            .staging
            .as_mut()
            .ok_or_else(|| fail("reusable rendering requires a staging policy"))?;
        if runtime_staging.tag_source != staging_intent.tag_source {
            return Err(fail(
                "shape staging tag source differs from provider staging policy",
            ));
        }
        if staging_intent.provider != StagingProvider::GitHubDraft {
            return Err(fail("unsupported shape staging provider"));
        }
        runtime_staging.inputs.github_policy = RUNTIME_GITHUB_POLICY.to_owned();
    }
    let resolve = RunnerCommand::ResolveRelease {
        contract: static_inputs.contract.clone(),
        pack_config: pack_config_path,
        build_bindings: static_inputs.build_bindings.clone(),
        qualification_bindings: static_inputs.qualification_bindings.clone(),
        consumer_validators: static_inputs.consumer_validators.clone(),
        selected: shape.selected_aliases.join(","),
        // Every workflow-selected ref stays in environment data. The
        // renderer expands only the quoted variable reference in the shell.
        tag: "$release_tag".to_owned(),
        source_revision: "$head_sha".to_owned(),
        template: template_path,
        source_root: "${{ github.workspace }}".to_owned(),
        output_plan: format!("./{RUNTIME_RELEASE_PLAN}"),
        output_ci_plan: format!("./{RUNTIME_CI_PLAN}"),
        output_github_policy: format!("./{RUNTIME_GITHUB_POLICY}"),
        identity_mode: policy.release_identity_mode,
    };
    let rendered =
        render_release_github_inner(&graph, &runtime_policy, Some(&RuntimeRender { resolve }))?;
    if rendered.contains(UNRESOLVED_RELEASE_ID) || rendered.contains(UNRESOLVED_SOURCE_REVISION) {
        return Err(fail("reusable workflow embeds unresolved release identity"));
    }
    Ok(rendered)
}

/// Validated release-render inputs shared by every job and step emitter.
///
/// The context is built once, after all fail-closed validation, so the
/// emitters never re-derive a policy decision.
struct ReleaseRenderContext<'a> {
    /// Projected release CI graph (intent, not evidence).
    graph: &'a ReleaseCIPlanV1,
    /// Caller-supplied GitHub policy.
    policy: &'a GitHubPolicy,
    /// Runtime render configuration; `Some` only for reusable workflows.
    runtime: Option<&'a RuntimeRender>,
    /// Pinned Eggpack tool policy.
    tool: &'a EggpackToolPolicy,
    /// Pinned `actions/download-artifact` action.
    download_pin: &'a ActionPin,
    /// Explicit release input paths.
    inputs: &'a GitHubReleaseInputsV1,
    /// Explicit consumer validator map path, when the graph has validators.
    validator_map: Option<String>,
    /// Whether the graph carries a staging intent.
    staging_enabled: bool,
}

impl ReleaseRenderContext<'_> {
    /// Whether this is the reusable (runtime) rendering.
    fn is_runtime(&self) -> bool {
        self.runtime.is_some()
    }

    /// The frozen source SHA expression for release-producing jobs.
    fn frozen_sha(&self) -> &'static str {
        frozen_source_sha_expression(self.is_runtime())
    }

    /// The frozen source SHA expression, but only when staging pins the
    /// checkout to a resolved revision.
    fn frozen_sha_opt(&self) -> Option<&'static str> {
        self.staging_enabled.then_some(self.frozen_sha())
    }
}

/// Validate the projected release graph and the caller-supplied policy.
fn validate_release_render_policy(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
) -> Result<(), CiError> {
    graph.validate()?;
    policy.validate(&graph.ci_plan)?;
    Ok(())
}

/// Resolve the release-identity mode for a release render.
///
/// A mapped identity is only expressible in the reusable workflow, and only
/// in the mapped stable-semver mode; every other combination fails closed.
fn validate_release_identity_mode(
    policy: &GitHubPolicy,
    runtime: Option<&RuntimeRender>,
) -> Result<(), CiError> {
    if policy.release_identity_mode.is_some() && runtime.is_none() {
        return Err(fail(
            "release identity mapping requires reusable workflow rendering",
        ));
    }
    if let Some(mode) = policy.release_identity_mode {
        if mode != ReleaseIdentityMode::VPrefixedStableSemver {
            return Err(fail("explicit exact-tag mode must be omitted"));
        }
    }
    Ok(())
}

/// Cross-check the staging tag source against the workflow triggers.
///
/// A mapped identity is resolved from exactly one event shape: a tag push
/// for `RefName`, an explicit dispatch input for `DispatchInput`.
fn validate_staging_trigger_match(policy: &GitHubPolicy) -> Result<(), CiError> {
    if policy.release_identity_mode.is_some() {
        let staging = policy
            .staging
            .as_ref()
            .ok_or_else(|| fail("release identity mapping requires staging"))?;
        let has_push = policy.triggers.contains(&WorkflowTrigger::Push);
        let has_dispatch = policy.triggers.contains(&WorkflowTrigger::WorkflowDispatch);
        match staging.tag_source {
            StagingTagSource::RefName if has_push && !has_dispatch => {}
            StagingTagSource::DispatchInput if has_dispatch && !has_push => {}
            _ => {
                return Err(fail(
                    "release identity tag source does not match workflow triggers",
                ))
            }
        }
    }
    Ok(())
}

/// Validate the pinned Eggpack tool, the pinned download action, and the
/// explicit release input paths.
fn validate_release_tool_pins(
    policy: &GitHubPolicy,
) -> Result<(&EggpackToolPolicy, &ActionPin, &GitHubReleaseInputsV1), CiError> {
    let tool = policy
        .eggpack_tool
        .as_ref()
        .ok_or_else(|| fail("M002 release rendering requires pinned Eggpack tool policy"))?;
    tool.validate()?;
    let download_pin = policy
        .download_artifact
        .as_ref()
        .ok_or_else(|| fail("M002 release rendering requires download-artifact pin"))?;
    validate_pin(download_pin, "actions/download-artifact")?;
    let inputs = policy
        .release_inputs
        .as_ref()
        .ok_or_else(|| fail("M002a release rendering requires explicit release input paths"))?;
    inputs.validate()?;
    Ok((tool, download_pin, inputs))
}

/// Emulated qualification requires an explicit finite provider runtime
/// policy; never silently invoke M003 with an empty runtime.
fn validate_emulated_runtime_policy(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
) -> Result<(), CiError> {
    for qual in &graph.qualifications {
        let planned = graph
            .ci_plan
            .targets
            .iter()
            .find(|job| job.planned.target == qual.target)
            .ok_or_else(|| fail("qualification target is not in CIPlan"))?;
        if planned.planned.policy.qualification == Qualification::Emulated {
            let has_runtime = policy
                .emulated_sysroots
                .as_ref()
                .is_some_and(|map| map.contains_key(&qual.target));
            if !has_runtime {
                return Err(fail(
                    "emulated qualification requires an explicit provider runtime policy",
                ));
            }
        }
    }
    Ok(())
}

/// Consumer validators require an explicit checked-in validator map path;
/// never resolve validator scripts by discovery.
fn resolve_consumer_validator_map(
    graph: &ReleaseCIPlanV1,
    inputs: &GitHubReleaseInputsV1,
) -> Result<Option<String>, CiError> {
    if graph.consumer_validators.is_empty() {
        return Ok(None);
    }
    Ok(Some(inputs.consumer_validators.clone().ok_or_else(
        || fail("consumer validators require an explicit validator map path"),
    )?))
}

/// Emit the source checkout step for a release-producing job.
fn emit_source_checkout_step(
    out: &mut String,
    ctx: &ReleaseRenderContext<'_>,
    source_sha: Option<&str>,
) {
    checkout_snippet(out, ctx.policy, source_sha);
}

/// Emit the runtime identity download step for a release-producing job.
fn emit_runtime_identity_download_step(out: &mut String, ctx: &ReleaseRenderContext<'_>) {
    runtime_identity_download_step(out, ctx.download_pin, ctx.is_runtime());
}

/// Emit the pinned tool install and the source verification that follows it.
///
/// M003e: the pinned tool is installed before any Eggpack invocation. Source
/// verification runs here — after the toolchain the install relies on, before
/// any release work — instead of ahead of the tool.
fn emit_tool_install_and_verify_steps(out: &mut String, ctx: &ReleaseRenderContext<'_>) {
    out.push_str(&tool_install_snippet(ctx.tool));
    source_verify_snippet(out, ctx.inputs, ctx.staging_enabled, ctx.policy);
}

/// Emit the release workflow header: name, triggers, permissions, and
/// concurrency.
///
/// The header mirrors M001 build rendering (deterministic, read-only, pinned).
/// When staging is enabled, workflow_dispatch carries an explicit
/// release_tag input (exact existing tag; no latest/branch default).
fn emit_release_workflow_header(out: &mut String, ctx: &ReleaseRenderContext<'_>) {
    let policy = ctx.policy;
    let staging_enabled = ctx.staging_enabled;
    out.push_str("name: Eggpack candidate builds\n'on':\n");
    for trigger in &policy.triggers {
        match trigger {
            WorkflowTrigger::Push => out.push_str("  push:\n"),
            WorkflowTrigger::WorkflowDispatch => {
                out.push_str("  workflow_dispatch:\n");
                if staging_enabled
                    && policy.staging.as_ref().is_some_and(|staging| {
                        staging.tag_source == StagingTagSource::DispatchInput
                    })
                {
                    out.push_str("    inputs:\n      release_tag:\n        description: Exact existing tag to stage\n        required: true\n        type: string\n");
                }
            }
        }
    }
    out.push_str("permissions:\n  contents: read\nconcurrency:\n");
    // Release-scoped concurrency: tag pushes serialize on their tag ref.
    // When staging is enabled, manual dispatch also serializes on the
    // resolved staging tag (not just the branch ref). M003a remote
    // reconciliation remains authoritative; concurrency is best-effort.
    if staging_enabled
        && policy
            .staging
            .as_ref()
            .is_some_and(|staging| staging.tag_source == StagingTagSource::DispatchInput)
    {
        out.push_str("  group: eggpack-${{ github.workflow }}-${{ github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref }}\n");
    } else {
        out.push_str("  group: eggpack-${{ github.workflow }}-${{ github.ref }}\n");
    }
    out.push_str("  cancel-in-progress: ");
    out.push_str(if policy.cancel_in_progress {
        "true\n"
    } else {
        "false\n"
    });
}

/// Emit the preflight job: runner, permissions, tag resolution, checkout, and
/// the Cargo availability probe.
fn emit_preflight_job(out: &mut String, ctx: &ReleaseRenderContext<'_>) -> Result<(), CiError> {
    let policy = ctx.policy;
    let staging_enabled = ctx.staging_enabled;
    out.push_str("jobs:\n  preflight:\n");
    if staging_enabled
        && policy
            .staging
            .as_ref()
            .is_some_and(|staging| staging.tag_source == StagingTagSource::RefName)
    {
        out.push_str("    if: github.event_name == 'push' && github.ref_type == 'tag'\n");
    }
    out.push_str("    runs-on: ");
    out.push_str(&yaml_scalar(&policy.preflight_runner));
    out.push_str("\n    permissions:\n      contents: read\n");
    if staging_enabled {
        out.push_str(
            "    outputs:\n      source_sha: ${{ steps.resolve_tag.outputs.source_sha }}\n",
        );
    }
    out.push_str("    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n");
    if staging_enabled {
        let staging_policy = policy
            .staging
            .as_ref()
            .ok_or_else(|| fail("staging requested but no GitHub staging policy exists"))?;
        emit_tag_resolution_step(
            out,
            staging_policy.tag_source,
            policy.release_identity_mode == Some(ReleaseIdentityMode::VPrefixedStableSemver),
            None,
            "resolve_tag",
        );
        checkout_snippet(
            out,
            policy,
            Some("${{ steps.resolve_tag.outputs.source_sha }}"),
        );
    } else {
        checkout_snippet(out, policy, None);
    }
    // M003e: exact staging renders verify the checked-in release plan in
    // preflight, but only after installing the pinned tool — no earlier step
    // provides it. Reusable renders carry no Eggpack invocation in preflight:
    // the runtime plan does not exist yet (`resolve` produces it), `resolve`
    // verifies HEAD during `_resolve-release`, and every later job
    // re-verifies against the downloaded runtime identity.
    if staging_enabled {
        if ctx.runtime.is_none() {
            out.push_str(&tool_install_snippet(ctx.tool));
            source_verify_snippet(out, ctx.inputs, staging_enabled, policy);
        }
    } else {
        source_verify_snippet(out, ctx.inputs, staging_enabled, policy);
    }
    out.push_str("      - name: Check Cargo availability\n        shell: bash\n        run: cargo --version\n");
    Ok(())
}

/// Emit the runtime identity preflight job (reusable mode only).
///
/// The job checks out the commit resolved by preflight, derives/verifies
/// HEAD, resolves the invocation-local ReleasePlan/ReleaseCIPlan/
/// GitHubDraftPolicy, and uploads those runtime documents as an internal
/// preflight artifact. Exact mode emits nothing here (M003c
/// byte-compatible).
fn emit_resolve_job(out: &mut String, ctx: &ReleaseRenderContext<'_>) -> Result<(), CiError> {
    let policy = ctx.policy;
    let Some(runtime) = ctx.runtime else {
        return Ok(());
    };
    let staging_policy = policy
        .staging
        .as_ref()
        .ok_or_else(|| fail("reusable rendering requires a staging policy"))?;
    out.push_str("  resolve:\n    needs: preflight\n    outputs:\n      source_sha: ${{ needs.preflight.outputs.source_sha }}\n    runs-on: ");
    out.push_str(&yaml_scalar(&policy.preflight_runner));
    out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n");
    checkout_snippet(
        out,
        policy,
        Some("${{ needs.preflight.outputs.source_sha }}"),
    );
    out.push_str(&tool_install_snippet(ctx.tool));
    // M003e: the runtime identity directory must exist before
    // `_resolve-release` writes the three invocation-local documents.
    mkdir_step(
        out,
        "Create runtime identity directory",
        &format!("./{RUNTIME_IDENTITY_DIR}"),
    );
    out.push_str("      - name: Resolve runtime release identity\n        shell: bash\n");
    let expression = match staging_policy.tag_source {
        StagingTagSource::RefName => "${{ github.ref_name }}",
        StagingTagSource::DispatchInput => "${{ inputs.release_tag }}",
    };
    out.push_str("        env:\n          EGGPACK_RELEASE_TAG: ");
    out.push_str(&yaml_scalar(expression));
    out.push('\n');
    out.push_str("        run: |\n          head_sha=\"$(git rev-parse --verify HEAD^{commit})\"\n          ");
    out.push_str(&runtime.resolve.to_shell());
    out.push('\n');
    out.push_str("      - name: Upload runtime release identity\n        uses: ");
    out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
    out.push_str("\n        with:\n          name: ");
    out.push_str(&yaml_scalar(RUNTIME_IDENTITY_ARTIFACT));
    out.push_str("\n          path: ");
    out.push_str(&yaml_scalar(&format!("./{RUNTIME_IDENTITY_DIR}")));
    out.push_str("\n          if-no-files-found: error\n          retention-days: ");
    out.push_str(&policy.artifact_retention_days.to_string());
    out.push('\n');
    Ok(())
}

/// Emit the build jobs: M001 cargo invocations plus pinned tool install,
/// explicit `_capture-build` into the canonical per-target directory, and
/// upload of the entire canonical directory as one deterministic artifact.
fn emit_build_jobs(out: &mut String, ctx: &ReleaseRenderContext<'_>) -> Result<(), CiError> {
    let policy = ctx.policy;
    for job in &ctx.graph.ci_plan.targets {
        let runner = policy
            .runners
            .iter()
            .find(|mapping| {
                mapping.os == job.planned.policy.host_os
                    && mapping.arch == job.planned.policy.host_arch
            })
            .ok_or_else(|| fail("GitHub runner mapping missing for build host"))?;
        if job.planned.policy.strategy == BuildStrategy::CargoZigbuild {
            if let Some(provisioning) = &policy.cross_tools {
                provisioning.validate()?;
                if job.planned.policy.host_os != HostOs::Linux
                    || !matches!(
                        job.planned.policy.host_arch,
                        HostArch::X86_64 | HostArch::Aarch64
                    )
                {
                    return Err(fail(
                        "cross-tool provisioning supports only Linux x86-64/AArch64 hosts",
                    ));
                }
            } else if !runner.cargo_zigbuild || !runner.zig {
                return Err(fail(
                    "cross-build runner lacks preinstalled cargo-zigbuild or Zig",
                ));
            }
        }
        out.push_str("  ");
        out.push_str(&job.job_id);
        // Reusable builds wait for runtime identity; exact builds wait for
        // preflight directly.
        if ctx.is_runtime() {
            out.push_str(":\n    needs: resolve\n    runs-on: ");
        } else {
            out.push_str(":\n    needs: preflight\n    runs-on: ");
        }
        out.push_str(&yaml_scalar(&runner.label));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    continue-on-error: ");
        out.push_str(if job.required { "false\n" } else { "true\n" });
        out.push_str("    steps:\n");
        emit_source_checkout_step(out, ctx, ctx.frozen_sha_opt());
        emit_runtime_identity_download_step(out, ctx);
        out.push_str("      - name: Set up Rust toolchain\n        uses: ");
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
        // M003e: the pinned tool is installed before any Eggpack invocation.
        // Source verification runs here — after the toolchain the install
        // relies on, before any release work — instead of ahead of the tool.
        emit_tool_install_and_verify_steps(out, ctx);
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
        }
        let capture = RunnerCommand::CaptureBuild {
            contract: ctx.inputs.contract.clone(),
            release_plan: ctx.inputs.release_plan.clone(),
            build_bindings: ctx.inputs.build_bindings.clone(),
            target: job.planned.target.clone(),
            cargo_target_dir:
                "${{ runner.temp }}/eggpack/${{ github.run_id }}-${{ github.run_attempt }}".into(),
            output_dir: format!("./eggpack-build/{}", job.job_id),
        };
        let qual = ctx
            .graph
            .qualifications
            .iter()
            .find(|q| q.build_job_id == job.job_id)
            .ok_or_else(|| fail("qualification job missing for build target"))?;
        out.push_str(
            "      - name: Capture canonical build handoff\n        shell: bash\n        run: ",
        );
        out.push_str(&yaml_scalar(&capture.to_shell()));
        out.push('\n');
        out.push_str("      - name: Upload canonical build handoff\n        uses: ");
        out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&qual.build_handoff_name));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!("./eggpack-build/{}", job.job_id)));
        out.push_str("\n          if-no-files-found: error\n          retention-days: ");
        out.push_str(&policy.artifact_retention_days.to_string());
        out.push('\n');
    }
    Ok(())
}

/// Emit the qualification jobs: download the matching canonical build
/// artifact into a target-specific private directory, invoke
/// `_qualify-target` with all explicit inputs, and upload the complete
/// qualification directory.
fn emit_qualification_jobs(
    out: &mut String,
    ctx: &ReleaseRenderContext<'_>,
) -> Result<(), CiError> {
    let policy = ctx.policy;
    for qual in &ctx.graph.qualifications {
        let runner = policy
            .runners
            .iter()
            .find(|mapping| mapping.os == qual.host.os && mapping.arch == qual.host.arch)
            .ok_or_else(|| fail("runner mapping missing for qualification host"))?;
        let qemu_sysroot = policy
            .emulated_sysroots
            .as_ref()
            .and_then(|map| map.get(&qual.target).cloned());
        let qualify = RunnerCommand::QualifyTarget {
            contract: ctx.inputs.contract.clone(),
            release_plan: ctx.inputs.release_plan.clone(),
            build_bindings: ctx.inputs.build_bindings.clone(),
            qualification_bindings: ctx.inputs.qualification_bindings.clone(),
            target: qual.target.clone(),
            candidate_dir: format!("./eggpack-handoff/{}/{}", qual.build_job_id, CANDIDATES_DIR),
            build_handoff: format!(
                "./eggpack-handoff/{}/{}",
                qual.build_job_id, BUILD_HANDOFF_FILE
            ),
            output_dir: format!("./eggpack-qualification/{}", qual.build_job_id),
            qemu_sysroot,
        };
        out.push_str("  ");
        out.push_str(&qual.job_id);
        out.push_str(":\n    needs: ");
        emit_source_needs(
            out,
            std::slice::from_ref(&qual.build_job_id),
            ctx.staging_enabled,
            ctx.is_runtime(),
        );
        out.push_str("\n    runs-on: ");
        out.push_str(&yaml_scalar(&runner.label));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    continue-on-error: ");
        out.push_str(if qual.required { "false\n" } else { "true\n" });
        out.push_str("    steps:\n");
        emit_source_checkout_step(out, ctx, ctx.frozen_sha_opt());
        emit_runtime_identity_download_step(out, ctx);
        // M003e: install before any Eggpack invocation; verify right after.
        emit_tool_install_and_verify_steps(out, ctx);
        out.push_str("      - name: Download build handoff\n        uses: ");
        out.push_str(&yaml_scalar(&ctx.download_pin.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&qual.build_handoff_name));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!(
            "./eggpack-handoff/{}",
            qual.build_job_id
        )));
        out.push_str("\n      - name: Qualify target\n        shell: bash\n        run: ");
        out.push_str(&yaml_scalar(&qualify.to_shell()));
        out.push('\n');
        out.push_str("      - name: Upload qualification evidence\n        uses: ");
        out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!(
            "./eggpack-qualification/{}",
            qual.build_job_id
        )));
        out.push_str("\n          if-no-files-found: error\n          retention-days: ");
        out.push_str(&policy.artifact_retention_days.to_string());
        out.push('\n');
    }
    Ok(())
}

/// Emit the consumer validation jobs (M003d): one per target with a consumer
/// validator, running after core qualification and before the gate.
///
/// Each consumes the canonical qualification handoff (exact candidate bytes
/// already validated by Eggpack) and never rebuilds the binary. Graphs
/// without validators emit no jobs here (M003c byte-compatible).
fn emit_consumer_validation_jobs(
    out: &mut String,
    ctx: &ReleaseRenderContext<'_>,
) -> Result<(), CiError> {
    let policy = ctx.policy;
    if let Some(validator_map) = &ctx.validator_map {
        for qual in &ctx.graph.qualifications {
            if !ctx.graph.consumer_validators.contains_key(&qual.target) {
                continue;
            }
            let runner = policy
                .runners
                .iter()
                .find(|mapping| mapping.os == qual.host.os && mapping.arch == qual.host.arch)
                .ok_or_else(|| fail("runner mapping missing for qualification host"))?;
            let job_id = consumer_job_id(&qual.build_job_id);
            let validate = RunnerCommand::ValidateConsumer {
                consumer_validators: validator_map.clone(),
                target: qual.target.clone(),
                candidate_dir: format!(
                    "./eggpack-handoff/{}/{}",
                    qual.build_job_id, CANDIDATES_DIR
                ),
                build_handoff: format!(
                    "./eggpack-handoff/{}/{}",
                    qual.build_job_id, BUILD_HANDOFF_FILE
                ),
                evidence: format!(
                    "./eggpack-handoff/{}/{}",
                    qual.build_job_id, QUALIFICATION_EVIDENCE_FILE
                ),
                source_root: "${{ github.workspace }}".into(),
                output: format!(
                    "./eggpack-consumer/{}/{}",
                    qual.build_job_id, CONSUMER_EVIDENCE_FILE
                ),
            };
            out.push_str("  ");
            out.push_str(&job_id);
            out.push_str(":\n    needs: ");
            emit_source_needs(
                out,
                std::slice::from_ref(&qual.job_id),
                ctx.staging_enabled,
                ctx.is_runtime(),
            );
            out.push_str("\n    runs-on: ");
            out.push_str(&yaml_scalar(&runner.label));
            out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
            out.push_str(&policy.timeout_minutes.to_string());
            out.push_str("\n    continue-on-error: ");
            out.push_str(if qual.required { "false\n" } else { "true\n" });
            out.push_str("    steps:\n");
            emit_source_checkout_step(out, ctx, ctx.frozen_sha_opt());
            emit_runtime_identity_download_step(out, ctx);
            // M003e: install before any Eggpack invocation; verify right after.
            emit_tool_install_and_verify_steps(out, ctx);
            out.push_str("      - name: Download qualification handoff\n        uses: ");
            out.push_str(&yaml_scalar(&ctx.download_pin.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!(
                "./eggpack-handoff/{}",
                qual.build_job_id
            )));
            // M003e: the consumer evidence directory must exist before
            // `_validate-consumer` writes the per-target evidence file.
            out.push('\n');
            mkdir_step(
                out,
                "Create consumer evidence directory",
                &format!("./eggpack-consumer/{}", qual.build_job_id),
            );
            out.push_str("      - name: Validate exact candidate (consumer)\n        shell: bash\n        run: ");
            out.push_str(&yaml_scalar(&validate.to_shell()));
            out.push('\n');
            out.push_str("      - name: Upload consumer validation evidence\n        uses: ");
            out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&consumer_evidence_handoff_name(&qual.target)));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!(
                "./eggpack-consumer/{}",
                qual.build_job_id
            )));
            out.push_str("\n          if-no-files-found: error\n          retention-days: ");
            out.push_str(&policy.artifact_retention_days.to_string());
            out.push('\n');
        }
    }
    Ok(())
}

/// Emit the required gate job: one explicit download step per target into the
/// canonical `eggpack-inputs/<target>/` layout, then `_evaluate-gate` with
/// the exact CI plan, inputs directory, and outcome path.
fn emit_gate_job(out: &mut String, ctx: &ReleaseRenderContext<'_>) -> Result<(), CiError> {
    let policy = ctx.policy;
    let gate_cmd = RunnerCommand::EvaluateGate {
        ci_plan: ctx.inputs.ci_plan.clone(),
        inputs_dir: "./eggpack-inputs".into(),
        output: format!("./eggpack-gate/{}", GATE_OUTCOME_FILE),
    };
    out.push_str("  ");
    out.push_str(&ctx.graph.gate_job_id);
    out.push_str(":\n    needs: ");
    // Gate waits for the consumer validation job where one exists,
    // otherwise directly for the qualification job. Without validators
    // this is exactly the M003c dependency list.
    let gate_needs: Vec<String> = ctx
        .graph
        .qualifications
        .iter()
        .map(|q| {
            if ctx.graph.consumer_validators.contains_key(&q.target) {
                consumer_job_id(&q.build_job_id)
            } else {
                q.job_id.clone()
            }
        })
        .collect();
    let mut gate_needs = gate_needs;
    gate_needs.sort();
    if ctx.staging_enabled {
        emit_source_needs(out, &gate_needs, true, ctx.is_runtime());
    } else {
        out.push('[');
        out.push_str(&gate_needs.join(", "));
        out.push(']');
    }
    out.push_str("\n    runs-on: ");
    out.push_str(&yaml_scalar(&policy.preflight_runner));
    out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n");
    emit_source_checkout_step(out, ctx, ctx.frozen_sha_opt());
    emit_runtime_identity_download_step(out, ctx);
    // M003e: install before any Eggpack invocation; verify right after.
    emit_tool_install_and_verify_steps(out, ctx);
    for qual in &ctx.graph.qualifications {
        out.push_str("      - name: Download qualification ");
        out.push_str(&yaml_scalar(&qual.target));
        out.push_str("\n        uses: ");
        out.push_str(&yaml_scalar(&ctx.download_pin.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
        out.push('\n');
    }
    // Consumer evidence joins the same canonical per-target layout so
    // the gate consumes one complete directory per target.
    for qual in &ctx.graph.qualifications {
        if !ctx.graph.consumer_validators.contains_key(&qual.target) {
            continue;
        }
        out.push_str("      - name: Download consumer evidence ");
        out.push_str(&yaml_scalar(&qual.target));
        out.push_str("\n        uses: ");
        out.push_str(&yaml_scalar(&ctx.download_pin.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&consumer_evidence_handoff_name(&qual.target)));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
        out.push('\n');
    }
    // M003e: the gate outcome directory must exist before
    // `_evaluate-gate` writes the outcome file.
    mkdir_step(out, "Create gate outcome directory", "./eggpack-gate");
    out.push_str(
        "      - name: Evaluate required qualification gate\n        shell: bash\n        run: ",
    );
    out.push_str(&yaml_scalar(&gate_cmd.to_shell()));
    out.push('\n');
    Ok(())
}

/// Emit the aggregate job: same canonical per-target directories, explicit
/// contract / release-plan / CI-plan / inputs-dir / output-root / summary
/// arguments.
///
/// The finalized internal release is uploaded only when the aggregate outcome
/// is Complete (the CLI fails closed otherwise, so no partial release can be
/// uploaded).
fn emit_aggregate_job(out: &mut String, ctx: &ReleaseRenderContext<'_>) -> Result<(), CiError> {
    let policy = ctx.policy;
    let agg_cmd = RunnerCommand::Aggregate {
        contract: ctx.inputs.contract.clone(),
        release_plan: ctx.inputs.release_plan.clone(),
        ci_plan: ctx.inputs.ci_plan.clone(),
        inputs_dir: "./eggpack-inputs".into(),
        output_root: "./eggpack-finalized/root".into(),
        output: "./eggpack-finalized/summary.json".into(),
    };
    out.push_str("  ");
    out.push_str(&ctx.graph.aggregate.job_id);
    out.push_str(":\n    needs: ");
    emit_source_needs(
        out,
        std::slice::from_ref(&ctx.graph.gate_job_id),
        ctx.staging_enabled,
        ctx.is_runtime(),
    );
    out.push_str("\n    runs-on: ");
    out.push_str(&yaml_scalar(&policy.preflight_runner));
    out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n");
    emit_source_checkout_step(out, ctx, ctx.frozen_sha_opt());
    emit_runtime_identity_download_step(out, ctx);
    // M003e: install before any Eggpack invocation; verify right after.
    emit_tool_install_and_verify_steps(out, ctx);
    for qual in &ctx.graph.qualifications {
        out.push_str("      - name: Download qualification ");
        out.push_str(&yaml_scalar(&qual.target));
        out.push_str("\n        uses: ");
        out.push_str(&yaml_scalar(&ctx.download_pin.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
        out.push('\n');
    }
    for qual in &ctx.graph.qualifications {
        if !ctx.graph.consumer_validators.contains_key(&qual.target) {
            continue;
        }
        out.push_str("      - name: Download consumer evidence ");
        out.push_str(&yaml_scalar(&qual.target));
        out.push_str("\n        uses: ");
        out.push_str(&yaml_scalar(&ctx.download_pin.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&consumer_evidence_handoff_name(&qual.target)));
        out.push_str("\n          path: ");
        out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
        out.push('\n');
    }
    // M003e: the finalized release directory must exist before
    // `_aggregate` writes the summary and adjacent manifest.
    mkdir_step(
        out,
        "Create finalized release directory",
        "./eggpack-finalized",
    );
    out.push_str(
        "      - name: Aggregate and finalize release\n        shell: bash\n        run: ",
    );
    out.push_str(&yaml_scalar(&agg_cmd.to_shell()));
    out.push('\n');
    out.push_str("      - name: Upload internal finalized release\n        uses: ");
    out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
    out.push_str("\n        with:\n          name: ");
    out.push_str(&yaml_scalar(&ctx.graph.aggregate.final_handoff_name));
    out.push_str("\n          path: ./eggpack-finalized\n          if-no-files-found: error\n          retention-days: ");
    out.push_str(&policy.artifact_retention_days.to_string());
    out.push('\n');
    Ok(())
}

/// Emit the stage job (M003b): exactly one least-privilege draft staging job.
///
/// It runs only after aggregate, only for exact tags (tag push or explicit
/// dispatch input), consumes the exact aggregate artifact, calls only the
/// M003a draft-only commands, and uploads the bounded staging receipt. Every
/// prior job remains read-only; only stage has contents:write; no job
/// receives id-token:write; the token travels via environment only.
fn emit_stage_job(out: &mut String, ctx: &ReleaseRenderContext<'_>) -> Result<(), CiError> {
    let policy = ctx.policy;
    let Some(staging) = &ctx.graph.staging else {
        return Ok(());
    };
    let staging_policy = policy
        .staging
        .as_ref()
        .ok_or_else(|| fail("staging requested but no GitHub staging policy exists"))?;
    if staging.provider != StagingProvider::GitHubDraft {
        return Err(fail("unsupported staging provider"));
    }
    if staging_policy.owner.is_empty() || staging_policy.repository.is_empty() {
        return Err(fail("invalid GitHub staging repository identity"));
    }
    let prepare = RunnerCommand::PrepareStage {
        contract: staging_policy.inputs.contract.clone(),
        release_manifest: "./eggpack-finalized/release-manifest.json".into(),
        finalized_root: "./eggpack-finalized/root".into(),
        github_policy: staging_policy.inputs.github_policy.clone(),
        install_policy: staging_policy.inputs.install_policy.clone(),
        output_dir: "./eggpack-staging".into(),
        output_payload: "./eggpack-staging-payload.json".into(),
        // M003d installer presentation is explicit: the presentation
        // path comes from staging inputs and the source root is the
        // checked-out repository. Absent for M003c-compatible staging.
        installer_presentation: staging_policy.inputs.installer_presentation.clone(),
        source_root: staging_policy
            .inputs
            .installer_presentation
            .as_ref()
            .map(|_| "${{ github.workspace }}".to_owned()),
    };
    let stage_cmd = RunnerCommand::StageGithubDraft {
        payload: "./eggpack-staging-payload.json".into(),
        github_policy: staging_policy.inputs.github_policy.clone(),
        staging_dir: "./eggpack-staging".into(),
        output_receipt: "./eggpack-staging-receipt.json".into(),
    };
    out.push_str("  ");
    out.push_str(&staging.job_id);
    out.push_str(":\n    needs: ");
    emit_source_needs(
        out,
        std::slice::from_ref(&staging.aggregate_job_id),
        true,
        ctx.is_runtime(),
    );
    let stage_if = match staging_policy.tag_source {
        StagingTagSource::RefName => "github.event_name == 'push' && github.ref_type == 'tag'",
        StagingTagSource::DispatchInput => "github.event_name == 'workflow_dispatch'",
    };
    out.push_str("\n    if: ");
    out.push_str(stage_if);
    out.push_str("\n    runs-on: ");
    out.push_str(&yaml_scalar(&staging_policy.runner));
    if let Some(environment) = &staging_policy.environment {
        out.push_str("\n    environment:\n      name: ");
        out.push_str(&yaml_scalar(environment));
    }
    out.push_str("\n    permissions:\n      contents: write\n    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n");
    emit_tag_resolution_step(
        out,
        staging_policy.tag_source,
        policy.release_identity_mode == Some(ReleaseIdentityMode::VPrefixedStableSemver),
        Some(ctx.frozen_sha()),
        "confirm_tag_source",
    );
    checkout_snippet(out, policy, Some(ctx.frozen_sha()));
    emit_runtime_identity_download_step(out, ctx);
    // M003e: install before any Eggpack invocation; verify right after.
    emit_tool_install_and_verify_steps(out, ctx);
    out.push_str("      - name: Download finalized release\n        uses: ");
    out.push_str(&yaml_scalar(&ctx.download_pin.reference));
    out.push_str("\n        with:\n          name: ");
    out.push_str(&yaml_scalar(&staging.finalized_handoff_name));
    out.push_str("\n          path: ./eggpack-finalized\n          if-no-files-found: error\n");
    out.push_str("      - name: Prepare staging payload\n        shell: bash\n        run: ");
    out.push_str(&yaml_scalar(&prepare.to_shell()));
    out.push('\n');
    out.push_str("      - name: Stage GitHub draft release\n        shell: bash\n        env:\n          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n        run: ");
    out.push_str(&yaml_scalar(&stage_cmd.to_shell()));
    out.push('\n');
    out.push_str("      - name: Upload staging receipt\n        uses: ");
    out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
    out.push_str("\n        with:\n          name: ");
    out.push_str(&yaml_scalar(&staging.receipt_handoff_name));
    out.push_str("\n          path: ./eggpack-staging-receipt.json\n          if-no-files-found: error\n          retention-days: ");
    out.push_str(&staging_policy.receipt_retention_days.to_string());
    out.push('\n');
    Ok(())
}

/// Apply the static guards to a rendered release workflow.
///
/// The generated workflow must never publish, clobber via release CLIs,
/// mutate tags, or mint OIDC tokens. The only permitted network
/// acquisition is the finite M005 build-tool bootstrap: a `curl` line is
/// allowed only when it carries the HTTPS-only flags, bounded timeouts, and
/// the renderer-owned `$zig_url` variable that is assigned the fixed
/// `ziglang.org/download/` origin two lines above (M005 §8). Any other
/// `curl` invocation still fails closed.
fn validate_rendered_release_workflow(out: &str) -> Result<(), CiError> {
    if out.len() > MAX_WORKFLOW_BYTES {
        return Err(fail("rendered release workflow exceeds size bound"));
    }
    if out.contains("id-token: write") {
        return Err(fail("generated workflow must not request id-token write"));
    }
    for forbidden in [
        "gh release",
        "--clobber",
        "release --publish",
        "git tag ",
        "git push --tags",
    ] {
        if out.contains(forbidden) {
            return Err(fail(
                "generated workflow contains a forbidden release command",
            ));
        }
    }
    for line in out.lines() {
        if line.contains("curl ")
            && !(line.contains("curl --proto '=https'")
                && line.contains("--tlsv1.2")
                && line.contains("--proto-redir '=https'")
                && line.contains("--connect-timeout")
                && line.contains("--max-time")
                && line.contains("\"$zig_url\""))
        {
            return Err(fail(
                "generated workflow contains a forbidden curl invocation",
            ));
        }
    }
    if out.contains("curl --proto '=https'") && !out.contains("https://ziglang.org/download/") {
        return Err(fail(
            "generated workflow bootstrap curl lacks the fixed Zig origin",
        ));
    }
    if contains_unsafe_dispatch_expression_in_run(out) {
        return Err(fail(
            "generated run script interpolates an untrusted dispatch tag",
        ));
    }
    Ok(())
}

/// Deterministically render the release workflow for a projected graph.
///
/// The renderer validates first, then emits: the workflow header, the
/// preflight job, the runtime-identity job (reusable mode only), the build,
/// qualification, consumer-validation, gate, aggregate, and stage jobs.
/// Every emitter appends to one buffer, so the rendered bytes are a pure
/// function of the validated inputs.
fn render_release_github_inner(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
    runtime: Option<&RuntimeRender>,
) -> Result<String, CiError> {
    validate_release_render_policy(graph, policy)?;
    validate_release_identity_mode(policy, runtime)?;
    validate_staging_trigger_match(policy)?;
    let (tool, download_pin, inputs) = validate_release_tool_pins(policy)?;
    validate_emulated_runtime_policy(graph, policy)?;
    let validator_map = resolve_consumer_validator_map(graph, inputs)?;
    let ctx = ReleaseRenderContext {
        graph,
        policy,
        runtime,
        tool,
        download_pin,
        inputs,
        validator_map,
        staging_enabled: graph.staging.is_some(),
    };
    let mut out = String::new();
    emit_release_workflow_header(&mut out, &ctx);
    emit_preflight_job(&mut out, &ctx)?;
    emit_resolve_job(&mut out, &ctx)?;
    emit_build_jobs(&mut out, &ctx)?;
    emit_qualification_jobs(&mut out, &ctx)?;
    emit_consumer_validation_jobs(&mut out, &ctx)?;
    emit_gate_job(&mut out, &ctx)?;
    emit_aggregate_job(&mut out, &ctx)?;
    emit_stage_job(&mut out, &ctx)?;
    validate_rendered_release_workflow(&out)?;
    Ok(out)
}
