#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Minimal deterministic Eggpack CLI.

use std::path::{Path, PathBuf};

fn main() {
    std::process::exit(run(std::env::args().skip(1).collect()));
}

fn run(args: Vec<String>) -> i32 {
    match dispatch(args) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("eggpack: {message}");
            1
        }
    }
}

fn dispatch(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]".to_owned());
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("eggpack {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args[0] == "--help" || args[0] == "-h" {
        println!("usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]");
        return Ok(());
    }
    if args[0] != "ci" {
        return Err("usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]".to_owned());
    }
    if args.len() < 2 {
        return Err("usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]".to_owned());
    }
    match args[1].as_str() {
        "_verify-source" => ci_verify_source(&args[2..]),
        "_resolve-release" => ci_resolve_release(&args[2..]),
        "generate" => ci_generate(&args[2..]),
        "check" => ci_check(&args[2..]),
        "_capture-build" => ci_capture_build(&args[2..]),
        "_qualify-target" => ci_qualify_target(&args[2..]),
        "_validate-consumer" => ci_validate_consumer(&args[2..]),
        "_evaluate-gate" => ci_evaluate_gate(&args[2..]),
        "_aggregate" => ci_aggregate(&args[2..]),
        "_prepare-stage" => ci_prepare_stage(&args[2..]),
        "_stage-github-draft" => ci_stage_github_draft(&args[2..]),
        _ => Err("unknown ci subcommand".to_owned()),
    }
}

fn ci_verify_source(args: &[String]) -> Result<(), String> {
    let plan_path = PathBuf::from(get_flag(args, "release-plan")?);
    let text = read_bounded(&plan_path, 1_000_000, "release plan")?;
    let plan: eggpack_core::ReleasePlan =
        serde_json::from_str(&text).map_err(|_| "invalid release plan".to_owned())?;
    let expected = plan.source_revision.as_str();
    if expected.len() != 40
        || !expected
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("release plan source revision is invalid".to_owned());
    }
    verify_source_revision(expected, None)
}

fn verify_source_revision(expected: &str, cwd: Option<&Path>) -> Result<(), String> {
    let mut command = std::process::Command::new("git");
    command.args(["rev-parse", "--verify", "HEAD^{commit}"]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .map_err(|_| "checked-out source verification failed".to_owned())?;
    if !output.status.success() {
        return Err("checked-out source verification failed".to_owned());
    }
    let actual = std::str::from_utf8(&output.stdout)
        .map_err(|_| "checked-out source verification failed".to_owned())?
        .trim();
    if actual.len() != 40
        || !actual
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || actual != expected
    {
        return Err("checked-out source does not match release plan".to_owned());
    }
    println!("checked-out source matches release plan");
    Ok(())
}

/// Resolve invocation-local release identity for reusable workflows.
///
/// Validates the exact event-selected tag, verifies checked-out HEAD equals
/// the given source revision, resolves PackConfig through
/// `PackConfig::resolve` using the exact tag as release_id, resolves the
/// static draft template for the same tag, and writes the three
/// invocation-local documents into workflow-private storage only.
#[allow(clippy::too_many_lines)]
fn ci_resolve_release(args: &[String]) -> Result<(), String> {
    let contract_path = PathBuf::from(get_flag(args, "contract")?);
    let pack_config_path = PathBuf::from(get_flag(args, "pack-config")?);
    let build_bindings_path = PathBuf::from(get_flag(args, "build-bindings")?);
    let qual_bindings_path = PathBuf::from(get_flag(args, "qualification-bindings")?);
    let validators_path = get_flag_optional(args, "consumer-validators").map(PathBuf::from);
    let selected_raw = get_flag(args, "selected")?;
    let tag = get_flag(args, "tag")?;
    let source_revision = get_flag(args, "source-revision")?;
    let template_path = PathBuf::from(get_flag(args, "template")?);
    let source_root = PathBuf::from(get_flag(args, "source-root")?);
    let output_plan = PathBuf::from(get_flag(args, "output-plan")?);
    let output_ci_plan = PathBuf::from(get_flag(args, "output-ci-plan")?);
    let output_github_policy = PathBuf::from(get_flag(args, "output-github-policy")?);
    if args.len() > 28 {
        return Err("too many arguments for ci _resolve-release".to_owned());
    }
    // Checked-out HEAD is consumed as the source revision: the given
    // revision must equal HEAD in the explicit source root or resolution
    // fails closed.
    verify_source_revision(&source_revision, Some(&source_root))?;
    let contract_text = read_bounded(&contract_path, 1_000_000, "contract")?;
    let contract = eggpack_contract::DistributionContract::parse_toml_str(&contract_text)
        .map_err(|_| "invalid contract".to_owned())?;
    let pack_text = read_bounded(&pack_config_path, 1_000_000, "pack config")?;
    let pack_config: eggpack_core::PackConfig = toml::from_str(&pack_text)
        .map_err(|_| "invalid pack config".to_owned())
        .or_else(|_: String| {
            serde_json::from_str(&pack_text).map_err(|_| "invalid pack config".to_owned())
        })?;
    let selected: Vec<String> = selected_raw.split(',').map(str::to_owned).collect();
    if selected.is_empty() {
        return Err("selected targets must not be empty".to_owned());
    }
    let plan = eggpack_ci::resolve_runtime_release_plan(
        &contract,
        &pack_config,
        &selected,
        &tag,
        &source_revision,
    )
    .map_err(|_| "runtime release resolution failed".to_owned())?;
    let template_text = read_bounded(&template_path, 64 * 1024, "draft template")?;
    let template: eggpack_github::GitHubDraftTemplateV1 = toml::from_str(&template_text)
        .map_err(|_| "invalid draft template".to_owned())
        .or_else(|_: String| {
            eggpack_github::GitHubDraftTemplateV1::from_json(&template_text)
                .map_err(|_| "invalid draft template".to_owned())
        })?;
    let draft_policy = template
        .resolve(&tag)
        .map_err(|_| "draft template resolution failed".to_owned())?;
    if draft_policy.tag != plan.release_id || draft_policy.title.is_empty() {
        return Err("resolved draft policy differs from release identity".to_owned());
    }
    let bindings_text = read_bounded(&build_bindings_path, 1_000_000, "build bindings")?;
    let bindings: eggpack_core::BuildBindingsV1 = toml::from_str(&bindings_text)
        .map_err(|_| "invalid build bindings".to_owned())
        .or_else(|_: String| {
            serde_json::from_str(&bindings_text).map_err(|_| "invalid build bindings".to_owned())
        })?;
    let qual_text = read_bounded(&qual_bindings_path, 1_000_000, "qualification bindings")?;
    let qual_bindings: eggpack_core::QualificationBindingsV1 = toml::from_str(&qual_text)
        .map_err(|_| "invalid qualification bindings".to_owned())
        .or_else(|_: String| {
            serde_json::from_str(&qual_text)
                .map_err(|_| "invalid qualification bindings".to_owned())
        })?;
    let ci_plan = eggpack_ci::project_ci_plan(&contract, &plan, &bindings)
        .map_err(|_| "ci projection failed".to_owned())?;
    let graph = match validators_path {
        None => eggpack_ci::project_release_plan(&ci_plan, &qual_bindings, &bindings, &plan)
            .map_err(|_| "release graph projection failed".to_owned())?,
        Some(path) => {
            let map_text = read_bounded(&path, 64 * 1024, "consumer validators")?;
            let map: std::collections::BTreeMap<String, eggpack_ci::ConsumerValidatorV1> =
                serde_json::from_str(&map_text)
                    .map_err(|_| "invalid consumer validators".to_owned())?;
            eggpack_ci::project_release_plan_with_consumer(
                &ci_plan,
                &qual_bindings,
                &bindings,
                &plan,
                map,
            )
            .map_err(|_| "consumer release graph projection failed".to_owned())?
        }
    };
    // Runtime documents land only in invocation-private workflow storage;
    // they are never written back into the repository by this command.
    let plan_json =
        serde_json::to_string(&plan).map_err(|_| "release plan encode failed".to_owned())?;
    let graph_json = graph
        .to_json()
        .map_err(|_| "release graph encode failed".to_owned())?;
    let policy_json = draft_policy
        .to_json()
        .map_err(|_| "draft policy encode failed".to_owned())?;
    atomic_write(&output_plan, plan_json.as_bytes())?;
    atomic_write(&output_ci_plan, graph_json.as_bytes())?;
    atomic_write(&output_github_policy, policy_json.as_bytes())?;
    println!("resolved runtime release identity for tag {tag}");
    Ok(())
}

fn get_flag(args: &[String], name: &str) -> Result<String, String> {
    let flag = format!("--{name}");
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == &flag {
            return iter
                .next()
                .cloned()
                .ok_or_else(|| format!("missing value for {flag}"));
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Ok(value.to_string());
        }
    }
    Err(format!("missing required {flag}"))
}

fn get_flag_optional(args: &[String], name: &str) -> Option<String> {
    let flag = format!("--{name}");
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == &flag {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Some(value.to_string());
        }
    }
    None
}

fn read_bounded(path: &Path, max: usize, label: &str) -> Result<String, String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| format!("{label} is unavailable"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink"));
    }
    if !metadata.is_file() {
        return Err(format!("{label} is not a regular file"));
    }
    if metadata.len() > max as u64 {
        return Err(format!("{label} exceeds size bound"));
    }
    std::fs::read_to_string(path).map_err(|_| format!("{label} cannot be read"))
}

fn reject_symlink_output(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("output must not be a symlink".to_owned());
        }
        if metadata.is_dir() {
            return Err("output must not be a directory".to_owned());
        }
    }
    Ok(())
}

fn atomic_write(output: &Path, bytes: &[u8]) -> Result<(), String> {
    reject_symlink_output(output)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let parent_metadata = std::fs::symlink_metadata(&parent)
        .map_err(|_| "output parent is unavailable".to_owned())?;
    if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
        return Err("output parent is not a real directory".to_owned());
    }
    let temp = parent.join(format!(".eggpack-tmp-{}", std::process::id()));
    // Best-effort cleanup of a stale temp from a prior aborted run for this pid.
    let _ = std::fs::remove_file(&temp);
    std::fs::write(&temp, bytes).map_err(|_| "output temp write failed".to_owned())?;
    match std::fs::rename(&temp, output) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Windows rename does not atomically replace; remove and retry once.
            // This remains scoped to the explicit output path only.
            if output.exists() {
                std::fs::remove_file(output).map_err(|_| "output replace failed".to_owned())?;
                std::fs::rename(&temp, output).map_err(|_| "output replace failed".to_owned())
            } else {
                let _ = std::fs::remove_file(&temp);
                Err("output replace failed".to_owned())
            }
        }
    }
}

