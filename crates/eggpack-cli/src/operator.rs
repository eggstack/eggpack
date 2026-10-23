//! Local producer operator commands (Build/Qualification M009).
//!
//! Six top-level commands — `check`, `plan`, `package`, `qualify`, `collect`,
//! and `manifest` — let a producer construct, qualify, finalize, and aggregate
//! a real release on a workstation, without a GitHub Actions run and without
//! the `ci _*` internal runner family.
//!
//! Every command here is deterministic wiring over an already-closed
//! `eggpack-core` API: it selects and validates local inputs, calls one
//! existing core function, and writes one artifact through the shared bounded
//! writer. No command in this set stages or publishes, so no command in this
//! set can mutate a public release.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{
    absolutize_cli_path, atomic_write, bound_echo, collect_finalization_inputs,
    copy_candidate_bytes, encode_release_manifest, encode_release_plan, read_bounded,
    same_location, stage_qualification_artifacts,
};

/// Maximum bytes accepted for any single local input document.
const MAX_INPUT_BYTES: usize = 1_000_000;

/// Maximum bytes accepted for one consumer validator map.
const MAX_CONSUMER_VALIDATOR_BYTES: usize = 64 * 1024;

/// Fixed streaming buffer size for one archive member digest.
const ARCHIVE_BUFFER_BYTES: usize = 64 * 1024;

/// Private build scratch directory created under the caller's output root.
const BUILD_SCRATCH_DIR: &str = ".eggpack-build";

/// Stable invocation identity for the private Cargo target directories.
const BUILD_INVOCATION: &str = "package";

/// One operator command's declared identity for `--help`.
pub(crate) struct OperatorCommand {
    /// Command name as typed on the command line.
    pub(crate) name: &'static str,
    /// Command class: validation, build, release aggregation, or final-bytes
    /// evidence.
    pub(crate) class: &'static str,
    /// One-line description of the command's effect.
    pub(crate) summary: &'static str,
    /// Full usage line for the command.
    pub(crate) usage: &'static str,
}

/// The six local producer operator commands, in help order.
pub(crate) const COMMANDS: [OperatorCommand; 6] = [
    OperatorCommand {
        name: "check",
        class: "validation",
        summary: "validate contract, policy, and bindings before anything is built",
        usage: "usage: eggpack check --contract <path> --pack-config <path> --build-bindings <path> --qualification-bindings <path> --selected <alias[,alias]> --release-id <id> --source-revision <revision> [--identity-mode <mode>] [--consumer-validators <path>]",
    },
    OperatorCommand {
        name: "plan",
        class: "validation",
        summary: "resolve the release identity and write a ReleasePlan",
        usage: "usage: eggpack plan --contract <path> --pack-config <path> --build-bindings <path> --qualification-bindings <path> --selected <alias[,alias]> --release-id <id> --source-revision <revision> --output <path> [--identity-mode <mode>] [--consumer-validators <path>]",
    },
    OperatorCommand {
        name: "package",
        class: "build",
        summary: "build every planned target's candidates under an output root",
        usage: "usage: eggpack package --release-plan <path> --build-bindings <path> --source-root <path> --output-root <path>",
    },
    OperatorCommand {
        name: "qualify",
        class: "build",
        summary: "qualify built candidates and write qualification evidence",
        usage: "usage: eggpack qualify --contract <path> --release-plan <path> --build-bindings <path> --qualification-bindings <path> --artifacts-dir <path> --output-dir <path> [--qemu-sysroot <path>]",
    },
    OperatorCommand {
        name: "collect",
        class: "release aggregation",
        summary: "aggregate qualified candidates into a finalized release tree",
        usage: "usage: eggpack collect --contract <path> --release-plan <path> --build-bindings <path> --qualification-bindings <path> --artifacts-dir <path> --output-root <path> [--consumer-validators <path>]",
    },
    OperatorCommand {
        name: "manifest",
        class: "final-bytes evidence",
        summary: "derive release-manifest.json from a finalized release tree",
        usage: "usage: eggpack manifest --contract <path> --release-plan <path> --build-bindings <path> --qualification-bindings <path> --finalized-root <path> --output <path> [--artifacts-dir <path>] [--consumer-validators <path>]",
    },
];

/// Required flags of `eggpack check`.
const CHECK_FLAGS: [&str; 7] = [
    "contract",
    "pack-config",
    "build-bindings",
    "qualification-bindings",
    "selected",
    "release-id",
    "source-revision",
];

/// Required flags of `eggpack plan`.
const PLAN_FLAGS: [&str; 8] = [
    "contract",
    "pack-config",
    "build-bindings",
    "qualification-bindings",
    "selected",
    "release-id",
    "source-revision",
    "output",
];

/// Required flags of `eggpack package`.
const PACKAGE_FLAGS: [&str; 4] = [
    "release-plan",
    "build-bindings",
    "source-root",
    "output-root",
];

/// Required flags of `eggpack qualify`.
const QUALIFY_FLAGS: [&str; 6] = [
    "contract",
    "release-plan",
    "build-bindings",
    "qualification-bindings",
    "artifacts-dir",
    "output-dir",
];

