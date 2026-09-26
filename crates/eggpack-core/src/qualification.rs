//! Typed qualification bindings, bounded native/QEMU execution, and candidate evidence.

use crate::{
    builder::{
        run_qemu_process, run_qualification_process, CandidateArtifact, CommandSpec,
        ProcessEvidence,
    },
    BuildAttempt, BuildBindingsV1, BuildCancellation, BuildStrategy, HostArch, HostOs,
    HostRequirement, LogicalOutputSelector, PlannedTarget, Qualification, ReleasePlan, SupportTier,
};
use eggpack_contract::DistributionContract;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_TARGETS: usize = 256;
const MAX_ARGS: usize = 128;
const MAX_ARG_LEN: usize = 4096;
const MAX_CAPTURE: usize = 256 * 1024;
const MAX_BINARY_HEADER: u64 = 4096;

/// Qualification binding validation or execution setup error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualificationError(String);
impl std::fmt::Display for QualificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for QualificationError {}
fn qerr(message: &str) -> QualificationError {
    QualificationError(message.to_owned())
}

/// One fixed-argument smoke invocation for an exact logical candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateSmokeBinding {
    /// Exact contract logical output selector; never inferred as a primary output.
    pub selector: LogicalOutputSelector,
    /// Fixed arguments passed directly to the selected candidate, without a shell.
    #[serde(default)]
    pub argv: Vec<String>,
    /// Per-process deadline in milliseconds (1..=86,400,000).
    pub timeout_ms: u64,
    /// Retained stdout bound (1..=262,144 bytes).
    pub stdout_limit: usize,
    /// Retained stderr bound (1..=262,144 bytes).
    pub stderr_limit: usize,
}

/// Qualification settings for one canonical planned target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TargetQualificationBinding {
    /// One candidate smoke check; required by Native, DeferredNative, and Emulated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke: Option<CandidateSmokeBinding>,
}

/// Strict producer-side qualification bindings schema version 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationBindingsV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Settings keyed by canonical target triple.
    pub targets: BTreeMap<String, TargetQualificationBinding>,
}

impl QualificationBindingsV1 {
    /// Strictly parse TOML qualification bindings.
    pub fn from_toml(text: &str) -> Result<Self, QualificationError> {
        let value: Self = toml::from_str(text).map_err(|_| qerr("invalid qualification TOML"))?;
        value.validate_shape()?;
        Ok(value)
    }

    /// Validate exact selected-target coverage and smoke selectors against build bindings.
    pub fn validate_for(
        &self,
        plan: &ReleasePlan,
        build_bindings: &BuildBindingsV1,
    ) -> Result<(), QualificationError> {
        self.validate_shape()?;
        if self.targets.len() != plan.targets.len()
            || self
                .targets
                .keys()
                .any(|target| !plan.targets.iter().any(|planned| &planned.target == target))
        {
            return Err(qerr(
                "qualification target inventory differs from ReleasePlan",
            ));
        }
        for target in &plan.targets {
            let settings = self
                .targets
                .get(&target.target)
                .ok_or_else(|| qerr("qualification target binding missing"))?;
            let must_smoke = matches!(
                target.policy.qualification,
                Qualification::Native | Qualification::DeferredNative | Qualification::Emulated
            );
            if settings.smoke.is_some() != must_smoke {
                return Err(qerr(
                    "smoke binding does not match qualification classification",
                ));
            }
            if let Some(smoke) = &settings.smoke {
                let candidates = build_bindings
                    .targets
                    .get(&target.target)
                    .ok_or_else(|| qerr("build target binding missing"))?;
                if !candidates
                    .iter()
                    .any(|candidate| candidate.selector == smoke.selector)
                {
                    return Err(qerr("smoke selector is absent from build bindings"));
                }
            }
            if target.policy.qualification == Qualification::DeferredNative
                && target.policy.qualification_host.is_none()
            {
                return Err(qerr("DeferredNative requires qualification_host"));
            }
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), QualificationError> {
        if self.schema_version != 1 || self.targets.is_empty() || self.targets.len() > MAX_TARGETS {
            return Err(qerr("unsupported version or invalid target count"));
        }
        for (target, binding) in &self.targets {
            if target.is_empty()
                || target.len() > 128
                || !target
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            {
                return Err(qerr("invalid canonical target key"));
            }
            if let Some(smoke) = &binding.smoke {
                if smoke.timeout_ms == 0
                    || smoke.timeout_ms > 86_400_000
                    || smoke.stdout_limit == 0
                    || smoke.stdout_limit > MAX_CAPTURE
                    || smoke.stderr_limit == 0
                    || smoke.stderr_limit > MAX_CAPTURE
                    || smoke.argv.len() > MAX_ARGS
                    || smoke.argv.iter().any(|arg| {
                        arg.len() > MAX_ARG_LEN
                            || arg.contains('\0')
                            || arg.bytes().any(|b| b.is_ascii_control())
                    })
                {
                    return Err(qerr("smoke bounds or arguments are invalid"));
                }
            }
        }
        Ok(())
    }
}

/// Bounded QEMU runtime input. The executable and flags are fixed by the target mapping.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QualificationRuntime {
    /// Optional absolute QEMU user-mode sysroot directory used with `-L`.
    pub qemu_sysroot: Option<PathBuf>,
}

/// Actual qualification method, distinct from requested policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationMethod {
    /// Candidate was directly run on the matching native qualification host.
    Native,
    /// DeferredNative was inspected on a different host and remains pending.
    Deferred,
    /// DeferredNative candidate ran on its matching later native host.
    DeferredNativeOnNativeHost,
    /// Candidate ran through a finite QEMU Linux user-mode mapping.
    QemuUser,
    /// Candidate structure was checked but no execution occurred.
    Structural,
}

/// Typed qualification outcome; a built candidate can never imply Passed by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationStatus {
    /// Structural and any required execution checks passed.
    Passed,
    /// DeferredNative execution is pending on its required host.
    Deferred,
    /// Qualification failed for the bounded reason below.
    Failed(QualificationFailure),
}

/// Bounded reason for failed qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationFailure {
    /// Candidate format or architecture does not match the planned target.
    StructuralMismatch,
    /// Candidate bytes changed during qualification.
    CandidateChanged,
    /// The executing host does not match the required qualification host.
    HostMismatch,
    /// Candidate smoke exited unsuccessfully or could not be started.
    SmokeFailed,
    /// Candidate smoke exceeded its deadline.
    SmokeTimedOut,
    /// Candidate smoke exceeded its output bound.
    OutputLimitExceeded,
    /// Required finite QEMU executable was unavailable or failed preflight.
    EmulatorUnavailable,
    /// QEMU process failed or returned a non-zero exit status.
    EmulatorFailed,
    /// Candidate could not be safely read or its runtime preconditions failed.
    InvalidRuntimeEnvironment,
}

/// Supported executable file formats checked structurally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateFormat {
    /// ELF executable.
    Elf,
    /// PE/COFF executable.
    PeCoff,
    /// Thin Mach-O executable.
    MachO,
}

/// Supported executable CPU architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateArchitecture {
    /// x86-64.
    X86_64,
    /// AArch64.
    Aarch64,
    /// 32-bit ARM.
    Armv7,
}

/// Exact candidate byte and structural identity observed for qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedCandidateEvidence {
    /// Contract logical output selector.
    pub selector: LogicalOutputSelector,
    /// Explicit M002 source package.
    pub package: String,
    /// Explicit M002 binary target.
    pub binary: String,
    /// Number of bytes observed.
    pub size: u64,
    /// SHA-256 of exact pre-qualification bytes.
    pub sha256: String,
    /// Detected executable format.
    pub format: CandidateFormat,
    /// Detected machine architecture.
    pub architecture: CandidateArchitecture,
}

/// One bounded process result without output contents or environment values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationProcessEvidence {
    /// `candidate_smoke` or `qemu_preflight`.
    pub purpose: String,
    /// Candidate selector associated with execution, absent for QEMU preflight.
    pub selector: Option<LogicalOutputSelector>,
    /// Exit/deadline/cancellation/output-limit classification.
    pub process: ProcessEvidence,
}

/// Complete bounded evidence for one release target qualification attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationEvidence {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Release identity from ReleasePlan and BuildAttempt.
    pub release_id: String,
    /// Source revision from ReleasePlan and BuildAttempt.
    pub source_revision: String,
    /// Canonical target triple.
    pub target: String,
    /// Requested qualification classification.
    pub planned_classification: Qualification,
    /// Method actually applied.
    pub method: QualificationMethod,
    /// Actual host capability used for this attempt.
    pub actual_host: HostRequirement,
    /// Support tier, recorded without changing result semantics.
    pub support: SupportTier,
    /// Typed result.
    pub status: QualificationStatus,
    /// Per-candidate evidence ordered by logical selector.
    pub candidates: Vec<QualifiedCandidateEvidence>,
    /// Configured smoke selector, if this mode has a smoke check.
    pub smoke_selector: Option<LogicalOutputSelector>,
    /// Bounded process outcome summaries.
    pub processes: Vec<QualificationProcessEvidence>,
}