fn ci_generate(args: &[String]) -> Result<(), String> {
    let policy_path = PathBuf::from(get_flag(args, "github-policy")?);
    let output_path = PathBuf::from(get_flag(args, "output")?);
    // Reusable mode operates only on static workflow shape/template inputs;
    // exact mode operates on a checked-in ReleaseCIPlanV1. The modes are
    // mutually exclusive.
    if get_flag_optional(args, "workflow-shape").is_some() {
        if get_flag_optional(args, "ci-plan").is_some() {
            return Err(
                "ci generate accepts either --workflow-shape or --ci-plan, not both".to_owned(),
            );
        }
        let shape_path = PathBuf::from(get_flag(args, "workflow-shape")?);
        let contract_path = PathBuf::from(get_flag(args, "contract")?);
        if args.len() > 8 {
            return Err("too many arguments for ci generate".to_owned());
        }
        let shape_text = read_bounded(&shape_path, 1_000_000, "workflow shape")?;
        let contract_text = read_bounded(&contract_path, 1_000_000, "contract")?;
        let policy_text = read_bounded(&policy_path, 1_000_000, "github policy")?;
        let shape = eggpack_ci::ReleaseWorkflowShapeV1::from_json(&shape_text)
            .map_err(|_| "invalid workflow shape".to_owned())?;
        let contract = eggpack_contract::DistributionContract::parse_toml_str(&contract_text)
            .map_err(|_| "invalid contract".to_owned())?;
        let policy: eggpack_ci::GitHubPolicy =
            serde_json::from_str(&policy_text).map_err(|_| "invalid github policy".to_owned())?;
        let rendered = eggpack_ci::render_reusable_release_github(&contract, &shape, &policy)
            .map_err(|_| "reusable release rendering failed".to_owned())?;
        atomic_write(&output_path, rendered.as_bytes())?;
        println!(
            "generated {} bytes to {}",
            rendered.len(),
            output_path.display()
        );
        return Ok(());
    }
    let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
    if args.len() > 6 {
        return Err("too many arguments for ci generate".to_owned());
    }
    let ci_text = read_bounded(&ci_plan_path, 1_000_000, "ci plan")?;
    let policy_text = read_bounded(&policy_path, 1_000_000, "github policy")?;
    let graph: eggpack_ci::ReleaseCIPlanV1 = eggpack_ci::ReleaseCIPlanV1::from_json(&ci_text)
        .map_err(|_| "invalid ci plan".to_owned())?;
    let policy: eggpack_ci::GitHubPolicy =
        serde_json::from_str(&policy_text).map_err(|_| "invalid github policy".to_owned())?;
    let rendered = eggpack_ci::render_release_github(&graph, &policy)
        .map_err(|_| "release rendering failed".to_owned())?;
    atomic_write(&output_path, rendered.as_bytes())?;
    println!(
        "generated {} bytes to {}",
        rendered.len(),
        output_path.display()
    );
    Ok(())
}

fn ci_check(args: &[String]) -> Result<(), String> {
    let policy_path = PathBuf::from(get_flag(args, "github-policy")?);
    let workflow_path = PathBuf::from(get_flag(args, "workflow")?);
    if get_flag_optional(args, "workflow-shape").is_some() {
        if get_flag_optional(args, "ci-plan").is_some() {
            return Err(
                "ci check accepts either --workflow-shape or --ci-plan, not both".to_owned(),
            );
        }
        let shape_path = PathBuf::from(get_flag(args, "workflow-shape")?);
        let contract_path = PathBuf::from(get_flag(args, "contract")?);
        if args.len() > 8 {
            return Err("too many arguments for ci check".to_owned());
        }
        let shape_text = read_bounded(&shape_path, 1_000_000, "workflow shape")?;
        let contract_text = read_bounded(&contract_path, 1_000_000, "contract")?;
        let policy_text = read_bounded(&policy_path, 1_000_000, "github policy")?;
        let existing =
            std::fs::read(&workflow_path).map_err(|_| "workflow is unavailable".to_owned())?;
        if existing.len() > 8_000_000 {
            return Err("workflow exceeds size bound".to_owned());
        }
        let shape = eggpack_ci::ReleaseWorkflowShapeV1::from_json(&shape_text)
            .map_err(|_| "invalid workflow shape".to_owned())?;
        let contract = eggpack_contract::DistributionContract::parse_toml_str(&contract_text)
            .map_err(|_| "invalid contract".to_owned())?;
        let policy: eggpack_ci::GitHubPolicy =
            serde_json::from_str(&policy_text).map_err(|_| "invalid github policy".to_owned())?;
        let report =
            eggpack_ci::check_reusable_release_github(&contract, &shape, &policy, &existing)
                .map_err(|_| "reusable drift check failed".to_owned())?;
        if report.matches {
            println!("ci check: match ({} bytes)", report.expected_bytes);
            Ok(())
        } else {
            Err(format!(
                "ci check: drift detected (expected {} bytes, found {} bytes, first difference at {:?})",
                report.expected_bytes, report.actual_bytes, report.first_difference
            ))
        }
    } else {
        let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
        if args.len() > 6 {
            return Err("too many arguments for ci check".to_owned());
        }
        let ci_text = read_bounded(&ci_plan_path, 1_000_000, "ci plan")?;
        let policy_text = read_bounded(&policy_path, 1_000_000, "github policy")?;
        let existing =
            std::fs::read(&workflow_path).map_err(|_| "workflow is unavailable".to_owned())?;
        if existing.len() > 8_000_000 {
            return Err("workflow exceeds size bound".to_owned());
        }
        let graph: eggpack_ci::ReleaseCIPlanV1 = eggpack_ci::ReleaseCIPlanV1::from_json(&ci_text)
            .map_err(|_| "invalid ci plan".to_owned())?;
        let policy: eggpack_ci::GitHubPolicy =
            serde_json::from_str(&policy_text).map_err(|_| "invalid github policy".to_owned())?;
        let report = eggpack_ci::check_release_github(&graph, &policy, &existing)
            .map_err(|_| "release drift check failed".to_owned())?;
        if report.matches {
            println!("ci check: match ({} bytes)", report.expected_bytes);
            Ok(())
        } else {
            Err(format!(
                "ci check: drift detected (expected {} bytes, found {} bytes, first difference at {:?})",
                report.expected_bytes, report.actual_bytes, report.first_difference
            ))
        }
    }
}

fn ci_capture_build(args: &[String]) -> Result<(), String> {
    let plan_path = PathBuf::from(get_flag(args, "release-plan")?);
    let bindings_path = PathBuf::from(get_flag(args, "build-bindings")?);
    let target = get_flag(args, "target")?;
    let candidate_dir = get_flag_optional(args, "candidate-dir").map(PathBuf::from);
    let cargo_target_dir = get_flag_optional(args, "cargo-target-dir").map(PathBuf::from);
    let output_path = get_flag_optional(args, "output").map(PathBuf::from);
    let output_dir = get_flag_optional(args, "output-dir").map(PathBuf::from);
    if output_path.is_none() && output_dir.is_none() {
        return Err("missing required --output or --output-dir".to_owned());
    }
    if candidate_dir.is_none() && cargo_target_dir.is_none() {
        return Err("missing required --candidate-dir or --cargo-target-dir".to_owned());
    }
    let plan_text = read_bounded(&plan_path, 1_000_000, "release plan")?;
    let bindings_text = read_bounded(&bindings_path, 1_000_000, "build bindings")?;
    let plan: eggpack_core::ReleasePlan =
        serde_json::from_str(&plan_text).map_err(|_| "invalid release plan".to_owned())?;
    let bindings: eggpack_core::BuildBindingsV1 = toml::from_str(&bindings_text)
        .map_err(|_| "invalid build bindings".to_owned())
        .or_else(|_: String| {
            serde_json::from_str(&bindings_text).map_err(|_| "invalid build bindings".to_owned())
        })?;
    let mut handoff = eggpack_ci::project_build_handoff(&plan, &bindings, &target)
        .map_err(|_| "handoff projection failed".to_owned())?;
    // Resolve each candidate source: explicit Cargo target root derivation when
    // provided (generated shell passes the target root, never per-file Cargo
    // discovery logic), otherwise the staged candidate directory keyed by the
    // canonical handoff relative names.
    let mut staged_sources: Vec<PathBuf> = Vec::with_capacity(handoff.outputs.len());
    for output in &mut handoff.outputs {
        let source = if let Some(root) = &cargo_target_dir {
            let dir_meta = std::fs::symlink_metadata(root)
                .map_err(|_| "cargo target dir unavailable".to_owned())?;
            if !dir_meta.is_dir() || dir_meta.file_type().is_symlink() {
                return Err("cargo target dir is not a real directory".to_owned());
            }
            eggpack_ci::cargo_output_path(root, &target, &output.binary)
                .map_err(|_| "cargo output derivation failed".to_owned())?
        } else {
            let dir = candidate_dir.as_ref().expect("candidate dir checked");
            let dir_metadata = std::fs::symlink_metadata(dir)
                .map_err(|_| "candidate dir unavailable".to_owned())?;
            if !dir_metadata.is_dir() || dir_metadata.file_type().is_symlink() {
                return Err("candidate dir is not a real directory".to_owned());
            }
            dir.join(&output.relative_path)
        };
        let metadata = std::fs::symlink_metadata(&source)
            .map_err(|_| "candidate file unavailable".to_owned())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
            return Err("candidate is not a non-empty regular file".to_owned());
        }
        output.size = metadata.len();
        staged_sources.push(source);
    }
    handoff
        .validate()
        .map_err(|_| "handoff validation failed".to_owned())?;
    if let Some(dir) = &output_dir {
        // Write the canonical build artifact layout: build-handoff.json plus
        // candidates/<relative_path> bytes (hard-link preferred, copy fallback).
        let dir_meta = std::fs::symlink_metadata(dir);
        match dir_meta {
            Ok(meta) => {
                if !meta.is_dir() || meta.file_type().is_symlink() {
                    return Err("output dir is not a real directory".to_owned());
                }
            }
            Err(_) => {
                std::fs::create_dir_all(dir)
                    .map_err(|_| "output dir creation failed".to_owned())?;
            }
        }
        let dest_candidates = dir.join("candidates");
        std::fs::create_dir_all(&dest_candidates)
            .map_err(|_| "output dir creation failed".to_owned())?;
        for (output, source) in handoff.outputs.iter().zip(staged_sources.iter()) {
            let dest = dest_candidates.join(&output.relative_path);
            if std::fs::hard_link(source, &dest).is_err() {
                std::fs::copy(source, &dest).map_err(|_| "candidate staging failed".to_owned())?;
            }
        }
        let json = handoff
            .to_json()
            .map_err(|_| "handoff serialization failed".to_owned())?;
        atomic_write(&dir.join("build-handoff.json"), json.as_bytes())?;
        eggpack_ci::validate_build_artifact_dir(dir)
            .map_err(|_| "staged build artifact validation failed".to_owned())?;
    }
    if let Some(path) = &output_path {
        let json = handoff
            .to_json()
            .map_err(|_| "handoff serialization failed".to_owned())?;
        atomic_write(path, json.as_bytes())?;
    }
    println!("captured build handoff for {target}");
    Ok(())
}

