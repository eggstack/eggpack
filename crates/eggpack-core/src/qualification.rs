//! Bounded target qualification over explicit builder candidates.

use crate::{
    builder::{run_qualification_process, CandidateArtifact, CommandSpec, ProcessEvidence},
    BuildAttempt, BuildCancellation, BuildError, HostArch, HostOs, HostRequirement, PlannedTarget,
    Qualification, ReleasePlan,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_HOOKS: usize = 32;
const MAX_ARGS: usize = 128;
const MAX_ARG_LEN: usize = 4096;
const MAX_ENV: usize = 32;
const MAX_CAPTURE: usize = 256 * 1024;

/// Explicit bounded application-owned smoke command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationHook {
    /// Stable short identifier for this check.
    pub name: String,
    /// Absolute executable path; no shell is invoked.
    pub executable: PathBuf,
    /// Arguments. `{candidate}` and `{target}` are expanded per candidate.
    pub args: Vec<String>,
    /// Explicit environment overrides; values are never copied to evidence.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// Bounded qualification execution settings for one planned target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationConfig {
    /// Arguments passed to native candidates or after the candidate path to an emulator.
    #[serde(default)]
    pub execution_args: Vec<String>,
    /// Absolute emulator executable for `Qualification::Emulated` targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emulator: Option<PathBuf>,
    /// Arguments passed to the emulator before the candidate path.
    #[serde(default)]
    pub emulator_args: Vec<String>,
    /// Application-owned smoke checks run once for each candidate after execution.
    #[serde(default)]
    pub hooks: Vec<QualificationHook>,
    /// Per-process deadline in milliseconds (1..=86,400,000).
    pub timeout_ms: u64,
    /// Retained stdout bound per process (1..=262,144).
    pub stdout_limit: usize,
    /// Retained stderr bound per process (1..=262,144).
    pub stderr_limit: usize,
}

/// Qualification execution mode, distinct from build strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationMode {
    /// Candidate executed directly on a matching target host.
    Native,
    /// Native execution remains pending on another host.
    DeferredNative,
    /// Candidate executed through the explicitly configured emulator.
    Emulated,
    /// Candidate files were inspected without executing them.
    Structural,
}

/// Qualification result state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationStatus {
    /// All configured execution and hooks completed successfully.
    Passed,
    /// Native qualification is pending on the required host.
    Deferred,
    /// A check failed, timed out, was cancelled, or exceeded an output bound.
    Failed,
    /// Structural checks completed without target execution.
    StructuralOnly,
}

/// Exact candidate byte evidence inspected or exercised by qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedCandidate {
    /// Contract logical output selector.
    pub selector: crate::LogicalOutputSelector,
    /// Number of bytes observed immediately before qualification.
    pub size: u64,
    /// SHA-256 of those exact bytes; integrity evidence only.
    pub sha256: String,
}

/// One bounded process result, without captured output or environment values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationProcess {
    /// Hook name, or `candidate_execution` for target execution.
    pub name: String,
    /// Candidate selector associated with this invocation.
    pub selector: crate::LogicalOutputSelector,
    /// Process exit/deadline/cancellation/output-limit classification.
    pub evidence: ProcessEvidence,
}

/// Identity-bound qualification evidence for one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationEvidence {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Release identity from the supplied ReleasePlan.
    pub release_id: String,
    /// Source identity from the supplied ReleasePlan.
    pub source_revision: String,
    /// Canonical planned target triple.
    pub target: String,
    /// Mode that was actually applied.
    pub mode: QualificationMode,
    /// Result classification.
    pub status: QualificationStatus,
    /// Exact candidate bytes observed by the run.
    pub candidates: Vec<QualifiedCandidate>,
    /// Bounded process summaries; output contents are deliberately omitted.
    pub processes: Vec<QualificationProcess>,
}