impl QualificationEvidence {
    /// Validate evidence identity, candidate inventory, classification, and bounded fields.
    pub fn validate_for(
        &self,
        plan: &ReleasePlan,
        target: &PlannedTarget,
        attempt: &BuildAttempt,
    ) -> Result<(), QualificationError> {
        if self.schema_version != 1
            || !plan.targets.iter().any(|planned| planned == target)
            || self.release_id != plan.release_id
            || self.source_revision != plan.source_revision
            || attempt.release_id != plan.release_id
            || attempt.source_revision != plan.source_revision
            || self.target != target.target
            || self.planned_classification != target.policy.qualification
            || self.support != target.policy.support
            || (!matches!(self.status, QualificationStatus::Failed(_))
                && self.candidates.len() != attempt.candidates.len())
            || self.candidates.len() > 256
            || self.processes.len() > 1 + MAX_ARGS
        {
            return Err(qerr(
                "qualification evidence identity or bounds are invalid",
            ));
        }
        let mut expected_candidates = BTreeMap::new();
        for candidate in &attempt.candidates {
            if expected_candidates
                .insert(candidate.selector.clone(), candidate)
                .is_some()
            {
                return Err(qerr("BuildAttempt candidate selector is duplicated"));
            }
        }
        for record in &self.candidates {
            let candidate = expected_candidates
                .get(&record.selector)
                .ok_or_else(|| qerr("qualification evidence contains an unknown candidate"))?;
            if record.selector != candidate.selector
                || record.package != candidate.package
                || record.binary != candidate.binary
                || record.size != candidate.size
                || record.sha256.len() != 64
                || !record
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(qerr("qualification evidence candidate identity is invalid"));
            }
        }
        if self
            .candidates
            .windows(2)
            .any(|pair| pair[0].selector >= pair[1].selector)
            || self.processes.iter().any(|process| {
                process.purpose.len() > 64
                    || process.process.stdout_bytes > MAX_CAPTURE
                    || process.process.stderr_bytes > MAX_CAPTURE
            })
        {
            return Err(qerr(
                "qualification evidence ordering or process bounds are invalid",
            ));
        }
        let method_matches = matches!(
            (target.policy.qualification, self.method),
            (Qualification::Native, QualificationMethod::Native)
                | (Qualification::DeferredNative, QualificationMethod::Deferred)
                | (
                    Qualification::DeferredNative,
                    QualificationMethod::DeferredNativeOnNativeHost
                )
                | (Qualification::Emulated, QualificationMethod::QemuUser)
                | (Qualification::Structural, QualificationMethod::Structural)
        );
        if !method_matches
            || (self.status == QualificationStatus::Deferred
                && (self.method != QualificationMethod::Deferred
                    || !self.processes.is_empty()
                    || target.policy.qualification_host.is_none()
                    || target.policy.qualification_host == Some(self.actual_host)))
            || (self.status == QualificationStatus::Passed
                && self.method == QualificationMethod::Deferred)
            || (self.method == QualificationMethod::Structural
                && (!self.processes.is_empty() || self.smoke_selector.is_some()))
            || (self.status == QualificationStatus::Passed
                && matches!(
                    target.policy.qualification,
                    Qualification::Native | Qualification::DeferredNative | Qualification::Emulated
                )
                && self.smoke_selector.is_none())
            || (self.status == QualificationStatus::Passed
                && matches!(
                    self.method,
                    QualificationMethod::Native | QualificationMethod::DeferredNativeOnNativeHost
                )
                && !target_matches_host(&target.target, self.actual_host))
            || (self.status == QualificationStatus::Passed
                && target.policy.qualification == Qualification::Native
                && target
                    .policy
                    .qualification_host
                    .is_some_and(|required| required != self.actual_host))
            || (self.status == QualificationStatus::Passed
                && target.policy.qualification == Qualification::DeferredNative
                && target
                    .policy
                    .qualification_host
                    .is_some_and(|required| required != self.actual_host))
            || (self.status == QualificationStatus::Passed
                && target.policy.qualification == Qualification::Emulated
                && target
                    .policy
                    .qualification_host
                    .is_some_and(|required| required != self.actual_host))
            || (self.status == QualificationStatus::Passed
                && self
                    .processes
                    .iter()
                    .any(|evidence| evidence.process.outcome != crate::CommandOutcome::Success))
            || (self.status == QualificationStatus::Passed
                && self.processes.iter().any(|evidence| {
                    match (
                        self.method,
                        evidence.purpose.as_str(),
                        evidence.selector.as_ref(),
                    ) {
                        (
                            QualificationMethod::Native
                            | QualificationMethod::DeferredNativeOnNativeHost,
                            "candidate_smoke",
                            Some(selector),
                        ) => self.smoke_selector.as_ref() != Some(selector),
                        (QualificationMethod::QemuUser, "qemu_preflight", None) => false,
                        (QualificationMethod::QemuUser, "candidate_smoke", Some(selector)) => {
                            self.smoke_selector.as_ref() != Some(selector)
                        }
                        _ => true,
                    }
                }))
            || (self.status == QualificationStatus::Passed
                && match self.method {
                    QualificationMethod::Native
                    | QualificationMethod::DeferredNativeOnNativeHost => self.processes.len() != 1,
                    QualificationMethod::QemuUser => self.processes.len() != 2,
                    QualificationMethod::Structural => !self.processes.is_empty(),
                    QualificationMethod::Deferred => true,
                })
        {
            return Err(qerr(
                "qualification evidence method or status is inconsistent",
            ));
        }
        Ok(())
    }
}

/// All explicit producer-side inputs required to qualify one planned target.
pub struct QualificationRequest<'a> {
    /// Portable distribution identity authority.
    pub contract: &'a DistributionContract,
    /// Invocation release plan.
    pub plan: &'a ReleasePlan,
    /// One exact selected plan target.
    pub target: &'a PlannedTarget,
    /// Successful M002 candidate build attempt.
    pub attempt: &'a BuildAttempt,
    /// Exact package/binary to logical-slot bindings used by M002.
    pub build_bindings: &'a BuildBindingsV1,
    /// Exact qualification smoke policy.
    pub qualification_bindings: &'a QualificationBindingsV1,
    /// Optional explicitly supplied runtime-specific path inputs.
    pub runtime: &'a QualificationRuntime,
    /// Caller cancellation flag.
    pub cancellation: &'a BuildCancellation,
}

/// Execute M003 qualification using actual local host facts.
pub fn qualify_target(
    request: QualificationRequest<'_>,
) -> Result<QualificationEvidence, QualificationError> {
    let QualificationRequest {
        contract,
        plan,
        target,
        attempt,
        build_bindings,
        qualification_bindings,
        runtime,
        cancellation,
    } = request;
    let host = actual_host()?;
    qualify_target_for_host(
        contract,
        plan,
        target,
        attempt,
        build_bindings,
        qualification_bindings,
        runtime,
        cancellation,
        host,
    )
}

#[allow(clippy::too_many_arguments)]
fn qualify_target_for_host(
    contract: &DistributionContract,
    plan: &ReleasePlan,
    target: &PlannedTarget,
    attempt: &BuildAttempt,
    build_bindings: &BuildBindingsV1,
    qualification_bindings: &QualificationBindingsV1,
    runtime: &QualificationRuntime,
    cancellation: &BuildCancellation,
    host: HostRequirement,
) -> Result<QualificationEvidence, QualificationError> {
    let runner = |spec: &CommandSpec, qemu: bool| {
        let result = if qemu {
            run_qemu_process(spec, cancellation)
        } else {
            run_qualification_process(spec, cancellation)
        };
        result.map_err(|_| ())
    };
    qualify_target_for_host_with_runner(
        contract,
        plan,
        target,
        attempt,
        build_bindings,
        qualification_bindings,
        runtime,
        cancellation,
        host,
        &runner,
    )
}