/// Required flags of `eggpack collect`.
const COLLECT_FLAGS: [&str; 6] = [
    "contract",
    "release-plan",
    "build-bindings",
    "qualification-bindings",
    "artifacts-dir",
    "output-root",
];

/// Required flags of `eggpack manifest`.
const MANIFEST_FLAGS: [&str; 6] = [
    "contract",
    "release-plan",
    "build-bindings",
    "qualification-bindings",
    "finalized-root",
    "output",
];

/// Optional flags of `eggpack check` and `eggpack plan`.
///
/// The same two surfaces the `ci _resolve-release` runner accepts: the
/// release identity mode and the consumer validator map.
const PLANNING_OPTIONAL_FLAGS: [&str; 2] = ["identity-mode", "consumer-validators"];

/// One operator command's parsed `--flag value` / `--flag=value` arguments.
///
/// Every flag is accepted at most once, values must be non-empty, unknown
/// options and positional arguments are rejected, and each required flag must
/// be present exactly once. The same surface the historical `contract expand`
/// command established, applied to every command in this family.
struct OperatorArgs {
    /// Parsed flag values, keyed by flag name without the leading dashes.
    values: BTreeMap<String, String>,
}

impl OperatorArgs {
    /// Parse one command's arguments against its required and optional flags.
    fn parse(
        args: &[String],
        required: &[&str],
        optional: &[&str],
        command: &str,
    ) -> Result<Self, String> {
        let mut values: BTreeMap<String, String> = BTreeMap::new();
        let mut index = 0usize;
        while index < args.len() {
            let arg = args[index].as_str();
            let Some(body) = arg.strip_prefix("--") else {
                return Err(format!(
                    "unexpected argument {:?}: {} accepts only named options",
                    bound_echo(arg),
                    command
                ));
            };
            let (name, inline) = match body.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (body, None),
            };
            if !required.contains(&name) && !optional.contains(&name) {
                return Err(format!("unknown option --{}", bound_echo(name)));
            }
            if values.contains_key(name) {
                return Err(format!("--{name} may be given only once"));
            }
            let value = match inline {
                Some(value) => value,
                None => {
                    index += 1;
                    args.get(index)
                        .cloned()
                        .ok_or_else(|| format!("missing value for --{name}"))?
                }
            };
            if value.is_empty() {
                return Err(format!("--{name} must not be empty"));
            }
            values.insert(name.to_owned(), value);
            index += 1;
        }

        let mut missing: Vec<String> = required
            .iter()
            .filter(|name| !values.contains_key(**name))
            .map(|name| format!("--{name}"))
            .collect();
        missing.sort_unstable();
        if !missing.is_empty() {
            return Err(format!("missing required {}", missing.join(", ")));
        }
        Ok(Self { values })
    }

    /// One required flag's value.
    fn get(&self, name: &str) -> String {
        self.values.get(name).cloned().unwrap_or_default()
    }

    /// One optional flag's value, when the caller supplied it.
    fn get_optional(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

/// Dispatch one operator command by name.
pub(crate) fn dispatch(name: &str, args: &[String]) -> Result<(), String> {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        print_help(name);
        return Ok(());
    }
    match name {
        "check" => check(args),
        "plan" => plan(args),
        "package" => package(args),
        "qualify" => qualify(args),
        "collect" => collect(args),
        "manifest" => manifest(args),
        other => Err(format!("unknown operator command: {other}")),
    }
}

/// Print one command's name, class, and usage line.
fn print_help(name: &str) {
    if let Some(command) = COMMANDS.iter().find(|command| command.name == name) {
        println!("eggpack {} — {}", command.name, command.class);
        println!("{}", command.usage);
        println!("class: {}", command.class);
        println!("No command in this set stages or publishes.");
    }
}

/// One resolved planning input set.
///
/// The same documents the `ci _resolve-release` runner consumes, plus the
/// consumer validator map when the caller configures one.
struct PlanningInputs {
    /// The distribution contract.
    contract: eggpack_contract::DistributionContract,
    /// The resolved release plan.
    plan: eggpack_core::ReleasePlan,
    /// The build bindings.
    bindings: eggpack_core::BuildBindingsV1,
    /// The qualification bindings.
    qualification_bindings: eggpack_core::QualificationBindingsV1,
    /// The consumer validator map, when the caller configured one.
    consumer_validators: Option<BTreeMap<String, eggpack_ci::ConsumerValidatorV1>>,
}