/// Validate and execute the qualification policy for one M002 build attempt.
pub fn qualify_target(
    plan: &ReleasePlan,
    target: &PlannedTarget,
    attempt: &BuildAttempt,
    config: &QualificationConfig,
    cancellation: &BuildCancellation,
) -> Result<QualificationEvidence, BuildError> {
    if plan.schema_version != 1
        || plan.release_id.is_empty()
        || plan.source_revision.is_empty()
        || !plan.targets.iter().any(|planned| planned == target)
        || attempt.target != target.target
        || attempt.strategy != target.policy.strategy
        || !matches!(attempt.process.outcome, crate::CommandOutcome::Success)
        || attempt.candidates.is_empty()
        || attempt.candidates.len() > 256
    {
        return Err(BuildError(
            "build attempt does not match release plan".into(),
        ));
    }
    validate_config(config)?;
    let mode = match target.policy.qualification {
        Qualification::Native => QualificationMode::Native,
        Qualification::DeferredNative => QualificationMode::DeferredNative,
        Qualification::Emulated => QualificationMode::Emulated,
        Qualification::Structural => QualificationMode::Structural,
    };
    if (mode == QualificationMode::Emulated) != config.emulator.is_some()
        || (mode != QualificationMode::Emulated && !config.emulator_args.is_empty())
        || (matches!(
            mode,
            QualificationMode::DeferredNative | QualificationMode::Structural
        ) && (!config.execution_args.is_empty() || !config.hooks.is_empty()))
    {
        return Err(BuildError(
            "qualification configuration does not match target policy".into(),
        ));
    }
    if mode == QualificationMode::Native && !host_matches(target) {
        return Err(BuildError(
            "native qualification host does not match target".into(),
        ));
    }

    let mut selectors = std::collections::BTreeSet::new();
    let mut candidates = Vec::with_capacity(attempt.candidates.len());
    for candidate in &attempt.candidates {
        if candidate.target != target.target || !selectors.insert(candidate.selector.clone()) {
            return Err(BuildError("candidate target or selector mismatch".into()));
        }
        candidates.push(inspect_candidate(candidate)?);
    }
    if mode != QualificationMode::DeferredNative
        && mode != QualificationMode::Structural
        && !attempt
            .candidates
            .iter()
            .any(|candidate| candidate.size > 0)
    {
        return Err(BuildError(
            "qualification has no executable candidate".into(),
        ));
    }

    let mut processes = Vec::new();
    let mut failed = false;
    if mode == QualificationMode::Native || mode == QualificationMode::Emulated {
        for candidate in &attempt.candidates {
            let (executable, args) = if mode == QualificationMode::Native {
                (candidate.path.clone(), config.execution_args.clone())
            } else {
                let mut args = config.emulator_args.clone();
                args.push(candidate.path.to_string_lossy().into_owned());
                args.extend(config.execution_args.clone());
                (config.emulator.clone().expect("validated emulator"), args)
            };
            let evidence = run_process(executable, args, BTreeMap::new(), config, cancellation)?;
            failed |= evidence.outcome != crate::CommandOutcome::Success;
            processes.push(QualificationProcess {
                name: "candidate_execution".into(),
                selector: candidate.selector.clone(),
                evidence,
            });
            if failed || cancellation.is_cancelled() {
                break;
            }
            for hook in &config.hooks {
                let args = hook
                    .args
                    .iter()
                    .map(|arg| expand_argument(arg, candidate, target))
                    .collect();
                let evidence = run_process(
                    hook.executable.clone(),
                    args,
                    hook.env.clone(),
                    config,
                    cancellation,
                )?;
                failed |= evidence.outcome != crate::CommandOutcome::Success;
                processes.push(QualificationProcess {
                    name: hook.name.clone(),
                    selector: candidate.selector.clone(),
                    evidence,
                });
                if failed || cancellation.is_cancelled() {
                    break;
                }
            }
            if !failed && !cancellation.is_cancelled() {
                for (candidate, before) in attempt.candidates.iter().zip(&candidates) {
                    match inspect_candidate(candidate) {
                        Ok(after) if after.size == before.size && after.sha256 == before.sha256 => {
                        }
                        _ => {
                            failed = true;
                            break;
                        }
                    }
                }
            }
            if failed || cancellation.is_cancelled() {
                break;
            }
        }
    }
    let status = match mode {
        QualificationMode::DeferredNative => QualificationStatus::Deferred,
        QualificationMode::Structural => QualificationStatus::StructuralOnly,
        _ if failed || cancellation.is_cancelled() => QualificationStatus::Failed,
        _ => QualificationStatus::Passed,
    };
    Ok(QualificationEvidence {
        schema_version: 1,
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        target: target.target.clone(),
        mode,
        status,
        candidates,
        processes,
    })
}

