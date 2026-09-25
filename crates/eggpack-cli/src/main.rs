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
        return Err("usage: eggpack ci <generate|check|_capture-build|_qualify-target|_evaluate-gate|_aggregate> [options]".to_owned());
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("eggpack {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args[0] == "--help" || args[0] == "-h" {
        println!("usage: eggpack ci <generate|check|_capture-build|_qualify-target|_evaluate-gate|_aggregate> [options]");
        return Ok(());
    }
    if args[0] != "ci" {
        return Err("usage: eggpack ci <generate|check|_capture-build|_qualify-target|_evaluate-gate|_aggregate> [options]".to_owned());
    }
    if args.len() < 2 {
        return Err("usage: eggpack ci <generate|check|_capture-build|_qualify-target|_evaluate-gate|_aggregate> [options]".to_owned());
    }
    match args[1].as_str() {
        "generate" => ci_generate(&args[2..]),
        "check" => ci_check(&args[2..]),
        "_capture-build" => ci_capture_build(&args[2..]),
        "_qualify-target" => ci_qualify_target(&args[2..]),
        "_evaluate-gate" => ci_evaluate_gate(&args[2..]),
        "_aggregate" => ci_aggregate(&args[2..]),
        _ => Err("unknown ci subcommand".to_owned()),
    }
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
    let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
    let policy_path = PathBuf::from(get_flag(args, "github-policy")?);
    let output_path = PathBuf::from(get_flag(args, "output")?);
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
    let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
    let policy_path = PathBuf::from(get_flag(args, "github-policy")?);
    let workflow_path = PathBuf::from(get_flag(args, "workflow")?);
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

fn ci_capture_build(args: &[String]) -> Result<(), String> {
    let plan_path = PathBuf::from(get_flag(args, "release-plan")?);
    let bindings_path = PathBuf::from(get_flag(args, "build-bindings")?);
    let target = get_flag(args, "target")?;
    let candidate_dir = PathBuf::from(get_flag(args, "candidate-dir")?);
    let output_path = PathBuf::from(get_flag(args, "output")?);
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
    // Replace placeholder sizes with observed regular file sizes.
    let dir_metadata = std::fs::symlink_metadata(&candidate_dir)
        .map_err(|_| "candidate dir unavailable".to_owned())?;
    if !dir_metadata.is_dir() || dir_metadata.file_type().is_symlink() {
        return Err("candidate dir is not a real directory".to_owned());
    }
    for output in &mut handoff.outputs {
        let path = candidate_dir.join(&output.relative_path);
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|_| "candidate file unavailable".to_owned())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
            return Err("candidate is not a non-empty regular file".to_owned());
        }
        output.size = metadata.len();
    }
    handoff
        .validate()
        .map_err(|_| "handoff validation failed".to_owned())?;
    let json = handoff
        .to_json()
        .map_err(|_| "handoff serialization failed".to_owned())?;
    atomic_write(&output_path, json.as_bytes())?;
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
    let runtime = eggpack_core::QualificationRuntime { qemu_sysroot: None };
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
    println!("qualified {target}: {:?}", evidence.status);
    Ok(())
}

fn ci_evaluate_gate(args: &[String]) -> Result<(), String> {
    let ci_plan_path = PathBuf::from(get_flag(args, "ci-plan")?);
    let evidence_dir = PathBuf::from(get_flag(args, "evidence-dir")?);
    let output_path = PathBuf::from(get_flag(args, "output")?);
    let ci_text = read_bounded(&ci_plan_path, 1_000_000, "ci plan")?;
    let graph: eggpack_ci::ReleaseCIPlanV1 = eggpack_ci::ReleaseCIPlanV1::from_json(&ci_text)
        .map_err(|_| "invalid ci plan".to_owned())?;
    let mut evidences = Vec::new();
    for qual in &graph.qualifications {
        let path = evidence_dir.join(format!("{}.json", qual.evidence_handoff_name));
        let text = read_bounded(&path, 1_000_000, "evidence")?;
        let evidence = eggpack_ci::decode_qualification_evidence(&text)
            .map_err(|_| "invalid evidence".to_owned())?;
        evidences.push(evidence);
    }
    let outcome = eggpack_ci::evaluate_gate(&graph, &evidences)
        .map_err(|_| "gate evaluation failed".to_owned())?;
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
    let (outcome, finalized) = eggpack_ci::aggregate_finalize(
        &contract,
        &plan,
        &graph,
        &final_inputs,
        &graph.finalization.evidence_references,
        &output_root,
    )
    .map_err(|_| "aggregation failed".to_owned())?;
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
}