/// Read, resolve, and validate the whole planning input set.
///
/// Resolution goes through the same function the `ci _resolve-release` runner
/// calls, so both surfaces derive one plan from one rule set.
fn resolve_planning_inputs(parsed: &OperatorArgs) -> Result<PlanningInputs, String> {
    let contract = read_contract(&parsed.get("contract"))?;
    let pack_config = read_pack_config(&parsed.get("pack-config"))?;
    let bindings = read_build_bindings(&parsed.get("build-bindings"))?;
    let qualification_bindings =
        read_qualification_bindings(&parsed.get("qualification-bindings"))?;
    let selected = parse_selected(&parsed.get("selected"))?;
    let release_id = parsed.get("release-id");
    let source_revision = parsed.get("source-revision");
    let mode = parse_identity_mode(parsed.get_optional("identity-mode").as_deref())?;
    let consumer_validators = parsed
        .get_optional("consumer-validators")
        .map(|path| read_consumer_validators(&path))
        .transpose()?;
    let plan = eggpack_ci::resolve_runtime_release_plan_with_mode(
        &contract,
        &pack_config,
        &selected,
        &release_id,
        &source_revision,
        mode,
    )
    .map_err(|error| format!("release plan resolution failed: {}", diagnostic(&error)))?;
    // A plan that disagrees with its own declared floor is refused before
    // it can be written or used.
    plan.validate()
        .map_err(|error| format!("release plan is invalid: {}", diagnostic(&error)))?;
    bindings.validate_for(&contract, &plan).map_err(|error| {
        format!(
            "build bindings do not match the release plan: {}",
            diagnostic(&error)
        )
    })?;
    qualification_bindings
        .validate_for(&plan, &bindings)
        .map_err(|error| {
            format!(
                "qualification bindings do not match the release plan: {}",
                diagnostic(&error)
            )
        })?;
    Ok(PlanningInputs {
        contract,
        plan,
        bindings,
        qualification_bindings,
        consumer_validators,
    })
}

/// Project the release graph the same way the `ci _resolve-release` runner
/// does.
///
/// Without a consumer validator map the graph is the historical one; with one,
/// the consumer-aware projection the CI path uses is projected instead, so a
/// consumer-validated release aggregates exactly as it would in CI.
fn project_release_graph(inputs: &PlanningInputs) -> Result<eggpack_ci::ReleaseCIPlanV1, String> {
    let ci_plan = eggpack_ci::project_ci_plan(&inputs.contract, &inputs.plan, &inputs.bindings)
        .map_err(|error| format!("ci projection failed: {}", diagnostic(&error)))?;
    match &inputs.consumer_validators {
        None => eggpack_ci::project_release_plan(
            &ci_plan,
            &inputs.qualification_bindings,
            &inputs.bindings,
            &inputs.plan,
        )
        .map_err(|error| format!("release graph projection failed: {}", diagnostic(&error))),
        Some(map) => eggpack_ci::project_release_plan_with_consumer(
            &ci_plan,
            &inputs.qualification_bindings,
            &inputs.bindings,
            &inputs.plan,
            map.clone(),
        )
        .map_err(|error| {
            format!(
                "consumer release graph projection failed: {}",
                diagnostic(&error)
            )
        }),
    }
}

/// Parse one release identity mode.
///
/// `exact_tag` keeps the historical opaque identity. `v_prefixed_stable_semver`
/// maps exactly `vMAJOR.MINOR.PATCH` to `MAJOR.MINOR.PATCH`, exactly as the
/// `ci _resolve-release` runner does.
fn parse_identity_mode(raw: Option<&str>) -> Result<eggpack_ci::ReleaseIdentityMode, String> {
    match raw {
        None | Some("exact_tag") => Ok(eggpack_ci::ReleaseIdentityMode::ExactTag),
        Some("v_prefixed_stable_semver") => {
            Ok(eggpack_ci::ReleaseIdentityMode::VPrefixedStableSemver)
        }
        _ => Err("unknown release identity mode".to_owned()),
    }
}

/// Read and parse one consumer validator map.
///
/// The map is the same JSON document the `ci _resolve-release` runner consumes.
fn read_consumer_validators(
    path: &str,
) -> Result<BTreeMap<String, eggpack_ci::ConsumerValidatorV1>, String> {
    let text = read_bounded(
        Path::new(path),
        MAX_CONSUMER_VALIDATOR_BYTES,
        "consumer validators",
    )?;
    let map: BTreeMap<String, eggpack_ci::ConsumerValidatorV1> =
        serde_json::from_str(&text).map_err(|_| "invalid consumer validators".to_owned())?;
    for validator in map.values() {
        validator
            .validate()
            .map_err(|error| format!("invalid consumer validators: {}", diagnostic(&error)))?;
    }
    Ok(map)
}

/// Read and parse one distribution contract.
fn read_contract(path: &str) -> Result<eggpack_contract::DistributionContract, String> {
    let text = read_bounded(Path::new(path), MAX_INPUT_BYTES, "contract")?;
    eggpack_contract::DistributionContract::parse_toml_str(&text)
        .map_err(|error| format!("invalid contract: {}", diagnostic(&error)))
}

/// Read and parse one producer policy document.
///
/// TOML is the checked-in encoding and JSON is accepted as well, exactly as
/// the `ci _resolve-release` runner accepts both. The JSON document is
/// normalized into the canonical document shape and re-read by the same
/// strict core parser, so one validator stays authoritative for both
/// encodings.
fn read_pack_config(path: &str) -> Result<eggpack_core::PackConfig, String> {
    read_strict_document(path, "pack config", |text| {
        eggpack_core::PackConfig::from_toml(text)
            .map_err(|error| format!("invalid pack config: {}", diagnostic(&error)))
    })
}

/// Read and parse one build bindings document.
///
/// TOML is the checked-in encoding and JSON is accepted as well, exactly as
/// the `ci _resolve-release` runner accepts both.
fn read_build_bindings(path: &str) -> Result<eggpack_core::BuildBindingsV1, String> {
    read_strict_document(path, "build bindings", |text| {
        eggpack_core::BuildBindingsV1::from_toml(text)
            .map_err(|error| format!("invalid build bindings: {}", diagnostic(&error)))
    })
}

