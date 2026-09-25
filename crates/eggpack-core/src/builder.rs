//! Explicit Cargo build bindings, command descriptions, and bounded process execution.

use crate::{BuildStrategy, CompatibilityFloor, ReleasePlan};
use command_group::CommandGroup;
use eggpack_contract::{DistributionContract, ExpandedAssets};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const MAX_OUTPUT: usize = 256 * 1024;

/// Contract logical location for one Cargo-produced executable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogicalOutputSelector {
    /// Direct artifact executable.
    Direct,
    /// Bundle entry by its contract order index.
    BundleEntry {
        /// Zero-based contract entry index.
        index: usize,
    },
    /// Archive member by its exact contract source.
    ArchiveMember {
        /// Contract archive source.
        source: String,
    },
}

/// A Cargo package and binary target explicitly bound to a logical contract slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildBinding {
    /// Logical contract output slot.
    pub selector: LogicalOutputSelector,
    /// Cargo package name.
    pub package: String,
    /// Cargo binary target name.
    pub binary: String,
}

/// Strict producer-side BuildBindings schema version 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildBindingsV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Bindings for canonical target triple.
    pub targets: BTreeMap<String, Vec<BuildBinding>>,
}

/// Failure from producer build planning or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError(pub(crate) String);
impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for BuildError {}
fn build_err(s: &str) -> BuildError {
    BuildError(s.to_owned())
}

/// Explicit shell-free process command. Executable and argv are separate OS arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// Executable path or tool name.
    pub executable: String,
    /// Ordered arguments.
    pub args: Vec<String>,
    /// Working directory.
    pub cwd: PathBuf,
    /// Explicit environment overrides only.
    pub env: BTreeMap<String, String>,
    /// Maximum wall time.
    pub timeout: Duration,
    /// Maximum retained stdout bytes.
    pub stdout_limit: usize,
    /// Maximum retained stderr bytes.
    pub stderr_limit: usize,
    /// Optional tool version substring checked internally and never returned.
    pub expected_stdout: Option<String>,
}

/// Exact source command associated with a logical output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundCommand {
    /// Output selector.
    pub selector: LogicalOutputSelector,
    /// Cargo package.
    pub package: String,
    /// Cargo binary target.
    pub binary: String,
    /// Process description.
    pub command: CommandSpec,
}

/// Command exit classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandOutcome {
    /// Process exited with success.
    Success,
    /// Process exited nonzero.
    Failed(i32),
    /// Deadline exceeded; process group cleanup completed.
    TimedOut,
    /// Caller cancelled execution; process-group cleanup completed.
    Cancelled,
    /// Output exceeded a configured bound.
    OutputLimitExceeded,
}