#[allow(clippy::too_many_arguments)]
fn qualify_target_for_host_with_runner<F>(
    contract: &DistributionContract,
    plan: &ReleasePlan,
    target: &PlannedTarget,
    attempt: &BuildAttempt,
    build_bindings: &BuildBindingsV1,
    qualification_bindings: &QualificationBindingsV1,
    runtime: &QualificationRuntime,
    cancellation: &BuildCancellation,
    host: HostRequirement,
    runner: &F,
) -> Result<QualificationEvidence, QualificationError>
where
    F: Fn(&CommandSpec, bool) -> Result<ProcessEvidence, ()>,
{
    if plan.schema_version != 1
        || plan.release_id.is_empty()
        || plan.source_revision.is_empty()
        || !plan.targets.iter().any(|planned| planned == target)
        || attempt.release_id != plan.release_id
        || attempt.source_revision != plan.source_revision
        || attempt.target != target.target
        || attempt.strategy != target.policy.strategy
        || attempt.process.outcome != crate::CommandOutcome::Success
        || attempt.candidates.is_empty()
        || attempt.candidates.len() > 256
    {
        return Err(qerr(
            "BuildAttempt identity or result does not match ReleasePlan",
        ));
    }
    build_bindings
        .validate_for(contract, plan)
        .map_err(|_| qerr("BuildBindings do not exactly cover ReleasePlan"))?;
    qualification_bindings.validate_for(plan, build_bindings)?;
    validate_runtime(runtime)?;
    if target.policy.qualification == Qualification::Native
        && target.policy.strategy == BuildStrategy::CargoZigbuild
    {
        return Err(qerr("Native qualification cannot use CargoZigbuild"));
    }
    if target.policy.qualification != Qualification::Emulated && runtime.qemu_sysroot.is_some() {
        return Err(qerr(
            "QEMU sysroot is only valid for Emulated qualification",
        ));
    }

    let target_builds = build_bindings
        .targets
        .get(&target.target)
        .ok_or_else(|| qerr("build target binding missing"))?;
    let mut expected = BTreeMap::new();
    for binding in target_builds {
        expected.insert(
            binding.selector.clone(),
            (binding.package.as_str(), binding.binary.as_str()),
        );
    }
    let mut actual = BTreeMap::new();
    for candidate in &attempt.candidates {
        if candidate.target != target.target
            || actual
                .insert(
                    candidate.selector.clone(),
                    (
                        candidate.package.as_str(),
                        candidate.binary.as_str(),
                        candidate,
                    ),
                )
                .is_some()
        {
            return Err(qerr("candidate target or selector is duplicated"));
        }
    }
    if expected.len() != actual.len()
        || expected.iter().any(|(selector, (package, binary))| {
            actual
                .get(selector)
                .is_none_or(|(actual_package, actual_binary, _)| {
                    package != actual_package || binary != actual_binary
                })
        })
    {
        return Err(qerr(
            "BuildAttempt candidate inventory differs from BuildBindings",
        ));
    }

    let mut evidence_candidates = Vec::new();
    for (selector, (_, _, candidate)) in &actual {
        match inspect_candidate(candidate, &target.target) {
            Ok(value) => evidence_candidates.push(value),
            Err(failure) => {
                let method = match target.policy.qualification {
                    Qualification::Native => QualificationMethod::Native,
                    Qualification::DeferredNative => QualificationMethod::Deferred,
                    Qualification::Emulated => QualificationMethod::QemuUser,
                    Qualification::Structural => QualificationMethod::Structural,
                };
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    method,
                    QualificationStatus::Failed(failure),
                    Vec::new(),
                    None,
                    Vec::new(),
                ));
            }
        }
        debug_assert_eq!(
            evidence_candidates.last().map(|c| &c.selector),
            Some(selector)
        );
    }
    let settings = qualification_bindings
        .targets
        .get(&target.target)
        .expect("validated target qualification binding");
    let smoke = settings.smoke.as_ref();
    let selected_smoke = smoke.and_then(|binding| actual.get(&binding.selector).map(|v| v.2));
    if smoke.is_some() && selected_smoke.is_none() {
        return Err(qerr("smoke selector is absent from candidate inventory"));
    }

    let (method, should_execute) = match target.policy.qualification {
        Qualification::Structural => (QualificationMethod::Structural, false),
        Qualification::Native => {
            let required = target.policy.qualification_host.unwrap_or(HostRequirement {
                os: target.policy.host_os,
                arch: target.policy.host_arch,
            });
            if host != required || !target_matches_host(&target.target, host) {
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    QualificationMethod::Native,
                    QualificationStatus::Failed(QualificationFailure::HostMismatch),
                    evidence_candidates,
                    smoke.map(|s| s.selector.clone()),
                    Vec::new(),
                ));
            }
            (QualificationMethod::Native, true)
        }
        Qualification::DeferredNative => {
            let required = target
                .policy
                .qualification_host
                .ok_or_else(|| qerr("DeferredNative requires qualification_host"))?;
            if host != required {
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    QualificationMethod::Deferred,
                    QualificationStatus::Deferred,
                    evidence_candidates,
                    smoke.map(|s| s.selector.clone()),
                    Vec::new(),
                ));
            }
            if !target_matches_host(&target.target, host) {
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    QualificationMethod::DeferredNativeOnNativeHost,
                    QualificationStatus::Failed(QualificationFailure::HostMismatch),
                    evidence_candidates,
                    smoke.map(|s| s.selector.clone()),
                    Vec::new(),
                ));
            }
            (QualificationMethod::DeferredNativeOnNativeHost, true)
        }
        Qualification::Emulated => {
            if !target.target.contains("-linux-")
                || evidence_candidates
                    .iter()
                    .any(|c| c.format != CandidateFormat::Elf)
            {
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    QualificationMethod::QemuUser,
                    QualificationStatus::Failed(QualificationFailure::StructuralMismatch),
                    evidence_candidates,
                    smoke.map(|s| s.selector.clone()),
                    Vec::new(),
                ));
            }
            if target
                .policy
                .qualification_host
                .is_some_and(|required| required != host)
            {
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    QualificationMethod::QemuUser,
                    QualificationStatus::Failed(QualificationFailure::HostMismatch),
                    evidence_candidates,
                    smoke.map(|s| s.selector.clone()),
                    Vec::new(),
                ));
            }
            (QualificationMethod::QemuUser, true)
        }
    };
    if !should_execute {
        return Ok(make_evidence(
            plan,
            target,
            host,
            method,
            QualificationStatus::Passed,
            evidence_candidates,
            None,
            Vec::new(),
        ));
    }
    let smoke = smoke.ok_or_else(|| qerr("execution qualification requires a smoke binding"))?;
    let candidate = selected_smoke.expect("smoke selector was validated");
    let mut processes = Vec::new();
    if method == QualificationMethod::QemuUser {
        let emulator =
            qemu_for_target(&target.target).ok_or_else(|| qerr("unsupported QEMU target"))?;
        let preflight = CommandSpec {
            executable: emulator.into(),
            args: vec!["--version".into()],
            cwd: qualification_cwd(candidate)?,
            env: BTreeMap::new(),
            timeout: Duration::from_secs(10),
            stdout_limit: 8192,
            stderr_limit: 8192,
            expected_stdout: None,
        };
        let result = match runner(&preflight, true) {
            Ok(result) => result,
            Err(_) => {
                return Ok(make_evidence(
                    plan,
                    target,
                    host,
                    method,
                    QualificationStatus::Failed(QualificationFailure::EmulatorUnavailable),
                    evidence_candidates,
                    Some(smoke.selector.clone()),
                    processes,
                ))
            }
        };
        let preflight_ok = result.outcome == crate::CommandOutcome::Success;
        processes.push(QualificationProcessEvidence {
            purpose: "qemu_preflight".into(),
            selector: None,
            process: result,
        });
        if !preflight_ok {
            return Ok(make_evidence(
                plan,
                target,
                host,
                method,
                QualificationStatus::Failed(QualificationFailure::EmulatorUnavailable),
                evidence_candidates,
                Some(smoke.selector.clone()),
                processes,
            ));
        }
    }
    if cancellation.is_cancelled() {
        return Ok(make_evidence(
            plan,
            target,
            host,
            method,
            QualificationStatus::Failed(QualificationFailure::SmokeFailed),
            evidence_candidates,
            Some(smoke.selector.clone()),
            processes,
        ));
    }
    let (command, is_qemu) = if method == QualificationMethod::QemuUser {
        let command = CommandSpec {
            executable: qemu_for_target(&target.target)
                .expect("validated QEMU target")
                .into(),
            args: qemu_args(runtime, candidate, smoke),
            cwd: qualification_cwd(candidate)?,
            env: BTreeMap::new(),
            timeout: Duration::from_millis(smoke.timeout_ms),
            stdout_limit: smoke.stdout_limit,
            stderr_limit: smoke.stderr_limit,
            expected_stdout: None,
        };
        (command, true)
    } else {
        let command = CommandSpec {
            executable: candidate.path.to_string_lossy().into_owned(),
            args: smoke.argv.clone(),
            cwd: qualification_cwd(candidate)?,
            env: BTreeMap::new(),
            timeout: Duration::from_millis(smoke.timeout_ms),
            stdout_limit: smoke.stdout_limit,
            stderr_limit: smoke.stderr_limit,
            expected_stdout: None,
        };
        (command, false)
    };
    let process = match runner(&command, is_qemu) {
        Ok(process) => process,
        Err(_) => {
            return Ok(make_evidence(
                plan,
                target,
                host,
                method,
                QualificationStatus::Failed(if method == QualificationMethod::QemuUser {
                    QualificationFailure::EmulatorFailed
                } else {
                    QualificationFailure::SmokeFailed
                }),
                evidence_candidates,
                Some(smoke.selector.clone()),
                processes,
            ))
        }
    };
    let status = status_from_process(process.outcome, method);
    processes.push(QualificationProcessEvidence {
        purpose: "candidate_smoke".into(),
        selector: Some(smoke.selector.clone()),
        process,
    });
    let mut status = status;
    if status == QualificationStatus::Passed {
        for (selector, (_, _, candidate)) in &actual {
            match inspect_candidate(candidate, &target.target) {
                Ok(after) => {
                    let before = evidence_candidates
                        .iter()
                        .find(|evidence| &evidence.selector == selector)
                        .expect("preflight candidate evidence");
                    if after.size != before.size || after.sha256 != before.sha256 {
                        status =
                            QualificationStatus::Failed(QualificationFailure::CandidateChanged);
                        break;
                    }
                }
                Err(_) => {
                    status = QualificationStatus::Failed(QualificationFailure::CandidateChanged);
                    break;
                }
            }
        }
    }
    Ok(make_evidence(
        plan,
        target,
        host,
        method,
        status,
        evidence_candidates,
        Some(smoke.selector.clone()),
        processes,
    ))
}