/// Read and parse one qualification bindings document.
///
/// TOML is the checked-in encoding and JSON is accepted as well, exactly as
/// the `ci _resolve-release` runner accepts both.
fn read_qualification_bindings(
    path: &str,
) -> Result<eggpack_core::QualificationBindingsV1, String> {
    read_strict_document(path, "qualification bindings", |text| {
        eggpack_core::QualificationBindingsV1::from_toml(text)
            .map_err(|error| format!("invalid qualification bindings: {}", diagnostic(&error)))
    })
}

/// Read one bounded input document and parse it with one strict parser.
///
/// The document is read as TOML first, which is the checked-in encoding. A
/// JSON document is accepted as well: it is decoded into the canonical
/// document shape and re-read by the same strict parser, so the document's
/// bounds, unknown-field rejection, and strictness stay in one place.
fn read_strict_document<T>(
    path: &str,
    label: &str,
    parse: impl Fn(&str) -> Result<T, String>,
) -> Result<T, String> {
    let text = read_bounded(Path::new(path), MAX_INPUT_BYTES, label)?;
    if let Ok(value) = parse(&text) {
        return Ok(value);
    }
    let value: toml::Value = serde_json::from_str(&text).map_err(|_| format!("invalid {label}"))?;
    let canonical = toml::to_string(&value).map_err(|_| format!("invalid {label}"))?;
    parse(&canonical)
}

/// Read and validate one release plan document.
fn read_release_plan(path: &str) -> Result<eggpack_core::ReleasePlan, String> {
    let text = read_bounded(Path::new(path), MAX_INPUT_BYTES, "release plan")?;
    let plan: eggpack_core::ReleasePlan =
        serde_json::from_str(&text).map_err(|_| "invalid release plan".to_owned())?;
    plan.validate()
        .map_err(|error| format!("release plan is invalid: {}", diagnostic(&error)))?;
    Ok(plan)
}

/// Split the comma-separated selected target alias list.
///
/// Every element must name a target: an empty element is refused here rather
/// than reaching the resolver as an out-of-bounds alias.
fn parse_selected(raw: &str) -> Result<Vec<String>, String> {
    let mut selected = Vec::new();
    for alias in raw.split(',') {
        if alias.is_empty() {
            return Err("selected target aliases must not be empty".to_owned());
        }
        selected.push(alias.to_owned());
    }
    Ok(selected)
}

/// Validate the whole planning input set without writing anything.
fn check(args: &[String]) -> Result<(), String> {
    let parsed = OperatorArgs::parse(args, &CHECK_FLAGS, &PLANNING_OPTIONAL_FLAGS, "check")?;
    let inputs = resolve_planning_inputs(&parsed)?;
    // A configured consumer validator map is projected here, so a map that
    // does not describe this release is refused before anything is built.
    if inputs.consumer_validators.is_some() {
        project_release_graph(&inputs)?;
    }
    println!("check: ok");
    Ok(())
}

/// Resolve the release plan and write it as the canonical plan document.
fn plan(args: &[String]) -> Result<(), String> {
    let parsed = OperatorArgs::parse(args, &PLAN_FLAGS, &PLANNING_OPTIONAL_FLAGS, "plan")?;
    let inputs = resolve_planning_inputs(&parsed)?;
    if inputs.consumer_validators.is_some() {
        project_release_graph(&inputs)?;
    }
    let output = PathBuf::from(parsed.get("output"));
    // The same encoder the `ci _resolve-release` runner writes with, so both
    // surfaces emit byte-identical plans for one resolved plan.
    let plan_json = encode_release_plan(&inputs.plan)?;
    atomic_write(&output, plan_json.as_bytes())?;
    println!("plan: ok");
    Ok(())
}

/// Build every planned target's candidates under the caller's output root.
fn package(args: &[String]) -> Result<(), String> {
    let parsed = OperatorArgs::parse(args, &PACKAGE_FLAGS, &[], "package")?;
    let plan = read_release_plan(&parsed.get("release-plan"))?;
    let bindings = read_build_bindings(&parsed.get("build-bindings"))?;
    let cwd = std::env::current_dir().map_err(|_| "working directory is unavailable".to_owned())?;
    let source_root = absolutize_cli_path(Path::new(&parsed.get("source-root")), &cwd)?;
    let output_root = absolutize_cli_path(Path::new(&parsed.get("output-root")), &cwd)?;
    reject_symlink_dir(&output_root, "output root")?;
    let work_root = output_root.join(BUILD_SCRATCH_DIR);
    reset_build_scratch(&work_root)?;

    // Every target is built before any artifact is written, so a failed build
    // leaves no half-written release behind.
    let mut built = Vec::with_capacity(plan.targets.len());
    for planned in &plan.targets {
        let target_bindings = bindings
            .targets
            .get(&planned.target)
            .ok_or_else(|| format!("build bindings are missing target {}", planned.target))?;
        let attempt = eggpack_core::execute_target(
            &plan,
            planned,
            target_bindings,
            &source_root,
            &work_root,
            BUILD_INVOCATION,
        )
        .map_err(|error| {
            format!(
                "build failed for {}: {}",
                planned.target,
                diagnostic(&error)
            )
        })?;
        if attempt.process.outcome != eggpack_core::CommandOutcome::Success {
            return Err(format!("build did not succeed for {}", planned.target));
        }
        let handoff = eggpack_ci::project_build_handoff(&plan, &bindings, &planned.target)
            .map_err(|error| format!("build handoff projection failed: {}", diagnostic(&error)))?;
        built.push((planned.target.clone(), attempt, handoff));
    }
    for (target, attempt, handoff) in built {
        write_build_artifacts(&output_root, &target, &attempt, handoff)?;
    }
    println!("package: ok");
    Ok(())
}

