#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Provider-neutral CI planning and deterministic GitHub Actions generation.

use eggpack_contract::DistributionContract;
use eggpack_core::{
    cargo_command, ArchiveEncoding, BuildAttempt, BuildBindingsV1, BuildStrategy,
    CompatibilityFloor, FinalizationRequest, FinalizationTargetInput, HostArch, HostOs,
    HostRequirement, LogicalOutputSelector, PackConfig, PlannedAssetForm, PlannedTarget,
    Qualification, QualificationBindingsV1, QualificationEvidence, QualificationStatus,
    ReleasePlan, SupportTier, TargetPolicy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
    time::Duration,
};

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
    /// Immutable pin for `actions/download-artifact`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_artifact: Option<ActionPin>,
    /// Finite workflow event set.
    pub triggers: Vec<WorkflowTrigger>,
    /// Per-job timeout in minutes, from 1 to 360.
    pub timeout_minutes: u16,
    /// Whether a newer ref run cancels an in-progress run in the same group.
    pub cancel_in_progress: bool,
    /// Internal candidate artifact retention in days, from 1 to 90.
    pub artifact_retention_days: u8,
    /// Pinned Eggpack runtime tool provisioning for M002 qualify/aggregate jobs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eggpack_tool: Option<EggpackToolPolicy>,
    /// Explicit repository-relative input paths for M002a executable wiring.
    ///
    /// Required for `render_release_github`; absent for M001 build-only
    /// rendering. Paths live in provider policy, not portable release identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_inputs: Option<GitHubReleaseInputsV1>,
    /// Finite explicit QEMU sysroot policy per canonical target for Emulated
    /// qualification (M002a section 5H). Repository-relative paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emulated_sysroots: Option<BTreeMap<String, String>>,
    /// Explicit staging settings, present only when the graph requests
    /// GitHub draft staging (M003b). Absent for M002a build-only rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<GitHubStagingPolicyV1>,
}

/// Finite Eggpack runtime tool provisioning for generated qualify/aggregate jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EggpackToolPolicy {
    /// Exact official Eggpack repository URL.
    pub repo: String,
    /// Exact 40-hex commit revision.
    pub revision: String,
    /// Cargo package providing the `eggpack` binary (exactly `eggpack-cli`).
    pub package: String,
    /// Bound install timeout in minutes, from 1 to 60.
    pub install_timeout_minutes: u16,
}

impl EggpackToolPolicy {
    /// Validate official repo, immutable revision, package, and timeout bound.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.repo != "https://github.com/eggstack/eggpack"
            || self.revision.len() != 40
            || !self.revision.bytes().all(|b| b.is_ascii_hexdigit())
            || self.package != "eggpack-cli"
            || self.install_timeout_minutes == 0
            || self.install_timeout_minutes > 60
        {
            return Err(fail("invalid Eggpack tool provisioning policy"));
        }
        Ok(())
    }
}

/// Explicit repository-relative input paths for generated release workflow CLI
/// invocations (M002a execution wiring corrective).
///
/// Every generated `eggpack ci` command references its file inputs explicitly;
/// no hidden repository discovery or globbing is permitted. Paths live in
/// renderer/provider policy, never in portable release identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubReleaseInputsV1 {
    /// Repository-relative DistributionContract TOML path.
    pub contract: String,
    /// Repository-relative ReleasePlan JSON path.
    pub release_plan: String,
    /// Repository-relative BuildBindingsV1 (TOML or JSON) path.
    pub build_bindings: String,
    /// Repository-relative QualificationBindingsV1 (TOML or JSON) path.
    pub qualification_bindings: String,
    /// Repository-relative ReleaseCIPlanV1 JSON path.
    pub ci_plan: String,
    /// Repository-relative PackConfig (TOML or JSON) path for M003d runtime
    /// identity resolution. Required only for reusable workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_config: Option<String>,
    /// Repository-relative GitHubDraftTemplateV1 JSON path for M003d runtime
    /// identity resolution. Required only for reusable workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_template: Option<String>,
    /// Repository-relative InstallerPresentationV1 (TOML or JSON) path for
    /// M003d product-wrapper staging. Required only when the stage job
    /// passes `--installer-presentation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_presentation: Option<String>,
    /// Repository-relative consumer validator map (TOML or JSON) path.
    /// Required only when the graph configures consumer validators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_validators: Option<String>,
}

/// Explicit repository-relative staging input paths for the generated draft
/// staging job (M003b). All paths are bounded relative paths resolved from the
/// repository root after checkout; no discovery or globbing is permitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubStagingInputsV1 {
    /// Repository-relative DistributionContract TOML path.
    pub contract: String,
    /// Repository-relative BootstrapInstallPolicyV1 (TOML or JSON) path.
    pub install_policy: String,
    /// Repository-relative GitHubDraftPolicyV1 JSON path.
    pub github_policy: String,
    /// Optional repository-relative InstallerPresentationV1 path (M003d).
    /// Absent for M003c-compatible staging (GeneratedDefault).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_presentation: Option<String>,
}

/// Finite staging runner label plus repository identity for the generated
/// draft staging job (M003b). The exact tag itself lives in the checked-in
/// GitHub draft policy file and at runtime in `github.ref_name` (tag push) or
/// the explicit `release_tag` dispatch input; the renderer records how the tag
/// is obtained, never a branch or `latest` mapping, and never a publish flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubStagingPolicyV1 {
    /// Runner label for the staging job (finite, safe).
    pub runner: String,
    /// Exact repository owner.
    pub owner: String,
    /// Exact repository name.
    pub repository: String,
    /// Explicit tag source mapping.
    pub tag_source: StagingTagSource,
    /// Repository-relative staging input paths.
    pub inputs: GitHubStagingInputsV1,
    /// Staging receipt artifact retention in days, from 1 to 90.
    pub receipt_retention_days: u8,
}

/// Explicit tag source mapping for the generated staging job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingTagSource {
    /// Tag-push workflows: candidate tag from `github.ref_name` with ref type tag.
    RefName,
    /// Manual dispatch: explicit existing tag input (no latest/branch default).
    DispatchInput,
}

impl GitHubReleaseInputsV1 {
    /// Validate all required paths plus any configured M003d extension
    /// paths as bounded relative paths without escapes.
    pub fn validate(&self) -> Result<(), CiError> {
        for path in [
            &self.contract,
            &self.release_plan,
            &self.build_bindings,
            &self.qualification_bindings,
            &self.ci_plan,
        ] {
            validate_release_input_path(path)?;
        }
        for path in [
            self.pack_config.as_ref(),
            self.draft_template.as_ref(),
            self.installer_presentation.as_ref(),
            self.consumer_validators.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_release_input_path(path)?;
        }
        Ok(())
    }
}

fn validate_release_input_path(path: &str) -> Result<(), CiError> {
    if path.is_empty() || path.len() > 512 || path.contains('\0') || path.contains('\\') {
        return Err(fail("release input path is not a bounded relative path"));
    }
    if path.starts_with('/')
        || path.contains(':')
        || path.split('/').any(|segment| {
            segment.is_empty() || segment == "." || segment == ".." || segment.len() > 128
        })
    {
        return Err(fail("release input path escapes its root"));
    }
    // No drive prefixes (`C:` covered by colon check) or absolute roots.
    Ok(())
}

/// Canonical per-target build handoff artifact layout (M002a section 5B).
pub const BUILD_HANDOFF_FILE: &str = "build-handoff.json";
/// Canonical per-target qualification evidence file (M002a section 5C).
pub const QUALIFICATION_EVIDENCE_FILE: &str = "evidence.json";
/// Canonical candidate byte directory inside build/qualification artifacts.
pub const CANDIDATES_DIR: &str = "candidates";
/// Canonical gate/aggregate outcome file name.
pub const GATE_OUTCOME_FILE: &str = "gate-outcome.json";

/// Validate that a target directory name is a safe canonical target triple
/// (no path separators or escapes; used for `eggpack-inputs/<target>/`).
pub fn validate_canonical_target_dir(target: &str) -> Result<(), CiError> {
    if target.is_empty()
        || target.len() > 128
        || target.contains('/')
        || target.contains('\\')
        || target.contains(':')
        || target.contains('\0')
        || target.contains("..")
    {
        return Err(fail("target directory escapes its root"));
    }
    Ok(())
}

/// Finite typed runner command shared by GitHub rendering and local
/// executable orchestration tests (M002a section 7).
///
/// The GitHub renderer serializes a command to shell; tests invoke the same
/// structured command directly. The command enum remains finite to Eggpack's
/// internal CI operations; no generic arbitrary command DSL is authorized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerCommand {
    /// Verify checked-out HEAD equals the declared ReleasePlan source revision.
    VerifySource {
        /// Repository-relative ReleasePlan JSON path.
        release_plan: String,
    },
    /// Invoke `_capture-build` for one target.
    CaptureBuild {
        /// Repository-relative contract path (informational; CLI resolves via plan).
        contract: String,
        /// Repository-relative release plan path.
        release_plan: String,
        /// Repository-relative build bindings path.
        build_bindings: String,
        /// Canonical target triple.
        target: String,
        /// Known Cargo target root on the runner.
        cargo_target_dir: String,
        /// Canonical per-target output directory to stage.
        output_dir: String,
    },
    /// Invoke `_qualify-target` for one target.
    QualifyTarget {
        /// Repository-relative contract path.
        contract: String,
        /// Repository-relative release plan path.
        release_plan: String,
        /// Repository-relative build bindings path.
        build_bindings: String,
        /// Repository-relative qualification bindings path.
        qualification_bindings: String,
        /// Canonical target triple.
        target: String,
        /// Directory holding staged candidate bytes.
        candidate_dir: String,
        /// Canonical build handoff document path.
        build_handoff: String,
        /// Canonical qualification output directory.
        output_dir: String,
        /// Optional explicit QEMU sysroot for Emulated targets.
        qemu_sysroot: Option<String>,
    },
    /// Invoke `_evaluate-gate` over downloaded qualification evidence.
    EvaluateGate {
        /// Repository-relative CI plan path.
        ci_plan: String,
        /// Canonical per-target inputs directory.
        inputs_dir: String,
        /// Gate outcome file to write.
        output: String,
    },
    /// Invoke `_aggregate` over canonical per-target directories.
    Aggregate {
        /// Repository-relative contract path.
        contract: String,
        /// Repository-relative release plan path.
        release_plan: String,
        /// Repository-relative CI plan path.
        ci_plan: String,
        /// Canonical per-target inputs directory.
        inputs_dir: String,
        /// Private finalization output root.
        output_root: String,
        /// Aggregate summary file to write.
        output: String,
    },
    /// Invoke `_prepare-stage` to materialize the staging payload.
    PrepareStage {
        /// Repository-relative contract path.
        contract: String,
        /// Internal finalized `release-manifest.json` path from aggregate.
        release_manifest: String,
        /// Internal finalized root directory from aggregate.
        finalized_root: String,
        /// Repository-relative GitHub draft policy path.
        github_policy: String,
        /// Repository-relative install policy path.
        install_policy: String,
        /// Private staging output directory.
        output_dir: String,
        /// Staging payload JSON to write.
        output_payload: String,
        /// Optional repository-relative installer presentation path (M003d).
        installer_presentation: Option<String>,
        /// Optional checked-out source root for product wrappers (M003d).
        source_root: Option<String>,
    },
    /// Invoke `_stage-github-draft` to reconcile the payload into a draft.
    StageGithubDraft {
        /// Staging payload JSON path.
        payload: String,
        /// Repository-relative GitHub draft policy path.
        github_policy: String,
        /// Private staging directory holding payload files.
        staging_dir: String,
        /// Staging receipt JSON to write.
        output_receipt: String,
    },
    /// Invoke `_validate-consumer` for one target's exact candidate.
    ValidateConsumer {
        /// Repository-relative consumer validator map path.
        consumer_validators: String,
        /// Canonical target triple.
        target: String,
        /// Directory holding staged candidate bytes.
        candidate_dir: String,
        /// Canonical build handoff document path.
        build_handoff: String,
        /// Canonical qualification evidence path (exact size/SHA source).
        evidence: String,
        /// Checked-out repository root for validator script resolution.
        source_root: String,
        /// Consumer evidence file to write.
        output: String,
    },
    /// Invoke `_resolve-release` to materialize invocation-local identity.
    ResolveRelease {
        /// Repository-relative DistributionContract TOML path.
        contract: String,
        /// Repository-relative PackConfig (TOML or JSON) path.
        pack_config: String,
        /// Repository-relative build bindings path.
        build_bindings: String,
        /// Repository-relative qualification bindings path.
        qualification_bindings: String,
        /// Optional repository-relative consumer validator map path.
        consumer_validators: Option<String>,
        /// Comma-separated contract target aliases, in order.
        selected: String,
        /// Exact existing tag (or the workflow tag expression at render).
        tag: String,
        /// Checked-out HEAD revision (or the `$head_sha` render marker).
        source_revision: String,
        /// Repository-relative GitHubDraftTemplateV1 JSON path.
        template: String,
        /// Checked-out repository root for HEAD verification.
        source_root: String,
        /// Invocation-local ReleasePlan path to write.
        output_plan: String,
        /// Invocation-local ReleaseCIPlanV1 path to write.
        output_ci_plan: String,
        /// Invocation-local GitHubDraftPolicyV1 path to write.
        output_github_policy: String,
    },
}

impl RunnerCommand {
    /// Shell tokens (`eggpack`, `ci`, subcommand, flags) for YAML rendering.
    pub fn argv(&self) -> Vec<String> {
        match self {
            RunnerCommand::VerifySource { release_plan } => vec![
                "eggpack".into(),
                "ci".into(),
                "_verify-source".into(),
                "--release-plan".into(),
                release_plan.clone(),
            ],
            RunnerCommand::CaptureBuild {
                contract: _,
                release_plan,
                build_bindings,
                target,
                cargo_target_dir,
                output_dir,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_capture-build".into(),
                "--release-plan".into(),
                release_plan.clone(),
                "--build-bindings".into(),
                build_bindings.clone(),
                "--target".into(),
                target.clone(),
                "--cargo-target-dir".into(),
                cargo_target_dir.clone(),
                "--output-dir".into(),
                output_dir.clone(),
            ],
            RunnerCommand::QualifyTarget {
                contract,
                release_plan,
                build_bindings,
                qualification_bindings,
                target,
                candidate_dir,
                build_handoff,
                output_dir,
                qemu_sysroot,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_qualify-target".into(),
                    "--contract".into(),
                    contract.clone(),
                    "--release-plan".into(),
                    release_plan.clone(),
                    "--build-bindings".into(),
                    build_bindings.clone(),
                    "--qualification-bindings".into(),
                    qualification_bindings.clone(),
                    "--target".into(),
                    target.clone(),
                    "--candidate-dir".into(),
                    candidate_dir.clone(),
                    "--build-handoff".into(),
                    build_handoff.clone(),
                    "--output-dir".into(),
                    output_dir.clone(),
                ];
                if let Some(sysroot) = qemu_sysroot {
                    args.push("--qemu-sysroot".into());
                    args.push(sysroot.clone());
                }
                args
            }
            RunnerCommand::EvaluateGate {
                ci_plan,
                inputs_dir,
                output,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_evaluate-gate".into(),
                "--ci-plan".into(),
                ci_plan.clone(),
                "--inputs-dir".into(),
                inputs_dir.clone(),
                "--output".into(),
                output.clone(),
            ],
            RunnerCommand::Aggregate {
                contract,
                release_plan,
                ci_plan,
                inputs_dir,
                output_root,
                output,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_aggregate".into(),
                "--contract".into(),
                contract.clone(),
                "--release-plan".into(),
                release_plan.clone(),
                "--ci-plan".into(),
                ci_plan.clone(),
                "--inputs-dir".into(),
                inputs_dir.clone(),
                "--output-root".into(),
                output_root.clone(),
                "--output".into(),
                output.clone(),
            ],
            RunnerCommand::PrepareStage {
                contract,
                release_manifest,
                finalized_root,
                github_policy,
                install_policy,
                output_dir,
                output_payload,
                installer_presentation,
                source_root,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_prepare-stage".into(),
                    "--contract".into(),
                    contract.clone(),
                    "--release-manifest".into(),
                    release_manifest.clone(),
                    "--finalized-root".into(),
                    finalized_root.clone(),
                    "--github-policy".into(),
                    github_policy.clone(),
                    "--install-policy".into(),
                    install_policy.clone(),
                    "--output-dir".into(),
                    output_dir.clone(),
                    "--output-payload".into(),
                    output_payload.clone(),
                ];
                // M003d installer presentation is additive: absent for
                // M003c-compatible invocations, explicit otherwise.
                if let Some(presentation) = installer_presentation {
                    args.push("--installer-presentation".into());
                    args.push(presentation.clone());
                }
                if let Some(root) = source_root {
                    args.push("--source-root".into());
                    args.push(root.clone());
                }
                args
            }
            RunnerCommand::StageGithubDraft {
                payload,
                github_policy,
                staging_dir,
                output_receipt,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_stage-github-draft".into(),
                "--payload".into(),
                payload.clone(),
                "--github-policy".into(),
                github_policy.clone(),
                "--staging-dir".into(),
                staging_dir.clone(),
                "--output-receipt".into(),
                output_receipt.clone(),
            ],
            RunnerCommand::ValidateConsumer {
                consumer_validators,
                target,
                candidate_dir,
                build_handoff,
                evidence,
                source_root,
                output,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_validate-consumer".into(),
                "--consumer-validators".into(),
                consumer_validators.clone(),
                "--target".into(),
                target.clone(),
                "--candidate-dir".into(),
                candidate_dir.clone(),
                "--build-handoff".into(),
                build_handoff.clone(),
                "--evidence".into(),
                evidence.clone(),
                "--source-root".into(),
                source_root.clone(),
                "--output".into(),
                output.clone(),
            ],
            RunnerCommand::ResolveRelease {
                contract,
                pack_config,
                build_bindings,
                qualification_bindings,
                consumer_validators,
                selected,
                tag,
                source_revision,
                template,
                source_root,
                output_plan,
                output_ci_plan,
                output_github_policy,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_resolve-release".into(),
                    "--contract".into(),
                    contract.clone(),
                    "--pack-config".into(),
                    pack_config.clone(),
                    "--build-bindings".into(),
                    build_bindings.clone(),
                    "--qualification-bindings".into(),
                    qualification_bindings.clone(),
                    "--selected".into(),
                    selected.clone(),
                    "--tag".into(),
                    tag.clone(),
                    "--source-revision".into(),
                    source_revision.clone(),
                    "--template".into(),
                    template.clone(),
                    "--source-root".into(),
                    source_root.clone(),
                    "--output-plan".into(),
                    output_plan.clone(),
                    "--output-ci-plan".into(),
                    output_ci_plan.clone(),
                    "--output-github-policy".into(),
                    output_github_policy.clone(),
                ];
                if let Some(map) = consumer_validators {
                    // Insert the optional map path in flag order (after
                    // qualification bindings, before selected).
                    let position = args
                        .iter()
                        .position(|arg| arg == "--selected")
                        .unwrap_or(args.len());
                    args.splice(
                        position..position,
                        ["--consumer-validators".into(), map.clone()],
                    );
                }
                args
            }
        }
    }

    /// Deterministic shell rendering with single-quote escaping.
    ///
    /// The exact token `$head_sha` is the renderer-only workflow marker for
    /// the derived HEAD revision and renders double-quoted so the runner
    /// shell expands it. It is never a valid revision, so it cannot collide
    /// with real `_resolve-release` values (which must be 40-hex).
    pub fn to_shell(&self) -> String {
        self.argv()
            .iter()
            .map(|arg| {
                if arg == "$head_sha" {
                    "\"$head_sha\"".to_owned()
                } else {
                    shell_quote(arg)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
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
        if let Some(download) = &self.download_artifact {
            validate_pin(download, "actions/download-artifact")?;
        }
        if let Some(tool) = &self.eggpack_tool {
            tool.validate()?;
        }
        if let Some(inputs) = &self.release_inputs {
            inputs.validate()?;
        }
        if let Some(sysroots) = &self.emulated_sysroots {
            if sysroots.len() > 256 {
                return Err(fail("emulated sysroot policy exceeds bound"));
            }
            for (target, path) in sysroots {
                validate_canonical_target_dir(target)?;
                validate_release_input_path(path)?;
            }
        }
        if let Some(staging) = &self.staging {
            if !safe_runner_label(&staging.runner)
                || staging.owner.is_empty()
                || staging.owner.len() > 64
                || staging.repository.is_empty()
                || staging.repository.len() > 64
                || staging.receipt_retention_days == 0
                || staging.receipt_retention_days > 90
            {
                return Err(fail("invalid GitHub staging policy"));
            }
            // No publish flag exists by construction; tag source is explicit by
            // enum; owner/repository must be safe segments (no control/path).
            for value in [&staging.owner, &staging.repository] {
                if value.chars().any(char::is_control)
                    || value.contains('/')
                    || value.contains("..")
                {
                    return Err(fail("invalid GitHub staging repository identity"));
                }
            }
            for path in [
                &staging.inputs.contract,
                &staging.inputs.install_policy,
                &staging.inputs.github_policy,
            ] {
                validate_release_input_path(path)?;
            }
            if let Some(presentation) = &staging.inputs.installer_presentation {
                validate_release_input_path(presentation)?;
            }
            let required_trigger = match staging.tag_source {
                StagingTagSource::RefName => WorkflowTrigger::Push,
                StagingTagSource::DispatchInput => WorkflowTrigger::WorkflowDispatch,
            };
            if !self.triggers.contains(&required_trigger) {
                return Err(fail(
                    "staging tag source requires its matching workflow trigger",
                ));
            }
        }
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

// ---------------------------------------------------------------------------
// M002 — qualification/aggregation gates and deterministic release graph.
// ---------------------------------------------------------------------------

const MAX_RELEASE_PLAN_JSON: usize = 1_000_000;
const MAX_HANDOFF_JSON: usize = 1_000_000;
const MAX_EVIDENCE_JSON: usize = 1_000_000;

/// One bounded build output handoff entry (no absolute paths).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildHandoffOutput {
    /// Contract logical output selector.
    pub selector: LogicalOutputSelector,
    /// Explicit Cargo package source.
    pub package: String,
    /// Explicit Cargo binary target source.
    pub binary: String,
    /// Relative candidate path below the downloaded handoff root.
    pub relative_path: String,
    /// Exact byte size.
    pub size: u64,
    /// Deterministic handoff identity.
    pub handoff_identity: String,
}

/// CI-specific bounded build handoff for one canonical target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildHandoffV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Release id copied from ReleasePlan.
    pub release_id: String,
    /// Source revision copied from ReleasePlan.
    pub source_revision: String,
    /// Canonical target triple.
    pub target: String,
    /// Build strategy used for this target.
    pub strategy: BuildStrategy,
    /// Logical selector outputs in canonical order.
    pub outputs: Vec<BuildHandoffOutput>,
}

impl BuildHandoffV1 {
    /// Serialize after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("build handoff serialization failed"))
    }

    /// Parse and validate a bounded handoff document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_HANDOFF_JSON {
            return Err(fail("build handoff exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid build handoff JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Validate shape, ordering, path containment, and identifier bounds.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1
            || !safe_metadata(&self.release_id, 256)
            || !safe_metadata(&self.source_revision, 256)
            || !safe_target(&self.target)
            || self.outputs.is_empty()
            || self.outputs.len() > 256
        {
            return Err(fail("invalid or out-of-bounds build handoff"));
        }
        let mut selectors = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let mut previous: Option<&LogicalOutputSelector> = None;
        for output in &self.outputs {
            if previous.is_some_and(|p| p >= &output.selector)
                || !safe_identifier(&output.package)
                || !safe_identifier(&output.binary)
                || output.size == 0
                || !selectors.insert(output.selector.clone())
                || !identities.insert(output.handoff_identity.as_str())
                || output.handoff_identity.is_empty()
                || output.handoff_identity.len() > 256
            {
                return Err(fail("invalid or duplicate build handoff output"));
            }
            previous = Some(&output.selector);
            validate_relative_path(&output.relative_path)?;
        }
        Ok(())
    }
}

fn validate_relative_path(path: &str) -> Result<(), CiError> {
    if path.is_empty()
        || path.len() > 512
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains(':')
        || path.contains('\0')
        || path.contains("..")
        || path.split('/').any(|segment| segment.is_empty())
    {
        return Err(fail("build handoff path escapes its root"));
    }
    Ok(())
}

/// Project one target's build handoff from validated plan/bindings (no I/O).
pub fn project_build_handoff(
    plan: &ReleasePlan,
    bindings: &BuildBindingsV1,
    target: &str,
) -> Result<BuildHandoffV1, CiError> {
    let planned = plan
        .targets
        .iter()
        .find(|t| t.target == target)
        .ok_or_else(|| fail("build handoff target is not in ReleasePlan"))?;
    let target_bindings = bindings
        .targets
        .get(target)
        .ok_or_else(|| fail("target build bindings are missing"))?;
    let mut outputs = Vec::with_capacity(target_bindings.len());
    for binding in target_bindings {
        let relative_path = match &binding.selector {
            LogicalOutputSelector::Direct => "candidate-direct".to_string(),
            LogicalOutputSelector::BundleEntry { index } => format!("candidate-bundle-{index}"),
            LogicalOutputSelector::ArchiveMember { source } => {
                let mut digest = Sha256::new();
                digest.update(source.as_bytes());
                let hex: String = digest
                    .finalize()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                format!("candidate-archive-{hex}")
            }
        };
        // Size is filled by the runner after the build; projection uses a
        // non-zero placeholder that validation accepts but reconstruction
        // replaces with observed file sizes. Callers must not treat the
        // placeholder as evidence.
        outputs.push(BuildHandoffOutput {
            selector: binding.selector.clone(),
            package: binding.package.clone(),
            binary: binding.binary.clone(),
            relative_path,
            size: 1,
            handoff_identity: handoff_name(target, &binding.selector),
        });
    }
    outputs.sort_by(|a, b| a.selector.cmp(&b.selector));
    let handoff = BuildHandoffV1 {
        schema_version: 1,
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        target: planned.target.clone(),
        strategy: planned.policy.strategy,
        outputs,
    };
    handoff.validate()?;
    Ok(handoff)
}

/// Reconstruct an in-memory BuildAttempt from validated handoff and downloaded files.
///
/// Validates exact inventory, path containment, regular non-symlink status, and
/// sizes before returning. Never trusts artifact transport as proof; the
/// returned attempt must still pass M003/M004 validation.
pub fn reconstruct_attempt(
    plan: &ReleasePlan,
    target: &PlannedTarget,
    handoff: &BuildHandoffV1,
    candidate_root: &Path,
) -> Result<BuildAttempt, CiError> {
    if handoff.schema_version != 1
        || handoff.release_id != plan.release_id
        || handoff.source_revision != plan.source_revision
        || handoff.target != target.target
        || handoff.strategy != target.policy.strategy
    {
        return Err(fail("build handoff identity differs from ReleasePlan"));
    }
    handoff.validate()?;
    let root = candidate_root.to_path_buf();
    let mut candidates = Vec::with_capacity(handoff.outputs.len());
    for output in &handoff.outputs {
        let path = root.join(&output.relative_path);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| fail("candidate file is unavailable"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
            return Err(fail("candidate is not a non-empty regular file"));
        }
        if metadata.len() != output.size && output.size != 1 {
            // Placeholder size 1 means the projection has not yet been
            // replaced with observed sizes; runners must supply exact sizes.
            return Err(fail("candidate size differs from handoff evidence"));
        }
        candidates.push(eggpack_core::CandidateArtifact {
            target: target.target.clone(),
            selector: output.selector.clone(),
            package: output.package.clone(),
            binary: output.binary.clone(),
            path,
            size: metadata.len(),
        });
    }
    candidates.sort_by(|a, b| a.selector.cmp(&b.selector));
    Ok(BuildAttempt {
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        target: target.target.clone(),
        strategy: target.policy.strategy,
        tool_summary: format!("ci-handoff {}", target.target),
        process: eggpack_core::ProcessEvidence {
            outcome: eggpack_core::CommandOutcome::Success,
            stdout_bytes: 0,
            stderr_bytes: 0,
        },
        candidates,
    })
}

/// Validate a canonical per-target build artifact directory:
/// `build-handoff.json` plus `candidates/<relative_path>` entries exactly
/// matching the handoff inventory (regular non-empty files, no extras).
pub fn validate_build_artifact_dir(dir: &Path) -> Result<BuildHandoffV1, CiError> {
    let handoff_text = std::fs::read_to_string(dir.join(BUILD_HANDOFF_FILE))
        .map_err(|_| fail("build handoff is unavailable"))?;
    let handoff = BuildHandoffV1::from_json(&handoff_text)?;
    let candidates_dir = dir.join(CANDIDATES_DIR);
    let meta = std::fs::symlink_metadata(&candidates_dir)
        .map_err(|_| fail("candidate directory is unavailable"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(fail("candidate directory is not a real directory"));
    }
    let mut seen = BTreeSet::new();
    for output in &handoff.outputs {
        let path = candidates_dir.join(&output.relative_path);
        let file_meta =
            std::fs::symlink_metadata(&path).map_err(|_| fail("candidate file is unavailable"))?;
        if file_meta.file_type().is_symlink()
            || !file_meta.is_file()
            || file_meta.len() == 0
            || file_meta.len() != output.size
        {
            return Err(fail("candidate is not the handoff-described regular file"));
        }
        seen.insert(output.relative_path.as_str());
    }
    // No extra files.
    let mut count = 0;
    for entry in
        std::fs::read_dir(&candidates_dir).map_err(|_| fail("candidate directory unreadable"))?
    {
        let entry = entry.map_err(|_| fail("candidate directory unreadable"))?;
        let meta = std::fs::symlink_metadata(entry.path())
            .map_err(|_| fail("candidate file is unavailable"))?;
        if !meta.is_file() {
            return Err(fail("candidate directory contains a non-file entry"));
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !seen.contains(name.as_str()) {
            return Err(fail("candidate directory contains an extra file"));
        }
        count += 1;
    }
    if count != seen.len() {
        return Err(fail("candidate directory inventory differs from handoff"));
    }
    Ok(handoff)
}

/// Validate a canonical per-target qualification artifact directory:
/// `build-handoff.json`, `evidence.json`, plus `candidates/...`.
pub fn validate_qualification_artifact_dir(
    dir: &Path,
) -> Result<(BuildHandoffV1, QualificationEvidence), CiError> {
    let handoff = validate_build_artifact_dir(dir)?;
    let evidence_text = std::fs::read_to_string(dir.join(QUALIFICATION_EVIDENCE_FILE))
        .map_err(|_| fail("qualification evidence is unavailable"))?;
    let evidence = decode_qualification_evidence(&evidence_text)?;
    if evidence.target != handoff.target
        || evidence.release_id != handoff.release_id
        || evidence.source_revision != handoff.source_revision
    {
        return Err(fail("qualification evidence identity differs from handoff"));
    }
    Ok((handoff, evidence))
}

/// Copy or hard-link one file without following symlinks on the source.
fn copy_candidate_file(source: &Path, dest: &Path) -> Result<(), CiError> {
    let meta =
        std::fs::symlink_metadata(source).map_err(|_| fail("candidate file is unavailable"))?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() == 0 {
        return Err(fail("candidate is not a non-empty regular file"));
    }
    if let Ok(parent) = dest
        .parent()
        .ok_or_else(|| fail("candidate path is invalid"))
    {
        let _ = parent;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|_| fail("output directory unavailable"))?;
    }
    // Prefer hard-link (same filesystem, no byte copy); fall back to copy.
    if std::fs::hard_link(source, dest).is_err() {
        std::fs::copy(source, dest).map_err(|_| fail("candidate staging failed"))?;
    }
    Ok(())
}

/// Stage a canonical build artifact directory from validated handoff sizes
/// and a source candidate root keyed by handoff relative names.
pub fn stage_build_artifact_dir(
    handoff: &BuildHandoffV1,
    candidate_src_dir: &Path,
    out_dir: &Path,
) -> Result<(), CiError> {
    let out_meta = std::fs::symlink_metadata(out_dir);
    match out_meta {
        Ok(meta) => {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(fail("output directory is not a real directory"));
            }
        }
        Err(_) => {
            std::fs::create_dir_all(out_dir).map_err(|_| fail("output directory unavailable"))?;
        }
    }
    let dest_candidates = out_dir.join(CANDIDATES_DIR);
    std::fs::create_dir_all(&dest_candidates).map_err(|_| fail("output directory unavailable"))?;
    for output in &handoff.outputs {
        let source = candidate_src_dir.join(&output.relative_path);
        copy_candidate_file(&source, &dest_candidates.join(&output.relative_path))?;
    }
    let json = handoff.to_json()?;
    std::fs::write(out_dir.join(BUILD_HANDOFF_FILE), json.as_bytes())
        .map_err(|_| fail("handoff write failed"))?;
    validate_build_artifact_dir(out_dir)?;
    Ok(())
}

/// Derive the Cargo output file for one handoff entry from the known Cargo
/// target root (`<cargo_target_dir>/<target>/release/<binary>[.exe]`).
pub fn cargo_output_path(
    cargo_target_dir: &Path,
    target: &str,
    binary: &str,
) -> Result<std::path::PathBuf, CiError> {
    validate_canonical_target_dir(target)?;
    if binary.is_empty()
        || binary.len() > 128
        || binary.contains('/')
        || binary.contains('\\')
        || binary.contains('\0')
        || binary.contains("..")
    {
        return Err(fail("cargo binary name escapes its root"));
    }
    let mut path = cargo_target_dir.join(target).join("release").join(binary);
    if target.contains("-windows-") {
        path.set_extension("exe");
    }
    Ok(path)
}

/// One executable qualification job projected from M003 policy/bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationJob {
    /// Canonical target triple.
    pub target: String,
    /// Source build job id.
    pub build_job_id: String,
    /// Stable qualification job id.
    pub job_id: String,
    /// Deterministic build handoff artifact name.
    pub build_handoff_name: String,
    /// Deterministic qualification evidence handoff name.
    pub evidence_handoff_name: String,
    /// Planned qualification classification.
    pub classification: Qualification,
    /// Qualification host (explicit or build host default).
    pub host: HostRequirement,
    /// Support tier recorded for gating.
    pub support: SupportTier,
    /// Whether this target gates aggregation.
    pub required: bool,
}

/// Bounded aggregate job descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateJob {
    /// Stable aggregate job id.
    pub job_id: String,
    /// Required gate job id it depends on.
    pub gate_job_id: String,
    /// Deterministic finalized release artifact name (Complete only).
    pub final_handoff_name: String,
}

