//! The provider-neutral CI plan: schema-v1 graph types, projection, and validation.

use crate::error::{fail, CiError};
use crate::validate::{safe_identifier, safe_metadata, safe_target};
use eggpack_contract::DistributionContract;
use eggpack_core::{
    cargo_command, BuildBindingsV1, HostRequirement, LogicalOutputSelector, PlannedAssetForm,
    PlannedTarget, Qualification, ReleasePlan, SupportTier,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

const MAX_CI_PLAN_JSON: usize = 1_000_000;

const MAX_OUTPUTS: usize = 4_096;

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
            // The producer owns the policy rules. This crate projects the
            // pipeline; it does not restate the toolchain, floor, or
            // native-qualification host rules, so it applies them instead.
            let policy_valid = eggpack_core::validate_policy(policy, &target.target).is_ok()
                && eggpack_core::validate_floor_applicability(
                    &policy.floor,
                    policy.strategy,
                    &target.target,
                )
                .is_ok();
            if !safe_target(&target.target)
                || target.policy.target != target.target
                || !policy_valid
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

pub(crate) fn handoff_name(target: &str, selector: &LogicalOutputSelector) -> String {
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