/// One bounded, single-line diagnostic extracted from a parser or core error.
///
/// The exit discipline is one `eggpack: <message>` line, so a multi-line
/// parser diagnostic is reduced to its most specific line.
fn diagnostic(error: &impl std::fmt::Display) -> String {
    let text = error.to_string();
    let line = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("invalid document")
        .trim();
    bound_echo(line)
}

/// Require one existing, real directory.
fn require_real_dir(path: &Path, label: &str) -> Result<(), String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| format!("{label} is unavailable"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink"));
    }
    if !metadata.is_dir() {
        return Err(format!("{label} is not a real directory"));
    }
    Ok(())
}

/// Reject an output directory that already exists as a symlink.
///
/// An output root the caller names is written into directly, so a symlinked
/// directory is refused rather than followed.
fn reject_symlink_dir(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!("{label} must not be a symlink"));
            }
            if !metadata.is_dir() {
                return Err(format!("{label} is not a real directory"));
            }
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

/// Recreate Eggpack's private build scratch directory under the output root.
///
/// The scratch root holds the private Cargo target directories the core
/// builder creates. It is recreated on every run so a restarted stage starts
/// from a clean private build directory.
fn reset_build_scratch(work_root: &Path) -> Result<(), String> {
    if let Ok(metadata) = std::fs::symlink_metadata(work_root) {
        if metadata.file_type().is_symlink() {
            return Err("build scratch root must not be a symlink".to_owned());
        }
        if !metadata.is_dir() {
            return Err("build scratch root is not a directory".to_owned());
        }
        std::fs::remove_dir_all(work_root).map_err(|_| "build scratch reset failed".to_owned())?;
    }
    std::fs::create_dir_all(work_root).map_err(|_| "build scratch creation failed".to_owned())?;
    Ok(())
}

/// Write one target's canonical build artifact directory.
fn write_build_artifacts(
    output_root: &Path,
    target: &str,
    attempt: &eggpack_core::BuildAttempt,
    mut handoff: eggpack_ci::BuildHandoffV1,
) -> Result<(), String> {
    let dir = output_root.join(target);
    let candidates_dir = dir.join("candidates");
    std::fs::create_dir_all(&candidates_dir)
        .map_err(|_| "output dir creation failed".to_owned())?;
    for output in &mut handoff.outputs {
        let candidate = attempt
            .candidates
            .iter()
            .find(|candidate| candidate.selector == output.selector)
            .ok_or_else(|| "build attempt is missing a bound candidate".to_owned())?;
        let metadata = std::fs::symlink_metadata(&candidate.path)
            .map_err(|_| "candidate file unavailable".to_owned())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
            return Err("candidate is not a non-empty regular file".to_owned());
        }
        output.size = metadata.len();
        let dest = candidates_dir.join(&output.relative_path);
        copy_candidate_bytes(&candidate.path, &dest)?;
    }
    handoff
        .validate()
        .map_err(|_| "handoff validation failed".to_owned())?;
    let handoff_json = handoff
        .to_json()
        .map_err(|_| "handoff serialization failed".to_owned())?;
    atomic_write(&dir.join("build-handoff.json"), handoff_json.as_bytes())?;
    eggpack_ci::validate_build_artifact_dir(&dir)
        .map_err(|_| "staged build artifact validation failed".to_owned())?;
    Ok(())
}