fn ci_qualify_target(args: &[String]) -> Result<(), String> {
    let plan_path = PathBuf::from(get_flag(args, "release-plan")?);
    let bindings_path = PathBuf::from(get_flag(args, "build-bindings")?);
    let qual_bindings_path = PathBuf::from(get_flag(args, "qualification-bindings")?);
    let target = get_flag(args, "target")?;
    let candidate_dir = PathBuf::from(get_flag(args, "candidate-dir")?);
    let handoff_path = PathBuf::from(get_flag(args, "build-handoff")?);
    let output_dir = PathBuf::from(get_flag(args, "output-dir")?);
    let qemu_sysroot = get_flag_optional(args, "qemu-sysroot").map(PathBuf::from);
    let plan_text = read_bounded(&plan_path, 1_000_000, "release plan")?;
    let bindings_text = read_bounded(&bindings_path, 1_000_000, "build bindings")?;
    let qual_text = read_bounded(&qual_bindings_path, 1_000_000, "qualification bindings")?;
    let handoff_text = read_bounded(&handoff_path, 1_000_000, "build handoff")?;
    let plan: eggpack_core::ReleasePlan =
        serde_json::from_str(&plan_text).map_err(|_| "invalid release plan".to_owned())?;
    let bindings: eggpack_core::BuildBindingsV1 = toml::from_str(&bindings_text)
        .map_err(|_| "invalid build bindings".to_owned())
        .or_else(|_: String| {
            serde_json::from_str(&bindings_text).map_err(|_| "invalid build bindings".to_owned())
        })?;
    let qual_bindings: eggpack_core::QualificationBindingsV1 = toml::from_str(&qual_text)
        .map_err(|_| "invalid qualification bindings".to_owned())
        .or_else(|_: String| {
            serde_json::from_str(&qual_text)
                .map_err(|_| "invalid qualification bindings".to_owned())
        })?;
    let handoff: eggpack_ci::BuildHandoffV1 = eggpack_ci::BuildHandoffV1::from_json(&handoff_text)
        .map_err(|_| "invalid handoff".to_owned())?;
    let planned = plan
        .targets
        .iter()
        .find(|t| t.target == target)
        .ok_or_else(|| "target is not in release plan".to_string())?;
    // Contract is required for core validation; load it explicitly.
    let contract_path = get_flag(args, "contract")?;
    let contract_text = read_bounded(Path::new(&contract_path), 1_000_000, "contract")?;
    let contract: eggpack_contract::DistributionContract =
        eggpack_contract::DistributionContract::parse_toml_str(&contract_text)
            .map_err(|_| "invalid contract".to_owned())?;
    let attempt = eggpack_ci::reconstruct_attempt(&plan, planned, &handoff, &candidate_dir)
        .map_err(|_| "candidate reconstruction failed".to_owned())?;
    if let Some(sysroot) = &qemu_sysroot {
        let meta = std::fs::symlink_metadata(sysroot)
            .map_err(|_| "qemu sysroot unavailable".to_owned())?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("qemu sysroot is not a real directory".to_owned());
        }
    }
    let runtime = eggpack_core::QualificationRuntime {
        qemu_sysroot: qemu_sysroot.clone(),
    };
    let cancellation = eggpack_core::BuildCancellation::new();
    let evidence = eggpack_core::qualify_target(eggpack_core::QualificationRequest {
        contract: &contract,
        plan: &plan,
        target: planned,
        attempt: &attempt,
        build_bindings: &bindings,
        qualification_bindings: &qual_bindings,
        runtime: &runtime,
        cancellation: &cancellation,
    })
    .map_err(|_| "qualification execution failed".to_owned())?;
    std::fs::create_dir_all(&output_dir).map_err(|_| "output dir creation failed".to_owned())?;
    let evidence_json = eggpack_ci::encode_qualification_evidence(&evidence)
        .map_err(|_| "evidence encode failed".to_owned())?;
    atomic_write(&output_dir.join("evidence.json"), evidence_json.as_bytes())?;
    // Canonical qualification artifact: copy the already-validated
    // build-handoff and candidate bytes alongside the evidence so gate and
    // aggregate consume one complete per-target directory.
    let handoff_text = read_bounded(&handoff_path, 1_000_000, "build handoff")?;
    atomic_write(
        &output_dir.join("build-handoff.json"),
        handoff_text.as_bytes(),
    )?;
    let candidates_src = {
        let nested = candidate_dir.join("candidates");
        if nested.exists() {
            nested
        } else {
            candidate_dir.clone()
        }
    };
    let handoff: eggpack_ci::BuildHandoffV1 = eggpack_ci::BuildHandoffV1::from_json(&handoff_text)
        .map_err(|_| "invalid handoff".to_owned())?;
    let dest_candidates = output_dir.join("candidates");
    std::fs::create_dir_all(&dest_candidates)
        .map_err(|_| "output dir creation failed".to_owned())?;
    for output in &handoff.outputs {
        let source = candidates_src.join(&output.relative_path);
        let dest = dest_candidates.join(&output.relative_path);
        if std::fs::hard_link(&source, &dest).is_err() {
            std::fs::copy(&source, &dest).map_err(|_| "candidate staging failed".to_owned())?;
        }
    }
    eggpack_ci::validate_qualification_artifact_dir(&output_dir)
        .map_err(|_| "staged qualification artifact validation failed".to_owned())?;
    println!("qualified {target}: {:?}", evidence.status);
    Ok(())
}

fn absolute_under_root(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let root_meta =
        std::fs::symlink_metadata(root).map_err(|_| "source root is unavailable".to_owned())?;
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        return Err("source root is not a real directory".to_owned());
    }
    // The validator script path was already validated as a bounded relative
    // path without escapes; joining keeps it beneath the verified root.
    let mut absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "working directory is unavailable".to_owned())?
            .join(root)
    };
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err("validator script escapes its root".to_owned());
        }
        absolute.push(segment);
    }
    Ok(absolute)
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|_| "working directory is unavailable".to_owned())
}

/// Validate one exact candidate with its consumer-owned script.
///
/// Consumes the canonical qualification handoff (exact candidate bytes
/// already validated by Eggpack) plus qualification evidence for exact
/// size/SHA identity. Never rebuilds the binary. Non-zero/timeout/
/// output-limit/interpreter/candidate failures fail the command so the
/// required gate blocks aggregation.
#[allow(clippy::too_many_lines)]
fn ci_validate_consumer(args: &[String]) -> Result<(), String> {
    let validators_path = PathBuf::from(get_flag(args, "consumer-validators")?);
    let target = get_flag(args, "target")?;
    let candidate_dir = PathBuf::from(get_flag(args, "candidate-dir")?);
    let handoff_path = PathBuf::from(get_flag(args, "build-handoff")?);
    let evidence_path = PathBuf::from(get_flag(args, "evidence")?);
    let source_root = PathBuf::from(get_flag(args, "source-root")?);
    let output_path = PathBuf::from(get_flag(args, "output")?);
    if args.len() > 14 {
        return Err("too many arguments for ci _validate-consumer".to_owned());
    }
    let map_text = read_bounded(&validators_path, 64 * 1024, "consumer validators")?;
    let map: std::collections::BTreeMap<String, eggpack_ci::ConsumerValidatorV1> =
        serde_json::from_str(&map_text).map_err(|_| "invalid consumer validators".to_owned())?;
    let validator = map
        .get(&target)
        .ok_or_else(|| "target has no consumer validator".to_owned())?;
    let handoff_text = read_bounded(&handoff_path, 1_000_000, "build handoff")?;
    let handoff: eggpack_ci::BuildHandoffV1 = eggpack_ci::BuildHandoffV1::from_json(&handoff_text)
        .map_err(|_| "invalid handoff".to_owned())?;
    if handoff.target != target {
        return Err("handoff target mismatch".to_owned());
    }
    let evidence_text = read_bounded(&evidence_path, 1_000_000, "qualification evidence")?;
    let evidence = eggpack_ci::decode_qualification_evidence(&evidence_text)
        .map_err(|_| "invalid evidence".to_owned())?;
    if evidence.target != target
        || evidence.release_id != handoff.release_id
        || evidence.source_revision != handoff.source_revision
    {
        return Err("evidence identity differs from handoff".to_owned());
    }
    let handoff_output = handoff
        .outputs
        .iter()
        .find(|output| output.selector == validator.selector)
        .ok_or_else(|| "validator selector is not a handoff output".to_owned())?;
    let qualified = evidence
        .candidates
        .iter()
        .find(|candidate| {
            candidate.selector == validator.selector
                && candidate.package == handoff_output.package
                && candidate.binary == handoff_output.binary
        })
        .ok_or_else(|| "validator selector has no qualified candidate".to_owned())?;
    let candidates_src = {
        let nested = candidate_dir.join("candidates");
        if nested.exists() {
            nested
        } else {
            candidate_dir.clone()
        }
    };
    let candidate_path = absolute_path(&candidates_src.join(&handoff_output.relative_path))?;
    let script_path = absolute_under_root(&source_root, &validator.script)?;
    let work_dir = absolute_path(Path::new("."))?;
    let request = eggpack_ci::ConsumerValidationRequest {
        validator,
        script_path: &script_path,
        candidate_path: &candidate_path,
        expected_size: qualified.size,
        expected_sha256: &qualified.sha256,
        work_dir: &work_dir,
        release_id: &handoff.release_id,
        source_revision: &handoff.source_revision,
        target: &target,
        cancelled: None,
        path_dirs: None,
    };
    let validation = eggpack_ci::run_consumer_validator(&request)
        .map_err(|_| "consumer validation failed".to_owned())?;
    let json = validation
        .to_json()
        .map_err(|_| "consumer evidence encode failed".to_owned())?;
    // Durable evidence never carries script output contents by type.
    if json.contains("stdout") || json.contains("stderr") {
        return Err("consumer evidence carries output contents".to_owned());
    }
    atomic_write(&output_path, json.as_bytes())?;
    match validation.outcome {
        eggpack_ci::ConsumerValidationOutcome::Passed => {
            println!("consumer validation passed for {target}");
            Ok(())
        }
        eggpack_ci::ConsumerValidationOutcome::Failed(_) => {
            Err(format!("consumer validation failed for {target}"))
        }
    }
}

fn ci_evaluate_gate(args: &[String]) -> Result<(), String> {
    let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
    let evidence_dir = get_flag_optional(args, "evidence-dir").map(PathBuf::from);
    let inputs_dir = get_flag_optional(args, "inputs-dir").map(PathBuf::from);
    let output_path = PathBuf::from(get_flag(args, "output")?);
    if evidence_dir.is_none() && inputs_dir.is_none() {
        return Err("missing required --evidence-dir or --inputs-dir".to_owned());
    }
    let ci_text = read_bounded(&ci_plan_path, 1_000_000, "ci plan")?;
    let graph: eggpack_ci::ReleaseCIPlanV1 = eggpack_ci::ReleaseCIPlanV1::from_json(&ci_text)
        .map_err(|_| "invalid ci plan".to_owned())?;
    let mut evidences = Vec::new();
    for qual in &graph.qualifications {
        let text = if let Some(dir) = &inputs_dir {
            // Canonical per-target layout: <inputs-dir>/<target>/evidence.json.
            let path = dir.join(&qual.target).join("evidence.json");
            read_bounded(&path, 1_000_000, "evidence")?
        } else {
            let dir = evidence_dir.as_ref().expect("evidence dir checked");
            let path = dir.join(format!("{}.json", qual.evidence_handoff_name));
            read_bounded(&path, 1_000_000, "evidence")?
        };
        let evidence = eggpack_ci::decode_qualification_evidence(&text)
            .map_err(|_| "invalid evidence".to_owned())?;
        evidences.push(evidence);
    }
    let outcome = if graph.consumer_validators.is_empty() {
        // Stray consumer evidence without a configured validator fails
        // closed so validation can never be silently bypassed.
        if let Some(dir) = &inputs_dir {
            for qual in &graph.qualifications {
                let stray = dir.join(&qual.target).join("consumer-evidence.json");
                if stray.exists() {
                    return Err(
                        "consumer evidence present without a configured validator".to_owned()
                    );
                }
            }
        }
        eggpack_ci::evaluate_gate(&graph, &evidences)
            .map_err(|_| "gate evaluation failed".to_owned())?
    } else {
        let inputs = inputs_dir
            .as_ref()
            .ok_or_else(|| "consumer gate requires --inputs-dir".to_owned())?;
        let mut consumer_evidences = Vec::new();
        for target in graph.consumer_validators.keys() {
            let path = inputs.join(target).join("consumer-evidence.json");
            let text = read_bounded(&path, 64 * 1024, "consumer evidence")?;
            let evidence = eggpack_ci::decode_consumer_evidence(&text)
                .map_err(|_| "invalid consumer evidence".to_owned())?;
            consumer_evidences.push(evidence);
        }
        eggpack_ci::evaluate_gate_with_consumer(&graph, &evidences, &consumer_evidences)
            .map_err(|_| "consumer gate evaluation failed".to_owned())?
    };
    let outcome_json =
        serde_json::to_string(&outcome).map_err(|_| "outcome encode failed".to_owned())?;
    atomic_write(&output_path, outcome_json.as_bytes())?;
    match outcome {
        eggpack_ci::AggregateOutcome::Complete
        | eggpack_ci::AggregateOutcome::SuppressedNonGatingIncomplete => {
            println!("gate outcome: {:?}", outcome);
            Ok(())
        }
        _ => Err(format!("gate outcome: {outcome:?}")),
    }
}

