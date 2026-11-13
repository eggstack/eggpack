//! The release CI graph: schema-v1 projection, the qualification gate, aggregation, and the staging intent.

use crate::consumer_validator::{
    ConsumerValidationEvidenceV1, ConsumerValidationOutcome, ConsumerValidatorV1,
    MAX_CONSUMER_EVIDENCE_JSON,
};
use crate::error::{fail, CiError};
use crate::planning::CIPlan;
use crate::validate::safe_metadata;
use eggpack_contract::DistributionContract;
use eggpack_core::{
    ArchiveEncoding, BuildBindingsV1, FinalizationRequest, FinalizationTargetInput,
    HostRequirement, Qualification, QualificationBindingsV1, QualificationEvidence,
    QualificationStatus, ReleasePlan, SupportTier,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

const MAX_RELEASE_PLAN_JSON: usize = 1_000_000;

const MAX_EVIDENCE_JSON: usize = 1_000_000;

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