fn validate_config(config: &QualificationConfig) -> Result<(), BuildError> {
    if config.timeout_ms == 0
        || config.timeout_ms > 86_400_000
        || config.stdout_limit == 0
        || config.stdout_limit > MAX_CAPTURE
        || config.stderr_limit == 0
        || config.stderr_limit > MAX_CAPTURE
        || config.execution_args.len() > MAX_ARGS
        || config.emulator_args.len() > MAX_ARGS
        || config.hooks.len() > MAX_HOOKS
        || config
            .execution_args
            .iter()
            .chain(config.emulator_args.iter())
            .any(|value| value.len() > MAX_ARG_LEN || value.contains('\0'))
    {
        return Err(BuildError(
            "qualification process bounds are invalid".into(),
        ));
    }
    for hook in &config.hooks {
        if hook.name.is_empty()
            || hook.name.len() > 64
            || !hook
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            || !hook.executable.is_absolute()
            || hook.args.len() > MAX_ARGS
            || hook.args.iter().any(|arg| {
                arg.len() > MAX_ARG_LEN || arg.contains('\0') || invalid_template_argument(arg)
            })
            || hook.env.len() > MAX_ENV
            || hook.env.iter().any(|(key, value)| {
                key.is_empty()
                    || key.len() > 128
                    || !key
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    || value.len() > MAX_ARG_LEN
                    || value.contains('\0')
            })
        {
            return Err(BuildError(
                "qualification hook configuration is invalid".into(),
            ));
        }
    }
    if config
        .emulator
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
    {
        return Err(BuildError("emulator path must be absolute".into()));
    }
    Ok(())
}

fn run_process(
    executable: PathBuf,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    config: &QualificationConfig,
    cancellation: &BuildCancellation,
) -> Result<ProcessEvidence, BuildError> {
    let original_metadata = fs::symlink_metadata(&executable)
        .map_err(|_| BuildError("qualification executable unavailable".into()))?;
    if original_metadata.file_type().is_symlink() || !original_metadata.is_file() {
        return Err(BuildError(
            "qualification executable is not a regular file".into(),
        ));
    }
    let executable = fs::canonicalize(&executable)
        .map_err(|_| BuildError("qualification executable unavailable".into()))?;
    let metadata = fs::symlink_metadata(&executable)
        .map_err(|_| BuildError("qualification executable unavailable".into()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(BuildError(
            "qualification executable is not a regular file".into(),
        ));
    }
    let cwd = std::env::current_dir()
        .map_err(|_| BuildError("qualification working directory unavailable".into()))?;
    run_qualification_process(
        &CommandSpec {
            executable: executable.to_string_lossy().into_owned(),
            args,
            cwd,
            env,
            timeout: Duration::from_millis(config.timeout_ms),
            stdout_limit: config.stdout_limit,
            stderr_limit: config.stderr_limit,
            expected_stdout: None,
        },
        cancellation,
    )
    .map_err(|_| BuildError("qualification process execution failed".into()))
}

fn inspect_candidate(candidate: &CandidateArtifact) -> Result<QualifiedCandidate, BuildError> {
    let metadata = fs::symlink_metadata(&candidate.path)
        .map_err(|_| BuildError("candidate unavailable for qualification".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        return Err(BuildError(
            "candidate is not a non-empty regular file".into(),
        ));
    }
    let canonical = fs::canonicalize(&candidate.path)
        .map_err(|_| BuildError("candidate unavailable for qualification".into()))?;
    let target_dir = candidate
        .path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| BuildError("candidate location is invalid".into()))?;
    let release_dir = candidate
        .path
        .parent()
        .ok_or_else(|| BuildError("candidate location is invalid".into()))?;
    for directory in [target_dir, release_dir] {
        let directory_metadata = fs::symlink_metadata(directory)
            .map_err(|_| BuildError("candidate directory unavailable".into()))?;
        if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
            return Err(BuildError("candidate directory is not private".into()));
        }
    }
    let canonical_root = fs::canonicalize(target_dir)
        .map_err(|_| BuildError("candidate root unavailable for qualification".into()))?;
    if !canonical.starts_with(&canonical_root) || metadata.len() != candidate.size {
        return Err(BuildError(
            "candidate changed or escaped its private target".into(),
        ));
    }
    let mut file =
        fs::File::open(&canonical).map_err(|_| BuildError("candidate read failed".into()))?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| BuildError("candidate read failed".into()))?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .ok_or_else(|| BuildError("candidate size overflow".into()))?;
        hasher.update(&buffer[..read]);
    }
    let after = fs::symlink_metadata(&candidate.path)
        .map_err(|_| BuildError("candidate changed while being inspected".into()))?;
    if bytes != metadata.len()
        || after.file_type().is_symlink()
        || !after.is_file()
        || after.len() != metadata.len()
        || fs::canonicalize(&candidate.path).ok().as_ref() != Some(&canonical)
    {
        return Err(BuildError("candidate changed while being inspected".into()));
    }
    Ok(QualifiedCandidate {
        selector: candidate.selector.clone(),
        size: bytes,
        sha256: hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    })
}