#[allow(clippy::too_many_arguments)]
fn make_evidence(
    plan: &ReleasePlan,
    target: &PlannedTarget,
    host: HostRequirement,
    method: QualificationMethod,
    status: QualificationStatus,
    candidates: Vec<QualifiedCandidateEvidence>,
    smoke_selector: Option<LogicalOutputSelector>,
    processes: Vec<QualificationProcessEvidence>,
) -> QualificationEvidence {
    QualificationEvidence {
        schema_version: 1,
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        target: target.target.clone(),
        planned_classification: target.policy.qualification,
        method,
        actual_host: host,
        support: target.policy.support,
        status,
        candidates,
        smoke_selector,
        processes,
    }
}

fn status_from_process(
    outcome: crate::CommandOutcome,
    method: QualificationMethod,
) -> QualificationStatus {
    match outcome {
        crate::CommandOutcome::Success => QualificationStatus::Passed,
        crate::CommandOutcome::TimedOut => {
            QualificationStatus::Failed(if method == QualificationMethod::QemuUser {
                QualificationFailure::EmulatorFailed
            } else {
                QualificationFailure::SmokeTimedOut
            })
        }
        crate::CommandOutcome::OutputLimitExceeded => {
            QualificationStatus::Failed(QualificationFailure::OutputLimitExceeded)
        }
        crate::CommandOutcome::Failed(_) | crate::CommandOutcome::Cancelled => {
            QualificationStatus::Failed(if method == QualificationMethod::QemuUser {
                QualificationFailure::EmulatorFailed
            } else {
                QualificationFailure::SmokeFailed
            })
        }
    }
}

fn validate_runtime(runtime: &QualificationRuntime) -> Result<(), QualificationError> {
    if let Some(sysroot) = &runtime.qemu_sysroot {
        if !sysroot.is_absolute() {
            return Err(qerr("QEMU sysroot must be absolute"));
        }
        let metadata =
            fs::symlink_metadata(sysroot).map_err(|_| qerr("QEMU sysroot unavailable"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(qerr("QEMU sysroot must be a regular directory"));
        }
    }
    Ok(())
}

fn qualification_cwd(candidate: &CandidateArtifact) -> Result<PathBuf, QualificationError> {
    let root = candidate
        .path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| qerr("candidate qualification directory unavailable"))?;
    fs::canonicalize(root).map_err(|_| qerr("candidate qualification directory unavailable"))
}

fn actual_host() -> Result<HostRequirement, QualificationError> {
    let os = match std::env::consts::OS {
        "linux" => HostOs::Linux,
        "macos" => HostOs::Macos,
        "windows" => HostOs::Windows,
        _ => return Err(qerr("unsupported qualification host OS")),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => HostArch::X86_64,
        "aarch64" => HostArch::Aarch64,
        #[cfg(all(target_arch = "arm", target_feature = "v7"))]
        "arm" => HostArch::Armv7,
        _ => return Err(qerr("unsupported qualification host architecture")),
    };
    Ok(HostRequirement { os, arch })
}

fn target_matches_host(target: &str, host: HostRequirement) -> bool {
    let os = match host.os {
        HostOs::Linux => target.contains("-linux-"),
        HostOs::Macos => target.ends_with("-apple-darwin"),
        HostOs::Windows => target.contains("-windows-"),
    };
    let arch = match host.arch {
        HostArch::X86_64 => target.starts_with("x86_64-"),
        HostArch::Aarch64 => target.starts_with("aarch64-"),
        HostArch::Armv7 => target.starts_with("armv7-"),
    };
    os && arch
}

fn qemu_for_target(target: &str) -> Option<&'static str> {
    if !target.contains("-linux-") {
        return None;
    }
    match target.split('-').next()? {
        "x86_64" => Some("qemu-x86_64"),
        "aarch64" => Some("qemu-aarch64"),
        "armv7" => Some("qemu-arm"),
        _ => None,
    }
}

fn qemu_args(
    runtime: &QualificationRuntime,
    candidate: &CandidateArtifact,
    smoke: &CandidateSmokeBinding,
) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(sysroot) = runtime.qemu_sysroot.as_ref() {
        args.extend(["-L".to_owned(), sysroot.to_string_lossy().into_owned()]);
    }
    args.push(candidate.path.to_string_lossy().into_owned());
    args.extend(smoke.argv.clone());
    args
}

fn target_identity(target: &str) -> Option<(CandidateFormat, CandidateArchitecture)> {
    let format = if target.contains("-linux-") {
        CandidateFormat::Elf
    } else if target.ends_with("-apple-darwin") {
        CandidateFormat::MachO
    } else if target.contains("-windows-") {
        CandidateFormat::PeCoff
    } else {
        return None;
    };
    let architecture = if target.starts_with("x86_64-") {
        CandidateArchitecture::X86_64
    } else if target.starts_with("aarch64-") {
        CandidateArchitecture::Aarch64
    } else if target.starts_with("armv7-") {
        CandidateArchitecture::Armv7
    } else {
        return None;
    };
    if architecture == CandidateArchitecture::Armv7 && format != CandidateFormat::Elf {
        return None;
    }
    Some((format, architecture))
}

fn inspect_candidate(
    candidate: &CandidateArtifact,
    target: &str,
) -> Result<QualifiedCandidateEvidence, QualificationFailure> {
    let metadata = fs::symlink_metadata(&candidate.path)
        .map_err(|_| QualificationFailure::InvalidRuntimeEnvironment)?;
    if !candidate.path.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
    {
        return Err(QualificationFailure::InvalidRuntimeEnvironment);
    }
    if metadata.len() != candidate.size {
        return Err(QualificationFailure::CandidateChanged);
    }
    let canonical = fs::canonicalize(&candidate.path)
        .map_err(|_| QualificationFailure::InvalidRuntimeEnvironment)?;
    let target_dir = candidate
        .path
        .parent()
        .and_then(Path::parent)
        .ok_or(QualificationFailure::InvalidRuntimeEnvironment)?;
    let release_dir = candidate
        .path
        .parent()
        .ok_or(QualificationFailure::InvalidRuntimeEnvironment)?;
    for directory in [target_dir, release_dir] {
        let parent = fs::symlink_metadata(directory)
            .map_err(|_| QualificationFailure::InvalidRuntimeEnvironment)?;
        if parent.file_type().is_symlink() || !parent.is_dir() {
            return Err(QualificationFailure::InvalidRuntimeEnvironment);
        }
    }
    let canonical_root = fs::canonicalize(target_dir)
        .map_err(|_| QualificationFailure::InvalidRuntimeEnvironment)?;
    if !canonical.starts_with(canonical_root) {
        return Err(QualificationFailure::InvalidRuntimeEnvironment);
    }
    let (expected_format, expected_arch) =
        target_identity(target).ok_or(QualificationFailure::StructuralMismatch)?;
    let format_arch = parse_binary_header(&canonical, expected_format)
        .ok_or(QualificationFailure::StructuralMismatch)?;
    if format_arch != (expected_format, expected_arch) {
        return Err(QualificationFailure::StructuralMismatch);
    }
    let mut file =
        File::open(&canonical).map_err(|_| QualificationFailure::InvalidRuntimeEnvironment)?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| QualificationFailure::InvalidRuntimeEnvironment)?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .ok_or(QualificationFailure::InvalidRuntimeEnvironment)?;
        hasher.update(&buffer[..read]);
    }
    let after = fs::symlink_metadata(&candidate.path)
        .map_err(|_| QualificationFailure::CandidateChanged)?;
    if size != metadata.len()
        || after.file_type().is_symlink()
        || !after.is_file()
        || after.len() != size
        || fs::canonicalize(&candidate.path).ok().as_ref() != Some(&canonical)
    {
        return Err(QualificationFailure::CandidateChanged);
    }
    Ok(QualifiedCandidateEvidence {
        selector: candidate.selector.clone(),
        package: candidate.package.clone(),
        binary: candidate.binary.clone(),
        size,
        sha256: hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        format: expected_format,
        architecture: expected_arch,
    })
}

