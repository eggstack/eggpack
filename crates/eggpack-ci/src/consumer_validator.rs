//! The consumer-owned exact-candidate validator: configuration, evidence, and the bounded interpreter run.

use crate::error::{fail, CiError};
use crate::policy::{validate_release_input_path, validate_sha256_hex};
use crate::validate::{safe_metadata, safe_target};
use eggpack_core::LogicalOutputSelector;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Canonical consumer validation evidence file name inside per-target dirs.
pub const CONSUMER_EVIDENCE_FILE: &str = "consumer-evidence.json";

const MAX_VALIDATOR_JSON: usize = 64 * 1024;

pub(crate) const MAX_CONSUMER_EVIDENCE_JSON: usize = 64 * 1024;

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
    /// Observed candidate size in bytes; 0 when no candidate was observed.
    pub candidate_size: u64,
    /// Observed candidate lowercase SHA-256 hex, or `None` when no candidate
    /// was observed.
    ///
    /// An absent observation is recorded as absent. This document is evidence
    /// of what the producer actually saw, so a never-read candidate must never
    /// appear here as a plausible-looking digest.
    pub candidate_sha256: Option<String>,
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
        {
            return Err(fail("invalid or out-of-bounds consumer evidence"));
        }
        // Either a candidate was observed, in which case both halves of its
        // identity are present and well-formed, or neither is. A size without
        // a digest — or a digest without a size — would record an identity
        // that was never observed.
        match (&self.candidate_sha256, self.candidate_size) {
            (Some(digest), size) if size > 0 => {
                validate_sha256_hex(digest)
                    .map_err(|error| fail(&format!("consumer evidence digest: {error}")))?;
            }
            (Some(_), _) => {
                return Err(fail(
                    "consumer evidence digest without an observed candidate size",
                ));
            }
            (None, 0) => {}
            (None, _) => {
                return Err(fail(
                    "consumer evidence candidate size without an observed digest",
                ));
            }
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
    candidate_sha256: Option<String>,
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

/// Retained-output bound for the bounded `--version` interpreter preflight.
const PREFLIGHT_OUTPUT_LIMIT: usize = 8192;

/// Build the bounded command spec for one consumer-validator invocation.
///
/// The argument vector is OS-native so a validated absolute path reaches the
/// interpreter byte-exact, and the explicit environment override carries the
/// caller's PATH so a hermetic interpreter lookup stays hermetic. Everything
/// else — the deadline, the output bounds, the environment allowlist, the
/// process-group termination, and the bounded reader joins — belongs to the
/// shared runner in `eggpack-core`.
fn validator_spec(
    exe: &str,
    path_value: &str,
    request: &ConsumerValidationRequest<'_>,
    args: Vec<std::ffi::OsString>,
) -> eggpack_core::InterpreterCommandSpec {
    let mut env = std::collections::BTreeMap::new();
    env.insert("PATH".to_owned(), path_value.to_owned());
    eggpack_core::InterpreterCommandSpec {
        executable: exe.to_owned(),
        args,
        cwd: request.work_dir.to_path_buf(),
        env,
        timeout: Duration::from_millis(request.validator.timeout_ms),
        stdout_limit: request.validator.stdout_limit,
        stderr_limit: request.validator.stderr_limit,
        expected_stdout: None,
    }
}

/// Execute one consumer validator against the exact candidate.
///
/// Shell-free fixed invocation `Python3 <script> <candidate>` with cleared
/// environment (the shared runner's documented allowlist), explicit working
/// directory, null stdin, bounded timeout/output, process-group termination,
/// and exact candidate identity verification. Returns bounded evidence
/// without script output contents; only caller misuse (invalid config,
/// non-absolute paths, malformed expectations, unusable working directory)
/// returns `CiError`. If Python is unavailable, validation fails with
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
    validate_sha256_hex(request.expected_sha256)
        .map_err(|error| fail(&format!("expected candidate digest: {error}")))?;
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
    // carries the validated linkage. When there is nothing to observe, the
    // evidence records an absent candidate rather than a synthetic one: a
    // never-read candidate must not be published as a digest, because this
    // document is a claim about final bytes.
    let observed = hash_candidate_file(request.candidate_path);
    let candidate_ok = match &observed {
        Ok((size, sha)) => {
            is_regular_nonempty_file(request.candidate_path)
                && *size == request.expected_size
                && *sha == request.expected_sha256
        }
        Err(_) => false,
    };
    let (evidence_size, evidence_sha) = match &observed {
        Ok((size, sha)) => (*size, Some(sha.clone())),
        Err(_) => (0, None),
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
    // F10a: the candidate bytes already match; restore exec bits stripped
    // by artifact transfer before the validator spawns the binary.
    if !ensure_candidate_executable(request.candidate_path) {
        return consumer_evidence_shell(
            request,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch),
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

/// Restore POSIX exec bits on a transferred candidate before spawning a
/// validator against it. Artifact transfer between jobs strips exec bits
/// (upload-artifact documents 644 for all files); the recorded size/digest
/// cover bytes only, so restoring mode cannot change identity. Callers must
/// verify byte identity first and map failure to CandidateMismatch.
fn ensure_candidate_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(meta) = std::fs::symlink_metadata(path) else {
            return false;
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            return false;
        }
        let mut permissions = meta.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        std::fs::set_permissions(path, permissions).is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

/// Digest one candidate file for the consumer-evidence identity.
///
/// The canonical producer file-digest helper owns this. There is no sentinel
/// success value, no empty digest, and no zero-size placeholder: a refusal is
/// a typed error and the caller records an absent candidate.
pub(crate) fn hash_candidate_file(path: &Path) -> Result<(u64, String), eggpack_core::DigestError> {
    eggpack_core::file_digest(path)
}

/// Bounded `--version` preflight for the Python 3 interpreter.
///
/// The probe runs through the shared bounded runner, so it inherits the
/// process-group termination, the bounded reader joins, and the shared
/// environment allowlist. The version banner is matched inside the runner
/// and never crosses the boundary: the caller learns only whether the
/// interpreter identified itself as Python 3.
fn run_interpreter_preflight(
    exe: &str,
    path_value: &str,
    request: &ConsumerValidationRequest<'_>,
) -> bool {
    let mut spec = validator_spec(
        exe,
        path_value,
        request,
        vec![std::ffi::OsString::from("--version")],
    );
    spec.timeout = Duration::from_millis(request.validator.timeout_ms.min(30_000));
    spec.stdout_limit = PREFLIGHT_OUTPUT_LIMIT;
    spec.stderr_limit = PREFLIGHT_OUTPUT_LIMIT;
    spec.expected_stdout = Some("Python 3".to_owned());
    matches!(
        eggpack_core::run_interpreter_bounded(&spec, None),
        Ok(evidence) if evidence.outcome == eggpack_core::CommandOutcome::Success
    )
}

/// Run the consumer validator against the exact candidate.
///
/// The runner outcome maps onto the consumer-validation outcome exactly:
///
/// | Runner outcome | Consumer validation outcome |
/// | --- | --- |
/// | success | `Passed` |
/// | non-zero exit | `Failed(NonZeroExit)` |
/// | deadline exceeded | `Failed(Timeout)` |
/// | output bound exceeded | `Failed(OutputLimit)` |
/// | cancellation observed | `Failed(Cancelled)` |
/// | interpreter not launchable | `Failed(InterpreterUnavailable)` |
fn run_validator_process(
    request: &ConsumerValidationRequest<'_>,
    exe: &str,
    path_value: &str,
) -> ConsumerValidationOutcome {
    let spec = validator_spec(
        exe,
        path_value,
        request,
        vec![
            std::ffi::OsString::from(request.script_path),
            std::ffi::OsString::from(request.candidate_path),
        ],
    );
    match eggpack_core::run_interpreter_bounded(&spec, request.cancelled) {
        Ok(evidence) => match evidence.outcome {
            eggpack_core::CommandOutcome::Success => ConsumerValidationOutcome::Passed,
            eggpack_core::CommandOutcome::TimedOut => {
                ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout)
            }
            eggpack_core::CommandOutcome::Cancelled => {
                ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Cancelled)
            }
            eggpack_core::CommandOutcome::OutputLimitExceeded => {
                ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit)
            }
            eggpack_core::CommandOutcome::Failed(_) => {
                ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit)
            }
        },
        // The interpreter could not be launched at all.
        Err(e) => {
            eprintln!("DIAG-CONSUMER-VALIDATOR: runner error: {e}");
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::InterpreterUnavailable)
        }
    }
}

// ---------------------------------------------------------------------------
// M003d — consumer-owned exact-candidate validator.
// ---------------------------------------------------------------------------
