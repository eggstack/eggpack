#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Provider-neutral CI planning and deterministic GitHub Actions generation.

mod consumer_validator;
mod drift;
mod error;
mod handoff;
mod identity;
mod planning;
mod policy;
mod release_plan;
mod render_github;
mod render_release;
mod runner;
mod validate;

#[cfg(test)]
mod tests;

pub use consumer_validator::{
    python_interpreter_exe, python_interpreter_exe_for_windows, run_consumer_validator,
    ConsumerValidationEvidenceV1, ConsumerValidationFailure, ConsumerValidationOutcome,
    ConsumerValidationRequest, ConsumerValidatorV1, ValidatorInterpreterV1, CONSUMER_EVIDENCE_FILE,
};
pub use drift::{check_github, check_release_github, check_reusable_release_github, DriftReport};
pub use error::CiError;
pub use handoff::{
    cargo_output_path, project_build_handoff, reconstruct_attempt, stage_build_artifact_dir,
    validate_build_artifact_dir, validate_qualification_artifact_dir, BuildHandoffOutput,
    BuildHandoffV1,
};
pub use identity::{
    resolve_runtime_release_plan, resolve_runtime_release_plan_with_mode, ReleaseWorkflowShapeV1,
    ShapeStagingIntentV1, RUNTIME_CI_PLAN, RUNTIME_GITHUB_POLICY, RUNTIME_IDENTITY_ARTIFACT,
    RUNTIME_IDENTITY_DIR, RUNTIME_RELEASE_PLAN,
};
pub use planning::{
    project_ci_plan, BuildOutput, CIPlan, QualificationIntent, QualificationState, TargetJob,
};
pub use policy::{
    validate_canonical_target_dir, validate_provisioned_zig_dir, validate_sha256_hex,
    validate_zig_archive_listing, zig_archive_name, zig_download_url, zig_expected_digest,
    ActionPin, CrossToolProvisioningV1, EggpackToolPolicy, GitHubPolicy, GitHubReleaseInputsV1,
    GitHubStagingInputsV1, GitHubStagingPolicyV1, ReleaseIdentityMode, RunnerMapping,
    StagingTagSource, WorkflowTrigger, ZigOfficialArchiveV1, BUILD_HANDOFF_FILE, CANDIDATES_DIR,
    GATE_OUTCOME_FILE, QUALIFICATION_EVIDENCE_FILE,
};
pub use release_plan::{
    aggregate_finalize, aggregate_finalize_with_consumer, consumer_evidence_handoff_name,
    consumer_job_id, decode_consumer_evidence, decode_qualification_evidence,
    encode_qualification_evidence, evaluate_gate, evaluate_gate_with_consumer,
    project_release_plan, project_release_plan_with_consumer, with_github_draft_staging,
    AggregateJob, AggregateOutcome, ArchiveEncodingWrapper, FinalizationSettings, QualificationJob,
    ReleaseCIPlanV1, StagingJob, StagingProvider,
};
pub use render_github::render_github;
pub use render_release::{render_release_github, render_reusable_release_github};
pub use runner::RunnerCommand;