/// Qualify every planned target's candidates and write the evidence.
fn qualify(args: &[String]) -> Result<(), String> {
    let parsed = OperatorArgs::parse(args, &QUALIFY_FLAGS, &["qemu-sysroot"], "qualify")?;
    let contract = read_contract(&parsed.get("contract"))?;
    let plan = read_release_plan(&parsed.get("release-plan"))?;
    let bindings = read_build_bindings(&parsed.get("build-bindings"))?;
    let qualification_bindings =
        read_qualification_bindings(&parsed.get("qualification-bindings"))?;
    let artifacts_dir = PathBuf::from(parsed.get("artifacts-dir"));
    let output_dir = PathBuf::from(parsed.get("output-dir"));
    reject_symlink_dir(&output_dir, "output dir")?;
    // Staging the qualification artifacts into the directory the candidates
    // were read from would rewrite the very bytes the qualification is about,
    // so the same location is refused before anything is written.
    if same_location(&artifacts_dir, &output_dir) {
        return Err("output dir must differ from artifacts dir".to_owned());
    }
    // An emulated target is qualified with the caller's QEMU sysroot, exactly
    // as the `ci _qualify-target` runner qualifies it: without one the
    // evidence would be weaker than the CI path produces.
    let qemu_sysroot = parsed.get_optional("qemu-sysroot").map(PathBuf::from);
    if let Some(sysroot) = &qemu_sysroot {
        let metadata = std::fs::symlink_metadata(sysroot)
            .map_err(|_| "qemu sysroot unavailable".to_owned())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("qemu sysroot is not a real directory".to_owned());
        }
    }
    let cwd = std::env::current_dir().map_err(|_| "working directory is unavailable".to_owned())?;
    let cancellation = eggpack_core::BuildCancellation::new();
    let runtime = eggpack_core::QualificationRuntime { qemu_sysroot };

    // Every target is qualified before any artifact is written, so a failed
    // qualification leaves no half-written release behind.
    let mut qualified = Vec::with_capacity(plan.targets.len());
    for planned in &plan.targets {
        let target_dir = artifacts_dir.join(&planned.target);
        let handoff_text = read_bounded(
            &target_dir.join("build-handoff.json"),
            MAX_INPUT_BYTES,
            "build handoff",
        )?;
        let handoff = eggpack_ci::BuildHandoffV1::from_json(&handoff_text)
            .map_err(|_| "invalid handoff".to_owned())?;
        let candidate_dir = absolutize_cli_path(&target_dir.join("candidates"), &cwd)?;
        let attempt = eggpack_ci::reconstruct_attempt(&plan, planned, &handoff, &candidate_dir)
            .map_err(|_| "candidate reconstruction failed".to_owned())?;
        let evidence = eggpack_core::qualify_target(eggpack_core::QualificationRequest {
            contract: &contract,
            plan: &plan,
            target: planned,
            attempt: &attempt,
            build_bindings: &bindings,
            qualification_bindings: &qualification_bindings,
            runtime: &runtime,
            cancellation: &cancellation,
        })
        .map_err(|error| {
            format!(
                "qualification failed for {}: {}",
                planned.target,
                diagnostic(&error)
            )
        })?;
        qualified.push((
            planned.target.clone(),
            handoff_text,
            evidence,
            candidate_dir,
        ));
    }
    for (target, handoff_text, evidence, candidate_dir) in qualified {
        let dir = output_dir.join(&target);
        std::fs::create_dir_all(&dir).map_err(|_| "output dir creation failed".to_owned())?;
        let evidence_json = eggpack_ci::encode_qualification_evidence(&evidence)
            .map_err(|_| "evidence encode failed".to_owned())?;
        atomic_write(&dir.join("evidence.json"), evidence_json.as_bytes())?;
        stage_qualification_artifacts(&dir, &candidate_dir, &handoff_text)?;
        eggpack_ci::validate_qualification_artifact_dir(&dir)
            .map_err(|_| "staged qualification artifact validation failed".to_owned())?;
    }
    println!("qualify: ok");
    Ok(())
}

/// Aggregate qualified candidates into a finalized release tree.
fn collect(args: &[String]) -> Result<(), String> {
    let parsed = OperatorArgs::parse(args, &COLLECT_FLAGS, &["consumer-validators"], "collect")?;
    let contract = read_contract(&parsed.get("contract"))?;
    let plan = read_release_plan(&parsed.get("release-plan"))?;
    let bindings = read_build_bindings(&parsed.get("build-bindings"))?;
    let qualification_bindings =
        read_qualification_bindings(&parsed.get("qualification-bindings"))?;
    let consumer_validators = parsed
        .get_optional("consumer-validators")
        .map(|path| read_consumer_validators(&path))
        .transpose()?;
    let artifacts_dir = PathBuf::from(parsed.get("artifacts-dir"));
    let cwd = std::env::current_dir().map_err(|_| "working directory is unavailable".to_owned())?;
    let output_root = absolutize_cli_path(Path::new(&parsed.get("output-root")), &cwd)?;
    reject_symlink_dir(&output_root, "output root")?;
    // The finalizer writes one new release root under a parent the caller
    // already secures, and the manifest document lands beside that root.
    let output_parent = output_root
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    require_real_dir(&output_parent, "output root parent")?;

    // The same graph projection the `ci _resolve-release` runner performs, so
    // the local path aggregates the same targets with the same finalization
    // settings the generated workflow would use. A configured consumer
    // validator map projects the consumer-aware graph the CI path uses.
    let ci_plan = eggpack_ci::project_ci_plan(&contract, &plan, &bindings)
        .map_err(|error| format!("ci projection failed: {}", diagnostic(&error)))?;
    let graph = match &consumer_validators {
        None => {
            eggpack_ci::project_release_plan(&ci_plan, &qualification_bindings, &bindings, &plan)
                .map_err(|error| {
                    format!("release graph projection failed: {}", diagnostic(&error))
                })?
        }
        Some(map) => eggpack_ci::project_release_plan_with_consumer(
            &ci_plan,
            &qualification_bindings,
            &bindings,
            &plan,
            map.clone(),
        )
        .map_err(|error| {
            format!(
                "consumer release graph projection failed: {}",
                diagnostic(&error)
            )
        })?,
    };
    let targets: Vec<String> = graph
        .qualifications
        .iter()
        .map(|qualification| qualification.target.clone())
        .collect();
    let inputs = collect_finalization_inputs(&artifacts_dir, &plan, &targets)?;
    let (outcome, finalized) = if graph.consumer_validators.is_empty() {
        // Stray consumer evidence without a configured validator fails closed
        // so validation can never be silently bypassed.
        for target in &targets {
            if artifacts_dir
                .join(target)
                .join("consumer-evidence.json")
                .exists()
            {
                return Err("consumer evidence present without a configured validator".to_owned());
            }
        }
        eggpack_ci::aggregate_finalize(
            &contract,
            &plan,
            &graph,
            &inputs,
            &graph.finalization.evidence_references,
            &output_root,
        )
        .map_err(|error| format!("aggregation failed: {}", diagnostic(&error)))?
    } else {
        // A consumer-validated release aggregates with its consumer evidence,
        // exactly as the `ci _aggregate` runner does.
        let mut consumer_evidences = Vec::new();
        for target in graph.consumer_validators.keys() {
            let path = artifacts_dir.join(target).join("consumer-evidence.json");
            let text = read_bounded(&path, MAX_CONSUMER_VALIDATOR_BYTES, "consumer evidence")?;
            let evidence = eggpack_ci::decode_consumer_evidence(&text)
                .map_err(|_| "invalid consumer evidence".to_owned())?;
            consumer_evidences.push(evidence);
        }
        eggpack_ci::aggregate_finalize_with_consumer(
            &contract,
            &plan,
            &graph,
            &inputs,
            &consumer_evidences,
            &graph.finalization.evidence_references,
            &output_root,
        )
        .map_err(|error| format!("consumer aggregation failed: {}", diagnostic(&error)))?
    };
    let finalized = finalized.ok_or_else(|| "aggregation did not finalize a release".to_owned())?;
    if outcome != eggpack_ci::AggregateOutcome::Complete {
        return Err("aggregation did not complete".to_owned());
    }
    // The manifest document lives beside the finalized root, never inside it.
    let manifest_json = encode_release_manifest(&finalized.manifest)?;
    atomic_write(
        &output_parent.join("release-manifest.json"),
        manifest_json.as_bytes(),
    )?;
    println!("collect: ok");
    Ok(())
}