fn expand_argument(
    argument: &str,
    candidate: &CandidateArtifact,
    target: &PlannedTarget,
) -> String {
    argument
        .replace("{candidate}", &candidate.path.to_string_lossy())
        .replace("{target}", &target.target)
}

fn invalid_template_argument(argument: &str) -> bool {
    let without_known = argument.replace("{candidate}", "").replace("{target}", "");
    without_known.contains('{') || without_known.contains('}')
}

fn host_matches(target: &PlannedTarget) -> bool {
    let host = target.policy.qualification_host.unwrap_or(HostRequirement {
        os: target.policy.host_os,
        arch: target.policy.host_arch,
    });
    let os_matches = matches!(
        (host.os, std::env::consts::OS),
        (HostOs::Linux, "linux") | (HostOs::Macos, "macos") | (HostOs::Windows, "windows")
    );
    let arch_matches = matches!(
        (host.arch, std::env::consts::ARCH),
        (HostArch::X86_64, "x86_64") | (HostArch::Aarch64, "aarch64") | (HostArch::Armv7, "arm")
    );
    let target_os_matches = match host.os {
        HostOs::Linux => target.target.contains("-linux-"),
        HostOs::Macos => target.target.ends_with("-apple-darwin"),
        HostOs::Windows => target.target.contains("-windows-"),
    };
    let target_arch_matches = match host.arch {
        HostArch::X86_64 => target.target.starts_with("x86_64-"),
        HostArch::Aarch64 => target.target.starts_with("aarch64-"),
        HostArch::Armv7 => target.target.starts_with("armv7-") || target.target.starts_with("arm-"),
    };
    os_matches && arch_matches && target_os_matches && target_arch_matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BuildAttempt, BuildStrategy, CandidateArtifact, CommandOutcome, HostRequirement,
        LogicalOutputSelector, PlannedAssetForm, ProcessEvidence, SupportTier, TargetPolicy,
        ToolchainRequirement,
    };

    fn host() -> (HostOs, HostArch, &'static str) {
        #[cfg(target_os = "linux")]
        let os = HostOs::Linux;
        #[cfg(target_os = "macos")]
        let os = HostOs::Macos;
        #[cfg(target_os = "windows")]
        let os = HostOs::Windows;
        #[cfg(target_arch = "x86_64")]
        let (arch, triple) = (
            HostArch::X86_64,
            match os {
                HostOs::Linux => "x86_64-unknown-linux-gnu",
                HostOs::Macos => "x86_64-apple-darwin",
                HostOs::Windows => "x86_64-pc-windows-msvc",
            },
        );
        #[cfg(target_arch = "aarch64")]
        let (arch, triple) = (
            HostArch::Aarch64,
            match os {
                HostOs::Linux => "aarch64-unknown-linux-gnu",
                HostOs::Macos => "aarch64-apple-darwin",
                HostOs::Windows => "aarch64-pc-windows-msvc",
            },
        );
        #[cfg(target_arch = "arm")]
        let (arch, triple) = (HostArch::Armv7, "armv7-unknown-linux-gnueabihf");
        (os, arch, triple)
    }

    fn fixture(
        qualification: Qualification,
    ) -> (ReleasePlan, PlannedTarget, BuildAttempt, PathBuf) {
        let root = crate::test_temp_dir("qualification");
        let target_dir = root.join("invocation-target");
        let release_dir = target_dir.join("release");
        fs::create_dir_all(&release_dir).unwrap();
        #[cfg(windows)]
        let candidate_path = release_dir.join("fixture.exe");
        #[cfg(not(windows))]
        let candidate_path = release_dir.join("fixture");
        fs::write(&candidate_path, b"candidate bytes").unwrap();
        let (host_os, host_arch, triple) = host();
        let target = PlannedTarget {
            target: triple.into(),
            policy: TargetPolicy {
                target: triple.into(),
                strategy: BuildStrategy::NativeCargo,
                host_os,
                host_arch,
                qualification_host: Some(HostRequirement {
                    os: host_os,
                    arch: host_arch,
                }),
                toolchain: ToolchainRequirement {
                    rust: "stable".into(),
                    cargo_zigbuild: None,
                },
                floor: crate::CompatibilityFloor::None,
                qualification,
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
        let attempt = BuildAttempt {
            target: triple.into(),
            strategy: BuildStrategy::NativeCargo,
            tool_summary: "rust stable".into(),
            process: ProcessEvidence {
                outcome: CommandOutcome::Success,
                stdout_bytes: 0,
                stderr_bytes: 0,
            },
            candidates: vec![CandidateArtifact {
                target: triple.into(),
                selector: LogicalOutputSelector::Direct,
                package: "fixture".into(),
                binary: "fixture".into(),
                path: candidate_path,
                size: b"candidate bytes".len() as u64,
            }],
        };
        (plan, target, attempt, root)
    }

    fn config() -> QualificationConfig {
        QualificationConfig {
            execution_args: vec![],
            emulator: None,
            emulator_args: vec![],
            hooks: vec![],
            timeout_ms: 1000,
            stdout_limit: 1024,
            stderr_limit: 1024,
        }
    }

    #[test]
    fn deferred_native_is_pending_and_binds_candidate_digest() {
        let (plan, target, attempt, root) = fixture(Qualification::DeferredNative);
        let evidence = qualify_target(
            &plan,
            &target,
            &attempt,
            &config(),
            &BuildCancellation::new(),
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::Deferred);
        assert_eq!(evidence.candidates[0].size, b"candidate bytes".len() as u64);
        assert_eq!(evidence.candidates[0].sha256.len(), 64);
        let json = serde_json::to_string(&evidence).unwrap();
        assert!(!json.contains(&attempt.candidates[0].path.to_string_lossy().to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn structural_result_cannot_run_product_hooks() {
        let (plan, target, attempt, root) = fixture(Qualification::Structural);
        let mut invalid = config();
        invalid.hooks.push(QualificationHook {
            name: "smoke".into(),
            executable: PathBuf::from("/bin/true"),
            args: vec![],
            env: BTreeMap::new(),
        });
        assert!(qualify_target(
            &plan,
            &target,
            &attempt,
            &invalid,
            &BuildCancellation::new()
        )
        .is_err());
        let evidence = qualify_target(
            &plan,
            &target,
            &attempt,
            &config(),
            &BuildCancellation::new(),
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::StructuralOnly);
        assert!(evidence.processes.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_claim_fails_on_mismatched_execution_host() {
        let (plan, mut target, attempt, root) = fixture(Qualification::Native);
        target.policy.qualification_host = Some(HostRequirement {
            os: match host().0 {
                HostOs::Linux => HostOs::Macos,
                _ => HostOs::Linux,
            },
            arch: host().1,
        });
        let plan = ReleasePlan {
            targets: vec![target.clone()],
            ..plan
        };
        assert!(qualify_target(
            &plan,
            &target,
            &attempt,
            &config(),
            &BuildCancellation::new()
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn qualification_child_target() {}

    #[test]
    #[ignore]
    fn qualification_child_sleeps() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    fn native_execution_records_success_and_runs_bounded_hook() {
        let (plan, target, mut attempt, root) = fixture(Qualification::Native);
        fs::copy(
            std::env::current_exe().unwrap(),
            &attempt.candidates[0].path,
        )
        .unwrap();
        attempt.candidates[0].size = fs::metadata(&attempt.candidates[0].path).unwrap().len();
        let mut settings = config();
        settings.execution_args = vec![
            "--exact".into(),
            "qualification::tests::qualification_child_target".into(),
            "--nocapture".into(),
        ];
        #[cfg(unix)]
        settings.hooks.push(QualificationHook {
            name: "candidate-path-argument".into(),
            executable: PathBuf::from("/bin/true"),
            args: vec!["{candidate}".into(), "{target}".into()],
            env: BTreeMap::new(),
        });
        let evidence = qualify_target(
            &plan,
            &target,
            &attempt,
            &settings,
            &BuildCancellation::new(),
        )
        .unwrap();
        assert_eq!(evidence.status, QualificationStatus::Passed);
        assert_eq!(evidence.processes[0].name, "candidate_execution");
        #[cfg(unix)]
        assert_eq!(evidence.processes[1].name, "candidate-path-argument");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn emulated_execution_is_separately_classified() {
        let (plan, mut target, mut attempt, root) = fixture(Qualification::Emulated);
        fs::copy(
            std::env::current_exe().unwrap(),
            &attempt.candidates[0].path,
        )
        .unwrap();
        attempt.candidates[0].size = fs::metadata(&attempt.candidates[0].path).unwrap().len();
        target.policy.qualification = Qualification::Emulated;
        let plan = ReleasePlan {
            targets: vec![target.clone()],
            ..plan
        };
        let mut settings = config();
        settings.emulator = Some(PathBuf::from("/bin/true"));
        let evidence = qualify_target(
            &plan,
            &target,
            &attempt,
            &settings,
            &BuildCancellation::new(),
        )
        .unwrap();
        assert_eq!(evidence.mode, QualificationMode::Emulated);
        assert_eq!(evidence.status, QualificationStatus::Passed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_execution_timeout_and_cancellation_are_evidenced() {
        use std::thread;

        for cancel in [false, true] {
            let (plan, target, mut attempt, root) = fixture(Qualification::Native);
            fs::copy(
                std::env::current_exe().unwrap(),
                &attempt.candidates[0].path,
            )
            .unwrap();
            attempt.candidates[0].size = fs::metadata(&attempt.candidates[0].path).unwrap().len();
            let mut settings = config();
            settings.execution_args = vec![
                "--exact".into(),
                "qualification::tests::qualification_child_sleeps".into(),
                "--ignored".into(),
                "--nocapture".into(),
            ];
            settings.timeout_ms = if cancel { 5_000 } else { 100 };
            let cancellation = BuildCancellation::new();
            let cancel_thread = cancel.then(|| {
                let cancellation = cancellation.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                    cancellation.cancel();
                })
            });
            let evidence =
                qualify_target(&plan, &target, &attempt, &settings, &cancellation).unwrap();
            assert_eq!(evidence.status, QualificationStatus::Failed);
            assert_eq!(
                evidence.processes[0].evidence.outcome,
                if cancel {
                    crate::CommandOutcome::Cancelled
                } else {
                    crate::CommandOutcome::TimedOut
                }
            );
            if let Some(thread) = cancel_thread {
                thread.join().unwrap();
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}