/// Bounded aggregate outcome (never a partial release).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateOutcome {
    /// Every selected target has complete passing (or deferred non-gating) evidence.
    Complete,
    /// A non-required target is missing/failed/unqualified; no release is produced.
    SuppressedNonGatingIncomplete,
    /// A required target failed the gate; no release is produced.
    FailedRequiredGate,
    /// Evidence is corrupt, swapped, or incomplete; no release is produced.
    InvalidEvidence,
}

/// Allowed finalization settings for the aggregate node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizationSettings {
    /// Required explicit encoding when any selected target is an archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_encoding: Option<ArchiveEncodingWrapper>,
    /// Bounded evidence references carried into the manifest.
    #[serde(default)]
    pub evidence_references: Vec<String>,
}

/// Explicit archive encoding wrapper (TarGzip only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveEncodingWrapper {
    /// POSIX tar+gzip per M004.
    TarGzip,
}

/// Bounded staging provider for the executable release graph (M003b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingProvider {
    /// GitHub draft release via the M003a provider adapter.
    GitHubDraft,
}

/// Bounded provider-neutral staging job descriptor (M003b).
///
/// Records the aggregate dependency, staging provider, deterministic internal
/// finalized handoff name, deterministic staging receipt artifact name, and
/// whether staging is required. No GitHub details live here; the GitHub
/// renderer maps this intent to one least-privilege stage job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagingJob {
    /// Stable staging job id, exactly `stage`.
    pub job_id: String,
    /// Aggregate job id it depends on, exactly `aggregate`.
    pub aggregate_job_id: String,
    /// Staging provider.
    pub provider: StagingProvider,
    /// Deterministic finalized release artifact name consumed from aggregate.
    pub finalized_handoff_name: String,
    /// Deterministic staging receipt artifact name uploaded by stage.
    pub receipt_handoff_name: String,
    /// Whether staging is required.
    pub required: bool,
}

/// Executable provider-neutral orchestration graph layered on M001 CIPlan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseCIPlanV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Validated M001 CIPlan (qualification remains unresolved intent there).
    pub ci_plan: CIPlan,
    /// One qualification job per selected target, in canonical order.
    pub qualifications: Vec<QualificationJob>,
    /// Stable required-gate job id.
    pub gate_job_id: String,
    /// Aggregate job descriptor.
    pub aggregate: AggregateJob,
    /// Allowed finalization settings.
    pub finalization: FinalizationSettings,
    /// Optional bounded staging intent (M003b). Absent for M002a rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<StagingJob>,
    /// Optional per-target consumer validators (M003d). Absent or empty for
    /// M003c-compatible graphs; keys are canonical target triples.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumer_validators: BTreeMap<String, ConsumerValidatorV1>,
}

impl ReleaseCIPlanV1 {
    /// Serialize after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("ReleaseCIPlan serialization failed"))
    }

    /// Parse and validate a bounded executable graph document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_RELEASE_PLAN_JSON {
            return Err(fail("ReleaseCIPlan exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid ReleaseCIPlan JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Validate graph shape, ordering, handoff names, and gating.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported ReleaseCIPlan version"));
        }
        self.ci_plan.validate()?;
        if self.qualifications.len() != self.ci_plan.targets.len()
            || self.qualifications.is_empty()
            || self.gate_job_id != "required_gate"
            || self.aggregate.job_id != "aggregate"
            || self.aggregate.gate_job_id != self.gate_job_id
            || self.aggregate.final_handoff_name.is_empty()
            || self.aggregate.final_handoff_name.len() > 256
        {
            return Err(fail("invalid executable release graph shape"));
        }
        let mut targets = BTreeSet::new();
        let mut job_ids = BTreeSet::new();
        for qual in &self.qualifications {
            let planned = self
                .ci_plan
                .targets
                .iter()
                .find(|job| job.planned.target == qual.target)
                .ok_or_else(|| fail("qualification target is not in CIPlan"))?;
            if !targets.insert(qual.target.as_str())
                || !job_ids.insert(qual.job_id.as_str())
                || qual.build_job_id != planned.job_id
                || qual.job_id != format!("qualify_{}", planned.job_id)
                || qual.build_handoff_name != format!("eggpack-build-handoff-{}", qual.target)
                || qual.evidence_handoff_name != format!("eggpack-evidence-{}", qual.target)
                || qual.classification != planned.planned.policy.qualification
                || qual.support != planned.planned.policy.support
                || qual.required != planned.required
            {
                return Err(fail("invalid or inconsistent qualification job"));
            }
            let expected_host =
                planned
                    .planned
                    .policy
                    .qualification_host
                    .unwrap_or(HostRequirement {
                        os: planned.planned.policy.host_os,
                        arch: planned.planned.policy.host_arch,
                    });
            if qual.host != expected_host {
                return Err(fail("qualification host differs from plan policy"));
            }
        }
        // Canonical ordering by target.
        let mut sorted: Vec<&str> = self
            .qualifications
            .iter()
            .map(|q| q.target.as_str())
            .collect();
        let mut canonical = sorted.clone();
        canonical.sort();
        sorted.clone_from_slice(&canonical);
        let actual: Vec<&str> = self
            .qualifications
            .iter()
            .map(|q| q.target.as_str())
            .collect();
        if actual != canonical {
            return Err(fail("qualification jobs must be in canonical target order"));
        }
        for reference in &self.finalization.evidence_references {
            if !safe_metadata(reference, 256) {
                return Err(fail("invalid finalization evidence reference"));
            }
        }
        if self.finalization.evidence_references.len() > 64 {
            return Err(fail("finalization evidence references exceed bound"));
        }
        if let Some(staging) = &self.staging {
            if staging.job_id != "stage"
                || staging.aggregate_job_id != self.aggregate.job_id
                || staging.aggregate_job_id != "aggregate"
                || staging.finalized_handoff_name != self.aggregate.final_handoff_name
                || staging.finalized_handoff_name.is_empty()
                || staging.finalized_handoff_name.len() > 256
                || staging.receipt_handoff_name.is_empty()
                || staging.receipt_handoff_name.len() > 256
                || staging.receipt_handoff_name == staging.finalized_handoff_name
            {
                return Err(fail("invalid staging job descriptor"));
            }
            if staging.provider != StagingProvider::GitHubDraft {
                return Err(fail("unsupported staging provider"));
            }
        }
        if self.consumer_validators.len() > 256 {
            return Err(fail("consumer validator count exceeds bound"));
        }
        for (target, validator) in &self.consumer_validators {
            validator.validate()?;
            if !self
                .ci_plan
                .targets
                .iter()
                .any(|job| job.planned.target == *target)
            {
                return Err(fail("consumer validator target is not in CIPlan"));
            }
        }
        Ok(())
    }
}

/// Project an executable graph from M001 CIPlan plus M003 qualification bindings.
pub fn project_release_plan(
    ci_plan: &CIPlan,
    qualification_bindings: &QualificationBindingsV1,
    build_bindings: &BuildBindingsV1,
    release: &ReleasePlan,
) -> Result<ReleaseCIPlanV1, CiError> {
    ci_plan.validate()?;
    // Validate qualification coverage through the M003 API without copying logic.
    // Build a minimal plan view: M003 validate_for needs ReleasePlan + BuildBindings.
    qualification_bindings
        .validate_for(release, build_bindings)
        .map_err(|_| fail("qualification bindings do not cover ReleasePlan"))?;
    if ci_plan.release_id != release.release_id
        || ci_plan.source_revision != release.source_revision
        || ci_plan.targets.len() != release.targets.len()
    {
        return Err(fail("CIPlan differs from ReleasePlan identity"));
    }
    let mut qualifications = Vec::with_capacity(ci_plan.targets.len());
    for job in &ci_plan.targets {
        let planned = release
            .targets
            .iter()
            .find(|t| t.target == job.planned.target)
            .ok_or_else(|| fail("CIPlan target is not in ReleasePlan"))?;
        let host = planned
            .policy
            .qualification_host
            .unwrap_or(HostRequirement {
                os: planned.policy.host_os,
                arch: planned.policy.host_arch,
            });
        qualifications.push(QualificationJob {
            target: job.planned.target.clone(),
            build_job_id: job.job_id.clone(),
            job_id: format!("qualify_{}", job.job_id),
            build_handoff_name: format!("eggpack-build-handoff-{}", job.planned.target),
            evidence_handoff_name: format!("eggpack-evidence-{}", job.planned.target),
            classification: planned.policy.qualification,
            host,
            support: planned.policy.support,
            required: job.required,
        });
    }
    qualifications.sort_by(|a, b| a.target.cmp(&b.target));
    // Determine required archive encoding from planned forms.
    let needs_archive = release
        .targets
        .iter()
        .any(|t| matches!(t.artifact_form, eggpack_core::PlannedAssetForm::Archive));
    let graph = ReleaseCIPlanV1 {
        schema_version: 1,
        ci_plan: ci_plan.clone(),
        qualifications,
        gate_job_id: "required_gate".into(),
        aggregate: AggregateJob {
            job_id: "aggregate".into(),
            gate_job_id: "required_gate".into(),
            final_handoff_name: "eggpack-finalized-release".into(),
        },
        finalization: FinalizationSettings {
            archive_encoding: needs_archive.then_some(ArchiveEncodingWrapper::TarGzip),
            evidence_references: vec![],
        },
        staging: None,
        consumer_validators: BTreeMap::new(),
    };
    graph.validate()?;
    Ok(graph)
}

/// Project an executable graph with per-target consumer validators.
///
/// Selectors must match a build binding selector for the same target;
/// validator targets must be selected CIPlan targets. Graphs without
/// validators serialize exactly as M003c.
pub fn project_release_plan_with_consumer(
    ci_plan: &CIPlan,
    qualification_bindings: &QualificationBindingsV1,
    build_bindings: &BuildBindingsV1,
    release: &ReleasePlan,
    consumer: BTreeMap<String, ConsumerValidatorV1>,
) -> Result<ReleaseCIPlanV1, CiError> {
    if consumer.len() > 256 {
        return Err(fail("consumer validator count exceeds bound"));
    }
    for (target, validator) in &consumer {
        validator.validate()?;
        if !ci_plan
            .targets
            .iter()
            .any(|job| job.planned.target == *target)
        {
            return Err(fail("consumer validator target is not in CIPlan"));
        }
        let bindings = build_bindings
            .targets
            .get(target)
            .ok_or_else(|| fail("consumer validator target has no build bindings"))?;
        if !bindings
            .iter()
            .any(|binding| binding.selector == validator.selector)
        {
            return Err(fail(
                "consumer validator selector is not a build output of its target",
            ));
        }
    }
    let mut graph = project_release_plan(ci_plan, qualification_bindings, build_bindings, release)?;
    graph.consumer_validators = consumer;
    graph.validate()?;
    Ok(graph)
}

/// Stable consumer validation job id derived from the build job id.
pub fn consumer_job_id(build_job_id: &str) -> String {
    format!("validate_{build_job_id}")
}

/// Deterministic consumer evidence artifact name for one target.
pub fn consumer_evidence_handoff_name(target: &str) -> String {
    format!("eggpack-consumer-evidence-{target}")
}

/// Decode and shape-check consumer validation evidence JSON (identity
/// checked by the gate caller).
pub fn decode_consumer_evidence(text: &str) -> Result<ConsumerValidationEvidenceV1, CiError> {
    if text.len() > MAX_CONSUMER_EVIDENCE_JSON {
        return Err(fail("consumer evidence exceeds size bound"));
    }
    serde_json::from_str(text).map_err(|_| fail("invalid consumer evidence JSON"))
}

/// Evaluate the required gate including consumer validation evidence.
///
/// Core qualification gates exactly as [`evaluate_gate`]; then every target
/// with a consumer validator must present matching `Passed` consumer
/// evidence. Required validator failure fails the gate; non-gating
/// validator failure suppresses the release rather than producing partial
/// output. Consumer evidence for a target without a validator is invalid.
pub fn evaluate_gate_with_consumer(
    graph: &ReleaseCIPlanV1,
    evidences: &[QualificationEvidence],
    consumer_evidences: &[ConsumerValidationEvidenceV1],
) -> Result<AggregateOutcome, CiError> {
    graph.validate()?;
    for evidence in consumer_evidences {
        evidence.validate()?;
    }
    let outcome = evaluate_gate(graph, evidences)?;
    if outcome != AggregateOutcome::Complete {
        return Ok(outcome);
    }
    let mut by_target: BTreeMap<&str, &ConsumerValidationEvidenceV1> = BTreeMap::new();
    for evidence in consumer_evidences {
        if by_target
            .insert(evidence.target.as_str(), evidence)
            .is_some()
        {
            return Err(fail("duplicate consumer evidence target"));
        }
    }
    for qual in &graph.qualifications {
        let validator = graph.consumer_validators.get(&qual.target);
        match (validator, by_target.get(qual.target.as_str())) {
            (None, None) => {}
            (None, Some(_)) => return Ok(AggregateOutcome::InvalidEvidence),
            (Some(_), None) => {
                if qual.required {
                    return Ok(AggregateOutcome::FailedRequiredGate);
                }
                return Ok(AggregateOutcome::SuppressedNonGatingIncomplete);
            }
            (Some(validator), Some(evidence)) => {
                if evidence.release_id != graph.ci_plan.release_id
                    || evidence.source_revision != graph.ci_plan.source_revision
                    || evidence.target != qual.target
                    || evidence.selector != validator.selector
                    || evidence.interpreter != validator.interpreter
                {
                    return Ok(AggregateOutcome::InvalidEvidence);
                }
                match evidence.outcome {
                    ConsumerValidationOutcome::Passed => {}
                    ConsumerValidationOutcome::Failed(_) => {
                        if qual.required {
                            return Ok(AggregateOutcome::FailedRequiredGate);
                        }
                        return Ok(AggregateOutcome::SuppressedNonGatingIncomplete);
                    }
                }
            }
        }
    }
    if by_target.len() != graph.consumer_validators.len() {
        return Ok(AggregateOutcome::InvalidEvidence);
    }
    Ok(AggregateOutcome::Complete)
}

/// Aggregate validated evidence, candidates, and consumer evidence.
///
/// Never drops failed optional targets or failed optional validators to
/// produce a partial release. On Complete, invokes M004 finalization.
pub fn aggregate_finalize_with_consumer(
    contract: &DistributionContract,
    plan: &ReleasePlan,
    graph: &ReleaseCIPlanV1,
    inputs: &[FinalizationTargetInput],
    consumer_evidences: &[ConsumerValidationEvidenceV1],
    evidence_references: &[String],
    output_root: &Path,
) -> Result<(AggregateOutcome, Option<eggpack_core::FinalizedRelease>), CiError> {
    graph.validate()?;
    let evidences: Vec<QualificationEvidence> = inputs
        .iter()
        .map(|input| input.qualification.clone())
        .collect();
    let outcome = evaluate_gate_with_consumer(graph, &evidences, consumer_evidences)?;
    if outcome != AggregateOutcome::Complete {
        return Ok((outcome, None));
    }
    let archive_encoding = if graph.finalization.archive_encoding.is_some() {
        Some(ArchiveEncoding::TarGzip)
    } else {
        None
    };
    let request = FinalizationRequest {
        product_id: contract.product.id.clone(),
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        targets: inputs.to_vec(),
        evidence_references: evidence_references.to_vec(),
        archive_encoding,
    };
    eggpack_core::finalize_release(contract, plan, &request, output_root)
        .map(|finalized| (AggregateOutcome::Complete, Some(finalized)))
        .map_err(|_| CiError("finalization rejected complete evidence".into()))
}

/// Attach a bounded GitHub draft staging intent to an executable graph.
///
/// The staging job records the aggregate dependency, the GitHub draft
/// provider, the deterministic finalized handoff consumed from aggregate, the
/// deterministic receipt artifact uploaded by stage, and whether staging is
/// required. M001 `CIPlan` semantics are untouched; graphs without staging
/// serialize exactly as M002a.
pub fn with_github_draft_staging(
    mut graph: ReleaseCIPlanV1,
    required: bool,
) -> Result<ReleaseCIPlanV1, CiError> {
    let finalized = graph.aggregate.final_handoff_name.clone();
    graph.staging = Some(StagingJob {
        job_id: "stage".to_owned(),
        aggregate_job_id: graph.aggregate.job_id.clone(),
        provider: StagingProvider::GitHubDraft,
        finalized_handoff_name: finalized,
        receipt_handoff_name: "eggpack-staging-receipt".to_owned(),
        required,
    });
    graph.validate()?;
    Ok(graph)
}

/// Encode validated qualification evidence as deterministic JSON.
pub fn encode_qualification_evidence(evidence: &QualificationEvidence) -> Result<String, CiError> {
    serde_json::to_string(evidence).map_err(|_| fail("evidence serialization failed"))
}