/// Cloneable caller cancellation flag for bounded builder work.
#[derive(Debug, Clone, Default)]
pub struct BuildCancellation(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl BuildCancellation {
    /// Create an active cancellation flag.
    pub fn new() -> Self {
        Self::default()
    }
    /// Request cancellation; an active child process group is killed and waited for.
    pub fn cancel(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Bounded process evidence with no environment dump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEvidence {
    /// Outcome classification.
    pub outcome: CommandOutcome,
    /// Number of retained stdout bytes.
    pub stdout_bytes: usize,
    /// Number of retained stderr bytes.
    pub stderr_bytes: usize,
}

/// Candidate bytes produced for a declared logical output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateArtifact {
    /// Canonical target triple.
    pub target: String,
    /// Logical output slot.
    pub selector: LogicalOutputSelector,
    /// Explicit source package.
    pub package: String,
    /// Explicit binary target.
    pub binary: String,
    /// Private candidate path.
    pub path: PathBuf,
    /// Candidate size in bytes.
    pub size: u64,
}

/// Result of a single bounded build attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildAttempt {
    /// Canonical target triple.
    pub target: String,
    /// Selected strategy.
    pub strategy: BuildStrategy,
    /// Tool summary, without environment values.
    pub tool_summary: String,
    /// Bounded process evidence.
    pub process: ProcessEvidence,
    /// Validated candidate outputs.
    pub candidates: Vec<CandidateArtifact>,
}

impl BuildBindingsV1 {
    /// Strictly parse TOML bindings.
    pub fn from_toml(text: &str) -> Result<Self, BuildError> {
        let value: Self =
            toml::from_str(text).map_err(|_| build_err("invalid BuildBindings TOML"))?;
        value.validate_shape()?;
        Ok(value)
    }
    fn validate_shape(&self) -> Result<(), BuildError> {
        if self.schema_version != 1 || self.targets.is_empty() || self.targets.len() > 256 {
            return Err(build_err("invalid BuildBindings version or target count"));
        }
        for (target, bindings) in &self.targets {
            if target.is_empty() || bindings.is_empty() || bindings.len() > 256 {
                return Err(build_err("invalid BuildBindings target or binding count"));
            }
            let mut slots = BTreeSet::new();
            for b in bindings {
                if !slots.insert(&b.selector)
                    || !valid_identifier(&b.package)
                    || !valid_identifier(&b.binary)
                {
                    return Err(build_err("duplicate slot or invalid Cargo identifier"));
                }
                if let LogicalOutputSelector::ArchiveMember { source } = &b.selector {
                    if source.is_empty()
                        || source.len() > 255
                        || source.starts_with('/')
                        || source
                            .split('/')
                            .any(|p| p.is_empty() || p == "." || p == "..")
                    {
                        return Err(build_err("invalid archive member source"));
                    }
                }
            }
        }
        Ok(())
    }
    /// Validate that each plan target's bindings exactly cover Cargo executable contract slots.
    pub fn validate_for(
        &self,
        contract: &DistributionContract,
        plan: &ReleasePlan,
    ) -> Result<(), BuildError> {
        self.validate_shape()?;
        for target in &plan.targets {
            let bindings = self
                .targets
                .get(&target.target)
                .ok_or_else(|| build_err("missing target bindings"))?;
            let expanded = contract
                .expand(&target.target, &plan.release_id)
                .map_err(|_| build_err("contract target expansion failed"))?;
            let expected: BTreeSet<LogicalOutputSelector> = match expanded.assets {
                ExpandedAssets::Direct(_) => [LogicalOutputSelector::Direct].into_iter().collect(),
                ExpandedAssets::Bundle(b) => (0..b.entries.len())
                    .map(|index| LogicalOutputSelector::BundleEntry { index })
                    .collect(),
                ExpandedAssets::Archive(a) => a
                    .members
                    .into_iter()
                    .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
                    .collect(),
            };
            let actual: BTreeSet<_> = bindings.iter().map(|b| b.selector.clone()).collect();
            if actual != expected {
                return Err(build_err(
                    "bindings do not exactly cover contract Cargo slots",
                ));
            }
        }
        if self
            .targets
            .keys()
            .any(|k| !plan.targets.iter().any(|t| &t.target == k))
        {
            return Err(build_err("bindings include target outside ReleasePlan"));
        }
        Ok(())
    }
}
fn valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

/// Construct the only supported Cargo command for a planned logical output.
pub fn cargo_command(
    target: &crate::PlannedTarget,
    binding: &BuildBinding,
    cwd: &Path,
    target_dir: &Path,
    timeout: Duration,
) -> Result<CommandSpec, BuildError> {
    if !valid_identifier(&binding.package) || !valid_identifier(&binding.binary) {
        return Err(build_err("invalid Cargo identifier"));
    }
    let rustup = target.policy.toolchain.rust.as_str();
    let mut args = match target.policy.strategy {
        BuildStrategy::NativeCargo => vec![
            "+".to_owned() + rustup,
            "build".into(),
            "--release".into(),
            "--locked".into(),
            "--target".into(),
            target.target.clone(),
        ],
        BuildStrategy::CargoZigbuild => vec![
            "+".to_owned() + rustup,
            "zigbuild".into(),
            "--release".into(),
            "--locked".into(),
            "--target".into(),
            target.target.clone(),
        ],
    };
    if target.policy.strategy == BuildStrategy::CargoZigbuild {
        if let CompatibilityFloor::Glibc { major, minor } = target.policy.floor {
            args[5] = format!("{}.{major}.{minor}", target.target);
        } else if matches!(target.policy.floor, CompatibilityFloor::Macos { .. }) {
            return Err(build_err("macOS floor cannot use cargo-zigbuild"));
        }
    }
    args.extend([
        "--package".into(),
        binding.package.clone(),
        "--bin".into(),
        binding.binary.clone(),
    ]);
    let mut env = BTreeMap::new();
    env.insert(
        "CARGO_TARGET_DIR".into(),
        target_dir.to_string_lossy().into_owned(),
    );
    Ok(CommandSpec {
        executable: "cargo".into(),
        args,
        cwd: cwd.to_path_buf(),
        env,
        timeout,
        stdout_limit: MAX_OUTPUT,
        stderr_limit: MAX_OUTPUT,
        expected_stdout: None,
    })
}

/// Run a direct child process group with a deadline and bounded retained output.
/// Run a process with a deadline and caller cancellation, cleaning up its process group.
pub fn run_bounded_cancellable(
    spec: &CommandSpec,
    cancellation: &BuildCancellation,
) -> Result<ProcessEvidence, BuildError> {
    run_bounded_inner(spec, Some(cancellation), false)
}

pub(crate) fn run_qualification_process(
    spec: &CommandSpec,
    cancellation: &BuildCancellation,
) -> Result<ProcessEvidence, BuildError> {
    run_bounded_inner(spec, Some(cancellation), true)
}

fn run_bounded_inner(
    spec: &CommandSpec,
    cancellation: Option<&BuildCancellation>,
    absolute_executable: bool,
) -> Result<ProcessEvidence, BuildError> {
    if !(matches!(spec.executable.as_str(), "cargo" | "rustc" | "zig")
        || (absolute_executable && Path::new(&spec.executable).is_absolute()))
        || spec.timeout.is_zero()
        || spec.timeout > Duration::from_secs(86_400)
        || spec.stdout_limit == 0
        || spec.stderr_limit == 0
        || spec.stdout_limit > MAX_OUTPUT
        || spec.stderr_limit > MAX_OUTPUT
    {
        return Err(build_err("invalid process bounds or executable"));
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let environment = if absolute_executable {
        ["PATH", "SystemRoot", "WINDIR", "TEMP", "TMP"].as_slice()
    } else {
        [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SystemRoot",
            "WINDIR",
            "TEMP",
            "TMP",
            "CARGO_HOME",
            "RUSTUP_HOME",
            "LIB",
            "LIBPATH",
            "INCLUDE",
            "VCToolsInstallDir",
            "VisualStudioVersion",
            "VCINSTALLDIR",
            "WindowsSdkDir",
            "WindowsSDKVersion",
            "UniversalCRTSdkDir",
            "UCRTVersion",
        ]
        .as_slice()
    };
    for key in environment {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    #[cfg(windows)]
    if let Some(vc_tools) = std::env::var_os("VCToolsInstallDir") {
        let preferred_linker = PathBuf::from(vc_tools)
            .join("bin")
            .join("Hostx64")
            .join("x64");
        if let Some(path) = std::env::var_os("PATH") {
            let path = std::iter::once(preferred_linker).chain(std::env::split_paths(&path));
            if let Ok(path) = std::env::join_paths(path) {
                command.env("PATH", path);
            }
        }
    }
    command.envs(&spec.env);
    let mut child = command
        .group_spawn()
        .map_err(|_| build_err("process spawn failed"))?;
    let stdout = child
        .inner()
        .stdout
        .take()
        .ok_or_else(|| build_err("stdout pipe unavailable"))?;
    let stderr = child
        .inner()
        .stderr
        .take()
        .ok_or_else(|| build_err("stderr pipe unavailable"))?;
    let out = Arc::new(Mutex::new((Vec::new(), false)));
    let err = Arc::new(Mutex::new((Vec::new(), false)));
    let out2 = out.clone();
    let out_limit = spec.stdout_limit;
    let t1 = thread::spawn(move || drain(stdout, out2, out_limit));
    let err2 = err.clone();
    let err_limit = spec.stderr_limit;
    let t2 = thread::spawn(move || drain(stderr, err2, err_limit));
    let until = Instant::now() + spec.timeout;
    let (timed_out, cancelled) = loop {
        if cancellation.is_some_and(BuildCancellation::is_cancelled) {
            child
                .kill()
                .map_err(|_| build_err("cancelled process-group termination failed"))?;
            child
                .wait()
                .map_err(|_| build_err("cancelled process-group wait failed"))?;
            break (false, true);
        }
        if Instant::now() >= until {
            child
                .kill()
                .map_err(|_| build_err("process-group termination failed"))?;
            child
                .wait()
                .map_err(|_| build_err("process-group wait failed"))?;
            break (true, false);
        }
        if child
            .try_wait()
            .map_err(|_| build_err("process wait failed"))?
            .is_some()
        {
            break (false, false);
        }
        thread::sleep(Duration::from_millis(20));
    };
    t1.join().map_err(|_| build_err("stdout reader failed"))?;
    t2.join().map_err(|_| build_err("stderr reader failed"))?;
    let status = child
        .try_wait()
        .map_err(|_| build_err("process wait failed"))?
        .ok_or_else(|| build_err("process cleanup incomplete"))?;
    let (stdout, out_over) = out
        .lock()
        .map_err(|_| build_err("stdout state unavailable"))?
        .clone();
    let (stderr, err_over) = err
        .lock()
        .map_err(|_| build_err("stderr state unavailable"))?
        .clone();
    let expected_missing = spec
        .expected_stdout
        .as_ref()
        .is_some_and(|expected| !String::from_utf8_lossy(&stdout).contains(expected));
    let outcome = if timed_out {
        CommandOutcome::TimedOut
    } else if cancelled {
        CommandOutcome::Cancelled
    } else if out_over || err_over {
        CommandOutcome::OutputLimitExceeded
    } else if expected_missing {
        CommandOutcome::Failed(-1)
    } else if status.success() {
        CommandOutcome::Success
    } else {
        CommandOutcome::Failed(status.code().unwrap_or(-1))
    };
    Ok(ProcessEvidence {
        outcome,
        stdout_bytes: stdout.len(),
        stderr_bytes: stderr.len(),
    })
}
fn drain<R: Read>(mut reader: R, capture: Arc<Mutex<(Vec<u8>, bool)>>, limit: usize) {
    let mut buffer = [0; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if let Ok(mut state) = capture.lock() {
                    let keep = n.min(limit.saturating_sub(state.0.len()));
                    state.0.extend_from_slice(&buffer[..keep]);
                    if keep < n {
                        state.1 = true;
                    }
                }
            }
        }
    }
}

/// Confirm configured Rust/cargo-zigbuild versions before build execution.
fn preflight(
    target: &crate::PlannedTarget,
    cwd: &Path,
    cancellation: &BuildCancellation,
) -> Result<(), BuildError> {
    let rust = run_bounded_cancellable(
        &CommandSpec {
            executable: "rustc".into(),
            args: vec![
                "+".to_owned() + &target.policy.toolchain.rust,
                "--version".into(),
            ],
            cwd: cwd.into(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(10),
            stdout_limit: 1024,
            stderr_limit: 1024,
            expected_stdout: None,
        },
        cancellation,
    )?;
    if rust.outcome != CommandOutcome::Success {
        return Err(build_err("configured Rust toolchain preflight failed"));
    }
    let cargo = run_bounded_cancellable(
        &CommandSpec {
            executable: "cargo".into(),
            args: vec![
                "+".to_owned() + &target.policy.toolchain.rust,
                "--version".into(),
            ],
            cwd: cwd.into(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(10),
            stdout_limit: 1024,
            stderr_limit: 1024,
            expected_stdout: None,
        },
        cancellation,
    )?;
    if cargo.outcome != CommandOutcome::Success {
        return Err(build_err("configured Cargo toolchain preflight failed"));
    }
    if target.policy.strategy == BuildStrategy::CargoZigbuild {
        let expected = target
            .policy
            .toolchain
            .cargo_zigbuild
            .as_deref()
            .ok_or_else(|| build_err("cargo-zigbuild version missing"))?;
        let actual = run_bounded_cancellable(
            &CommandSpec {
                executable: "cargo".into(),
                args: vec!["zigbuild".into(), "--version".into()],
                cwd: cwd.into(),
                env: BTreeMap::new(),
                timeout: Duration::from_secs(10),
                stdout_limit: 1024,
                stderr_limit: 1024,
                expected_stdout: Some(format!("cargo-zigbuild {expected}")),
            },
            cancellation,
        )?;
        if actual.outcome != CommandOutcome::Success {
            return Err(build_err("cargo-zigbuild version mismatch"));
        }
        let zig = run_bounded_cancellable(
            &CommandSpec {
                executable: "zig".into(),
                args: vec!["version".into()],
                cwd: cwd.into(),
                env: BTreeMap::new(),
                timeout: Duration::from_secs(10),
                stdout_limit: 1024,
                stderr_limit: 1024,
                expected_stdout: None,
            },
            cancellation,
        )?;
        if zig.outcome != CommandOutcome::Success {
            return Err(build_err("Zig preflight failed"));
        }
    }
    Ok(())
}

/// Execute one target using explicit bindings, private target storage, and exact output discovery.
pub fn execute_target(
    plan: &ReleasePlan,
    target: &crate::PlannedTarget,
    bindings: &[BuildBinding],
    repository_root: &Path,
    work_root: &Path,
    invocation: &str,
) -> Result<BuildAttempt, BuildError> {
    execute_target_cancellable(
        plan,
        target,
        bindings,
        repository_root,
        work_root,
        invocation,
        &BuildCancellation::new(),
    )
}

/// Execute a target with explicit caller cancellation and bounded cleanup.
pub fn execute_target_cancellable(
    plan: &ReleasePlan,
    target: &crate::PlannedTarget,
    bindings: &[BuildBinding],
    repository_root: &Path,
    work_root: &Path,
    invocation: &str,
    cancellation: &BuildCancellation,
) -> Result<BuildAttempt, BuildError> {
    if !plan.targets.iter().any(|planned| planned == target)
        || plan.schema_version != 1
        || plan.release_id.is_empty()
        || plan.source_revision.is_empty()
    {
        return Err(build_err(
            "target is not part of the supplied ReleasePlan identity",
        ));
    }
    if !repository_root.is_absolute() {
        return Err(build_err("repository root must be absolute"));
    }
    let repository =
        fs::canonicalize(repository_root).map_err(|_| build_err("repository root unavailable"))?;
    preflight(target, &repository, cancellation)?;
    let target_dir = private_target_dir(work_root, invocation, &target.target)?;
    fs::create_dir(&target_dir)
        .map_err(|_| build_err("private Cargo target directory creation failed"))?;
    let mut candidates = Vec::new();
    let mut last = ProcessEvidence {
        outcome: CommandOutcome::Success,
        stdout_bytes: 0,
        stderr_bytes: 0,
    };
    for binding in bindings {
        let spec = cargo_command(
            target,
            binding,
            &repository,
            &target_dir,
            Duration::from_secs(1800),
        )?;
        let evidence = run_bounded_cancellable(&spec, cancellation)?;
        if evidence.outcome != CommandOutcome::Success {
            return Ok(BuildAttempt {
                target: target.target.clone(),
                strategy: target.policy.strategy,
                tool_summary: format!("rust {}", target.policy.toolchain.rust),
                process: evidence,
                candidates: Vec::new(),
            });
        }
        let (path, size) = discover_candidate(&target_dir, &target.target, &binding.binary)?;
        candidates.push(CandidateArtifact {
            target: target.target.clone(),
            selector: binding.selector.clone(),
            package: binding.package.clone(),
            binary: binding.binary.clone(),
            path,
            size,
        });
        last = evidence;
    }
    Ok(BuildAttempt {
        target: target.target.clone(),
        strategy: target.policy.strategy,
        tool_summary: format!(
            "rust {}{}",
            target.policy.toolchain.rust,
            target
                .policy
                .toolchain
                .cargo_zigbuild
                .as_ref()
                .map(|v| format!(" cargo-zigbuild {v}"))
                .unwrap_or_default()
        ),
        process: last,
        candidates,
    })
}

/// Establish a caller-owned absolute work root and a unique target directory.
pub fn private_target_dir(
    work_root: &Path,
    invocation: &str,
    target: &str,
) -> Result<PathBuf, BuildError> {
    if !work_root.is_absolute()
        || invocation.is_empty()
        || invocation.len() > 128
        || target.is_empty()
    {
        return Err(build_err("invalid private work root identity"));
    }
    if fs::symlink_metadata(work_root)
        .map_err(|_| build_err("work root must pre-exist"))?
        .file_type()
        .is_symlink()
    {
        return Err(build_err("symlink work root rejected"));
    }
    let canonical =
        fs::canonicalize(work_root).map_err(|_| build_err("work root canonicalization failed"))?;
    let name = format!("{}-{}", safe_component(invocation), safe_component(target));
    let path = canonical.join(name);
    match fs::create_dir(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(build_err("private build directory already exists"))
        }
        Err(_) => return Err(build_err("private build directory creation failed")),
    }
    let check = fs::canonicalize(&path)
        .map_err(|_| build_err("private build path canonicalization failed"))?;
    if !check.starts_with(&canonical) {
        return Err(build_err("private build path escapes work root"));
    }
    let mut metadata = fs::File::create(path.join(".eggpack-owner"))
        .map_err(|_| build_err("ownership marker creation failed"))?;
    metadata
        .write_all(format!("1\n{}\n{}\n", invocation, target).as_bytes())
        .map_err(|_| build_err("ownership marker write failed"))?;
    Ok(path.join("target"))
}
fn safe_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Resolve and validate exactly the Cargo release output path.
pub fn discover_candidate(
    target_dir: &Path,
    target: &str,
    binary: &str,
) -> Result<(PathBuf, u64), BuildError> {
    if !valid_identifier(binary) {
        return Err(build_err("invalid binary name"));
    }
    let mut path = target_dir.join(target).join("release").join(binary);
    if target.contains("windows") {
        path.set_extension("exe");
    }
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| build_err("expected candidate missing"))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err(build_err("candidate is not a non-empty regular file"));
    }
    let canonical_root =
        fs::canonicalize(target_dir).map_err(|_| build_err("target directory unavailable"))?;
    let canonical_file =
        fs::canonicalize(&path).map_err(|_| build_err("candidate canonicalization failed"))?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(build_err("candidate escapes private target directory"));
    }
    let release_dir = target_dir.join(target).join("release");
    let target_meta = fs::symlink_metadata(target_dir.join(target))
        .map_err(|_| build_err("target output directory missing"))?;
    let release_meta = fs::symlink_metadata(&release_dir)
        .map_err(|_| build_err("release output directory missing"))?;
    if target_meta.file_type().is_symlink()
        || !target_meta.is_dir()
        || release_meta.file_type().is_symlink()
        || !release_meta.is_dir()
    {
        return Err(build_err(
            "candidate parent directory is not private regular directory",
        ));
    }
    Ok((path, metadata.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostArch, HostOs, Qualification, SupportTier, TargetPolicy, ToolchainRequirement};

    fn target() -> crate::PlannedTarget {
        crate::PlannedTarget {
            target: "x86_64-unknown-linux-gnu".into(),
            policy: TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                },
                floor: CompatibilityFloor::None,
                qualification: Qualification::Native,
                support: SupportTier::Required,
            },
            artifact_form: crate::PlannedAssetForm::Direct,
        }
    }
    #[test]
    fn command_is_explicit_and_never_shell_based() {
        let b = BuildBinding {
            selector: LogicalOutputSelector::Direct,
            package: "demo".into(),
            binary: "demo-bin".into(),
        };
        let c = cargo_command(
            &target(),
            &b,
            Path::new("/repo"),
            Path::new("/private/target"),
            Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(c.executable, "cargo");
        assert_eq!(
            c.args,
            [
                "+1.89.0",
                "build",
                "--release",
                "--locked",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--package",
                "demo",
                "--bin",
                "demo-bin"
            ]
        );
        assert_eq!(c.env["CARGO_TARGET_DIR"], "/private/target");
        let mut arbitrary = c;
        arbitrary.executable = "sh".into();
        assert!(run_bounded_inner(&arbitrary, None, false).is_err());
    }
    #[test]
    fn zigbuild_floor_uses_only_gnu_target_syntax() {
        let mut t = target();
        t.policy.strategy = BuildStrategy::CargoZigbuild;
        t.policy.toolchain.cargo_zigbuild = Some("0.19.8".into());
        t.policy.floor = CompatibilityFloor::Glibc {
            major: 2,
            minor: 17,
        };
        let binding = BuildBinding {
            selector: LogicalOutputSelector::Direct,
            package: "demo".into(),
            binary: "demo".into(),
        };
        let command = cargo_command(
            &t,
            &binding,
            Path::new("/repo"),
            Path::new("/target"),
            Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(command.args[1], "zigbuild");
        assert_eq!(command.args[5], "x86_64-unknown-linux-gnu.2.17");
        t.policy.floor = CompatibilityFloor::Macos {
            major: 13,
            minor: 0,
        };
        assert!(cargo_command(
            &t,
            &binding,
            Path::new("/repo"),
            Path::new("/target"),
            Duration::from_secs(10)
        )
        .is_err());
    }
    #[test]
    fn binding_document_rejects_duplicate_and_unknown_fields() {
        let good = r#"schema_version = 1
[targets]
"x86_64-unknown-linux-gnu" = [{ selector = { kind = "direct" }, package = "demo", binary = "demo" }]
"#;
        assert!(BuildBindingsV1::from_toml(good).is_ok());
        assert!(BuildBindingsV1::from_toml(
            &good.replace("schema_version = 1", "schema_version = 2")
        )
        .is_err());
        assert!(
            BuildBindingsV1::from_toml(&good.replace("package =", "extra = 1, package =")).is_err()
        );
    }
    #[test]
    fn bindings_must_match_direct_contract_slots_exactly() {
        let contract = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let plan = crate::PackConfig {
            schema_version: 1,
            targets: vec![target().policy],
        }
        .resolve(&contract, "1.0", "abc", &["linux-x64".into()])
        .unwrap();
        let bindings = BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                vec![BuildBinding {
                    selector: LogicalOutputSelector::Direct,
                    package: "demo".into(),
                    binary: "demo".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        assert!(bindings.validate_for(&contract, &plan).is_ok());
        let mut invalid = bindings;
        invalid.targets.get_mut("x86_64-unknown-linux-gnu").unwrap()[0].selector =
            LogicalOutputSelector::BundleEntry { index: 0 };
        assert!(invalid.validate_for(&contract, &plan).is_err());
    }
    #[test]
    fn candidate_lookup_is_exact_regular_and_nonempty() {
        let root = crate::test_temp_dir("builder-test");
        let out = root.join("x86_64-unknown-linux-gnu/release");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("demo"), b"binary").unwrap();
        assert_eq!(
            discover_candidate(&root, "x86_64-unknown-linux-gnu", "demo")
                .unwrap()
                .1,
            6
        );
        fs::write(out.join("empty"), b"").unwrap();
        assert!(discover_candidate(&root, "x86_64-unknown-linux-gnu", "empty").is_err());
        assert!(discover_candidate(&root, "x86_64-unknown-linux-gnu", "other").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn private_workspace_rejects_relative_and_reused_invocations() {
        let root = crate::test_temp_dir("private-test");
        assert!(private_target_dir(Path::new("relative"), "run", "target").is_err());
        assert!(private_target_dir(&root, "run", "target").is_ok());
        assert!(private_target_dir(&root, "run", "target").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn process_output_is_bounded_and_exit_status_explicit() {
        let spec = CommandSpec {
            executable: "rustc".into(),
            args: vec!["--version".into()],
            cwd: std::env::current_dir().unwrap(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(5),
            stdout_limit: 1024,
            stderr_limit: 1024,
            expected_stdout: None,
        };
        let result = run_bounded_inner(&spec, None, false).unwrap();
        assert_eq!(result.outcome, CommandOutcome::Success);
        assert!(result.stdout_bytes <= 1024);
    }
    #[test]
    fn nonzero_tool_exit_is_not_success() {
        let spec = CommandSpec {
            executable: "rustc".into(),
            args: vec!["--eggpack-invalid-option".into()],
            cwd: std::env::current_dir().unwrap(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(5),
            stdout_limit: 1024,
            stderr_limit: 1024,
            expected_stdout: None,
        };
        assert!(matches!(
            run_bounded_inner(&spec, None, false).unwrap().outcome,
            CommandOutcome::Failed(_)
        ));
    }
    #[test]
    fn stdout_overflow_is_bounded_and_not_success() {
        let root = crate::test_temp_dir("output-bound");
        let source = root.join("many-errors.rs");
        let body = (0..200)
            .map(|i| format!("fn f{i}() {{ absent_symbol_{i}(); }}\n"))
            .collect::<String>();
        fs::write(&source, body).unwrap();
        let spec = CommandSpec {
            executable: "rustc".into(),
            args: vec![
                "--crate-type".into(),
                "lib".into(),
                source.to_string_lossy().into_owned(),
            ],
            cwd: root.clone(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(10),
            stdout_limit: 128,
            stderr_limit: 128,
            expected_stdout: None,
        };
        assert_eq!(
            run_bounded_inner(&spec, None, false).unwrap().outcome,
            CommandOutcome::OutputLimitExceeded
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn timeout_kills_and_waits_for_the_process_group() {
        let root = crate::test_temp_dir("timeout");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname='timeout-fixture'\nversion='0.1.0'\nedition='2021'\nbuild='build.rs'\n").unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 3\n\n[[package]]\nname = \"timeout-fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("build.rs"),
            "fn main() { println!(\"cargo:rerun-if-env-changed=EGGPACK_BUILD_STARTED\"); println!(\"cargo:rerun-if-env-changed=EGGPACK_BUILD_SLEEP\"); if let Some(path) = std::env::var_os(\"EGGPACK_BUILD_STARTED\") { std::fs::write(path, b\"started\").unwrap(); } if std::env::var_os(\"EGGPACK_BUILD_SLEEP\").is_some() { std::thread::sleep(std::time::Duration::from_secs(30)); } }\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        let target_dir = root.join("cargo-target");
        let mut spec = CommandSpec {
            executable: "cargo".into(),
            args: vec![
                "build".into(),
                "--locked".into(),
                "--offline".into(),
                "--manifest-path".into(),
                root.join("Cargo.toml").to_string_lossy().into_owned(),
            ],
            cwd: root.clone(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(60),
            stdout_limit: 4096,
            stderr_limit: 4096,
            expected_stdout: None,
        };
        spec.env.insert(
            "CARGO_TARGET_DIR".into(),
            target_dir.to_string_lossy().into_owned(),
        );
        let warm_started = root.join("warm-started");
        spec.env.insert(
            "EGGPACK_BUILD_STARTED".into(),
            warm_started.to_string_lossy().into_owned(),
        );
        assert_eq!(
            run_bounded_inner(&spec, None, false).unwrap().outcome,
            CommandOutcome::Success
        );
        assert!(warm_started.is_file(), "warm build script did not start");

        for iteration in 0..3 {
            spec.timeout = Duration::from_secs(5);
            spec.env.remove("EGGPACK_BUILD_SLEEP");
            spec.env.insert("EGGPACK_BUILD_SLEEP".into(), "1".into());
            let timeout_started = root.join(format!("timeout-started-{iteration}"));
            spec.env.insert(
                "EGGPACK_BUILD_STARTED".into(),
                timeout_started.to_string_lossy().into_owned(),
            );
            assert_eq!(
                run_bounded_inner(&spec, None, false).unwrap().outcome,
                CommandOutcome::TimedOut
            );
            assert!(timeout_started.is_file(), "build script never started");
            spec.timeout = Duration::from_secs(180);
            let cancellation = BuildCancellation::new();
            let request = cancellation.clone();
            let cancellation_started = root.join(format!("cancellation-started-{iteration}"));
            spec.env.insert(
                "EGGPACK_BUILD_STARTED".into(),
                cancellation_started.to_string_lossy().into_owned(),
            );
            let cancellation_started_by_thread = cancellation_started.clone();
            let canceller = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(15);
                while !cancellation_started_by_thread.is_file() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(10));
                }
                assert!(
                    cancellation_started_by_thread.is_file(),
                    "build script did not start before cancellation deadline"
                );
                request.cancel();
            });
            assert_eq!(
                run_bounded_cancellable(&spec, &cancellation)
                    .unwrap()
                    .outcome,
                CommandOutcome::Cancelled
            );
            canceller.join().unwrap();
            assert!(cancellation_started.is_file());
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn real_local_cargo_fixture_builds_a_direct_candidate() {
        let base = crate::test_temp_dir("cargo-smoke");
        let repo = base.join("repo");
        let work = base.join("work");
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::create_dir_all(&work).unwrap();
        fs::write(
            repo.join("Cargo.toml"),
            "[package]\nname='smoke-fixture'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            repo.join("Cargo.lock"),
            "version = 3\n\n[[package]]\nname = \"smoke-fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
        let mut t = target();
        t.policy.toolchain.rust = "stable".into();
        let (triple, os, arch) = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("windows", "x86_64") => ("x86_64-pc-windows-msvc", HostOs::Windows, HostArch::X86_64),
            ("macos", "x86_64") => ("x86_64-apple-darwin", HostOs::Macos, HostArch::X86_64),
            ("macos", "aarch64") => ("aarch64-apple-darwin", HostOs::Macos, HostArch::Aarch64),
            ("linux", "x86_64") => ("x86_64-unknown-linux-gnu", HostOs::Linux, HostArch::X86_64),
            ("linux", "aarch64") => (
                "aarch64-unknown-linux-gnu",
                HostOs::Linux,
                HostArch::Aarch64,
            ),
            other => panic!("unsupported hosted Cargo smoke platform: {other:?}"),
        };
        t.target = triple.into();
        t.policy.target = triple.into();
        t.policy.host_os = os;
        t.policy.host_arch = arch;
        let binding = BuildBinding {
            selector: LogicalOutputSelector::Direct,
            package: "smoke-fixture".into(),
            binary: "smoke-fixture".into(),
        };
        let plan = ReleasePlan {
            schema_version: 1,
            release_id: "test".into(),
            source_revision: "abc".into(),
            targets: vec![t.clone()],
        };
        let attempt = execute_target(&plan, &t, &[binding], &repo, &work, "smoke").unwrap();
        assert_eq!(attempt.process.outcome, CommandOutcome::Success);
        assert_eq!(attempt.candidates.len(), 1);
        assert!(attempt.candidates[0].size > 0);
        fs::remove_dir_all(base).unwrap();
    }
}