/// Derive the release manifest from a finalized release tree's bytes.
///
/// The derived manifest is self-checking: every archive member digest is
/// read back out of the finalized `.tar.gz` and compared with the candidate
/// bytes the digest is taken from, and the derived document is compared with
/// the collector's `release-manifest.json` when that file is present beside
/// the finalized root. A stale or tampered artifacts directory fails closed
/// instead of producing a manifest that describes bytes the release does not
/// contain.
fn manifest(args: &[String]) -> Result<(), String> {
    let parsed = OperatorArgs::parse(
        args,
        &MANIFEST_FLAGS,
        &["artifacts-dir", "consumer-validators"],
        "manifest",
    )?;
    let contract = read_contract(&parsed.get("contract"))?;
    let plan = read_release_plan(&parsed.get("release-plan"))?;
    let bindings = read_build_bindings(&parsed.get("build-bindings"))?;
    let qualification_bindings =
        read_qualification_bindings(&parsed.get("qualification-bindings"))?;
    let consumer_validators = parsed
        .get_optional("consumer-validators")
        .map(|path| read_consumer_validators(&path))
        .transpose()?;
    let finalized_root = PathBuf::from(parsed.get("finalized-root"));
    let output = PathBuf::from(parsed.get("output"));
    let artifacts_dir = parsed.get_optional("artifacts-dir").map(PathBuf::from);

    // The same graph projection the `ci _resolve-release` runner performs, so
    // the derived manifest carries the same finalization settings — including
    // the evidence references — as the aggregated manifest.
    let ci_plan = eggpack_ci::project_ci_plan(&contract, &plan, &bindings)
        .map_err(|error| format!("ci projection failed: {}", diagnostic(&error)))?;
    let graph = match &consumer_validators {
        None => {
            eggpack_ci::project_release_plan(&ci_plan, &qualification_bindings, &bindings, &plan)
                .map_err(|error| {
                    format!("release graph projection failed: {}", diagnostic(&error))
                })?
        }
        Some(map) => eggpack_ci::project_release_plan_with_consumer(
            &ci_plan,
            &qualification_bindings,
            &bindings,
            &plan,
            map.clone(),
        )
        .map_err(|error| {
            format!(
                "consumer release graph projection failed: {}",
                diagnostic(&error)
            )
        })?,
    };

    let mut targets = Vec::with_capacity(plan.targets.len());
    for planned in &plan.targets {
        let expanded = contract
            .expand(&planned.target, &plan.release_id)
            .map_err(|error| format!("contract expansion failed: {}", diagnostic(&error)))?;
        let expected =
            eggpack_contract::expected_release_files(&contract, &planned.target, &plan.release_id)
                .map_err(|error| {
                    format!(
                        "expected inventory derivation failed: {}",
                        diagnostic(&error)
                    )
                })?;
        let mut release_files = BTreeMap::new();
        for file in expected {
            release_files.insert(file.file_name.clone(), finalized_root.join(&file.file_name));
        }
        let mut archive_members = BTreeMap::new();
        if let eggpack_contract::ExpandedAssets::Archive(archive) = &expanded.assets {
            // An archive's member digests are facts about the member bytes.
            // The finalized tree keeps only the archive itself, so every
            // member is read back out of the finalized `.tar.gz` and compared
            // with the candidate bytes the digest is taken from. A member that
            // is absent from the finalized archive, or that differs from it,
            // fails closed: the manifest never describes bytes the finalized
            // release does not contain.
            let artifacts_dir = artifacts_dir
                .as_ref()
                .ok_or_else(|| "archive target requires --artifacts-dir".to_owned())?;
            let target_dir = artifacts_dir.join(&planned.target);
            let handoff_text = read_bounded(
                &target_dir.join("build-handoff.json"),
                MAX_INPUT_BYTES,
                "build handoff",
            )?;
            let handoff = eggpack_ci::BuildHandoffV1::from_json(&handoff_text)
                .map_err(|_| "invalid handoff".to_owned())?;
            let finalized_archive = finalized_root.join(&archive.archive_file);
            for member in &archive.members {
                let selector = eggpack_core::LogicalOutputSelector::ArchiveMember {
                    source: member.source.clone(),
                };
                let output = handoff
                    .outputs
                    .iter()
                    .find(|output| output.selector == selector)
                    .ok_or_else(|| "archive member is absent from the build handoff".to_owned())?;
                let candidate = target_dir.join("candidates").join(&output.relative_path);
                let (size, sha256) = eggpack_core::file_digest(&candidate)
                    .map_err(|error| format!("archive member candidate {}", error.reason()))?;
                let (archived_size, archived_sha256) =
                    digest_archive_member(&finalized_archive, &member.source)?;
                if size != archived_size || sha256 != archived_sha256 {
                    return Err(format!(
                        "archive member {} does not match the finalized archive",
                        member.source
                    ));
                }
                archive_members.insert(member.source.clone(), candidate);
            }
        }
        targets.push(eggpack_core::FinalizedTargetInput {
            target: planned.target.clone(),
            release_files,
            archive_members,
        });
    }
    let manifest = eggpack_core::build_manifest(
        &contract,
        &eggpack_core::FinalizedReleaseInput {
            product_id: contract.product.id.clone(),
            release_id: plan.release_id.clone(),
            source_revision: plan.source_revision.clone(),
            targets,
            // The same evidence references the aggregator records, taken from
            // the same projected graph, so the derived document and the
            // collected document never describe different releases.
            evidence_references: graph.finalization.evidence_references.clone(),
        },
    )
    .map_err(|error| format!("manifest construction failed: {}", diagnostic(&error)))?;
    let manifest_json = encode_release_manifest(&manifest)?;
    // The aggregator writes its manifest beside the finalized root. When it
    // is there, the derived document must be the same document.
    compare_collected_manifest(&finalized_root, &manifest_json)?;
    atomic_write(&output, manifest_json.as_bytes())?;
    println!("manifest: ok");
    Ok(())
}