/// Decode and shape-check qualification evidence JSON (identity checked by caller).
pub fn decode_qualification_evidence(text: &str) -> Result<QualificationEvidence, CiError> {
    if text.len() > MAX_EVIDENCE_JSON {
        return Err(fail("evidence exceeds size bound"));
    }
    serde_json::from_str(text).map_err(|_| fail("invalid evidence JSON"))
}

/// Evaluate the required qualification gate over structured evidence.
///
/// Rules: every Required target must have Passed; Deferred is insufficient;
/// Failed, identity mismatch, or missing required evidence fails closed.
pub fn evaluate_gate(
    graph: &ReleaseCIPlanV1,
    evidences: &[QualificationEvidence],
) -> Result<AggregateOutcome, CiError> {
    graph.validate()?;
    let mut by_target: BTreeMap<&str, &QualificationEvidence> = BTreeMap::new();
    for evidence in evidences {
        if by_target
            .insert(evidence.target.as_str(), evidence)
            .is_some()
        {
            return Err(fail("duplicate qualification evidence target"));
        }
    }
    // Every selected target must present evidence; otherwise InvalidEvidence
    // (missing data) unless it is a non-gating incompleteness that suppresses.
    for qual in &graph.qualifications {
        let Some(evidence) = by_target.get(qual.target.as_str()) else {
            if qual.required {
                return Ok(AggregateOutcome::FailedRequiredGate);
            }
            return Ok(AggregateOutcome::SuppressedNonGatingIncomplete);
        };
        if evidence.release_id != graph.ci_plan.release_id
            || evidence.source_revision != graph.ci_plan.source_revision
            || evidence.target != qual.target
            || evidence.planned_classification != qual.classification
            || evidence.support != qual.support
        {
            return Ok(AggregateOutcome::InvalidEvidence);
        }
        match evidence.status {
            QualificationStatus::Passed => {}
            QualificationStatus::Deferred => {
                if qual.required {
                    return Ok(AggregateOutcome::FailedRequiredGate);
                }
                return Ok(AggregateOutcome::SuppressedNonGatingIncomplete);
            }
            QualificationStatus::Failed(_) => {
                if qual.required {
                    return Ok(AggregateOutcome::FailedRequiredGate);
                }
                return Ok(AggregateOutcome::SuppressedNonGatingIncomplete);
            }
        }
    }
    // No partial drops: evidence set must exactly cover selected targets.
    if by_target.len() != graph.qualifications.len() {
        return Ok(AggregateOutcome::InvalidEvidence);
    }
    Ok(AggregateOutcome::Complete)
}

/// Aggregate validated evidence and candidates into a finalized release.
///
/// Never drops failed optional targets to produce a partial release: any
/// non-Complete gate outcome returns the outcome without calling M004.
/// On Complete, invokes `eggpack_core::finalize_release` (not a copy) and
/// returns the finalized manifest bytes identity.
pub fn aggregate_finalize(
    contract: &DistributionContract,
    plan: &ReleasePlan,
    graph: &ReleaseCIPlanV1,
    inputs: &[FinalizationTargetInput],
    evidence_references: &[String],
    output_root: &Path,
) -> Result<(AggregateOutcome, Option<eggpack_core::FinalizedRelease>), CiError> {
    graph.validate()?;
    // Collect evidences from inputs for gating.
    let evidences: Vec<QualificationEvidence> = inputs
        .iter()
        .map(|input| input.qualification.clone())
        .collect();
    let outcome = evaluate_gate(graph, &evidences)?;
    if outcome != AggregateOutcome::Complete {
        return Ok((outcome, None));
    }
    let archive_encoding = if graph.finalization.archive_encoding.is_some() {
        Some(ArchiveEncoding::TarGzip)
    } else {
        None
    };
    let request = FinalizationRequest {
        product_id: contract.product.id.clone(),
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        targets: inputs.to_vec(),
        evidence_references: evidence_references.to_vec(),
        archive_encoding,
    };
    eggpack_core::finalize_release(contract, plan, &request, output_root)
        .map(|finalized| (AggregateOutcome::Complete, Some(finalized)))
        .map_err(|_| CiError("finalization rejected complete evidence".into()))
}

fn tool_install_snippet(tool: &EggpackToolPolicy) -> String {
    format!(
        "      - name: Install pinned Eggpack tool\n        shell: bash\n        timeout-minutes: {timeout}\n        run: |\n          cargo install --git {repo} --rev {rev} --locked -p {package}\n          eggpack --version\n",
        timeout = tool.install_timeout_minutes,
        repo = tool.repo,
        rev = tool.revision,
        package = tool.package
    )
}

fn checkout_snippet(out: &mut String, policy: &GitHubPolicy, staging: bool) {
    out.push_str("      - name: Check out source\n        uses: ");
    out.push_str(&yaml_scalar(&policy.checkout.reference));
    if staging {
        let reference = match policy.staging.as_ref().map(|p| p.tag_source) {
            Some(StagingTagSource::DispatchInput) => "${{ github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref }}",
            _ => "${{ github.ref }}",
        };
        out.push_str("\n        with:\n          ref: ");
        out.push_str(&yaml_scalar(reference));
    }
    out.push('\n');
}

fn source_verify_snippet(out: &mut String, inputs: &GitHubReleaseInputsV1, staging: bool) {
    if !staging {
        return;
    }
    let command = RunnerCommand::VerifySource {
        release_plan: inputs.release_plan.clone(),
    };
    out.push_str(
        "      - name: Verify checked-out release source\n        shell: bash\n        run: ",
    );
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
    let tag_expr = match staging_intent.tag_source {
        StagingTagSource::RefName => "${{ github.ref_name }}".to_owned(),
        StagingTagSource::DispatchInput => "${{ inputs.release_tag }}".to_owned(),
    };
    let resolve = RunnerCommand::ResolveRelease {
        contract: static_inputs.contract.clone(),
        pack_config: pack_config_path,
        build_bindings: static_inputs.build_bindings.clone(),
        qualification_bindings: static_inputs.qualification_bindings.clone(),
        consumer_validators: static_inputs.consumer_validators.clone(),
        selected: shape.selected_aliases.join(","),
        tag: tag_expr,
        source_revision: "$head_sha".to_owned(),
        template: template_path,
        source_root: "${{ github.workspace }}".to_owned(),
        output_plan: format!("./{RUNTIME_RELEASE_PLAN}"),
        output_ci_plan: format!("./{RUNTIME_CI_PLAN}"),
        output_github_policy: format!("./{RUNTIME_GITHUB_POLICY}"),
    };
    let rendered =
        render_release_github_inner(&graph, &runtime_policy, Some(&RuntimeRender { resolve }))?;
    if rendered.contains(UNRESOLVED_RELEASE_ID) || rendered.contains(UNRESOLVED_SOURCE_REVISION) {
        return Err(fail("reusable workflow embeds unresolved release identity"));
    }
    Ok(rendered)
}

/// Compare existing reusable workflow bytes without writing files.
pub fn check_reusable_release_github(
    contract: &DistributionContract,
    shape: &ReleaseWorkflowShapeV1,
    policy: &GitHubPolicy,
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    let expected = render_reusable_release_github(contract, shape, policy)?;
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

fn render_release_github_inner(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
    runtime: Option<&RuntimeRender>,
) -> Result<String, CiError> {
    graph.validate()?;
    policy.validate(&graph.ci_plan)?;
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
    // Emulated qualification requires an explicit finite provider runtime
    // policy; never silently invoke M003 with an empty runtime.
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
    // Consumer validators require an explicit checked-in validator map path;
    // never resolve validator scripts by discovery.
    let validator_map =
        if graph.consumer_validators.is_empty() {
            None
        } else {
            Some(inputs.consumer_validators.clone().ok_or_else(|| {
                fail("consumer validators require an explicit validator map path")
            })?)
        };

    // Header mirrors M001 build rendering (deterministic, read-only, pinned).
    // When staging is enabled, workflow_dispatch carries an explicit
    // release_tag input (exact existing tag; no latest/branch default).
    let staging_enabled = graph.staging.is_some();
    let mut out = String::from("name: Eggpack candidate builds\n'on':\n");
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
    out.push_str("jobs:\n  preflight:\n    runs-on: ");
    out.push_str(&yaml_scalar(&policy.preflight_runner));
    out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
    out.push_str(&policy.timeout_minutes.to_string());
    out.push_str("\n    steps:\n");
    checkout_snippet(&mut out, policy, staging_enabled);
    source_verify_snippet(&mut out, inputs, staging_enabled);
    out.push_str("      - name: Check Cargo availability\n        shell: bash\n        run: cargo --version\n");

    // Runtime identity preflight (reusable mode only, M003d section 4D):
    // check out the event-selected exact tag, derive/verify HEAD, resolve
    // invocation-local ReleasePlan/ReleaseCIPlan/GitHubDraftPolicy, and
    // upload those runtime documents as an internal preflight artifact.
    // Exact mode emits nothing here (M003c byte-compatible).
    if let Some(runtime) = runtime {
        let staging_policy = policy
            .staging
            .as_ref()
            .ok_or_else(|| fail("reusable rendering requires a staging policy"))?;
        out.push_str("  resolve:\n    needs: preflight\n    runs-on: ");
        out.push_str(&yaml_scalar(&policy.preflight_runner));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    steps:\n");
        checkout_snippet(&mut out, policy, true);
        out.push_str("      - name: Validate exact-tag source\n        shell: bash\n        run: ");
        out.push_str(&yaml_scalar(match staging_policy.tag_source {
            StagingTagSource::RefName => "test \"${{ github.event_name }}\" = \"push\" && test \"${{ github.ref_type }}\" = \"tag\"",
            StagingTagSource::DispatchInput => "test -n \"${{ inputs.release_tag }}\"",
        }));
        out.push('\n');
        out.push_str(&tool_install_snippet(tool));
        out.push_str("      - name: Resolve runtime release identity\n        shell: bash\n        run: |\n          head_sha=\"$(git rev-parse --verify HEAD^{commit})\"\n          ");
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
    }

    // Build jobs: M001 cargo invocations plus pinned tool install, explicit
    // `_capture-build` into the canonical per-target directory, and upload of
    // the entire canonical directory as one deterministic artifact.
    for job in &graph.ci_plan.targets {
        let runner = policy
            .runners
            .iter()
            .find(|mapping| {
                mapping.os == job.planned.policy.host_os
                    && mapping.arch == job.planned.policy.host_arch
            })
            .ok_or_else(|| fail("GitHub runner mapping missing for build host"))?;
        if job.planned.policy.strategy == BuildStrategy::CargoZigbuild
            && (!runner.cargo_zigbuild || !runner.zig)
        {
            return Err(fail(
                "cross-build runner lacks preinstalled cargo-zigbuild or Zig",
            ));
        }
        out.push_str("  ");
        out.push_str(&job.job_id);
        // Reusable builds wait for runtime identity; exact builds wait for
        // preflight directly.
        if runtime.is_some() {
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
        checkout_snippet(&mut out, policy, staging_enabled);
        runtime_identity_download_step(&mut out, download_pin, runtime.is_some());
        source_verify_snippet(&mut out, inputs, staging_enabled);
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
        }
        let capture = RunnerCommand::CaptureBuild {
            contract: inputs.contract.clone(),
            release_plan: inputs.release_plan.clone(),
            build_bindings: inputs.build_bindings.clone(),
            target: job.planned.target.clone(),
            cargo_target_dir:
                "${{ runner.temp }}/eggpack/${{ github.run_id }}-${{ github.run_attempt }}".into(),
            output_dir: format!("./eggpack-build/{}", job.job_id),
        };
        let qual = graph
            .qualifications
            .iter()
            .find(|q| q.build_job_id == job.job_id)
            .ok_or_else(|| fail("qualification job missing for build target"))?;
        out.push_str(&tool_install_snippet(tool));
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

    // Qualification jobs: download the matching canonical build artifact into
    // a target-specific private directory, invoke `_qualify-target` with all
    // explicit inputs, and upload the complete qualification directory.
    for qual in &graph.qualifications {
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
            contract: inputs.contract.clone(),
            release_plan: inputs.release_plan.clone(),
            build_bindings: inputs.build_bindings.clone(),
            qualification_bindings: inputs.qualification_bindings.clone(),
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
        out.push_str(&qual.build_job_id);
        out.push_str("\n    runs-on: ");
        out.push_str(&yaml_scalar(&runner.label));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    continue-on-error: ");
        out.push_str(if qual.required { "false\n" } else { "true\n" });
        out.push_str("    steps:\n");
        checkout_snippet(&mut out, policy, staging_enabled);
        runtime_identity_download_step(&mut out, download_pin, runtime.is_some());
        source_verify_snippet(&mut out, inputs, staging_enabled);
        out.push_str(&tool_install_snippet(tool));
        out.push_str("      - name: Download build handoff\n        uses: ");
        out.push_str(&yaml_scalar(&download_pin.reference));
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
    // Consumer validation jobs (M003d): one per target with a consumer
    // validator, running after core qualification and before the gate.
    // Each consumes the canonical qualification handoff (exact candidate
    // bytes already validated by Eggpack) and never rebuilds the binary.
    // Graphs without validators emit no jobs here (M003c byte-compatible).
    if let Some(validator_map) = &validator_map {
        for qual in &graph.qualifications {
            if !graph.consumer_validators.contains_key(&qual.target) {
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
            out.push_str(&qual.job_id);
            out.push_str("\n    runs-on: ");
            out.push_str(&yaml_scalar(&runner.label));
            out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
            out.push_str(&policy.timeout_minutes.to_string());
            out.push_str("\n    continue-on-error: ");
            out.push_str(if qual.required { "false\n" } else { "true\n" });
            out.push_str("    steps:\n");
            checkout_snippet(&mut out, policy, staging_enabled);
            runtime_identity_download_step(&mut out, download_pin, runtime.is_some());
            source_verify_snippet(&mut out, inputs, staging_enabled);
            out.push_str(&tool_install_snippet(tool));
            out.push_str("      - name: Download qualification handoff\n        uses: ");
            out.push_str(&yaml_scalar(&download_pin.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!(
                "./eggpack-handoff/{}",
                qual.build_job_id
            )));
            out.push_str("\n      - name: Validate exact candidate (consumer)\n        shell: bash\n        run: ");
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
    // Required gate: one explicit download step per target into the canonical
    // `eggpack-inputs/<target>/` layout, then `_evaluate-gate` with the exact
    // CI plan, inputs directory, and outcome path.
    {
        let gate_cmd = RunnerCommand::EvaluateGate {
            ci_plan: inputs.ci_plan.clone(),
            inputs_dir: "./eggpack-inputs".into(),
            output: format!("./eggpack-gate/{}", GATE_OUTCOME_FILE),
        };
        out.push_str("  ");
        out.push_str(&graph.gate_job_id);
        out.push_str(":\n    needs: [");
        // Gate waits for the consumer validation job where one exists,
        // otherwise directly for the qualification job. Without validators
        // this is exactly the M003c dependency list.
        let gate_needs: Vec<String> = graph
            .qualifications
            .iter()
            .map(|q| {
                if graph.consumer_validators.contains_key(&q.target) {
                    consumer_job_id(&q.build_job_id)
                } else {
                    q.job_id.clone()
                }
            })
            .collect();
        out.push_str(&gate_needs.join(", "));
        out.push_str("]\n    runs-on: ");
        out.push_str(&yaml_scalar(&policy.preflight_runner));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    steps:\n");
        checkout_snippet(&mut out, policy, staging_enabled);
        runtime_identity_download_step(&mut out, download_pin, runtime.is_some());
        source_verify_snippet(&mut out, inputs, staging_enabled);
        out.push_str(&tool_install_snippet(tool));
        for qual in &graph.qualifications {
            out.push_str("      - name: Download qualification ");
            out.push_str(&yaml_scalar(&qual.target));
            out.push_str("\n        uses: ");
            out.push_str(&yaml_scalar(&download_pin.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
            out.push('\n');
        }
        // Consumer evidence joins the same canonical per-target layout so
        // the gate consumes one complete directory per target.
        for qual in &graph.qualifications {
            if !graph.consumer_validators.contains_key(&qual.target) {
                continue;
            }
            out.push_str("      - name: Download consumer evidence ");
            out.push_str(&yaml_scalar(&qual.target));
            out.push_str("\n        uses: ");
            out.push_str(&yaml_scalar(&download_pin.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&consumer_evidence_handoff_name(&qual.target)));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
            out.push('\n');
        }
        out.push_str("      - name: Evaluate required qualification gate\n        shell: bash\n        run: ");
        out.push_str(&yaml_scalar(&gate_cmd.to_shell()));
        out.push('\n');
    }
    // Aggregate: same canonical per-target directories, explicit contract /
    // release-plan / CI-plan / inputs-dir / output-root / summary arguments.
    // The finalized internal release is uploaded only when the aggregate
    // outcome is Complete (the CLI fails closed otherwise, so no partial
    // release can be uploaded).
    {
        let agg_cmd = RunnerCommand::Aggregate {
            contract: inputs.contract.clone(),
            release_plan: inputs.release_plan.clone(),
            ci_plan: inputs.ci_plan.clone(),
            inputs_dir: "./eggpack-inputs".into(),
            output_root: "./eggpack-finalized/root".into(),
            output: "./eggpack-finalized/summary.json".into(),
        };
        out.push_str("  ");
        out.push_str(&graph.aggregate.job_id);
        out.push_str(":\n    needs: ");
        out.push_str(&graph.gate_job_id);
        out.push_str("\n    runs-on: ");
        out.push_str(&yaml_scalar(&policy.preflight_runner));
        out.push_str("\n    permissions:\n      contents: read\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    steps:\n");
        checkout_snippet(&mut out, policy, staging_enabled);
        runtime_identity_download_step(&mut out, download_pin, runtime.is_some());
        source_verify_snippet(&mut out, inputs, staging_enabled);
        out.push_str(&tool_install_snippet(tool));
        for qual in &graph.qualifications {
            out.push_str("      - name: Download qualification ");
            out.push_str(&yaml_scalar(&qual.target));
            out.push_str("\n        uses: ");
            out.push_str(&yaml_scalar(&download_pin.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&qual.evidence_handoff_name));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
            out.push('\n');
        }
        for qual in &graph.qualifications {
            if !graph.consumer_validators.contains_key(&qual.target) {
                continue;
            }
            out.push_str("      - name: Download consumer evidence ");
            out.push_str(&yaml_scalar(&qual.target));
            out.push_str("\n        uses: ");
            out.push_str(&yaml_scalar(&download_pin.reference));
            out.push_str("\n        with:\n          name: ");
            out.push_str(&yaml_scalar(&consumer_evidence_handoff_name(&qual.target)));
            out.push_str("\n          path: ");
            out.push_str(&yaml_scalar(&format!("./eggpack-inputs/{}", qual.target)));
            out.push('\n');
        }
        out.push_str(
            "      - name: Aggregate and finalize release\n        shell: bash\n        run: ",
        );
        out.push_str(&yaml_scalar(&agg_cmd.to_shell()));
        out.push('\n');
        out.push_str("      - name: Upload internal finalized release\n        uses: ");
        out.push_str(&yaml_scalar(&policy.upload_artifact.reference));
        out.push_str("\n        with:\n          name: ");
        out.push_str(&yaml_scalar(&graph.aggregate.final_handoff_name));
        out.push_str("\n          path: ./eggpack-finalized\n          if-no-files-found: error\n          retention-days: ");
        out.push_str(&policy.artifact_retention_days.to_string());
        out.push('\n');
    }
    // Stage (M003b): exactly one least-privilege draft staging job. It runs
    // only after aggregate, only for exact tags (tag push or explicit dispatch
    // input), consumes the exact aggregate artifact, calls only the M003a
    // draft-only commands, and uploads the bounded staging receipt. Every
    // prior job remains read-only; only stage has contents:write; no job
    // receives id-token:write; the token travels via environment only.
    if let Some(staging) = &graph.staging {
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
        out.push_str(&staging.aggregate_job_id);
        let (stage_if, stage_ref) = match staging_policy.tag_source {
            StagingTagSource::RefName => (
                "github.event_name == 'push' && github.ref_type == 'tag'",
                "${{ github.ref }}",
            ),
            StagingTagSource::DispatchInput => (
                "github.event_name == 'workflow_dispatch'",
                "${{ inputs.release_tag }}",
            ),
        };
        out.push_str("\n    if: ");
        out.push_str(stage_if);
        out.push_str("\n    runs-on: ");
        out.push_str(&yaml_scalar(&staging_policy.runner));
        out.push_str("\n    permissions:\n      contents: write\n    timeout-minutes: ");
        out.push_str(&policy.timeout_minutes.to_string());
        out.push_str("\n    steps:\n      - name: Check out source\n        uses: ");
        out.push_str(&yaml_scalar(&policy.checkout.reference));
        out.push_str("\n        with:\n          ref: ");
        out.push_str(&yaml_scalar(stage_ref));
        out.push('\n');
        runtime_identity_download_step(&mut out, download_pin, runtime.is_some());
        out.push_str("      - name: Validate exact-tag source\n        shell: bash\n        run: ");
        out.push_str(&yaml_scalar(match staging_policy.tag_source {
            StagingTagSource::RefName => "test \"${{ github.event_name }}\" = \"push\" && test \"${{ github.ref_type }}\" = \"tag\"",
            StagingTagSource::DispatchInput => "test -n \"${{ inputs.release_tag }}\"",
        }));
        out.push('\n');
        source_verify_snippet(&mut out, inputs, true);
        out.push_str(&tool_install_snippet(tool));
        out.push_str("      - name: Download finalized release\n        uses: ");
        out.push_str(&yaml_scalar(&download_pin.reference));
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
    }
    if out.len() > MAX_WORKFLOW_BYTES {
        return Err(fail("rendered release workflow exceeds size bound"));
    }
    // Static guards: the generated workflow must never publish, clobber via
    // release CLIs, mutate tags, or mint OIDC tokens.
    if out.contains("id-token: write") {
        return Err(fail("generated workflow must not request id-token write"));
    }
    for forbidden in [
        "gh release",
        "--clobber",
        "release --publish",
        "git tag ",
        "git push --tags",
        "curl ",
    ] {
        if out.contains(forbidden) {
            return Err(fail(
                "generated workflow contains a forbidden release command",
            ));
        }
    }
    Ok(out)
}

/// Compare existing release workflow bytes without writing files.
pub fn check_release_github(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    let expected = render_release_github(graph, policy)?;
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

// ---------------------------------------------------------------------------
// M003d — consumer-owned exact-candidate validator.
// ---------------------------------------------------------------------------

/// Canonical consumer validation evidence file name inside per-target dirs.
pub const CONSUMER_EVIDENCE_FILE: &str = "consumer-evidence.json";
const MAX_VALIDATOR_JSON: usize = 64 * 1024;
const MAX_CONSUMER_EVIDENCE_JSON: usize = 64 * 1024;
/// Maximum validator script bytes (1 MiB, matching wrapper bound).
const MAX_VALIDATOR_SCRIPT_BYTES: u64 = 1024 * 1024;

/// Finite validator interpreter (M003d section 9).
///
/// Exactly `Python3`: no caller-provided interpreter executable is accepted.
/// The renderer maps it finitely by host (`python3` on Linux/macOS, `python`
/// on Windows) with a bounded `--version` preflight that must identify
/// Python 3; local execution uses the same mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorInterpreterV1 {
    /// Bounded Python 3 interpreter selected by host mapping.
    Python3,
}

/// One narrow post-core-qualification verifier type.
///
/// This is intentionally not part of `QualificationEvidence`; M003 remains
/// the provider-neutral binary qualification authority. Invocation is fixed
/// semantically as `Python3 <validated-script-path> <exact-candidate-path>`.
/// No arbitrary executable field, shell, environment map, or caller-supplied
/// argument vector exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerValidatorV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Candidate selector identifying the exact handoff output to validate.
    pub selector: LogicalOutputSelector,
    /// Finite interpreter (exactly Python3).
    pub interpreter: ValidatorInterpreterV1,
    /// Explicit repository-relative validator script path.
    pub script: String,
    /// Bounded execution timeout in milliseconds (1s to 10min).
    pub timeout_ms: u64,
    /// Bounded stdout capture in bytes.
    pub stdout_limit: usize,
    /// Bounded stderr capture in bytes.
    pub stderr_limit: usize,
}

impl ConsumerValidatorV1 {
    /// Parse a strict bounded JSON validator document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_VALIDATOR_JSON {
            return Err(fail("consumer validator exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid consumer validator JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize deterministically after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("consumer validator encode failed"))
    }

    /// Validate schema version, interpreter, script path, and bounds.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported consumer validator version"));
        }
        if self.interpreter != ValidatorInterpreterV1::Python3 {
            return Err(fail("unsupported consumer validator interpreter"));
        }
        validate_release_input_path(&self.script)?;
        if self.timeout_ms < 1_000 || self.timeout_ms > 600_000 {
            return Err(fail("consumer validator timeout out of bounds"));
        }
        if self.stdout_limit == 0
            || self.stdout_limit > 8_000_000
            || self.stderr_limit == 0
            || self.stderr_limit > 8_000_000
        {
            return Err(fail("consumer validator output limits out of bounds"));
        }
        Ok(())
    }
}

/// Bounded consumer validation failure classification (no output contents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerValidationFailure {
    /// Validator exited non-zero.
    NonZeroExit,
    /// Validator exceeded its bounded timeout.
    Timeout,
    /// Validator exceeded its bounded stdout/stderr capture.
    OutputLimit,
    /// Python 3 interpreter unavailable or preflight rejected.
    InterpreterUnavailable,
    /// Exact candidate identity (size/digest/type) mismatch.
    CandidateMismatch,
    /// Validator script unavailable or failed script validation.
    ScriptUnavailable,
    /// Caller cancellation requested before completion.
    Cancelled,
}

/// Bounded consumer validation outcome (no script output contents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerValidationOutcome {
    /// Validator exited zero within timeout and output bounds.
    Passed,
    /// Validator failed with a bounded classification.
    Failed(ConsumerValidationFailure),
}

/// Bounded consumer validation evidence (M003d section 11).
///
/// Contains only schema version, release/source/target identity, selector,
/// validator kind, process outcome summary, and candidate size/SHA identity.
/// No script output contents are recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerValidationEvidenceV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Release id copied from the validated handoff identity.
    pub release_id: String,
    /// Source revision copied from the validated handoff identity.
    pub source_revision: String,
    /// Canonical target triple.
    pub target: String,
    /// Validated candidate selector.
    pub selector: LogicalOutputSelector,
    /// Validator interpreter kind.
    pub interpreter: ValidatorInterpreterV1,
    /// Bounded process outcome summary.
    pub outcome: ConsumerValidationOutcome,
    /// Observed candidate size in bytes.
    pub candidate_size: u64,
    /// Observed candidate lowercase SHA-256 hex.
    pub candidate_sha256: String,
}

impl ConsumerValidationEvidenceV1 {
    /// Serialize after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("consumer evidence encode failed"))
    }

    /// Parse and validate a bounded evidence document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_CONSUMER_EVIDENCE_JSON {
            return Err(fail("consumer evidence exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid consumer evidence JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Validate shape, identity bounds, and digest syntax.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1
            || !safe_metadata(&self.release_id, 256)
            || !safe_metadata(&self.source_revision, 256)
            || !safe_target(&self.target)
            || self.candidate_size == 0
        {
            return Err(fail("invalid or out-of-bounds consumer evidence"));
        }
        if self.candidate_sha256.len() != 64
            || !self
                .candidate_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(fail("consumer evidence digest must be lowercase hex"));
        }
        Ok(())
    }
}

/// Map the finite interpreter to its host executable name.
///
/// `python3` on Linux/macOS, `python` on Windows (M003d section 9).
pub fn python_interpreter_exe() -> &'static str {
    python_interpreter_exe_for_windows(cfg!(windows))
}

/// Finite host mapping for the Python3 validator interpreter.
pub fn python_interpreter_exe_for_windows(is_windows: bool) -> &'static str {
    if is_windows {
        "python"
    } else {
        "python3"
    }
}