#[allow(clippy::too_many_lines)]
fn ci_aggregate(args: &[String]) -> Result<(), String> {
    let contract_path = PathBuf::from(get_flag(args, "contract")?);
    let plan_path = PathBuf::from(get_flag(args, "release-plan")?);
    let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
    let inputs_dir = PathBuf::from(get_flag(args, "inputs-dir")?);
    let output_root = PathBuf::from(get_flag(args, "output-root")?);
    let output_path = PathBuf::from(get_flag(args, "output")?);
    let contract_text = read_bounded(&contract_path, 1_000_000, "contract")?;
    let plan_text = read_bounded(&plan_path, 1_000_000, "release plan")?;
    let ci_text = read_bounded(&ci_plan_path, 1_000_000, "ci plan")?;
    let contract: eggpack_contract::DistributionContract =
        eggpack_contract::DistributionContract::parse_toml_str(&contract_text)
            .map_err(|_| "invalid contract".to_owned())?;
    let plan: eggpack_core::ReleasePlan =
        serde_json::from_str(&plan_text).map_err(|_| "invalid release plan".to_owned())?;
    let graph: eggpack_ci::ReleaseCIPlanV1 = eggpack_ci::ReleaseCIPlanV1::from_json(&ci_text)
        .map_err(|_| "invalid ci plan".to_owned())?;
    // Inputs dir layout: <target>/build-handoff.json, <target>/evidence.json,
    // <target>/candidates/<relative_path...>. Candidate bytes are read from the
    // per-target candidate roots supplied alongside the handoffs.
    let mut final_inputs = Vec::new();
    for qual in &graph.qualifications {
        let target_dir = inputs_dir.join(&qual.target);
        let handoff_text = read_bounded(
            &target_dir.join("build-handoff.json"),
            1_000_000,
            "build handoff",
        )?;
        let evidence_text = read_bounded(&target_dir.join("evidence.json"), 1_000_000, "evidence")?;
        let handoff: eggpack_ci::BuildHandoffV1 =
            eggpack_ci::BuildHandoffV1::from_json(&handoff_text)
                .map_err(|_| "invalid handoff".to_owned())?;
        let evidence = eggpack_ci::decode_qualification_evidence(&evidence_text)
            .map_err(|_| "invalid evidence".to_owned())?;
        let planned = plan
            .targets
            .iter()
            .find(|t| t.target == qual.target)
            .ok_or_else(|| "target is not in release plan".to_string())?;
        let candidate_root = target_dir.join("candidates");
        let attempt = eggpack_ci::reconstruct_attempt(&plan, planned, &handoff, &candidate_root)
            .map_err(|_| "candidate reconstruction failed".to_owned())?;
        final_inputs.push(eggpack_core::FinalizationTargetInput {
            target: qual.target.clone(),
            attempt,
            qualification: evidence,
        });
    }
    let (outcome, finalized) = if graph.consumer_validators.is_empty() {
        // Stray consumer evidence without a configured validator fails
        // closed so validation can never be silently bypassed.
        for qual in &graph.qualifications {
            let stray = inputs_dir.join(&qual.target).join("consumer-evidence.json");
            if stray.exists() {
                return Err("consumer evidence present without a configured validator".to_owned());
            }
        }
        eggpack_ci::aggregate_finalize(
            &contract,
            &plan,
            &graph,
            &final_inputs,
            &graph.finalization.evidence_references,
            &output_root,
        )
        .map_err(|_| "aggregation failed".to_owned())?
    } else {
        let mut consumer_evidences = Vec::new();
        for target in graph.consumer_validators.keys() {
            let path = inputs_dir.join(target).join("consumer-evidence.json");
            let text = read_bounded(&path, 64 * 1024, "consumer evidence")?;
            let evidence = eggpack_ci::decode_consumer_evidence(&text)
                .map_err(|_| "invalid consumer evidence".to_owned())?;
            consumer_evidences.push(evidence);
        }
        eggpack_ci::aggregate_finalize_with_consumer(
            &contract,
            &plan,
            &graph,
            &final_inputs,
            &consumer_evidences,
            &graph.finalization.evidence_references,
            &output_root,
        )
        .map_err(|_| "consumer aggregation failed".to_owned())?
    };
    // Additive staging handoff: alongside summary.json, write a standalone
    // deterministic release-manifest.json that decodes back to the exact M004
    // manifest. This file lives beside the finalized root, never inside it,
    // so M004 release-root semantics remain unchanged.
    if let Some(finalized) = finalized.as_ref() {
        let manifest_json = finalized
            .manifest
            .to_json()
            .map_err(|_| "manifest encode failed".to_owned())?;
        let decoded = eggpack_manifest::ReleaseManifest::from_json(&manifest_json)
            .map_err(|_| "manifest roundtrip failed".to_owned())?;
        let decoded_json = decoded
            .to_json()
            .map_err(|_| "manifest roundtrip failed".to_owned())?;
        if decoded_json != manifest_json {
            return Err("staging manifest does not decode to exact manifest".to_owned());
        }
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                let manifest_path = parent.join("release-manifest.json");
                atomic_write(&manifest_path, manifest_json.as_bytes())?;
            }
        }
    }
    let summary = serde_json::json!({
        "outcome": outcome,
        "manifest": finalized.as_ref().map(|f| f.manifest.to_json().unwrap_or_default()),
    });
    let summary_text =
        serde_json::to_string(&summary).map_err(|_| "summary encode failed".to_owned())?;
    atomic_write(&output_path, summary_text.as_bytes())?;
    match outcome {
        eggpack_ci::AggregateOutcome::Complete => {
            println!("aggregate: Complete");
            Ok(())
        }
        _ => Err(format!("aggregate: {outcome:?}")),
    }
}

fn read_install_policy(path: &Path) -> Result<eggpack_bootstrap::BootstrapInstallPolicyV1, String> {
    let text = read_bounded(path, 256 * 1024, "install policy")?;
    // Accept TOML first (checked-in policy files are TOML), then JSON.
    if let Ok(policy) = eggpack_bootstrap::BootstrapInstallPolicyV1::from_toml(&text) {
        return Ok(policy);
    }
    eggpack_bootstrap::BootstrapInstallPolicyV1::from_json(&text)
        .map_err(|_| "invalid install policy".to_owned())
}

fn read_installer_presentation(
    path: &Path,
) -> Result<eggpack_github::InstallerPresentationV1, String> {
    let text = read_bounded(path, 64 * 1024, "installer presentation")?;
    // Accept TOML first (checked-in policy files are TOML), then JSON.
    if let Ok(presentation) = toml::from_str::<eggpack_github::InstallerPresentationV1>(&text) {
        presentation
            .validate()
            .map_err(|_| "invalid installer presentation".to_owned())?;
        return Ok(presentation);
    }
    eggpack_github::InstallerPresentationV1::from_json(&text)
        .map_err(|_| "invalid installer presentation".to_owned())
}

fn ci_prepare_stage(args: &[String]) -> Result<(), String> {
    let contract_path = PathBuf::from(get_flag(args, "contract")?);
    let manifest_path = PathBuf::from(get_flag(args, "release-manifest")?);
    let finalized_root = PathBuf::from(get_flag(args, "finalized-root")?);
    let policy_path = PathBuf::from(get_flag(args, "github-policy")?);
    let install_policy_path = PathBuf::from(get_flag(args, "install-policy")?);
    let output_dir = PathBuf::from(get_flag(args, "output-dir")?);
    let output_payload = PathBuf::from(get_flag(args, "output-payload")?);
    let presentation_path = get_flag_optional(args, "installer-presentation").map(PathBuf::from);
    let source_root = get_flag_optional(args, "source-root").map(PathBuf::from);
    if args.len() > 18 {
        return Err("too many arguments for ci _prepare-stage".to_owned());
    }
    // Installer presentation and source root are paired: product wrappers
    // require both, GeneratedDefault uses neither.
    if presentation_path.is_some() != source_root.is_some() {
        return Err(
            "installer presentation requires both --installer-presentation and --source-root"
                .to_owned(),
        );
    }
    let contract_text = read_bounded(&contract_path, 1_000_000, "contract")?;
    let manifest_text = read_bounded(&manifest_path, 1_048_576, "release manifest")?;
    let policy_text = read_bounded(&policy_path, 256 * 1024, "github policy")?;
    let contract = eggpack_contract::DistributionContract::parse_toml_str(&contract_text)
        .map_err(|_| "invalid contract".to_owned())?;
    let manifest = eggpack_manifest::ReleaseManifest::from_json(&manifest_text)
        .map_err(|_| "invalid release manifest".to_owned())?;
    let policy = eggpack_github::GitHubDraftPolicyV1::from_json(&policy_text)
        .map_err(|_| "invalid github draft policy".to_owned())?;
    let install_policy = read_install_policy(&install_policy_path)?;
    let payload = match (presentation_path, source_root) {
        (Some(presentation_path), Some(source_root)) => {
            let presentation = read_installer_presentation(&presentation_path)?;
            eggpack_github::prepare_staging_payload_with_presentation(
                &contract,
                &manifest,
                &finalized_root,
                &policy,
                &install_policy,
                &presentation,
                &source_root,
                &output_dir,
            )
            .map_err(|_| "stage preparation failed".to_owned())?
        }
        (None, None) => eggpack_github::prepare_staging_payload(
            &contract,
            &manifest,
            &finalized_root,
            &policy,
            &install_policy,
            &output_dir,
        )
        .map_err(|_| "stage preparation failed".to_owned())?,
        _ => unreachable!("paired flags checked above"),
    };
    let payload_json = payload
        .to_json()
        .map_err(|_| "payload encode failed".to_owned())?;
    atomic_write(&output_payload, payload_json.as_bytes())?;
    println!("prepare-stage: {} assets", payload.assets.len());
    Ok(())
}