fn parse_binary_header(
    path: &Path,
    format: CandidateFormat,
) -> Option<(CandidateFormat, CandidateArchitecture)> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    match format {
        CandidateFormat::Elf => {
            let mut header = [0u8; 20];
            file.read_exact(&mut header).ok()?;
            if &header[..4] != b"\x7fELF" || header[5] != 1 {
                return None;
            }
            let architecture = match u16::from_le_bytes([header[18], header[19]]) {
                62 if header[4] == 2 => CandidateArchitecture::X86_64,
                183 if header[4] == 2 => CandidateArchitecture::Aarch64,
                40 if header[4] == 1 => CandidateArchitecture::Armv7,
                _ => return None,
            };
            Some((CandidateFormat::Elf, architecture))
        }
        CandidateFormat::PeCoff => {
            let mut dos = [0u8; 64];
            file.read_exact(&mut dos).ok()?;
            if &dos[..2] != b"MZ" {
                return None;
            }
            let offset = u32::from_le_bytes(dos[60..64].try_into().ok()?) as u64;
            if !(64..=MAX_BINARY_HEADER).contains(&offset) || offset.checked_add(6)? > length {
                return None;
            }
            file.seek(SeekFrom::Start(offset)).ok()?;
            let mut header = [0u8; 6];
            file.read_exact(&mut header).ok()?;
            if &header[..4] != b"PE\0\0" {
                return None;
            }
            let architecture = match u16::from_le_bytes([header[4], header[5]]) {
                0x8664 => CandidateArchitecture::X86_64,
                0xAA64 => CandidateArchitecture::Aarch64,
                0x01c4 => CandidateArchitecture::Armv7,
                _ => return None,
            };
            Some((CandidateFormat::PeCoff, architecture))
        }
        CandidateFormat::MachO => {
            let mut header = [0u8; 8];
            file.read_exact(&mut header).ok()?;
            let endian = match &header[..4] {
                [0xfe, 0xed, 0xfa, 0xcf] => false,
                [0xcf, 0xfa, 0xed, 0xfe] => true,
                _ => return None,
            };
            let cpu = if endian {
                u32::from_le_bytes(header[4..8].try_into().ok()?)
            } else {
                u32::from_be_bytes(header[4..8].try_into().ok()?)
            };
            let architecture = match cpu {
                0x01000007 => CandidateArchitecture::X86_64,
                0x0100000c => CandidateArchitecture::Aarch64,
                _ => return None,
            };
            Some((CandidateFormat::MachO, architecture))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BuildAttempt, BuildBindingsV1, BuildStrategy, CandidateArtifact, CommandOutcome,
        LogicalOutputSelector, PlannedAssetForm, PlannedTarget, ProcessEvidence, TargetPolicy,
        ToolchainRequirement,
    };
    use eggpack_contract::DistributionContract;

    fn local_host() -> HostRequirement {
        let os = match std::env::consts::OS {
            "linux" => HostOs::Linux,
            "macos" => HostOs::Macos,
            "windows" => HostOs::Windows,
            _ => panic!("unsupported test host"),
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => HostArch::X86_64,
            "aarch64" => HostArch::Aarch64,
            "arm" => HostArch::Armv7,
            _ => panic!("unsupported test architecture"),
        };
        HostRequirement { os, arch }
    }

    fn local_target() -> &'static str {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
            ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
            ("linux", "arm") => "armv7-unknown-linux-gnueabihf",
            ("macos", "x86_64") => "x86_64-apple-darwin",
            ("macos", "aarch64") => "aarch64-apple-darwin",
            ("windows", "x86_64") => "x86_64-pc-windows-msvc",
            ("windows", "aarch64") => "aarch64-pc-windows-msvc",
            _ => panic!("unsupported test target"),
        }
    }

    fn contract(target: &str) -> DistributionContract {
        DistributionContract::parse_toml_str(&format!(
            "schema_version = 1\n[product]\nid = \"fixture\"\ndisplay_name = \"fixture\"\n[[targets]]\ntriple = \"{target}\"\naliases = []\n[targets.asset]\nkind = \"direct\"\nasset = \"{{product}}-{{version}}-{{target}}\"\ninstall = \"fixture\"\n[targets.checksum]\nsidecar = \"{{asset}}.sha256\"\n"
        ))
        .unwrap()
    }

    fn candidate_path(root: &Path, target: &str) -> PathBuf {
        let mut path = root.join(target).join("release").join("fixture");
        if target.contains("windows") {
            path.set_extension("exe");
        }
        path
    }

    fn fixture(
        classification: Qualification,
        candidate_bytes: &[u8],
    ) -> (
        DistributionContract,
        ReleasePlan,
        PlannedTarget,
        BuildAttempt,
        BuildBindingsV1,
        QualificationBindingsV1,
        PathBuf,
    ) {
        let target_name = local_target();
        let root = crate::test_temp_dir("qualification");
        let path = candidate_path(&root, target_name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, candidate_bytes).unwrap();
        let host = local_host();
        let target = PlannedTarget {
            target: target_name.into(),
            policy: TargetPolicy {
                target: target_name.into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: host.os,
                host_arch: host.arch,
                qualification_host: Some(host),
                toolchain: ToolchainRequirement {
                    rust: "stable".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: crate::CompatibilityFloor::None,
                qualification: classification,
                support: SupportTier::Required,
            },
            artifact_form: PlannedAssetForm::Direct,
        };
        let plan = ReleasePlan {
            schema_version: 1,
            release_id: "v1".into(),
            source_revision: "abc123".into(),
            targets: vec![target.clone()],
        };
        let build_bindings = BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                target_name.into(),
                vec![crate::BuildBinding {
                    selector: LogicalOutputSelector::Direct,
                    package: "fixture".into(),
                    binary: "fixture".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let qualification_bindings = QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                target_name.into(),
                TargetQualificationBinding {
                    smoke: matches!(
                        classification,
                        Qualification::Native
                            | Qualification::DeferredNative
                            | Qualification::Emulated
                    )
                    .then(|| CandidateSmokeBinding {
                        selector: LogicalOutputSelector::Direct,
                        argv: vec![],
                        timeout_ms: 5000,
                        stdout_limit: 4096,
                        stderr_limit: 4096,
                    }),
                },
            )]
            .into_iter()
            .collect(),
        };
        let attempt = BuildAttempt {
            release_id: plan.release_id.clone(),
            source_revision: plan.source_revision.clone(),
            target: target_name.into(),
            strategy: BuildStrategy::NativeCargo,
            tool_summary: "rust stable".into(),
            process: ProcessEvidence {
                outcome: CommandOutcome::Success,
                stdout_bytes: 0,
                stderr_bytes: 0,
            },
            candidates: vec![CandidateArtifact {
                target: target_name.into(),
                selector: LogicalOutputSelector::Direct,
                package: "fixture".into(),
                binary: "fixture".into(),
                path,
                size: candidate_bytes.len() as u64,
            }],
        };
        (
            contract(target_name),
            plan,
            target,
            attempt,
            build_bindings,
            qualification_bindings,
            root,
        )
    }

    fn elf(architecture: CandidateArchitecture) -> Vec<u8> {
        let mut bytes = vec![0u8; 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[5] = 1;
        let (class, machine) = match architecture {
            CandidateArchitecture::X86_64 => (2, 62u16),
            CandidateArchitecture::Aarch64 => (2, 183),
            CandidateArchitecture::Armv7 => (1, 40),
        };
        bytes[4] = class;
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    fn pe(architecture: CandidateArchitecture) -> Vec<u8> {
        let mut bytes = vec![0u8; 128 + 6];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        let machine = match architecture {
            CandidateArchitecture::X86_64 => 0x8664u16,
            CandidateArchitecture::Aarch64 => 0xAA64,
            CandidateArchitecture::Armv7 => 0x01c4,
        };
        bytes[132..134].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    fn macho(architecture: CandidateArchitecture) -> Vec<u8> {
        let mut bytes = vec![0u8; 32];
        bytes[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        let cpu = match architecture {
            CandidateArchitecture::X86_64 => 0x01000007u32,
            CandidateArchitecture::Aarch64 => 0x0100000c,
            CandidateArchitecture::Armv7 => 12,
        };
        bytes[4..8].copy_from_slice(&cpu.to_le_bytes());
        bytes
    }

    #[test]
    fn structural_headers_cover_elf_pe_and_thin_macho_targets() {
        for arch in [
            CandidateArchitecture::X86_64,
            CandidateArchitecture::Aarch64,
            CandidateArchitecture::Armv7,
        ] {
            assert_header(
                &elf(arch),
                CandidateFormat::Elf,
                Some((CandidateFormat::Elf, arch)),
            );
            assert_header(
                &pe(arch),
                CandidateFormat::PeCoff,
                Some((CandidateFormat::PeCoff, arch)),
            );
        }
        for arch in [
            CandidateArchitecture::X86_64,
            CandidateArchitecture::Aarch64,
        ] {
            assert_header(
                &macho(arch),
                CandidateFormat::MachO,
                Some((CandidateFormat::MachO, arch)),
            );
        }
        assert_header(b"bad", CandidateFormat::Elf, None);
        let mut fat = macho(CandidateArchitecture::X86_64);
        fat[..4].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe]);
        assert_header(&fat, CandidateFormat::MachO, None);
        let mut bad_pe = pe(CandidateArchitecture::X86_64);
        bad_pe[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_header(&bad_pe, CandidateFormat::PeCoff, None);
        assert_eq!(
            qemu_for_target("x86_64-unknown-linux-gnu"),
            Some("qemu-x86_64")
        );
        assert_eq!(
            qemu_for_target("aarch64-unknown-linux-gnu"),
            Some("qemu-aarch64")
        );
        assert_eq!(
            qemu_for_target("armv7-unknown-linux-gnueabihf"),
            Some("qemu-arm")
        );
        assert_eq!(qemu_for_target("x86_64-pc-windows-msvc"), None);
    }

    fn assert_header(
        bytes: &[u8],
        format: CandidateFormat,
        expected: Option<(CandidateFormat, CandidateArchitecture)>,
    ) {
        let root = crate::test_temp_dir("header");
        let path = root.join("header");
        fs::write(&path, bytes).unwrap();
        assert_eq!(parse_binary_header(&path, format), expected);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn structural_classification_passes_without_execution() {
        let host = local_host();
        let target_name = local_target();
        let bytes = match target_identity(target_name).unwrap().0 {
            CandidateFormat::Elf => elf(host.arch.into()),
            CandidateFormat::PeCoff => pe(host.arch.into()),
            CandidateFormat::MachO => macho(host.arch.into()),
        };
        let (contract, plan, target, attempt, builds, quals, root) =
            fixture(Qualification::Structural, &bytes);
        let evidence = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::Passed);
        assert_eq!(evidence.method, QualificationMethod::Structural);
        assert!(evidence.processes.is_empty());
        assert_eq!(evidence.candidates[0].sha256.len(), 64);
        fs::remove_dir_all(root).unwrap();
    }

    impl From<HostArch> for CandidateArchitecture {
        fn from(value: HostArch) -> Self {
            match value {
                HostArch::X86_64 => Self::X86_64,
                HostArch::Aarch64 => Self::Aarch64,
                HostArch::Armv7 => Self::Armv7,
            }
        }
    }

    #[test]
    fn build_and_qualification_release_identity_mismatch_is_rejected() {
        let host = local_host();
        let bytes = match target_identity(local_target()).unwrap().0 {
            CandidateFormat::Elf => elf(host.arch.into()),
            CandidateFormat::PeCoff => pe(host.arch.into()),
            CandidateFormat::MachO => macho(host.arch.into()),
        };
        let (contract, plan, target, mut attempt, builds, quals, root) =
            fixture(Qualification::Structural, &bytes);
        attempt.source_revision = "other".into();
        assert!(qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deferred_native_stays_pending_off_the_required_host() {
        let host = local_host();
        let bytes = match target_identity(local_target()).unwrap().0 {
            CandidateFormat::Elf => elf(host.arch.into()),
            CandidateFormat::PeCoff => pe(host.arch.into()),
            CandidateFormat::MachO => macho(host.arch.into()),
        };
        let (contract, plan, mut target, attempt, builds, quals, root) =
            fixture(Qualification::DeferredNative, &bytes);
        let other = HostRequirement {
            os: if host.os == HostOs::Linux {
                HostOs::Macos
            } else {
                HostOs::Linux
            },
            arch: host.arch,
        };
        target.policy.qualification_host = Some(other);
        let plan = ReleasePlan {
            targets: vec![target.clone()],
            ..plan
        };
        let evidence = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::Deferred);
        assert!(evidence.processes.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn qualification_bindings_are_strict_and_smoke_selector_is_exact() {
        let good = r#"schema_version = 1
[targets."x86_64-unknown-linux-gnu".smoke]
selector = { kind = "direct" }
argv = []
timeout_ms = 5000
stdout_limit = 1024
stderr_limit = 1024
"#;
        assert!(QualificationBindingsV1::from_toml(good).is_ok());
        assert!(QualificationBindingsV1::from_toml(
            &good.replace("schema_version = 1", "schema_version = 2")
        )
        .is_err());
        assert!(QualificationBindingsV1::from_toml(
            &good.replace("selector =", "extra = 1\nselector =")
        )
        .is_err());
    }

    #[test]
    fn classification_requires_the_exact_smoke_and_host_configuration() {
        let bytes = elf(CandidateArchitecture::X86_64);
        let (_contract, plan, target, _attempt, builds, mut quals, root) =
            fixture(Qualification::Native, &bytes);
        quals.targets.get_mut(&target.target).unwrap().smoke = None;
        assert!(quals.validate_for(&plan, &builds).is_err());
        fs::remove_dir_all(root).unwrap();

        let (contract, plan, target, attempt, builds, _quals, root) =
            fixture(Qualification::Structural, &bytes);
        let structural = QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                target.target.clone(),
                TargetQualificationBinding {
                    smoke: Some(CandidateSmokeBinding {
                        selector: LogicalOutputSelector::Direct,
                        argv: vec![],
                        timeout_ms: 1000,
                        stdout_limit: 1024,
                        stderr_limit: 1024,
                    }),
                },
            )]
            .into_iter()
            .collect(),
        };
        assert!(structural.validate_for(&plan, &builds).is_err());
        assert!(qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &structural,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            local_host(),
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_smoke_executes_the_exact_candidate_on_a_matching_host() {
        let host = local_host();
        let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
        let (contract, plan, target, mut attempt, builds, mut quals, root) =
            fixture(Qualification::Native, &bytes);
        fs::copy(
            std::env::current_exe().unwrap(),
            &attempt.candidates[0].path,
        )
        .unwrap();
        attempt.candidates[0].size = fs::metadata(&attempt.candidates[0].path).unwrap().len();
        let smoke = quals
            .targets
            .get_mut(&target.target)
            .unwrap()
            .smoke
            .as_mut()
            .unwrap();
        smoke.argv = vec![
            "--exact".into(),
            "qualification::tests::qualification_child_target".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        let evidence = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::Passed);
        assert_eq!(evidence.method, QualificationMethod::Native);
        assert_eq!(
            evidence.processes[0].process.outcome,
            CommandOutcome::Success
        );
        assert!(evidence.validate_for(&plan, &target, &attempt).is_ok());
        let json = serde_json::to_string(&evidence).unwrap();
        assert!(!json.contains(&attempt.candidates[0].path.to_string_lossy().to_string()));
        assert_eq!(json, serde_json::to_string(&evidence).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_inventory_and_structural_architecture_must_match() {
        let host = local_host();
        let wrong_arch = match host.arch {
            HostArch::X86_64 => CandidateArchitecture::Aarch64,
            _ => CandidateArchitecture::X86_64,
        };
        let bytes = match target_identity(local_target()).unwrap().0 {
            CandidateFormat::Elf => elf(wrong_arch),
            CandidateFormat::PeCoff => pe(wrong_arch),
            CandidateFormat::MachO => macho(wrong_arch),
        };
        let (contract, plan, target, mut attempt, builds, quals, root) =
            fixture(Qualification::Structural, &bytes);
        let result = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(
            result.status,
            QualificationStatus::Failed(QualificationFailure::StructuralMismatch)
        );
        attempt.candidates.clear();
        assert!(qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_host_mismatch_is_a_failure_and_never_runs_smoke() {
        let host = local_host();
        let bytes = match target_identity(local_target()).unwrap().0 {
            CandidateFormat::Elf => elf(host.arch.into()),
            CandidateFormat::PeCoff => pe(host.arch.into()),
            CandidateFormat::MachO => macho(host.arch.into()),
        };
        let (contract, plan, mut target, attempt, builds, quals, root) =
            fixture(Qualification::Native, &bytes);
        let other = HostRequirement {
            os: if host.os == HostOs::Linux {
                HostOs::Macos
            } else {
                HostOs::Linux
            },
            arch: host.arch,
        };
        target.policy.qualification_host = Some(other);
        let plan = ReleasePlan {
            targets: vec![target.clone()],
            ..plan
        };
        let evidence = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            QualificationStatus::Failed(QualificationFailure::HostMismatch)
        );
        assert!(evidence.processes.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deferred_native_matching_host_executes_and_wrong_host_stays_deferred() {
        let host = local_host();
        let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
        let (contract, plan, mut target, mut attempt, builds, mut quals, root) =
            fixture(Qualification::DeferredNative, &bytes);
        fs::copy(
            std::env::current_exe().unwrap(),
            &attempt.candidates[0].path,
        )
        .unwrap();
        attempt.candidates[0].size = fs::metadata(&attempt.candidates[0].path).unwrap().len();
        let smoke = quals
            .targets
            .get_mut(&target.target)
            .unwrap()
            .smoke
            .as_mut()
            .unwrap();
        smoke.argv = vec![
            "--exact".into(),
            "qualification::tests::qualification_child_target".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        let other = HostRequirement {
            os: if host.os == HostOs::Linux {
                HostOs::Macos
            } else {
                HostOs::Linux
            },
            arch: host.arch,
        };
        target.policy.qualification_host = Some(other);
        let deferred_plan = ReleasePlan {
            targets: vec![target.clone()],
            ..plan.clone()
        };
        let deferred = qualify_target_for_host(
            &contract,
            &deferred_plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(deferred.status, QualificationStatus::Deferred);
        assert!(deferred.processes.is_empty());

        target.policy.qualification_host = Some(host);
        let matching_plan = ReleasePlan {
            targets: vec![target.clone()],
            ..plan
        };
        let passed = qualify_target_for_host(
            &contract,
            &matching_plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert_eq!(passed.status, QualificationStatus::Passed);
        assert_eq!(
            passed.method,
            QualificationMethod::DeferredNativeOnNativeHost
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn qemu_command_mapping_is_fixed_and_uses_only_the_declared_sysroot() {
        let sysroot = PathBuf::from("/fixture/sysroot");
        let runtime = QualificationRuntime {
            qemu_sysroot: Some(sysroot),
        };
        let candidate = CandidateArtifact {
            target: "x86_64-unknown-linux-gnu".into(),
            selector: LogicalOutputSelector::Direct,
            package: "fixture".into(),
            binary: "fixture".into(),
            path: PathBuf::from("/private/candidate"),
            size: 1,
        };
        let smoke = CandidateSmokeBinding {
            selector: LogicalOutputSelector::Direct,
            argv: vec!["--smoke".into()],
            timeout_ms: 1000,
            stdout_limit: 1024,
            stderr_limit: 1024,
        };
        let args = qemu_args(&runtime, &candidate, &smoke);
        assert_eq!(
            qemu_for_target("x86_64-unknown-linux-gnu"),
            Some("qemu-x86_64")
        );
        assert_eq!(
            args,
            vec!["-L", "/fixture/sysroot", "/private/candidate", "--smoke"]
        );
        assert_eq!(qemu_for_target("aarch64-apple-darwin"), None);
    }

    #[test]
    fn emulated_linux_path_uses_injected_finite_qemu_runner() {
        let target_name = "x86_64-unknown-linux-gnu";
        let contract = contract(target_name);
        let host = local_host();
        let target = PlannedTarget {
            target: target_name.into(),
            policy: TargetPolicy {
                target: target_name.into(),
                strategy: BuildStrategy::CargoZigbuild,
                host_os: host.os,
                host_arch: host.arch,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "stable".into(),
                    cargo_zigbuild: Some("0.20.0".into()),
                    zig: Some("0.14.1".into()),
                },
                floor: crate::CompatibilityFloor::None,
                qualification: Qualification::Emulated,
                support: SupportTier::NonGating,
            },
            artifact_form: PlannedAssetForm::Direct,
        };
        let plan = ReleasePlan {
            schema_version: 1,
            release_id: "v2".into(),
            source_revision: "def456".into(),
            targets: vec![target.clone()],
        };
        let root = crate::test_temp_dir("qemu-fixture");
        let path = candidate_path(&root, target_name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = elf(CandidateArchitecture::X86_64);
        fs::write(&path, &bytes).unwrap();
        let builds = BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                target_name.into(),
                vec![crate::BuildBinding {
                    selector: LogicalOutputSelector::Direct,
                    package: "fixture".into(),
                    binary: "fixture".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let quals = QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                target_name.into(),
                TargetQualificationBinding {
                    smoke: Some(CandidateSmokeBinding {
                        selector: LogicalOutputSelector::Direct,
                        argv: vec!["--smoke".into()],
                        timeout_ms: 2000,
                        stdout_limit: 1024,
                        stderr_limit: 1024,
                    }),
                },
            )]
            .into_iter()
            .collect(),
        };
        let attempt = BuildAttempt {
            release_id: plan.release_id.clone(),
            source_revision: plan.source_revision.clone(),
            target: target_name.into(),
            strategy: BuildStrategy::CargoZigbuild,
            tool_summary: "rust stable cargo-zigbuild 0.20.0".into(),
            process: ProcessEvidence {
                outcome: CommandOutcome::Success,
                stdout_bytes: 0,
                stderr_bytes: 0,
            },
            candidates: vec![CandidateArtifact {
                target: target_name.into(),
                selector: LogicalOutputSelector::Direct,
                package: "fixture".into(),
                binary: "fixture".into(),
                path: path.clone(),
                size: bytes.len() as u64,
            }],
        };
        let commands = std::sync::Mutex::new(Vec::new());
        let runner = |spec: &CommandSpec, qemu: bool| {
            assert!(qemu);
            commands.lock().unwrap().push(spec.clone());
            Ok(ProcessEvidence {
                outcome: CommandOutcome::Success,
                stdout_bytes: 10,
                stderr_bytes: 0,
            })
        };
        let evidence = qualify_target_for_host_with_runner(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
            &runner,
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::Passed);
        assert_eq!(evidence.method, QualificationMethod::QemuUser);
        assert!(evidence.validate_for(&plan, &target, &attempt).is_ok());
        let commands = commands.into_inner().unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].executable, "qemu-x86_64");
        assert_eq!(commands[0].args, ["--version"]);
        assert_eq!(commands[1].executable, "qemu-x86_64");
        assert_eq!(
            commands[1].args,
            vec![path.to_string_lossy().into_owned(), "--smoke".into()]
        );
        let unavailable_runner = |spec: &CommandSpec, qemu: bool| {
            assert!(qemu);
            assert_eq!(spec.args, ["--version"]);
            Ok(ProcessEvidence {
                outcome: CommandOutcome::Failed(127),
                stdout_bytes: 0,
                stderr_bytes: 0,
            })
        };
        let unavailable = qualify_target_for_host_with_runner(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
            &unavailable_runner,
        )
        .unwrap();
        assert_eq!(
            unavailable.status,
            QualificationStatus::Failed(QualificationFailure::EmulatorUnavailable)
        );
        assert_eq!(unavailable.processes.len(), 1);
        assert_eq!(unavailable.processes[0].purpose, "qemu_preflight");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn qualification_evidence_validation_reconciles_plan_and_candidates() {
        let host = local_host();
        let bytes = match target_identity(local_target()).unwrap().0 {
            CandidateFormat::Elf => elf(host.arch.into()),
            CandidateFormat::PeCoff => pe(host.arch.into()),
            CandidateFormat::MachO => macho(host.arch.into()),
        };
        let (contract, plan, target, attempt, builds, quals, root) =
            fixture(Qualification::Structural, &bytes);
        let evidence = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &BuildCancellation::new(),
            host,
        )
        .unwrap();
        assert!(evidence.validate_for(&plan, &target, &attempt).is_ok());
        let json = serde_json::to_string(&evidence).unwrap();
        let roundtrip: QualificationEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, evidence);
        assert!(roundtrip.validate_for(&plan, &target, &attempt).is_ok());
        let mut corrupted = roundtrip;
        corrupted.release_id = "other-release".into();
        assert!(corrupted.validate_for(&plan, &target, &attempt).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_digest_changes_when_same_size_bytes_change() {
        let host = local_host();
        let bytes = match target_identity(local_target()).unwrap().0 {
            CandidateFormat::Elf => elf(host.arch.into()),
            CandidateFormat::PeCoff => pe(host.arch.into()),
            CandidateFormat::MachO => macho(host.arch.into()),
        };
        let (_contract, _plan, target, attempt, _builds, _quals, root) =
            fixture(Qualification::Structural, &bytes);
        let before = inspect_candidate(&attempt.candidates[0], &target.target).unwrap();
        let mut changed = bytes.clone();
        // Keep the structural header intact on all host formats. The final
        // byte in the minimal PE fixture is part of its COFF machine field.
        changed[20] ^= 1;
        fs::write(&attempt.candidates[0].path, &changed).unwrap();
        let after = inspect_candidate(&attempt.candidates[0], &target.target).unwrap();
        assert_eq!(before.size, after.size);
        assert_ne!(before.sha256, after.sha256);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn candidate_symlink_and_relative_paths_are_rejected() {
        use std::os::unix::fs::symlink;

        let host = local_host();
        let bytes = elf(host.arch.into());
        let (_contract, _plan, target, mut attempt, _builds, _quals, root) =
            fixture(Qualification::Structural, &bytes);
        let candidate = &mut attempt.candidates[0];
        let original = candidate.path.with_extension("original");
        fs::rename(&candidate.path, &original).unwrap();
        symlink(&original, &candidate.path).unwrap();
        assert_eq!(
            inspect_candidate(candidate, &target.target),
            Err(QualificationFailure::InvalidRuntimeEnvironment)
        );
        fs::remove_file(&candidate.path).unwrap();
        fs::rename(original, &candidate.path).unwrap();
        candidate.path = PathBuf::from("relative-candidate");
        assert_eq!(
            inspect_candidate(candidate, &target.target),
            Err(QualificationFailure::InvalidRuntimeEnvironment)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore]
    fn qualification_child_target() {}

    #[test]
    #[ignore]
    fn qualification_child_sleep() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore]
    fn qualification_child_output() {
        print!("{}", "x".repeat(16 * 1024));
    }

    #[test]
    #[ignore]
    fn qualification_child_nonzero() {
        std::process::exit(7);
    }

    fn run_native_child(
        child: &str,
        timeout_ms: u64,
        stdout_limit: usize,
        cancel_after: Option<Duration>,
    ) -> QualificationEvidence {
        let host = local_host();
        let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
        let (contract, plan, target, mut attempt, builds, mut quals, root) =
            fixture(Qualification::Native, &bytes);
        fs::copy(
            std::env::current_exe().unwrap(),
            &attempt.candidates[0].path,
        )
        .unwrap();
        attempt.candidates[0].size = fs::metadata(&attempt.candidates[0].path).unwrap().len();
        let smoke = quals
            .targets
            .get_mut(&target.target)
            .unwrap()
            .smoke
            .as_mut()
            .unwrap();
        smoke.argv = vec![
            "--exact".into(),
            format!("qualification::tests::{child}"),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        smoke.timeout_ms = timeout_ms;
        smoke.stdout_limit = stdout_limit;
        let cancellation = BuildCancellation::new();
        let cancel_thread = cancel_after.map(|delay| {
            let cancellation = cancellation.clone();
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                cancellation.cancel();
            })
        });
        let evidence = qualify_target_for_host(
            &contract,
            &plan,
            &target,
            &attempt,
            &builds,
            &quals,
            &QualificationRuntime::default(),
            &cancellation,
            host,
        )
        .unwrap();
        if let Some(thread) = cancel_thread {
            thread.join().unwrap();
        }
        fs::remove_dir_all(root).unwrap();
        evidence
    }

    #[test]
    fn native_smoke_failure_timeout_cancellation_and_output_limit_are_typed() {
        let failed = run_native_child("qualification_child_nonzero", 5_000, 4096, None);
        assert_eq!(
            failed.status,
            QualificationStatus::Failed(QualificationFailure::SmokeFailed)
        );
        let timed_out = run_native_child("qualification_child_sleep", 100, 4096, None);
        assert_eq!(
            timed_out.status,
            QualificationStatus::Failed(QualificationFailure::SmokeTimedOut)
        );
        let cancelled = run_native_child(
            "qualification_child_sleep",
            5_000,
            4096,
            Some(Duration::from_millis(100)),
        );
        assert_eq!(
            cancelled.status,
            QualificationStatus::Failed(QualificationFailure::SmokeFailed)
        );
        let limited = run_native_child("qualification_child_output", 5_000, 1024, None);
        assert_eq!(
            limited.status,
            QualificationStatus::Failed(QualificationFailure::OutputLimitExceeded)
        );
    }

    #[test]
    fn structural_inspection_covers_pe_and_macho_targets_independent_of_host() {
        for (target_name, bytes, format, architecture) in [
            (
                "x86_64-pc-windows-msvc",
                pe(CandidateArchitecture::X86_64),
                CandidateFormat::PeCoff,
                CandidateArchitecture::X86_64,
            ),
            (
                "aarch64-pc-windows-msvc",
                pe(CandidateArchitecture::Aarch64),
                CandidateFormat::PeCoff,
                CandidateArchitecture::Aarch64,
            ),
            (
                "x86_64-apple-darwin",
                macho(CandidateArchitecture::X86_64),
                CandidateFormat::MachO,
                CandidateArchitecture::X86_64,
            ),
            (
                "aarch64-apple-darwin",
                macho(CandidateArchitecture::Aarch64),
                CandidateFormat::MachO,
                CandidateArchitecture::Aarch64,
            ),
        ] {
            let root = crate::test_temp_dir("cross-structure");
            let path = candidate_path(&root, target_name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, &bytes).unwrap();
            let candidate = CandidateArtifact {
                target: target_name.into(),
                selector: LogicalOutputSelector::Direct,
                package: "fixture".into(),
                binary: "fixture".into(),
                path,
                size: bytes.len() as u64,
            };
            let evidence = inspect_candidate(&candidate, target_name).unwrap();
            assert_eq!(evidence.format, format);
            assert_eq!(evidence.architecture, architecture);
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn structural_candidate_inventory_supports_direct_bundle_and_archive() {
        let direct = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let bundle = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/codegg-bundle.toml"
        ))
        .unwrap();
        let archive = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/egress-archive.toml"
        ))
        .unwrap();
        let linux = "x86_64-unknown-linux-gnu";
        for (contract, selectors, form) in [
            (
                direct,
                vec![LogicalOutputSelector::Direct],
                PlannedAssetForm::Direct,
            ),
            (
                bundle,
                (0..3)
                    .map(|index| LogicalOutputSelector::BundleEntry { index })
                    .collect(),
                PlannedAssetForm::Bundle,
            ),
            (
                archive,
                vec![
                    LogicalOutputSelector::ArchiveMember {
                        source: "egress".into(),
                    },
                    LogicalOutputSelector::ArchiveMember {
                        source: "bin/egress-helper".into(),
                    },
                ],
                PlannedAssetForm::Archive,
            ),
        ] {
            let root = crate::test_temp_dir("layout-structure");
            let host = local_host();
            let planned = PlannedTarget {
                target: linux.into(),
                policy: TargetPolicy {
                    target: linux.into(),
                    strategy: BuildStrategy::NativeCargo,
                    host_os: host.os,
                    host_arch: host.arch,
                    qualification_host: None,
                    toolchain: ToolchainRequirement {
                        rust: "stable".into(),
                        cargo_zigbuild: None,
                        zig: None,
                    },
                    floor: crate::CompatibilityFloor::None,
                    qualification: Qualification::Structural,
                    support: SupportTier::Required,
                },
                artifact_form: form,
            };
            let plan = ReleasePlan {
                schema_version: 1,
                release_id: "v1".into(),
                source_revision: "abc".into(),
                targets: vec![planned.clone()],
            };
            let mut bindings = Vec::new();
            let mut candidates = Vec::new();
            for (index, selector) in selectors.into_iter().enumerate() {
                let binary = format!("fixture-{index}");
                bindings.push(crate::BuildBinding {
                    selector: selector.clone(),
                    package: "fixture".into(),
                    binary: binary.clone(),
                });
                let path = root.join(linux).join("release").join(&binary);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, elf(CandidateArchitecture::X86_64)).unwrap();
                candidates.push(CandidateArtifact {
                    target: linux.into(),
                    selector,
                    package: "fixture".into(),
                    binary,
                    path,
                    size: 64,
                });
            }
            let builds = BuildBindingsV1 {
                schema_version: 1,
                targets: [(linux.into(), bindings)].into_iter().collect(),
            };
            let quals = QualificationBindingsV1 {
                schema_version: 1,
                targets: [(linux.into(), TargetQualificationBinding::default())]
                    .into_iter()
                    .collect(),
            };
            let attempt = BuildAttempt {
                release_id: plan.release_id.clone(),
                source_revision: plan.source_revision.clone(),
                target: linux.into(),
                strategy: BuildStrategy::NativeCargo,
                tool_summary: "rust stable".into(),
                process: ProcessEvidence {
                    outcome: CommandOutcome::Success,
                    stdout_bytes: 0,
                    stderr_bytes: 0,
                },
                candidates,
            };
            let evidence = qualify_target_for_host(
                &contract,
                &plan,
                &planned,
                &attempt,
                &builds,
                &quals,
                &QualificationRuntime::default(),
                &BuildCancellation::new(),
                host,
            )
            .unwrap();
            assert_eq!(evidence.status, QualificationStatus::Passed);
            assert_eq!(evidence.candidates.len(), builds.targets[linux].len());
            assert!(evidence.validate_for(&plan, &planned, &attempt).is_ok());
            fs::remove_dir_all(root).unwrap();
        }
    }
}