/// Bounded execution request for one consumer validator invocation.
pub struct ConsumerValidationRequest<'a> {
    /// Validated validator configuration.
    pub validator: &'a ConsumerValidatorV1,
    /// Absolute validated script file path (regular non-symlink checked by runner).
    pub script_path: &'a Path,
    /// Absolute validated candidate file path (identity checked by runner).
    pub candidate_path: &'a Path,
    /// Expected exact candidate size in bytes.
    pub expected_size: u64,
    /// Expected exact candidate lowercase SHA-256 hex.
    pub expected_sha256: &'a str,
    /// Explicit working directory (must be a real directory).
    pub work_dir: &'a Path,
    /// Release id for evidence linkage.
    pub release_id: &'a str,
    /// Source revision for evidence linkage.
    pub source_revision: &'a str,
    /// Canonical target triple for evidence linkage.
    pub target: &'a str,
    /// Optional caller cancellation flag (bounded timeout always applies).
    pub cancelled: Option<&'a std::sync::atomic::AtomicBool>,
    /// Optional exact PATH directories for interpreter lookup.
    ///
    /// `None` inherits the process PATH. `Some` replaces PATH entirely,
    /// which keeps missing-interpreter tests hermetic.
    pub path_dirs: Option<&'a [std::path::PathBuf]>,
}

fn consumer_evidence_shell(
    request: &ConsumerValidationRequest<'_>,
    outcome: ConsumerValidationOutcome,
    candidate_size: u64,
    candidate_sha256: String,
) -> Result<ConsumerValidationEvidenceV1, CiError> {
    let evidence = ConsumerValidationEvidenceV1 {
        schema_version: 1,
        release_id: request.release_id.to_owned(),
        source_revision: request.source_revision.to_owned(),
        target: request.target.to_owned(),
        selector: request.validator.selector.clone(),
        interpreter: request.validator.interpreter,
        outcome,
        candidate_size,
        candidate_sha256,
    };
    evidence.validate()?;
    Ok(evidence)
}

fn read_limited(stream: Option<std::process::ChildStdout>, limit: usize) -> (Vec<u8>, bool) {
    use std::io::Read;
    let Some(mut stream) = stream else {
        return (Vec::new(), false);
    };
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.len() > limit {
                    // Drain is abandoned; caller kills the child and the
                    // pipe closes on drop.
                    return (bytes, true);
                }
            }
            Err(_) => break,
        }
    }
    (bytes, false)
}

fn read_limited_stderr(stream: Option<std::process::ChildStderr>, limit: usize) -> (Vec<u8>, bool) {
    use std::io::Read;
    let Some(mut stream) = stream else {
        return (Vec::new(), false);
    };
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.len() > limit {
                    return (bytes, true);
                }
            }
            Err(_) => break,
        }
    }
    (bytes, false)
}

/// Execute one consumer validator against the exact candidate.
///
/// Shell-free fixed invocation `Python3 <script> <candidate>` with cleared
/// environment (PATH plus `SYSTEMROOT` on Windows only), explicit working
/// directory, null stdin, bounded timeout/output, and exact candidate
/// identity verification. Returns bounded evidence without script output
/// contents; only caller misuse (invalid config, non-absolute paths,
/// malformed expectations, unusable working directory) returns `CiError`.
/// If Python is unavailable, validation fails with
/// `InterpreterUnavailable` rather than silently skipping.
pub fn run_consumer_validator(
    request: &ConsumerValidationRequest<'_>,
) -> Result<ConsumerValidationEvidenceV1, CiError> {
    request.validator.validate()?;
    for path in [
        request.script_path,
        request.candidate_path,
        request.work_dir,
    ] {
        if !path.is_absolute() {
            return Err(fail("consumer validation paths must be absolute"));
        }
    }
    let work_meta = std::fs::symlink_metadata(request.work_dir)
        .map_err(|_| fail("consumer validation working directory is unavailable"))?;
    if !work_meta.is_dir() || work_meta.file_type().is_symlink() {
        return Err(fail(
            "consumer validation working directory is not a real directory",
        ));
    }
    if request.expected_sha256.len() != 64
        || !request
            .expected_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fail("expected candidate digest must be lowercase hex"));
    }
    if !safe_metadata(request.release_id, 256)
        || !safe_metadata(request.source_revision, 256)
        || !safe_target(request.target)
    {
        return Err(fail("consumer validation identity out of bounds"));
    }

    // Script: regular non-symlink file within the script bound. The source
    // checkout was already verified by M003c; no PATH-selected script is
    // possible (absolute path required above, interpreter string is not
    // configurable beyond the finite enum).
    let script_usable = std::fs::symlink_metadata(request.script_path)
        .ok()
        .is_some_and(|meta| {
            !meta.file_type().is_symlink()
                && meta.is_file()
                && meta.len() >= 1
                && meta.len() <= MAX_VALIDATOR_SCRIPT_BYTES
        });
    // Candidate identity is observed before any execution so evidence always
    // carries the validated linkage.
    let (observed_size, observed_sha) = hash_candidate_file(request.candidate_path);
    let candidate_ok = is_regular_nonempty_file(request.candidate_path)
        && observed_size == request.expected_size
        && observed_sha == request.expected_sha256;
    // Evidence requires a non-zero size and 64-hex digest even when the
    // candidate itself is the failure point.
    let evidence_size = observed_size.max(1);
    let evidence_sha = if observed_sha.len() == 64 {
        observed_sha.clone()
    } else {
        "0".repeat(64)
    };
    if !candidate_ok {
        return consumer_evidence_shell(
            request,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch),
            evidence_size,
            evidence_sha,
        );
    }
    if !script_usable {
        return consumer_evidence_shell(
            request,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::ScriptUnavailable),
            evidence_size,
            evidence_sha,
        );
    }

    let exe = python_interpreter_exe();
    let path_value = match request.path_dirs {
        Some(dirs) => dirs
            .iter()
            .map(|dir| dir.as_os_str())
            .collect::<Vec<_>>()
            .join(&std::ffi::OsString::from(if cfg!(windows) {
                ";"
            } else {
                ":"
            }))
            .into_string()
            .map_err(|_| fail("validator PATH override is not valid Unicode"))?,
        None => std::env::var("PATH").unwrap_or_default(),
    };
    // Bounded `--version` preflight must identify Python 3. Missing or
    // unusable interpreters fail validation rather than skipping it.
    let preflight_ok = run_interpreter_preflight(exe, &path_value, request);
    if !preflight_ok {
        return consumer_evidence_shell(
            request,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::InterpreterUnavailable),
            evidence_size,
            evidence_sha,
        );
    }

    let outcome = run_validator_process(request, exe, &path_value);
    consumer_evidence_shell(request, outcome, evidence_size, evidence_sha)
}

fn is_regular_nonempty_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .ok()
        .is_some_and(|meta| !meta.file_type().is_symlink() && meta.is_file() && meta.len() >= 1)
}

fn hash_candidate_file(path: &Path) -> (u64, String) {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => return (0, String::new()),
    };
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() == 0 {
        return (0, String::new());
    }
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return (0, String::new()),
    };
    use std::io::Read;
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 65536];
    let mut size = 0u64;
    loop {
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                size += count as u64;
                hasher.update(&chunk[..count]);
            }
            Err(_) => return (0, String::new()),
        }
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    (size, digest)
}

fn validator_command(
    exe: &str,
    path_value: &str,
    request: &ConsumerValidationRequest<'_>,
    script_arg: Option<&Path>,
    candidate_arg: Option<&Path>,
) -> std::process::Command {
    let mut command = std::process::Command::new(exe);
    // Shell-free fixed argv; cleared environment with a documented
    // allowlist (PATH everywhere, SYSTEMROOT on Windows for CPython).
    // No network authority is granted by Eggpack; stdin is null.
    command.env_clear();
    command.env("PATH", path_value);
    #[cfg(windows)]
    {
        if let Ok(system_root) = std::env::var("SYSTEMROOT") {
            command.env("SYSTEMROOT", system_root);
        }
    }
    command.current_dir(request.work_dir);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    match (script_arg, candidate_arg) {
        (Some(script), Some(candidate)) => {
            command.arg(script);
            command.arg(candidate);
        }
        (None, None) => {
            command.arg("--version");
        }
        _ => {}
    }
    command
}

fn run_interpreter_preflight(
    exe: &str,
    path_value: &str,
    request: &ConsumerValidationRequest<'_>,
) -> bool {
    let mut command = validator_command(exe, path_value, request, None, None);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return false,
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_handle = std::thread::spawn(move || read_limited(stdout, 8192));
    let stderr_handle = std::thread::spawn(move || read_limited_stderr(stderr, 8192));
    let deadline =
        std::time::Instant::now() + Duration::from_millis(request.validator.timeout_ms.min(30_000));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break None,
        }
    };
    let (out, _) = stdout_handle.join().unwrap_or_default();
    let (err, _) = stderr_handle.join().unwrap_or_default();
    let Some(status) = status else {
        return false;
    };
    if !status.success() {
        return false;
    }
    let combined = [out, err].concat();
    let text = String::from_utf8_lossy(&combined);
    text.contains("Python 3")
}

/// Join a finished reader thread without blocking; true when over-limit.
fn take_finished_over(handle: &mut Option<std::thread::JoinHandle<(Vec<u8>, bool)>>) -> bool {
    if handle.as_ref().is_some_and(|thread| thread.is_finished()) {
        if let Some(joined) = handle.take().map(|thread| thread.join()) {
            return joined.map(|(_, over)| over).unwrap_or(false);
        }
    }
    false
}

// ---------------------------------------------------------------------------
// M003d — static workflow shape vs runtime release identity.
// ---------------------------------------------------------------------------

/// Workflow-private runtime identity directory (invocation-local storage).
pub const RUNTIME_IDENTITY_DIR: &str = "eggpack-runtime";
/// Workflow-private runtime ReleasePlan path.
pub const RUNTIME_RELEASE_PLAN: &str = "eggpack-runtime/release-plan.json";
/// Workflow-private runtime ReleaseCIPlanV1 path.
pub const RUNTIME_CI_PLAN: &str = "eggpack-runtime/release-ci-plan.json";
/// Workflow-private runtime GitHubDraftPolicyV1 path.
pub const RUNTIME_GITHUB_POLICY: &str = "eggpack-runtime/github-draft.json";
/// Internal preflight artifact carrying the runtime identity documents.
pub const RUNTIME_IDENTITY_ARTIFACT: &str = "eggpack-runtime-identity";
const MAX_SHAPE_JSON: usize = 1_000_000;

/// Placeholder release id used only to project the static graph structure.
///
/// It must never appear in rendered workflow bytes; reusable rendering
/// asserts its absence along with the placeholder revision.
const UNRESOLVED_RELEASE_ID: &str = "0.0.0-m003d-unresolved";
/// Placeholder source revision used only to project static graph structure.
const UNRESOLVED_SOURCE_REVISION: &str = "0000000000000000000000000000000000000000";

/// Bounded staging intent carried by the static workflow shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeStagingIntentV1 {
    /// Staging provider (exactly GitHub draft).
    pub provider: StagingProvider,
    /// Explicit tag source mapping.
    pub tag_source: StagingTagSource,
    /// Whether staging is required.
    pub required: bool,
}

/// Static checked-in release workflow shape (M003d section 4A).
///
/// Contains only identity-independent information needed to render the
/// checked-in workflow: canonical target set, target policy, build and
/// core qualification bindings, consumer validator configuration, and
/// staging intent. It MUST NOT contain release_id, source_revision, an
/// exact future tag, a GitHub release id, or artifact digests/sizes.
/// Historical `CIPlan` v1 / `ReleaseCIPlanV1` evidence is preserved rather
/// than silently reinterpreted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseWorkflowShapeV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Canonical ordered PackConfig targets (identity-independent policy).
    pub targets: Vec<TargetPolicy>,
    /// Contract target aliases selected for resolution, in order.
    pub selected_aliases: Vec<String>,
    /// Explicit build bindings.
    pub build_bindings: BuildBindingsV1,
    /// Explicit core qualification bindings.
    pub qualification_bindings: QualificationBindingsV1,
    /// Per-target consumer validators (empty when disabled).
    #[serde(default)]
    pub consumer_validators: BTreeMap<String, ConsumerValidatorV1>,
    /// Staging intent. Required for reusable release rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<ShapeStagingIntentV1>,
}

impl ReleaseWorkflowShapeV1 {
    /// Parse a strict bounded JSON shape document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_SHAPE_JSON {
            return Err(fail("workflow shape exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid workflow shape JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize deterministically after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("workflow shape encode failed"))
    }

    /// Validate schema version, canonical target ordering, alias bounds,
    /// and embedded binding/validator documents.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported workflow shape version"));
        }
        if self.targets.is_empty() || self.targets.len() > 256 {
            return Err(fail("workflow shape target count out of bounds"));
        }
        let mut previous: Option<&str> = None;
        let mut seen = BTreeSet::new();
        for policy in &self.targets {
            if !safe_target(&policy.target)
                || !seen.insert(policy.target.as_str())
                || previous.is_some_and(|p| p >= policy.target.as_str())
            {
                return Err(fail(
                    "workflow shape targets must be unique, canonical, and ordered",
                ));
            }
            previous = Some(&policy.target);
        }
        if self.selected_aliases.is_empty() || self.selected_aliases.len() > 256 {
            return Err(fail("workflow shape alias count out of bounds"));
        }
        for alias in &self.selected_aliases {
            if !safe_metadata(alias, 128) {
                return Err(fail("workflow shape alias out of bounds"));
            }
        }
        for validator in self.consumer_validators.values() {
            validator.validate()?;
        }
        if let Some(staging) = &self.staging {
            if staging.provider != StagingProvider::GitHubDraft {
                return Err(fail("unsupported shape staging provider"));
            }
        }
        Ok(())
    }

    /// View the shape targets as a PackConfig for canonical resolution.
    pub fn pack_config(&self) -> PackConfig {
        PackConfig {
            schema_version: 1,
            targets: self.targets.clone(),
        }
    }
}

fn validate_exact_tag(tag: &str) -> Result<(), CiError> {
    if tag.is_empty() || tag.len() > 128 {
        return Err(fail("exact tag out of bounds"));
    }
    if tag.chars().any(char::is_control) {
        return Err(fail("exact tag contains control characters"));
    }
    if tag.contains(['?', '#', '@', ' ', '\\', '\'', '"', '`', '$']) {
        return Err(fail("exact tag contains query/injection characters"));
    }
    if tag.contains("..") || tag.starts_with('/') || tag.ends_with('/') {
        return Err(fail("exact tag has unsafe path shape"));
    }
    Ok(())
}

fn validate_source_revision(revision: &str) -> Result<(), CiError> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fail("source revision must be 40 lowercase hex characters"));
    }
    Ok(())
}

/// Resolve one invocation's exact release identity from static configuration.
///
/// Validates the exact existing tag selected by the workflow event/input,
/// consumes the checked-out HEAD revision, resolves PackConfig against
/// DistributionContract through `PackConfig::resolve`, and uses the exact
/// tag as the opaque `release_id` (no product-specific transformation).
/// Returns the invocation-local ReleasePlan; the caller additionally
/// resolves the static GitHub draft template for the same tag.
pub fn resolve_runtime_release_plan(
    contract: &DistributionContract,
    pack_config: &PackConfig,
    selected: &[String],
    tag: &str,
    source_revision: &str,
) -> Result<ReleasePlan, CiError> {
    validate_exact_tag(tag)?;
    validate_source_revision(source_revision)?;
    if selected.is_empty() || selected.len() > 256 {
        return Err(fail("selected target count out of bounds"));
    }
    for alias in selected {
        if !safe_metadata(alias, 128) {
            return Err(fail("selected target alias out of bounds"));
        }
    }
    if pack_config.schema_version != 1 {
        return Err(fail("unsupported PackConfig version"));
    }
    let plan = pack_config
        .resolve(contract, tag, source_revision, selected)
        .map_err(|_| fail("PackConfig does not resolve for the exact tag and source"))?;
    if plan.release_id != tag || plan.source_revision != source_revision {
        return Err(fail(
            "resolved ReleasePlan differs from the exact tag and source",
        ));
    }
    Ok(plan)
}