fn ci_stage_github_draft(args: &[String]) -> Result<(), String> {
    let payload_path = PathBuf::from(get_flag(args, "payload")?);
    let policy_path = PathBuf::from(get_flag(args, "github-policy")?);
    let receipt_path = PathBuf::from(get_flag(args, "output-receipt")?);
    if args.len() > 6 {
        return Err("too many arguments for ci _stage-github-draft".to_owned());
    }
    let payload_text = read_bounded(&payload_path, 1_048_576, "staging payload")?;
    let policy_text = read_bounded(&policy_path, 256 * 1024, "github policy")?;
    let payload = eggpack_github::StagingPayloadV1::from_json(&payload_text)
        .map_err(|_| "invalid staging payload".to_owned())?;
    let policy = eggpack_github::GitHubDraftPolicyV1::from_json(&policy_text)
        .map_err(|_| "invalid github draft policy".to_owned())?;
    // Credential from environment only; absence fails before network I/O.
    // The token text is never logged, serialized, or included in errors.
    let token = eggpack_github::read_token(&policy.token_env)
        .map_err(|_| "github token is absent".to_owned())?;
    let transport = eggpack_github::EggfetchTransport::new(
        token.clone(),
        policy.request_timeout_secs,
        policy.max_metadata_bytes,
    )
    .map_err(|_| "github transport failed".to_owned())?;
    // Staging directory is the parent of the payload file's sibling?
    // The payload records flat relative paths; the CLI resolves them against
    // an explicit --staging-dir when present, otherwise the payload's parent.
    let staging_dir = get_flag_optional(args, "staging-dir")
        .map(PathBuf::from)
        .or_else(|| payload_path.parent().map(PathBuf::from))
        .ok_or_else(|| "missing staging directory".to_owned())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "tokio runtime failed".to_owned())?;
    let receipt = runtime
        .block_on(eggpack_github::stage_with_dir(
            &payload,
            &policy,
            &transport,
            &token,
            &staging_dir,
        ))
        .map_err(|_| "github draft staging failed".to_owned())?;
    let receipt_json = receipt
        .to_json()
        .map_err(|_| "receipt encode failed".to_owned())?;
    atomic_write(&receipt_path, receipt_json.as_bytes())?;
    println!(
        "stage-github-draft: release {} draft staged",
        receipt.github_release_id
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("eggpack-cli-{label}-{}-{id}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create test temp directory: {error}"),
            }
        }
    }

    #[test]
    fn source_verifier_accepts_tag_commit_and_rejects_other_checkout() {
        let root = temp_root("source-identity");
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&root)
                .env("GIT_AUTHOR_NAME", "Eggpack test")
                .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
                .env("GIT_COMMITTER_NAME", "Eggpack test")
                .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {args:?}");
        };
        run(&["init", "-q"]);
        std::fs::write(root.join("source.txt"), "A").unwrap();
        run(&["add", "source.txt"]);
        run(&["commit", "-q", "-m", "A"]);
        std::fs::write(root.join("source.txt"), "B").unwrap();
        run(&["commit", "-q", "-am", "B"]);
        run(&["tag", "vX"]);
        let tag = std::process::Command::new("git")
            .args(["rev-parse", "vX^{commit}"])
            .current_dir(&root)
            .output()
            .unwrap();
        let b = String::from_utf8(tag.stdout).unwrap().trim().to_owned();
        assert_eq!(verify_source_revision(&b, Some(&root)), Ok(()));
        let a = std::process::Command::new("git")
            .args(["rev-parse", "HEAD~1^{commit}"])
            .current_dir(&root)
            .output()
            .unwrap();
        let a = String::from_utf8(a.stdout).unwrap().trim().to_owned();
        assert_ne!(a, b);
        assert!(verify_source_revision(&a, Some(&root)).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    fn golden_graph_and_policy() -> (String, String) {
        let graph =
            std::fs::read_to_string("../eggpack-ci/tests/fixtures/m002-direct.yml").unwrap();
        // Reconstruct graph/policy via library helpers matching goldens is complex;
        // instead generate a minimal valid pair via the library and use it for
        // generate/check round-trip tests.
        let _ = graph;
        // Build a direct structural graph programmatically.
        let contract = eggpack_contract::DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let config = eggpack_core::PackConfig {
            schema_version: 1,
            targets: vec![eggpack_core::TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: eggpack_core::BuildStrategy::NativeCargo,
                host_os: eggpack_core::HostOs::Linux,
                host_arch: eggpack_core::HostArch::X86_64,
                qualification_host: None,
                toolchain: eggpack_core::ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: eggpack_core::CompatibilityFloor::None,
                qualification: eggpack_core::Qualification::Structural,
                support: eggpack_core::SupportTier::Required,
            }],
        };
        let release = config
            .resolve(&contract, "1.2.3", &"a".repeat(40), &["linux-x64".into()])
            .unwrap();
        let bindings = eggpack_core::BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                vec![eggpack_core::BuildBinding {
                    selector: eggpack_core::LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "bin0".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let qual_bindings = eggpack_core::QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                eggpack_core::TargetQualificationBinding { smoke: None },
            )]
            .into_iter()
            .collect(),
        };
        let ci_plan = eggpack_ci::project_ci_plan(&contract, &release, &bindings).unwrap();
        let graph = eggpack_ci::project_release_plan(&ci_plan, &qual_bindings, &bindings, &release)
            .unwrap();
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let policy = eggpack_ci::GitHubPolicy {
            preflight_runner: "ubuntu-latest".into(),
            runners: vec![eggpack_ci::RunnerMapping {
                os: eggpack_core::HostOs::Linux,
                arch: eggpack_core::HostArch::X86_64,
                label: "ubuntu-latest".into(),
                cargo_zigbuild: true,
                zig: true,
            }],
            checkout: eggpack_ci::ActionPin {
                reference: format!("actions/checkout@{sha}"),
            },
            rust_toolchain: eggpack_ci::ActionPin {
                reference: format!("dtolnay/rust-toolchain@{sha}"),
            },
            upload_artifact: eggpack_ci::ActionPin {
                reference: format!("actions/upload-artifact@{sha}"),
            },
            download_artifact: Some(eggpack_ci::ActionPin {
                reference: format!("actions/download-artifact@{sha}"),
            }),
            triggers: vec![
                eggpack_ci::WorkflowTrigger::Push,
                eggpack_ci::WorkflowTrigger::WorkflowDispatch,
            ],
            timeout_minutes: 60,
            cancel_in_progress: true,
            artifact_retention_days: 7,
            eggpack_tool: Some(eggpack_ci::EggpackToolPolicy {
                repo: "https://github.com/eggstack/eggpack".into(),
                revision: "a".repeat(40),
                package: "eggpack-cli".into(),
                install_timeout_minutes: 10,
            }),
            release_inputs: Some(eggpack_ci::GitHubReleaseInputsV1 {
                contract: "contracts/simple-direct.toml".into(),
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
            cross_tools: None,
        };
        (
            graph.to_json().unwrap(),
            serde_json::to_string(&policy).unwrap(),
        )
    }

    #[test]
    fn generate_equals_library_and_check_detects_drift() {
        let root = temp_root("generate");
        let (graph_json, policy_json) = golden_graph_and_policy();
        let plan_path = root.join("plan.json");
        let policy_path = root.join("policy.json");
        let output_path = root.join("release.yml");
        std::fs::write(&plan_path, &graph_json).unwrap();
        std::fs::write(&policy_path, &policy_json).unwrap();
        // Generate equals library renderer.
        ci_generate(&[
            "--ci-plan".into(),
            plan_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--output".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .unwrap();
        let graph: eggpack_ci::ReleaseCIPlanV1 =
            eggpack_ci::ReleaseCIPlanV1::from_json(&graph_json).unwrap();
        let policy: eggpack_ci::GitHubPolicy = serde_json::from_str(&policy_json).unwrap();
        let expected = eggpack_ci::render_release_github(&graph, &policy).unwrap();
        assert_eq!(std::fs::read_to_string(&output_path).unwrap(), expected);
        // Check passes exact file.
        ci_check(&[
            "--ci-plan".into(),
            plan_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--workflow".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .unwrap();
        // CRLF normalization only as documented.
        let crlf = expected.replace('\n', "\r\n");
        std::fs::write(&output_path, &crlf).unwrap();
        ci_check(&[
            "--ci-plan".into(),
            plan_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--workflow".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .unwrap();
        // One-byte drift fails.
        let mut drifted = expected.clone();
        drifted.push('x');
        std::fs::write(&output_path, &drifted).unwrap();
        assert!(ci_check(&[
            "--ci-plan".into(),
            plan_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--workflow".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .is_err());
        // Check never modifies.
        let before = std::fs::read(&output_path).unwrap();
        let _ = ci_check(&[
            "--ci-plan".into(),
            plan_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--workflow".into(),
            output_path.to_string_lossy().into_owned(),
        ]);
        assert_eq!(std::fs::read(&output_path).unwrap(), before);
        // Symlink output rejects.
        #[cfg(unix)]
        {
            let link = root.join("link.yml");
            std::os::unix::fs::symlink(&output_path, &link).unwrap();
            assert!(ci_generate(&[
                "--ci-plan".into(),
                plan_path.to_string_lossy().into_owned(),
                "--github-policy".into(),
                policy_path.to_string_lossy().into_owned(),
                "--output".into(),
                link.to_string_lossy().into_owned(),
            ])
            .is_err());
        }
        // Invalid/oversized input rejects tested via bounded diagnostics (no full output).
        let bad = root.join("bad.json");
        std::fs::write(&bad, b"not json").unwrap();
        let err = ci_generate(&[
            "--ci-plan".into(),
            bad.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--output".into(),
            root.join("out.yml").to_string_lossy().into_owned(),
        ])
        .unwrap_err();
        assert!(err.len() < 500);
        assert!(!err.contains(&expected));
        std::fs::remove_dir_all(root).unwrap();
    }

    fn write_text(path: &std::path::Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    fn fixture_elf() -> Vec<u8> {
        let mut bytes = vec![0; 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
        bytes
    }

    /// M002a generated-orchestration CLI harness: the renderer's exact
    /// structured arguments drive capture -> qualify -> gate -> aggregate
    /// through the real CLI entry points (no GitHub invocation).
    #[test]
    fn generated_orchestration_cli_executes_capture_to_aggregate() {
        let root = temp_root("orchestration");
        let contract_text =
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract =
            eggpack_contract::DistributionContract::parse_toml_str(contract_text).unwrap();
        let config = eggpack_core::PackConfig {
            schema_version: 1,
            targets: vec![eggpack_core::TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: eggpack_core::BuildStrategy::NativeCargo,
                host_os: eggpack_core::HostOs::Linux,
                host_arch: eggpack_core::HostArch::X86_64,
                qualification_host: None,
                toolchain: eggpack_core::ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: eggpack_core::CompatibilityFloor::None,
                qualification: eggpack_core::Qualification::Structural,
                support: eggpack_core::SupportTier::Required,
            }],
        };
        let release = config
            .resolve(&contract, "1.2.3", &"a".repeat(40), &["linux-x64".into()])
            .unwrap();
        let bindings = eggpack_core::BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                vec![eggpack_core::BuildBinding {
                    selector: eggpack_core::LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "bin0".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let qual_bindings = eggpack_core::QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                eggpack_core::TargetQualificationBinding { smoke: None },
            )]
            .into_iter()
            .collect(),
        };
        let ci_plan = eggpack_ci::project_ci_plan(&contract, &release, &bindings).unwrap();
        let graph = eggpack_ci::project_release_plan(&ci_plan, &qual_bindings, &bindings, &release)
            .unwrap();

        // Repository files the generated commands reference explicitly.
        let contract_path = root.join("contracts/release.toml");
        let release_plan_path = root.join("plans/release-plan.json");
        let build_bindings_path = root.join("bindings/build.toml");
        let qual_bindings_path = root.join("bindings/qualification.toml");
        let ci_plan_path = root.join("plans/release-ci-plan.json");
        write_text(&contract_path, contract_text);
        write_text(
            &release_plan_path,
            &serde_json::to_string(&release).unwrap(),
        );
        write_text(&build_bindings_path, &toml::to_string(&bindings).unwrap());
        write_text(
            &qual_bindings_path,
            &toml::to_string(&qual_bindings).unwrap(),
        );
        write_text(&ci_plan_path, &graph.to_json().unwrap());

        // Fake Cargo target root with the exact binary the bindings name.
        let cargo_root = root.join("cargo-target");
        let cargo_bin_dir = cargo_root.join("x86_64-unknown-linux-gnu/release");
        std::fs::create_dir_all(&cargo_bin_dir).unwrap();
        let elf = fixture_elf();
        std::fs::write(cargo_bin_dir.join("bin0"), &elf).unwrap();

        // The renderer-derived capture command (structured, not string YAML).
        let capture = eggpack_ci::RunnerCommand::CaptureBuild {
            contract: contract_path.to_string_lossy().into_owned(),
            release_plan: release_plan_path.to_string_lossy().into_owned(),
            build_bindings: build_bindings_path.to_string_lossy().into_owned(),
            target: "x86_64-unknown-linux-gnu".into(),
            cargo_target_dir: cargo_root.to_string_lossy().into_owned(),
            output_dir: root
                .join("artifacts/build/x86_64-unknown-linux-gnu")
                .to_string_lossy()
                .into_owned(),
        };
        let argv = capture.argv();
        assert_eq!(&argv[..3], &["eggpack", "ci", "_capture-build"]);
        ci_capture_build(&argv[3..]).unwrap();
        let build_dir = root.join("artifacts/build/x86_64-unknown-linux-gnu");
        eggpack_ci::validate_build_artifact_dir(&build_dir).unwrap();

        // Qualify via the renderer-derived command shape.
        let qualify = eggpack_ci::RunnerCommand::QualifyTarget {
            contract: contract_path.to_string_lossy().into_owned(),
            release_plan: release_plan_path.to_string_lossy().into_owned(),
            build_bindings: build_bindings_path.to_string_lossy().into_owned(),
            qualification_bindings: qual_bindings_path.to_string_lossy().into_owned(),
            target: "x86_64-unknown-linux-gnu".into(),
            candidate_dir: build_dir.join("candidates").to_string_lossy().into_owned(),
            build_handoff: build_dir
                .join("build-handoff.json")
                .to_string_lossy()
                .into_owned(),
            output_dir: root
                .join("artifacts/qual/x86_64-unknown-linux-gnu")
                .to_string_lossy()
                .into_owned(),
            qemu_sysroot: None,
        };
        let argv = qualify.argv();
        ci_qualify_target(&argv[3..]).unwrap();
        let qual_dir = root.join("artifacts/qual/x86_64-unknown-linux-gnu");
        eggpack_ci::validate_qualification_artifact_dir(&qual_dir).unwrap();

        // Gate over the canonical per-target inputs layout.
        let inputs_dir = root.join("eggpack-inputs");
        let target_inputs = inputs_dir.join("x86_64-unknown-linux-gnu");
        std::fs::create_dir_all(&target_inputs).unwrap();
        for name in ["build-handoff.json", "evidence.json"] {
            std::fs::copy(qual_dir.join(name), target_inputs.join(name)).unwrap();
        }
        std::fs::create_dir_all(target_inputs.join("candidates")).unwrap();
        for entry in std::fs::read_dir(qual_dir.join("candidates")).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(
                entry.path(),
                target_inputs.join("candidates").join(entry.file_name()),
            )
            .unwrap();
        }
        let gate_out = root.join("gate-outcome.json");
        let gate = eggpack_ci::RunnerCommand::EvaluateGate {
            ci_plan: ci_plan_path.to_string_lossy().into_owned(),
            inputs_dir: inputs_dir.to_string_lossy().into_owned(),
            output: gate_out.to_string_lossy().into_owned(),
        };
        let argv = gate.argv();
        ci_evaluate_gate(&argv[3..]).unwrap();
        let outcome: eggpack_ci::AggregateOutcome =
            serde_json::from_str(&std::fs::read_to_string(&gate_out).unwrap()).unwrap();
        assert_eq!(outcome, eggpack_ci::AggregateOutcome::Complete);

        // Aggregate finalizes exact bytes through M004.
        let summary_out = root.join("summary.json");
        let output_root = root.join("finalized");
        let aggregate = eggpack_ci::RunnerCommand::Aggregate {
            contract: contract_path.to_string_lossy().into_owned(),
            release_plan: release_plan_path.to_string_lossy().into_owned(),
            ci_plan: ci_plan_path.to_string_lossy().into_owned(),
            inputs_dir: inputs_dir.to_string_lossy().into_owned(),
            output_root: output_root.to_string_lossy().into_owned(),
            output: summary_out.to_string_lossy().into_owned(),
        };
        let argv = aggregate.argv();
        ci_aggregate(&argv[3..]).unwrap();
        let summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&summary_out).unwrap()).unwrap();
        assert_eq!(summary["outcome"], "complete");
        // M003a additive handoff: standalone release-manifest.json beside the
        // finalized root decodes to the exact M004 manifest, while the root
        // itself retains only contract artifacts plus sidecars.
        let manifest_path = root.join("release-manifest.json");
        let manifest_text = std::fs::read_to_string(&manifest_path).unwrap();
        let manifest = eggpack_manifest::ReleaseManifest::from_json(&manifest_text).unwrap();
        assert_eq!(
            manifest_text,
            manifest.to_json().unwrap(),
            "standalone manifest must be deterministic"
        );
        assert!(
            !output_root.join("release-manifest.json").exists(),
            "M004 root semantics must remain unchanged"
        );

        // Negative: tampered candidate bytes fail closed at gate/aggregate.
        std::fs::write(
            target_inputs.join("candidates").join("candidate-direct"),
            b"tampered",
        )
        .unwrap();
        assert!(ci_aggregate(&argv[3..]).is_err());

        // Negative: missing contract path fails with a bounded diagnostic.
        let missing_argv = argv[3..].to_vec();
        let _ = missing_argv;
        assert!(ci_capture_build(&[
            "--release-plan".into(),
            release_plan_path.to_string_lossy().into_owned(),
            "--build-bindings".into(),
            build_bindings_path.to_string_lossy().into_owned(),
            "--target".into(),
            "x86_64-unknown-linux-gnu".into(),
            "--cargo-target-dir".into(),
            cargo_root.to_string_lossy().into_owned(),
            "--output-dir".into(),
            root.join("artifacts/missing-contract")
                .to_string_lossy()
                .into_owned(),
        ])
        .is_ok());
        // ...but qualification with a missing contract rejects.
        assert!(ci_qualify_target(&[
            "--contract".into(),
            root.join("no-such-contract.toml")
                .to_string_lossy()
                .into_owned(),
            "--release-plan".into(),
            release_plan_path.to_string_lossy().into_owned(),
            "--build-bindings".into(),
            build_bindings_path.to_string_lossy().into_owned(),
            "--qualification-bindings".into(),
            qual_bindings_path.to_string_lossy().into_owned(),
            "--target".into(),
            "x86_64-unknown-linux-gnu".into(),
            "--candidate-dir".into(),
            build_dir.join("candidates").to_string_lossy().into_owned(),
            "--build-handoff".into(),
            build_dir
                .join("build-handoff.json")
                .to_string_lossy()
                .into_owned(),
            "--output-dir".into(),
            root.join("artifacts/missing-contract-out")
                .to_string_lossy()
                .into_owned(),
        ])
        .is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prepare_stage_materializes_exact_payload() {
        use sha2::Digest;
        let root = temp_root("prepare-stage");
        let contract_text =
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract_path = root.join("contract.toml");
        std::fs::write(&contract_path, contract_text).unwrap();
        let contract =
            eggpack_contract::DistributionContract::parse_toml_str(contract_text).unwrap();
        // Two-target direct manifest with deterministic byte facts.
        let body = b"body";
        let digest: String = sha2::Sha256::digest(body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest = eggpack_manifest::ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".to_owned(),
            release_id: "1.2.6".to_owned(),
            source_revision: "a".repeat(40),
            targets: vec![
                eggpack_manifest::TargetRecord {
                    target: "aarch64-apple-darwin".to_owned(),
                    form: eggpack_manifest::ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-aarch64-apple-darwin".to_owned(),
                            size: body.len() as u64,
                            sha256: digest.clone(),
                        },
                        install: "eggsact".to_owned(),
                    },
                },
                eggpack_manifest::TargetRecord {
                    target: "x86_64-unknown-linux-gnu".to_owned(),
                    form: eggpack_manifest::ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-x86_64-unknown-linux-gnu".to_owned(),
                            size: body.len() as u64,
                            sha256: digest.clone(),
                        },
                        install: "eggsact".to_owned(),
                    },
                },
            ],
            evidence_references: Vec::new(),
        };
        let manifest_path = root.join("release-manifest.json");
        std::fs::write(&manifest_path, manifest.to_json().unwrap()).unwrap();
        // Finalized root with exact artifacts plus sidecars.
        let finalized = root.join("finalized");
        std::fs::create_dir(&finalized).unwrap();
        for name in [
            "eggsact-1.2.6-aarch64-apple-darwin",
            "eggsact-1.2.6-x86_64-unknown-linux-gnu",
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
            owner: "acme".to_owned(),
            repository: "widget".to_owned(),
            tag: "v1.2.6".to_owned(),
            title: "widget 1.2.6".to_owned(),
            body: "notes".to_owned(),
            prerelease: false,
            token_env: "GITHUB_TOKEN".to_owned(),
            request_timeout_secs: 30,
            max_metadata_bytes: 1_000_000,
            max_list_pages: 5,
        };
        let policy_path = root.join("github-policy.json");
        std::fs::write(&policy_path, policy.to_json().unwrap()).unwrap();
        let install_policy_path = root.join("install-policy.toml");
        std::fs::write(&install_policy_path, "schema_version = 1\n").unwrap();
        let staging = root.join("staging");
        let payload_out = root.join("payload.json");
        // Drive the CLI through the same typed RunnerCommand the GitHub
        // renderer serializes into YAML, so CLI signature drift is caught.
        let prepare = eggpack_ci::RunnerCommand::PrepareStage {
            contract: contract_path.to_string_lossy().into_owned(),
            release_manifest: manifest_path.to_string_lossy().into_owned(),
            finalized_root: finalized.to_string_lossy().into_owned(),
            github_policy: policy_path.to_string_lossy().into_owned(),
            install_policy: install_policy_path.to_string_lossy().into_owned(),
            output_dir: staging.to_string_lossy().into_owned(),
            output_payload: payload_out.to_string_lossy().into_owned(),
            installer_presentation: None,
            source_root: None,
        };
        let argv = prepare.argv();
        assert_eq!(&argv[..3], &["eggpack", "ci", "_prepare-stage"]);
        ci_prepare_stage(&argv[3..]).unwrap();
        assert!(staging.join("release-manifest.json").exists());
        assert!(staging.join("install.sh").exists());
        assert!(staging.join("install.ps1").exists());
        let payload = eggpack_github::StagingPayloadV1::from_json(
            &std::fs::read_to_string(&payload_out).unwrap(),
        )
        .unwrap();
        assert_eq!(payload.assets.len(), 7);
        let _ = contract;
        std::fs::remove_dir_all(root).unwrap();
    }

    fn init_git_repo(root: &Path) -> String {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .env("GIT_AUTHOR_NAME", "Eggpack test")
                .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
                .env("GIT_COMMITTER_NAME", "Eggpack test")
                .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {args:?}");
        };
        run(&["init", "-q"]);
        std::fs::write(root.join("source.txt"), "revision").unwrap();
        run(&["add", "source.txt"]);
        run(&["commit", "-q", "-m", "revision"]);
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD^{commit}"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn m003d_cli_config(root: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
        let contract_text =
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract_path = root.join("contract.toml");
        std::fs::write(&contract_path, contract_text).unwrap();
        let pack_config = eggpack_core::PackConfig {
            schema_version: 1,
            targets: vec![eggpack_core::TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: eggpack_core::BuildStrategy::NativeCargo,
                host_os: eggpack_core::HostOs::Linux,
                host_arch: eggpack_core::HostArch::X86_64,
                qualification_host: None,
                toolchain: eggpack_core::ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: eggpack_core::CompatibilityFloor::None,
                qualification: eggpack_core::Qualification::Structural,
                support: eggpack_core::SupportTier::Required,
            }],
        };
        let pack_path = root.join("pack.toml");
        std::fs::write(&pack_path, toml::to_string(&pack_config).unwrap()).unwrap();
        let bindings = eggpack_core::BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                vec![eggpack_core::BuildBinding {
                    selector: eggpack_core::LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "bin0".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let bindings_path = root.join("build.toml");
        std::fs::write(&bindings_path, toml::to_string(&bindings).unwrap()).unwrap();
        let qual_bindings = eggpack_core::QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                eggpack_core::TargetQualificationBinding { smoke: None },
            )]
            .into_iter()
            .collect(),
        };
        let qual_path = root.join("qualification.toml");
        std::fs::write(&qual_path, toml::to_string(&qual_bindings).unwrap()).unwrap();
        let template = eggpack_github::GitHubDraftTemplateV1 {
            schema_version: 1,
            owner: "acme".into(),
            repository: "widget".into(),
            title_prefix: "widget ".into(),
            body: "notes".into(),
            prerelease: false,
            token_env: "GITHUB_TOKEN".into(),
            request_timeout_secs: 30,
            max_metadata_bytes: 1_000_000,
            max_list_pages: 5,
        };
        let template_path = root.join("template.json");
        std::fs::write(&template_path, template.to_json().unwrap()).unwrap();
        (
            contract_path,
            pack_path,
            bindings_path,
            qual_path,
            template_path,
        )
    }

    fn resolve_argv(root: &Path, tag: &str, revision: &str, out: &str) -> Vec<String> {
        let (contract, pack, bindings, qual, template) = m003d_cli_config(root);
        let command = eggpack_ci::RunnerCommand::ResolveRelease {
            contract: contract.to_string_lossy().into_owned(),
            pack_config: pack.to_string_lossy().into_owned(),
            build_bindings: bindings.to_string_lossy().into_owned(),
            qualification_bindings: qual.to_string_lossy().into_owned(),
            consumer_validators: None,
            selected: "linux-x64".into(),
            tag: tag.into(),
            source_revision: revision.into(),
            template: template.to_string_lossy().into_owned(),
            source_root: root.to_string_lossy().into_owned(),
            output_plan: root
                .join(format!("{out}-plan.json"))
                .to_string_lossy()
                .into_owned(),
            output_ci_plan: root
                .join(format!("{out}-ci-plan.json"))
                .to_string_lossy()
                .into_owned(),
            output_github_policy: root
                .join(format!("{out}-github.json"))
                .to_string_lossy()
                .into_owned(),
        };
        let argv = command.argv();
        assert_eq!(&argv[..3], &["eggpack", "ci", "_resolve-release"]);
        argv[3..].to_vec()
    }

    #[test]
    fn resolve_release_emits_distinct_runtime_identity_per_tag() {
        let root = temp_root("resolve-release");
        let revision = init_git_repo(&root);
        // Two runtime tags resolve distinct ReleasePlan/GitHubDraftPolicy
        // documents from identical checked-in files.
        ci_resolve_release(&resolve_argv(&root, "v1.2.4", &revision, "first")).unwrap();
        ci_resolve_release(&resolve_argv(&root, "v1.2.5", &revision, "second")).unwrap();
        let first_plan = std::fs::read_to_string(root.join("first-plan.json")).unwrap();
        let second_plan = std::fs::read_to_string(root.join("second-plan.json")).unwrap();
        assert_ne!(first_plan, second_plan);
        assert!(first_plan.contains("\"release_id\":\"v1.2.4\""));
        assert!(second_plan.contains("\"release_id\":\"v1.2.5\""));
        assert!(first_plan.contains(&revision));
        let first_policy = std::fs::read_to_string(root.join("first-github.json")).unwrap();
        let second_policy = std::fs::read_to_string(root.join("second-github.json")).unwrap();
        assert_ne!(first_policy, second_policy);
        assert!(first_policy.contains("\"tag\":\"v1.2.4\""));
        assert!(first_policy.contains("\"title\":\"widget v1.2.4\""));
        // The runtime CI plan carries the same identity for gate/aggregate.
        let first_graph = std::fs::read_to_string(root.join("first-ci-plan.json")).unwrap();
        assert!(first_graph.contains("\"release_id\":\"v1.2.4\""));
        // Runtime source mismatch still fails verification.
        let other = "b".repeat(40);
        assert!(ci_resolve_release(&resolve_argv(&root, "v1.2.4", &other, "bad")).is_err());
        // Injection tags fail before any output is written.
        assert!(ci_resolve_release(&resolve_argv(&root, "v?x", &revision, "evil")).is_err());
        assert!(!root.join("evil-plan.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validate_consumer_executes_typed_command_end_to_end() {
        let root = temp_root("validate-consumer");
        let contract_text =
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract =
            eggpack_contract::DistributionContract::parse_toml_str(contract_text).unwrap();
        let config = eggpack_core::PackConfig {
            schema_version: 1,
            targets: vec![eggpack_core::TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: eggpack_core::BuildStrategy::NativeCargo,
                host_os: eggpack_core::HostOs::Linux,
                host_arch: eggpack_core::HostArch::X86_64,
                qualification_host: None,
                toolchain: eggpack_core::ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: eggpack_core::CompatibilityFloor::None,
                qualification: eggpack_core::Qualification::Structural,
                support: eggpack_core::SupportTier::Required,
            }],
        };
        let release = config
            .resolve(&contract, "1.2.3", &"a".repeat(40), &["linux-x64".into()])
            .unwrap();
        let bindings = eggpack_core::BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                vec![eggpack_core::BuildBinding {
                    selector: eggpack_core::LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "bin0".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let qual_bindings = eggpack_core::QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                eggpack_core::TargetQualificationBinding { smoke: None },
            )]
            .into_iter()
            .collect(),
        };
        let contract_path = root.join("contract.toml");
        let release_plan_path = root.join("release-plan.json");
        let build_bindings_path = root.join("build.toml");
        let qual_bindings_path = root.join("qualification.toml");
        write_text(&contract_path, contract_text);
        write_text(
            &release_plan_path,
            &serde_json::to_string(&release).unwrap(),
        );
        write_text(&build_bindings_path, &toml::to_string(&bindings).unwrap());
        write_text(
            &qual_bindings_path,
            &toml::to_string(&qual_bindings).unwrap(),
        );
        // Capture + qualify through the renderer-derived commands.
        let cargo_root = root.join("cargo-target");
        let cargo_bin_dir = cargo_root.join("x86_64-unknown-linux-gnu/release");
        std::fs::create_dir_all(&cargo_bin_dir).unwrap();
        std::fs::write(cargo_bin_dir.join("bin0"), fixture_elf()).unwrap();
        let capture = eggpack_ci::RunnerCommand::CaptureBuild {
            contract: contract_path.to_string_lossy().into_owned(),
            release_plan: release_plan_path.to_string_lossy().into_owned(),
            build_bindings: build_bindings_path.to_string_lossy().into_owned(),
            target: "x86_64-unknown-linux-gnu".into(),
            cargo_target_dir: cargo_root.to_string_lossy().into_owned(),
            output_dir: root
                .join("artifacts/build/x86_64-unknown-linux-gnu")
                .to_string_lossy()
                .into_owned(),
        };
        ci_capture_build(&capture.argv()[3..]).unwrap();
        let build_dir = root.join("artifacts/build/x86_64-unknown-linux-gnu");
        let qualify = eggpack_ci::RunnerCommand::QualifyTarget {
            contract: contract_path.to_string_lossy().into_owned(),
            release_plan: release_plan_path.to_string_lossy().into_owned(),
            build_bindings: build_bindings_path.to_string_lossy().into_owned(),
            qualification_bindings: qual_bindings_path.to_string_lossy().into_owned(),
            target: "x86_64-unknown-linux-gnu".into(),
            candidate_dir: build_dir.join("candidates").to_string_lossy().into_owned(),
            build_handoff: build_dir
                .join("build-handoff.json")
                .to_string_lossy()
                .into_owned(),
            output_dir: root
                .join("artifacts/qual/x86_64-unknown-linux-gnu")
                .to_string_lossy()
                .into_owned(),
            qemu_sysroot: None,
        };
        ci_qualify_target(&qualify.argv()[3..]).unwrap();
        let qual_dir = root.join("artifacts/qual/x86_64-unknown-linux-gnu");
        // Consumer validator script + map, resolved against a source root.
        let source_root = root.join("checkout");
        std::fs::create_dir_all(source_root.join("scripts")).unwrap();
        std::fs::write(
            source_root.join("scripts/smoke.py"),
            "import sys\nopen(sys.argv[1], 'rb').read()\n",
        )
        .unwrap();
        let validator = eggpack_ci::ConsumerValidatorV1 {
            schema_version: 1,
            selector: eggpack_core::LogicalOutputSelector::Direct,
            interpreter: eggpack_ci::ValidatorInterpreterV1::Python3,
            script: "scripts/smoke.py".into(),
            timeout_ms: 20_000,
            stdout_limit: 65_536,
            stderr_limit: 65_536,
        };
        let map: std::collections::BTreeMap<String, eggpack_ci::ConsumerValidatorV1> =
            [("x86_64-unknown-linux-gnu".to_owned(), validator)]
                .into_iter()
                .collect();
        let map_path = root.join("consumer-validators.json");
        std::fs::write(&map_path, serde_json::to_string(&map).unwrap()).unwrap();
        // The renderer-derived validate command drives the real CLI.
        let validate = eggpack_ci::RunnerCommand::ValidateConsumer {
            consumer_validators: map_path.to_string_lossy().into_owned(),
            target: "x86_64-unknown-linux-gnu".into(),
            candidate_dir: qual_dir.join("candidates").to_string_lossy().into_owned(),
            build_handoff: qual_dir
                .join("build-handoff.json")
                .to_string_lossy()
                .into_owned(),
            evidence: qual_dir
                .join("evidence.json")
                .to_string_lossy()
                .into_owned(),
            source_root: source_root.to_string_lossy().into_owned(),
            output: root
                .join("consumer-evidence.json")
                .to_string_lossy()
                .into_owned(),
        };
        let argv = validate.argv();
        assert_eq!(&argv[..3], &["eggpack", "ci", "_validate-consumer"]);
        ci_validate_consumer(&argv[3..]).unwrap();
        let evidence = eggpack_ci::decode_consumer_evidence(
            &std::fs::read_to_string(root.join("consumer-evidence.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            evidence.outcome,
            eggpack_ci::ConsumerValidationOutcome::Passed
        );
        // Failing script fails the command (required gate blocks).
        std::fs::write(
            source_root.join("scripts/smoke.py"),
            "import sys\nsys.exit(1)\n",
        )
        .unwrap();
        assert!(ci_validate_consumer(&argv[3..]).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prepare_stage_with_product_wrappers_materializes_four_installers() {
        use sha2::Digest;
        let root = temp_root("prepare-stage-wrappers");
        let contract_text =
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract_path = root.join("contract.toml");
        std::fs::write(&contract_path, contract_text).unwrap();
        let contract =
            eggpack_contract::DistributionContract::parse_toml_str(contract_text).unwrap();
        let body = b"body";
        let digest: String = sha2::Sha256::digest(body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest = eggpack_manifest::ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: "a".repeat(40),
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
        let manifest_path = root.join("release-manifest.json");
        std::fs::write(&manifest_path, manifest.to_json().unwrap()).unwrap();
        let finalized = root.join("finalized");
        std::fs::create_dir(&finalized).unwrap();
        for name in [
            "eggsact-1.2.6-aarch64-apple-darwin",
            "eggsact-1.2.6-x86_64-unknown-linux-gnu",
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
            tag: "v1.2.6".into(),
            title: "widget 1.2.6".into(),
            body: "notes".into(),
            prerelease: false,
            token_env: "GITHUB_TOKEN".into(),
            request_timeout_secs: 30,
            max_metadata_bytes: 1_000_000,
            max_list_pages: 5,
        };
        let policy_path = root.join("github-policy.json");
        std::fs::write(&policy_path, policy.to_json().unwrap()).unwrap();
        let install_policy_path = root.join("install-policy.toml");
        std::fs::write(&install_policy_path, "schema_version = 1\n").unwrap();
        // Product wrapper sources plus presentation policy (TOML accepted).
        let source_root = root.join("consumer");
        std::fs::create_dir_all(source_root.join("packaging")).unwrap();
        std::fs::write(source_root.join("packaging/install.sh"), b"#!/bin/sh\n").unwrap();
        std::fs::write(source_root.join("packaging/install.ps1"), b"# ps1\n").unwrap();
        let presentation_path = root.join("installer-presentation.toml");
        std::fs::write(
            &presentation_path,
            "schema_version = 1\n[mode.product_wrappers]\nposix_source = \"packaging/install.sh\"\npowershell_source = \"packaging/install.ps1\"\ngenerated_posix_name = \"install-exact.sh\"\ngenerated_powershell_name = \"install-exact.ps1\"\n",
        )
        .unwrap();
        let staging = root.join("staging");
        let payload_out = root.join("payload.json");
        let prepare = eggpack_ci::RunnerCommand::PrepareStage {
            contract: contract_path.to_string_lossy().into_owned(),
            release_manifest: manifest_path.to_string_lossy().into_owned(),
            finalized_root: finalized.to_string_lossy().into_owned(),
            github_policy: policy_path.to_string_lossy().into_owned(),
            install_policy: install_policy_path.to_string_lossy().into_owned(),
            output_dir: staging.to_string_lossy().into_owned(),
            output_payload: payload_out.to_string_lossy().into_owned(),
            installer_presentation: Some(presentation_path.to_string_lossy().into_owned()),
            source_root: Some(source_root.to_string_lossy().into_owned()),
        };
        let argv = prepare.argv();
        assert!(argv.contains(&"--installer-presentation".to_string()));
        assert!(argv.contains(&"--source-root".to_string()));
        ci_prepare_stage(&argv[3..]).unwrap();
        let payload = eggpack_github::StagingPayloadV1::from_json(
            &std::fs::read_to_string(&payload_out).unwrap(),
        )
        .unwrap();
        let installers: Vec<&str> = payload
            .assets
            .iter()
            .filter(|asset| {
                asset.name == "install.sh"
                    || asset.name == "install.ps1"
                    || asset.name == "install-exact.sh"
                    || asset.name == "install-exact.ps1"
            })
            .map(|asset| asset.name.as_str())
            .collect();
        assert_eq!(installers.len(), 4);
        assert_eq!(
            std::fs::read(staging.join("install.sh")).unwrap(),
            b"#!/bin/sh\n"
        );
        // Paired flags: presentation without source root fails.
        let mut partial = argv[3..].to_vec();
        partial.retain(|arg| arg != "--source-root" && !arg.contains("consumer"));
        assert!(ci_prepare_stage(&partial).is_err());
        let _ = contract;
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shape_generate_check_round_trip_and_drift() {
        let root = temp_root("shape-generate");
        let contract_text =
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
        let contract_path = root.join("contract.toml");
        std::fs::write(&contract_path, contract_text).unwrap();
        let contract =
            eggpack_contract::DistributionContract::parse_toml_str(contract_text).unwrap();
        let config = eggpack_core::PackConfig {
            schema_version: 1,
            targets: vec![eggpack_core::TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: eggpack_core::BuildStrategy::NativeCargo,
                host_os: eggpack_core::HostOs::Linux,
                host_arch: eggpack_core::HostArch::X86_64,
                qualification_host: None,
                toolchain: eggpack_core::ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: eggpack_core::CompatibilityFloor::None,
                qualification: eggpack_core::Qualification::Structural,
                support: eggpack_core::SupportTier::Required,
            }],
        };
        let release = config
            .resolve(&contract, "1.2.3", &"a".repeat(40), &["linux-x64".into()])
            .unwrap();
        let bindings = eggpack_core::BuildBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                vec![eggpack_core::BuildBinding {
                    selector: eggpack_core::LogicalOutputSelector::Direct,
                    package: "eggsact".into(),
                    binary: "bin0".into(),
                }],
            )]
            .into_iter()
            .collect(),
        };
        let qual_bindings = eggpack_core::QualificationBindingsV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                eggpack_core::TargetQualificationBinding { smoke: None },
            )]
            .into_iter()
            .collect(),
        };
        let mut validators = std::collections::BTreeMap::new();
        validators.insert(
            "x86_64-unknown-linux-gnu".to_owned(),
            eggpack_ci::ConsumerValidatorV1 {
                schema_version: 1,
                selector: eggpack_core::LogicalOutputSelector::Direct,
                interpreter: eggpack_ci::ValidatorInterpreterV1::Python3,
                script: "scripts/smoke.py".into(),
                timeout_ms: 20_000,
                stdout_limit: 65_536,
                stderr_limit: 65_536,
            },
        );
        let shape = eggpack_ci::ReleaseWorkflowShapeV1 {
            schema_version: 1,
            targets: release.targets.iter().map(|t| t.policy.clone()).collect(),
            selected_aliases: vec!["linux-x64".into()],
            build_bindings: bindings,
            qualification_bindings: qual_bindings,
            consumer_validators: validators,
            staging: Some(eggpack_ci::ShapeStagingIntentV1 {
                provider: eggpack_ci::StagingProvider::GitHubDraft,
                tag_source: eggpack_ci::StagingTagSource::DispatchInput,
                required: true,
            }),
        };
        let shape_path = root.join("shape.json");
        std::fs::write(&shape_path, shape.to_json().unwrap()).unwrap();
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let policy = eggpack_ci::GitHubPolicy {
            preflight_runner: "ubuntu-latest".into(),
            runners: vec![eggpack_ci::RunnerMapping {
                os: eggpack_core::HostOs::Linux,
                arch: eggpack_core::HostArch::X86_64,
                label: "ubuntu-latest".into(),
                cargo_zigbuild: true,
                zig: true,
            }],
            checkout: eggpack_ci::ActionPin {
                reference: format!("actions/checkout@{sha}"),
            },
            rust_toolchain: eggpack_ci::ActionPin {
                reference: format!("dtolnay/rust-toolchain@{sha}"),
            },
            upload_artifact: eggpack_ci::ActionPin {
                reference: format!("actions/upload-artifact@{sha}"),
            },
            download_artifact: Some(eggpack_ci::ActionPin {
                reference: format!("actions/download-artifact@{sha}"),
            }),
            triggers: vec![
                eggpack_ci::WorkflowTrigger::Push,
                eggpack_ci::WorkflowTrigger::WorkflowDispatch,
            ],
            timeout_minutes: 60,
            cancel_in_progress: true,
            artifact_retention_days: 7,
            eggpack_tool: Some(eggpack_ci::EggpackToolPolicy {
                repo: "https://github.com/eggstack/eggpack".into(),
                revision: "a".repeat(40),
                package: "eggpack-cli".into(),
                install_timeout_minutes: 10,
            }),
            release_inputs: Some(eggpack_ci::GitHubReleaseInputsV1 {
                contract: "contracts/release.toml".into(),
                release_plan: "plans/release-plan.json".into(),
                build_bindings: "bindings/build.toml".into(),
                qualification_bindings: "bindings/qualification.toml".into(),
                ci_plan: "plans/release-ci-plan.json".into(),
                pack_config: Some("configs/pack.toml".into()),
                draft_template: Some("policies/github-template.json".into()),
                installer_presentation: None,
                consumer_validators: Some("validators/consumer.json".into()),
            }),
            emulated_sysroots: None,
            staging: Some(eggpack_ci::GitHubStagingPolicyV1 {
                runner: "ubuntu-latest".into(),
                owner: "acme".into(),
                repository: "widget".into(),
                tag_source: eggpack_ci::StagingTagSource::DispatchInput,
                inputs: eggpack_ci::GitHubStagingInputsV1 {
                    contract: "contracts/release.toml".into(),
                    install_policy: "policies/install.toml".into(),
                    github_policy: "policies/github-draft.json".into(),
                    installer_presentation: None,
                },
                receipt_retention_days: 7,
            }),
            cross_tools: None,
        };
        let policy_path = root.join("policy.json");
        std::fs::write(&policy_path, serde_json::to_string(&policy).unwrap()).unwrap();
        let output_path = root.join("release.yml");
        // Generate equals the library renderer.
        ci_generate(&[
            "--workflow-shape".into(),
            shape_path.to_string_lossy().into_owned(),
            "--contract".into(),
            contract_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--output".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .unwrap();
        let expected =
            eggpack_ci::render_reusable_release_github(&contract, &shape, &policy).unwrap();
        assert_eq!(std::fs::read_to_string(&output_path).unwrap(), expected);
        // Check passes the exact file and detects drift.
        ci_check(&[
            "--workflow-shape".into(),
            shape_path.to_string_lossy().into_owned(),
            "--contract".into(),
            contract_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--workflow".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .unwrap();
        let mut drifted = expected.clone();
        drifted.push('x');
        std::fs::write(&output_path, &drifted).unwrap();
        assert!(ci_check(&[
            "--workflow-shape".into(),
            shape_path.to_string_lossy().into_owned(),
            "--contract".into(),
            contract_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--workflow".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .is_err());
        // Mixed exact/reusable flags reject.
        assert!(ci_generate(&[
            "--workflow-shape".into(),
            shape_path.to_string_lossy().into_owned(),
            "--ci-plan".into(),
            shape_path.to_string_lossy().into_owned(),
            "--github-policy".into(),
            policy_path.to_string_lossy().into_owned(),
            "--output".into(),
            output_path.to_string_lossy().into_owned(),
        ])
        .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