/// Compare the derived manifest with the collector's manifest.
///
/// The collector writes `release-manifest.json` beside the finalized root. A
/// derived manifest that differs from it describes a different release, so
/// the difference fails closed instead of being written out.
fn compare_collected_manifest(finalized_root: &Path, manifest_json: &str) -> Result<(), String> {
    let Some(parent) = finalized_root
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    let collected = parent.join("release-manifest.json");
    if !collected.exists() {
        return Ok(());
    }
    let text = read_bounded(&collected, MAX_INPUT_BYTES, "collected release manifest")?;
    let decoded = eggpack_manifest::ReleaseManifest::from_json(&text)
        .map_err(|_| "collected release manifest is invalid".to_owned())?;
    let collected_json = encode_release_manifest(&decoded)?;
    if collected_json != manifest_json {
        return Err("derived manifest differs from the collected release manifest".to_owned());
    }
    Ok(())
}

/// Digest one member inside a finalized `.tar.gz` archive.
///
/// The member is located by its archive entry path, which the finalizer
/// writes as the member's contract `source`. The bytes are streamed through a
/// fixed buffer, so a large member never grows the process to accommodate it.
fn digest_archive_member(archive: &Path, member_source: &str) -> Result<(u64, String), String> {
    let file = std::fs::File::open(archive)
        .map_err(|_| "finalized archive could not be opened".to_owned())?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let entries = tar
        .entries()
        .map_err(|_| "finalized archive could not be read".to_owned())?;
    for entry in entries {
        let mut entry = entry.map_err(|_| "finalized archive could not be read".to_owned())?;
        let path = entry
            .path()
            .map_err(|_| "finalized archive entry path is invalid".to_owned())?;
        if path.to_string_lossy() != member_source {
            continue;
        }
        if !entry.header().entry_type().is_file() {
            return Err("finalized archive member is not a regular file".to_owned());
        }
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; ARCHIVE_BUFFER_BYTES];
        loop {
            let count = entry
                .read(&mut buffer)
                .map_err(|_| "finalized archive member could not be read".to_owned())?;
            if count == 0 {
                break;
            }
            size = size
                .checked_add(count as u64)
                .ok_or_else(|| "finalized archive member size overflow".to_owned())?;
            hasher.update(&buffer[..count]);
        }
        let digest = hasher.finalize();
        let hex = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        return Ok((size, hex));
    }
    Err("finalized archive does not contain the member".to_owned())
}