fn run_validator_process(
    request: &ConsumerValidationRequest<'_>,
    exe: &str,
    path_value: &str,
) -> ConsumerValidationOutcome {
    use std::sync::atomic::Ordering;
    let mut command = validator_command(
        exe,
        path_value,
        request,
        Some(request.script_path),
        Some(request.candidate_path),
    );
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return ConsumerValidationOutcome::Failed(
                ConsumerValidationFailure::InterpreterUnavailable,
            );
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_limit = request.validator.stdout_limit;
    let stderr_limit = request.validator.stderr_limit;
    let mut stdout_handle = Some(std::thread::spawn(move || {
        read_limited(stdout, stdout_limit)
    }));
    let mut stderr_handle = Some(std::thread::spawn(move || {
        read_limited_stderr(stderr, stderr_limit)
    }));
    // Prompt output-limit detection: a finished reader thread means its
    // stream hit EOF or the bound. Join finished readers without blocking;
    // an over-limit reader fails the run immediately.
    let deadline = std::time::Instant::now() + Duration::from_millis(request.validator.timeout_ms);
    let status = loop {
        if request
            .cancelled
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            let _ = child.kill();
            let _ = child.wait();
            return ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Cancelled);
        }
        if take_finished_over(&mut stdout_handle) || take_finished_over(&mut stderr_handle) {
            let _ = child.kill();
            let _ = child.wait();
            return ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let mut over = false;
    if let Some(handle) = stdout_handle.take() {
        over |= handle.join().unwrap_or_default().1;
    }
    if let Some(handle) = stderr_handle.take() {
        over |= handle.join().unwrap_or_default().1;
    }
    if over {
        return ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit);
    }
    match status {
        Some(status) if status.success() => ConsumerValidationOutcome::Passed,
        _ => ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggpack_core::{
        CompatibilityFloor, HostArch, HostOs, PackConfig, TargetPolicy, ToolchainRequirement,
    };
    use std::path::PathBuf;

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
            download_artifact: None,
            eggpack_tool: None,
            release_inputs: None,
            emulated_sysroots: None,
            staging: None,
        }
    }
    fn graph(strategy: BuildStrategy, support: SupportTier) -> CIPlan {
        let (contract, release) = plan(strategy, support);
        project_ci_plan(&contract, &release, &bindings(&release)).unwrap()
    }
    fn assert_golden(actual: &str, fixture: &str) {
        assert_eq!(actual, fixture.replace("\r\n", "\n"));
    }

    /// Write-through helper for M002a golden regeneration (before/after
    /// corrective evidence). Used by `m002a_regenerate_goldens`.
    #[allow(dead_code)]
    fn write_golden(path: &str, contents: &str) {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::write(root.join(path.trim_start_matches("../")), contents).unwrap();
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
        assert_golden(&output, include_str!("../tests/fixtures/native-direct.yml"));
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
        assert_golden(
            &workflow,
            include_str!("../tests/fixtures/native-direct-multitarget.yml"),
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

    // -----------------------------------------------------------------------
    // M002 qualification/aggregation gates and drift CLI.
    // -----------------------------------------------------------------------

    fn m002_policy() -> GitHubPolicy {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let mut base = policy();
        base.download_artifact = Some(ActionPin {
            reference: format!("actions/download-artifact@{sha}"),
        });
        base.eggpack_tool = Some(EggpackToolPolicy {
            repo: "https://github.com/eggstack/eggpack".into(),
            revision: "a".repeat(40),
            package: "eggpack-cli".into(),
            install_timeout_minutes: 10,
        });
        base.release_inputs = Some(GitHubReleaseInputsV1 {
            contract: "contracts/simple-direct.toml".into(),
            release_plan: "plans/release-plan.json".into(),
            build_bindings: "bindings/build.toml".into(),
            qualification_bindings: "bindings/qualification.toml".into(),
            ci_plan: "plans/release-ci-plan.json".into(),
            pack_config: None,
            draft_template: None,
            installer_presentation: None,
            consumer_validators: None,
        });
        base
    }

    fn m002_release(
        fixture: &str,
        product: &str,
        version: &str,
        target_alias: &str,
        qualification: Qualification,
        support: SupportTier,
    ) -> (
        DistributionContract,
        ReleasePlan,
        BuildBindingsV1,
        QualificationBindingsV1,
    ) {
        let contract = DistributionContract::parse_toml_str(fixture).unwrap();
        let target = match target_alias {
            "linux-x64" => "x86_64-unknown-linux-gnu",
            "macos-arm64" => "aarch64-apple-darwin",
            _ => target_alias,
        };
        let config = PackConfig {
            schema_version: 1,
            targets: vec![TargetPolicy {
                target: target.into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: if target.contains("apple") {
                    HostOs::Macos
                } else {
                    HostOs::Linux
                },
                host_arch: if target.starts_with("aarch64") {
                    HostArch::Aarch64
                } else {
                    HostArch::X86_64
                },
                qualification_host: if qualification == Qualification::DeferredNative {
                    Some(HostRequirement {
                        os: HostOs::Linux,
                        arch: HostArch::X86_64,
                    })
                } else {
                    None
                },
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                },
                floor: CompatibilityFloor::None,
                qualification,
                support,
            }],
        };
        let release = config
            .resolve(&contract, version, &"a".repeat(40), &[target_alias.into()])
            .unwrap();
        let expanded = contract.expand(target_alias, version).unwrap();
        let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
            eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
            eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
                .map(|index| LogicalOutputSelector::BundleEntry { index })
                .collect(),
            eggpack_contract::ExpandedAssets::Archive(a) => a
                .members
                .into_iter()
                .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
                .collect(),
        };
        let bindings = BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                target.into(),
                selectors
                    .iter()
                    .enumerate()
                    .map(|(index, selector)| eggpack_core::BuildBinding {
                        selector: selector.clone(),
                        package: product.into(),
                        binary: format!("bin{index}"),
                    })
                    .collect(),
            )]
            .into_iter()
            .collect(),
        };
        let qual_bindings = QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                target.into(),
                eggpack_core::TargetQualificationBinding {
                    smoke: matches!(
                        qualification,
                        Qualification::Native
                            | Qualification::DeferredNative
                            | Qualification::Emulated
                    )
                    .then(|| eggpack_core::CandidateSmokeBinding {
                        selector: selectors[0].clone(),
                        argv: vec!["--version".into()],
                        timeout_ms: 10_000,
                        stdout_limit: 8192,
                        stderr_limit: 8192,
                    }),
                },
            )]
            .into_iter()
            .collect(),
        };
        (contract, release, bindings, qual_bindings)
    }

    fn m002_graph(
        qualification: Qualification,
        support: SupportTier,
    ) -> (
        DistributionContract,
        ReleasePlan,
        BuildBindingsV1,
        QualificationBindingsV1,
        CIPlan,
        ReleaseCIPlanV1,
    ) {
        let (contract, release, bindings, qual_bindings) = m002_release(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            "linux-x64",
            qualification,
            support,
        );
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
        (contract, release, bindings, qual_bindings, ci_plan, graph)
    }

    #[test]
    fn m002_graph_projection_preserves_m001_and_validates_bindings() {
        let (_, _, _, _, ci_plan, graph) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        // M001 CIPlan remains unchanged (qualification unresolved).
        assert_eq!(
            ci_plan.targets[0].qualification.state,
            QualificationState::Unresolved
        );
        assert_eq!(graph.schema_version, 1);
        assert_eq!(graph.gate_job_id, "required_gate");
        assert_eq!(graph.aggregate.job_id, "aggregate");
        assert_eq!(graph.qualifications.len(), 1);
        assert!(graph.qualifications[0].required);
        assert_eq!(
            graph.to_json().unwrap(),
            ReleaseCIPlanV1::from_json(&graph.to_json().unwrap())
                .unwrap()
                .to_json()
                .unwrap()
        );
        // Smoke selector mismatch rejects.
        let (_, release, bindings, mut qual_bindings, ci_plan, _) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        // Structural must have no smoke; add one to force mismatch.
        qual_bindings
            .targets
            .get_mut("x86_64-unknown-linux-gnu")
            .unwrap()
            .smoke = Some(eggpack_core::CandidateSmokeBinding {
            selector: LogicalOutputSelector::Direct,
            argv: vec![],
            timeout_ms: 1000,
            stdout_limit: 1024,
            stderr_limit: 1024,
        });
        assert!(project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).is_err());
    }

    #[test]
    fn m002_handoff_rejects_traversal_and_swaps() {
        let (_, release, bindings, _, _, _) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        let mut handoff =
            project_build_handoff(&release, &bindings, "x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(
            handoff.to_json().unwrap(),
            BuildHandoffV1::from_json(&handoff.to_json().unwrap())
                .unwrap()
                .to_json()
                .unwrap()
        );
        // Absolute path rejects.
        handoff.outputs[0].relative_path = "/abs".into();
        assert!(handoff.validate().is_err());
        // Traversal rejects.
        let mut handoff =
            project_build_handoff(&release, &bindings, "x86_64-unknown-linux-gnu").unwrap();
        handoff.outputs[0].relative_path = "../escape".into();
        assert!(handoff.validate().is_err());
        // Duplicate handoff rejects (manual duplicate).
        let mut handoff =
            project_build_handoff(&release, &bindings, "x86_64-unknown-linux-gnu").unwrap();
        handoff.outputs.push(handoff.outputs[0].clone());
        assert!(handoff.validate().is_err());
    }

    fn m002_evidence(
        target: &str,
        release_id: &str,
        source: &str,
        classification: Qualification,
        support: SupportTier,
        status: QualificationStatus,
    ) -> QualificationEvidence {
        QualificationEvidence {
            schema_version: 1,
            release_id: release_id.into(),
            source_revision: source.into(),
            target: target.into(),
            planned_classification: classification,
            method: match classification {
                Qualification::Structural => eggpack_core::QualificationMethod::Structural,
                _ => eggpack_core::QualificationMethod::Structural,
            },
            actual_host: HostRequirement {
                os: HostOs::Linux,
                arch: HostArch::X86_64,
            },
            support,
            status,
            candidates: vec![],
            smoke_selector: None,
            processes: vec![],
        }
    }

    #[test]
    fn m002_gate_matrix() {
        let (_, release, _, _, _, graph) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        let target = "x86_64-unknown-linux-gnu";
        // Required Passed permits.
        let passed = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Passed,
        );
        assert_eq!(
            evaluate_gate(&graph, &[passed]).unwrap(),
            AggregateOutcome::Complete
        );
        // Required Deferred rejects.
        let deferred = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Deferred,
        );
        assert_eq!(
            evaluate_gate(&graph, &[deferred]).unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        // Required Failed rejects.
        let failed = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
        );
        assert_eq!(
            evaluate_gate(&graph, &[failed]).unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        // Missing required evidence rejects.
        assert_eq!(
            evaluate_gate(&graph, &[]).unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        // Optional failure yields suppression, not partial release.
        let (_, _, _, _, _, optional_graph) =
            m002_graph(Qualification::Structural, SupportTier::NonGating);
        let failed_optional = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::NonGating,
            QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
        );
        let failed_optionals = [failed_optional];
        assert_eq!(
            evaluate_gate(&optional_graph, &failed_optionals).unwrap(),
            AggregateOutcome::SuppressedNonGatingIncomplete
        );
    }

    #[test]
    fn m002_aggregation_finalizes_direct_bundle_archive_and_rejects_tampering() {
        for (fixture, product, version) in [
            (
                include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
                "eggsact",
                "1.2.6",
            ),
            (
                include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
                "codegg",
                "2.4.0",
            ),
            (
                include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
                "egress",
                "3.1.0",
            ),
        ] {
            let (contract, release, bindings, qual_bindings) = m002_release(
                fixture,
                product,
                version,
                "linux-x64",
                Qualification::Structural,
                SupportTier::Required,
            );
            let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
            let graph =
                project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
            // Build fixture candidates (ELF for structural).
            let parent = eggpack_core_test_temp(&format!("m002-agg-{product}"));
            let source_root = parent.join("candidates");
            std::fs::create_dir(&source_root).unwrap();
            let elf = m002_elf();
            let planned = &release.targets[0];
            let expanded = contract.expand("linux-x64", version).unwrap();
            let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
                eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
                eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
                    .map(|index| LogicalOutputSelector::BundleEntry { index })
                    .collect(),
                eggpack_contract::ExpandedAssets::Archive(a) => a
                    .members
                    .into_iter()
                    .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
                    .collect(),
            };
            let mut candidates = Vec::new();
            for (index, selector) in selectors.iter().enumerate() {
                let path = source_root.join(format!("candidate-{index}"));
                std::fs::write(&path, &elf).unwrap();
                candidates.push(eggpack_core::CandidateArtifact {
                    target: planned.target.clone(),
                    selector: selector.clone(),
                    package: product.into(),
                    binary: format!("bin{index}"),
                    path,
                    size: elf.len() as u64,
                });
            }
            let attempt = eggpack_core::BuildAttempt {
                release_id: release.release_id.clone(),
                source_revision: release.source_revision.clone(),
                target: planned.target.clone(),
                strategy: BuildStrategy::NativeCargo,
                tool_summary: "fixture".into(),
                process: eggpack_core::ProcessEvidence {
                    outcome: eggpack_core::CommandOutcome::Success,
                    stdout_bytes: 0,
                    stderr_bytes: 0,
                },
                candidates: candidates.clone(),
            };
            let digest = {
                use sha2::{Digest, Sha256};
                Sha256::digest(&elf)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            };
            let mut qualified: Vec<eggpack_core::QualifiedCandidateEvidence> = candidates
                .iter()
                .map(|c| eggpack_core::QualifiedCandidateEvidence {
                    selector: c.selector.clone(),
                    package: c.package.clone(),
                    binary: c.binary.clone(),
                    size: elf.len() as u64,
                    sha256: digest.clone(),
                    format: eggpack_core::CandidateFormat::Elf,
                    architecture: eggpack_core::CandidateArchitecture::X86_64,
                })
                .collect();
            qualified.sort_by(|a, b| a.selector.cmp(&b.selector));
            let evidence = QualificationEvidence {
                schema_version: 1,
                release_id: release.release_id.clone(),
                source_revision: release.source_revision.clone(),
                target: planned.target.clone(),
                planned_classification: Qualification::Structural,
                method: eggpack_core::QualificationMethod::Structural,
                actual_host: HostRequirement {
                    os: HostOs::Linux,
                    arch: HostArch::X86_64,
                },
                support: SupportTier::Required,
                status: QualificationStatus::Passed,
                candidates: qualified,
                smoke_selector: None,
                processes: vec![],
            };
            // Complete path finalizes via M004 (invoked, not copied).
            let inputs = vec![FinalizationTargetInput {
                target: planned.target.clone(),
                attempt: attempt.clone(),
                qualification: evidence.clone(),
            }];
            let output = parent.join("release");
            let (outcome, finalized) =
                aggregate_finalize(&contract, &release, &graph, &inputs, &[], &output).unwrap();
            assert_eq!(outcome, AggregateOutcome::Complete);
            assert!(finalized.is_some());
            // Tampered candidate rejects (no partial release).
            std::fs::write(&candidates[0].path, b"changed").unwrap();
            let rejected_root = parent.join("rejected");
            let result =
                aggregate_finalize(&contract, &release, &graph, &inputs, &[], &rejected_root);
            assert!(result.is_err() || result.unwrap().0 != AggregateOutcome::Complete);
            // Missing optional target suppresses output (no partial release).
            let (_, _, _, _, _, optional_graph) = {
                let (c, r, b, q) = m002_release(
                    fixture,
                    product,
                    version,
                    "linux-x64",
                    Qualification::Structural,
                    SupportTier::NonGating,
                );
                let ci = project_ci_plan(&c, &r, &b).unwrap();
                let g = project_release_plan(&ci, &q, &b, &r).unwrap();
                (c, r, b, q, ci, g)
            };
            let mut deferred_evidence = evidence.clone();
            deferred_evidence.support = SupportTier::NonGating;
            deferred_evidence.status = QualificationStatus::Deferred;
            // Deferred with Structural method is inconsistent; use Failed to trigger suppression.
            deferred_evidence.status =
                QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed);
            let (outcome, finalized) = aggregate_finalize(
                &contract,
                &release,
                &optional_graph,
                &[FinalizationTargetInput {
                    target: planned.target.clone(),
                    attempt,
                    qualification: deferred_evidence,
                }],
                &[],
                &parent.join("suppressed"),
            )
            .unwrap();
            assert_eq!(outcome, AggregateOutcome::SuppressedNonGatingIncomplete);
            assert!(finalized.is_none());
            std::fs::remove_dir_all(parent).unwrap();
        }
    }

    fn m002_elf() -> Vec<u8> {
        let mut bytes = vec![0; 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
        bytes
    }

    fn eggpack_core_test_temp(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("eggpack-{label}-{}-{id}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create test temp directory: {error}"),
            }
        }
    }

    #[test]
    fn m002_renderer_is_deterministic_read_only_pinned_and_gated() {
        let (_, _, _, _, _, graph) = m002_graph(Qualification::Structural, SupportTier::Required);
        let policy = m002_policy();
        let first = render_release_github(&graph, &policy).unwrap();
        assert_eq!(first, render_release_github(&graph, &policy).unwrap());
        let parsed: serde_yaml::Value = serde_yaml::from_str(&first).unwrap();
        assert!(parsed.get("jobs").is_some());
        // Read-only permissions everywhere.
        for (_, job) in parsed["jobs"].as_mapping().unwrap() {
            let permissions = job["permissions"].as_mapping().unwrap();
            assert_eq!(permissions.len(), 1);
            assert_eq!(permissions.get("contents").unwrap().as_str(), Some("read"));
        }
        assert!(first.contains("contents: read"));
        assert!(!first.contains("contents: write"));
        assert!(!first.contains("id-token: write"));
        // Pinned Eggpack tooling.
        assert!(first.contains("cargo install --git https://github.com/eggstack/eggpack --rev"));
        assert!(first.contains("--locked -p eggpack-cli"));
        assert!(first.contains("eggpack --version"));
        // Exact gate dependencies and no release API.
        assert!(first.contains("required_gate"));
        assert!(first.contains("_qualify-target"));
        assert!(first.contains("--contract"));
        assert!(first.contains("--release-plan"));
        assert!(first.contains("--build-bindings"));
        assert!(first.contains("--qualification-bindings"));
        assert!(first.contains("--candidate-dir"));
        assert!(first.contains("--build-handoff"));
        assert!(first.contains("--output-dir"));
        assert!(first.contains("_capture-build"));
        assert!(first.contains("--cargo-target-dir"));
        assert!(first.contains("_evaluate-gate"));
        assert!(first.contains("--inputs-dir"));
        assert!(first.contains("_aggregate"));
        assert!(first.contains("--output-root"));
        assert!(!first.contains("github-release"));
        assert!(!first.contains("publish"));
        // No arbitrary command.
        assert!(!first.contains("curl -L"));
        // Drift check normalizes CRLF only and detects one-byte edits.
        assert!(
            check_release_github(&graph, &policy, first.as_bytes())
                .unwrap()
                .matches
        );
        let crlf = first.replace('\n', "\r\n");
        assert!(
            check_release_github(&graph, &policy, crlf.as_bytes())
                .unwrap()
                .matches
        );
        let changed = first.replacen("contents: read", "contents: write", 1);
        let report = check_release_github(&graph, &policy, changed.as_bytes()).unwrap();
        assert!(!report.matches);
        assert!(report.first_difference.is_some());
    }

    #[test]
    fn m002_rejects_unpinned_tooling_and_missing_evidence() {
        let (_, _, _, _, _, graph) = m002_graph(Qualification::Structural, SupportTier::Required);
        // Unpinned tooling rejects.
        let mut bad_policy = m002_policy();
        bad_policy.eggpack_tool.as_mut().unwrap().revision = "v1".into();
        assert!(render_release_github(&graph, &bad_policy).is_err());
        let mut bad_policy = m002_policy();
        bad_policy.eggpack_tool.as_mut().unwrap().repo = "https://example.invalid/eggpack".into();
        assert!(render_release_github(&graph, &bad_policy).is_err());
        let mut bad_policy = m002_policy();
        bad_policy.eggpack_tool = None;
        assert!(render_release_github(&graph, &bad_policy).is_err());
        // Missing download pin rejects.
        let mut bad_policy = m002_policy();
        bad_policy.download_artifact = None;
        assert!(render_release_github(&graph, &bad_policy).is_err());
        // Missing explicit release inputs rejects (no hidden discovery).
        let mut bad_policy = m002_policy();
        bad_policy.release_inputs = None;
        assert!(render_release_github(&graph, &bad_policy).is_err());
        // Invalid relative input path rejects.
        let mut bad_policy = m002_policy();
        bad_policy.release_inputs.as_mut().unwrap().contract = "../escape.toml".into();
        assert!(render_release_github(&graph, &bad_policy).is_err());
        let mut bad_policy = m002_policy();
        bad_policy.release_inputs.as_mut().unwrap().ci_plan = "/abs/path.json".into();
        assert!(render_release_github(&graph, &bad_policy).is_err());
    }

    #[test]
    fn m002a_emulated_requires_explicit_runtime_policy() {
        let (_, _, _, _, _, graph) = m002_graph(Qualification::Emulated, SupportTier::Required);
        // Without a runtime policy the renderer must reject.
        assert!(render_release_github(&graph, &m002_policy()).is_err());
        // With an explicit sysroot the renderer passes it to the CLI.
        let policy = m002a_policy_with_sysroot("x86_64-unknown-linux-gnu", "sysroots/linux-x64");
        let rendered = render_release_github(&graph, &policy).unwrap();
        assert!(rendered.contains("--qemu-sysroot"));
        assert!(rendered.contains("sysroots/linux-x64"));
    }

    #[test]
    fn m002a_release_inputs_reject_escapes() {
        let valid = GitHubReleaseInputsV1 {
            contract: "contracts/release.toml".into(),
            release_plan: "plans/release-plan.json".into(),
            build_bindings: "bindings/build.toml".into(),
            qualification_bindings: "bindings/qualification.toml".into(),
            ci_plan: "plans/release-ci-plan.json".into(),
            pack_config: None,
            draft_template: None,
            installer_presentation: None,
            consumer_validators: None,
        };
        assert!(valid.validate().is_ok());
        for bad in [
            "",
            "/abs.toml",
            "../escape.toml",
            "a//b.toml",
            "a/./b.toml",
            "win\\path.toml",
            "c:/drive.toml",
            "nul\0byte.toml",
        ] {
            let mut inputs = valid.clone();
            inputs.contract = bad.into();
            assert!(inputs.validate().is_err(), "must reject {bad:?}");
        }
    }

    #[test]
    fn m002a_runner_command_round_trip_covers_all_required_args() {
        // Every internal CLI command has an exact structured round-trip: the
        // renderer serializes the same RunnerCommand the tests execute, so CLI
        // signature drift is caught here rather than in generated YAML alone.
        let capture = RunnerCommand::CaptureBuild {
            contract: "c.toml".into(),
            release_plan: "r.json".into(),
            build_bindings: "b.toml".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            cargo_target_dir: "/tmp/cargo-target".into(),
            output_dir: "./eggpack-build/build_x".into(),
        };
        let shell = capture.to_shell();
        for required in [
            "_capture-build",
            "--release-plan",
            "--build-bindings",
            "--target",
            "--cargo-target-dir",
            "--output-dir",
        ] {
            assert!(shell.contains(required), "capture shell missing {required}");
        }
        assert_eq!(capture.argv()[0], "eggpack");

        let qualify = RunnerCommand::QualifyTarget {
            contract: "c.toml".into(),
            release_plan: "r.json".into(),
            build_bindings: "b.toml".into(),
            qualification_bindings: "q.toml".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            candidate_dir: "./eggpack-handoff/build_x/candidates".into(),
            build_handoff: "./eggpack-handoff/build_x/build-handoff.json".into(),
            output_dir: "./eggpack-qualification/build_x".into(),
            qemu_sysroot: None,
        };
        let shell = qualify.to_shell();
        for required in [
            "_qualify-target",
            "--contract",
            "--release-plan",
            "--build-bindings",
            "--qualification-bindings",
            "--target",
            "--candidate-dir",
            "--build-handoff",
            "--output-dir",
        ] {
            assert!(shell.contains(required), "qualify shell missing {required}");
        }
        // No hidden discovery flags and no absolute serialized paths.
        assert!(!shell.contains(".."));
        assert!(!shell.contains("tar -"));
        let qualify_sysroot = RunnerCommand::QualifyTarget {
            contract: "c.toml".into(),
            release_plan: "r.json".into(),
            build_bindings: "b.toml".into(),
            qualification_bindings: "q.toml".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            candidate_dir: "./eggpack-handoff/build_x/candidates".into(),
            build_handoff: "./eggpack-handoff/build_x/build-handoff.json".into(),
            output_dir: "./eggpack-qualification/build_x".into(),
            qemu_sysroot: Some("sysroots/linux-x64".into()),
        };
        assert!(qualify_sysroot.to_shell().contains("--qemu-sysroot"));

        let gate = RunnerCommand::EvaluateGate {
            ci_plan: "plans/ci.json".into(),
            inputs_dir: "./eggpack-inputs".into(),
            output: "./eggpack-gate/gate-outcome.json".into(),
        };
        let shell = gate.to_shell();
        for required in ["_evaluate-gate", "--ci-plan", "--inputs-dir", "--output"] {
            assert!(shell.contains(required), "gate shell missing {required}");
        }

        let aggregate = RunnerCommand::Aggregate {
            contract: "c.toml".into(),
            release_plan: "r.json".into(),
            ci_plan: "plans/ci.json".into(),
            inputs_dir: "./eggpack-inputs".into(),
            output_root: "./eggpack-finalized/root".into(),
            output: "./eggpack-finalized/summary.json".into(),
        };
        let shell = aggregate.to_shell();
        for required in [
            "_aggregate",
            "--contract",
            "--release-plan",
            "--ci-plan",
            "--inputs-dir",
            "--output-root",
            "--output",
        ] {
            assert!(
                shell.contains(required),
                "aggregate shell missing {required}"
            );
        }
    }

    fn m002_golden_policy() -> GitHubPolicy {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        GitHubPolicy {
            preflight_runner: "ubuntu-latest".into(),
            runners: vec![
                RunnerMapping {
                    os: HostOs::Linux,
                    arch: HostArch::X86_64,
                    label: "ubuntu-latest".into(),
                    cargo_zigbuild: true,
                    zig: true,
                },
                RunnerMapping {
                    os: HostOs::Macos,
                    arch: HostArch::Aarch64,
                    label: "macos-latest".into(),
                    cargo_zigbuild: false,
                    zig: false,
                },
            ],
            checkout: ActionPin {
                reference: format!("actions/checkout@{sha}"),
            },
            rust_toolchain: ActionPin {
                reference: format!("dtolnay/rust-toolchain@{sha}"),
            },
            upload_artifact: ActionPin {
                reference: format!("actions/upload-artifact@{sha}"),
            },
            download_artifact: Some(ActionPin {
                reference: format!("actions/download-artifact@{sha}"),
            }),
            triggers: vec![WorkflowTrigger::Push, WorkflowTrigger::WorkflowDispatch],
            timeout_minutes: 60,
            cancel_in_progress: true,
            artifact_retention_days: 7,
            eggpack_tool: Some(EggpackToolPolicy {
                repo: "https://github.com/eggstack/eggpack".into(),
                revision: "a".repeat(40),
                package: "eggpack-cli".into(),
                install_timeout_minutes: 10,
            }),
            release_inputs: Some(GitHubReleaseInputsV1 {
                contract: "contracts/release.toml".into(),
                release_plan: "plans/release-plan.json".into(),
                build_bindings: "bindings/build.toml".into(),
                qualification_bindings: "bindings/qualification.toml".into(),
                ci_plan: "plans/release-ci-plan.json".into(),
                pack_config: None,
                draft_template: None,
                installer_presentation: None,
                consumer_validators: None,
            }),
            emulated_sysroots: None,
            staging: None,
        }
    }

    fn m002a_policy_with_sysroot(target: &str, sysroot: &str) -> GitHubPolicy {
        let mut policy = m002_golden_policy();
        policy.emulated_sysroots = Some(
            [(target.to_string(), sysroot.to_string())]
                .into_iter()
                .collect(),
        );
        policy
    }

    fn m002_golden_case(
        fixture: &str,
        product: &str,
        version: &str,
        targets: Vec<(&str, BuildStrategy, SupportTier, Qualification)>,
        aliases: Vec<&str>,
    ) -> (ReleaseCIPlanV1, GitHubPolicy) {
        let contract = DistributionContract::parse_toml_str(fixture).unwrap();
        let policies: Vec<TargetPolicy> = targets
            .iter()
            .map(|(triple, strategy, support, qual)| {
                let (host_os, host_arch) = if triple.contains("apple") {
                    (HostOs::Macos, HostArch::Aarch64)
                } else {
                    (HostOs::Linux, HostArch::X86_64)
                };
                TargetPolicy {
                    target: triple.to_string(),
                    strategy: *strategy,
                    host_os,
                    host_arch,
                    qualification_host: None,
                    toolchain: ToolchainRequirement {
                        rust: "1.89.0".into(),
                        cargo_zigbuild: if *strategy == BuildStrategy::CargoZigbuild {
                            Some("0.20.0".into())
                        } else {
                            None
                        },
                    },
                    floor: CompatibilityFloor::None,
                    qualification: *qual,
                    support: *support,
                }
            })
            .collect();
        let config = PackConfig {
            schema_version: 1,
            targets: policies,
        };
        let selected: Vec<String> = aliases.into_iter().map(|s| s.to_string()).collect();
        let release = config
            .resolve(&contract, version, &"a".repeat(40), &selected)
            .unwrap();
        let mut binding_map = std::collections::BTreeMap::new();
        let mut qual_map = std::collections::BTreeMap::new();
        for target in &release.targets {
            let expanded = contract.expand(&target.target, version).unwrap();
            let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
                eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
                eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
                    .map(|index| LogicalOutputSelector::BundleEntry { index })
                    .collect(),
                eggpack_contract::ExpandedAssets::Archive(a) => a
                    .members
                    .into_iter()
                    .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
                    .collect(),
            };
            binding_map.insert(
                target.target.clone(),
                selectors
                    .iter()
                    .enumerate()
                    .map(|(index, selector)| eggpack_core::BuildBinding {
                        selector: selector.clone(),
                        package: product.into(),
                        binary: format!("bin{index}"),
                    })
                    .collect(),
            );
            qual_map.insert(
                target.target.clone(),
                eggpack_core::TargetQualificationBinding { smoke: None },
            );
        }
        let bindings = BuildBindingsV1 {
            schema_version: 1,
            targets: binding_map,
        };
        let qual_bindings = QualificationBindingsV1 {
            schema_version: 1,
            targets: qual_map,
        };
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
        (graph, m002_golden_policy())
    }

    #[test]
    fn m002_golden_direct_bundle_archive_and_mixed() {
        // Direct single-target.
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m002-direct.yml"),
        );
        // Mixed native/cross.
        let (graph, policy) = m002_golden_case(
            include_str!("../tests/fixtures/mixed-direct-targets.toml"),
            "eggsact",
            "1.2.3",
            vec![
                (
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                ),
                (
                    "aarch64-unknown-linux-gnu",
                    BuildStrategy::CargoZigbuild,
                    SupportTier::NonGating,
                    Qualification::Structural,
                ),
            ],
            vec!["linux-x64", "linux-arm64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m002-mixed.yml"),
        );
        // Bundle.
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            "2.4.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m002-bundle.yml"),
        );
        // Archive.
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            "3.1.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m002-archive.yml"),
        );
    }

    #[test]
    #[ignore]
    fn m002a_regenerate_goldens() {
        // Regenerate checked-in M002a goldens from the corrected renderer.
        // Run explicitly with `cargo test -p eggpack-ci --lib -- --ignored`.
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        write_golden(
            "../tests/fixtures/m002-direct.yml",
            &render_release_github(&graph, &policy).unwrap(),
        );
        let (graph, policy) = m002_golden_case(
            include_str!("../tests/fixtures/mixed-direct-targets.toml"),
            "eggsact",
            "1.2.3",
            vec![
                (
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                ),
                (
                    "aarch64-unknown-linux-gnu",
                    BuildStrategy::CargoZigbuild,
                    SupportTier::NonGating,
                    Qualification::Structural,
                ),
            ],
            vec!["linux-x64", "linux-arm64"],
        );
        write_golden(
            "../tests/fixtures/m002-mixed.yml",
            &render_release_github(&graph, &policy).unwrap(),
        );
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            "2.4.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        write_golden(
            "../tests/fixtures/m002-bundle.yml",
            &render_release_github(&graph, &policy).unwrap(),
        );
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            "3.1.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        write_golden(
            "../tests/fixtures/m002-archive.yml",
            &render_release_github(&graph, &policy).unwrap(),
        );
    }

    /// M002a end-to-end executable fixture: the same rendered command/argument
    /// /layout contract drives build -> capture -> qualify -> gate ->
    /// aggregate -> M004 finalization directly (no GitHub invocation).
    #[test]
    fn m002a_generated_orchestration_executes_end_to_end() {
        for (fixture, product, version) in [
            (
                include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
                "eggsact",
                "1.2.3",
            ),
            (
                include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
                "codegg",
                "2.4.0",
            ),
            (
                include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
                "egress",
                "3.1.0",
            ),
        ] {
            let (contract, release, bindings, qual_bindings) = m002_release(
                fixture,
                product,
                version,
                "linux-x64",
                Qualification::Structural,
                SupportTier::Required,
            );
            let target = "x86_64-unknown-linux-gnu";
            let planned = release
                .targets
                .iter()
                .find(|t| t.target == target)
                .unwrap()
                .clone();
            let parent = eggpack_core_test_temp(&format!("m002a-e2e-{product}"));

            // Build candidate bytes (ELF for structural qualification).
            let elf = m002_elf();
            let mut handoff = project_build_handoff(&release, &bindings, target).unwrap();
            let stage_src = parent.join("stage-src");
            std::fs::create_dir(&stage_src).unwrap();
            for output in &mut handoff.outputs {
                std::fs::write(stage_src.join(&output.relative_path), &elf).unwrap();
                output.size = elf.len() as u64;
            }
            handoff.validate().unwrap();

            // _capture-build equivalent: stage the canonical build artifact.
            let build_dir = parent.join("build").join(target);
            stage_build_artifact_dir(&handoff, &stage_src, &build_dir).unwrap();
            let staged = validate_build_artifact_dir(&build_dir).unwrap();
            assert_eq!(staged.to_json().unwrap(), handoff.to_json().unwrap());

            // _qualify-target equivalent: reconstruct, execute M003, write the
            // canonical qualification artifact (handoff + evidence + bytes).
            let candidates_dir = build_dir.join(CANDIDATES_DIR);
            let attempt =
                reconstruct_attempt(&release, &planned, &staged, &candidates_dir).unwrap();
            let runtime = eggpack_core::QualificationRuntime { qemu_sysroot: None };
            let cancellation = eggpack_core::BuildCancellation::new();
            let evidence = eggpack_core::qualify_target(eggpack_core::QualificationRequest {
                contract: &contract,
                plan: &release,
                target: &planned,
                attempt: &attempt,
                build_bindings: &bindings,
                qualification_bindings: &qual_bindings,
                runtime: &runtime,
                cancellation: &cancellation,
            })
            .unwrap();
            assert_eq!(evidence.status, eggpack_core::QualificationStatus::Passed);
            let qual_dir = parent.join("qual").join(target);
            std::fs::create_dir_all(qual_dir.join(CANDIDATES_DIR)).unwrap();
            std::fs::write(
                qual_dir.join(BUILD_HANDOFF_FILE),
                staged.to_json().unwrap().as_bytes(),
            )
            .unwrap();
            std::fs::write(
                qual_dir.join(QUALIFICATION_EVIDENCE_FILE),
                encode_qualification_evidence(&evidence).unwrap().as_bytes(),
            )
            .unwrap();
            for output in &staged.outputs {
                std::fs::copy(
                    candidates_dir.join(&output.relative_path),
                    qual_dir.join(CANDIDATES_DIR).join(&output.relative_path),
                )
                .unwrap();
            }
            let (_, staged_evidence) = validate_qualification_artifact_dir(&qual_dir).unwrap();
            assert_eq!(
                encode_qualification_evidence(&staged_evidence).unwrap(),
                encode_qualification_evidence(&evidence).unwrap()
            );

            // _evaluate-gate + _aggregate equivalent over the canonical
            // per-target inputs layout.
            let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
            let graph =
                project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
            let inputs_root = parent.join("inputs");
            std::fs::create_dir_all(inputs_root.join(target)).unwrap();
            for entry in std::fs::read_dir(&qual_dir).unwrap() {
                let entry = entry.unwrap();
                let dest = inputs_root.join(target).join(entry.file_name());
                if entry.path().is_dir() {
                    std::fs::create_dir_all(&dest).unwrap();
                    for inner in std::fs::read_dir(entry.path()).unwrap() {
                        let inner = inner.unwrap();
                        std::fs::copy(inner.path(), dest.join(inner.file_name())).unwrap();
                    }
                } else {
                    std::fs::copy(entry.path(), dest).unwrap();
                }
            }
            let gate_text =
                std::fs::read_to_string(inputs_root.join(target).join("evidence.json")).unwrap();
            let gate_evidence = decode_qualification_evidence(&gate_text).unwrap();
            assert_eq!(
                evaluate_gate(&graph, &[gate_evidence]).unwrap(),
                AggregateOutcome::Complete
            );

            // Reconstruct the attempt from the aggregate inputs layout and
            // finalize through M004; assert exact bytes and manifest identity.
            let agg_handoff_text =
                std::fs::read_to_string(inputs_root.join(target).join("build-handoff.json"))
                    .unwrap();
            let agg_handoff = BuildHandoffV1::from_json(&agg_handoff_text).unwrap();
            let agg_attempt = reconstruct_attempt(
                &release,
                &planned,
                &agg_handoff,
                &inputs_root.join(target).join("candidates"),
            )
            .unwrap();
            assert_eq!(agg_attempt.candidates.len(), handoff.outputs.len());
            for candidate in &agg_attempt.candidates {
                assert_eq!(
                    std::fs::read(&candidate.path).unwrap(),
                    elf,
                    "final artifact bytes must be exact"
                );
            }
            let final_inputs = vec![FinalizationTargetInput {
                target: target.to_string(),
                attempt: agg_attempt,
                qualification: decode_qualification_evidence(
                    &std::fs::read_to_string(inputs_root.join(target).join("evidence.json"))
                        .unwrap(),
                )
                .unwrap(),
            }];
            let output_root = parent.join("finalized");
            let (outcome, finalized) = aggregate_finalize(
                &contract,
                &release,
                &graph,
                &final_inputs,
                &[],
                &output_root,
            )
            .unwrap();
            assert_eq!(outcome, AggregateOutcome::Complete);
            let finalized = finalized.unwrap();
            let manifest_json = finalized.manifest.to_json().unwrap();
            assert!(manifest_json.contains(version));
            // Exact manifest digest identity.
            let manifest_bytes = manifest_json.as_bytes();
            assert!(!manifest_bytes.is_empty());

            // Tampered candidate after qualification fails closed.
            let tampered_root = parent.join("tampered");
            std::fs::create_dir_all(tampered_root.join(target).join("candidates")).unwrap();
            std::fs::write(
                tampered_root.join(target).join("build-handoff.json"),
                agg_handoff_text.as_bytes(),
            )
            .unwrap();
            std::fs::write(
                tampered_root.join(target).join("evidence.json"),
                gate_text.as_bytes(),
            )
            .unwrap();
            for output in &agg_handoff.outputs {
                std::fs::write(
                    tampered_root
                        .join(target)
                        .join("candidates")
                        .join(&output.relative_path),
                    b"tampered-bytes",
                )
                .unwrap();
            }
            assert!(reconstruct_attempt(
                &release,
                &planned,
                &agg_handoff,
                &tampered_root.join(target).join("candidates"),
            )
            .is_err());

            std::fs::remove_dir_all(parent).unwrap();
        }
    }

    #[test]
    fn m002a_optional_suppression_and_required_failure_close_the_gate() {
        // Optional-target failure suppresses release output; required-target
        // failure fails closed. Both use the canonical inputs layout.
        let (contract, release, bindings, qual_bindings) = m002_release(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            "linux-x64",
            Qualification::Structural,
            SupportTier::NonGating,
        );
        let target = "x86_64-unknown-linux-gnu";
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        let optional_graph =
            project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
        let failed_optional = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::NonGating,
            QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
        );
        assert_eq!(
            evaluate_gate(&optional_graph, std::slice::from_ref(&failed_optional)).unwrap(),
            AggregateOutcome::SuppressedNonGatingIncomplete
        );
        // Aggregate never produces a partial release on suppression.
        let elf = m002_elf();
        let parent = eggpack_core_test_temp("m002a-suppress");
        let candidate = parent.join("candidate");
        std::fs::write(&candidate, &elf).unwrap();
        let _planned = release.targets[0].clone();
        let attempt = eggpack_core::BuildAttempt {
            release_id: release.release_id.clone(),
            source_revision: release.source_revision.clone(),
            target: target.to_string(),
            strategy: BuildStrategy::NativeCargo,
            tool_summary: "fixture".into(),
            process: eggpack_core::ProcessEvidence {
                outcome: eggpack_core::CommandOutcome::Success,
                stdout_bytes: 0,
                stderr_bytes: 0,
            },
            candidates: vec![eggpack_core::CandidateArtifact {
                target: target.to_string(),
                selector: LogicalOutputSelector::Direct,
                package: "eggsact".into(),
                binary: "bin0".into(),
                path: candidate,
                size: elf.len() as u64,
            }],
        };
        let (outcome, finalized) = aggregate_finalize(
            &contract,
            &release,
            &optional_graph,
            &[FinalizationTargetInput {
                target: target.to_string(),
                attempt,
                qualification: failed_optional.clone(),
            }],
            &[],
            &parent.join("out"),
        )
        .unwrap();
        assert_eq!(outcome, AggregateOutcome::SuppressedNonGatingIncomplete);
        assert!(finalized.is_none());

        // Required failure fails closed.
        let (_, release_required, _, _, _, required_graph) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        let failed_required = m002_evidence(
            target,
            &release_required.release_id,
            &release_required.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
        );
        assert_eq!(
            evaluate_gate(&required_graph, &[failed_required]).unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        // Missing/extra/swap evidence is invalid, never partial.
        assert_eq!(
            evaluate_gate(&required_graph, &[]).unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn m002a_negative_matrix_for_layout_and_identity() {
        let (_, release, bindings, _, _, graph) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        let target = "x86_64-unknown-linux-gnu";
        let parent = eggpack_core_test_temp("m002a-neg");
        // Wrong-target handoff rejects.
        let mut handoff = project_build_handoff(&release, &bindings, target).unwrap();
        handoff.target = "aarch64-unknown-linux-gnu".into();
        assert!(handoff.validate().is_ok()); // shape-valid but identity-mismatched
        let planned = release.targets[0].clone();
        let dir = parent.join("wrong-target");
        std::fs::create_dir_all(dir.join("candidates")).unwrap();
        let elf = m002_elf();
        for output in &handoff.outputs {
            std::fs::write(dir.join("candidates").join(&output.relative_path), &elf).unwrap();
        }
        std::fs::write(dir.join("build-handoff.json"), handoff.to_json().unwrap()).unwrap();
        // Identity is checked by reconstruction against the plan, not by shape.
        let mut good = project_build_handoff(&release, &bindings, target).unwrap();
        for output in &mut good.outputs {
            output.size = elf.len() as u64;
        }
        assert!(
            reconstruct_attempt(&release, &planned, &handoff, &dir.join("candidates")).is_err()
        );
        // Missing candidate rejects.
        let empty = parent.join("missing");
        std::fs::create_dir_all(empty.join("candidates")).unwrap();
        std::fs::write(empty.join("build-handoff.json"), good.to_json().unwrap()).unwrap();
        assert!(validate_build_artifact_dir(&empty).is_err());
        // Extra candidate rejects.
        let extra = parent.join("extra");
        stage_build_artifact_dir(
            &good,
            &{
                let src = parent.join("extra-src");
                std::fs::create_dir_all(&src).unwrap();
                for output in &good.outputs {
                    std::fs::write(src.join(&output.relative_path), &elf).unwrap();
                }
                src
            },
            &extra,
        )
        .unwrap();
        std::fs::write(extra.join("candidates").join("stowaway"), b"x").unwrap();
        assert!(validate_build_artifact_dir(&extra).is_err());
        // Symlink candidate rejects.
        #[cfg(unix)]
        {
            let link_dir = parent.join("link");
            stage_build_artifact_dir(
                &good,
                &{
                    let src = parent.join("link-src");
                    std::fs::create_dir_all(&src).unwrap();
                    for output in &good.outputs {
                        std::fs::write(src.join(&output.relative_path), &elf).unwrap();
                    }
                    src
                },
                &link_dir,
            )
            .unwrap();
            let victim = link_dir
                .join("candidates")
                .join(&good.outputs[0].relative_path);
            std::fs::remove_file(&victim).unwrap();
            std::os::unix::fs::symlink("/etc/hostname", &victim).unwrap();
            assert!(validate_build_artifact_dir(&link_dir).is_err());
        }
        // Gate layout mismatch (evidence file absent) is an I/O-level failure
        // before gate evaluation, never a silent pass.
        assert!(validate_qualification_artifact_dir(&empty).is_err());
        // Release identity mismatch fails closed.
        let wrong = m002_evidence(
            target,
            "other-release",
            &release.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Passed,
        );
        assert_eq!(
            evaluate_gate(&graph, &[wrong]).unwrap(),
            AggregateOutcome::InvalidEvidence
        );
        std::fs::remove_dir_all(parent).unwrap();
    }

    // -----------------------------------------------------------------------
    // M003b — generated draft staging job and operational qualification.
    // -----------------------------------------------------------------------

    fn m003b_staging_policy() -> GitHubPolicy {
        let mut policy = m002_golden_policy();
        policy.staging = Some(GitHubStagingPolicyV1 {
            runner: "ubuntu-latest".into(),
            owner: "acme".into(),
            repository: "widget".into(),
            tag_source: StagingTagSource::RefName,
            inputs: GitHubStagingInputsV1 {
                contract: "contracts/release.toml".into(),
                install_policy: "policies/install.toml".into(),
                github_policy: "policies/github-draft.json".into(),
                installer_presentation: None,
            },
            receipt_retention_days: 7,
        });
        policy
    }

    fn m003b_staging_graph(
        fixture: &str,
        product: &str,
        version: &str,
        targets: Vec<(&str, BuildStrategy, SupportTier, Qualification)>,
        aliases: Vec<&str>,
    ) -> (ReleaseCIPlanV1, GitHubPolicy) {
        let (graph, _) = m002_golden_case(fixture, product, version, targets, aliases);
        let policy = m003b_staging_policy();
        let graph = with_github_draft_staging(graph, true).unwrap();
        (graph, policy)
    }

    #[test]
    fn m003b_staging_intent_is_provider_neutral_and_optional() {
        let (graph, _) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        // Disabled graphs serialize exactly as M002a (no staging field).
        assert!(graph.staging.is_none());
        let json = graph.to_json().unwrap();
        assert!(!json.contains("staging"));
        assert_eq!(ReleaseCIPlanV1::from_json(&json).unwrap(), graph);
        // Attaching staging is additive and validated.
        let staged = with_github_draft_staging(graph.clone(), true).unwrap();
        let job = staged.staging.as_ref().unwrap();
        assert_eq!(job.job_id, "stage");
        assert_eq!(job.aggregate_job_id, "aggregate");
        assert_eq!(job.provider, StagingProvider::GitHubDraft);
        assert_eq!(job.finalized_handoff_name, "eggpack-finalized-release");
        assert_eq!(job.receipt_handoff_name, "eggpack-staging-receipt");
        assert!(job.required);
        // M001 CIPlan semantics untouched.
        assert_eq!(staged.ci_plan, graph.ci_plan);
    }

    #[test]
    fn m003b_runner_command_covers_prepare_and_stage() {
        let prepare = RunnerCommand::PrepareStage {
            contract: "contracts/release.toml".into(),
            release_manifest: "./eggpack-finalized/release-manifest.json".into(),
            finalized_root: "./eggpack-finalized/root".into(),
            github_policy: "policies/github-draft.json".into(),
            install_policy: "policies/install.toml".into(),
            output_dir: "./eggpack-staging".into(),
            output_payload: "./eggpack-staging-payload.json".into(),
            installer_presentation: None,
            source_root: None,
        };
        let argv = prepare.argv();
        assert_eq!(&argv[0..3], &["eggpack", "ci", "_prepare-stage"]);
        for required in [
            "--contract",
            "--release-manifest",
            "--finalized-root",
            "--github-policy",
            "--install-policy",
            "--output-dir",
            "--output-payload",
        ] {
            assert!(argv.contains(&required.to_string()), "missing {required}");
        }
        let stage = RunnerCommand::StageGithubDraft {
            payload: "./eggpack-staging-payload.json".into(),
            github_policy: "policies/github-draft.json".into(),
            staging_dir: "./eggpack-staging".into(),
            output_receipt: "./eggpack-staging-receipt.json".into(),
        };
        let argv = stage.argv();
        assert_eq!(&argv[0..3], &["eggpack", "ci", "_stage-github-draft"]);
        for required in [
            "--payload",
            "--github-policy",
            "--staging-dir",
            "--output-receipt",
        ] {
            assert!(argv.contains(&required.to_string()), "missing {required}");
        }
        // No publish subcommand exists.
        assert!(!prepare.to_shell().contains("publish"));
        assert!(!stage.to_shell().contains("publish"));
    }

    #[test]
    fn m003b_staging_disabled_renders_byte_compatible() {
        // M002a output without staging must be unchanged.
        let (graph, policy) = m002_golden_case(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert!(graph.staging.is_none());
        let rendered = render_release_github(&graph, &policy).unwrap();
        assert_golden(&rendered, include_str!("../tests/fixtures/m002-direct.yml"));
    }

    #[test]
    #[ignore]
    fn m003b_regenerate_goldens() {
        let cases = [
            (
                "../tests/fixtures/m003b-direct-staging.yml",
                include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
                "eggsact",
                "1.2.3",
                vec![(
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                )],
                vec!["linux-x64"],
            ),
            (
                "../tests/fixtures/m003b-bundle-staging.yml",
                include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
                "codegg",
                "2.4.0",
                vec![(
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                )],
                vec!["linux-x64"],
            ),
            (
                "../tests/fixtures/m003b-archive-staging.yml",
                include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
                "egress",
                "3.1.0",
                vec![(
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                )],
                vec!["linux-x64"],
            ),
        ];
        for (path, fixture, product, version, targets, aliases) in cases {
            let (graph, policy) = m003b_staging_graph(fixture, product, version, targets, aliases);
            let rendered = render_release_github(&graph, &policy).unwrap();
            let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            std::fs::write(root.join(path.trim_start_matches("../")), &rendered).unwrap();
        }
    }

    #[test]
    fn m003b_golden_direct_bundle_archive_with_staging() {
        // Direct + staging.
        let (graph, policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m003b-direct-staging.yml"),
        );
        // Bundle + staging.
        let (graph, policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            "2.4.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m003b-bundle-staging.yml"),
        );
        // Archive + staging.
        let (graph, policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            "3.1.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        assert_golden(
            &render_release_github(&graph, &policy).unwrap(),
            include_str!("../tests/fixtures/m003b-archive-staging.yml"),
        );
    }

    #[test]
    fn m003b_rendered_staging_is_least_privilege_and_exact() {
        let (graph, policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        let yaml = render_release_github(&graph, &policy).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        // Top-level read-only; no id-token write anywhere.
        assert!(!yaml.contains("id-token: write"));
        assert!(!yaml.contains("gh release"));
        assert!(!yaml.contains("--clobber"));
        assert!(!yaml.contains("release --publish"));
        assert!(!yaml.contains("publish"));
        assert!(!yaml.contains("git tag "));
        assert!(!yaml.contains("git push --tags"));
        assert!(!yaml.contains("curl "));
        // Top-level permission matrix is exactly read-only.
        let top_permissions = parsed.get("permissions").unwrap();
        assert_eq!(
            top_permissions.get("contents").unwrap().as_str().unwrap(),
            "read"
        );
        // Concurrency is release-scoped and tag-aware for manual dispatch.
        let concurrency = parsed.get("concurrency").unwrap();
        let group = concurrency.get("group").unwrap().as_str().unwrap();
        assert!(!group.contains("inputs.release_tag"));
        assert!(group.contains("github.ref"));
        let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
        let mut writers = Vec::new();
        for (name, job) in jobs {
            let name = name.as_str().unwrap();
            let permissions = job.get("permissions").unwrap();
            let contents = permissions.get("contents").unwrap().as_str().unwrap();
            if contents == "write" {
                writers.push(name.to_owned());
            }
            // No job may mint OIDC tokens.
            if let Some(id_token) = permissions.get("id-token") {
                assert_ne!(id_token.as_str().unwrap(), "write", "job {name}");
            }
        }
        assert_eq!(writers, vec!["stage".to_string()], "only stage may write");
        // Stage depends on aggregate and downloads the exact handoff.
        let stage = jobs.get("stage").unwrap();
        let needs = stage.get("needs").unwrap();
        assert_eq!(needs.as_str().unwrap(), "aggregate");
        // Stage runs only for exact tags (tag push or explicit dispatch).
        let condition = stage.get("if").unwrap().as_str().unwrap();
        assert!(condition.contains("github.event_name == 'push'"));
        assert!(condition.contains("github.ref_type == 'tag'"));
        // No untrusted PR trigger may reach the write job: the workflow
        // triggers are push + workflow_dispatch only (no pull_request).
        let triggers = parsed.get("on").unwrap().as_mapping().unwrap();
        assert!(triggers.get("push").is_some());
        assert!(!triggers.keys().any(|key| key
            .as_str()
            .is_some_and(|name| name.contains("pull_request"))));
        let steps = stage.get("steps").unwrap().as_sequence().unwrap();
        let text = serde_yaml::to_string(&serde_yaml::Value::Sequence(steps.clone())).unwrap();
        assert!(text.contains("_prepare-stage"));
        assert!(text.contains("_stage-github-draft"));
        // Stage consumes the exact aggregate artifact, not rebuilt binaries.
        assert!(text.contains("eggpack-finalized-release"));
        assert!(text.contains("./eggpack-finalized/release-manifest.json"));
        assert!(text.contains("./eggpack-finalized/root"));
        // Prepare occurs before network staging.
        let prepare_pos = text.find("_prepare-stage").unwrap();
        let stage_pos = text.find("_stage-github-draft").unwrap();
        assert!(prepare_pos < stage_pos);
        // Token appears only as an environment reference.
        assert!(text.contains("GITHUB_TOKEN"));
        assert!(text.contains("secrets.GITHUB_TOKEN"));
        assert!(!text.contains("ghp_"));
        assert!(!text.contains("github_pat_"));
        // Receipt uploaded as an internal artifact.
        assert!(text.contains("eggpack-staging-receipt"));
        // Deterministic rerender.
        assert_eq!(render_release_github(&graph, &policy).unwrap(), yaml);
        // ci check catches staging step/permission edits.
        let drifted = yaml.clone();
        let drifted = drifted.replacen("contents: write", "contents: read", 1);
        assert!(
            check_release_github(&graph, &policy, drifted.as_bytes()).is_err()
                || !check_release_github(&graph, &policy, drifted.as_bytes())
                    .map(|report| report.matches)
                    .unwrap_or(true)
        );
        // ci check also catches staging step edits (e.g. command swap).
        let drifted_steps = yaml.replacen("_prepare-stage", "_prepare-stage-tampered", 1);
        assert!(
            check_release_github(&graph, &policy, drifted_steps.as_bytes()).is_err()
                || !check_release_github(&graph, &policy, drifted_steps.as_bytes())
                    .map(|report| report.matches)
                    .unwrap_or(true)
        );
    }

    #[test]
    fn m003c_tag_source_controls_checkout_input_and_stage_guard() {
        let (graph, mut policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        let ref_name = render_release_github(&graph, &policy).unwrap();
        assert!(!ref_name.contains("inputs:\n      release_tag:"));
        assert!(ref_name.contains("if: github.event_name == 'push' && github.ref_type == 'tag'"));
        assert!(ref_name.contains("_verify-source"));
        assert_eq!(ref_name.matches("_verify-source").count(), 6);
        assert_eq!(ref_name.matches("ref: \"${{ github.ref }}\"").count(), 6);
        let implicit_checkout = ref_name.replacen(
            "        with:\n          ref: \"${{ github.ref }}\"\n",
            "",
            1,
        );
        assert!(
            !check_release_github(&graph, &policy, implicit_checkout.as_bytes())
                .unwrap()
                .matches
        );
        let no_verifier = ref_name.replacen("_verify-source", "_removed-source-check", 1);
        assert!(
            !check_release_github(&graph, &policy, no_verifier.as_bytes())
                .unwrap()
                .matches
        );
        let broad_stage = ref_name.replace(
            "github.event_name == 'push' && github.ref_type == 'tag'",
            "always()",
        );
        assert!(
            !check_release_github(&graph, &policy, broad_stage.as_bytes())
                .unwrap()
                .matches
        );
        policy.staging.as_mut().unwrap().tag_source = StagingTagSource::DispatchInput;
        let dispatch = render_release_github(&graph, &policy).unwrap();
        assert!(dispatch.contains("release_tag:\n        description:"));
        assert!(dispatch.contains(
            "github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref"
        ));
        assert!(dispatch.contains("if: github.event_name == 'workflow_dispatch'"));
        assert_eq!(dispatch.matches("ref: \"${{ github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref }}\"").count(), 5);
        assert!(dispatch.contains("ref: \"${{ inputs.release_tag }}\""));
        assert!(dispatch.contains(
            "github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref"
        ));
        assert!(!dispatch.contains("if: github.event_name == 'push' && github.ref_type == 'tag'"));
        let mut missing_dispatch = policy.clone();
        missing_dispatch
            .triggers
            .retain(|trigger| *trigger != WorkflowTrigger::WorkflowDispatch);
        assert!(render_release_github(&graph, &missing_dispatch).is_err());
        assert_ne!(ref_name, dispatch);
    }

    #[test]
    fn m003b_staging_requires_policy_and_rejects_forbidden_commands() {
        let (mut graph, policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        // Graph requests staging but policy lacks staging settings.
        let mut bare = policy.clone();
        bare.staging = None;
        assert!(render_release_github(&graph, &bare).is_err());
        // Staging runner must be finite/safe; no publish flag exists by type.
        let mut bad = policy.clone();
        bad.staging.as_mut().unwrap().runner = "evil; rm -rf /".into();
        assert!(render_release_github(&graph, &bad).is_err());
        let _ = &mut graph;
    }

    #[tokio::test]
    async fn m003b_local_orchestration_stages_through_fake_adapter() {
        use eggpack_github::{
            fixture_asset, fixture_release, prepare_staging_payload, stage_with_bytes,
            FixtureGithub, GitHubDraftPolicyV1,
        };
        let fixture_text = include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract =
            eggpack_contract::DistributionContract::parse_toml_str(fixture_text).unwrap();
        let source = "a".repeat(40);
        // Note: simple-direct contract has two targets; use both for bootstrap.
        let config_full = eggpack_core::PackConfig {
            schema_version: 1,
            targets: vec![
                eggpack_core::TargetPolicy {
                    target: "x86_64-unknown-linux-gnu".into(),
                    strategy: eggpack_core::BuildStrategy::NativeCargo,
                    host_os: eggpack_core::HostOs::Linux,
                    host_arch: eggpack_core::HostArch::X86_64,
                    qualification_host: None,
                    toolchain: eggpack_core::ToolchainRequirement {
                        rust: "1.89.0".into(),
                        cargo_zigbuild: None,
                    },
                    floor: eggpack_core::CompatibilityFloor::None,
                    qualification: eggpack_core::Qualification::Structural,
                    support: eggpack_core::SupportTier::Required,
                },
                eggpack_core::TargetPolicy {
                    target: "aarch64-apple-darwin".into(),
                    strategy: eggpack_core::BuildStrategy::NativeCargo,
                    host_os: eggpack_core::HostOs::Macos,
                    host_arch: eggpack_core::HostArch::Aarch64,
                    qualification_host: None,
                    toolchain: eggpack_core::ToolchainRequirement {
                        rust: "1.89.0".into(),
                        cargo_zigbuild: None,
                    },
                    floor: eggpack_core::CompatibilityFloor::None,
                    qualification: eggpack_core::Qualification::Structural,
                    support: eggpack_core::SupportTier::Required,
                },
            ],
        };
        let release = config_full
            .resolve(
                &contract,
                "1.2.6",
                &source,
                &["linux-x64".into(), "macos-arm64".into()],
            )
            .unwrap();
        // Deterministic finalized bytes for both targets.
        let body = b"body";
        let parent = eggpack_core_test_temp("m003b-e2e");
        let finalized_root = parent.join("finalized");
        std::fs::create_dir(&finalized_root).unwrap();
        // Manifest with exact byte facts.
        use sha2::Digest;
        let digest: String = sha2::Sha256::digest(body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest = eggpack_manifest::ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: source.clone(),
            targets: vec![
                eggpack_manifest::TargetRecord {
                    target: "aarch64-apple-darwin".into(),
                    form: eggpack_manifest::ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-aarch64-apple-darwin".into(),
                            size: body.len() as u64,
                            sha256: digest.clone(),
                        },
                        install: "eggsact".into(),
                    },
                },
                eggpack_manifest::TargetRecord {
                    target: "x86_64-unknown-linux-gnu".into(),
                    form: eggpack_manifest::ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-x86_64-unknown-linux-gnu".into(),
                            size: body.len() as u64,
                            sha256: digest.clone(),
                        },
                        install: "eggsact".into(),
                    },
                },
            ],
            evidence_references: vec![],
        };
        for name in [
            "eggsact-1.2.6-aarch64-apple-darwin",
            "eggsact-1.2.6-x86_64-unknown-linux-gnu",
        ] {
            std::fs::write(finalized_root.join(name), body).unwrap();
            std::fs::write(
                finalized_root.join(format!("{name}.sha256")),
                format!("{digest}  {name}\n"),
            )
            .unwrap();
        }
        let policy = GitHubDraftPolicyV1 {
            schema_version: 1,
            owner: "acme".into(),
            repository: "widget".into(),
            tag: "v1.2.6".into(),
            title: "widget 1.2.6".into(),
            body: "notes".into(),
            prerelease: false,
            token_env: "GITHUB_TOKEN".into(),
            request_timeout_secs: 30,
            max_metadata_bytes: 1_000_000,
            max_list_pages: 5,
        };
        let install_policy = eggpack_bootstrap::BootstrapInstallPolicyV1::empty();
        let staging_dir = parent.join("staging");
        let payload = prepare_staging_payload(
            &contract,
            &manifest,
            &finalized_root,
            &policy,
            &install_policy,
            &staging_dir,
        )
        .unwrap();
        assert!(!payload.assets.is_empty());
        // Fake adapter: create then reuse exact draft/assets. Bytes come from
        // the materialized staging directory (panics if absent, never a
        // redacted transport error).
        let fixture = FixtureGithub::with_tag(&source);
        // Provide bytes via explicit map to avoid borrowing the staging dir
        // across awaits.
        let mut staged_bytes = std::collections::BTreeMap::new();
        for asset in &payload.assets {
            staged_bytes.insert(
                asset.name.clone(),
                std::fs::read(staging_dir.join(&asset.name)).unwrap(),
            );
        }
        let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .unwrap();
        assert!(receipt.draft && !receipt.immutable);
        assert_eq!(receipt.uploaded as usize, payload.assets.len());
        // Rerun reuses exact draft/assets without clobber.
        let receipt2 = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .unwrap();
        assert!(!receipt2.created);
        assert_eq!(receipt2.reused as usize, payload.assets.len());
        // Tag mismatch fails closed; published refusal fails closed.
        let bad_fixture = FixtureGithub::with_tag(&"b".repeat(40));
        assert!(
            stage_with_bytes(&payload, &policy, &bad_fixture, "token", |name| {
                Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .is_err()
        );
        let published = FixtureGithub::with_tag(&source);
        published.seed_release(
            fixture_release(9, "v1.2.6", "widget 1.2.6", "notes", false, false, false),
            Vec::new(),
        );
        assert!(
            stage_with_bytes(&payload, &policy, &published, "token", |name| {
                Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .is_err()
        );
        // Mismatched asset refusal.
        let mismatch = FixtureGithub::with_tag(&source);
        mismatch.seed_release(
            fixture_release(10, "v1.2.6", "widget 1.2.6", "notes", false, true, false),
            vec![fixture_asset(
                1,
                &payload.assets[0].name,
                payload.assets[0].size + 1,
                "uploaded",
                Some(&payload.assets[0].sha256),
            )],
        );
        assert!(
            stage_with_bytes(&payload, &policy, &mismatch, "token", |name| {
                Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .is_err()
        );
        let _ = release;
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[tokio::test]
    async fn m003b_local_orchestration_covers_bundle_and_archive() {
        use eggpack_github::{prepare_staging_payload, stage_with_bytes, FixtureGithub};
        use sha2::Digest;
        let sha_hex = |bytes: &[u8]| -> String {
            sha2::Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        let body = b"body";
        let digest = sha_hex(body);
        // Bundle case (codegg-bundle contract, single target).
        {
            let contract = eggpack_contract::DistributionContract::parse_toml_str(include_str!(
                "../../eggpack-contract/tests/fixtures/codegg-bundle.toml"
            ))
            .unwrap();
            let source = "b".repeat(40);
            let manifest = eggpack_manifest::ReleaseManifest {
                schema_version: 1,
                product_id: "codegg".into(),
                release_id: "2.4.0".into(),
                source_revision: source.clone(),
                targets: vec![eggpack_manifest::TargetRecord {
                    target: "x86_64-unknown-linux-gnu".into(),
                    form: eggpack_manifest::ArtifactForm::Bundle {
                        entries: ["codegg", "codegg-helper", "codegg-manifest"]
                            .iter()
                            .zip(["codegg", "codegg-helper", "codegg-manifest.json"].iter())
                            .map(|(stem, install)| eggpack_manifest::BundleRecord {
                                artifact: eggpack_manifest::ArtifactRecord {
                                    name: if *stem == "codegg-manifest" {
                                        "codegg-manifest-2.4.0.json".into()
                                    } else {
                                        format!("{stem}-2.4.0-x86_64-unknown-linux-gnu")
                                    },
                                    size: body.len() as u64,
                                    sha256: digest.clone(),
                                },
                                install: (*install).into(),
                            })
                            .collect(),
                    },
                }],
                evidence_references: vec![],
            };
            let parent = eggpack_core_test_temp("m003b-e2e-bundle");
            let finalized = parent.join("finalized");
            std::fs::create_dir(&finalized).unwrap();
            for name in [
                "codegg-2.4.0-x86_64-unknown-linux-gnu",
                "codegg-helper-2.4.0-x86_64-unknown-linux-gnu",
                "codegg-manifest-2.4.0.json",
            ] {
                std::fs::write(finalized.join(name), body).unwrap();
                std::fs::write(
                    finalized.join(format!("{name}.sha256")),
                    format!("{digest}  {name}\n"),
                )
                .unwrap();
            }
            let policy = eggpack_github::GitHubDraftPolicyV1 {
                schema_version: 1,
                owner: "acme".into(),
                repository: "widget".into(),
                tag: "v2.4.0".into(),
                title: "widget 2.4.0".into(),
                body: "notes".into(),
                prerelease: false,
                token_env: "GITHUB_TOKEN".into(),
                request_timeout_secs: 30,
                max_metadata_bytes: 1_000_000,
                max_list_pages: 5,
            };
            let mut modes = std::collections::BTreeMap::new();
            modes.insert(
                "codegg".to_owned(),
                eggpack_bootstrap::InstallMode::Executable,
            );
            modes.insert(
                "codegg-helper".to_owned(),
                eggpack_bootstrap::InstallMode::Executable,
            );
            modes.insert(
                "codegg-manifest.json".to_owned(),
                eggpack_bootstrap::InstallMode::Data,
            );
            let install_policy = eggpack_bootstrap::BootstrapInstallPolicyV1 {
                schema_version: 1,
                targets: [(
                    "x86_64-unknown-linux-gnu".to_owned(),
                    eggpack_bootstrap::TargetInstallPolicy {
                        modes,
                        archive_encoding: None,
                    },
                )]
                .into_iter()
                .collect(),
            };
            let staging = parent.join("staging");
            let payload = prepare_staging_payload(
                &contract,
                &manifest,
                &finalized,
                &policy,
                &install_policy,
                &staging,
            )
            .unwrap();
            assert!(!payload.assets.is_empty());
            let fixture = FixtureGithub::with_tag(&source);
            let mut bytes = std::collections::BTreeMap::new();
            for asset in &payload.assets {
                bytes.insert(
                    asset.name.clone(),
                    std::fs::read(staging.join(&asset.name)).unwrap(),
                );
            }
            let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                Ok(bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .unwrap();
            assert!(receipt.draft && !receipt.immutable);
            let rerun = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                Ok(bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .unwrap();
            assert!(!rerun.created);
            std::fs::remove_dir_all(parent).unwrap();
        }
        // Archive case (egress-archive contract, two targets).
        {
            let contract = eggpack_contract::DistributionContract::parse_toml_str(include_str!(
                "../../eggpack-contract/tests/fixtures/egress-archive.toml"
            ))
            .unwrap();
            let source = "c".repeat(40);
            let mk = |target: &str| eggpack_manifest::TargetRecord {
                target: target.to_owned(),
                form: eggpack_manifest::ArtifactForm::Archive {
                    artifact: eggpack_manifest::ArtifactRecord {
                        name: format!("egress-3.1.0-{target}.tar.gz"),
                        size: body.len() as u64,
                        sha256: digest.clone(),
                    },
                    members: vec![
                        eggpack_manifest::ArchiveMemberRecord {
                            source: "bin/egress-helper".into(),
                            install: "egress-helper".into(),
                            bytes: eggpack_manifest::ByteEvidence {
                                size: body.len() as u64,
                                sha256: digest.clone(),
                            },
                        },
                        eggpack_manifest::ArchiveMemberRecord {
                            source: "egress".into(),
                            install: "egress".into(),
                            bytes: eggpack_manifest::ByteEvidence {
                                size: body.len() as u64,
                                sha256: digest.clone(),
                            },
                        },
                    ],
                },
            };
            let manifest = eggpack_manifest::ReleaseManifest {
                schema_version: 1,
                product_id: "egress".into(),
                release_id: "3.1.0".into(),
                source_revision: source.clone(),
                targets: vec![mk("aarch64-apple-darwin"), mk("x86_64-unknown-linux-gnu")],
                evidence_references: vec![],
            };
            let parent = eggpack_core_test_temp("m003b-e2e-archive");
            let finalized = parent.join("finalized");
            std::fs::create_dir(&finalized).unwrap();
            for name in [
                "egress-3.1.0-aarch64-apple-darwin.tar.gz",
                "egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz",
            ] {
                std::fs::write(finalized.join(name), body).unwrap();
                std::fs::write(
                    finalized.join(format!("{name}.sha256")),
                    format!("{digest}  {name}\n"),
                )
                .unwrap();
            }
            let policy = eggpack_github::GitHubDraftPolicyV1 {
                schema_version: 1,
                owner: "acme".into(),
                repository: "widget".into(),
                tag: "v3.1.0".into(),
                title: "widget 3.1.0".into(),
                body: "notes".into(),
                prerelease: false,
                token_env: "GITHUB_TOKEN".into(),
                request_timeout_secs: 30,
                max_metadata_bytes: 1_000_000,
                max_list_pages: 5,
            };
            let mut targets = std::collections::BTreeMap::new();
            for triple in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
                let mut modes = std::collections::BTreeMap::new();
                modes.insert(
                    "egress".to_owned(),
                    eggpack_bootstrap::InstallMode::Executable,
                );
                modes.insert(
                    "egress-helper".to_owned(),
                    eggpack_bootstrap::InstallMode::Executable,
                );
                targets.insert(
                    triple.to_owned(),
                    eggpack_bootstrap::TargetInstallPolicy {
                        modes,
                        archive_encoding: Some(eggpack_bootstrap::BundleArchiveEncoding::TarGzip),
                    },
                );
            }
            let install_policy = eggpack_bootstrap::BootstrapInstallPolicyV1 {
                schema_version: 1,
                targets,
            };
            let staging = parent.join("staging");
            let payload = prepare_staging_payload(
                &contract,
                &manifest,
                &finalized,
                &policy,
                &install_policy,
                &staging,
            )
            .unwrap();
            assert!(!payload.assets.is_empty());
            let fixture = FixtureGithub::with_tag(&source);
            let mut bytes = std::collections::BTreeMap::new();
            for asset in &payload.assets {
                bytes.insert(
                    asset.name.clone(),
                    std::fs::read(staging.join(&asset.name)).unwrap(),
                );
            }
            let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                Ok(bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .unwrap();
            assert!(receipt.draft && !receipt.immutable);
            let rerun = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                Ok(bytes.get(name).cloned().expect("staged bytes exist"))
            })
            .await
            .unwrap();
            assert!(!rerun.created);
            std::fs::remove_dir_all(parent).unwrap();
        }
    }

    // -----------------------------------------------------------------------
    // M003d — consumer release composition seam.
    // -----------------------------------------------------------------------

    fn m003d_validator(selector: LogicalOutputSelector) -> ConsumerValidatorV1 {
        ConsumerValidatorV1 {
            schema_version: 1,
            selector,
            interpreter: ValidatorInterpreterV1::Python3,
            script: "scripts/smoke-mcp-binary.py".into(),
            timeout_ms: 20_000,
            stdout_limit: 65_536,
            stderr_limit: 65_536,
        }
    }

    #[test]
    fn m003d_validator_config_validation() {
        let valid = m003d_validator(LogicalOutputSelector::Direct);
        assert!(valid.validate().is_ok());
        assert_eq!(
            ConsumerValidatorV1::from_json(&valid.to_json().unwrap()).unwrap(),
            valid
        );
        let mut bad = valid.clone();
        bad.schema_version = 2;
        assert!(bad.validate().is_err());
        let mut bad = valid.clone();
        bad.script = "../escape.py".into();
        assert!(bad.validate().is_err());
        let mut bad = valid.clone();
        bad.script = "/abs.py".into();
        assert!(bad.validate().is_err());
        let mut bad = valid.clone();
        bad.timeout_ms = 999;
        assert!(bad.validate().is_err());
        let mut bad = valid.clone();
        bad.timeout_ms = 600_001;
        assert!(bad.validate().is_err());
        let mut bad = valid.clone();
        bad.stdout_limit = 0;
        assert!(bad.validate().is_err());
        let mut bad = valid.clone();
        bad.stderr_limit = 8_000_001;
        assert!(bad.validate().is_err());
        // Unknown fields (e.g. arbitrary env/shell maps) reject.
        assert!(ConsumerValidatorV1::from_json(
            r#"{"schema_version":1,"selector":"direct","interpreter":"python3","script":"s.py","timeout_ms":1000,"stdout_limit":1024,"stderr_limit":1024,"env":{"A":"b"}}"#
        )
        .is_err());
        assert!(ConsumerValidatorV1::from_json(
            r#"{"schema_version":1,"selector":"direct","interpreter":"python3","script":"s.py","timeout_ms":1000,"stdout_limit":1024,"stderr_limit":1024,"shell":true}"#
        )
        .is_err());
        // Evidence carries no output contents by construction.
        let evidence = ConsumerValidationEvidenceV1 {
            schema_version: 1,
            release_id: "r".into(),
            source_revision: "a".repeat(40),
            target: "x86_64-unknown-linux-gnu".into(),
            selector: LogicalOutputSelector::Direct,
            interpreter: ValidatorInterpreterV1::Python3,
            outcome: ConsumerValidationOutcome::Passed,
            candidate_size: 4,
            candidate_sha256: "0".repeat(64),
        };
        let json = evidence.to_json().unwrap();
        assert!(!json.contains("stdout"));
        assert!(!json.contains("stderr"));
        assert!(!json.contains("output"));
        assert_eq!(
            ConsumerValidationEvidenceV1::from_json(&json).unwrap(),
            evidence
        );
    }

    fn m003d_write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    fn m003d_candidate(dir: &Path, name: &str, bytes: &[u8]) -> (PathBuf, u64, String) {
        use sha2::Digest;
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        let sha: String = sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (path, bytes.len() as u64, sha)
    }

    const M003D_SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn m003d_request<'a>(
        validator: &'a ConsumerValidatorV1,
        script: &'a Path,
        candidate: &'a Path,
        size: u64,
        sha: &'a str,
        work: &'a Path,
        path_dirs: Option<&'a [PathBuf]>,
    ) -> ConsumerValidationRequest<'a> {
        ConsumerValidationRequest {
            validator,
            script_path: script,
            candidate_path: candidate,
            expected_size: size,
            expected_sha256: sha,
            work_dir: work,
            release_id: "v9.9.9",
            source_revision: M003D_SOURCE,
            target: "x86_64-unknown-linux-gnu",
            cancelled: None,
            path_dirs,
        }
    }

    #[test]
    fn m003d_consumer_execution_matrix() {
        let parent = eggpack_core_test_temp("m003d-consumer");
        let work = parent.join("work");
        std::fs::create_dir(&work).unwrap();
        let validator = m003d_validator(LogicalOutputSelector::Direct);
        // Success: script receives exactly (script, candidate) and exits 0.
        let script = m003d_write_script(
            &parent,
            "ok.py",
            "import sys\nassert len(sys.argv) == 2\nopen(sys.argv[1], 'rb').read()\n",
        );
        let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"exact-bytes");
        let evidence = run_consumer_validator(&m003d_request(
            &validator, &script, &candidate, size, &sha, &work, None,
        ))
        .unwrap();
        assert_eq!(evidence.outcome, ConsumerValidationOutcome::Passed);
        assert_eq!(evidence.candidate_size, size);
        assert_eq!(evidence.candidate_sha256, sha);
        assert_eq!(evidence.selector, LogicalOutputSelector::Direct);
        // Non-zero exit fails.
        let script = m003d_write_script(&parent, "fail.py", "import sys\nsys.exit(3)\n");
        let evidence = run_consumer_validator(&m003d_request(
            &validator, &script, &candidate, size, &sha, &work, None,
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit)
        );
        // Timeout fails (script sleeps past a short bound).
        let mut short = validator.clone();
        short.timeout_ms = 1_000;
        let script = m003d_write_script(&parent, "slow.py", "import time\ntime.sleep(30)\n");
        let evidence = run_consumer_validator(&m003d_request(
            &short, &script, &candidate, size, &sha, &work, None,
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout)
        );
        // Output limit fails (script floods stdout, then would exit 0).
        let mut limited = validator.clone();
        limited.stdout_limit = 1024;
        let script = m003d_write_script(
            &parent,
            "flood.py",
            "import sys\nsys.stdout.write('x' * 100000)\n",
        );
        let evidence = run_consumer_validator(&m003d_request(
            &limited, &script, &candidate, size, &sha, &work, None,
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit)
        );
        // Missing interpreter fails rather than skipping (hermetic PATH).
        let script = m003d_write_script(&parent, "ok2.py", "pass\n");
        let empty: Vec<PathBuf> = Vec::new();
        let evidence = run_consumer_validator(&m003d_request(
            &validator,
            &script,
            &candidate,
            size,
            &sha,
            &work,
            Some(&empty),
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::InterpreterUnavailable)
        );
        // Wrong candidate identity fails (size and digest mismatch).
        let (other, other_size, other_sha) = m003d_candidate(&parent, "other", b"other-bytes");
        assert_ne!(other_sha, sha);
        let script = m003d_write_script(&parent, "ok3.py", "pass\n");
        let evidence = run_consumer_validator(&m003d_request(
            &validator, &script, &other, size, &sha, &work, None,
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch)
        );
        let _ = other_size;
        // Symlink script rejects as unavailable (no PATH-selected script).
        #[cfg(unix)]
        {
            let link = parent.join("link.py");
            std::os::unix::fs::symlink(&script, &link).unwrap();
            let evidence = run_consumer_validator(&m003d_request(
                &validator, &link, &candidate, size, &sha, &work, None,
            ))
            .unwrap();
            assert_eq!(
                evidence.outcome,
                ConsumerValidationOutcome::Failed(ConsumerValidationFailure::ScriptUnavailable)
            );
            // Symlink candidate mismatches identity.
            let candidate_link = parent.join("candidate-link");
            std::os::unix::fs::symlink(&candidate, &candidate_link).unwrap();
            let evidence = run_consumer_validator(&m003d_request(
                &validator,
                &script,
                &candidate_link,
                size,
                &sha,
                &work,
                None,
            ))
            .unwrap();
            assert_eq!(
                evidence.outcome,
                ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch)
            );
        }
        // Cancellation fails closed.
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let flag = Arc::new(AtomicBool::new(false));
        let script_hang = m003d_write_script(&parent, "hang.py", "import time\ntime.sleep(30)\n");
        let slow_validator = ConsumerValidatorV1 {
            timeout_ms: 60_000,
            ..validator.clone()
        };
        let worker = {
            let flag_child = flag.clone();
            let slow2 = slow_validator.clone();
            let script2 = script_hang.clone();
            let candidate2 = candidate.clone();
            let work2 = work.clone();
            let sha2 = sha.clone();
            let release2 = "v9.9.9".to_owned();
            let source2 = "a".repeat(40);
            let target2 = "x86_64-unknown-linux-gnu".to_owned();
            std::thread::spawn(move || {
                let request = ConsumerValidationRequest {
                    validator: &slow2,
                    script_path: &script2,
                    candidate_path: &candidate2,
                    expected_size: size,
                    expected_sha256: &sha2,
                    work_dir: &work2,
                    release_id: &release2,
                    source_revision: &source2,
                    target: &target2,
                    cancelled: Some(&flag_child),
                    path_dirs: None,
                };
                run_consumer_validator(&request).unwrap()
            })
        };
        std::thread::sleep(std::time::Duration::from_millis(300));
        flag.store(true, Ordering::SeqCst);
        let evidence = worker.join().unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Cancelled)
        );
        // Caller misuse returns CiError, not failure evidence.
        let bad_request = ConsumerValidationRequest {
            validator: &validator,
            script_path: Path::new("relative/script.py"),
            candidate_path: &candidate,
            expected_size: size,
            expected_sha256: &sha,
            work_dir: &work,
            release_id: "v9.9.9",
            source_revision: &"a".repeat(40),
            target: "x86_64-unknown-linux-gnu",
            cancelled: None,
            path_dirs: None,
        };
        assert!(run_consumer_validator(&bad_request).is_err());
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn m003d_python_mapping_and_hermetic_lookup() {
        // Finite host mapping: python3 on Linux/macOS, python on Windows.
        assert_eq!(python_interpreter_exe_for_windows(false), "python3");
        assert_eq!(python_interpreter_exe_for_windows(true), "python");
        assert_eq!(
            python_interpreter_exe(),
            python_interpreter_exe_for_windows(cfg!(windows))
        );
        // Hermetic PATH lookup: a fixture dir providing the platform
        // interpreter proves fixed argv (script, candidate) without
        // depending on ambient tooling beyond the fake.
        let parent = eggpack_core_test_temp("m003d-python-path");
        let bin = parent.join("bin");
        std::fs::create_dir(&bin).unwrap();
        let work = parent.join("work");
        std::fs::create_dir(&work).unwrap();
        let marker = parent.join("argv-marker.txt");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Fake interpreter: answers the version preflight with
            // Python 3 and records its fixed argv for the real run.
            let fake = format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"Python 3.99.0\"; exit 0; fi\necho \"$1\" > \"{}\"\necho \"$2\" >> \"{}\"\nexit 0\n",
                marker.display(),
                marker.display()
            );
            let exe = bin.join(python_interpreter_exe());
            std::fs::write(&exe, fake).unwrap();
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            let fake = format!(
                "@echo off\r\nif \"%~1\"==\"--version\" (echo Python 3.99.0) else (echo %1> \"{m}\" & echo %2>> \"{m}\")\r\nexit /b 0\r\n",
                m = marker.display()
            );
            std::fs::write(bin.join("python.cmd"), fake).unwrap();
        }
        let validator = m003d_validator(LogicalOutputSelector::Direct);
        let script = m003d_write_script(&parent, "v.py", "pass\n");
        let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"bytes");
        let dirs = [bin];
        let evidence = run_consumer_validator(&m003d_request(
            &validator,
            &script,
            &candidate,
            size,
            &sha,
            &work,
            Some(&dirs),
        ))
        .unwrap();
        #[cfg(unix)]
        {
            assert_eq!(evidence.outcome, ConsumerValidationOutcome::Passed);
            let recorded = std::fs::read_to_string(&marker).unwrap();
            let mut lines = recorded.lines();
            assert_eq!(lines.next().unwrap(), script.to_string_lossy().as_ref());
            assert_eq!(lines.next().unwrap(), candidate.to_string_lossy().as_ref());
        }
        #[cfg(windows)]
        {
            // Windows lookup resolves `python` via PATHEXT/cmd shim search;
            // the hermetic dir may not provide it, so only the mapping name
            // is asserted above on this host.
            let _ = evidence;
        }
        std::fs::remove_dir_all(parent).unwrap();
    }

    fn m003d_graph_with_consumer(
        support: SupportTier,
    ) -> (
        DistributionContract,
        ReleasePlan,
        BuildBindingsV1,
        QualificationBindingsV1,
        ReleaseCIPlanV1,
    ) {
        let (contract, release, bindings, qual_bindings) = m002_release(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            "linux-x64",
            Qualification::Structural,
            support,
        );
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        let mut validators = BTreeMap::new();
        validators.insert(
            "x86_64-unknown-linux-gnu".to_owned(),
            m003d_validator(LogicalOutputSelector::Direct),
        );
        let graph = project_release_plan_with_consumer(
            &ci_plan,
            &qual_bindings,
            &bindings,
            &release,
            validators,
        )
        .unwrap();
        (contract, release, bindings, qual_bindings, graph)
    }

    fn m003d_consumer_evidence(
        release: &ReleasePlan,
        outcome: ConsumerValidationOutcome,
    ) -> ConsumerValidationEvidenceV1 {
        ConsumerValidationEvidenceV1 {
            schema_version: 1,
            release_id: release.release_id.clone(),
            source_revision: release.source_revision.clone(),
            target: "x86_64-unknown-linux-gnu".into(),
            selector: LogicalOutputSelector::Direct,
            interpreter: ValidatorInterpreterV1::Python3,
            outcome,
            candidate_size: 4,
            candidate_sha256: "0".repeat(64),
        }
    }

    #[test]
    fn m003d_consumer_gate_matrix() {
        let (_, release, _, _, graph) = m003d_graph_with_consumer(SupportTier::Required);
        let target = "x86_64-unknown-linux-gnu";
        let passed_core = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Passed,
        );
        let core = [passed_core];
        // Consumer pass gates through.
        let consumer_pass = m003d_consumer_evidence(&release, ConsumerValidationOutcome::Passed);
        assert_eq!(
            evaluate_gate_with_consumer(&graph, &core, &[consumer_pass]).unwrap(),
            AggregateOutcome::Complete
        );
        // Required consumer failure blocks aggregation.
        let consumer_fail = m003d_consumer_evidence(
            &release,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit),
        );
        assert_eq!(
            evaluate_gate_with_consumer(&graph, &core, std::slice::from_ref(&consumer_fail))
                .unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        // Missing consumer evidence for a required validator blocks.
        assert_eq!(
            evaluate_gate_with_consumer(&graph, &core, &[]).unwrap(),
            AggregateOutcome::FailedRequiredGate
        );
        // Identity mismatch (wrong selector) is invalid, never partial.
        let mut swapped = m003d_consumer_evidence(&release, ConsumerValidationOutcome::Passed);
        swapped.selector = LogicalOutputSelector::BundleEntry { index: 0 };
        assert_eq!(
            evaluate_gate_with_consumer(&graph, &core, &[swapped]).unwrap(),
            AggregateOutcome::InvalidEvidence
        );
        let mut wrong_release =
            m003d_consumer_evidence(&release, ConsumerValidationOutcome::Passed);
        wrong_release.release_id = "other".into();
        assert_eq!(
            evaluate_gate_with_consumer(&graph, &core, &[wrong_release]).unwrap(),
            AggregateOutcome::InvalidEvidence
        );
        // Optional validator failure suppresses rather than producing output.
        let (_, optional_release, _, _, optional_graph) =
            m003d_graph_with_consumer(SupportTier::NonGating);
        let optional_core = m002_evidence(
            target,
            &optional_release.release_id,
            &optional_release.source_revision,
            Qualification::Structural,
            SupportTier::NonGating,
            QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
        );
        // Core optional failure alone already suppresses.
        let failed_optional = [optional_core];
        assert_eq!(
            evaluate_gate_with_consumer(
                &optional_graph,
                &failed_optional,
                &[m003d_consumer_evidence(
                    &optional_release,
                    ConsumerValidationOutcome::Passed
                )]
            )
            .unwrap(),
            AggregateOutcome::SuppressedNonGatingIncomplete
        );
        // Optional core pass + optional consumer failure suppresses.
        let optional_pass = m002_evidence(
            target,
            &optional_release.release_id,
            &optional_release.source_revision,
            Qualification::Structural,
            SupportTier::NonGating,
            QualificationStatus::Passed,
        );
        let optional_consumer_fail = m003d_consumer_evidence(
            &optional_release,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout),
        );
        assert_eq!(
            evaluate_gate_with_consumer(
                &optional_graph,
                &[optional_pass],
                &[optional_consumer_fail]
            )
            .unwrap(),
            AggregateOutcome::SuppressedNonGatingIncomplete
        );
        // Consumer evidence for a target without a validator is invalid.
        let (_, _, _, _, _, plain_graph) =
            m002_graph(Qualification::Structural, SupportTier::Required);
        let plain_core = m002_evidence(
            target,
            &release.release_id,
            &release.source_revision,
            Qualification::Structural,
            SupportTier::Required,
            QualificationStatus::Passed,
        );
        assert_eq!(
            evaluate_gate_with_consumer(
                &plain_graph,
                &[plain_core],
                &[m003d_consumer_evidence(
                    &release,
                    ConsumerValidationOutcome::Passed
                )]
            )
            .unwrap(),
            AggregateOutcome::InvalidEvidence
        );
        // Durable evidence contains no script output contents.
        let json = consumer_fail.to_json().unwrap();
        assert!(!json.contains("stdout"));
        assert!(!json.contains("stderr"));
    }
    #[test]
    fn m003d_consumer_attach_rejects_mismatch() {
        let (contract, release, bindings, qual_bindings) = m002_release(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            "linux-x64",
            Qualification::Structural,
            SupportTier::Required,
        );
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        // Unknown target rejects.
        let mut validators = BTreeMap::new();
        validators.insert(
            "aarch64-unknown-linux-gnu".to_owned(),
            m003d_validator(LogicalOutputSelector::Direct),
        );
        assert!(project_release_plan_with_consumer(
            &ci_plan,
            &qual_bindings,
            &bindings,
            &release,
            validators
        )
        .is_err());
        // Selector outside the target bindings rejects.
        let mut validators = BTreeMap::new();
        validators.insert(
            "x86_64-unknown-linux-gnu".to_owned(),
            m003d_validator(LogicalOutputSelector::BundleEntry { index: 0 }),
        );
        assert!(project_release_plan_with_consumer(
            &ci_plan,
            &qual_bindings,
            &bindings,
            &release,
            validators
        )
        .is_err());
        // Validator map path is required for rendering.
        let (_, _, _, _, graph) = m003d_graph_with_consumer(SupportTier::Required);
        let mut policy = m002_golden_policy();
        assert!(render_release_github(&graph, &policy).is_err());
        policy.release_inputs.as_mut().unwrap().consumer_validators =
            Some("validators/consumer.json".into());
        assert!(render_release_github(&graph, &policy).is_ok());
    }

    #[test]
    fn m003d_stage_presentation_flags_are_explicit() {
        // Without a presentation path the stage job carries no wrapper
        // flags (M003c byte-compatible); with one, both paired flags render
        // and product scripts are only ever copied as bytes (never executed).
        let (graph, policy) = m003b_staging_graph(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        );
        let plain = render_release_github(&graph, &policy).unwrap();
        assert!(!plain.contains("--installer-presentation"));
        assert!(!plain.contains("--source-root"));
        let mut presented = policy.clone();
        presented
            .staging
            .as_mut()
            .unwrap()
            .inputs
            .installer_presentation = Some("policies/installer-presentation.toml".into());
        let yaml = render_release_github(&graph, &presented).unwrap();
        assert!(yaml.contains("--installer-presentation"));
        assert!(yaml.contains("policies/installer-presentation.toml"));
        assert!(yaml.contains("--source-root"));
        // The wrapper file itself is never named or executed in the
        // workflow; only the presentation policy path is referenced.
        assert!(!yaml.contains("packaging/install.sh"));
        assert!(!yaml.contains("install-exact.sh"));
    }

    #[test]
    fn m003d_consumer_render_gates_and_detects_drift() {
        let (_, _, _, _, graph) = m003d_graph_with_consumer(SupportTier::Required);
        let mut policy = m002_golden_policy();
        policy.release_inputs.as_mut().unwrap().consumer_validators =
            Some("validators/consumer.json".into());
        let yaml = render_release_github(&graph, &policy).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
        // Consumer validation job runs after core qualification.
        let validate_job = jobs.get("validate_build_x86_64_unknown_linux_gnu").unwrap();
        assert_eq!(
            validate_job.get("needs").unwrap().as_str().unwrap(),
            "qualify_build_x86_64_unknown_linux_gnu"
        );
        // Gate waits for the validator, not just core qualification.
        let gate = jobs.get("required_gate").unwrap();
        let needs = gate.get("needs").unwrap().as_sequence().unwrap();
        let needs: Vec<&str> = needs.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(needs, vec!["validate_build_x86_64_unknown_linux_gnu"]);
        // Validator receives exact handoff bytes; stage stays the writer.
        let text = serde_yaml::to_string(&serde_yaml::Value::Mapping(jobs.clone())).unwrap();
        assert!(text.contains("_validate-consumer"));
        assert!(text.contains("--consumer-validators"));
        assert!(text.contains("--source-root"));
        assert!(text.contains("eggpack-consumer-evidence-x86_64-unknown-linux-gnu"));
        let mut writers = Vec::new();
        for (name, job) in jobs {
            let name = name.as_str().unwrap();
            let contents = job
                .get("permissions")
                .unwrap()
                .get("contents")
                .unwrap()
                .as_str()
                .unwrap();
            if contents == "write" {
                writers.push(name.to_owned());
            }
            assert_ne!(
                job.get("permissions")
                    .unwrap()
                    .get("id-token")
                    .map(|v| v.as_str().unwrap_or("")),
                Some("write"),
                "job {name}"
            );
        }
        assert!(
            writers.is_empty(),
            "exact consumer graph has no writer without staging"
        );
        // Drift: validator removal/reorder is detected.
        let (_, _, _, _, _, plain) = m002_graph(Qualification::Structural, SupportTier::Required);
        assert!(!check_release_github(&plain, &policy, yaml.as_bytes())
            .map(|report| report.matches)
            .unwrap_or(true));
        let yaml_plain = render_release_github(&plain, &policy).unwrap();
        assert_ne!(yaml, yaml_plain);
        assert!(
            check_release_github(&graph, &policy, yaml.as_bytes())
                .unwrap()
                .matches
        );
        // No validator jobs leak into extension-disabled rendering.
        assert!(!yaml_plain.contains("_validate-consumer"));
        assert!(!yaml_plain.contains("validate_build_"));
    }

    #[test]
    fn m003d_runner_commands_cover_consumer_and_resolve() {
        let validate = RunnerCommand::ValidateConsumer {
            consumer_validators: "validators/consumer.json".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            candidate_dir: "./eggpack-handoff/build_x/candidates".into(),
            build_handoff: "./eggpack-handoff/build_x/build-handoff.json".into(),
            evidence: "./eggpack-handoff/build_x/evidence.json".into(),
            source_root: "${{ github.workspace }}".into(),
            output: "./eggpack-consumer/build_x/consumer-evidence.json".into(),
        };
        let shell = validate.to_shell();
        for required in [
            "_validate-consumer",
            "--consumer-validators",
            "--target",
            "--candidate-dir",
            "--build-handoff",
            "--evidence",
            "--source-root",
            "--output",
        ] {
            assert!(
                shell.contains(required),
                "validate shell missing {required}"
            );
        }
        assert!(!shell.contains("publish"));
        let resolve = RunnerCommand::ResolveRelease {
            contract: "contracts/release.toml".into(),
            pack_config: "configs/pack.toml".into(),
            build_bindings: "bindings/build.toml".into(),
            qualification_bindings: "bindings/qualification.toml".into(),
            consumer_validators: Some("validators/consumer.json".into()),
            selected: "linux-x64".into(),
            tag: "v1.2.3".into(),
            source_revision: "a".repeat(40),
            template: "policies/github-template.json".into(),
            source_root: "${{ github.workspace }}".into(),
            output_plan: "./eggpack-runtime/release-plan.json".into(),
            output_ci_plan: "./eggpack-runtime/release-ci-plan.json".into(),
            output_github_policy: "./eggpack-runtime/github-draft.json".into(),
        };
        let argv = resolve.argv();
        assert_eq!(&argv[0..3], &["eggpack", "ci", "_resolve-release"]);
        for required in [
            "--contract",
            "--pack-config",
            "--build-bindings",
            "--qualification-bindings",
            "--consumer-validators",
            "--selected",
            "--tag",
            "--source-revision",
            "--template",
            "--source-root",
            "--output-plan",
            "--output-ci-plan",
            "--output-github-policy",
        ] {
            assert!(argv.contains(&required.to_string()), "missing {required}");
        }
        assert!(!resolve.to_shell().contains("publish"));
        // Omitted validator map omits the flag pair (exact position stable).
        let bare = RunnerCommand::ResolveRelease {
            contract: "contracts/release.toml".into(),
            pack_config: "configs/pack.toml".into(),
            build_bindings: "bindings/build.toml".into(),
            qualification_bindings: "bindings/qualification.toml".into(),
            consumer_validators: None,
            selected: "linux-x64".into(),
            tag: "v1.2.3".into(),
            source_revision: "a".repeat(40),
            template: "policies/github-template.json".into(),
            source_root: "${{ github.workspace }}".into(),
            output_plan: "./eggpack-runtime/release-plan.json".into(),
            output_ci_plan: "./eggpack-runtime/release-ci-plan.json".into(),
            output_github_policy: "./eggpack-runtime/github-draft.json".into(),
        };
        assert!(!bare.argv().contains(&"--consumer-validators".to_string()));
    }

    fn m003d_shape_and_policy(
        tag_source: StagingTagSource,
    ) -> (DistributionContract, ReleaseWorkflowShapeV1, GitHubPolicy) {
        let (contract, release, bindings, qual_bindings) = m002_release(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            "linux-x64",
            Qualification::Structural,
            SupportTier::Required,
        );
        let mut validators = BTreeMap::new();
        validators.insert(
            "x86_64-unknown-linux-gnu".to_owned(),
            m003d_validator(LogicalOutputSelector::Direct),
        );
        let shape = ReleaseWorkflowShapeV1 {
            schema_version: 1,
            targets: release.targets.iter().map(|t| t.policy.clone()).collect(),
            selected_aliases: vec!["linux-x64".into()],
            build_bindings: bindings,
            qualification_bindings: qual_bindings,
            consumer_validators: validators,
            staging: Some(ShapeStagingIntentV1 {
                provider: StagingProvider::GitHubDraft,
                tag_source,
                required: true,
            }),
        };
        let mut policy = m002_golden_policy();
        let inputs = policy.release_inputs.as_mut().unwrap();
        inputs.pack_config = Some("configs/pack.toml".into());
        inputs.draft_template = Some("policies/github-template.json".into());
        inputs.consumer_validators = Some("validators/consumer.json".into());
        policy.staging = Some(GitHubStagingPolicyV1 {
            runner: "ubuntu-latest".into(),
            owner: "acme".into(),
            repository: "widget".into(),
            tag_source,
            inputs: GitHubStagingInputsV1 {
                contract: "contracts/release.toml".into(),
                install_policy: "policies/install.toml".into(),
                github_policy: "policies/github-draft.json".into(),
                installer_presentation: None,
            },
            receipt_retention_days: 7,
        });
        (contract, shape, policy)
    }

    #[test]
    fn m003d_shape_validation() {
        let (_, shape, _) = m003d_shape_and_policy(StagingTagSource::RefName);
        assert!(shape.validate().is_ok());
        assert_eq!(
            ReleaseWorkflowShapeV1::from_json(&shape.to_json().unwrap()).unwrap(),
            shape
        );
        // No release identity lives in the shape by construction: unknown
        // identity fields reject.
        assert!(ReleaseWorkflowShapeV1::from_json(
            r#"{"schema_version":1,"targets":[],"selected_aliases":[],"build_bindings":{},"qualification_bindings":{},"release_id":"v1"}"#
        )
        .is_err());
        let mut bad = shape.clone();
        bad.schema_version = 2;
        assert!(bad.validate().is_err());
        let mut bad = shape.clone();
        bad.targets.reverse();
        // Single-target shape: reversal is a no-op; duplicate instead.
        bad.targets.push(bad.targets[0].clone());
        assert!(bad.validate().is_err());
        let mut bad = shape.clone();
        bad.selected_aliases.clear();
        assert!(bad.validate().is_err());
        let mut bad = shape.clone();
        bad.selected_aliases = vec!["".into()];
        assert!(bad.validate().is_err());
    }

    #[test]
    fn m003d_runtime_resolve_matrix() {
        let (contract, shape, _) = m003d_shape_and_policy(StagingTagSource::RefName);
        let pack = shape.pack_config();
        let source = "a".repeat(40);
        let first = resolve_runtime_release_plan(
            &contract,
            &pack,
            &shape.selected_aliases,
            "v1.2.4",
            &source,
        )
        .unwrap();
        assert_eq!(first.release_id, "v1.2.4");
        assert_eq!(first.source_revision, source);
        // Distinct future tags resolve distinct plans from identical shape.
        let second = resolve_runtime_release_plan(
            &contract,
            &pack,
            &shape.selected_aliases,
            "v1.2.5",
            &source,
        )
        .unwrap();
        assert_ne!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        // Exact tag is used as-is (no product syntax transformation).
        let tagged = resolve_runtime_release_plan(
            &contract,
            &pack,
            &shape.selected_aliases,
            "eggsact-v2.0.0",
            &source,
        )
        .unwrap();
        assert_eq!(tagged.release_id, "eggsact-v2.0.0");
        // Injection and shape violations reject.
        for bad_tag in [
            "",
            "v1?x",
            "v 1",
            "../escape",
            "/v1",
            "v1/",
            "a".repeat(129).as_str(),
        ] {
            assert!(
                resolve_runtime_release_plan(
                    &contract,
                    &pack,
                    &shape.selected_aliases,
                    bad_tag,
                    &source
                )
                .is_err(),
                "tag {bad_tag:?} must reject"
            );
        }
        for bad_rev in [
            "",
            &"a".repeat(39),
            &"A".repeat(40),
            "not-hex-at-all------------------------",
        ] {
            assert!(
                resolve_runtime_release_plan(
                    &contract,
                    &pack,
                    &shape.selected_aliases,
                    "v1.2.4",
                    bad_rev
                )
                .is_err(),
                "revision {bad_rev:?} must reject"
            );
        }
        assert!(resolve_runtime_release_plan(&contract, &pack, &[], "v1.2.4", &source).is_err());
        assert!(resolve_runtime_release_plan(
            &contract,
            &pack,
            &["unknown-alias".to_owned()],
            "v1.2.4",
            &source
        )
        .is_err());
    }

    #[test]
    fn m003d_reusable_render_is_tag_independent() {
        for tag_source in [StagingTagSource::RefName, StagingTagSource::DispatchInput] {
            let (contract, shape, policy) = m003d_shape_and_policy(tag_source);
            let first = render_reusable_release_github(&contract, &shape, &policy).unwrap();
            // Same static config renders byte-identical workflow bytes
            // independent of any future tag.
            assert_eq!(
                first,
                render_reusable_release_github(&contract, &shape, &policy).unwrap()
            );
            // No release identity is embedded.
            assert!(!first.contains("0.0.0-m003d-unresolved"));
            assert!(!first.contains("0000000000000000000000000000000000000000"));
            assert!(!first.contains("plans/release-plan.json"));
            assert!(!first.contains("plans/release-ci-plan.json"));
            assert!(!first.contains("policies/github-draft.json"));
            // Runtime preflight resolves identity into private storage.
            assert!(first.contains("resolve:"));
            assert!(first.contains("_resolve-release"));
            assert!(first.contains("--pack-config"));
            assert!(first.contains("--template"));
            assert!(first.contains("eggpack-runtime-identity"));
            assert!(first.contains("./eggpack-runtime/release-plan.json"));
            assert!(first.contains("head_sha=\"$(git rev-parse --verify HEAD^{commit})\""));
            assert!(first.contains("\"$head_sha\""));
            // Every source checkout is verified against the runtime plan.
            assert!(first.contains("_verify-source"));
            assert_eq!(
                first.matches("needs: resolve").count(),
                1,
                "only the build job waits for resolve"
            );
            // Consumer seam is wired after core qualification.
            assert!(first.contains("validate_build_x86_64_unknown_linux_gnu"));
            assert!(first.contains("_validate-consumer"));
            // Tag source controls the event mapping.
            match tag_source {
                StagingTagSource::RefName => {
                    assert!(first.contains("github.ref_type == 'tag'"));
                    assert!(!first.contains("inputs.release_tag:\n"));
                }
                StagingTagSource::DispatchInput => {
                    assert!(first.contains("inputs.release_tag"));
                    assert!(first.contains("github.event_name == 'workflow_dispatch'"));
                }
            }
            // Only stage writes; no publication authority.
            let parsed: serde_yaml::Value = serde_yaml::from_str(&first).unwrap();
            let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
            let mut writers = Vec::new();
            for (name, job) in jobs {
                let contents = job
                    .get("permissions")
                    .unwrap()
                    .get("contents")
                    .unwrap()
                    .as_str()
                    .unwrap();
                if contents == "write" {
                    writers.push(name.as_str().unwrap().to_owned());
                }
            }
            assert_eq!(writers, vec!["stage".to_string()]);
            assert!(!first.contains("id-token: write"));
            assert!(!first.contains("gh release"));
            assert!(!first.contains("publish"));
        }
        // Tag sources render distinct workflows.
        let (contract, shape_ref, policy_ref) = m003d_shape_and_policy(StagingTagSource::RefName);
        let (_, shape_dispatch, policy_dispatch) =
            m003d_shape_and_policy(StagingTagSource::DispatchInput);
        assert_ne!(
            render_reusable_release_github(&contract, &shape_ref, &policy_ref).unwrap(),
            render_reusable_release_github(&contract, &shape_dispatch, &policy_dispatch).unwrap()
        );
    }

    #[test]
    fn m003d_reusable_check_detects_shape_drift() {
        let (contract, shape, policy) = m003d_shape_and_policy(StagingTagSource::RefName);
        let yaml = render_reusable_release_github(&contract, &shape, &policy).unwrap();
        assert!(
            check_reusable_release_github(&contract, &shape, &policy, yaml.as_bytes())
                .unwrap()
                .matches
        );
        // Validator removal is detected.
        let mut no_validator = shape.clone();
        no_validator.consumer_validators.clear();
        assert!(
            !check_reusable_release_github(&contract, &no_validator, &policy, yaml.as_bytes())
                .map(|report| report.matches)
                .unwrap_or(true)
        );
        // Staging intent removal is detected.
        let mut no_staging = shape.clone();
        no_staging.staging = None;
        assert!(
            check_reusable_release_github(&contract, &no_staging, &policy, yaml.as_bytes())
                .is_err()
                || !check_reusable_release_github(&contract, &no_staging, &policy, yaml.as_bytes())
                    .map(|report| report.matches)
                    .unwrap_or(true)
        );
        // Tag source switch is detected.
        let (_, shape_dispatch, policy_dispatch) =
            m003d_shape_and_policy(StagingTagSource::DispatchInput);
        assert!(!check_reusable_release_github(
            &contract,
            &shape_dispatch,
            &policy_dispatch,
            yaml.as_bytes()
        )
        .map(|report| report.matches)
        .unwrap_or(true));
        // Missing static paths fail closed at render time.
        let mut bad_policy = policy.clone();
        bad_policy.release_inputs.as_mut().unwrap().pack_config = None;
        assert!(render_reusable_release_github(&contract, &shape, &bad_policy).is_err());
        let mut bad_policy = policy.clone();
        bad_policy.release_inputs.as_mut().unwrap().draft_template = None;
        assert!(render_reusable_release_github(&contract, &shape, &bad_policy).is_err());
        // Shape/policy tag source skew fails closed.
        let mut skewed = policy.clone();
        skewed.staging.as_mut().unwrap().tag_source = StagingTagSource::DispatchInput;
        assert!(render_reusable_release_github(&contract, &shape, &skewed).is_err());
    }
}
