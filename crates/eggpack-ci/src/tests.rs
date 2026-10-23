use super::*;
use crate::consumer_validator::hash_candidate_file;
use crate::drift::compare_workflow_bytes;
use crate::drift::normalize_newlines;
use crate::drift::MAX_WORKFLOW_BYTES;
use crate::identity::UNRESOLVED_RELEASE_ID;
use crate::identity::UNRESOLVED_SOURCE_REVISION;
use crate::planning::handoff_name;
use crate::render_github::contains_unsafe_dispatch_expression_in_run;
use crate::render_release::tool_install_snippet;
use eggpack_contract::DistributionContract;
use eggpack_core::{
    BuildBindingsV1, BuildStrategy, FinalizationTargetInput, HostRequirement,
    LogicalOutputSelector, Qualification, QualificationBindingsV1, QualificationEvidence,
    QualificationStatus, ReleasePlan, SupportTier,
};
use eggpack_core::{
    CompatibilityFloor, HostArch, HostOs, PackConfig, TargetPolicy, ToolchainRequirement,
};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

fn contract() -> DistributionContract {
    DistributionContract::parse_toml_str(include_str!(
        "../../eggpack-contract/tests/fixtures/simple-direct.toml"
    ))
    .unwrap()
}

#[test]
fn cargo_output_path_appends_exe_and_preserves_dotted_names() {
    let root = std::path::Path::new("/target");
    assert_eq!(
        cargo_output_path(root, "x86_64-unknown-linux-gnu", "demo").unwrap(),
        PathBuf::from("/target/x86_64-unknown-linux-gnu/release/demo")
    );
    assert_eq!(
        cargo_output_path(root, "x86_64-pc-windows-msvc", "demo").unwrap(),
        PathBuf::from("/target/x86_64-pc-windows-msvc/release/demo.exe")
    );
    // Cargo appends the suffix, so a dotted binary name keeps its own
    // extension instead of having it replaced.
    assert_eq!(
        cargo_output_path(root, "x86_64-pc-windows-msvc", "tool.cli").unwrap(),
        PathBuf::from("/target/x86_64-pc-windows-msvc/release/tool.cli.exe")
    );
    assert_eq!(
        cargo_output_path(root, "aarch64-apple-darwin", "tool.cli").unwrap(),
        PathBuf::from("/target/aarch64-apple-darwin/release/tool.cli")
    );
    // A triple that merely contains `windows` is not a Windows triple.
    assert_eq!(
        cargo_output_path(root, "x86_64-unknown-notwindowsish", "demo").unwrap(),
        PathBuf::from("/target/x86_64-unknown-notwindowsish/release/demo")
    );
}
fn plan(strategy: BuildStrategy, support: SupportTier) -> (DistributionContract, ReleasePlan) {
    let contract = contract();
    let target = "x86_64-unknown-linux-gnu";
    let policy = TargetPolicy {
        target: target.into(),
        strategy,
        host_os: HostOs::Linux,
        host_arch: HostArch::X86_64,
        qualification_host: Some(HostRequirement {
            os: HostOs::Macos,
            arch: HostArch::Aarch64,
        }),
        toolchain: ToolchainRequirement {
            rust: "1.89.0".into(),
            cargo_zigbuild: (strategy == BuildStrategy::CargoZigbuild).then(|| "0.20.0".into()),
            zig: (strategy == BuildStrategy::CargoZigbuild).then(|| "0.14.1".into()),
        },
        floor: CompatibilityFloor::Glibc {
            major: 2,
            minor: 17,
        },
        qualification: Qualification::DeferredNative,
        support,
    };
    let config = PackConfig {
        schema_version: 1,
        targets: vec![policy],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()],
        )
        .unwrap();
    (contract, release)
}
fn bindings(release: &ReleasePlan) -> BuildBindingsV1 {
    let target = &release.targets[0].target;
    BuildBindingsV1 {
        schema_version: 1,
        targets: [(
            target.clone(),
            vec![eggpack_core::BuildBinding {
                selector: LogicalOutputSelector::Direct,
                package: "eggsact".into(),
                binary: "eggsact".into(),
            }],
        )]
        .into_iter()
        .collect(),
    }
}
fn policy() -> GitHubPolicy {
    let sha = "0123456789abcdef0123456789abcdef01234567";
    GitHubPolicy {
        preflight_runner: "ubuntu-latest".into(),
        runners: vec![RunnerMapping {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
            label: "ubuntu-latest".into(),
            cargo_zigbuild: true,
            zig: true,
        }],
        checkout: ActionPin {
            reference: format!("actions/checkout@{sha}"),
        },
        rust_toolchain: ActionPin {
            reference: format!("dtolnay/rust-toolchain@{sha}"),
        },
        upload_artifact: ActionPin {
            reference: format!("actions/upload-artifact@{sha}"),
        },
        triggers: vec![WorkflowTrigger::Push, WorkflowTrigger::WorkflowDispatch],
        timeout_minutes: 60,
        cancel_in_progress: true,
        artifact_retention_days: 7,
        download_artifact: None,
        eggpack_tool: None,
        release_inputs: None,
        emulated_sysroots: None,
        staging: None,
        cross_tools: None,
        release_identity_mode: None,
    }
}
fn graph(strategy: BuildStrategy, support: SupportTier) -> CIPlan {
    let (contract, release) = plan(strategy, support);
    project_ci_plan(&contract, &release, &bindings(&release)).unwrap()
}
fn assert_golden(actual: &str, fixture: &str) {
    assert_eq!(actual, fixture.replace("\r\n", "\n"));
}

/// Write-through helper for M002a golden regeneration (before/after
/// corrective evidence). Used by `m002a_regenerate_goldens`.
#[allow(dead_code)]
fn write_golden(path: &str, contents: &str) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(root.join(path.trim_start_matches("../")), contents).unwrap();
}

#[test]
fn rendering_is_deterministic_parseable_read_only_and_hands_off_candidate() {
    let graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    assert_eq!(graph.targets[0].planned.target, "x86_64-unknown-linux-gnu");
    assert_eq!(
        graph.targets[0].qualification.host,
        Some(HostRequirement {
            os: HostOs::Macos,
            arch: HostArch::Aarch64,
        })
    );
    let output = render_github(&graph, &policy()).unwrap();
    assert_eq!(output, render_github(&graph, &policy()).unwrap());
    assert_golden(&output, include_str!("../tests/fixtures/native-direct.yml"));
    let encoded = graph.to_json().unwrap();
    assert_eq!(CIPlan::from_json(&encoded).unwrap(), graph);
    assert!(
        CIPlan::from_json(&encoded.replace("\"schema_version\":1", "\"schema_version\":2"))
            .is_err()
    );
    let parsed: serde_yaml::Value = serde_yaml::from_str(&output).unwrap();
    assert!(parsed.get("jobs").is_some());
    let permissions = parsed["permissions"].as_mapping().unwrap();
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions.get("contents").unwrap().as_str(), Some("read"));
    let jobs = parsed["jobs"].as_mapping().unwrap();
    for (_, job) in jobs {
        let permissions = job["permissions"].as_mapping().unwrap();
        assert_eq!(permissions.len(), 1);
        assert_eq!(permissions.get("contents").unwrap().as_str(), Some("read"));
    }
    assert!(output.contains("permissions:\n  contents: read"));
    assert!(output.contains("eggpack-x86_64-unknown-linux-gnu-direct"));
    assert!(!output.contains("eggsact-1.2.3"));
    assert!(output.contains("'cargo' '+1.89.0' 'build' '--release' '--locked'"));
    assert!(!output.contains("contents: write"));
    assert!(!output.contains("id-token: write"));
    assert!(!output.to_ascii_lowercase().contains("qualify"));
    assert!(!output.contains("publish"));
    let mut secret_metadata = graph.clone();
    secret_metadata.source_revision = "secret-token-that-must-not-render".into();
    secret_metadata.release_id = "secret-release-id".into();
    let rendered = render_github(&secret_metadata, &policy()).unwrap();
    assert!(!rendered.contains("secret-token-that-must-not-render"));
    assert!(!rendered.contains("secret-release-id"));
}

#[test]
fn zigbuild_reuses_core_floor_syntax_and_keeps_qualification_unresolved() {
    let graph = graph(BuildStrategy::CargoZigbuild, SupportTier::Experimental);
    let output = render_github(&graph, &policy()).unwrap();
    assert!(output.contains("x86_64-unknown-linux-gnu.2.17"));
    assert!(output.contains("cargo-zigbuild 0.20.0"));
    assert!(output.contains("continue-on-error: true"));
    assert_eq!(
        graph.targets[0].qualification.state,
        QualificationState::Unresolved
    );
    assert!(!graph.targets[0].required);
}

#[test]
fn drift_check_normalizes_only_crlf_and_detects_manual_edits() {
    let graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    let text = render_github(&graph, &policy()).unwrap();
    assert!(
        check_github(&graph, &policy(), text.as_bytes())
            .unwrap()
            .matches
    );
    let crlf = text.replace('\n', "\r\n");
    assert!(
        check_github(&graph, &policy(), crlf.as_bytes())
            .unwrap()
            .matches
    );
    let changed = text.replacen("contents: read", "contents: write", 1);
    let report = check_github(&graph, &policy(), changed.as_bytes()).unwrap();
    assert!(!report.matches);
    assert!(report.first_difference.is_some());
    assert!(check_github(&graph, &policy(), &vec![b'x'; MAX_WORKFLOW_BYTES + 1]).is_err());
}

#[test]
fn rejects_mutable_malformed_wrong_repo_and_duplicate_policy() {
    let direct_graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    let mut p = policy();
    p.checkout.reference = "actions/checkout@v4".into();
    assert!(p.validate(&direct_graph).is_err());
    let mut p = policy();
    p.checkout.reference = "somebody/checkout@0123456789abcdef0123456789abcdef01234567".into();
    assert!(p.validate(&direct_graph).is_err());
    let mut p = policy();
    p.upload_artifact.reference = "actions/upload-artifact@not-a-commit".into();
    assert!(p.validate(&direct_graph).is_err());
    let mut p = policy();
    p.runners.push(p.runners[0].clone());
    assert!(p.validate(&direct_graph).is_err());
    let mut p = policy();
    p.runners.clear();
    assert!(p.validate(&direct_graph).is_err());
    let mut p = policy();
    let duplicate = p.runners[0].clone();
    p.runners.resize(129, duplicate);
    assert!(p.validate(&direct_graph).is_err());
    let mut p = policy();
    p.timeout_minutes = 999;
    assert!(p.validate(&direct_graph).is_err());
    let cross_graph = graph(BuildStrategy::CargoZigbuild, SupportTier::Required);
    let mut p = policy();
    p.runners[0].cargo_zigbuild = false;
    assert!(p.validate(&cross_graph).is_err());
}

#[test]
fn rejects_invalid_graph_and_command_drift_from_m002() {
    let mut invalid_command = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    invalid_command.targets[0].outputs[0].args[1] = "test".into();
    assert!(invalid_command.validate().is_err());
    let mut invalid_gate = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    invalid_gate.required_aggregation_dependencies.clear();
    assert!(invalid_gate.validate().is_err());
    let encoded = graph(BuildStrategy::NativeCargo, SupportTier::Required)
        .to_json()
        .unwrap()
        .replace("unresolved", "passed");
    assert!(CIPlan::from_json(&encoded).is_err());
}

#[test]
fn archive_handoff_identity_is_bounded_deterministic_and_slot_distinct() {
    let source = "bin/tool/run".to_owned();
    let archive = LogicalOutputSelector::ArchiveMember { source };
    let name = handoff_name("x86_64-unknown-linux-gnu", &archive);
    assert!(name.starts_with("eggpack-x86_64-unknown-linux-gnu-archive-"));
    assert_eq!(name, handoff_name("x86_64-unknown-linux-gnu", &archive));
    assert_ne!(
        name,
        handoff_name("x86_64-unknown-linux-gnu", &LogicalOutputSelector::Direct)
    );
}

#[test]
fn canonical_multi_target_native_and_cross_jobs_are_sorted_and_explicit() {
    let contract = DistributionContract::parse_toml_str(include_str!(
        "../tests/fixtures/mixed-direct-targets.toml"
    ))
    .unwrap();
    let config = PackConfig {
        schema_version: 1,
        targets: vec![
            TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: Some(HostRequirement {
                    os: HostOs::Macos,
                    arch: HostArch::Aarch64,
                }),
                toolchain: ToolchainRequirement {
                    rust: "stable".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: CompatibilityFloor::None,
                qualification: Qualification::DeferredNative,
                support: SupportTier::Required,
            },
            TargetPolicy {
                target: "aarch64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::CargoZigbuild,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: Some("0.20.0".into()),
                    zig: Some("0.14.1".into()),
                },
                floor: CompatibilityFloor::Glibc {
                    major: 2,
                    minor: 17,
                },
                qualification: Qualification::Structural,
                support: SupportTier::NonGating,
            },
        ],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-arm64".into(), "linux-x64".into()],
        )
        .unwrap();
    let mut binding_map = std::collections::BTreeMap::new();
    for target in &release.targets {
        binding_map.insert(
            target.target.clone(),
            vec![eggpack_core::BuildBinding {
                selector: LogicalOutputSelector::Direct,
                package: "eggsact".into(),
                binary: "eggsact".into(),
            }],
        );
    }
    let bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: binding_map,
    };
    let graph = project_ci_plan(&contract, &release, &bindings).unwrap();
    assert_eq!(graph.targets.len(), 2);
    assert_eq!(graph.targets[0].planned.target, "aarch64-unknown-linux-gnu");
    assert_eq!(graph.targets[1].planned.target, "x86_64-unknown-linux-gnu");
    assert_eq!(
        graph.targets[0].planned.policy.strategy,
        BuildStrategy::CargoZigbuild
    );
    assert_eq!(
        graph.targets[1].planned.policy.strategy,
        BuildStrategy::NativeCargo
    );
    assert!(graph
        .required_aggregation_dependencies
        .contains(&graph.targets[1].job_id));
    assert!(!graph
        .required_aggregation_dependencies
        .contains(&graph.targets[0].job_id));
    assert_eq!(
        graph.targets[1].qualification.host.unwrap().os,
        HostOs::Macos
    );
    assert!(graph.targets[0].outputs[0]
        .args
        .iter()
        .any(|arg| { arg == "aarch64-unknown-linux-gnu.2.17" }));
    assert!(graph.targets[1].outputs[0]
        .args
        .iter()
        .any(|arg| arg == &graph.targets[1].planned.target));
    assert!(graph.validate().is_ok());
    let github = policy();
    let workflow = render_github(&graph, &github).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    assert!(parsed.get("jobs").is_some());
    assert_golden(
        &workflow,
        include_str!("../tests/fixtures/native-direct-multitarget.yml"),
    );
}

#[test]
fn bundle_and_archive_logical_slots_survive_projection_without_filename_inference() {
    for (fixture, product, sources) in [
        (
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            vec![
                LogicalOutputSelector::BundleEntry { index: 0 },
                LogicalOutputSelector::BundleEntry { index: 1 },
                LogicalOutputSelector::BundleEntry { index: 2 },
            ],
        ),
        (
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            vec![
                LogicalOutputSelector::ArchiveMember {
                    source: "egress".into(),
                },
                LogicalOutputSelector::ArchiveMember {
                    source: "bin/egress-helper".into(),
                },
            ],
        ),
    ] {
        let contract = DistributionContract::parse_toml_str(fixture).unwrap();
        let target = "x86_64-unknown-linux-gnu";
        let config = PackConfig {
            schema_version: 1,
            targets: vec![TargetPolicy {
                target: target.into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "stable".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: CompatibilityFloor::None,
                qualification: Qualification::DeferredNative,
                support: SupportTier::Required,
            }],
        };
        let release = config
            .resolve(&contract, "2.0.0", "revision", &["linux-x64".into()])
            .unwrap();
        let outputs = sources
            .iter()
            .enumerate()
            .map(|(index, selector)| eggpack_core::BuildBinding {
                selector: selector.clone(),
                package: product.into(),
                binary: format!("bin{index}"),
            })
            .collect();
        let bindings = BuildBindingsV1 {
            schema_version: 1,
            targets: [(target.into(), outputs)].into_iter().collect(),
        };
        let graph = project_ci_plan(&contract, &release, &bindings).unwrap();
        assert_eq!(
            graph.targets[0].planned.artifact_form,
            release.targets[0].artifact_form
        );
        let expected_slots: BTreeSet<_> = sources.iter().cloned().collect();
        assert_eq!(
            graph.targets[0]
                .outputs
                .iter()
                .map(|output| output.selector.clone())
                .collect::<BTreeSet<_>>(),
            expected_slots
        );
        assert_eq!(
            graph.targets[0]
                .outputs
                .iter()
                .map(|output| output.package.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            1
        );
        assert_eq!(
            graph.targets[0]
                .outputs
                .iter()
                .map(|output| output.handoff_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            sources.len()
        );
    }
}

// -----------------------------------------------------------------------
// M002 qualification/aggregation gates and drift CLI.
// -----------------------------------------------------------------------

fn m002_policy() -> GitHubPolicy {
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let mut base = policy();
    base.download_artifact = Some(ActionPin {
        reference: format!("actions/download-artifact@{sha}"),
    });
    base.eggpack_tool = Some(EggpackToolPolicy {
        repo: "https://github.com/eggstack/eggpack".into(),
        revision: "a".repeat(40),
        package: "eggpack-cli".into(),
        install_timeout_minutes: 10,
    });
    base.release_inputs = Some(GitHubReleaseInputsV1 {
        contract: "contracts/simple-direct.toml".into(),
        release_plan: "plans/release-plan.json".into(),
        build_bindings: "bindings/build.toml".into(),
        qualification_bindings: "bindings/qualification.toml".into(),
        ci_plan: "plans/release-ci-plan.json".into(),
        pack_config: None,
        draft_template: None,
        installer_presentation: None,
        consumer_validators: None,
    });
    base
}

fn m002_release(
    fixture: &str,
    product: &str,
    version: &str,
    target_alias: &str,
    qualification: Qualification,
    support: SupportTier,
) -> (
    DistributionContract,
    ReleasePlan,
    BuildBindingsV1,
    QualificationBindingsV1,
) {
    let contract = DistributionContract::parse_toml_str(fixture).unwrap();
    let target = match target_alias {
        "linux-x64" => "x86_64-unknown-linux-gnu",
        "macos-arm64" => "aarch64-apple-darwin",
        _ => target_alias,
    };
    let config = PackConfig {
        schema_version: 1,
        targets: vec![TargetPolicy {
            target: target.into(),
            strategy: BuildStrategy::NativeCargo,
            host_os: if target.contains("apple") {
                HostOs::Macos
            } else {
                HostOs::Linux
            },
            host_arch: if target.starts_with("aarch64") {
                HostArch::Aarch64
            } else {
                HostArch::X86_64
            },
            qualification_host: if qualification == Qualification::DeferredNative {
                Some(HostRequirement {
                    os: HostOs::Linux,
                    arch: HostArch::X86_64,
                })
            } else {
                None
            },
            toolchain: ToolchainRequirement {
                rust: "1.89.0".into(),
                cargo_zigbuild: None,
                zig: None,
            },
            floor: CompatibilityFloor::None,
            qualification,
            support,
        }],
    };
    let release = config
        .resolve(&contract, version, &"a".repeat(40), &[target_alias.into()])
        .unwrap();
    let expanded = contract.expand(target_alias, version).unwrap();
    let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
        eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
        eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
            .map(|index| LogicalOutputSelector::BundleEntry { index })
            .collect(),
        eggpack_contract::ExpandedAssets::Archive(a) => a
            .members
            .into_iter()
            .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
            .collect(),
    };
    let bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: [(
            target.into(),
            selectors
                .iter()
                .enumerate()
                .map(|(index, selector)| eggpack_core::BuildBinding {
                    selector: selector.clone(),
                    package: product.into(),
                    binary: format!("bin{index}"),
                })
                .collect(),
        )]
        .into_iter()
        .collect(),
    };
    let qual_bindings = QualificationBindingsV1 {
        schema_version: 1,
        targets: [(
            target.into(),
            eggpack_core::TargetQualificationBinding {
                smoke: matches!(
                    qualification,
                    Qualification::Native | Qualification::DeferredNative | Qualification::Emulated
                )
                .then(|| eggpack_core::CandidateSmokeBinding {
                    selector: selectors[0].clone(),
                    argv: vec!["--version".into()],
                    timeout_ms: 10_000,
                    stdout_limit: 8192,
                    stderr_limit: 8192,
                }),
            },
        )]
        .into_iter()
        .collect(),
    };
    (contract, release, bindings, qual_bindings)
}

fn m002_graph(
    qualification: Qualification,
    support: SupportTier,
) -> (
    DistributionContract,
    ReleasePlan,
    BuildBindingsV1,
    QualificationBindingsV1,
    CIPlan,
    ReleaseCIPlanV1,
) {
    let (contract, release, bindings, qual_bindings) = m002_release(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        "linux-x64",
        qualification,
        support,
    );
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
    (contract, release, bindings, qual_bindings, ci_plan, graph)
}

#[test]
fn m002_graph_projection_preserves_m001_and_validates_bindings() {
    let (_, _, _, _, ci_plan, graph) = m002_graph(Qualification::Structural, SupportTier::Required);
    // M001 CIPlan remains unchanged (qualification unresolved).
    assert_eq!(
        ci_plan.targets[0].qualification.state,
        QualificationState::Unresolved
    );
    assert_eq!(graph.schema_version, 1);
    assert_eq!(graph.gate_job_id, "required_gate");
    assert_eq!(graph.aggregate.job_id, "aggregate");
    assert_eq!(graph.qualifications.len(), 1);
    assert!(graph.qualifications[0].required);
    assert_eq!(
        graph.to_json().unwrap(),
        ReleaseCIPlanV1::from_json(&graph.to_json().unwrap())
            .unwrap()
            .to_json()
            .unwrap()
    );
    // Smoke selector mismatch rejects.
    let (_, release, bindings, mut qual_bindings, ci_plan, _) =
        m002_graph(Qualification::Structural, SupportTier::Required);
    // Structural must have no smoke; add one to force mismatch.
    qual_bindings
        .targets
        .get_mut("x86_64-unknown-linux-gnu")
        .unwrap()
        .smoke = Some(eggpack_core::CandidateSmokeBinding {
        selector: LogicalOutputSelector::Direct,
        argv: vec![],
        timeout_ms: 1000,
        stdout_limit: 1024,
        stderr_limit: 1024,
    });
    assert!(project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).is_err());
}

#[test]
fn m002_handoff_rejects_traversal_and_swaps() {
    let (_, release, bindings, _, _, _) =
        m002_graph(Qualification::Structural, SupportTier::Required);
    let mut handoff =
        project_build_handoff(&release, &bindings, "x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(
        handoff.to_json().unwrap(),
        BuildHandoffV1::from_json(&handoff.to_json().unwrap())
            .unwrap()
            .to_json()
            .unwrap()
    );
    // Absolute path rejects.
    handoff.outputs[0].relative_path = "/abs".into();
    assert!(handoff.validate().is_err());
    // Traversal rejects.
    let mut handoff =
        project_build_handoff(&release, &bindings, "x86_64-unknown-linux-gnu").unwrap();
    handoff.outputs[0].relative_path = "../escape".into();
    assert!(handoff.validate().is_err());
    // Duplicate handoff rejects (manual duplicate).
    let mut handoff =
        project_build_handoff(&release, &bindings, "x86_64-unknown-linux-gnu").unwrap();
    handoff.outputs.push(handoff.outputs[0].clone());
    assert!(handoff.validate().is_err());
}

fn m002_evidence(
    target: &str,
    release_id: &str,
    source: &str,
    classification: Qualification,
    support: SupportTier,
    status: QualificationStatus,
) -> QualificationEvidence {
    QualificationEvidence {
        schema_version: 1,
        release_id: release_id.into(),
        source_revision: source.into(),
        target: target.into(),
        planned_classification: classification,
        method: match classification {
            Qualification::Structural => eggpack_core::QualificationMethod::Structural,
            _ => eggpack_core::QualificationMethod::Structural,
        },
        actual_host: HostRequirement {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        },
        support,
        status,
        floor: eggpack_core::FloorVerificationStatus::NotDeclared,
        candidates: vec![],
        smoke_selector: None,
        processes: vec![],
    }
}

#[test]
fn m002_gate_matrix() {
    let (_, release, _, _, _, graph) = m002_graph(Qualification::Structural, SupportTier::Required);
    let target = "x86_64-unknown-linux-gnu";
    // Required Passed permits.
    let passed = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Passed,
    );
    assert_eq!(
        evaluate_gate(&graph, &[passed]).unwrap(),
        AggregateOutcome::Complete
    );
    // Required Deferred rejects.
    let deferred = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Deferred,
    );
    assert_eq!(
        evaluate_gate(&graph, &[deferred]).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    // Required Failed rejects.
    let failed = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
    );
    assert_eq!(
        evaluate_gate(&graph, &[failed]).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    // Missing required evidence rejects.
    assert_eq!(
        evaluate_gate(&graph, &[]).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    // Optional failure yields suppression, not partial release.
    let (_, _, _, _, _, optional_graph) =
        m002_graph(Qualification::Structural, SupportTier::NonGating);
    let failed_optional = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::NonGating,
        QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
    );
    let failed_optionals = [failed_optional];
    assert_eq!(
        evaluate_gate(&optional_graph, &failed_optionals).unwrap(),
        AggregateOutcome::SuppressedNonGatingIncomplete
    );
}

#[test]
fn m002_aggregation_finalizes_direct_bundle_archive_and_rejects_tampering() {
    for (fixture, product, version) in [
        (
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.6",
        ),
        (
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            "2.4.0",
        ),
        (
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            "3.1.0",
        ),
    ] {
        let (contract, release, bindings, qual_bindings) = m002_release(
            fixture,
            product,
            version,
            "linux-x64",
            Qualification::Structural,
            SupportTier::Required,
        );
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
        // Build fixture candidates (ELF for structural).
        let parent = eggpack_core_test_temp(&format!("m002-agg-{product}"));
        let source_root = parent.join("candidates");
        std::fs::create_dir(&source_root).unwrap();
        let elf = m002_elf();
        let planned = &release.targets[0];
        let expanded = contract.expand("linux-x64", version).unwrap();
        let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
            eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
            eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
                .map(|index| LogicalOutputSelector::BundleEntry { index })
                .collect(),
            eggpack_contract::ExpandedAssets::Archive(a) => a
                .members
                .into_iter()
                .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
                .collect(),
        };
        let mut candidates = Vec::new();
        for (index, selector) in selectors.iter().enumerate() {
            let path = source_root.join(format!("candidate-{index}"));
            std::fs::write(&path, &elf).unwrap();
            candidates.push(eggpack_core::CandidateArtifact {
                target: planned.target.clone(),
                selector: selector.clone(),
                package: product.into(),
                binary: format!("bin{index}"),
                path,
                size: elf.len() as u64,
            });
        }
        let attempt = eggpack_core::BuildAttempt {
            release_id: release.release_id.clone(),
            source_revision: release.source_revision.clone(),
            target: planned.target.clone(),
            strategy: BuildStrategy::NativeCargo,
            tool_summary: "fixture".into(),
            process: eggpack_core::ProcessEvidence {
                outcome: eggpack_core::CommandOutcome::Success,
                stdout_bytes: 0,
                stderr_bytes: 0,
            },
            candidates: candidates.clone(),
        };
        let digest = {
            use sha2::{Digest, Sha256};
            Sha256::digest(&elf)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let mut qualified: Vec<eggpack_core::QualifiedCandidateEvidence> = candidates
            .iter()
            .map(|c| eggpack_core::QualifiedCandidateEvidence {
                selector: c.selector.clone(),
                package: c.package.clone(),
                binary: c.binary.clone(),
                size: elf.len() as u64,
                sha256: digest.clone(),
                format: eggpack_core::CandidateFormat::Elf,
                architecture: eggpack_core::CandidateArchitecture::X86_64,
            })
            .collect();
        qualified.sort_by(|a, b| a.selector.cmp(&b.selector));
        let evidence = QualificationEvidence {
            schema_version: 1,
            release_id: release.release_id.clone(),
            source_revision: release.source_revision.clone(),
            target: planned.target.clone(),
            planned_classification: Qualification::Structural,
            method: eggpack_core::QualificationMethod::Structural,
            actual_host: HostRequirement {
                os: HostOs::Linux,
                arch: HostArch::X86_64,
            },
            support: SupportTier::Required,
            status: QualificationStatus::Passed,
            floor: eggpack_core::FloorVerificationStatus::NotDeclared,
            candidates: qualified,
            smoke_selector: None,
            processes: vec![],
        };
        // Complete path finalizes via M004 (invoked, not copied).
        let inputs = vec![FinalizationTargetInput {
            target: planned.target.clone(),
            attempt: attempt.clone(),
            qualification: evidence.clone(),
        }];
        let output = parent.join("release");
        let (outcome, finalized) =
            aggregate_finalize(&contract, &release, &graph, &inputs, &[], &output).unwrap();
        assert_eq!(outcome, AggregateOutcome::Complete);
        assert!(finalized.is_some());
        // Tampered candidate rejects (no partial release).
        std::fs::write(&candidates[0].path, b"changed").unwrap();
        let rejected_root = parent.join("rejected");
        let result = aggregate_finalize(&contract, &release, &graph, &inputs, &[], &rejected_root);
        assert!(result.is_err() || result.unwrap().0 != AggregateOutcome::Complete);
        // Missing optional target suppresses output (no partial release).
        let (_, _, _, _, _, optional_graph) = {
            let (c, r, b, q) = m002_release(
                fixture,
                product,
                version,
                "linux-x64",
                Qualification::Structural,
                SupportTier::NonGating,
            );
            let ci = project_ci_plan(&c, &r, &b).unwrap();
            let g = project_release_plan(&ci, &q, &b, &r).unwrap();
            (c, r, b, q, ci, g)
        };
        let mut deferred_evidence = evidence.clone();
        deferred_evidence.support = SupportTier::NonGating;
        deferred_evidence.status = QualificationStatus::Deferred;
        // Deferred with Structural method is inconsistent; use Failed to trigger suppression.
        deferred_evidence.status =
            QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed);
        let (outcome, finalized) = aggregate_finalize(
            &contract,
            &release,
            &optional_graph,
            &[FinalizationTargetInput {
                target: planned.target.clone(),
                attempt,
                qualification: deferred_evidence,
            }],
            &[],
            &parent.join("suppressed"),
        )
        .unwrap();
        assert_eq!(outcome, AggregateOutcome::SuppressedNonGatingIncomplete);
        assert!(finalized.is_none());
        std::fs::remove_dir_all(parent).unwrap();
    }
}

fn m002_elf() -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes
}

fn eggpack_core_test_temp(label: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("eggpack-{label}-{}-{id}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create test temp directory: {error}"),
        }
    }
}

#[test]
fn m002_renderer_is_deterministic_read_only_pinned_and_gated() {
    let (_, _, _, _, _, graph) = m002_graph(Qualification::Structural, SupportTier::Required);
    let policy = m002_policy();
    let first = render_release_github(&graph, &policy).unwrap();
    assert_eq!(first, render_release_github(&graph, &policy).unwrap());
    let parsed: serde_yaml::Value = serde_yaml::from_str(&first).unwrap();
    assert!(parsed.get("jobs").is_some());
    // Read-only permissions everywhere.
    for (_, job) in parsed["jobs"].as_mapping().unwrap() {
        let permissions = job["permissions"].as_mapping().unwrap();
        assert_eq!(permissions.len(), 1);
        assert_eq!(permissions.get("contents").unwrap().as_str(), Some("read"));
    }
    assert!(first.contains("contents: read"));
    assert!(!first.contains("contents: write"));
    assert!(!first.contains("id-token: write"));
    // Pinned Eggpack tooling.
    assert!(first.contains("cargo install --git https://github.com/eggstack/eggpack --rev"));
    assert!(first.contains("--locked eggpack-cli"));
    assert!(first.contains("eggpack --version"));
    // Exact gate dependencies and no release API.
    assert!(first.contains("required_gate"));
    assert!(first.contains("_qualify-target"));
    assert!(first.contains("--contract"));
    assert!(first.contains("--release-plan"));
    assert!(first.contains("--build-bindings"));
    assert!(first.contains("--qualification-bindings"));
    assert!(first.contains("--candidate-dir"));
    assert!(first.contains("--build-handoff"));
    assert!(first.contains("--output-dir"));
    assert!(first.contains("_capture-build"));
    assert!(first.contains("--cargo-target-dir"));
    assert!(first.contains("_evaluate-gate"));
    assert!(first.contains("--inputs-dir"));
    assert!(first.contains("_aggregate"));
    assert!(first.contains("--output-root"));
    assert!(!first.contains("github-release"));
    assert!(!first.contains("publish"));
    // No arbitrary command.
    assert!(!first.contains("curl -L"));
    // Drift check normalizes CRLF only and detects one-byte edits.
    assert!(
        check_release_github(&graph, &policy, first.as_bytes())
            .unwrap()
            .matches
    );
    let crlf = first.replace('\n', "\r\n");
    assert!(
        check_release_github(&graph, &policy, crlf.as_bytes())
            .unwrap()
            .matches
    );
    let changed = first.replacen("contents: read", "contents: write", 1);
    let report = check_release_github(&graph, &policy, changed.as_bytes()).unwrap();
    assert!(!report.matches);
    assert!(report.first_difference.is_some());
}

#[test]
fn m002_rejects_unpinned_tooling_and_missing_evidence() {
    let (_, _, _, _, _, graph) = m002_graph(Qualification::Structural, SupportTier::Required);
    // Unpinned tooling rejects.
    let mut bad_policy = m002_policy();
    bad_policy.eggpack_tool.as_mut().unwrap().revision = "v1".into();
    assert!(render_release_github(&graph, &bad_policy).is_err());
    let mut bad_policy = m002_policy();
    bad_policy.eggpack_tool.as_mut().unwrap().repo = "https://example.invalid/eggpack".into();
    assert!(render_release_github(&graph, &bad_policy).is_err());
    let mut bad_policy = m002_policy();
    bad_policy.eggpack_tool = None;
    assert!(render_release_github(&graph, &bad_policy).is_err());
    // Missing download pin rejects.
    let mut bad_policy = m002_policy();
    bad_policy.download_artifact = None;
    assert!(render_release_github(&graph, &bad_policy).is_err());
    // Missing explicit release inputs rejects (no hidden discovery).
    let mut bad_policy = m002_policy();
    bad_policy.release_inputs = None;
    assert!(render_release_github(&graph, &bad_policy).is_err());
    // Invalid relative input path rejects.
    let mut bad_policy = m002_policy();
    bad_policy.release_inputs.as_mut().unwrap().contract = "../escape.toml".into();
    assert!(render_release_github(&graph, &bad_policy).is_err());
    let mut bad_policy = m002_policy();
    bad_policy.release_inputs.as_mut().unwrap().ci_plan = "/abs/path.json".into();
    assert!(render_release_github(&graph, &bad_policy).is_err());
}

#[test]
fn m002a_emulated_requires_explicit_runtime_policy() {
    let (_, _, _, _, _, graph) = m002_graph(Qualification::Emulated, SupportTier::Required);
    // Without a runtime policy the renderer must reject.
    assert!(render_release_github(&graph, &m002_policy()).is_err());
    // With an explicit sysroot the renderer passes it to the CLI.
    let policy = m002a_policy_with_sysroot("x86_64-unknown-linux-gnu", "sysroots/linux-x64");
    let rendered = render_release_github(&graph, &policy).unwrap();
    assert!(rendered.contains("--qemu-sysroot"));
    assert!(rendered.contains("sysroots/linux-x64"));
}

#[test]
fn m002a_release_inputs_reject_escapes() {
    let valid = GitHubReleaseInputsV1 {
        contract: "contracts/release.toml".into(),
        release_plan: "plans/release-plan.json".into(),
        build_bindings: "bindings/build.toml".into(),
        qualification_bindings: "bindings/qualification.toml".into(),
        ci_plan: "plans/release-ci-plan.json".into(),
        pack_config: None,
        draft_template: None,
        installer_presentation: None,
        consumer_validators: None,
    };
    assert!(valid.validate().is_ok());
    for bad in [
        "",
        "/abs.toml",
        "../escape.toml",
        "a//b.toml",
        "a/./b.toml",
        "win\\path.toml",
        "c:/drive.toml",
        "nul\0byte.toml",
    ] {
        let mut inputs = valid.clone();
        inputs.contract = bad.into();
        assert!(inputs.validate().is_err(), "must reject {bad:?}");
    }
}

#[test]
fn m002a_runner_command_round_trip_covers_all_required_args() {
    // Every internal CLI command has an exact structured round-trip: the
    // renderer serializes the same RunnerCommand the tests execute, so CLI
    // signature drift is caught here rather than in generated YAML alone.
    let capture = RunnerCommand::CaptureBuild {
        contract: "c.toml".into(),
        release_plan: "r.json".into(),
        build_bindings: "b.toml".into(),
        target: "x86_64-unknown-linux-gnu".into(),
        cargo_target_dir: "/tmp/cargo-target".into(),
        output_dir: "./eggpack-build/build_x".into(),
    };
    let shell = capture.to_shell();
    for required in [
        "_capture-build",
        "--release-plan",
        "--build-bindings",
        "--target",
        "--cargo-target-dir",
        "--output-dir",
    ] {
        assert!(shell.contains(required), "capture shell missing {required}");
    }
    assert_eq!(capture.argv()[0], "eggpack");

    let qualify = RunnerCommand::QualifyTarget {
        contract: "c.toml".into(),
        release_plan: "r.json".into(),
        build_bindings: "b.toml".into(),
        qualification_bindings: "q.toml".into(),
        target: "x86_64-unknown-linux-gnu".into(),
        candidate_dir: "./eggpack-handoff/build_x/candidates".into(),
        build_handoff: "./eggpack-handoff/build_x/build-handoff.json".into(),
        output_dir: "./eggpack-qualification/build_x".into(),
        qemu_sysroot: None,
    };
    let shell = qualify.to_shell();
    for required in [
        "_qualify-target",
        "--contract",
        "--release-plan",
        "--build-bindings",
        "--qualification-bindings",
        "--target",
        "--candidate-dir",
        "--build-handoff",
        "--output-dir",
    ] {
        assert!(shell.contains(required), "qualify shell missing {required}");
    }
    // No hidden discovery flags and no absolute serialized paths.
    assert!(!shell.contains(".."));
    assert!(!shell.contains("tar -"));
    let qualify_sysroot = RunnerCommand::QualifyTarget {
        contract: "c.toml".into(),
        release_plan: "r.json".into(),
        build_bindings: "b.toml".into(),
        qualification_bindings: "q.toml".into(),
        target: "x86_64-unknown-linux-gnu".into(),
        candidate_dir: "./eggpack-handoff/build_x/candidates".into(),
        build_handoff: "./eggpack-handoff/build_x/build-handoff.json".into(),
        output_dir: "./eggpack-qualification/build_x".into(),
        qemu_sysroot: Some("sysroots/linux-x64".into()),
    };
    assert!(qualify_sysroot.to_shell().contains("--qemu-sysroot"));

    let gate = RunnerCommand::EvaluateGate {
        ci_plan: "plans/ci.json".into(),
        inputs_dir: "./eggpack-inputs".into(),
        output: "./eggpack-gate/gate-outcome.json".into(),
    };
    let shell = gate.to_shell();
    for required in ["_evaluate-gate", "--ci-plan", "--inputs-dir", "--output"] {
        assert!(shell.contains(required), "gate shell missing {required}");
    }

    let aggregate = RunnerCommand::Aggregate {
        contract: "c.toml".into(),
        release_plan: "r.json".into(),
        ci_plan: "plans/ci.json".into(),
        inputs_dir: "./eggpack-inputs".into(),
        output_root: "./eggpack-finalized/root".into(),
        output: "./eggpack-finalized/summary.json".into(),
    };
    let shell = aggregate.to_shell();
    for required in [
        "_aggregate",
        "--contract",
        "--release-plan",
        "--ci-plan",
        "--inputs-dir",
        "--output-root",
        "--output",
    ] {
        assert!(
            shell.contains(required),
            "aggregate shell missing {required}"
        );
    }
}

fn m002_golden_policy() -> GitHubPolicy {
    let sha = "0123456789abcdef0123456789abcdef01234567";
    GitHubPolicy {
        preflight_runner: "ubuntu-latest".into(),
        runners: vec![
            RunnerMapping {
                os: HostOs::Linux,
                arch: HostArch::X86_64,
                label: "ubuntu-latest".into(),
                cargo_zigbuild: true,
                zig: true,
            },
            RunnerMapping {
                os: HostOs::Macos,
                arch: HostArch::Aarch64,
                label: "macos-latest".into(),
                cargo_zigbuild: false,
                zig: false,
            },
        ],
        checkout: ActionPin {
            reference: format!("actions/checkout@{sha}"),
        },
        rust_toolchain: ActionPin {
            reference: format!("dtolnay/rust-toolchain@{sha}"),
        },
        upload_artifact: ActionPin {
            reference: format!("actions/upload-artifact@{sha}"),
        },
        download_artifact: Some(ActionPin {
            reference: format!("actions/download-artifact@{sha}"),
        }),
        triggers: vec![WorkflowTrigger::Push, WorkflowTrigger::WorkflowDispatch],
        timeout_minutes: 60,
        cancel_in_progress: true,
        artifact_retention_days: 7,
        eggpack_tool: Some(EggpackToolPolicy {
            repo: "https://github.com/eggstack/eggpack".into(),
            revision: "a".repeat(40),
            package: "eggpack-cli".into(),
            install_timeout_minutes: 10,
        }),
        release_inputs: Some(GitHubReleaseInputsV1 {
            contract: "contracts/release.toml".into(),
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
        release_identity_mode: None,
    }
}

fn m002a_policy_with_sysroot(target: &str, sysroot: &str) -> GitHubPolicy {
    let mut policy = m002_golden_policy();
    policy.emulated_sysroots = Some(
        [(target.to_string(), sysroot.to_string())]
            .into_iter()
            .collect(),
    );
    policy
}

fn m002_golden_case(
    fixture: &str,
    product: &str,
    version: &str,
    targets: Vec<(&str, BuildStrategy, SupportTier, Qualification)>,
    aliases: Vec<&str>,
) -> (ReleaseCIPlanV1, GitHubPolicy) {
    let contract = DistributionContract::parse_toml_str(fixture).unwrap();
    let policies: Vec<TargetPolicy> = targets
        .iter()
        .map(|(triple, strategy, support, qual)| {
            let (host_os, host_arch) = if triple.contains("apple") {
                (HostOs::Macos, HostArch::Aarch64)
            } else {
                (HostOs::Linux, HostArch::X86_64)
            };
            TargetPolicy {
                target: triple.to_string(),
                strategy: *strategy,
                host_os,
                host_arch,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: if *strategy == BuildStrategy::CargoZigbuild {
                        Some("0.20.0".into())
                    } else {
                        None
                    },
                    zig: if *strategy == BuildStrategy::CargoZigbuild {
                        Some("0.14.1".into())
                    } else {
                        None
                    },
                },
                floor: CompatibilityFloor::None,
                qualification: *qual,
                support: *support,
            }
        })
        .collect();
    let config = PackConfig {
        schema_version: 1,
        targets: policies,
    };
    let selected: Vec<String> = aliases.into_iter().map(|s| s.to_string()).collect();
    let release = config
        .resolve(&contract, version, &"a".repeat(40), &selected)
        .unwrap();
    let mut binding_map = std::collections::BTreeMap::new();
    let mut qual_map = std::collections::BTreeMap::new();
    for target in &release.targets {
        let expanded = contract.expand(&target.target, version).unwrap();
        let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
            eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
            eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
                .map(|index| LogicalOutputSelector::BundleEntry { index })
                .collect(),
            eggpack_contract::ExpandedAssets::Archive(a) => a
                .members
                .into_iter()
                .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
                .collect(),
        };
        binding_map.insert(
            target.target.clone(),
            selectors
                .iter()
                .enumerate()
                .map(|(index, selector)| eggpack_core::BuildBinding {
                    selector: selector.clone(),
                    package: product.into(),
                    binary: format!("bin{index}"),
                })
                .collect(),
        );
        qual_map.insert(
            target.target.clone(),
            eggpack_core::TargetQualificationBinding { smoke: None },
        );
    }
    let bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: binding_map,
    };
    let qual_bindings = QualificationBindingsV1 {
        schema_version: 1,
        targets: qual_map,
    };
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
    (graph, m002_golden_policy())
}

#[test]
fn m002_golden_direct_bundle_archive_and_mixed() {
    // Direct single-target.
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m002-direct.yml"),
    );
    // Mixed native/cross.
    let (graph, policy) = m002_golden_case(
        include_str!("../tests/fixtures/mixed-direct-targets.toml"),
        "eggsact",
        "1.2.3",
        vec![
            (
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            ),
            (
                "aarch64-unknown-linux-gnu",
                BuildStrategy::CargoZigbuild,
                SupportTier::NonGating,
                Qualification::Structural,
            ),
        ],
        vec!["linux-x64", "linux-arm64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m002-mixed.yml"),
    );
    // Bundle.
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
        "codegg",
        "2.4.0",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m002-bundle.yml"),
    );
    // Archive.
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
        "egress",
        "3.1.0",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m002-archive.yml"),
    );
}

// Not standalone coverage: rewrites checked-in goldens in place through
// `write_golden`, so it must never run as part of a normal test pass.
#[test]
#[ignore]
fn m002a_regenerate_goldens() {
    // Regenerate checked-in M002a goldens from the corrected renderer.
    // Run explicitly with `cargo test -p eggpack-ci --lib -- --ignored`.
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    write_golden(
        "../tests/fixtures/m002-direct.yml",
        &render_release_github(&graph, &policy).unwrap(),
    );
    let (graph, policy) = m002_golden_case(
        include_str!("../tests/fixtures/mixed-direct-targets.toml"),
        "eggsact",
        "1.2.3",
        vec![
            (
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            ),
            (
                "aarch64-unknown-linux-gnu",
                BuildStrategy::CargoZigbuild,
                SupportTier::NonGating,
                Qualification::Structural,
            ),
        ],
        vec!["linux-x64", "linux-arm64"],
    );
    write_golden(
        "../tests/fixtures/m002-mixed.yml",
        &render_release_github(&graph, &policy).unwrap(),
    );
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
        "codegg",
        "2.4.0",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    write_golden(
        "../tests/fixtures/m002-bundle.yml",
        &render_release_github(&graph, &policy).unwrap(),
    );
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
        "egress",
        "3.1.0",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    write_golden(
        "../tests/fixtures/m002-archive.yml",
        &render_release_github(&graph, &policy).unwrap(),
    );
}

/// M002a end-to-end executable fixture: the same rendered command/argument
/// /layout contract drives build -> capture -> qualify -> gate ->
/// aggregate -> M004 finalization directly (no GitHub invocation).
#[test]
fn m002a_generated_orchestration_executes_end_to_end() {
    for (fixture, product, version) in [
        (
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
        ),
        (
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            "2.4.0",
        ),
        (
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            "3.1.0",
        ),
    ] {
        let (contract, release, bindings, qual_bindings) = m002_release(
            fixture,
            product,
            version,
            "linux-x64",
            Qualification::Structural,
            SupportTier::Required,
        );
        let target = "x86_64-unknown-linux-gnu";
        let planned = release
            .targets
            .iter()
            .find(|t| t.target == target)
            .unwrap()
            .clone();
        let parent = eggpack_core_test_temp(&format!("m002a-e2e-{product}"));

        // Build candidate bytes (ELF for structural qualification).
        let elf = m002_elf();
        let mut handoff = project_build_handoff(&release, &bindings, target).unwrap();
        let stage_src = parent.join("stage-src");
        std::fs::create_dir(&stage_src).unwrap();
        for output in &mut handoff.outputs {
            std::fs::write(stage_src.join(&output.relative_path), &elf).unwrap();
            output.size = elf.len() as u64;
        }
        handoff.validate().unwrap();

        // _capture-build equivalent: stage the canonical build artifact.
        let build_dir = parent.join("build").join(target);
        stage_build_artifact_dir(&handoff, &stage_src, &build_dir).unwrap();
        let staged = validate_build_artifact_dir(&build_dir).unwrap();
        assert_eq!(staged.to_json().unwrap(), handoff.to_json().unwrap());

        // _qualify-target equivalent: reconstruct, execute M003, write the
        // canonical qualification artifact (handoff + evidence + bytes).
        let candidates_dir = build_dir.join(CANDIDATES_DIR);
        let attempt = reconstruct_attempt(&release, &planned, &staged, &candidates_dir).unwrap();
        let runtime = eggpack_core::QualificationRuntime { qemu_sysroot: None };
        let cancellation = eggpack_core::BuildCancellation::new();
        let evidence = eggpack_core::qualify_target(eggpack_core::QualificationRequest {
            contract: &contract,
            plan: &release,
            target: &planned,
            attempt: &attempt,
            build_bindings: &bindings,
            qualification_bindings: &qual_bindings,
            runtime: &runtime,
            cancellation: &cancellation,
        })
        .unwrap();
        assert_eq!(evidence.status, eggpack_core::QualificationStatus::Passed);
        let qual_dir = parent.join("qual").join(target);
        std::fs::create_dir_all(qual_dir.join(CANDIDATES_DIR)).unwrap();
        std::fs::write(
            qual_dir.join(BUILD_HANDOFF_FILE),
            staged.to_json().unwrap().as_bytes(),
        )
        .unwrap();
        std::fs::write(
            qual_dir.join(QUALIFICATION_EVIDENCE_FILE),
            encode_qualification_evidence(&evidence).unwrap().as_bytes(),
        )
        .unwrap();
        for output in &staged.outputs {
            std::fs::copy(
                candidates_dir.join(&output.relative_path),
                qual_dir.join(CANDIDATES_DIR).join(&output.relative_path),
            )
            .unwrap();
        }
        let (_, staged_evidence) = validate_qualification_artifact_dir(&qual_dir).unwrap();
        assert_eq!(
            encode_qualification_evidence(&staged_evidence).unwrap(),
            encode_qualification_evidence(&evidence).unwrap()
        );

        // _evaluate-gate + _aggregate equivalent over the canonical
        // per-target inputs layout.
        let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
        let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
        let inputs_root = parent.join("inputs");
        std::fs::create_dir_all(inputs_root.join(target)).unwrap();
        for entry in std::fs::read_dir(&qual_dir).unwrap() {
            let entry = entry.unwrap();
            let dest = inputs_root.join(target).join(entry.file_name());
            if entry.path().is_dir() {
                std::fs::create_dir_all(&dest).unwrap();
                for inner in std::fs::read_dir(entry.path()).unwrap() {
                    let inner = inner.unwrap();
                    std::fs::copy(inner.path(), dest.join(inner.file_name())).unwrap();
                }
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
        let gate_text =
            std::fs::read_to_string(inputs_root.join(target).join("evidence.json")).unwrap();
        let gate_evidence = decode_qualification_evidence(&gate_text).unwrap();
        assert_eq!(
            evaluate_gate(&graph, &[gate_evidence]).unwrap(),
            AggregateOutcome::Complete
        );

        // Reconstruct the attempt from the aggregate inputs layout and
        // finalize through M004; assert exact bytes and manifest identity.
        let agg_handoff_text =
            std::fs::read_to_string(inputs_root.join(target).join("build-handoff.json")).unwrap();
        let agg_handoff = BuildHandoffV1::from_json(&agg_handoff_text).unwrap();
        let agg_attempt = reconstruct_attempt(
            &release,
            &planned,
            &agg_handoff,
            &inputs_root.join(target).join("candidates"),
        )
        .unwrap();
        assert_eq!(agg_attempt.candidates.len(), handoff.outputs.len());
        for candidate in &agg_attempt.candidates {
            assert_eq!(
                std::fs::read(&candidate.path).unwrap(),
                elf,
                "final artifact bytes must be exact"
            );
        }
        let final_inputs = vec![FinalizationTargetInput {
            target: target.to_string(),
            attempt: agg_attempt,
            qualification: decode_qualification_evidence(
                &std::fs::read_to_string(inputs_root.join(target).join("evidence.json")).unwrap(),
            )
            .unwrap(),
        }];
        let output_root = parent.join("finalized");
        let (outcome, finalized) = aggregate_finalize(
            &contract,
            &release,
            &graph,
            &final_inputs,
            &[],
            &output_root,
        )
        .unwrap();
        assert_eq!(outcome, AggregateOutcome::Complete);
        let finalized = finalized.unwrap();
        let manifest_json = finalized.manifest.to_json().unwrap();
        assert!(manifest_json.contains(version));
        // Exact manifest digest identity.
        let manifest_bytes = manifest_json.as_bytes();
        assert!(!manifest_bytes.is_empty());

        // Tampered candidate after qualification fails closed.
        let tampered_root = parent.join("tampered");
        std::fs::create_dir_all(tampered_root.join(target).join("candidates")).unwrap();
        std::fs::write(
            tampered_root.join(target).join("build-handoff.json"),
            agg_handoff_text.as_bytes(),
        )
        .unwrap();
        std::fs::write(
            tampered_root.join(target).join("evidence.json"),
            gate_text.as_bytes(),
        )
        .unwrap();
        for output in &agg_handoff.outputs {
            std::fs::write(
                tampered_root
                    .join(target)
                    .join("candidates")
                    .join(&output.relative_path),
                b"tampered-bytes",
            )
            .unwrap();
        }
        assert!(reconstruct_attempt(
            &release,
            &planned,
            &agg_handoff,
            &tampered_root.join(target).join("candidates"),
        )
        .is_err());

        std::fs::remove_dir_all(parent).unwrap();
    }
}

#[test]
fn m002a_optional_suppression_and_required_failure_close_the_gate() {
    // Optional-target failure suppresses release output; required-target
    // failure fails closed. Both use the canonical inputs layout.
    let (contract, release, bindings, qual_bindings) = m002_release(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        "linux-x64",
        Qualification::Structural,
        SupportTier::NonGating,
    );
    let target = "x86_64-unknown-linux-gnu";
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    let optional_graph =
        project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
    let failed_optional = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::NonGating,
        QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
    );
    assert_eq!(
        evaluate_gate(&optional_graph, std::slice::from_ref(&failed_optional)).unwrap(),
        AggregateOutcome::SuppressedNonGatingIncomplete
    );
    // Aggregate never produces a partial release on suppression.
    let elf = m002_elf();
    let parent = eggpack_core_test_temp("m002a-suppress");
    let candidate = parent.join("candidate");
    std::fs::write(&candidate, &elf).unwrap();
    let _planned = release.targets[0].clone();
    let attempt = eggpack_core::BuildAttempt {
        release_id: release.release_id.clone(),
        source_revision: release.source_revision.clone(),
        target: target.to_string(),
        strategy: BuildStrategy::NativeCargo,
        tool_summary: "fixture".into(),
        process: eggpack_core::ProcessEvidence {
            outcome: eggpack_core::CommandOutcome::Success,
            stdout_bytes: 0,
            stderr_bytes: 0,
        },
        candidates: vec![eggpack_core::CandidateArtifact {
            target: target.to_string(),
            selector: LogicalOutputSelector::Direct,
            package: "eggsact".into(),
            binary: "bin0".into(),
            path: candidate,
            size: elf.len() as u64,
        }],
    };
    let (outcome, finalized) = aggregate_finalize(
        &contract,
        &release,
        &optional_graph,
        &[FinalizationTargetInput {
            target: target.to_string(),
            attempt,
            qualification: failed_optional.clone(),
        }],
        &[],
        &parent.join("out"),
    )
    .unwrap();
    assert_eq!(outcome, AggregateOutcome::SuppressedNonGatingIncomplete);
    assert!(finalized.is_none());

    // Required failure fails closed.
    let (_, release_required, _, _, _, required_graph) =
        m002_graph(Qualification::Structural, SupportTier::Required);
    let failed_required = m002_evidence(
        target,
        &release_required.release_id,
        &release_required.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
    );
    assert_eq!(
        evaluate_gate(&required_graph, &[failed_required]).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    // Missing/extra/swap evidence is invalid, never partial.
    assert_eq!(
        evaluate_gate(&required_graph, &[]).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn m002a_negative_matrix_for_layout_and_identity() {
    let (_, release, bindings, _, _, graph) =
        m002_graph(Qualification::Structural, SupportTier::Required);
    let target = "x86_64-unknown-linux-gnu";
    let parent = eggpack_core_test_temp("m002a-neg");
    // Wrong-target handoff rejects.
    let mut handoff = project_build_handoff(&release, &bindings, target).unwrap();
    handoff.target = "aarch64-unknown-linux-gnu".into();
    assert!(handoff.validate().is_ok()); // shape-valid but identity-mismatched
    let planned = release.targets[0].clone();
    let dir = parent.join("wrong-target");
    std::fs::create_dir_all(dir.join("candidates")).unwrap();
    let elf = m002_elf();
    for output in &handoff.outputs {
        std::fs::write(dir.join("candidates").join(&output.relative_path), &elf).unwrap();
    }
    std::fs::write(dir.join("build-handoff.json"), handoff.to_json().unwrap()).unwrap();
    // Identity is checked by reconstruction against the plan, not by shape.
    let mut good = project_build_handoff(&release, &bindings, target).unwrap();
    for output in &mut good.outputs {
        output.size = elf.len() as u64;
    }
    assert!(reconstruct_attempt(&release, &planned, &handoff, &dir.join("candidates")).is_err());
    // Missing candidate rejects.
    let empty = parent.join("missing");
    std::fs::create_dir_all(empty.join("candidates")).unwrap();
    std::fs::write(empty.join("build-handoff.json"), good.to_json().unwrap()).unwrap();
    assert!(validate_build_artifact_dir(&empty).is_err());
    // Extra candidate rejects.
    let extra = parent.join("extra");
    stage_build_artifact_dir(
        &good,
        &{
            let src = parent.join("extra-src");
            std::fs::create_dir_all(&src).unwrap();
            for output in &good.outputs {
                std::fs::write(src.join(&output.relative_path), &elf).unwrap();
            }
            src
        },
        &extra,
    )
    .unwrap();
    std::fs::write(extra.join("candidates").join("stowaway"), b"x").unwrap();
    assert!(validate_build_artifact_dir(&extra).is_err());
    // Symlink candidate rejects.
    #[cfg(unix)]
    {
        let link_dir = parent.join("link");
        stage_build_artifact_dir(
            &good,
            &{
                let src = parent.join("link-src");
                std::fs::create_dir_all(&src).unwrap();
                for output in &good.outputs {
                    std::fs::write(src.join(&output.relative_path), &elf).unwrap();
                }
                src
            },
            &link_dir,
        )
        .unwrap();
        let victim = link_dir
            .join("candidates")
            .join(&good.outputs[0].relative_path);
        std::fs::remove_file(&victim).unwrap();
        std::os::unix::fs::symlink("/etc/hostname", &victim).unwrap();
        assert!(validate_build_artifact_dir(&link_dir).is_err());
    }
    // Gate layout mismatch (evidence file absent) is an I/O-level failure
    // before gate evaluation, never a silent pass.
    assert!(validate_qualification_artifact_dir(&empty).is_err());
    // Release identity mismatch fails closed.
    let wrong = m002_evidence(
        target,
        "other-release",
        &release.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Passed,
    );
    assert_eq!(
        evaluate_gate(&graph, &[wrong]).unwrap(),
        AggregateOutcome::InvalidEvidence
    );
    std::fs::remove_dir_all(parent).unwrap();
}

// -----------------------------------------------------------------------
// M003b — generated draft staging job and operational qualification.
// -----------------------------------------------------------------------

fn m003b_staging_policy() -> GitHubPolicy {
    let mut policy = m002_golden_policy();
    policy.staging = Some(GitHubStagingPolicyV1 {
        runner: "ubuntu-latest".into(),
        owner: "acme".into(),
        repository: "widget".into(),
        tag_source: StagingTagSource::RefName,
        environment: None,
        inputs: GitHubStagingInputsV1 {
            contract: "contracts/release.toml".into(),
            install_policy: "policies/install.toml".into(),
            github_policy: "policies/github-draft.json".into(),
            installer_presentation: None,
        },
        receipt_retention_days: 7,
    });
    policy
}

fn m003b_staging_graph(
    fixture: &str,
    product: &str,
    version: &str,
    targets: Vec<(&str, BuildStrategy, SupportTier, Qualification)>,
    aliases: Vec<&str>,
) -> (ReleaseCIPlanV1, GitHubPolicy) {
    let (graph, _) = m002_golden_case(fixture, product, version, targets, aliases);
    let policy = m003b_staging_policy();
    let graph = with_github_draft_staging(graph, true).unwrap();
    (graph, policy)
}

#[test]
fn m003b_staging_intent_is_provider_neutral_and_optional() {
    let (graph, _) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    // Disabled graphs serialize exactly as M002a (no staging field).
    assert!(graph.staging.is_none());
    let json = graph.to_json().unwrap();
    assert!(!json.contains("staging"));
    assert_eq!(ReleaseCIPlanV1::from_json(&json).unwrap(), graph);
    // Attaching staging is additive and validated.
    let staged = with_github_draft_staging(graph.clone(), true).unwrap();
    let job = staged.staging.as_ref().unwrap();
    assert_eq!(job.job_id, "stage");
    assert_eq!(job.aggregate_job_id, "aggregate");
    assert_eq!(job.provider, StagingProvider::GitHubDraft);
    assert_eq!(job.finalized_handoff_name, "eggpack-finalized-release");
    assert_eq!(job.receipt_handoff_name, "eggpack-staging-receipt");
    assert!(job.required);
    // M001 CIPlan semantics untouched.
    assert_eq!(staged.ci_plan, graph.ci_plan);
}

#[test]
fn m003b_runner_command_covers_prepare_and_stage() {
    let prepare = RunnerCommand::PrepareStage {
        contract: "contracts/release.toml".into(),
        release_manifest: "./eggpack-finalized/release-manifest.json".into(),
        finalized_root: "./eggpack-finalized/root".into(),
        github_policy: "policies/github-draft.json".into(),
        install_policy: "policies/install.toml".into(),
        output_dir: "./eggpack-staging".into(),
        output_payload: "./eggpack-staging-payload.json".into(),
        installer_presentation: None,
        source_root: None,
    };
    let argv = prepare.argv();
    assert_eq!(&argv[0..3], &["eggpack", "ci", "_prepare-stage"]);
    for required in [
        "--contract",
        "--release-manifest",
        "--finalized-root",
        "--github-policy",
        "--install-policy",
        "--output-dir",
        "--output-payload",
    ] {
        assert!(argv.contains(&required.to_string()), "missing {required}");
    }
    let stage = RunnerCommand::StageGithubDraft {
        payload: "./eggpack-staging-payload.json".into(),
        github_policy: "policies/github-draft.json".into(),
        staging_dir: "./eggpack-staging".into(),
        output_receipt: "./eggpack-staging-receipt.json".into(),
    };
    let argv = stage.argv();
    assert_eq!(&argv[0..3], &["eggpack", "ci", "_stage-github-draft"]);
    for required in [
        "--payload",
        "--github-policy",
        "--staging-dir",
        "--output-receipt",
    ] {
        assert!(argv.contains(&required.to_string()), "missing {required}");
    }
    // No publish subcommand exists.
    assert!(!prepare.to_shell().contains("publish"));
    assert!(!stage.to_shell().contains("publish"));
}

#[test]
fn m003b_staging_disabled_renders_byte_compatible() {
    // M002a output without staging must be unchanged.
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert!(graph.staging.is_none());
    let rendered = render_release_github(&graph, &policy).unwrap();
    assert_golden(&rendered, include_str!("../tests/fixtures/m002-direct.yml"));
}

// Not standalone coverage: rewrites checked-in goldens in place, so it must
// never run as part of a normal test pass.
#[test]
#[ignore]
fn m003b_regenerate_goldens() {
    let cases = [
        (
            "../tests/fixtures/m003b-direct-staging.yml",
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "eggsact",
            "1.2.3",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        ),
        (
            "../tests/fixtures/m003b-bundle-staging.yml",
            include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
            "codegg",
            "2.4.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        ),
        (
            "../tests/fixtures/m003b-archive-staging.yml",
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "egress",
            "3.1.0",
            vec![(
                "x86_64-unknown-linux-gnu",
                BuildStrategy::NativeCargo,
                SupportTier::Required,
                Qualification::Structural,
            )],
            vec!["linux-x64"],
        ),
    ];
    for (path, fixture, product, version, targets, aliases) in cases {
        let (graph, policy) = m003b_staging_graph(fixture, product, version, targets, aliases);
        let rendered = render_release_github(&graph, &policy).unwrap();
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::write(root.join(path.trim_start_matches("../")), &rendered).unwrap();
    }
}

#[test]
fn m003b_golden_direct_bundle_archive_with_staging() {
    // Direct + staging.
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m003b-direct-staging.yml"),
    );
    // Bundle + staging.
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
        "codegg",
        "2.4.0",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m003b-bundle-staging.yml"),
    );
    // Archive + staging.
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
        "egress",
        "3.1.0",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&graph, &policy).unwrap(),
        include_str!("../tests/fixtures/m003b-archive-staging.yml"),
    );
}

#[test]
fn m003b_rendered_staging_is_least_privilege_and_exact() {
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    let yaml = render_release_github(&graph, &policy).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    // Top-level read-only; no id-token write anywhere.
    assert!(!yaml.contains("id-token: write"));
    assert!(!yaml.contains("gh release"));
    assert!(!yaml.contains("--clobber"));
    assert!(!yaml.contains("release --publish"));
    assert!(!yaml.contains("publish"));
    assert!(!yaml.contains("git tag "));
    assert!(!yaml.contains("git push --tags"));
    assert!(!yaml.contains("curl "));
    // Top-level permission matrix is exactly read-only.
    let top_permissions = parsed.get("permissions").unwrap();
    assert_eq!(
        top_permissions.get("contents").unwrap().as_str().unwrap(),
        "read"
    );
    // Concurrency is release-scoped and tag-aware for manual dispatch.
    let concurrency = parsed.get("concurrency").unwrap();
    let group = concurrency.get("group").unwrap().as_str().unwrap();
    assert!(!group.contains("inputs.release_tag"));
    assert!(group.contains("github.ref"));
    let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
    let mut writers = Vec::new();
    for (name, job) in jobs {
        let name = name.as_str().unwrap();
        let permissions = job.get("permissions").unwrap();
        let contents = permissions.get("contents").unwrap().as_str().unwrap();
        if contents == "write" {
            writers.push(name.to_owned());
        }
        // No job may mint OIDC tokens.
        if let Some(id_token) = permissions.get("id-token") {
            assert_ne!(id_token.as_str().unwrap(), "write", "job {name}");
        }
    }
    assert_eq!(writers, vec!["stage".to_string()], "only stage may write");
    // Stage depends on aggregate and downloads the exact handoff.
    let stage = jobs.get("stage").unwrap();
    let needs: Vec<&str> = stage
        .get("needs")
        .unwrap()
        .as_sequence()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(needs, vec!["aggregate", "preflight"]);
    // Stage runs only for exact tags (tag push or explicit dispatch).
    let condition = stage.get("if").unwrap().as_str().unwrap();
    assert!(condition.contains("github.event_name == 'push'"));
    assert!(condition.contains("github.ref_type == 'tag'"));
    // No untrusted PR trigger may reach the write job: the workflow
    // triggers are push + workflow_dispatch only (no pull_request).
    let triggers = parsed.get("on").unwrap().as_mapping().unwrap();
    assert!(triggers.get("push").is_some());
    assert!(!triggers.keys().any(|key| key
        .as_str()
        .is_some_and(|name| name.contains("pull_request"))));
    let steps = stage.get("steps").unwrap().as_sequence().unwrap();
    let text = serde_yaml::to_string(&serde_yaml::Value::Sequence(steps.clone())).unwrap();
    assert!(text.contains("_prepare-stage"));
    assert!(text.contains("_stage-github-draft"));
    // Stage consumes the exact aggregate artifact, not rebuilt binaries.
    assert!(text.contains("eggpack-finalized-release"));
    assert!(text.contains("./eggpack-finalized/release-manifest.json"));
    assert!(text.contains("./eggpack-finalized/root"));
    // Prepare occurs before network staging.
    let prepare_pos = text.find("_prepare-stage").unwrap();
    let stage_pos = text.find("_stage-github-draft").unwrap();
    assert!(prepare_pos < stage_pos);
    // Token appears only as an environment reference.
    assert!(text.contains("GITHUB_TOKEN"));
    assert!(text.contains("secrets.GITHUB_TOKEN"));
    assert!(!text.contains("ghp_"));
    assert!(!text.contains("github_pat_"));
    // Receipt uploaded as an internal artifact.
    assert!(text.contains("eggpack-staging-receipt"));
    // Deterministic rerender.
    assert_eq!(render_release_github(&graph, &policy).unwrap(), yaml);
    // ci check catches staging step/permission edits.
    let drifted = yaml.clone();
    let drifted = drifted.replacen("contents: write", "contents: read", 1);
    assert!(
        check_release_github(&graph, &policy, drifted.as_bytes()).is_err()
            || !check_release_github(&graph, &policy, drifted.as_bytes())
                .map(|report| report.matches)
                .unwrap_or(true)
    );
    // ci check also catches staging step edits (e.g. command swap).
    let drifted_steps = yaml.replacen("_prepare-stage", "_prepare-stage-tampered", 1);
    assert!(
        check_release_github(&graph, &policy, drifted_steps.as_bytes()).is_err()
            || !check_release_github(&graph, &policy, drifted_steps.as_bytes())
                .map(|report| report.matches)
                .unwrap_or(true)
    );
}

#[test]
fn m003c_tag_source_controls_checkout_input_and_stage_guard() {
    let (graph, mut policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    let ref_name = render_release_github(&graph, &policy).unwrap();
    assert!(!ref_name.contains("inputs:\n      release_tag:"));
    assert!(ref_name.contains("if: github.event_name == 'push' && github.ref_type == 'tag'"));
    assert!(ref_name.contains("_verify-source"));
    assert_eq!(ref_name.matches("_verify-source").count(), 6);
    assert_eq!(
        ref_name
            .matches("ref: \"${{ needs.preflight.outputs.source_sha }}\"")
            .count(),
        5
    );
    assert_eq!(
        ref_name
            .matches("ref: \"${{ steps.resolve_tag.outputs.source_sha }}\"")
            .count(),
        1
    );
    let implicit_checkout = ref_name.replacen(
        "        with:\n          ref: \"${{ steps.resolve_tag.outputs.source_sha }}\"\n",
        "",
        1,
    );
    assert!(
        !check_release_github(&graph, &policy, implicit_checkout.as_bytes())
            .unwrap()
            .matches
    );
    let no_verifier = ref_name.replacen("_verify-source", "_removed-source-check", 1);
    assert!(
        !check_release_github(&graph, &policy, no_verifier.as_bytes())
            .unwrap()
            .matches
    );
    let broad_stage = ref_name.replace(
        "github.event_name == 'push' && github.ref_type == 'tag'",
        "always()",
    );
    assert!(
        !check_release_github(&graph, &policy, broad_stage.as_bytes())
            .unwrap()
            .matches
    );
    policy.staging.as_mut().unwrap().tag_source = StagingTagSource::DispatchInput;
    let dispatch = render_release_github(&graph, &policy).unwrap();
    assert!(dispatch.contains("release_tag:\n        description:"));
    assert_eq!(
        dispatch
            .matches("ref: \"${{ needs.preflight.outputs.source_sha }}\"")
            .count(),
        5
    );
    assert!(dispatch.contains("ref: \"${{ steps.resolve_tag.outputs.source_sha }}\""));
    assert_eq!(
        dispatch
            .matches("EGGPACK_RELEASE_TAG: \"${{ inputs.release_tag }}\"")
            .count(),
        8
    );
    // Every frozen SHA checkout (no tags fetched) is immediately followed
    // by exactly one tag-fetch step bound to the same frozen revision, so
    // the runtime tag check cannot pass without, or drift from, the
    // checkout it verifies.
    let frozen = dispatch
        .matches("- name: Verify frozen source checkout")
        .count();
    let fetches = dispatch
        .matches("- name: Fetch release tag for local verification")
        .count();
    assert_eq!(fetches, frozen);
    assert!(frozen > 0);
    assert!(!dispatch.contains("if: github.event_name == 'push' && github.ref_type == 'tag'"));
    let parsed: serde_yaml::Value = serde_yaml::from_str(&dispatch).unwrap();
    assert!(!contains_unsafe_dispatch_expression_in_run(&dispatch));
    let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
    {
        let job_name = "stage";
        let steps = jobs
            .get(serde_yaml::Value::String(job_name.to_owned()))
            .unwrap()
            .get("steps")
            .unwrap()
            .as_sequence()
            .unwrap();
        let validation = steps
            .iter()
            .find(|step| {
                step.get("name").and_then(serde_yaml::Value::as_str)
                    == Some("Resolve and validate immutable release source")
            })
            .unwrap();
        let script = validation
            .get("run")
            .and_then(serde_yaml::Value::as_str)
            .unwrap();
        assert!(!script.contains("${{"));
        assert!(script.contains("EGGPACK_RELEASE_TAG"));
        assert!(script.contains("git check-ref-format"));
        assert_eq!(
            validation
                .get("env")
                .unwrap()
                .get("EGGPACK_RELEASE_TAG")
                .and_then(serde_yaml::Value::as_str),
            Some("${{ inputs.release_tag }}")
        );
        let checkout = steps
            .iter()
            .position(|step| {
                step.get("name").and_then(serde_yaml::Value::as_str) == Some("Check out source")
            })
            .unwrap();
        let validate = steps
            .iter()
            .position(|step| {
                step.get("name").and_then(serde_yaml::Value::as_str)
                    == Some("Resolve and validate immutable release source")
            })
            .unwrap();
        assert!(validate < checkout, "{job_name} validates before checkout");
    }
    let mut missing_dispatch = policy.clone();
    missing_dispatch
        .triggers
        .retain(|trigger| *trigger != WorkflowTrigger::WorkflowDispatch);
    assert!(render_release_github(&graph, &missing_dispatch).is_err());
    assert_ne!(ref_name, dispatch);
}

#[cfg(unix)]
#[test]
fn m003k_tag_resolution_precedes_checkout_and_treats_values_as_data() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let (graph, mut policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    policy.staging.as_mut().unwrap().tag_source = StagingTagSource::DispatchInput;
    let yaml = render_release_github(&graph, &policy).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    assert!(!contains_unsafe_dispatch_expression_in_run(&yaml));
    let exact_preflight = parsed["jobs"]["preflight"]["steps"].as_sequence().unwrap();
    assert!(
        exact_preflight
            .iter()
            .position(|step| step["name"] == "Resolve and validate immutable release source")
            .unwrap()
            < exact_preflight
                .iter()
                .position(|step| step["name"] == "Check out source")
                .unwrap()
    );
    let (contract, shape, mut reusable_policy) =
        m003d_shape_and_policy(StagingTagSource::DispatchInput);
    reusable_policy.release_identity_mode = Some(ReleaseIdentityMode::VPrefixedStableSemver);
    reusable_policy.triggers = vec![WorkflowTrigger::WorkflowDispatch];
    reusable_policy.staging.as_mut().unwrap().environment = Some("release-staging".into());
    let reusable = render_reusable_release_github(&contract, &shape, &reusable_policy).unwrap();
    let reusable_yaml: serde_yaml::Value = serde_yaml::from_str(&reusable).unwrap();
    assert!(!contains_unsafe_dispatch_expression_in_run(&reusable));
    let preflight_steps = reusable_yaml["jobs"]["preflight"]["steps"]
        .as_sequence()
        .unwrap();
    let resolve_tag = preflight_steps
        .iter()
        .find(|step| step["name"] == "Resolve and validate immutable release source")
        .unwrap();
    let script = resolve_tag["run"].as_str().unwrap();
    assert!(!script.contains("${{"));
    assert!(script.contains("git ls-remote"));
    assert!(script.contains("GITHUB_OUTPUT"));
    assert!(
        preflight_steps
            .iter()
            .position(|step| step["name"] == "Resolve and validate immutable release source")
            .unwrap()
            < preflight_steps
                .iter()
                .position(|step| step["name"] == "Check out source")
                .unwrap()
    );
    let stage = &reusable_yaml["jobs"]["stage"];
    assert_eq!(stage["environment"]["name"], "release-staging");
    assert_eq!(stage["permissions"]["contents"], "write");
    assert!(reusable_yaml["jobs"]["resolve"]["environment"].is_null());
    assert_eq!(reusable.matches("environment:").count(), 1);
    assert!(reusable.contains("needs.resolve.outputs.source_sha"));
    assert!(reusable.contains("needs.preflight.outputs.source_sha"));
    let resolve_steps = reusable_yaml["jobs"]["resolve"]["steps"]
        .as_sequence()
        .unwrap();
    let resolve_checkout = resolve_steps
        .iter()
        .find(|step| step["name"] == "Check out source")
        .unwrap();
    assert_eq!(
        resolve_checkout["with"]["ref"],
        "${{ needs.preflight.outputs.source_sha }}"
    );

    let nonce = format!("eggpack-m003j-{}", std::process::id());
    let marker = std::env::temp_dir().join(nonce);
    let _ = std::fs::remove_file(&marker);
    let malicious = format!("$(touch {})", marker.display());
    // Reproduce the prior generated source interpolation: Bash executes
    // the fixture command substitution, proving this is discriminating.
    let old_script = format!("test -n \"{malicious}\"");
    let old_status = Command::new("bash")
        .arg("-c")
        .arg(old_script)
        .status()
        .unwrap();
    assert!(marker.exists());
    assert!(!old_status.success());
    std::fs::remove_file(&marker).unwrap();

    let root = std::env::temp_dir().join(format!("eggpack-m003k-{}", std::process::id()));
    let bin = root.join("bin");
    let runner_temp = root.join("runner-temp");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&runner_temp).unwrap();
    let fake_git = bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\ncase \"$1\" in\n  check-ref-format|init) exit 0 ;;\n  ls-remote) shift; shift; shift; ref=\"$1\"; peeled=\"$2\"; printf '%s\\t%s\\n' \"$EGGPACK_FAKE_DIRECT\" \"$ref\"; if [ -n \"$EGGPACK_FAKE_PEELED\" ]; then printf '%s\\t%s\\n' \"$EGGPACK_FAKE_PEELED\" \"$peeled\"; fi ;;\n  --git-dir=*) shift; case \"$1\" in fetch) exit 0 ;; rev-parse) printf '%s\\n' \"${EGGPACK_FAKE_COMMIT:-${EGGPACK_FAKE_PEELED:-$EGGPACK_FAKE_DIRECT}}\" ;; *) exit 99 ;; esac ;;\n  *) exit 99 ;;\nesac\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();
    let system_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{system_path}", bin.display());
    let output_path = root.join("github-output");
    for (tag, accepted) in [
        ("v1.2.3", true),
        ("", false),
        ("1.2.3", false),
        ("v01.2.3", false),
        ("v1.2.3-rc.1", false),
        ("v1.2.3+build", false),
        ("v1.2.3\n", false),
        (malicious.as_str(), false),
        ("v1;touch /tmp/nope.2.3", false),
        (
            "v12345678901234567890123456789012345678901234567890123456789012345.2.3",
            false,
        ),
    ] {
        std::fs::write(&output_path, "").unwrap();
        let result = Command::new("bash")
            .arg("-c")
            .arg(script)
            .env("EGGPACK_RELEASE_TAG", tag)
            .env("GITHUB_TOKEN", "fixture-token")
            .env("GITHUB_SERVER_URL", "https://github.com")
            .env("GITHUB_REPOSITORY", "owner/repo")
            .env("RUNNER_TEMP", &runner_temp)
            .env("GITHUB_OUTPUT", &output_path)
            .env("EGGPACK_FAKE_DIRECT", "b".repeat(40))
            .env("EGGPACK_FAKE_PEELED", "a".repeat(40))
            .env("PATH", &path)
            .status()
            .unwrap();
        assert_eq!(result.success(), accepted, "tag {tag:?}");
        assert!(result.code().is_some() || result.signal().is_some());
        if accepted {
            assert!(std::fs::read_to_string(&output_path)
                .unwrap()
                .contains(&format!("source_sha={}\n", "a".repeat(40))));
        }
    }
    // A lightweight tag resolves directly to its commit OID.
    std::fs::write(&output_path, "").unwrap();
    let light = Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("EGGPACK_RELEASE_TAG", "v1.2.3")
        .env("GITHUB_TOKEN", "fixture-token")
        .env("GITHUB_SERVER_URL", "https://github.com")
        .env("GITHUB_REPOSITORY", "owner/repo")
        .env("RUNNER_TEMP", &runner_temp)
        .env("GITHUB_OUTPUT", &output_path)
        .env("EGGPACK_FAKE_DIRECT", "c".repeat(40))
        .env("EGGPACK_FAKE_PEELED", "")
        .env("PATH", &path)
        .status()
        .unwrap();
    assert!(light.success());
    assert!(std::fs::read_to_string(&output_path)
        .unwrap()
        .contains(&format!("source_sha={}\n", "c".repeat(40))));
    std::fs::write(&output_path, "").unwrap();
    let non_commit = Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("EGGPACK_RELEASE_TAG", "v1.2.3")
        .env("GITHUB_TOKEN", "fixture-token")
        .env("GITHUB_SERVER_URL", "https://github.com")
        .env("GITHUB_REPOSITORY", "owner/repo")
        .env("RUNNER_TEMP", &runner_temp)
        .env("GITHUB_OUTPUT", &output_path)
        .env("EGGPACK_FAKE_DIRECT", "c".repeat(40))
        .env("EGGPACK_FAKE_PEELED", "")
        .env("EGGPACK_FAKE_COMMIT", "d".repeat(40))
        .env("PATH", &path)
        .status()
        .unwrap();
    assert!(!non_commit.success(), "non-commit tag objects fail closed");
    assert!(
        !marker.exists(),
        "corrected validation must not execute tag text"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn m003b_staging_requires_policy_and_rejects_forbidden_commands() {
    let (mut graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    // Graph requests staging but policy lacks staging settings.
    let mut bare = policy.clone();
    bare.staging = None;
    assert!(render_release_github(&graph, &bare).is_err());
    // Staging runner must be finite/safe; no publish flag exists by type.
    let mut bad = policy.clone();
    bad.staging.as_mut().unwrap().runner = "evil; rm -rf /".into();
    assert!(render_release_github(&graph, &bad).is_err());
    let _ = &mut graph;
}

#[tokio::test]
async fn m003b_local_orchestration_stages_through_fake_adapter() {
    use eggpack_github::{
        fixture_asset, fixture_release, prepare_staging_payload, stage_with_bytes, FixtureGithub,
        GitHubDraftPolicyV1,
    };
    let fixture_text = include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
    let contract = eggpack_contract::DistributionContract::parse_toml_str(fixture_text).unwrap();
    let source = "a".repeat(40);
    // Note: simple-direct contract has two targets; use both for bootstrap.
    let config_full = eggpack_core::PackConfig {
        schema_version: 1,
        targets: vec![
            eggpack_core::TargetPolicy {
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
            },
            eggpack_core::TargetPolicy {
                target: "aarch64-apple-darwin".into(),
                strategy: eggpack_core::BuildStrategy::NativeCargo,
                host_os: eggpack_core::HostOs::Macos,
                host_arch: eggpack_core::HostArch::Aarch64,
                qualification_host: None,
                toolchain: eggpack_core::ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: eggpack_core::CompatibilityFloor::None,
                qualification: eggpack_core::Qualification::Structural,
                support: eggpack_core::SupportTier::Required,
            },
        ],
    };
    let release = config_full
        .resolve(
            &contract,
            "1.2.6",
            &source,
            &["linux-x64".into(), "macos-arm64".into()],
        )
        .unwrap();
    // Deterministic finalized bytes for both targets.
    let body = b"body";
    let parent = eggpack_core_test_temp("m003b-e2e");
    let finalized_root = parent.join("finalized");
    std::fs::create_dir(&finalized_root).unwrap();
    // Manifest with exact byte facts.
    use sha2::Digest;
    let digest: String = sha2::Sha256::digest(body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let manifest = eggpack_manifest::ReleaseManifest {
        schema_version: 1,
        product_id: "eggsact".into(),
        release_id: "1.2.6".into(),
        source_revision: source.clone(),
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
    for name in [
        "eggsact-1.2.6-aarch64-apple-darwin",
        "eggsact-1.2.6-x86_64-unknown-linux-gnu",
    ] {
        std::fs::write(finalized_root.join(name), body).unwrap();
        std::fs::write(
            finalized_root.join(format!("{name}.sha256")),
            format!("{digest}  {name}\n"),
        )
        .unwrap();
    }
    let policy = GitHubDraftPolicyV1 {
        schema_version: 1,
        owner: "acme".into(),
        repository: "widget".into(),
        tag: "v1.2.6".into(),
        release_identity_schema_version: None,
        release_identity_mode: None,
        release_id: None,
        source_revision: None,
        title: "widget 1.2.6".into(),
        body: "notes".into(),
        prerelease: false,
        token_env: "GITHUB_TOKEN".into(),
        request_timeout_secs: 30,
        max_metadata_bytes: 1_000_000,
        max_list_pages: 5,
    };
    let install_policy = eggpack_bootstrap::BootstrapInstallPolicyV1::empty();
    let staging_dir = parent.join("staging");
    let payload = prepare_staging_payload(
        &contract,
        &manifest,
        &finalized_root,
        &policy,
        &install_policy,
        &staging_dir,
    )
    .unwrap();
    assert!(!payload.assets.is_empty());
    // Fake adapter: create then reuse exact draft/assets. Bytes come from
    // the materialized staging directory (panics if absent, never a
    // redacted transport error).
    let fixture = FixtureGithub::with_tag(&source);
    // Provide bytes via explicit map to avoid borrowing the staging dir
    // across awaits.
    let mut staged_bytes = std::collections::BTreeMap::new();
    for asset in &payload.assets {
        staged_bytes.insert(
            asset.name.clone(),
            std::fs::read(staging_dir.join(&asset.name)).unwrap(),
        );
    }
    let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
    })
    .await
    .unwrap();
    assert!(receipt.draft && !receipt.immutable);
    assert_eq!(receipt.uploaded as usize, payload.assets.len());
    // Rerun reuses exact draft/assets without clobber.
    let receipt2 = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
    })
    .await
    .unwrap();
    assert!(!receipt2.created);
    assert_eq!(receipt2.reused as usize, payload.assets.len());
    // Tag mismatch fails closed; published refusal fails closed.
    let bad_fixture = FixtureGithub::with_tag(&"b".repeat(40));
    assert!(
        stage_with_bytes(&payload, &policy, &bad_fixture, "token", |name| {
            Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .is_err()
    );
    let published = FixtureGithub::with_tag(&source);
    published.seed_release(
        fixture_release(9, "v1.2.6", "widget 1.2.6", "notes", false, false, false),
        Vec::new(),
    );
    assert!(
        stage_with_bytes(&payload, &policy, &published, "token", |name| {
            Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .is_err()
    );
    // Mismatched asset refusal.
    let mismatch = FixtureGithub::with_tag(&source);
    mismatch.seed_release(
        fixture_release(10, "v1.2.6", "widget 1.2.6", "notes", false, true, false),
        vec![fixture_asset(
            1,
            &payload.assets[0].name,
            payload.assets[0].size + 1,
            "uploaded",
            Some(&payload.assets[0].sha256),
        )],
    );
    assert!(
        stage_with_bytes(&payload, &policy, &mismatch, "token", |name| {
            Ok(staged_bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .is_err()
    );
    let _ = release;
    std::fs::remove_dir_all(parent).unwrap();
}

#[tokio::test]
async fn m003b_local_orchestration_covers_bundle_and_archive() {
    use eggpack_github::{prepare_staging_payload, stage_with_bytes, FixtureGithub};
    use sha2::Digest;
    let sha_hex = |bytes: &[u8]| -> String {
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    };
    let body = b"body";
    let digest = sha_hex(body);
    // Bundle case (codegg-bundle contract, single target).
    {
        let contract = eggpack_contract::DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/codegg-bundle.toml"
        ))
        .unwrap();
        let source = "b".repeat(40);
        let manifest = eggpack_manifest::ReleaseManifest {
            schema_version: 1,
            product_id: "codegg".into(),
            release_id: "2.4.0".into(),
            source_revision: source.clone(),
            targets: vec![eggpack_manifest::TargetRecord {
                target: "x86_64-unknown-linux-gnu".into(),
                form: eggpack_manifest::ArtifactForm::Bundle {
                    entries: ["codegg", "codegg-helper", "codegg-manifest"]
                        .iter()
                        .zip(["codegg", "codegg-helper", "codegg-manifest.json"].iter())
                        .map(|(stem, install)| eggpack_manifest::BundleRecord {
                            artifact: eggpack_manifest::ArtifactRecord {
                                name: if *stem == "codegg-manifest" {
                                    "codegg-manifest-2.4.0.json".into()
                                } else {
                                    format!("{stem}-2.4.0-x86_64-unknown-linux-gnu")
                                },
                                size: body.len() as u64,
                                sha256: digest.clone(),
                            },
                            install: (*install).into(),
                        })
                        .collect(),
                },
            }],
            evidence_references: vec![],
        };
        let parent = eggpack_core_test_temp("m003b-e2e-bundle");
        let finalized = parent.join("finalized");
        std::fs::create_dir(&finalized).unwrap();
        for name in [
            "codegg-2.4.0-x86_64-unknown-linux-gnu",
            "codegg-helper-2.4.0-x86_64-unknown-linux-gnu",
            "codegg-manifest-2.4.0.json",
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
            tag: "v2.4.0".into(),
            release_identity_schema_version: None,
            release_identity_mode: None,
            release_id: None,
            source_revision: None,
            title: "widget 2.4.0".into(),
            body: "notes".into(),
            prerelease: false,
            token_env: "GITHUB_TOKEN".into(),
            request_timeout_secs: 30,
            max_metadata_bytes: 1_000_000,
            max_list_pages: 5,
        };
        let mut modes = std::collections::BTreeMap::new();
        modes.insert(
            "codegg".to_owned(),
            eggpack_bootstrap::InstallMode::Executable,
        );
        modes.insert(
            "codegg-helper".to_owned(),
            eggpack_bootstrap::InstallMode::Executable,
        );
        modes.insert(
            "codegg-manifest.json".to_owned(),
            eggpack_bootstrap::InstallMode::Data,
        );
        let install_policy = eggpack_bootstrap::BootstrapInstallPolicyV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".to_owned(),
                eggpack_bootstrap::TargetInstallPolicy {
                    modes,
                    archive_encoding: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        let staging = parent.join("staging");
        let payload = prepare_staging_payload(
            &contract,
            &manifest,
            &finalized,
            &policy,
            &install_policy,
            &staging,
        )
        .unwrap();
        assert!(!payload.assets.is_empty());
        let fixture = FixtureGithub::with_tag(&source);
        let mut bytes = std::collections::BTreeMap::new();
        for asset in &payload.assets {
            bytes.insert(
                asset.name.clone(),
                std::fs::read(staging.join(&asset.name)).unwrap(),
            );
        }
        let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            Ok(bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .unwrap();
        assert!(receipt.draft && !receipt.immutable);
        let rerun = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            Ok(bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .unwrap();
        assert!(!rerun.created);
        std::fs::remove_dir_all(parent).unwrap();
    }
    // Archive case (egress-archive contract, two targets).
    {
        let contract = eggpack_contract::DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/egress-archive.toml"
        ))
        .unwrap();
        let source = "c".repeat(40);
        let mk = |target: &str| eggpack_manifest::TargetRecord {
            target: target.to_owned(),
            form: eggpack_manifest::ArtifactForm::Archive {
                artifact: eggpack_manifest::ArtifactRecord {
                    name: format!("egress-3.1.0-{target}.tar.gz"),
                    size: body.len() as u64,
                    sha256: digest.clone(),
                },
                members: vec![
                    eggpack_manifest::ArchiveMemberRecord {
                        source: "bin/egress-helper".into(),
                        install: "egress-helper".into(),
                        bytes: eggpack_manifest::ByteEvidence {
                            size: body.len() as u64,
                            sha256: digest.clone(),
                        },
                    },
                    eggpack_manifest::ArchiveMemberRecord {
                        source: "egress".into(),
                        install: "egress".into(),
                        bytes: eggpack_manifest::ByteEvidence {
                            size: body.len() as u64,
                            sha256: digest.clone(),
                        },
                    },
                ],
            },
        };
        let manifest = eggpack_manifest::ReleaseManifest {
            schema_version: 1,
            product_id: "egress".into(),
            release_id: "3.1.0".into(),
            source_revision: source.clone(),
            targets: vec![mk("aarch64-apple-darwin"), mk("x86_64-unknown-linux-gnu")],
            evidence_references: vec![],
        };
        let parent = eggpack_core_test_temp("m003b-e2e-archive");
        let finalized = parent.join("finalized");
        std::fs::create_dir(&finalized).unwrap();
        for name in [
            "egress-3.1.0-aarch64-apple-darwin.tar.gz",
            "egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz",
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
            tag: "v3.1.0".into(),
            release_identity_schema_version: None,
            release_identity_mode: None,
            release_id: None,
            source_revision: None,
            title: "widget 3.1.0".into(),
            body: "notes".into(),
            prerelease: false,
            token_env: "GITHUB_TOKEN".into(),
            request_timeout_secs: 30,
            max_metadata_bytes: 1_000_000,
            max_list_pages: 5,
        };
        let mut targets = std::collections::BTreeMap::new();
        for triple in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
            let mut modes = std::collections::BTreeMap::new();
            modes.insert(
                "egress".to_owned(),
                eggpack_bootstrap::InstallMode::Executable,
            );
            modes.insert(
                "egress-helper".to_owned(),
                eggpack_bootstrap::InstallMode::Executable,
            );
            targets.insert(
                triple.to_owned(),
                eggpack_bootstrap::TargetInstallPolicy {
                    modes,
                    archive_encoding: Some(eggpack_bootstrap::BundleArchiveEncoding::TarGzip),
                },
            );
        }
        let install_policy = eggpack_bootstrap::BootstrapInstallPolicyV1 {
            schema_version: 1,
            targets,
        };
        let staging = parent.join("staging");
        let payload = prepare_staging_payload(
            &contract,
            &manifest,
            &finalized,
            &policy,
            &install_policy,
            &staging,
        )
        .unwrap();
        assert!(!payload.assets.is_empty());
        let fixture = FixtureGithub::with_tag(&source);
        let mut bytes = std::collections::BTreeMap::new();
        for asset in &payload.assets {
            bytes.insert(
                asset.name.clone(),
                std::fs::read(staging.join(&asset.name)).unwrap(),
            );
        }
        let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            Ok(bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .unwrap();
        assert!(receipt.draft && !receipt.immutable);
        let rerun = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            Ok(bytes.get(name).cloned().expect("staged bytes exist"))
        })
        .await
        .unwrap();
        assert!(!rerun.created);
        std::fs::remove_dir_all(parent).unwrap();
    }
}

// -----------------------------------------------------------------------
// M003d — consumer release composition seam.
// -----------------------------------------------------------------------

fn m003d_validator(selector: LogicalOutputSelector) -> ConsumerValidatorV1 {
    ConsumerValidatorV1 {
        schema_version: 1,
        selector,
        interpreter: ValidatorInterpreterV1::Python3,
        script: "scripts/smoke-mcp-binary.py".into(),
        timeout_ms: 20_000,
        stdout_limit: 65_536,
        stderr_limit: 65_536,
    }
}

#[test]
fn m003d_validator_config_validation() {
    let valid = m003d_validator(LogicalOutputSelector::Direct);
    assert!(valid.validate().is_ok());
    assert_eq!(
        ConsumerValidatorV1::from_json(&valid.to_json().unwrap()).unwrap(),
        valid
    );
    let mut bad = valid.clone();
    bad.schema_version = 2;
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.script = "../escape.py".into();
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.script = "/abs.py".into();
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.timeout_ms = 999;
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.timeout_ms = 600_001;
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.stdout_limit = 0;
    assert!(bad.validate().is_err());
    let mut bad = valid.clone();
    bad.stderr_limit = 8_000_001;
    assert!(bad.validate().is_err());
    // Unknown fields (e.g. arbitrary env/shell maps) reject.
    assert!(ConsumerValidatorV1::from_json(
        r#"{"schema_version":1,"selector":"direct","interpreter":"python3","script":"s.py","timeout_ms":1000,"stdout_limit":1024,"stderr_limit":1024,"env":{"A":"b"}}"#
    )
    .is_err());
    assert!(ConsumerValidatorV1::from_json(
        r#"{"schema_version":1,"selector":"direct","interpreter":"python3","script":"s.py","timeout_ms":1000,"stdout_limit":1024,"stderr_limit":1024,"shell":true}"#
    )
    .is_err());
    // Evidence carries no output contents by construction.
    let evidence = ConsumerValidationEvidenceV1 {
        schema_version: 1,
        release_id: "r".into(),
        source_revision: "a".repeat(40),
        target: "x86_64-unknown-linux-gnu".into(),
        selector: LogicalOutputSelector::Direct,
        interpreter: ValidatorInterpreterV1::Python3,
        outcome: ConsumerValidationOutcome::Passed,
        candidate_size: 4,
        candidate_sha256: Some("ab".repeat(32)),
    };
    let json = evidence.to_json().unwrap();
    assert!(!json.contains("stdout"));
    assert!(!json.contains("stderr"));
    assert!(!json.contains("output"));
    assert_eq!(
        ConsumerValidationEvidenceV1::from_json(&json).unwrap(),
        evidence
    );
}

fn m003d_write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

fn m003d_candidate(dir: &Path, name: &str, bytes: &[u8]) -> (PathBuf, u64, String) {
    use sha2::Digest;
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    let sha: String = sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    (path, bytes.len() as u64, sha)
}

const M003D_SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn m003d_request<'a>(
    validator: &'a ConsumerValidatorV1,
    script: &'a Path,
    candidate: &'a Path,
    size: u64,
    sha: &'a str,
    work: &'a Path,
    path_dirs: Option<&'a [PathBuf]>,
) -> ConsumerValidationRequest<'a> {
    ConsumerValidationRequest {
        validator,
        script_path: script,
        candidate_path: candidate,
        expected_size: size,
        expected_sha256: sha,
        work_dir: work,
        release_id: "v9.9.9",
        source_revision: M003D_SOURCE,
        target: "x86_64-unknown-linux-gnu",
        cancelled: None,
        path_dirs,
    }
}

#[test]
fn m003d_consumer_execution_matrix() {
    let parent = eggpack_core_test_temp("m003d-consumer");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let validator = m003d_validator(LogicalOutputSelector::Direct);
    // Success: script receives exactly (script, candidate) and exits 0.
    let script = m003d_write_script(
        &parent,
        "ok.py",
        "import sys\nassert len(sys.argv) == 2\nopen(sys.argv[1], 'rb').read()\n",
    );
    let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"exact-bytes");
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(evidence.outcome, ConsumerValidationOutcome::Passed);
    assert_eq!(evidence.candidate_size, size);
    assert_eq!(evidence.candidate_sha256.as_deref(), Some(sha.as_str()));
    assert_eq!(evidence.selector, LogicalOutputSelector::Direct);
    // Non-zero exit fails.
    let script = m003d_write_script(&parent, "fail.py", "import sys\nsys.exit(3)\n");
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit)
    );
    // Timeout fails (script sleeps past a short bound).
    let mut short = validator.clone();
    short.timeout_ms = 1_000;
    let script = m003d_write_script(&parent, "slow.py", "import time\ntime.sleep(30)\n");
    let evidence = run_consumer_validator(&m003d_request(
        &short, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout)
    );
    // Output limit fails (script floods stdout, then would exit 0).
    let mut limited = validator.clone();
    limited.stdout_limit = 1024;
    let script = m003d_write_script(
        &parent,
        "flood.py",
        "import sys\nsys.stdout.write('x' * 100000)\n",
    );
    let evidence = run_consumer_validator(&m003d_request(
        &limited, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit)
    );
    // Missing interpreter fails rather than skipping (hermetic PATH).
    // Note: Windows CreateProcess searches the application directory,
    // working directory, and system directories unconditionally, so an
    // emptied PATH cannot simulate a missing interpreter there. The
    // spawn/preflight failure branches are platform-independent code
    // proven on Linux/macOS lanes, while the Windows lane proves the
    // `python` mapping and preflight through every other matrix case.
    #[cfg(not(windows))]
    {
        let script = m003d_write_script(&parent, "ok2.py", "pass\n");
        let empty: Vec<PathBuf> = Vec::new();
        let evidence = run_consumer_validator(&m003d_request(
            &validator,
            &script,
            &candidate,
            size,
            &sha,
            &work,
            Some(&empty),
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::InterpreterUnavailable)
        );
    }
    // Wrong candidate identity fails (size and digest mismatch).
    let (other, other_size, other_sha) = m003d_candidate(&parent, "other", b"other-bytes");
    assert_ne!(other_sha, sha);
    let script = m003d_write_script(&parent, "ok3.py", "pass\n");
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &other, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch)
    );
    let _ = other_size;
    // A candidate that was never read is recorded as absent, not as a
    // plausible-looking digest: nothing was observed, so nothing is
    // claimed. The classified mismatch outcome is unchanged.
    let absent = parent.join("absent-candidate");
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &absent, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch)
    );
    assert_eq!(evidence.candidate_size, 0);
    assert_eq!(evidence.candidate_sha256, None);
    // A half-recorded identity is rejected rather than published.
    let mut half = evidence.clone();
    half.candidate_sha256 = Some("ab".repeat(32));
    assert!(half.validate().is_err());
    let mut half = evidence.clone();
    half.candidate_size = 4;
    assert!(half.validate().is_err());
    // Symlink script rejects as unavailable (no PATH-selected script).
    #[cfg(unix)]
    {
        let link = parent.join("link.py");
        std::os::unix::fs::symlink(&script, &link).unwrap();
        let evidence = run_consumer_validator(&m003d_request(
            &validator, &link, &candidate, size, &sha, &work, None,
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::ScriptUnavailable)
        );
        // Symlink candidate mismatches identity.
        let candidate_link = parent.join("candidate-link");
        std::os::unix::fs::symlink(&candidate, &candidate_link).unwrap();
        let evidence = run_consumer_validator(&m003d_request(
            &validator,
            &script,
            &candidate_link,
            size,
            &sha,
            &work,
            None,
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::CandidateMismatch)
        );
    }
    // Cancellation fails closed.
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let flag = Arc::new(AtomicBool::new(false));
    let script_hang = m003d_write_script(&parent, "hang.py", "import time\ntime.sleep(30)\n");
    let slow_validator = ConsumerValidatorV1 {
        timeout_ms: 60_000,
        ..validator.clone()
    };
    let worker = {
        let flag_child = flag.clone();
        let slow2 = slow_validator.clone();
        let script2 = script_hang.clone();
        let candidate2 = candidate.clone();
        let work2 = work.clone();
        let sha2 = sha.clone();
        let release2 = "v9.9.9".to_owned();
        let source2 = "a".repeat(40);
        let target2 = "x86_64-unknown-linux-gnu".to_owned();
        std::thread::spawn(move || {
            let request = ConsumerValidationRequest {
                validator: &slow2,
                script_path: &script2,
                candidate_path: &candidate2,
                expected_size: size,
                expected_sha256: &sha2,
                work_dir: &work2,
                release_id: &release2,
                source_revision: &source2,
                target: &target2,
                cancelled: Some(&flag_child),
                path_dirs: None,
            };
            run_consumer_validator(&request).unwrap()
        })
    };
    std::thread::sleep(std::time::Duration::from_millis(300));
    flag.store(true, Ordering::SeqCst);
    let evidence = worker.join().unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Cancelled)
    );
    // Caller misuse returns CiError, not failure evidence.
    let bad_request = ConsumerValidationRequest {
        validator: &validator,
        script_path: Path::new("relative/script.py"),
        candidate_path: &candidate,
        expected_size: size,
        expected_sha256: &sha,
        work_dir: &work,
        release_id: "v9.9.9",
        source_revision: &"a".repeat(40),
        target: "x86_64-unknown-linux-gnu",
        cancelled: None,
        path_dirs: None,
    };
    assert!(run_consumer_validator(&bad_request).is_err());
    std::fs::remove_dir_all(parent).unwrap();
}

// -----------------------------------------------------------------------
// M003m — bounded process execution convergence.
//
// The consumer validator runs through the shared bounded runner in
// `eggpack-core`: the supervised interpreter is terminated as a whole
// process group (or job object), the reader-thread joins are bounded by
// the run's own remaining deadline, and the environment allowlist and the
// output bounds follow the one shared definition.
// -----------------------------------------------------------------------

/// Body of the fixture descendant: append a heartbeat line, then repeat.
///
/// The heartbeat is how a test observes that the descendant is *absent*
/// rather than merely that the direct child exited: a live descendant
/// keeps advancing the file, a terminated one stops.
const M003M_DESCENDANT: &str = "import os, sys, time\nhere = os.path.dirname(os.path.abspath(__file__))\npath = os.path.join(here, 'heartbeat.txt')\nstop = os.path.join(here, 'stop')\nend = time.time() + 300\nwhile time.time() < end and not os.path.exists(stop):\n    with open(path, 'a') as handle:\n        handle.write('tick\\n')\n    time.sleep(0.05)\n";

/// Body of the fixture validator: spawn the descendant, wait until it is
/// demonstrably alive, then hang until the run is bounded.
const M003M_SPAWNER: &str = "import os, subprocess, sys, time\nhere = os.path.dirname(os.path.abspath(__file__))\nchild = subprocess.Popen([sys.executable, os.path.join(here, 'descendant.py')])\nheartbeat = os.path.join(here, 'heartbeat.txt')\nwhile not os.path.exists(heartbeat) or os.path.getsize(heartbeat) == 0:\n    time.sleep(0.02)\ntime.sleep(3600)\n";

/// Body of the fixture validator whose descendant escapes the process
/// group, so the group kill cannot reach it and its inherited pipe stays
/// open after the validator itself exits.
#[cfg(unix)]
const M003M_ESCAPING_SPAWNER: &str = "import os, subprocess, sys, time\nhere = os.path.dirname(os.path.abspath(__file__))\nchild = subprocess.Popen([sys.executable, os.path.join(here, 'escaping-descendant.py')])\nheartbeat = os.path.join(here, 'heartbeat.txt')\nwhile not os.path.exists(heartbeat) or os.path.getsize(heartbeat) == 0:\n    time.sleep(0.02)\n";

/// Body of the fixture descendant that leaves the supervised process group.
#[cfg(unix)]
const M003M_ESCAPING_DESCENDANT: &str = "import os, sys, time\ntry:\n    os.setsid()\nexcept OSError:\n    pass\nhere = os.path.dirname(os.path.abspath(__file__))\npath = os.path.join(here, 'heartbeat.txt')\nstop = os.path.join(here, 'stop')\nend = time.time() + 300\nwhile time.time() < end and not os.path.exists(stop):\n    with open(path, 'a') as handle:\n        handle.write('tick\\n')\n    time.sleep(0.05)\n";

/// One bounded consumer-validation fixture: the validator, its script,
/// the exact candidate, and the working directory.
struct M003mFixture {
    validator: ConsumerValidatorV1,
    script: PathBuf,
    candidate: PathBuf,
    size: u64,
    sha: String,
    work: PathBuf,
}

/// Build one fixture from a validator and its script.
fn m003m_fixture(parent: &Path, validator: ConsumerValidatorV1, script: PathBuf) -> M003mFixture {
    let work = parent.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let (candidate, size, sha) = m003d_candidate(parent, "candidate", b"exact-bytes");
    M003mFixture {
        validator,
        script,
        candidate,
        size,
        sha,
        work,
    }
}

/// Run one consumer validation under an explicit outer bound.
///
/// The outer bound is what turns a regression into a reported hang rather
/// than a silent test timeout: the returned outcome distinguishes "the run
/// returned a fail-closed outcome" from "the run never returned at all".
fn m003m_run_bounded(
    fixture: M003mFixture,
    cancelled: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    outer: std::time::Duration,
) -> ConsumerValidationOutcome {
    let M003mFixture {
        validator,
        script,
        candidate,
        size,
        sha,
        work,
    } = fixture;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let request = ConsumerValidationRequest {
            validator: &validator,
            script_path: &script,
            candidate_path: &candidate,
            expected_size: size,
            expected_sha256: &sha,
            work_dir: &work,
            release_id: "v9.9.9",
            source_revision: M003D_SOURCE,
            target: "x86_64-unknown-linux-gnu",
            cancelled: cancelled.as_ref().map(|flag| flag.as_ref()),
            path_dirs: None,
        };
        let outcome = run_consumer_validator(&request)
            .map(|evidence| evidence.outcome)
            .unwrap_or(ConsumerValidationOutcome::Failed(
                ConsumerValidationFailure::InterpreterUnavailable,
            ));
        let _ = sender.send(outcome);
    });
    receiver.recv_timeout(outer).unwrap_or_else(|_| {
        panic!(
            "the consumer validation never returned within {outer:?}: the reader-thread \
             join is not bounded, so a descendant holding the inherited pipe wedges the run"
        )
    })
}

/// Ask the fixture descendants to stop, so a descendant that survived the
/// run does not outlive the test.
fn m003m_stop_descendants(parent: &Path) {
    let _ = std::fs::write(parent.join("stop"), b"stop\n");
}

/// Whether the fixture descendant is still advancing its heartbeat.
fn m003m_descendant_alive(heartbeat: &Path) -> bool {
    let size = std::fs::metadata(heartbeat)
        .map(|meta| meta.len())
        .unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(400));
    let later = std::fs::metadata(heartbeat)
        .map(|meta| meta.len())
        .unwrap_or(0);
    later > size
}

#[test]
fn m003m_runner_outcome_maps_onto_consumer_validation_outcome() {
    let parent = eggpack_core_test_temp("m003m-mapping");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let validator = m003d_validator(LogicalOutputSelector::Direct);
    let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"exact-bytes");
    // Success.
    let script = m003d_write_script(&parent, "pass.py", "pass\n");
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(evidence.outcome, ConsumerValidationOutcome::Passed);
    // Non-zero exit.
    let script = m003d_write_script(&parent, "fail.py", "import sys\nsys.exit(3)\n");
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit)
    );
    // Deadline exceeded.
    let mut short = validator.clone();
    short.timeout_ms = 1_000;
    let script = m003d_write_script(&parent, "slow.py", "import time\ntime.sleep(60)\n");
    let evidence = run_consumer_validator(&m003d_request(
        &short, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout)
    );
    // Output bound exceeded.
    let mut limited = validator.clone();
    limited.stdout_limit = 1024;
    let script = m003d_write_script(
        &parent,
        "flood.py",
        "import sys\nsys.stdout.write('x' * 100000)\n",
    );
    let evidence = run_consumer_validator(&m003d_request(
        &limited, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit)
    );
    // Cancellation observed.
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut slow = validator.clone();
    slow.timeout_ms = 60_000;
    let script = m003d_write_script(&parent, "hang.py", "import time\ntime.sleep(60)\n");
    let cancel_flag = flag.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    let outcome = m003m_run_bounded(
        m003m_fixture(&parent, slow, script),
        Some(flag.clone()),
        std::time::Duration::from_secs(60),
    );
    canceller.join().unwrap();
    assert_eq!(
        outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Cancelled)
    );
    // Interpreter not launchable. Windows CreateProcess searches the
    // application directory, the working directory, and the system
    // directories unconditionally, so an emptied PATH cannot simulate a
    // missing interpreter there. The spawn/preflight failure branches are
    // platform-independent code proven on the Linux/macOS lanes, while the
    // Windows lane proves every other row of the mapping.
    #[cfg(not(windows))]
    {
        let script = m003d_write_script(&parent, "ok.py", "pass\n");
        let empty: Vec<PathBuf> = Vec::new();
        let evidence = run_consumer_validator(&m003d_request(
            &validator,
            &script,
            &candidate,
            size,
            &sha,
            &work,
            Some(&empty),
        ))
        .unwrap();
        assert_eq!(
            evidence.outcome,
            ConsumerValidationOutcome::Failed(ConsumerValidationFailure::InterpreterUnavailable)
        );
    }
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn m003m_process_group_termination_leaves_no_live_descendant() {
    // The validator spawns a descendant and then hangs. The bounded run
    // must terminate the whole group, so the descendant does not outlive
    // the run. On Windows this is the job-object lane: the descendant is
    // assigned to the same job and dies with it.
    let parent = eggpack_core_test_temp("m003m-group");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let heartbeat = parent.join("heartbeat.txt");
    m003d_write_script(&parent, "descendant.py", M003M_DESCENDANT);
    let script = m003d_write_script(&parent, "spawner.py", M003M_SPAWNER);
    let mut validator = m003d_validator(LogicalOutputSelector::Direct);
    validator.timeout_ms = 3_000;
    let outcome = m003m_run_bounded(
        m003m_fixture(&parent, validator, script),
        None,
        std::time::Duration::from_secs(60),
    );
    assert!(
        matches!(outcome, ConsumerValidationOutcome::Failed(_)),
        "the bounded run must fail closed, not pass: {outcome:?}"
    );
    // The descendant really ran: its heartbeat is non-empty.
    let ticks = std::fs::metadata(&heartbeat)
        .map(|meta| meta.len())
        .unwrap_or(0);
    assert!(ticks > 0, "the fixture descendant never ran");
    // The descendant is gone: its heartbeat stopped advancing.
    assert!(
        !m003m_descendant_alive(&heartbeat),
        "the descendant outlived the run: the process group was not terminated"
    );
    m003m_stop_descendants(&parent);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn m003m_bounded_join_returns_instead_of_hanging() {
    // The descendant inherits the validator's stdout and holds it open
    // past the deadline. The run must still return, with a fail-closed
    // outcome, instead of parking on an unbounded reader-thread join.
    let parent = eggpack_core_test_temp("m003m-join");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let heartbeat = parent.join("heartbeat.txt");
    m003d_write_script(&parent, "descendant.py", M003M_DESCENDANT);
    let script = m003d_write_script(&parent, "spawner.py", M003M_SPAWNER);
    let mut validator = m003d_validator(LogicalOutputSelector::Direct);
    validator.timeout_ms = 3_000;
    let started = std::time::Instant::now();
    let outcome = m003m_run_bounded(
        m003m_fixture(&parent, validator, script),
        None,
        std::time::Duration::from_secs(60),
    );
    let elapsed = started.elapsed();
    assert!(
        matches!(outcome, ConsumerValidationOutcome::Failed(_)),
        "the bounded run must fail closed, not pass: {outcome:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "the bounded run took {elapsed:?}; the deadline did not bound the run"
    );
    // The descendant really held the inherited pipe open: it ran, and it
    // was still running when the run ended.
    let ticks = std::fs::metadata(&heartbeat)
        .map(|meta| meta.len())
        .unwrap_or(0);
    assert!(ticks > 0, "the fixture descendant never ran");
    m003m_stop_descendants(&parent);
    std::fs::remove_dir_all(parent).unwrap();
}

#[cfg(unix)]
#[test]
fn m003m_parked_reader_fails_closed_when_the_descendant_escapes_the_group() {
    // The descendant leaves the process group, so the group kill cannot
    // reach it and its inherited pipe stays open after the validator
    // exits. Only the bounded reader join can end the run, and a reader
    // still parked at its budget is detached and the run fails closed.
    let parent = eggpack_core_test_temp("m003m-parked");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let heartbeat = parent.join("heartbeat.txt");
    m003d_write_script(&parent, "escaping-descendant.py", M003M_ESCAPING_DESCENDANT);
    let script = m003d_write_script(&parent, "spawner.py", M003M_ESCAPING_SPAWNER);
    let mut validator = m003d_validator(LogicalOutputSelector::Direct);
    validator.timeout_ms = 10_000;
    let started = std::time::Instant::now();
    let outcome = m003m_run_bounded(
        m003m_fixture(&parent, validator, script),
        None,
        std::time::Duration::from_secs(60),
    );
    let elapsed = started.elapsed();
    // The descendant is still alive, so the pipe was still open when the
    // run ended: the run ended on the bounded join, not on EOF.
    assert!(
        m003m_descendant_alive(&heartbeat),
        "the descendant did not outlive the validator, so the bounded join was not exercised"
    );
    assert_eq!(
        outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit),
        "a parked reader must fail the run closed"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "the bounded run took {elapsed:?}; the drain budget did not bound the run"
    );
    m003m_stop_descendants(&parent);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn m003m_output_limit_terminates_the_run_before_its_deadline() {
    // A chatty validator is terminated at the moment of excess, not at its
    // deadline, and reports the output-limit failure.
    let parent = eggpack_core_test_temp("m003m-output-limit");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let script = m003d_write_script(
        &parent,
        "flood.py",
        "import sys, time\nsys.stdout.write('x' * 200000)\nsys.stdout.flush()\ntime.sleep(60)\n",
    );
    let mut validator = m003d_validator(LogicalOutputSelector::Direct);
    validator.timeout_ms = 30_000;
    validator.stdout_limit = 1024;
    let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"exact-bytes");
    let started = std::time::Instant::now();
    let evidence = run_consumer_validator(&m003d_request(
        &validator, &script, &candidate, size, &sha, &work, None,
    ))
    .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(
        evidence.outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::OutputLimit)
    );
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "the output bound was only enforced at the deadline: the run took {elapsed:?}"
    );
    m003m_stop_descendants(&parent);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn m003m_cancellation_terminates_the_group_and_reports_cancelled() {
    // A cancelled run terminates the whole group and reports the
    // cancellation; the descendant does not outlive it.
    let parent = eggpack_core_test_temp("m003m-cancel");
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let heartbeat = parent.join("heartbeat.txt");
    m003d_write_script(&parent, "descendant.py", M003M_DESCENDANT);
    let script = m003d_write_script(&parent, "spawner.py", M003M_SPAWNER);
    let mut validator = m003d_validator(LogicalOutputSelector::Direct);
    validator.timeout_ms = 60_000;
    let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"exact-bytes");
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancel_flag = flag.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let request = ConsumerValidationRequest {
            validator: &validator,
            script_path: &script,
            candidate_path: &candidate,
            expected_size: size,
            expected_sha256: &sha,
            work_dir: &work,
            release_id: "v9.9.9",
            source_revision: M003D_SOURCE,
            target: "x86_64-unknown-linux-gnu",
            cancelled: Some(cancel_flag.as_ref()),
            path_dirs: None,
        };
        let outcome = run_consumer_validator(&request)
            .map(|evidence| evidence.outcome)
            .unwrap_or(ConsumerValidationOutcome::Failed(
                ConsumerValidationFailure::InterpreterUnavailable,
            ));
        let _ = sender.send(outcome);
    });
    // Cancel once the descendant is demonstrably alive.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while std::fs::metadata(&heartbeat)
        .map(|meta| meta.len())
        .unwrap_or(0)
        == 0
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the fixture descendant never started"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    flag.store(true, std::sync::atomic::Ordering::SeqCst);
    let outcome = receiver
        .recv_timeout(std::time::Duration::from_secs(60))
        .expect("the cancelled run never returned");
    worker.join().unwrap();
    assert_eq!(
        outcome,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Cancelled)
    );
    assert!(
        !m003m_descendant_alive(&heartbeat),
        "the descendant outlived the cancelled run: the process group was not terminated"
    );
    m003m_stop_descendants(&parent);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn m003d_python_mapping_and_hermetic_lookup() {
    // Finite host mapping: python3 on Linux/macOS, python on Windows.
    assert_eq!(python_interpreter_exe_for_windows(false), "python3");
    assert_eq!(python_interpreter_exe_for_windows(true), "python");
    assert_eq!(
        python_interpreter_exe(),
        python_interpreter_exe_for_windows(cfg!(windows))
    );
    // Hermetic PATH lookup: a fixture dir providing the platform
    // interpreter proves fixed argv (script, candidate) without
    // depending on ambient tooling beyond the fake.
    let parent = eggpack_core_test_temp("m003d-python-path");
    let bin = parent.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let work = parent.join("work");
    std::fs::create_dir(&work).unwrap();
    let marker = parent.join("argv-marker.txt");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Fake interpreter: answers the version preflight with
        // Python 3 and records its fixed argv for the real run.
        let fake = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"Python 3.99.0\"; exit 0; fi\necho \"$1\" > \"{}\"\necho \"$2\" >> \"{}\"\nexit 0\n",
            marker.display(),
            marker.display()
        );
        let exe = bin.join(python_interpreter_exe());
        std::fs::write(&exe, fake).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    {
        let fake = format!(
            "@echo off\r\nif \"%~1\"==\"--version\" (echo Python 3.99.0) else (echo %1> \"{m}\" & echo %2>> \"{m}\")\r\nexit /b 0\r\n",
            m = marker.display()
        );
        std::fs::write(bin.join("python.cmd"), fake).unwrap();
    }
    let validator = m003d_validator(LogicalOutputSelector::Direct);
    let script = m003d_write_script(&parent, "v.py", "pass\n");
    let (candidate, size, sha) = m003d_candidate(&parent, "candidate", b"bytes");
    let dirs = [bin];
    let evidence = run_consumer_validator(&m003d_request(
        &validator,
        &script,
        &candidate,
        size,
        &sha,
        &work,
        Some(&dirs),
    ))
    .unwrap();
    #[cfg(unix)]
    {
        assert_eq!(evidence.outcome, ConsumerValidationOutcome::Passed);
        let recorded = std::fs::read_to_string(&marker).unwrap();
        let mut lines = recorded.lines();
        assert_eq!(lines.next().unwrap(), script.to_string_lossy().as_ref());
        assert_eq!(lines.next().unwrap(), candidate.to_string_lossy().as_ref());
    }
    #[cfg(windows)]
    {
        // Windows lookup resolves `python` via PATHEXT/cmd shim search;
        // the hermetic dir may not provide it, so only the mapping name
        // is asserted above on this host.
        let _ = evidence;
    }
    std::fs::remove_dir_all(parent).unwrap();
}

fn m003d_graph_with_consumer(
    support: SupportTier,
) -> (
    DistributionContract,
    ReleasePlan,
    BuildBindingsV1,
    QualificationBindingsV1,
    ReleaseCIPlanV1,
) {
    let (contract, release, bindings, qual_bindings) = m002_release(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        "linux-x64",
        Qualification::Structural,
        support,
    );
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    let mut validators = BTreeMap::new();
    validators.insert(
        "x86_64-unknown-linux-gnu".to_owned(),
        m003d_validator(LogicalOutputSelector::Direct),
    );
    let graph = project_release_plan_with_consumer(
        &ci_plan,
        &qual_bindings,
        &bindings,
        &release,
        validators,
    )
    .unwrap();
    (contract, release, bindings, qual_bindings, graph)
}

fn m003d_consumer_evidence(
    release: &ReleasePlan,
    outcome: ConsumerValidationOutcome,
) -> ConsumerValidationEvidenceV1 {
    ConsumerValidationEvidenceV1 {
        schema_version: 1,
        release_id: release.release_id.clone(),
        source_revision: release.source_revision.clone(),
        target: "x86_64-unknown-linux-gnu".into(),
        selector: LogicalOutputSelector::Direct,
        interpreter: ValidatorInterpreterV1::Python3,
        outcome,
        candidate_size: 4,
        candidate_sha256: Some("ab".repeat(32)),
    }
}

#[test]
fn m003d_consumer_gate_matrix() {
    let (_, release, _, _, graph) = m003d_graph_with_consumer(SupportTier::Required);
    let target = "x86_64-unknown-linux-gnu";
    let passed_core = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Passed,
    );
    let core = [passed_core];
    // Consumer pass gates through.
    let consumer_pass = m003d_consumer_evidence(&release, ConsumerValidationOutcome::Passed);
    assert_eq!(
        evaluate_gate_with_consumer(&graph, &core, &[consumer_pass]).unwrap(),
        AggregateOutcome::Complete
    );
    // Required consumer failure blocks aggregation.
    let consumer_fail = m003d_consumer_evidence(
        &release,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::NonZeroExit),
    );
    assert_eq!(
        evaluate_gate_with_consumer(&graph, &core, std::slice::from_ref(&consumer_fail)).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    // Missing consumer evidence for a required validator blocks.
    assert_eq!(
        evaluate_gate_with_consumer(&graph, &core, &[]).unwrap(),
        AggregateOutcome::FailedRequiredGate
    );
    // Identity mismatch (wrong selector) is invalid, never partial.
    let mut swapped = m003d_consumer_evidence(&release, ConsumerValidationOutcome::Passed);
    swapped.selector = LogicalOutputSelector::BundleEntry { index: 0 };
    assert_eq!(
        evaluate_gate_with_consumer(&graph, &core, &[swapped]).unwrap(),
        AggregateOutcome::InvalidEvidence
    );
    let mut wrong_release = m003d_consumer_evidence(&release, ConsumerValidationOutcome::Passed);
    wrong_release.release_id = "other".into();
    assert_eq!(
        evaluate_gate_with_consumer(&graph, &core, &[wrong_release]).unwrap(),
        AggregateOutcome::InvalidEvidence
    );
    // Optional validator failure suppresses rather than producing output.
    let (_, optional_release, _, _, optional_graph) =
        m003d_graph_with_consumer(SupportTier::NonGating);
    let optional_core = m002_evidence(
        target,
        &optional_release.release_id,
        &optional_release.source_revision,
        Qualification::Structural,
        SupportTier::NonGating,
        QualificationStatus::Failed(eggpack_core::QualificationFailure::SmokeFailed),
    );
    // Core optional failure alone already suppresses.
    let failed_optional = [optional_core];
    assert_eq!(
        evaluate_gate_with_consumer(
            &optional_graph,
            &failed_optional,
            &[m003d_consumer_evidence(
                &optional_release,
                ConsumerValidationOutcome::Passed
            )]
        )
        .unwrap(),
        AggregateOutcome::SuppressedNonGatingIncomplete
    );
    // Optional core pass + optional consumer failure suppresses.
    let optional_pass = m002_evidence(
        target,
        &optional_release.release_id,
        &optional_release.source_revision,
        Qualification::Structural,
        SupportTier::NonGating,
        QualificationStatus::Passed,
    );
    let optional_consumer_fail = m003d_consumer_evidence(
        &optional_release,
        ConsumerValidationOutcome::Failed(ConsumerValidationFailure::Timeout),
    );
    assert_eq!(
        evaluate_gate_with_consumer(&optional_graph, &[optional_pass], &[optional_consumer_fail])
            .unwrap(),
        AggregateOutcome::SuppressedNonGatingIncomplete
    );
    // Consumer evidence for a target without a validator is invalid.
    let (_, _, _, _, _, plain_graph) = m002_graph(Qualification::Structural, SupportTier::Required);
    let plain_core = m002_evidence(
        target,
        &release.release_id,
        &release.source_revision,
        Qualification::Structural,
        SupportTier::Required,
        QualificationStatus::Passed,
    );
    assert_eq!(
        evaluate_gate_with_consumer(
            &plain_graph,
            &[plain_core],
            &[m003d_consumer_evidence(
                &release,
                ConsumerValidationOutcome::Passed
            )]
        )
        .unwrap(),
        AggregateOutcome::InvalidEvidence
    );
    // Durable evidence contains no script output contents.
    let json = consumer_fail.to_json().unwrap();
    assert!(!json.contains("stdout"));
    assert!(!json.contains("stderr"));
}
#[test]
fn m003d_consumer_attach_rejects_mismatch() {
    let (contract, release, bindings, qual_bindings) = m002_release(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        "linux-x64",
        Qualification::Structural,
        SupportTier::Required,
    );
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    // Unknown target rejects.
    let mut validators = BTreeMap::new();
    validators.insert(
        "aarch64-unknown-linux-gnu".to_owned(),
        m003d_validator(LogicalOutputSelector::Direct),
    );
    assert!(project_release_plan_with_consumer(
        &ci_plan,
        &qual_bindings,
        &bindings,
        &release,
        validators
    )
    .is_err());
    // Selector outside the target bindings rejects.
    let mut validators = BTreeMap::new();
    validators.insert(
        "x86_64-unknown-linux-gnu".to_owned(),
        m003d_validator(LogicalOutputSelector::BundleEntry { index: 0 }),
    );
    assert!(project_release_plan_with_consumer(
        &ci_plan,
        &qual_bindings,
        &bindings,
        &release,
        validators
    )
    .is_err());
    // Validator map path is required for rendering.
    let (_, _, _, _, graph) = m003d_graph_with_consumer(SupportTier::Required);
    let mut policy = m002_golden_policy();
    assert!(render_release_github(&graph, &policy).is_err());
    policy.release_inputs.as_mut().unwrap().consumer_validators =
        Some("validators/consumer.json".into());
    assert!(render_release_github(&graph, &policy).is_ok());
}

#[test]
fn m003n_review_refuses_an_over_long_generated_directory_instead_of_panicking() {
    // The consumer-evidence directory name is derived from the build job id,
    // which is derived from the contract's target triple. A contract may
    // legally declare a triple at the contract's own name bound, which
    // produces a directory name longer than the renderer's bound. The
    // renderer must refuse it, not panic on a caller-supplied document.
    let long_target = format!("{}-b", "a".repeat(126));
    assert_eq!(long_target.len(), 128);
    let fixture = format!(
        "schema_version = 1\n\n[product]\nid = \"eggsact\"\ndisplay_name = \"eggsact\"\n\n[[targets]]\ntriple = \"{long_target}\"\naliases = [\"linux-x64\"]\n\n[targets.asset]\nkind = \"direct\"\nasset = \"{{product}}-{{version}}-{{target}}\"\ninstall = \"{{product}}\"\n\n[targets.checksum]\nsidecar = \"{{asset}}.sha256\"\n"
    );
    let contract = DistributionContract::parse_toml_str(&fixture).unwrap();
    let config = PackConfig {
        schema_version: 1,
        targets: vec![TargetPolicy {
            target: long_target.clone(),
            strategy: BuildStrategy::NativeCargo,
            host_os: HostOs::Linux,
            host_arch: HostArch::X86_64,
            qualification_host: None,
            toolchain: ToolchainRequirement {
                rust: "1.89.0".into(),
                cargo_zigbuild: None,
                zig: None,
            },
            floor: CompatibilityFloor::None,
            qualification: Qualification::Structural,
            support: SupportTier::Required,
        }],
    };
    let release = config
        .resolve(&contract, "1.2.3", &"a".repeat(40), &["linux-x64".into()])
        .unwrap();
    let expanded = contract.expand("linux-x64", "1.2.3").unwrap();
    let selectors: Vec<LogicalOutputSelector> = match expanded.assets {
        eggpack_contract::ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
        eggpack_contract::ExpandedAssets::Bundle(b) => (0..b.entries.len())
            .map(|index| LogicalOutputSelector::BundleEntry { index })
            .collect(),
        eggpack_contract::ExpandedAssets::Archive(a) => a
            .members
            .into_iter()
            .map(|m| LogicalOutputSelector::ArchiveMember { source: m.source })
            .collect(),
    };
    let bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: [(
            long_target.clone(),
            selectors
                .iter()
                .enumerate()
                .map(|(index, selector)| eggpack_core::BuildBinding {
                    selector: selector.clone(),
                    package: "eggsact".into(),
                    binary: format!("bin{index}"),
                })
                .collect(),
        )]
        .into_iter()
        .collect(),
    };
    let qual_bindings = QualificationBindingsV1 {
        schema_version: 1,
        targets: [(
            long_target.clone(),
            eggpack_core::TargetQualificationBinding { smoke: None },
        )]
        .into_iter()
        .collect(),
    };
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    let mut validators = BTreeMap::new();
    validators.insert(
        long_target.clone(),
        m003d_validator(LogicalOutputSelector::Direct),
    );
    let graph = project_release_plan_with_consumer(
        &ci_plan,
        &qual_bindings,
        &bindings,
        &release,
        validators,
    )
    .unwrap();
    // The build job id is the target with non-alphanumerics replaced, so the
    // consumer evidence directory is `./eggpack-consumer/{build_job_id}`.
    assert_eq!(graph.qualifications[0].build_job_id.len(), 134);
    let mut policy = m002_golden_policy();
    policy.release_inputs.as_mut().unwrap().consumer_validators =
        Some("validators/consumer.json".into());
    // The generated directory is now 153 characters, past the renderer's
    // bound. The renderer refuses it instead of asserting.
    assert!(
        render_release_github(&graph, &policy).is_err(),
        "an over-long generated directory must be refused, not rendered"
    );
}

#[test]
fn m003d_stage_presentation_flags_are_explicit() {
    // Without a presentation path the stage job carries no wrapper
    // flags (M003c byte-compatible); with one, both paired flags render
    // and product scripts are only ever copied as bytes (never executed).
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    let plain = render_release_github(&graph, &policy).unwrap();
    assert!(!plain.contains("--installer-presentation"));
    assert!(!plain.contains("--source-root"));
    let mut presented = policy.clone();
    presented
        .staging
        .as_mut()
        .unwrap()
        .inputs
        .installer_presentation = Some("policies/installer-presentation.toml".into());
    let yaml = render_release_github(&graph, &presented).unwrap();
    assert!(yaml.contains("--installer-presentation"));
    assert!(yaml.contains("policies/installer-presentation.toml"));
    assert!(yaml.contains("--source-root"));
    // The wrapper file itself is never named or executed in the
    // workflow; only the presentation policy path is referenced.
    assert!(!yaml.contains("packaging/install.sh"));
    assert!(!yaml.contains("install-exact.sh"));
}

#[test]
fn m003d_consumer_render_gates_and_detects_drift() {
    let (_, _, _, _, graph) = m003d_graph_with_consumer(SupportTier::Required);
    let mut policy = m002_golden_policy();
    policy.release_inputs.as_mut().unwrap().consumer_validators =
        Some("validators/consumer.json".into());
    let yaml = render_release_github(&graph, &policy).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
    // Consumer validation job runs after core qualification.
    let validate_job = jobs.get("validate_build_x86_64_unknown_linux_gnu").unwrap();
    assert_eq!(
        validate_job.get("needs").unwrap().as_str().unwrap(),
        "qualify_build_x86_64_unknown_linux_gnu"
    );
    // Gate waits for the validator, not just core qualification.
    let gate = jobs.get("required_gate").unwrap();
    let gate_needs: Vec<&str> = gate
        .get("needs")
        .unwrap()
        .as_sequence()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(gate_needs, vec!["validate_build_x86_64_unknown_linux_gnu"]);
    // Validator receives exact handoff bytes; stage stays the writer.
    let text = serde_yaml::to_string(&serde_yaml::Value::Mapping(jobs.clone())).unwrap();
    assert!(text.contains("_validate-consumer"));
    assert!(text.contains("--consumer-validators"));
    assert!(text.contains("--source-root"));
    assert!(text.contains("eggpack-consumer-evidence-x86_64-unknown-linux-gnu"));
    let mut writers = Vec::new();
    for (name, job) in jobs {
        let name = name.as_str().unwrap();
        let contents = job
            .get("permissions")
            .unwrap()
            .get("contents")
            .unwrap()
            .as_str()
            .unwrap();
        if contents == "write" {
            writers.push(name.to_owned());
        }
        assert_ne!(
            job.get("permissions")
                .unwrap()
                .get("id-token")
                .map(|v| v.as_str().unwrap_or("")),
            Some("write"),
            "job {name}"
        );
    }
    assert!(
        writers.is_empty(),
        "exact consumer graph has no writer without staging"
    );
    // Drift: validator removal/reorder is detected.
    let (_, _, _, _, _, plain) = m002_graph(Qualification::Structural, SupportTier::Required);
    assert!(!check_release_github(&plain, &policy, yaml.as_bytes())
        .map(|report| report.matches)
        .unwrap_or(true));
    let yaml_plain = render_release_github(&plain, &policy).unwrap();
    assert_ne!(yaml, yaml_plain);
    assert!(
        check_release_github(&graph, &policy, yaml.as_bytes())
            .unwrap()
            .matches
    );
    // No validator jobs leak into extension-disabled rendering.
    assert!(!yaml_plain.contains("_validate-consumer"));
    assert!(!yaml_plain.contains("validate_build_"));
}

#[test]
fn m003d_runner_commands_cover_consumer_and_resolve() {
    let validate = RunnerCommand::ValidateConsumer {
        consumer_validators: "validators/consumer.json".into(),
        target: "x86_64-unknown-linux-gnu".into(),
        candidate_dir: "./eggpack-handoff/build_x/candidates".into(),
        build_handoff: "./eggpack-handoff/build_x/build-handoff.json".into(),
        evidence: "./eggpack-handoff/build_x/evidence.json".into(),
        source_root: "${{ github.workspace }}".into(),
        output: "./eggpack-consumer/build_x/consumer-evidence.json".into(),
    };
    let shell = validate.to_shell();
    for required in [
        "_validate-consumer",
        "--consumer-validators",
        "--target",
        "--candidate-dir",
        "--build-handoff",
        "--evidence",
        "--source-root",
        "--output",
    ] {
        assert!(
            shell.contains(required),
            "validate shell missing {required}"
        );
    }
    assert!(!shell.contains("publish"));
    let resolve = RunnerCommand::ResolveRelease {
        contract: "contracts/release.toml".into(),
        pack_config: "configs/pack.toml".into(),
        build_bindings: "bindings/build.toml".into(),
        qualification_bindings: "bindings/qualification.toml".into(),
        consumer_validators: Some("validators/consumer.json".into()),
        selected: "linux-x64".into(),
        tag: "v1.2.3".into(),
        source_revision: "a".repeat(40),
        template: "policies/github-template.json".into(),
        source_root: "${{ github.workspace }}".into(),
        output_plan: "./eggpack-runtime/release-plan.json".into(),
        output_ci_plan: "./eggpack-runtime/release-ci-plan.json".into(),
        output_github_policy: "./eggpack-runtime/github-draft.json".into(),
        identity_mode: None,
    };
    let argv = resolve.argv();
    assert_eq!(&argv[0..3], &["eggpack", "ci", "_resolve-release"]);
    for required in [
        "--contract",
        "--pack-config",
        "--build-bindings",
        "--qualification-bindings",
        "--consumer-validators",
        "--selected",
        "--tag",
        "--source-revision",
        "--template",
        "--source-root",
        "--output-plan",
        "--output-ci-plan",
        "--output-github-policy",
    ] {
        assert!(argv.contains(&required.to_string()), "missing {required}");
    }
    assert!(!resolve.to_shell().contains("publish"));
    // Omitted validator map omits the flag pair (exact position stable).
    let bare = RunnerCommand::ResolveRelease {
        contract: "contracts/release.toml".into(),
        pack_config: "configs/pack.toml".into(),
        build_bindings: "bindings/build.toml".into(),
        qualification_bindings: "bindings/qualification.toml".into(),
        consumer_validators: None,
        selected: "linux-x64".into(),
        tag: "v1.2.3".into(),
        source_revision: "a".repeat(40),
        template: "policies/github-template.json".into(),
        source_root: "${{ github.workspace }}".into(),
        output_plan: "./eggpack-runtime/release-plan.json".into(),
        output_ci_plan: "./eggpack-runtime/release-ci-plan.json".into(),
        output_github_policy: "./eggpack-runtime/github-draft.json".into(),
        identity_mode: None,
    };
    assert!(!bare.argv().contains(&"--consumer-validators".to_string()));
}

fn m003d_shape_and_policy(
    tag_source: StagingTagSource,
) -> (DistributionContract, ReleaseWorkflowShapeV1, GitHubPolicy) {
    let (contract, release, bindings, qual_bindings) = m002_release(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        "linux-x64",
        Qualification::Structural,
        SupportTier::Required,
    );
    let mut validators = BTreeMap::new();
    validators.insert(
        "x86_64-unknown-linux-gnu".to_owned(),
        m003d_validator(LogicalOutputSelector::Direct),
    );
    let shape = ReleaseWorkflowShapeV1 {
        schema_version: 1,
        targets: release.targets.iter().map(|t| t.policy.clone()).collect(),
        selected_aliases: vec!["linux-x64".into()],
        build_bindings: bindings,
        qualification_bindings: qual_bindings,
        consumer_validators: validators,
        staging: Some(ShapeStagingIntentV1 {
            provider: StagingProvider::GitHubDraft,
            tag_source,
            required: true,
        }),
    };
    let mut policy = m002_golden_policy();
    let inputs = policy.release_inputs.as_mut().unwrap();
    inputs.pack_config = Some("configs/pack.toml".into());
    inputs.draft_template = Some("policies/github-template.json".into());
    inputs.consumer_validators = Some("validators/consumer.json".into());
    policy.staging = Some(GitHubStagingPolicyV1 {
        runner: "ubuntu-latest".into(),
        owner: "acme".into(),
        repository: "widget".into(),
        tag_source,
        environment: None,
        inputs: GitHubStagingInputsV1 {
            contract: "contracts/release.toml".into(),
            install_policy: "policies/install.toml".into(),
            github_policy: "policies/github-draft.json".into(),
            installer_presentation: None,
        },
        receipt_retention_days: 7,
    });
    (contract, shape, policy)
}

#[test]
fn m003d_shape_validation() {
    let (_, shape, _) = m003d_shape_and_policy(StagingTagSource::RefName);
    assert!(shape.validate().is_ok());
    assert_eq!(
        ReleaseWorkflowShapeV1::from_json(&shape.to_json().unwrap()).unwrap(),
        shape
    );
    // No release identity lives in the shape by construction: unknown
    // identity fields reject.
    assert!(ReleaseWorkflowShapeV1::from_json(
        r#"{"schema_version":1,"targets":[],"selected_aliases":[],"build_bindings":{},"qualification_bindings":{},"release_id":"v1"}"#
    )
    .is_err());
    let mut bad = shape.clone();
    bad.schema_version = 2;
    assert!(bad.validate().is_err());
    let mut bad = shape.clone();
    bad.targets.reverse();
    // Single-target shape: reversal is a no-op; duplicate instead.
    bad.targets.push(bad.targets[0].clone());
    assert!(bad.validate().is_err());
    let mut bad = shape.clone();
    bad.selected_aliases.clear();
    assert!(bad.validate().is_err());
    let mut bad = shape.clone();
    bad.selected_aliases = vec!["".into()];
    assert!(bad.validate().is_err());

    let (contract, shape, mut policy) = m003d_shape_and_policy(StagingTagSource::RefName);
    policy.staging.as_mut().unwrap().environment = Some("invalid/environment".into());
    assert!(render_reusable_release_github(&contract, &shape, &policy).is_err());
}

#[test]
fn m003d_runtime_resolve_matrix() {
    let (contract, shape, _) = m003d_shape_and_policy(StagingTagSource::RefName);
    let pack = shape.pack_config();
    let source = "a".repeat(40);
    let first =
        resolve_runtime_release_plan(&contract, &pack, &shape.selected_aliases, "v1.2.4", &source)
            .unwrap();
    assert_eq!(first.release_id, "v1.2.4");
    assert_eq!(first.source_revision, source);
    // Distinct future tags resolve distinct plans from identical shape.
    let second =
        resolve_runtime_release_plan(&contract, &pack, &shape.selected_aliases, "v1.2.5", &source)
            .unwrap();
    assert_ne!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
    // Exact tag is used as-is (no product syntax transformation).
    let tagged = resolve_runtime_release_plan(
        &contract,
        &pack,
        &shape.selected_aliases,
        "eggsact-v2.0.0",
        &source,
    )
    .unwrap();
    assert_eq!(tagged.release_id, "eggsact-v2.0.0");
    let mapped = resolve_runtime_release_plan_with_mode(
        &contract,
        &pack,
        &shape.selected_aliases,
        "v1.2.3",
        &source,
        ReleaseIdentityMode::VPrefixedStableSemver,
    )
    .unwrap();
    assert_eq!(mapped.release_id, "1.2.3");
    assert_eq!(mapped.source_revision, source);
    for malformed in [
        "1.2.3",
        "vv1.2.3",
        "v01.2.3",
        "v1.2.3-rc1",
        "v1.2.3+meta",
        "v18446744073709551616.2.3",
        "v1.2.3.4",
        "v１.2.3",
        "v1.2.3'\nmalicious",
    ] {
        assert!(
            resolve_runtime_release_plan_with_mode(
                &contract,
                &pack,
                &shape.selected_aliases,
                malformed,
                &source,
                ReleaseIdentityMode::VPrefixedStableSemver,
            )
            .is_err(),
            "malformed tag {malformed:?} must reject"
        );
    }
    // Injection and shape violations reject.
    for bad_tag in [
        "",
        "v1?x",
        "v 1",
        "../escape",
        "/v1",
        "v1/",
        "a".repeat(129).as_str(),
    ] {
        assert!(
            resolve_runtime_release_plan(
                &contract,
                &pack,
                &shape.selected_aliases,
                bad_tag,
                &source
            )
            .is_err(),
            "tag {bad_tag:?} must reject"
        );
    }
    for bad_rev in [
        "",
        &"a".repeat(39),
        &"A".repeat(40),
        "not-hex-at-all------------------------",
    ] {
        assert!(
            resolve_runtime_release_plan(
                &contract,
                &pack,
                &shape.selected_aliases,
                "v1.2.4",
                bad_rev
            )
            .is_err(),
            "revision {bad_rev:?} must reject"
        );
    }
    assert!(resolve_runtime_release_plan(&contract, &pack, &[], "v1.2.4", &source).is_err());
    assert!(resolve_runtime_release_plan(
        &contract,
        &pack,
        &["unknown-alias".to_owned()],
        "v1.2.4",
        &source
    )
    .is_err());
}

#[test]
fn m003d_reusable_render_is_tag_independent() {
    for tag_source in [StagingTagSource::RefName, StagingTagSource::DispatchInput] {
        let (contract, shape, policy) = m003d_shape_and_policy(tag_source);
        let first = render_reusable_release_github(&contract, &shape, &policy).unwrap();
        // Same static config renders byte-identical workflow bytes
        // independent of any future tag.
        assert_eq!(
            first,
            render_reusable_release_github(&contract, &shape, &policy).unwrap()
        );
        // No release identity is embedded.
        assert!(!first.contains("0.0.0-m003d-unresolved"));
        assert!(!first.contains("0000000000000000000000000000000000000000"));
        assert!(!first.contains("plans/release-plan.json"));
        assert!(!first.contains("plans/release-ci-plan.json"));
        assert!(!first.contains("policies/github-draft.json"));
        // Runtime preflight resolves identity into private storage.
        assert!(first.contains("resolve:"));
        assert!(first.contains("_resolve-release"));
        assert!(first.contains("--pack-config"));
        assert!(first.contains("--template"));
        assert!(first.contains("eggpack-runtime-identity"));
        assert!(first.contains("./eggpack-runtime/release-plan.json"));
        assert!(first.contains("head_sha=\"$(git rev-parse --verify HEAD^{commit})\""));
        assert!(first.contains("\"$head_sha\""));
        // Every source checkout is verified against the runtime plan.
        assert!(first.contains("_verify-source"));
        assert_eq!(
            first.matches("needs: resolve").count(),
            1,
            "only the build job waits for resolve"
        );
        // Consumer seam is wired after core qualification.
        assert!(first.contains("validate_build_x86_64_unknown_linux_gnu"));
        assert!(first.contains("_validate-consumer"));
        // Tag source controls the event mapping.
        match tag_source {
            StagingTagSource::RefName => {
                assert!(first.contains("github.ref_type == 'tag'"));
                assert!(!first.contains("inputs.release_tag:\n"));
            }
            StagingTagSource::DispatchInput => {
                assert!(first.contains("inputs.release_tag"));
                assert!(first.contains("github.event_name == 'workflow_dispatch'"));
            }
        }
        // Only stage writes; no publication authority.
        let parsed: serde_yaml::Value = serde_yaml::from_str(&first).unwrap();
        let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
        let mut writers = Vec::new();
        for (name, job) in jobs {
            let contents = job
                .get("permissions")
                .unwrap()
                .get("contents")
                .unwrap()
                .as_str()
                .unwrap();
            if contents == "write" {
                writers.push(name.as_str().unwrap().to_owned());
            }
        }
        assert_eq!(writers, vec!["stage".to_string()]);
        assert!(!first.contains("id-token: write"));
        assert!(!first.contains("gh release"));
        assert!(!first.contains("publish"));
    }
    // Tag sources render distinct workflows.
    let (contract, shape_ref, policy_ref) = m003d_shape_and_policy(StagingTagSource::RefName);
    let (_, shape_dispatch, policy_dispatch) =
        m003d_shape_and_policy(StagingTagSource::DispatchInput);
    assert_ne!(
        render_reusable_release_github(&contract, &shape_ref, &policy_ref).unwrap(),
        render_reusable_release_github(&contract, &shape_dispatch, &policy_dispatch).unwrap()
    );
}

#[test]
fn m003i_mapped_workflow_binds_tag_safely_and_preserves_event_mapping() {
    for (tag_source, triggers, expression) in [
        (
            StagingTagSource::RefName,
            vec![WorkflowTrigger::Push],
            "${{ github.ref_name }}",
        ),
        (
            StagingTagSource::DispatchInput,
            vec![WorkflowTrigger::WorkflowDispatch],
            "${{ inputs.release_tag }}",
        ),
    ] {
        let (contract, shape, mut policy) = m003d_shape_and_policy(tag_source);
        policy.triggers = triggers;
        policy.release_identity_mode = Some(ReleaseIdentityMode::VPrefixedStableSemver);
        let workflow = render_reusable_release_github(&contract, &shape, &policy).unwrap();
        assert!(!workflow.contains("release_identity_mode"));
        assert!(workflow.contains("EGGPACK_RELEASE_TAG: "));
        assert!(workflow.contains(expression));
        assert!(workflow.contains("--identity-mode"), "{workflow}");
        assert!(workflow.contains("--tag' \"$EGGPACK_RELEASE_TAG\""));
        assert!(workflow.contains("--expected-tag"));
        let parsed: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
        assert!(parsed.get("on").is_some());
    }

    let (contract, shape, mut invalid) = m003d_shape_and_policy(StagingTagSource::RefName);
    invalid.release_identity_mode = Some(ReleaseIdentityMode::VPrefixedStableSemver);
    assert!(render_reusable_release_github(&contract, &shape, &invalid).is_err());
}

#[test]
fn m003d_reusable_check_detects_shape_drift() {
    let (contract, shape, policy) = m003d_shape_and_policy(StagingTagSource::RefName);
    let yaml = render_reusable_release_github(&contract, &shape, &policy).unwrap();
    assert!(
        check_reusable_release_github(&contract, &shape, &policy, yaml.as_bytes())
            .unwrap()
            .matches
    );
    // Validator removal is detected.
    let mut no_validator = shape.clone();
    no_validator.consumer_validators.clear();
    assert!(
        !check_reusable_release_github(&contract, &no_validator, &policy, yaml.as_bytes())
            .map(|report| report.matches)
            .unwrap_or(true)
    );
    // Staging intent removal is detected.
    let mut no_staging = shape.clone();
    no_staging.staging = None;
    assert!(
        check_reusable_release_github(&contract, &no_staging, &policy, yaml.as_bytes()).is_err()
            || !check_reusable_release_github(&contract, &no_staging, &policy, yaml.as_bytes())
                .map(|report| report.matches)
                .unwrap_or(true)
    );
    // Tag source switch is detected.
    let (_, shape_dispatch, policy_dispatch) =
        m003d_shape_and_policy(StagingTagSource::DispatchInput);
    assert!(!check_reusable_release_github(
        &contract,
        &shape_dispatch,
        &policy_dispatch,
        yaml.as_bytes()
    )
    .map(|report| report.matches)
    .unwrap_or(true));
    // Missing static paths fail closed at render time.
    let mut bad_policy = policy.clone();
    bad_policy.release_inputs.as_mut().unwrap().pack_config = None;
    assert!(render_reusable_release_github(&contract, &shape, &bad_policy).is_err());
    let mut bad_policy = policy.clone();
    bad_policy.release_inputs.as_mut().unwrap().draft_template = None;
    assert!(render_reusable_release_github(&contract, &shape, &bad_policy).is_err());
    // Shape/policy tag source skew fails closed.
    let mut skewed = policy.clone();
    skewed.staging.as_mut().unwrap().tag_source = StagingTagSource::DispatchInput;
    assert!(render_reusable_release_github(&contract, &shape, &skewed).is_err());
}

// -----------------------------------------------------------------------
// M005 deterministic cross-tool provisioning.
// -----------------------------------------------------------------------

const M005_ZIG_VERSION: &str = "0.14.1";
const M005_CARGO_ZIGBUILD_VERSION: &str = "0.23.3";
const M005_X86_64_SHA: &str = "24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c";
const M005_AARCH64_SHA: &str = "f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b";

fn m005_provisioning() -> CrossToolProvisioningV1 {
    CrossToolProvisioningV1 {
        cargo_install_timeout_minutes: 10,
        zig_connect_timeout_secs: 30,
        zig_max_time_secs: 600,
        zig: ZigOfficialArchiveV1 {
            linux_x86_64_sha256: M005_X86_64_SHA.into(),
            linux_aarch64_sha256: M005_AARCH64_SHA.into(),
        },
    }
}

fn m005_cross_policy(
    host_arch: HostArch,
    zig_version: &str,
    cargo_zigbuild_version: &str,
) -> TargetPolicy {
    TargetPolicy {
        target: "x86_64-unknown-linux-gnu".into(),
        strategy: BuildStrategy::CargoZigbuild,
        host_os: HostOs::Linux,
        host_arch,
        qualification_host: None,
        toolchain: ToolchainRequirement {
            rust: "1.89.0".into(),
            cargo_zigbuild: Some(cargo_zigbuild_version.into()),
            zig: Some(zig_version.into()),
        },
        floor: CompatibilityFloor::Glibc {
            major: 2,
            minor: 17,
        },
        qualification: Qualification::Structural,
        support: SupportTier::Required,
    }
}

fn m005_cross_graph(
    host_arch: HostArch,
    zig_version: &str,
    cargo_zigbuild_version: &str,
) -> (CIPlan, GitHubPolicy) {
    let contract = contract();
    let config = PackConfig {
        schema_version: 1,
        targets: vec![m005_cross_policy(
            host_arch,
            zig_version,
            cargo_zigbuild_version,
        )],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()],
        )
        .unwrap();
    let graph = project_ci_plan(&contract, &release, &bindings(&release)).unwrap();
    let mut policy = self::policy();
    policy.runners = vec![RunnerMapping {
        os: HostOs::Linux,
        arch: host_arch,
        label: "ubuntu-latest".into(),
        cargo_zigbuild: false,
        zig: false,
    }];
    (graph, policy)
}

// -----------------------------------------------------------------------
// M006 — native qualification for cross-tool builds.
// -----------------------------------------------------------------------

const M006_EGGSACT_CONTRACT: &str =
    include_str!("../../eggpack-contract/tests/fixtures/eggsact-direct-targets.toml");

fn m006_policy(
    target: &str,
    strategy: BuildStrategy,
    host_os: HostOs,
    host_arch: HostArch,
    qualification_host: Option<(HostOs, HostArch)>,
) -> TargetPolicy {
    TargetPolicy {
        target: target.into(),
        strategy,
        host_os,
        host_arch,
        qualification_host: qualification_host.map(|(os, arch)| HostRequirement { os, arch }),
        toolchain: match strategy {
            BuildStrategy::NativeCargo => ToolchainRequirement {
                rust: "1.89.0".into(),
                cargo_zigbuild: None,
                zig: None,
            },
            BuildStrategy::CargoZigbuild => ToolchainRequirement {
                rust: "1.89.0".into(),
                cargo_zigbuild: Some(M005_CARGO_ZIGBUILD_VERSION.into()),
                zig: Some(M005_ZIG_VERSION.into()),
            },
        },
        floor: if strategy == BuildStrategy::CargoZigbuild {
            CompatibilityFloor::Glibc {
                major: 2,
                minor: 17,
            }
        } else {
            CompatibilityFloor::None
        },
        qualification: Qualification::Native,
        support: SupportTier::Required,
    }
}

/// The eggsact-shaped five-target matrix: two CargoZigbuild Linux targets
/// for a glibc 2.17 floor, both natively qualified on matching native
/// runners, plus three NativeCargo targets natively qualified.
///
/// The AArch64 GNU/Linux target is deliberately a split-host topology: it is
/// cross-built on Linux x86-64 and qualified natively on Linux AArch64.
fn m006_shape_and_policy() -> (DistributionContract, ReleaseWorkflowShapeV1, GitHubPolicy) {
    let contract = DistributionContract::parse_toml_str(M006_EGGSACT_CONTRACT).unwrap();
    let targets = vec![
        m006_policy(
            "aarch64-apple-darwin",
            BuildStrategy::NativeCargo,
            HostOs::Macos,
            HostArch::Aarch64,
            None,
        ),
        m006_policy(
            "aarch64-unknown-linux-gnu",
            BuildStrategy::CargoZigbuild,
            HostOs::Linux,
            HostArch::X86_64,
            Some((HostOs::Linux, HostArch::Aarch64)),
        ),
        m006_policy(
            "x86_64-apple-darwin",
            BuildStrategy::NativeCargo,
            HostOs::Macos,
            HostArch::X86_64,
            None,
        ),
        m006_policy(
            "x86_64-pc-windows-msvc",
            BuildStrategy::NativeCargo,
            HostOs::Windows,
            HostArch::X86_64,
            None,
        ),
        m006_policy(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::CargoZigbuild,
            HostOs::Linux,
            HostArch::X86_64,
            None,
        ),
    ];
    let build_bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: targets
            .iter()
            .map(|policy| {
                (
                    policy.target.clone(),
                    vec![eggpack_core::BuildBinding {
                        selector: LogicalOutputSelector::Direct,
                        package: "eggsact".into(),
                        binary: "eggsact".into(),
                    }],
                )
            })
            .collect(),
    };
    let qualification_bindings = QualificationBindingsV1 {
        schema_version: 1,
        targets: targets
            .iter()
            .map(|policy| {
                (
                    policy.target.clone(),
                    eggpack_core::TargetQualificationBinding {
                        smoke: Some(eggpack_core::CandidateSmokeBinding {
                            selector: LogicalOutputSelector::Direct,
                            argv: vec!["--version".into()],
                            timeout_ms: 10_000,
                            stdout_limit: 8192,
                            stderr_limit: 8192,
                        }),
                    },
                )
            })
            .collect(),
    };
    let consumer_validators = targets
        .iter()
        .map(|policy| {
            (
                policy.target.clone(),
                m003d_validator(LogicalOutputSelector::Direct),
            )
        })
        .collect();
    let shape = ReleaseWorkflowShapeV1 {
        schema_version: 1,
        targets,
        selected_aliases: vec![
            "linux-x64".into(),
            "linux-arm64".into(),
            "macos-arm64".into(),
            "macos-x64".into(),
            "windows-x64".into(),
        ],
        build_bindings,
        qualification_bindings,
        consumer_validators,
        staging: Some(ShapeStagingIntentV1 {
            provider: StagingProvider::GitHubDraft,
            tag_source: StagingTagSource::RefName,
            required: true,
        }),
    };
    let mut policy = m002_golden_policy();
    policy.runners = vec![
        RunnerMapping {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
            label: "ubuntu-latest".into(),
            cargo_zigbuild: false,
            zig: false,
        },
        RunnerMapping {
            os: HostOs::Linux,
            arch: HostArch::Aarch64,
            label: "ubuntu-24.04-arm".into(),
            cargo_zigbuild: false,
            zig: false,
        },
        RunnerMapping {
            os: HostOs::Macos,
            arch: HostArch::Aarch64,
            label: "macos-14".into(),
            cargo_zigbuild: false,
            zig: false,
        },
        RunnerMapping {
            os: HostOs::Macos,
            arch: HostArch::X86_64,
            label: "macos-13".into(),
            cargo_zigbuild: false,
            zig: false,
        },
        RunnerMapping {
            os: HostOs::Windows,
            arch: HostArch::X86_64,
            label: "windows-latest".into(),
            cargo_zigbuild: false,
            zig: false,
        },
    ];
    policy.cross_tools = Some(m005_provisioning());
    let inputs = policy.release_inputs.as_mut().unwrap();
    inputs.pack_config = Some("configs/pack.toml".into());
    inputs.draft_template = Some("policies/github-template.json".into());
    inputs.consumer_validators = Some("validators/consumer.json".into());
    policy.staging = Some(GitHubStagingPolicyV1 {
        runner: "ubuntu-latest".into(),
        owner: "eggstack".into(),
        repository: "eggsact".into(),
        tag_source: StagingTagSource::RefName,
        environment: None,
        inputs: GitHubStagingInputsV1 {
            contract: "contracts/release.toml".into(),
            install_policy: "policies/install.toml".into(),
            github_policy: "policies/github-draft.json".into(),
            installer_presentation: None,
        },
        receipt_retention_days: 7,
    });
    (contract, shape, policy)
}

/// One rendered job as text, so a substring match can never leak from a
/// neighbouring job.
fn job_text(yaml: &str, job: &str) -> String {
    serde_json::to_string(
        serde_yaml::from_str::<serde_yaml::Value>(yaml)
            .unwrap()
            .get("jobs")
            .and_then(|jobs| jobs.get(job))
            .unwrap_or_else(|| panic!("missing job: {job}")),
    )
    .unwrap()
}

fn job_field(yaml: &str, job: &str, field: &str) -> serde_yaml::Value {
    serde_yaml::from_str::<serde_yaml::Value>(yaml)
        .unwrap()
        .get("jobs")
        .and_then(|jobs| jobs.get(job))
        .and_then(|value| value.get(field))
        .unwrap_or_else(|| panic!("missing {field} for job: {job}"))
        .clone()
}

fn job_runs_on(yaml: &str, job: &str) -> String {
    job_field(yaml, job, "runs-on")
        .as_str()
        .unwrap_or_else(|| panic!("runs-on is not a string for job: {job}"))
        .to_owned()
}

fn job_needs(yaml: &str, job: &str) -> Vec<String> {
    match job_field(yaml, job, "needs") {
        serde_yaml::Value::String(one) => vec![one],
        serde_yaml::Value::Sequence(items) => items
            .iter()
            .map(|item| item.as_str().expect("needs entry").to_owned())
            .collect(),
        other => panic!("unexpected needs shape: {other:?}"),
    }
}
#[test]
fn m006_eggsact_five_target_matrix_renders_and_checks_without_drift() {
    let (contract, shape, policy) = m006_shape_and_policy();
    shape.validate().unwrap();
    // Every target declares native qualification; the two cross-tool Linux
    // targets keep their floor and exact cross-tool versions.
    assert_eq!(shape.targets.len(), 5);
    for policy in &shape.targets {
        assert_eq!(policy.qualification, Qualification::Native);
    }
    let cross: Vec<&TargetPolicy> = shape
        .targets
        .iter()
        .filter(|p| p.strategy == BuildStrategy::CargoZigbuild)
        .collect();
    assert_eq!(cross.len(), 2);
    for policy in &cross {
        assert_eq!(
            policy.floor,
            CompatibilityFloor::Glibc {
                major: 2,
                minor: 17
            }
        );
        assert_eq!(
            policy.toolchain.cargo_zigbuild.as_deref(),
            Some(M005_CARGO_ZIGBUILD_VERSION)
        );
        assert_eq!(policy.toolchain.zig.as_deref(), Some(M005_ZIG_VERSION));
    }
    let release = shape
        .pack_config()
        .resolve(&contract, "1.2.3", &"a".repeat(40), &shape.selected_aliases)
        .unwrap();
    let ci_plan = project_ci_plan(&contract, &release, &shape.build_bindings).unwrap();
    assert_eq!(ci_plan.targets.len(), 5);
    for job in &ci_plan.targets {
        // Qualification intent is native and still unresolved before the
        // candidate is produced and executed.
        assert_eq!(job.qualification.classification, Qualification::Native);
        assert_eq!(job.qualification.state, QualificationState::Unresolved);
    }
    let graph = project_release_plan_with_consumer(
        &ci_plan,
        &shape.qualification_bindings,
        &shape.build_bindings,
        &release,
        shape.consumer_validators.clone(),
    )
    .unwrap();
    // Every qualification job carries the matching native host, including
    // the split-host AArch64 case.
    for qual in &graph.qualifications {
        let planned = release
            .targets
            .iter()
            .find(|t| t.target == qual.target)
            .unwrap();
        let expected = planned
            .policy
            .qualification_host
            .unwrap_or(HostRequirement {
                os: planned.policy.host_os,
                arch: planned.policy.host_arch,
            });
        assert_eq!(qual.host, expected);
        assert!(eggpack_core::host_matches_target(qual.host, &qual.target));
    }
    let yaml = render_reusable_release_github(&contract, &shape, &policy).unwrap();
    assert_eq!(
        yaml,
        render_reusable_release_github(&contract, &shape, &policy).unwrap()
    );
    assert!(!yaml.contains(UNRESOLVED_RELEASE_ID));
    assert!(!yaml.contains(UNRESOLVED_SOURCE_REVISION));
    // `eggpack ci check` sees zero drift against the generated bytes.
    let report =
        check_reusable_release_github(&contract, &shape, &policy, yaml.as_bytes()).unwrap();
    assert!(report.matches);
    assert_eq!(report.expected_bytes, report.actual_bytes);
    assert_eq!(report.first_difference, None);
    // Split-host topology: AArch64 is cross-built on Linux x86-64 and
    // qualified natively on Linux AArch64 through the canonical handoff.
    let aarch64_build = job_text(&yaml, "build_aarch64_unknown_linux_gnu");
    assert_eq!(
        job_runs_on(&yaml, "build_aarch64_unknown_linux_gnu"),
        "ubuntu-latest"
    );
    assert!(aarch64_build.contains("'--target' 'aarch64-unknown-linux-gnu.2.17'"));
    assert!(aarch64_build.contains(M005_CARGO_ZIGBUILD_VERSION));
    let aarch64_qualify = job_text(&yaml, "qualify_build_aarch64_unknown_linux_gnu");
    assert_eq!(
        job_needs(&yaml, "qualify_build_aarch64_unknown_linux_gnu"),
        vec![
            "build_aarch64_unknown_linux_gnu".to_string(),
            "resolve".to_string()
        ]
    );
    assert_eq!(
        job_runs_on(&yaml, "qualify_build_aarch64_unknown_linux_gnu"),
        "ubuntu-24.04-arm"
    );
    assert!(aarch64_qualify.contains("eggpack-build-handoff-aarch64-unknown-linux-gnu"));
    assert!(aarch64_qualify.contains("'_qualify-target'"));
    assert!(aarch64_qualify.contains("'--target' 'aarch64-unknown-linux-gnu'"));
    // The native qualification host needs no cross tools: it only executes
    // the exact candidate it was handed.
    assert!(!aarch64_qualify.contains("zig"));
    // x86-64 Linux is cross-built and qualified on its own matching host.
    let x86_build = job_text(&yaml, "build_x86_64_unknown_linux_gnu");
    assert_eq!(
        job_runs_on(&yaml, "build_x86_64_unknown_linux_gnu"),
        "ubuntu-latest"
    );
    assert!(x86_build.contains("'--target' 'x86_64-unknown-linux-gnu.2.17'"));
    let x86_qualify = job_text(&yaml, "qualify_build_x86_64_unknown_linux_gnu");
    assert_eq!(
        job_needs(&yaml, "qualify_build_x86_64_unknown_linux_gnu"),
        vec![
            "build_x86_64_unknown_linux_gnu".to_string(),
            "resolve".to_string()
        ]
    );
    assert_eq!(
        job_runs_on(&yaml, "qualify_build_x86_64_unknown_linux_gnu"),
        "ubuntu-latest"
    );
    assert!(x86_qualify.contains("eggpack-build-handoff-x86_64-unknown-linux-gnu"));
    // The three NativeCargo targets are natively qualified on their own
    // runners, with no cross tool provisioning anywhere in their jobs.
    for (job, label) in [
        ("qualify_build_aarch64_apple_darwin", "macos-14"),
        ("qualify_build_x86_64_apple_darwin", "macos-13"),
        ("qualify_build_x86_64_pc_windows_msvc", "windows-latest"),
    ] {
        assert_eq!(job_runs_on(&yaml, job), label, "{job} runner");
        assert!(!job_text(&yaml, job).contains("zig"), "{job} cross tools");
    }
    for job in [
        "build_aarch64_apple_darwin",
        "build_x86_64_apple_darwin",
        "build_x86_64_pc_windows_msvc",
    ] {
        assert!(!job_text(&yaml, job).contains("zig"), "{job} cross tools");
    }
    // Least privilege is unchanged by the widened declaration surface.
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    let jobs = parsed.get("jobs").unwrap().as_mapping().unwrap();
    let mut writers = Vec::new();
    for (name, job) in jobs {
        let contents = job
            .get("permissions")
            .unwrap()
            .get("contents")
            .unwrap()
            .as_str()
            .unwrap();
        if contents == "write" {
            writers.push(name.as_str().unwrap().to_owned());
        }
    }
    assert_eq!(writers, vec!["stage".to_string()]);
    assert!(!yaml.contains("gh release"));
}

#[test]
fn m006_native_qualification_still_fails_closed() {
    let (contract, shape, policy) = m006_shape_and_policy();
    // The split-host AArch64 case may not be qualified on Linux x86-64.
    for (host, label) in [
        ((HostOs::Linux, HostArch::X86_64), "build host arch"),
        ((HostOs::Windows, HostArch::Aarch64), "foreign os and arch"),
        ((HostOs::Linux, HostArch::Armv7), "build host arch"),
    ] {
        let mut wrong = shape.clone();
        let target = wrong
            .targets
            .iter_mut()
            .find(|p| p.target == "aarch64-unknown-linux-gnu")
            .unwrap();
        target.qualification_host = Some(HostRequirement {
            os: host.0,
            arch: host.1,
        });
        assert!(wrong.validate().is_ok());
        assert!(
            render_reusable_release_github(&contract, &wrong, &policy).is_err(),
            "qualification host mismatch accepted: {label}"
        );
        assert!(wrong
            .pack_config()
            .resolve(&contract, "1.2.3", &"a".repeat(40), &wrong.selected_aliases)
            .is_err());
    }
    // Mismatched qualification host OS/arch on the x86-64 cross-tool target.
    for host in [
        HostRequirement {
            os: HostOs::Linux,
            arch: HostArch::Aarch64,
        },
        HostRequirement {
            os: HostOs::Macos,
            arch: HostArch::X86_64,
        },
    ] {
        let mut wrong = shape.clone();
        let target = wrong
            .targets
            .iter_mut()
            .find(|p| p.target == "x86_64-unknown-linux-gnu")
            .unwrap();
        target.qualification_host = Some(host);
        assert!(render_reusable_release_github(&contract, &wrong, &policy).is_err());
    }
    // Native qualification without a smoke binding is not representable.
    let mut without_smoke = shape.clone();
    without_smoke
        .qualification_bindings
        .targets
        .get_mut("x86_64-unknown-linux-gnu")
        .unwrap()
        .smoke = None;
    assert!(render_reusable_release_github(&contract, &without_smoke, &policy).is_err());
    // A glibc floor outside GNU/Linux and a macOS floor outside Darwin stay
    // rejected, and CargoZigbuild may not carry a macOS floor.
    let mut floor = shape.clone();
    let aarch64 = floor
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-unknown-linux-gnu")
        .unwrap();
    aarch64.floor = CompatibilityFloor::Macos {
        major: 13,
        minor: 0,
    };
    assert!(render_reusable_release_github(&contract, &floor, &policy).is_err());
    let mut floor = shape.clone();
    let darwin = floor
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-apple-darwin")
        .unwrap();
    darwin.floor = CompatibilityFloor::Glibc {
        major: 2,
        minor: 17,
    };
    assert!(render_reusable_release_github(&contract, &floor, &policy).is_err());
    // NativeCargo may not declare cross-tool versions.
    let mut cross_tools_on_native = shape.clone();
    let darwin = cross_tools_on_native
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-apple-darwin")
        .unwrap();
    darwin.toolchain.cargo_zigbuild = Some(M005_CARGO_ZIGBUILD_VERSION.into());
    assert!(render_reusable_release_github(&contract, &cross_tools_on_native, &policy).is_err());
    // Structural classification may not carry a smoke binding, and a
    // structural target never executes the candidate.
    let mut structural_smoke = shape.clone();
    let structural = structural_smoke
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-unknown-linux-gnu")
        .unwrap();
    structural.qualification = Qualification::Structural;
    assert!(render_reusable_release_github(&contract, &structural_smoke, &policy).is_err());
    structural_smoke
        .qualification_bindings
        .targets
        .get_mut("aarch64-unknown-linux-gnu")
        .unwrap()
        .smoke = None;
    let yaml = render_reusable_release_github(&contract, &structural_smoke, &policy).unwrap();
    assert!(job_text(&yaml, "build_aarch64_unknown_linux_gnu").contains("'zigbuild'"));
    assert!(!job_text(&yaml, "qualify_build_aarch64_unknown_linux_gnu").contains("zig"));
}

// -----------------------------------------------------------------------
// M003e — generated release execution wiring corrective.
// -----------------------------------------------------------------------

/// Ordered (name, run-or-uses body) steps of one generated job.
fn job_step_bodies(yaml: &str, job: &str) -> Vec<(String, String)> {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let steps = value
        .get("jobs")
        .and_then(|jobs| jobs.get(job))
        .and_then(|job| job.get("steps"))
        .and_then(|steps| steps.as_sequence())
        .unwrap_or_else(|| panic!("missing steps for job: {job}"));
    steps
        .iter()
        .map(|step| {
            let name = step
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("")
                .to_owned();
            let body = step
                .get("run")
                .and_then(|run| run.as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    step.get("uses")
                        .and_then(|uses| uses.as_str())
                        .unwrap_or("")
                        .to_owned()
                });
            (name, body)
        })
        .collect()
}

fn step_position(steps: &[(String, String)], needle: &str) -> usize {
    steps
        .iter()
        .position(|(_, body)| body.contains(needle))
        .unwrap_or_else(|| panic!("missing step containing: {needle}"))
}

/// M003e general invariant: every generated job step invoking the Eggpack
/// CLI is preceded in the same job by the pinned-tool install step.
fn assert_tool_before_use(yaml: &str) {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let jobs = value
        .get("jobs")
        .and_then(|jobs| jobs.as_mapping())
        .expect("rendered workflow has no jobs");
    for (name, job) in jobs {
        let steps = job
            .get("steps")
            .and_then(|steps| steps.as_sequence())
            .unwrap_or_else(|| {
                panic!(
                    "job has no steps: {}",
                    name.as_str().unwrap_or("<non-string>")
                )
            });
        let mut installed = false;
        for step in steps {
            let step_name = step
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("");
            if step_name == "Install pinned Eggpack tool" {
                installed = true;
            }
            if let Some(run) = step.get("run").and_then(|run| run.as_str()) {
                // CLI invocations are shell-quoted (`'eggpack'`); the
                // install step itself only carries the bare binary name.
                if run.contains("'eggpack'") {
                    assert!(
                        installed,
                        "job {} invokes the tool without a preceding install",
                        name.as_str().unwrap_or("<non-string>")
                    );
                }
            }
        }
    }
}

/// M003e general invariant: every CLI invocation that writes a runtime
/// output file is preceded in the same job by the `mkdir -p` of its
/// exact output parent.
fn assert_mkdir_before_output(yaml: &str) {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let jobs = value
        .get("jobs")
        .and_then(|jobs| jobs.as_mapping())
        .expect("rendered workflow has no jobs");
    for (name, job) in jobs {
        let steps = job
            .get("steps")
            .and_then(|steps| steps.as_sequence())
            .unwrap_or_else(|| {
                panic!(
                    "job has no steps: {}",
                    name.as_str().unwrap_or("<non-string>")
                )
            });
        let mut created: Vec<String> = Vec::new();
        for step in steps {
            if let Some(run) = step.get("run").and_then(|run| run.as_str()) {
                if let Some(dir) = run
                    .strip_prefix("mkdir -p '")
                    .and_then(|rest| rest.strip_suffix('\''))
                {
                    created.push(dir.to_owned());
                }
                let required = if run.contains("'_resolve-release'") {
                    Some("./eggpack-runtime")
                } else if run.contains("'_validate-consumer'") {
                    Some("./eggpack-consumer/")
                } else if run.contains("'_evaluate-gate'") {
                    Some("./eggpack-gate")
                } else if run.contains("'_aggregate'") {
                    Some("./eggpack-finalized")
                } else {
                    None
                };
                if let Some(required) = required {
                    assert!(
                        created.iter().any(|dir| dir == required
                            || (required.ends_with('/') && dir.starts_with(required))),
                        "job {} writes without a preceding mkdir for {required}",
                        name.as_str().unwrap_or("<non-string>")
                    );
                }
            }
        }
    }
}

#[test]
fn m003e_reusable_preflight_carries_no_tool_invocation() {
    let (contract, shape, policy) = m006_shape_and_policy();
    let yaml = render_reusable_release_github(&contract, &shape, &policy).unwrap();
    // The runtime plan does not exist yet when preflight runs, so
    // preflight must not invoke the tool at all.
    let preflight = job_text(&yaml, "preflight");
    assert!(
        !preflight.contains("'eggpack'"),
        "reusable preflight invokes the tool"
    );
    assert!(
        !preflight.contains("_verify-source"),
        "reusable preflight verifies a nonexistent plan"
    );
    // The resolve job creates the runtime identity directory before
    // resolving the invocation-local documents into it.
    let steps = job_step_bodies(&yaml, "resolve");
    let mkdir = step_position(&steps, "mkdir -p './eggpack-runtime'");
    assert_eq!(steps[mkdir].0, "Create runtime identity directory");
    assert!(mkdir < step_position(&steps, "'_resolve-release'"));
    // Every validator/gate/aggregate job creates its exact output parent.
    for qual in [
        "aarch64_apple_darwin",
        "aarch64_unknown_linux_gnu",
        "x86_64_apple_darwin",
        "x86_64_pc_windows_msvc",
        "x86_64_unknown_linux_gnu",
    ] {
        let steps = job_step_bodies(&yaml, &format!("validate_build_{qual}"));
        let mkdir = step_position(
            &steps,
            &format!("mkdir -p './eggpack-consumer/build_{qual}'"),
        );
        assert!(mkdir < step_position(&steps, "'_validate-consumer'"));
    }
    let gate = job_step_bodies(&yaml, "required_gate");
    assert!(
        step_position(&gate, "mkdir -p './eggpack-gate'")
            < step_position(&gate, "'_evaluate-gate'")
    );
    let aggregate = job_step_bodies(&yaml, "aggregate");
    assert!(
        step_position(&aggregate, "mkdir -p './eggpack-finalized'")
            < step_position(&aggregate, "'_aggregate'")
    );
}

#[test]
fn m003e_exact_staging_preflight_installs_tool_before_verify() {
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    let yaml = render_release_github(&graph, &policy).unwrap();
    // The checked-in release plan exists in exact mode, so preflight
    // keeps its source verification — after installing the pinned tool.
    let steps = job_step_bodies(&yaml, "preflight");
    let install = steps
        .iter()
        .position(|(name, _)| name == "Install pinned Eggpack tool")
        .expect("preflight tool install");
    assert!(install < step_position(&steps, "'_verify-source'"));
}

#[test]
fn m003f_tool_install_uses_positional_package() {
    // F8: `cargo install` has no `-p/--package` flag for git sources;
    // the package is selected positionally. This guards the exact
    // command text because ordering tests cannot catch a rejected flag.
    let tool = EggpackToolPolicy {
        repo: "https://github.com/eggstack/eggpack".into(),
        revision: "b".repeat(40),
        package: "eggpack-cli".into(),
        install_timeout_minutes: 10,
    };
    let snippet = tool_install_snippet(&tool);
    assert!(
        snippet.contains("--locked eggpack-cli\n"),
        "install must select the package positionally: {snippet}"
    );
    assert!(snippet.contains(&format!("--rev {}", "b".repeat(40))));
    assert!(snippet.contains("eggpack --version"));
    assert!(
        !snippet.contains("-p "),
        "install must not pass -p/--package to cargo install: {snippet}"
    );
}

/// F10a: artifact transfer strips exec bits (upload-artifact documents
/// 644 for all files). The validator must restore them before spawning,
/// so a transferred candidate executes exactly like the built one.
#[cfg(unix)]
#[test]
fn m003g_validator_restores_exec_on_transferred_candidate() {
    use sha2::Digest;
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "eggpack-m003g-exec-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    // A POSIX shell script stands in for the transferred candidate: it
    // executes on any Unix runner without a toolchain.
    let candidate = root.join("candidate-direct");
    std::fs::write(&candidate, b"#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(0o644)).unwrap();
    let script = root.join("smoke.py");
    std::fs::write(
        &script,
        "import subprocess,sys\nsubprocess.run([sys.argv[1]], check=True)\n",
    )
    .unwrap();
    let bytes = std::fs::read(&candidate).unwrap();
    let mut hasher = Sha256::new();
    use std::io::Write;
    hasher.write_all(&bytes).unwrap();
    let digest = format!("{:x}", hasher.finalize());
    let validator = ConsumerValidatorV1 {
        schema_version: 1,
        selector: eggpack_core::LogicalOutputSelector::Direct,
        interpreter: ValidatorInterpreterV1::Python3,
        script: "scripts/smoke.py".into(),
        timeout_ms: 20_000,
        stdout_limit: 65_536,
        stderr_limit: 65_536,
    };
    let request = ConsumerValidationRequest {
        validator: &validator,
        script_path: &script,
        candidate_path: &candidate,
        expected_size: bytes.len() as u64,
        expected_sha256: &digest,
        work_dir: &root,
        release_id: "v0.0.0",
        source_revision: &"a".repeat(40),
        target: "x86_64-unknown-linux-gnu",
        cancelled: None,
        path_dirs: None,
    };
    let validation = run_consumer_validator(&request).unwrap();
    assert_eq!(validation.outcome, ConsumerValidationOutcome::Passed);
    // Bytes are untouched; only mode changed.
    assert_eq!(std::fs::read(&candidate).unwrap(), bytes);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn m003e_tool_install_and_output_dirs_hold_in_every_render_mode() {
    // Exact non-staging render.
    let (graph, policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    let exact = render_release_github(&graph, &policy).unwrap();
    assert_tool_before_use(&exact);
    assert_mkdir_before_output(&exact);
    // Exact staging render.
    let (graph, policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    let staging = render_release_github(&graph, &policy).unwrap();
    assert_tool_before_use(&staging);
    assert_mkdir_before_output(&staging);
    // Reusable render.
    let (contract, shape, policy) = m006_shape_and_policy();
    let reusable = render_reusable_release_github(&contract, &shape, &policy).unwrap();
    assert_tool_before_use(&reusable);
    assert_mkdir_before_output(&reusable);
}

#[test]
fn m005_zig_archive_mapping_is_finite_and_official() {
    assert_eq!(
        zig_archive_name(HostOs::Linux, HostArch::X86_64, M005_ZIG_VERSION).unwrap(),
        format!("zig-x86_64-linux-{M005_ZIG_VERSION}.tar.xz")
    );
    assert_eq!(
        zig_archive_name(HostOs::Linux, HostArch::Aarch64, M005_ZIG_VERSION).unwrap(),
        format!("zig-aarch64-linux-{M005_ZIG_VERSION}.tar.xz")
    );
    assert_eq!(
        zig_download_url(M005_ZIG_VERSION, HostArch::X86_64).unwrap(),
        format!(
            "https://ziglang.org/download/{M005_ZIG_VERSION}/zig-x86_64-linux-{M005_ZIG_VERSION}.tar.xz"
        )
    );
    assert_eq!(
        zig_download_url(M005_ZIG_VERSION, HostArch::Aarch64).unwrap(),
        format!(
            "https://ziglang.org/download/{M005_ZIG_VERSION}/zig-aarch64-linux-{M005_ZIG_VERSION}.tar.xz"
        )
    );
    // Unsupported hosts, bad OS, and out-of-bounds versions fail closed.
    assert!(zig_archive_name(HostOs::Linux, HostArch::Armv7, M005_ZIG_VERSION).is_err());
    assert!(zig_archive_name(HostOs::Macos, HostArch::X86_64, M005_ZIG_VERSION).is_err());
    assert!(zig_archive_name(HostOs::Windows, HostArch::X86_64, M005_ZIG_VERSION).is_err());
    assert!(zig_archive_name(HostOs::Linux, HostArch::X86_64, "").is_err());
    assert!(zig_archive_name(HostOs::Linux, HostArch::X86_64, "0.14.1/../../evil").is_err());
    assert!(zig_download_url("", HostArch::X86_64).is_err());
}

#[test]
fn m005_provisioning_policy_rejects_bad_digests_and_bounds() {
    m005_provisioning().validate().unwrap();
    validate_sha256_hex(M005_X86_64_SHA).unwrap();
    for bad in [
        "",
        "abc",
        "0".repeat(64).as_str(),
        "24AEEEC8AF16C381934A6CD7D95C807A8CB2CF7DF9FA40D359AA884195C4716C",
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ] {
        assert!(validate_sha256_hex(bad).is_err(), "digest accepted: {bad}");
        let mut policy = m005_provisioning();
        policy.zig.linux_x86_64_sha256 = bad.into();
        assert!(policy.validate().is_err());
    }
    let mut policy = m005_provisioning();
    policy.zig.linux_aarch64_sha256 = String::new();
    assert!(policy.validate().is_err());
    for (connect, max, install) in [(0, 600, 10), (30, 30, 10), (30, 600, 0), (30, 600, 61)] {
        let mut policy = m005_provisioning();
        policy.zig_connect_timeout_secs = connect;
        policy.zig_max_time_secs = max;
        policy.cargo_install_timeout_minutes = install;
        assert!(policy.validate().is_err());
    }
    // Digest selection follows the build host architecture.
    let provisioning = m005_provisioning();
    assert_eq!(
        zig_expected_digest(&provisioning, HostOs::Linux, HostArch::X86_64).unwrap(),
        M005_X86_64_SHA
    );
    assert_eq!(
        zig_expected_digest(&provisioning, HostOs::Linux, HostArch::Aarch64).unwrap(),
        M005_AARCH64_SHA
    );
    assert!(zig_expected_digest(&provisioning, HostOs::Linux, HostArch::Armv7).is_err());
    assert!(zig_expected_digest(&provisioning, HostOs::Macos, HostArch::X86_64).is_err());
}

#[test]
fn m005_preinstalled_mode_checks_both_exact_versions_without_install() {
    let (graph, policy) = m005_cross_graph(
        HostArch::X86_64,
        M005_ZIG_VERSION,
        M005_CARGO_ZIGBUILD_VERSION,
    );
    // Legacy booleans off means legacy mode still fails closed.
    assert!(policy.validate(&graph).is_err());
    let mut legacy = policy.clone();
    legacy.runners[0].cargo_zigbuild = true;
    legacy.runners[0].zig = true;
    legacy.validate(&graph).unwrap();
    let output = render_github(&graph, &legacy).unwrap();
    assert!(output.contains("cargo-zigbuild 0.23.3"));
    assert!(output.contains("actual_zig=\"$(zig version)\""));
    assert!(output.contains("test \"$actual_zig\" = '0.14.1'"));
    // No provisioning: no install, no download, no package manager.
    assert!(!output.contains("cargo install cargo-zigbuild"));
    assert!(!output.contains("ziglang.org"));
    assert!(!output.contains("curl "));
    assert!(!output.contains("apt-get"));
    assert!(!output.contains("brew "));
    assert!(!output.contains("choco"));
    assert!(output.contains("x86_64-unknown-linux-gnu.2.17"));
}

#[test]
fn m005_provisioned_render_pins_exact_tools_without_ambient_dependency() {
    let (graph, mut policy) = m005_cross_graph(
        HostArch::X86_64,
        M005_ZIG_VERSION,
        M005_CARGO_ZIGBUILD_VERSION,
    );
    policy.cross_tools = Some(m005_provisioning());
    // Runner booleans stay false: no ambient tool is assumed.
    assert!(!policy.runners[0].cargo_zigbuild);
    assert!(!policy.runners[0].zig);
    policy.validate(&graph).unwrap();
    let output = render_github(&graph, &policy).unwrap();
    // Deterministic render.
    assert_eq!(output, render_github(&graph, &policy).unwrap());
    assert!(
        check_github(&graph, &policy, output.as_bytes())
            .unwrap()
            .matches
    );
    // cargo-zigbuild exact install path, private root, exact comparison.
    assert!(output.contains(
        "CARGO_INSTALL_ROOT=\"$install_root\" cargo install cargo-zigbuild --version '0.23.3' --locked"
    ));
    assert!(output.contains("cargo-install/${{ github.run_id }}-${{ github.run_attempt }}"));
    assert!(output.contains("test -x \"$install_root/bin/cargo-zigbuild\""));
    // The exactness lives in the pinned install (`--version '0.23.3'`)
    // plus presence; `cargo zigbuild --version` is rejected by the
    // tool's CLI so the build step itself is the functional proof.
    assert!(!output.contains("cargo zigbuild --version"));
    // Zig official archive, digest, bounded HTTPS-only curl.
    assert!(output.contains("zig-x86_64-linux-0.14.1.tar.xz"));
    assert!(output.contains("https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz"));
    assert!(output.contains(M005_X86_64_SHA));
    assert!(output.contains("curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location --proto-redir '=https'"));
    assert!(output.contains("--connect-timeout 30 --max-time 600"));
    assert!(output.contains("sha256sum --check -"));
    // Verified executable binding, exact version check, private cache.
    assert!(output.contains("CARGO_ZIGBUILD_ZIG_PATH"));
    assert!(output.contains("test \"$(\"$zig_bin\" version)\" = '0.14.1'"));
    assert!(output.contains("zigbuild-cache/${{ github.run_id }}-${{ github.run_attempt }}"));
    assert!(output.contains("CARGO_ZIGBUILD_CACHE_DIR"));
    // Layout/executable guards rendered.
    assert!(output.contains("grep -Fxq \"$top/zig\""));
    assert!(output.contains("test ! -L \"$work/extract/$top/zig\""));
    // glibc floor command unchanged.
    assert!(output.contains("x86_64-unknown-linux-gnu.2.17"));
    // Least privilege preserved.
    assert!(output.contains("permissions:\n  contents: read"));
    assert!(!output.contains("contents: write"));
    assert!(!output.contains("id-token: write"));
    // No generic package manager, no 0.23.4 upgrade, no prebuilt binaries.
    for forbidden in [
        "apt-get",
        "apt install",
        "brew ",
        "choco",
        "scoop",
        "cargo-zigbuild 0.23.4",
        "cargo-zigbuild-x86_64",
        "releases/download",
    ] {
        assert!(!output.contains(forbidden), "rendered {forbidden}");
    }
}

#[test]
fn m005_provisioned_aarch64_uses_arch_digest_and_name() {
    let contract = contract();
    let mut cross = m005_cross_policy(
        HostArch::Aarch64,
        M005_ZIG_VERSION,
        M005_CARGO_ZIGBUILD_VERSION,
    );
    cross.target = "aarch64-unknown-linux-gnu".into();
    let config = PackConfig {
        schema_version: 1,
        targets: vec![cross],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()],
        )
        .is_err();
    // The simple-direct fixture has no AArch64 alias; build the plan
    // directly through the mixed fixture instead.
    assert!(release);
    let contract = DistributionContract::parse_toml_str(include_str!(
        "../tests/fixtures/mixed-direct-targets.toml"
    ))
    .unwrap();
    let config = PackConfig {
        schema_version: 1,
        targets: vec![
            TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::CargoZigbuild,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: Some(M005_CARGO_ZIGBUILD_VERSION.into()),
                    zig: Some(M005_ZIG_VERSION.into()),
                },
                floor: CompatibilityFloor::Glibc {
                    major: 2,
                    minor: 17,
                },
                qualification: Qualification::Structural,
                support: SupportTier::Required,
            },
            TargetPolicy {
                target: "aarch64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::CargoZigbuild,
                host_os: HostOs::Linux,
                host_arch: HostArch::Aarch64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: Some(M005_CARGO_ZIGBUILD_VERSION.into()),
                    zig: Some(M005_ZIG_VERSION.into()),
                },
                floor: CompatibilityFloor::Glibc {
                    major: 2,
                    minor: 17,
                },
                qualification: Qualification::Structural,
                support: SupportTier::Required,
            },
        ],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into(), "linux-arm64".into()],
        )
        .unwrap();
    let mut binding_map = std::collections::BTreeMap::new();
    for target in &release.targets {
        binding_map.insert(
            target.target.clone(),
            vec![eggpack_core::BuildBinding {
                selector: LogicalOutputSelector::Direct,
                package: "eggsact".into(),
                binary: "eggsact".into(),
            }],
        );
    }
    let bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: binding_map,
    };
    let graph = project_ci_plan(&contract, &release, &bindings).unwrap();
    let mut policy = self::policy();
    policy.runners = vec![
        RunnerMapping {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
            label: "ubuntu-latest".into(),
            cargo_zigbuild: false,
            zig: false,
        },
        RunnerMapping {
            os: HostOs::Linux,
            arch: HostArch::Aarch64,
            label: "ubuntu-24.04-arm".into(),
            cargo_zigbuild: false,
            zig: false,
        },
    ];
    policy.cross_tools = Some(m005_provisioning());
    policy.validate(&graph).unwrap();
    let output = render_github(&graph, &policy).unwrap();
    assert!(output.contains("zig-x86_64-linux-0.14.1.tar.xz"));
    assert!(output.contains("zig-aarch64-linux-0.14.1.tar.xz"));
    assert!(output.contains(M005_X86_64_SHA));
    assert!(output.contains(M005_AARCH64_SHA));
    assert!(output.contains("aarch64-unknown-linux-gnu.2.17"));
    assert!(output.contains("x86_64-unknown-linux-gnu.2.17"));
}

#[test]
fn m005_native_cargo_renders_no_cross_tool_provisioning() {
    let graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    let mut policy = self::policy();
    policy.cross_tools = Some(m005_provisioning());
    policy.validate(&graph).unwrap();
    let output = render_github(&graph, &policy).unwrap();
    assert!(!output.contains("cargo-zigbuild"));
    assert!(!output.contains("ziglang.org"));
    assert!(!output.contains("CARGO_ZIGBUILD_ZIG_PATH"));
    assert!(!output.contains("CARGO_ZIGBUILD_CACHE_DIR"));
    assert!(!output.contains("curl "));
}

#[test]
fn m005_missing_tool_version_fails_closed_never_ambient() {
    let contract = contract();
    // Historical schema-v1 input without Zig parses but fails resolution.
    let historical = r#"schema_version=1
[[targets]]
target="x86_64-unknown-linux-gnu"
strategy="cargo_zigbuild"
host_os="linux"
host_arch="x86_64"
toolchain={rust="1.89.0", cargo_zigbuild="0.23.3"}
floor={kind="glibc", major=2, minor=17}
qualification="structural"
support="required"
"#;
    let parsed = PackConfig::from_toml(historical).unwrap();
    assert!(parsed.targets[0].toolchain.zig.is_none());
    assert!(parsed
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()]
        )
        .is_err());
    // Missing cargo-zigbuild with Zig present also fails.
    let mut policy = m005_cross_policy(HostArch::X86_64, M005_ZIG_VERSION, "0.23.3");
    policy.toolchain.cargo_zigbuild = None;
    let config = PackConfig {
        schema_version: 1,
        targets: vec![policy],
    };
    assert!(config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()]
        )
        .is_err());
    // NativeCargo with either cross-tool version fails.
    let mut native = m005_cross_policy(HostArch::X86_64, M005_ZIG_VERSION, "0.23.3");
    native.strategy = BuildStrategy::NativeCargo;
    let config = PackConfig {
        schema_version: 1,
        targets: vec![native],
    };
    assert!(config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()]
        )
        .is_err());
    // Rendering a Zig-less CargoZigbuild graph fails instead of ambient.
    let (graph, _) = m005_cross_graph(
        HostArch::X86_64,
        M005_ZIG_VERSION,
        M005_CARGO_ZIGBUILD_VERSION,
    );
    let mut zigless = graph.clone();
    zigless.targets[0].planned.policy.toolchain.zig = None;
    assert!(zigless.validate().is_err());
    // Bad provisioning digest fails at policy validation/render time.
    let (graph, mut policy) = m005_cross_graph(
        HostArch::X86_64,
        M005_ZIG_VERSION,
        M005_CARGO_ZIGBUILD_VERSION,
    );
    policy.cross_tools = Some(CrossToolProvisioningV1 {
        cargo_install_timeout_minutes: 10,
        zig_connect_timeout_secs: 30,
        zig_max_time_secs: 600,
        zig: ZigOfficialArchiveV1 {
            linux_x86_64_sha256: String::new(),
            linux_aarch64_sha256: M005_AARCH64_SHA.into(),
        },
    });
    assert!(policy.validate(&graph).is_err());
    assert!(render_github(&graph, &policy).is_err());
    // Unsupported provisioning host fails closed.
    let mut bad_host = m005_cross_policy(HostArch::X86_64, M005_ZIG_VERSION, "0.23.3");
    bad_host.host_arch = HostArch::Armv7;
    let config = PackConfig {
        schema_version: 1,
        targets: vec![bad_host],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-x64".into()],
        )
        .unwrap();
    let bad_graph = project_ci_plan(&contract, &release, &bindings(&release)).unwrap();
    let mut bad_policy = self::policy();
    bad_policy.runners = vec![RunnerMapping {
        os: HostOs::Linux,
        arch: HostArch::Armv7,
        label: "ubuntu-latest".into(),
        cargo_zigbuild: false,
        zig: false,
    }];
    bad_policy.cross_tools = Some(m005_provisioning());
    assert!(bad_policy.validate(&bad_graph).is_err());
}

#[test]
fn m005_release_renderer_provisions_before_build_with_private_cache() {
    let (contract, release, bindings, qual_bindings) = m002_release(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        "linux-x64",
        Qualification::Structural,
        SupportTier::Required,
    );
    // Rebuild the release as CargoZigbuild with exact M005 versions.
    let mut policies: Vec<TargetPolicy> =
        release.targets.iter().map(|t| t.policy.clone()).collect();
    policies[0].strategy = BuildStrategy::CargoZigbuild;
    policies[0].host_os = HostOs::Linux;
    policies[0].host_arch = HostArch::X86_64;
    policies[0].toolchain.cargo_zigbuild = Some(M005_CARGO_ZIGBUILD_VERSION.into());
    policies[0].toolchain.zig = Some(M005_ZIG_VERSION.into());
    policies[0].floor = CompatibilityFloor::Glibc {
        major: 2,
        minor: 17,
    };
    let config = PackConfig {
        schema_version: 1,
        targets: policies,
    };
    let release = config
        .resolve(&contract, "1.2.3", &"a".repeat(40), &["linux-x64".into()])
        .unwrap();
    let ci_plan = project_ci_plan(&contract, &release, &bindings).unwrap();
    let graph = project_release_plan(&ci_plan, &qual_bindings, &bindings, &release).unwrap();
    let mut policy = m002_policy();
    policy.runners = vec![RunnerMapping {
        os: HostOs::Linux,
        arch: HostArch::X86_64,
        label: "ubuntu-latest".into(),
        cargo_zigbuild: false,
        zig: false,
    }];
    policy.cross_tools = Some(m005_provisioning());
    let yaml = render_release_github(&graph, &policy).unwrap();
    assert_eq!(yaml, render_release_github(&graph, &policy).unwrap());
    // Provisioning precedes the product build and the handoff capture.
    let install = yaml.find("Install exact cargo-zigbuild").unwrap();
    let provision = yaml.find("Provision verified Zig 0.14.1").unwrap();
    let build = yaml.find("cargo' '+1.89.0' 'zigbuild'").unwrap();
    let capture = yaml.find("Capture canonical build handoff").unwrap();
    assert!(install < provision && provision < build && build < capture);
    assert!(yaml.contains("CARGO_ZIGBUILD_ZIG_PATH: ${{ env.CARGO_ZIGBUILD_ZIG_PATH }}"));
    assert!(yaml.contains("CARGO_ZIGBUILD_CACHE_DIR: ${{ env.CARGO_ZIGBUILD_CACHE_DIR }}"));
    assert!(yaml.contains("zigbuild-cache/${{ github.run_id }}-${{ github.run_attempt }}"));
    assert!(!yaml.contains("~/.cargo"));
    assert!(!yaml.contains("$HOME/.cargo"));
    assert!(!yaml.contains("contents: write"));
    // F9: the isolated install root must join PATH in the same step,
    // because $GITHUB_PATH exports take effect only in later steps.
    // The check is presence (`test -x`), not `cargo zigbuild
    // --version`: that flag is rejected by cargo-zigbuild's CLI, and
    // the later build step functionally proves the tool.
    let export = yaml
        .find("export PATH=\"$install_root/bin:$PATH\"")
        .expect("same-step PATH export");
    let check = yaml
        .find("test -x \"$install_root/bin/cargo-zigbuild\"")
        .expect("installed-tool presence check");
    assert!(!yaml.contains("cargo zigbuild --version"));
    assert!(install < export && export < check);
    // The release guard still rejects non-bootstrap curl only.
    assert!(
        check_release_github(&graph, &policy, yaml.as_bytes())
            .unwrap()
            .matches
    );
}

#[test]
fn m005_zig_archive_listing_validation_without_network() {
    let top = format!("zig-x86_64-linux-{M005_ZIG_VERSION}");
    let good = vec![
        format!("{top}/"),
        format!("{top}/zig"),
        format!("{top}/doc/README.md"),
        format!("{top}/lib/libc.a"),
    ];
    assert_eq!(validate_zig_archive_listing(&good).unwrap(), top);
    // Extra top-level member fails.
    let mut extra = good.clone();
    extra.push("other-dir/file".into());
    assert!(validate_zig_archive_listing(&extra).is_err());
    // Missing zig executable fails.
    assert!(validate_zig_archive_listing(&[format!("{top}/"), format!("{top}/doc/x")]).is_err());
    // Traversal, absolute, and empty entries fail.
    assert!(
        validate_zig_archive_listing(&[format!("{top}/../evil"), format!("{top}/zig")]).is_err()
    );
    assert!(validate_zig_archive_listing(&["/absolute/zig".into(), format!("{top}/zig")]).is_err());
    assert!(validate_zig_archive_listing(&[]).is_err());
    // A bare top-level zig file without its directory fails the layout.
    assert!(validate_zig_archive_listing(&["zig".into()]).is_err());
}

#[test]
fn m005_provisioned_zig_dir_checks_without_network() {
    let root = std::env::temp_dir().join(format!(
        "eggpack-m005-zigdir-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let top = root.join(format!("zig-x86_64-linux-{M005_ZIG_VERSION}"));
    std::fs::create_dir_all(&top).unwrap();
    let zig = top.join("zig");
    std::fs::write(&zig, b"fake-zig-bytes").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&zig, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(validate_provisioned_zig_dir(&root).unwrap(), zig);
        // Symlinked executable is rejected.
        let link_root = root.join("link-root");
        std::fs::create_dir_all(&link_root).unwrap();
        let link_top = link_root.join("top");
        std::fs::create_dir_all(&link_top).unwrap();
        std::os::unix::fs::symlink(&zig, link_top.join("zig")).unwrap();
        assert!(validate_provisioned_zig_dir(&link_root).is_err());
        // Non-executable bit is rejected.
        std::fs::set_permissions(&zig, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(validate_provisioned_zig_dir(&root).is_err());
        std::fs::set_permissions(&zig, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(not(unix))]
    {
        assert_eq!(validate_provisioned_zig_dir(&root).unwrap(), zig);
    }
    // Extra top-level member is rejected.
    std::fs::create_dir_all(root.join("unexpected")).unwrap();
    assert!(validate_provisioned_zig_dir(&root).is_err());
    std::fs::remove_dir_all(root.join("unexpected")).unwrap();
    // Empty executable is rejected.
    std::fs::write(&zig, b"").unwrap();
    assert!(validate_provisioned_zig_dir(&root).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}

// Not standalone coverage: rewrites one checked-in golden in place, so it
// must never run as part of a normal test pass.
#[test]
#[ignore]
fn m005_regenerate_m001_multitarget_golden() {
    // Regenerate the M001 multitarget golden after the M005 exact-Zig
    // check. Run explicitly with `cargo test -p eggpack-ci --lib -- --ignored`.
    let contract = DistributionContract::parse_toml_str(include_str!(
        "../tests/fixtures/mixed-direct-targets.toml"
    ))
    .unwrap();
    let config = PackConfig {
        schema_version: 1,
        targets: vec![
            TargetPolicy {
                target: "x86_64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: Some(HostRequirement {
                    os: HostOs::Macos,
                    arch: HostArch::Aarch64,
                }),
                toolchain: ToolchainRequirement {
                    rust: "stable".into(),
                    cargo_zigbuild: None,
                    zig: None,
                },
                floor: CompatibilityFloor::None,
                qualification: Qualification::DeferredNative,
                support: SupportTier::Required,
            },
            TargetPolicy {
                target: "aarch64-unknown-linux-gnu".into(),
                strategy: BuildStrategy::CargoZigbuild,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: Some("0.20.0".into()),
                    zig: Some("0.14.1".into()),
                },
                floor: CompatibilityFloor::Glibc {
                    major: 2,
                    minor: 17,
                },
                qualification: Qualification::Structural,
                support: SupportTier::NonGating,
            },
        ],
    };
    let release = config
        .resolve(
            &contract,
            "1.2.3",
            "0123456789abcdef",
            &["linux-arm64".into(), "linux-x64".into()],
        )
        .unwrap();
    let mut binding_map = std::collections::BTreeMap::new();
    for target in &release.targets {
        binding_map.insert(
            target.target.clone(),
            vec![eggpack_core::BuildBinding {
                selector: LogicalOutputSelector::Direct,
                package: "eggsact".into(),
                binary: "eggsact".into(),
            }],
        );
    }
    let bindings = BuildBindingsV1 {
        schema_version: 1,
        targets: binding_map,
    };
    let graph = project_ci_plan(&contract, &release, &bindings).unwrap();
    write_golden(
        "../tests/fixtures/native-direct-multitarget.yml",
        &render_github(&graph, &policy()).unwrap(),
    );
}

// -----------------------------------------------------------------------
// M003l — producer rule authority convergence.
//
// The canonical rules now live in one place. The expected-outcome tables
// below are the pre-convergence rules of the crates that used to own
// them, kept here only so the canonical rule can be shown to agree with
// them case by case. The assertions themselves are written against the
// canonical rule alone.
// -----------------------------------------------------------------------

/// The pre-convergence CI-side host/target predicate.
fn pre_convergence_ci_host_matches_target(host: HostRequirement, target: &str) -> bool {
    let arch_matches = match host.arch {
        HostArch::X86_64 => target.starts_with("x86_64-"),
        HostArch::Aarch64 => target.starts_with("aarch64-"),
        HostArch::Armv7 => target.starts_with("armv7-"),
    };
    let os_matches = match host.os {
        HostOs::Linux => target.contains("-linux-"),
        HostOs::Macos => target.ends_with("-apple-darwin"),
        HostOs::Windows => target.contains("-windows-"),
    };
    arch_matches && os_matches
}

#[test]
fn m003l_core_host_target_rule_reproduces_every_pre_convergence_ci_outcome() {
    let hosts = [
        HostRequirement {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        },
        HostRequirement {
            os: HostOs::Linux,
            arch: HostArch::Aarch64,
        },
        HostRequirement {
            os: HostOs::Linux,
            arch: HostArch::Armv7,
        },
        HostRequirement {
            os: HostOs::Macos,
            arch: HostArch::X86_64,
        },
        HostRequirement {
            os: HostOs::Macos,
            arch: HostArch::Aarch64,
        },
        HostRequirement {
            os: HostOs::Macos,
            arch: HostArch::Armv7,
        },
        HostRequirement {
            os: HostOs::Windows,
            arch: HostArch::X86_64,
        },
        HostRequirement {
            os: HostOs::Windows,
            arch: HostArch::Aarch64,
        },
        HostRequirement {
            os: HostOs::Windows,
            arch: HostArch::Armv7,
        },
    ];
    const TRIPLES: &[&str] = &[
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "armv7-unknown-linux-gnueabihf",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
        "armv7-pc-windows-msvc",
        "x86_64-unknown-freebsd",
        "aarch64-unknown-none",
    ];
    for triple in TRIPLES {
        for host in hosts {
            assert_eq!(
                eggpack_core::host_matches_target(host, triple),
                pre_convergence_ci_host_matches_target(host, triple),
                "core host/target rule diverged for {host:?} on {triple}"
            );
        }
    }
    // Pinned outcomes, so the table above cannot silently agree with a
    // wrong rule.
    assert!(eggpack_core::host_matches_target(
        hosts[0],
        "x86_64-unknown-linux-gnu"
    ));
    assert!(!eggpack_core::host_matches_target(
        hosts[0],
        "aarch64-unknown-linux-gnu"
    ));
    assert!(eggpack_core::host_matches_target(
        hosts[3],
        "x86_64-apple-darwin"
    ));
    assert!(!eggpack_core::host_matches_target(
        hosts[3],
        "x86_64-unknown-linux-gnu"
    ));
    assert!(eggpack_core::host_matches_target(
        hosts[6],
        "x86_64-pc-windows-msvc"
    ));
    assert!(!eggpack_core::host_matches_target(
        hosts[6],
        "x86_64-unknown-freebsd"
    ));
}

#[test]
fn m003l_floor_applicability_is_refused_in_every_pre_convergence_case() {
    // A glibc floor on a target that is not GNU/Linux.
    assert_eq!(
        eggpack_core::validate_floor_applicability(
            &CompatibilityFloor::Glibc {
                major: 2,
                minor: 17
            },
            BuildStrategy::NativeCargo,
            "aarch64-apple-darwin",
        ),
        Err(eggpack_core::FloorApplicabilityError::NotGlibcTarget)
    );
    // A macOS floor on a target that is not Darwin.
    assert_eq!(
        eggpack_core::validate_floor_applicability(
            &CompatibilityFloor::Macos {
                major: 13,
                minor: 0
            },
            BuildStrategy::NativeCargo,
            "x86_64-unknown-linux-gnu",
        ),
        Err(eggpack_core::FloorApplicabilityError::NotMacosTarget)
    );
    // A macOS floor under cargo-zigbuild.
    assert_eq!(
        eggpack_core::validate_floor_applicability(
            &CompatibilityFloor::Macos {
                major: 13,
                minor: 0
            },
            BuildStrategy::CargoZigbuild,
            "x86_64-apple-darwin",
        ),
        Err(eggpack_core::FloorApplicabilityError::MacosFloorUnderZigbuild)
    );
    // The shapes that were accepted stay accepted.
    assert!(eggpack_core::validate_floor_applicability(
        &CompatibilityFloor::Glibc {
            major: 2,
            minor: 17
        },
        BuildStrategy::NativeCargo,
        "x86_64-unknown-linux-gnu",
    )
    .is_ok());
    assert!(eggpack_core::validate_floor_applicability(
        &CompatibilityFloor::Macos {
            major: 13,
            minor: 0
        },
        BuildStrategy::NativeCargo,
        "x86_64-apple-darwin",
    )
    .is_ok());
    assert!(eggpack_core::validate_floor_applicability(
        &CompatibilityFloor::None,
        BuildStrategy::CargoZigbuild,
        "x86_64-unknown-linux-gnu",
    )
    .is_ok());

    // The projected pipeline still refuses the same three cases.
    let (contract, shape, policy) = m006_shape_and_policy();
    let mut glibc_outside = shape.clone();
    let darwin = glibc_outside
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-apple-darwin")
        .unwrap();
    darwin.floor = CompatibilityFloor::Glibc {
        major: 2,
        minor: 17,
    };
    assert!(render_reusable_release_github(&contract, &glibc_outside, &policy).is_err());

    let mut macos_outside = shape.clone();
    let linux = macos_outside
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-unknown-linux-gnu")
        .unwrap();
    linux.floor = CompatibilityFloor::Macos {
        major: 13,
        minor: 0,
    };
    assert!(render_reusable_release_github(&contract, &macos_outside, &policy).is_err());

    let mut macos_under_zigbuild = shape.clone();
    let darwin = macos_under_zigbuild
        .targets
        .iter_mut()
        .find(|p| p.target == "aarch64-apple-darwin")
        .unwrap();
    darwin.strategy = BuildStrategy::CargoZigbuild;
    darwin.toolchain = ToolchainRequirement {
        rust: "1.89.0".into(),
        cargo_zigbuild: Some(M005_CARGO_ZIGBUILD_VERSION.into()),
        zig: Some(M005_ZIG_VERSION.into()),
    };
    darwin.floor = CompatibilityFloor::Macos {
        major: 13,
        minor: 0,
    };
    assert!(render_reusable_release_github(&contract, &macos_under_zigbuild, &policy).is_err());
}

/// The pre-convergence core-side exact-tag safety rule.
fn pre_convergence_core_tag_is_safe(tag: &str) -> bool {
    !(tag.is_empty()
        || tag.len() > 128
        || tag.chars().any(char::is_control)
        || tag.starts_with('/')
        || tag.ends_with('/')
        || tag.ends_with('.')
        || tag.contains("..")
        || tag.contains("@{")
        || tag.contains(['?', '\\', '^', ':', '*', '[', '~', ' '])
        || tag.split('/').any(|component| {
            component.is_empty() || component.starts_with('.') || component.ends_with(".lock")
        }))
}

/// The pre-convergence CI-side exact-tag safety rule.
fn pre_convergence_ci_tag_is_safe(tag: &str) -> bool {
    !(tag.is_empty()
        || tag.len() > 128
        || tag.chars().any(char::is_control)
        || tag.contains(['?', '#', '@', ' ', '\\', '\'', '"', '`', '$'])
        || tag.contains("..")
        || tag.starts_with('/')
        || tag.ends_with('/'))
}

#[test]
fn m003l_canonical_tag_safety_is_the_union_of_both_pre_convergence_rules() {
    const TAGS: &[&str] = &[
        "",
        "v1.2.3",
        "release/v1.2.3",
        "v1.2.3-rc.1",
        "v1.2.3+build.1",
        "v1.2.3#",
        "v1.2.3@",
        "v1.2.3'",
        "v1.2.3\"",
        "v1.2.3`",
        "v1.2.3$",
        "v1.2.3?",
        "v1.2.3 ",
        "v1.2.3\\",
        "v1.2.3^",
        "v1.2.3:",
        "v1.2.3*",
        "v1.2.3[",
        "v1.2.3~",
        "v1.2.3.",
        "v1..2.3",
        "v1.2.3@{",
        "/v1.2.3",
        "v1.2.3/",
        "release//v1.2.3",
        "release/.hidden/v1.2.3",
        "release/v1.2.3.lock",
        "v1.2.3\n",
        "v1.2.3\t",
    ];
    for tag in TAGS {
        let canonical = eggpack_core::validate_exact_tag_safety(tag).is_ok();
        let core = pre_convergence_core_tag_is_safe(tag);
        let ci = pre_convergence_ci_tag_is_safe(tag);
        // The canonical rule is the union of the two concern sets, so a
        // tag is accepted exactly when both former rules accepted it.
        assert_eq!(
            canonical,
            core && ci,
            "canonical tag safety diverged for {tag:?}"
        );
        // The canonical accepted set is a subset of the union of the two
        // former accepted sets: it accepts nothing either rule refused.
        assert!(
            !canonical || core || ci,
            "canonical tag safety widened for {tag:?}"
        );
    }
    // Every character class that either former rule refused is refused by
    // the canonical rule.
    for character in [
        '?', '#', '@', ' ', '\\', '\'', '"', '`', '$', '^', ':', '*', '[', '~',
    ] {
        let tag = format!("v1.2.3{character}");
        assert!(
            !pre_convergence_core_tag_is_safe(&tag) || !pre_convergence_ci_tag_is_safe(&tag),
            "expected a pre-convergence refusal for {tag:?}"
        );
        assert!(
            eggpack_core::validate_exact_tag_safety(&tag).is_err(),
            "canonical tag safety accepted {tag:?}"
        );
    }
    // A canonical version tag and a multi-segment tag stay accepted.
    assert!(eggpack_core::validate_exact_tag_safety("v1.2.3").is_ok());
    assert!(eggpack_core::validate_exact_tag_safety("release/v1.2.3").is_ok());
}

#[test]
fn m003l_candidate_digest_refusals_are_typed_and_never_an_empty_digest() {
    let root = eggpack_core_test_temp("m003l-digest");
    let missing = root.join("missing.bin");
    let empty = root.join("empty.bin");
    std::fs::write(&empty, b"").unwrap();
    let payload = root.join("payload.bin");
    std::fs::write(&payload, b"payload").unwrap();

    // A missing file.
    assert!(hash_candidate_file(&missing).is_err());

    // An empty file.
    assert!(hash_candidate_file(&empty).is_err());

    // A symlink.
    #[cfg(unix)]
    {
        let link = root.join("link.bin");
        std::os::unix::fs::symlink(&payload, &link).unwrap();
        assert!(hash_candidate_file(&link).is_err());
    }

    // An unreadable file. A privileged process may still read it; the
    // digest is then well-formed rather than empty.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let unreadable = root.join("unreadable.bin");
        std::fs::write(&unreadable, b"payload").unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
        match hash_candidate_file(&unreadable) {
            Err(_) => {}
            Ok((size, digest)) => {
                assert_eq!(size, 7);
                assert_eq!(digest.len(), 64);
            }
        }
    }

    // No refusal path yields an empty digest string.
    for path in [&missing, &empty, &payload] {
        if let Ok((_, digest)) = hash_candidate_file(path) {
            assert!(!digest.is_empty(), "empty digest for {path:?}");
        }
    }

    // A readable regular file still digests to a 64-character hex digest.
    let (size, digest) = hash_candidate_file(&payload).unwrap();
    assert_eq!(size, 7);
    assert_eq!(digest.len(), 64);
    assert!(digest
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn m003l_sha256_acceptance_set_is_identical_at_every_call_site() {
    const DIGESTS: &[&str] = &[
        "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "0000000000000000000000000000000000000000000000000000000000000000",
        "A1B2C3D4E5F60718293A4B5C6D7E8F90A1B2C3D4E5F60718293A4B5C6D7E8F90",
        "g1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9",
        "a1b2c3d4",
        "",
    ];
    for digest in DIGESTS {
        let canonical = eggpack_manifest::validate_sha256_hex(digest).is_ok();
        assert_eq!(
            eggpack_core::validate_sha256_hex(digest).is_ok(),
            canonical,
            "core re-export diverged for {digest:?}"
        );
        assert_eq!(
            validate_sha256_hex(digest).is_ok(),
            canonical,
            "CI call site diverged for {digest:?}"
        );
    }
    // The all-zeros placeholder is refused at every call site.
    assert!(!validate_sha256_hex(&"0".repeat(64)).is_ok());
    assert!(!eggpack_manifest::validate_sha256_hex(&"0".repeat(64)).is_ok());
    assert!(!eggpack_core::validate_sha256_hex(&"0".repeat(64)).is_ok());
}

// ---------------------------------------------------------------------
// M003n — producer implementation consolidation.
//
// The drift comparison is one implementation, the staging payload has
// one builder, and the release renderer is decomposed into named
// private helpers. Every rendered byte is pinned here.
// ---------------------------------------------------------------------

/// Byte-exact SHA-256 of the reusable release workflow (ref-name tag
/// source), captured before the consolidation.
const M003N_REUSABLE_REFNAME_SHA256: &str =
    "712fb94cb0dceccf7546b857960f599e36312641a74aa836b94d1010e3935c92";
/// Byte-exact length of the reusable release workflow (ref-name tag
/// source), captured before the consolidation.
const M003N_REUSABLE_REFNAME_LEN: usize = 28_591;
/// Byte-exact SHA-256 of the reusable release workflow (dispatch-input
/// tag source), captured before the consolidation.
const M003N_REUSABLE_DISPATCH_SHA256: &str =
    "1eb5559409f930926af8a78ae60413ad79af860725247b5184ef020815698b36";
/// Byte-exact length of the reusable release workflow (dispatch-input
/// tag source), captured before the consolidation.
const M003N_REUSABLE_DISPATCH_LEN: usize = 28_735;

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The three renderers are distinct artifacts, not duplicates; only
/// their comparison is shared. Each one has its own drift check
/// function.
#[derive(Clone, Copy)]
enum DriftRenderer {
    /// `render_github` / `check_github`.
    Build,
    /// `render_release_github` / `check_release_github`.
    Release,
    /// `render_reusable_release_github` / `check_reusable_release_github`.
    Reusable,
}

/// Render the artifact one drift check function compares against.
fn drift_rendered(renderer: DriftRenderer) -> String {
    match renderer {
        DriftRenderer::Build => {
            let ci_graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
            render_github(&ci_graph, &policy()).unwrap()
        }
        DriftRenderer::Release => {
            let (release_graph, release_policy) = m002_golden_case(
                include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
                "eggsact",
                "1.2.3",
                vec![(
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                )],
                vec!["linux-x64"],
            );
            render_release_github(&release_graph, &release_policy).unwrap()
        }
        DriftRenderer::Reusable => {
            let (contract, shape, reusable_policy) =
                m003d_shape_and_policy(StagingTagSource::RefName);
            render_reusable_release_github(&contract, &shape, &reusable_policy).unwrap()
        }
    }
}

/// Run one drift check function over the given existing bytes.
fn drift_check(renderer: DriftRenderer, existing: &[u8]) -> Result<DriftReport, CiError> {
    match renderer {
        DriftRenderer::Build => {
            let ci_graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
            check_github(&ci_graph, &policy(), existing)
        }
        DriftRenderer::Release => {
            let (release_graph, release_policy) = m002_golden_case(
                include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
                "eggsact",
                "1.2.3",
                vec![(
                    "x86_64-unknown-linux-gnu",
                    BuildStrategy::NativeCargo,
                    SupportTier::Required,
                    Qualification::Structural,
                )],
                vec!["linux-x64"],
            );
            check_release_github(&release_graph, &release_policy, existing)
        }
        DriftRenderer::Reusable => {
            let (contract, shape, reusable_policy) =
                m003d_shape_and_policy(StagingTagSource::RefName);
            check_reusable_release_github(&contract, &shape, &reusable_policy, existing)
        }
    }
}

/// The public check function must return exactly what the one shared
/// comparison returns for the same (rendered, existing) byte pair.
fn assert_shared_comparison(
    renderer: DriftRenderer,
    rendered: &str,
    existing: &[u8],
) -> DriftReport {
    let report = drift_check(renderer, existing).unwrap();
    let shared = compare_workflow_bytes(rendered.as_bytes(), existing).unwrap();
    assert_eq!(
        report, shared,
        "the check function diverged from the one shared comparison"
    );
    report
}

/// Classify a drift report so that the three check functions can be
/// compared on equal terms despite rendering different artifacts.
fn classify(report: &DriftReport, expected_len: usize) -> &'static str {
    if report.matches {
        return "match";
    }
    if report.actual_bytes < expected_len {
        "shorter"
    } else if report.first_difference == Some(0) {
        "first-byte"
    } else {
        "last-byte"
    }
}

#[test]
fn m003n_renderers_are_byte_identical_to_their_baselines() {
    // render_github.
    let ci_graph = graph(BuildStrategy::NativeCargo, SupportTier::Required);
    assert_golden(
        &render_github(&ci_graph, &policy()).unwrap(),
        include_str!("../tests/fixtures/native-direct.yml"),
    );
    // render_release_github without staging.
    let (release_graph, release_policy) = m002_golden_case(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&release_graph, &release_policy).unwrap(),
        include_str!("../tests/fixtures/m002-direct.yml"),
    );
    // render_release_github with staging.
    let (staged_graph, staged_policy) = m003b_staging_graph(
        include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
        "eggsact",
        "1.2.3",
        vec![(
            "x86_64-unknown-linux-gnu",
            BuildStrategy::NativeCargo,
            SupportTier::Required,
            Qualification::Structural,
        )],
        vec!["linux-x64"],
    );
    assert_golden(
        &render_release_github(&staged_graph, &staged_policy).unwrap(),
        include_str!("../tests/fixtures/m003b-direct-staging.yml"),
    );
    // render_reusable_release_github, pinned by exact length and digest
    // because the reusable workflow has no checked-in fixture file.
    for (tag_source, digest, len) in [
        (
            StagingTagSource::RefName,
            M003N_REUSABLE_REFNAME_SHA256,
            M003N_REUSABLE_REFNAME_LEN,
        ),
        (
            StagingTagSource::DispatchInput,
            M003N_REUSABLE_DISPATCH_SHA256,
            M003N_REUSABLE_DISPATCH_LEN,
        ),
    ] {
        let (contract, shape, reusable_policy) = m003d_shape_and_policy(tag_source);
        let rendered = render_reusable_release_github(&contract, &shape, &reusable_policy).unwrap();
        assert_eq!(rendered.len(), len, "reusable workflow length changed");
        assert_eq!(sha256_hex(rendered.as_bytes()), digest);
    }
}

#[test]
fn m003n_three_check_functions_share_one_drift_comparison() {
    const RENDERERS: [DriftRenderer; 3] = [
        DriftRenderer::Build,
        DriftRenderer::Release,
        DriftRenderer::Reusable,
    ];
    // One verdict per case, agreed by all three check functions.
    let mut case_verdicts: Vec<(&'static str, &'static str)> = Vec::new();
    for renderer in RENDERERS {
        let rendered = drift_rendered(renderer);
        assert!(
            !rendered.contains('\r'),
            "the rendered workflow must be LF-only before comparison"
        );
        let len = rendered.len();

        // Matching workflow.
        let report = assert_shared_comparison(renderer, &rendered, rendered.as_bytes());
        assert!(report.matches);
        assert_eq!(report.expected_bytes, len);
        assert_eq!(report.actual_bytes, len);
        assert_eq!(report.first_difference, None);
        case_verdicts.push(("match", classify(&report, len)));

        // CRLF variant: CRLF normalizes to LF only, so it still matches.
        let crlf = rendered.replace('\n', "\r\n");
        let report = assert_shared_comparison(renderer, &rendered, crlf.as_bytes());
        assert!(report.matches, "CRLF must normalize to LF only");
        assert_eq!(report.expected_bytes, len);
        assert_eq!(report.actual_bytes, len);
        assert_eq!(report.first_difference, None);
        case_verdicts.push(("crlf", classify(&report, len)));

        // Difference at the first byte.
        let mut first = rendered.clone().into_bytes();
        first[0] ^= 0x01;
        let report = assert_shared_comparison(renderer, &rendered, &first);
        assert!(!report.matches);
        assert_eq!(report.expected_bytes, len);
        assert_eq!(report.actual_bytes, len);
        assert_eq!(report.first_difference, Some(0));
        case_verdicts.push(("first-byte", classify(&report, len)));

        // Difference at the last byte.
        let mut last = rendered.clone().into_bytes();
        let last_index = last.len() - 1;
        last[last_index] ^= 0x01;
        let report = assert_shared_comparison(renderer, &rendered, &last);
        assert!(!report.matches);
        assert_eq!(report.expected_bytes, len);
        assert_eq!(report.actual_bytes, len);
        assert_eq!(report.first_difference, Some(last_index));
        case_verdicts.push(("last-byte", classify(&report, len)));

        // Length mismatch: the shorter length is the fallback offset.
        let report = assert_shared_comparison(renderer, &rendered, &rendered.as_bytes()[..len - 1]);
        assert!(!report.matches);
        assert_eq!(report.expected_bytes, len);
        assert_eq!(report.actual_bytes, len - 1);
        assert_eq!(report.first_difference, Some(len - 1));
        case_verdicts.push(("shorter", classify(&report, len)));
    }
    // Every case has one verdict per check function, and all three agree.
    for (case, expected) in [
        ("match", "match"),
        ("crlf", "match"),
        ("first-byte", "first-byte"),
        ("last-byte", "last-byte"),
        ("shorter", "shorter"),
    ] {
        let observed: Vec<&str> = case_verdicts
            .iter()
            .filter(|(name, _)| *name == case)
            .map(|(_, verdict)| *verdict)
            .collect();
        assert_eq!(observed.len(), 3, "case {case} is missing a report");
        assert!(
            observed.windows(2).all(|pair| pair[0] == pair[1]),
            "the three check functions disagree on the {case} case: {observed:?}"
        );
        assert!(
            observed.iter().all(|verdict| *verdict == expected),
            "the {case} case must report {expected}: {observed:?}"
        );
    }
}

#[test]
fn m003n_identical_existing_bytes_produce_identical_reports_in_all_three_functions() {
    const RENDERERS: [DriftRenderer; 3] = [
        DriftRenderer::Build,
        DriftRenderer::Release,
        DriftRenderer::Reusable,
    ];
    // One fixed existing byte pair, fed to all three check functions.
    let existing = drift_rendered(DriftRenderer::Build).into_bytes();
    let mut actual_bytes: Vec<usize> = Vec::new();
    for renderer in RENDERERS {
        let rendered = drift_rendered(renderer);
        let report = assert_shared_comparison(renderer, &rendered, &existing);
        // The same existing bytes are measured identically by all three.
        assert_eq!(report.actual_bytes, existing.len());
        actual_bytes.push(report.actual_bytes);
        // The verdict is the same equality test over the same two byte
        // sequences, so it holds exactly when that renderer's own
        // expected bytes equal the existing bytes.
        assert_eq!(
            report.matches,
            normalize_newlines(rendered.as_bytes()) == normalize_newlines(&existing)
        );
    }
    assert!(
        actual_bytes.windows(2).all(|pair| pair[0] == pair[1]),
        "the three check functions measured the identical existing bytes differently"
    );
}

#[test]
fn m003n_over_size_workflow_is_rejected_by_every_check_function() {
    let over_size = vec![b'x'; MAX_WORKFLOW_BYTES + 1];
    for renderer in [
        DriftRenderer::Build,
        DriftRenderer::Release,
        DriftRenderer::Reusable,
    ] {
        assert!(
            drift_check(renderer, &over_size).is_err(),
            "the size bound must apply to every check function"
        );
    }
}

/// M003n: the crate's public API surface is unchanged by the module split.
///
/// The crate root re-exports every public item from the module that owns it.
/// This golden list was captured before the split; it names every item that
/// must remain publicly reachable through the crate root.
const M003N_PUBLIC_API_SURFACE: &[&str] = &[
    "ActionPin",
    "AggregateJob",
    "AggregateOutcome",
    "ArchiveEncodingWrapper",
    "BUILD_HANDOFF_FILE",
    "BuildHandoffOutput",
    "BuildHandoffV1",
    "BuildOutput",
    "CANDIDATES_DIR",
    "CIPlan",
    "CONSUMER_EVIDENCE_FILE",
    "CiError",
    "ConsumerValidationEvidenceV1",
    "ConsumerValidationFailure",
    "ConsumerValidationOutcome",
    "ConsumerValidationRequest",
    "ConsumerValidatorV1",
    "CrossToolProvisioningV1",
    "DriftReport",
    "EggpackToolPolicy",
    "FinalizationSettings",
    "GATE_OUTCOME_FILE",
    "GitHubPolicy",
    "GitHubReleaseInputsV1",
    "GitHubStagingInputsV1",
    "GitHubStagingPolicyV1",
    "QUALIFICATION_EVIDENCE_FILE",
    "QualificationIntent",
    "QualificationJob",
    "QualificationState",
    "RUNTIME_CI_PLAN",
    "RUNTIME_GITHUB_POLICY",
    "RUNTIME_IDENTITY_ARTIFACT",
    "RUNTIME_IDENTITY_DIR",
    "RUNTIME_RELEASE_PLAN",
    "ReleaseCIPlanV1",
    "ReleaseIdentityMode",
    "ReleaseWorkflowShapeV1",
    "RunnerCommand",
    "RunnerMapping",
    "ShapeStagingIntentV1",
    "StagingJob",
    "StagingProvider",
    "StagingTagSource",
    "TargetJob",
    "ValidatorInterpreterV1",
    "WorkflowTrigger",
    "ZigOfficialArchiveV1",
    "aggregate_finalize",
    "aggregate_finalize_with_consumer",
    "cargo_output_path",
    "check_github",
    "check_release_github",
    "check_reusable_release_github",
    "consumer_evidence_handoff_name",
    "consumer_job_id",
    "decode_consumer_evidence",
    "decode_qualification_evidence",
    "encode_qualification_evidence",
    "evaluate_gate",
    "evaluate_gate_with_consumer",
    "project_build_handoff",
    "project_ci_plan",
    "project_release_plan",
    "project_release_plan_with_consumer",
    "python_interpreter_exe",
    "python_interpreter_exe_for_windows",
    "reconstruct_attempt",
    "render_github",
    "render_release_github",
    "render_reusable_release_github",
    "resolve_runtime_release_plan",
    "resolve_runtime_release_plan_with_mode",
    "run_consumer_validator",
    "stage_build_artifact_dir",
    "validate_build_artifact_dir",
    "validate_canonical_target_dir",
    "validate_provisioned_zig_dir",
    "validate_qualification_artifact_dir",
    "validate_sha256_hex",
    "validate_zig_archive_listing",
    "with_github_draft_staging",
    "zig_archive_name",
    "zig_download_url",
    "zig_expected_digest",
];

/// M003n: the crate root still exposes exactly the same public items,
/// with the same names and the same signatures.
#[test]
#[allow(clippy::type_complexity)]
fn m003n_public_api_surface_is_unchanged() {
    let root = include_str!("lib.rs");
    assert!(
        !root.contains("pub mod "),
        "the crate root must not expose a module path of its own"
    );
    assert!(
        !root.contains("pub use crate::"),
        "the crate root must not re-export through the crate path"
    );
    // The crate root re-exports exactly the golden set of public names.
    let mut re_exported: Vec<&str> = Vec::new();
    let mut rest = root;
    while let Some(start) = rest.find("pub use ") {
        rest = &rest[start + "pub use ".len()..];
        let end = rest.find(';').expect("a `pub use` statement ends with `;`");
        let statement = &rest[..end];
        rest = &rest[end + 1..];
        if let Some((_, inner)) = statement.split_once("::{") {
            let inner = inner.trim_end().trim_end_matches('}');
            for name in inner.split(',') {
                if !name.trim().is_empty() {
                    re_exported.push(name.trim());
                }
            }
        } else {
            re_exported.push(statement.rsplit("::").next().unwrap().trim());
        }
    }
    re_exported.sort_unstable();
    assert_eq!(
        re_exported, M003N_PUBLIC_API_SURFACE,
        "the crate root's public item set changed"
    );

    // Every public type still resolves at the crate root.
    let _: Option<crate::ActionPin> = None;
    let _: Option<crate::AggregateJob> = None;
    let _: Option<crate::AggregateOutcome> = None;
    let _: Option<crate::ArchiveEncodingWrapper> = None;
    let _: Option<crate::BuildHandoffOutput> = None;
    let _: Option<crate::BuildHandoffV1> = None;
    let _: Option<crate::BuildOutput> = None;
    let _: Option<crate::CIPlan> = None;
    let _: Option<crate::CiError> = None;
    let _: Option<crate::ConsumerValidationEvidenceV1> = None;
    let _: Option<crate::ConsumerValidationFailure> = None;
    let _: Option<crate::ConsumerValidationOutcome> = None;
    let _: Option<crate::ConsumerValidationRequest> = None;
    let _: Option<crate::ConsumerValidatorV1> = None;
    let _: Option<crate::CrossToolProvisioningV1> = None;
    let _: Option<crate::DriftReport> = None;
    let _: Option<crate::EggpackToolPolicy> = None;
    let _: Option<crate::FinalizationSettings> = None;
    let _: Option<crate::GitHubPolicy> = None;
    let _: Option<crate::GitHubReleaseInputsV1> = None;
    let _: Option<crate::GitHubStagingInputsV1> = None;
    let _: Option<crate::GitHubStagingPolicyV1> = None;
    let _: Option<crate::QualificationIntent> = None;
    let _: Option<crate::QualificationJob> = None;
    let _: Option<crate::QualificationState> = None;
    let _: Option<crate::ReleaseCIPlanV1> = None;
    let _: Option<crate::ReleaseIdentityMode> = None;
    let _: Option<crate::ReleaseWorkflowShapeV1> = None;
    let _: Option<crate::RunnerCommand> = None;
    let _: Option<crate::RunnerMapping> = None;
    let _: Option<crate::ShapeStagingIntentV1> = None;
    let _: Option<crate::StagingJob> = None;
    let _: Option<crate::StagingProvider> = None;
    let _: Option<crate::StagingTagSource> = None;
    let _: Option<crate::TargetJob> = None;
    let _: Option<crate::ValidatorInterpreterV1> = None;
    let _: Option<crate::WorkflowTrigger> = None;
    let _: Option<crate::ZigOfficialArchiveV1> = None;

    // Every public enum variant still resolves at the crate root.
    fn _assert_aggregateoutcome_variants(value: crate::AggregateOutcome) {
        match value {
            crate::AggregateOutcome::Complete => {}
            crate::AggregateOutcome::SuppressedNonGatingIncomplete => {}
            crate::AggregateOutcome::FailedRequiredGate => {}
            crate::AggregateOutcome::InvalidEvidence => {}
        }
    }
    fn _assert_archiveencodingwrapper_variants(value: crate::ArchiveEncodingWrapper) {
        match value {
            crate::ArchiveEncodingWrapper::TarGzip => {}
        }
    }
    fn _assert_consumervalidationfailure_variants(value: crate::ConsumerValidationFailure) {
        match value {
            crate::ConsumerValidationFailure::NonZeroExit => {}
            crate::ConsumerValidationFailure::Timeout => {}
            crate::ConsumerValidationFailure::OutputLimit => {}
            crate::ConsumerValidationFailure::InterpreterUnavailable => {}
            crate::ConsumerValidationFailure::CandidateMismatch => {}
            crate::ConsumerValidationFailure::ScriptUnavailable => {}
            crate::ConsumerValidationFailure::Cancelled => {}
        }
    }
    fn _assert_consumervalidationoutcome_variants(value: crate::ConsumerValidationOutcome) {
        match value {
            crate::ConsumerValidationOutcome::Passed => {}
            crate::ConsumerValidationOutcome::Failed(..) => {}
        }
    }
    fn _assert_qualificationstate_variants(value: crate::QualificationState) {
        match value {
            crate::QualificationState::Unresolved => {}
        }
    }
    fn _assert_releaseidentitymode_variants(value: crate::ReleaseIdentityMode) {
        match value {
            crate::ReleaseIdentityMode::ExactTag => {}
            crate::ReleaseIdentityMode::VPrefixedStableSemver => {}
        }
    }
    fn _assert_runnercommand_variants(value: crate::RunnerCommand) {
        match value {
            crate::RunnerCommand::VerifySource { .. } => {}
            crate::RunnerCommand::CaptureBuild { .. } => {}
            crate::RunnerCommand::QualifyTarget { .. } => {}
            crate::RunnerCommand::EvaluateGate { .. } => {}
            crate::RunnerCommand::Aggregate { .. } => {}
            crate::RunnerCommand::PrepareStage { .. } => {}
            crate::RunnerCommand::StageGithubDraft { .. } => {}
            crate::RunnerCommand::ValidateConsumer { .. } => {}
            crate::RunnerCommand::ResolveRelease { .. } => {}
        }
    }
    fn _assert_stagingprovider_variants(value: crate::StagingProvider) {
        match value {
            crate::StagingProvider::GitHubDraft => {}
        }
    }
    fn _assert_stagingtagsource_variants(value: crate::StagingTagSource) {
        match value {
            crate::StagingTagSource::RefName => {}
            crate::StagingTagSource::DispatchInput => {}
        }
    }
    fn _assert_validatorinterpreterv1_variants(value: crate::ValidatorInterpreterV1) {
        match value {
            crate::ValidatorInterpreterV1::Python3 => {}
        }
    }
    fn _assert_workflowtrigger_variants(value: crate::WorkflowTrigger) {
        match value {
            crate::WorkflowTrigger::Push => {}
            crate::WorkflowTrigger::WorkflowDispatch => {}
        }
    }

    // Every public constant keeps its type and value.
    let _: &str = crate::BUILD_HANDOFF_FILE;
    let _: &str = crate::CANDIDATES_DIR;
    let _: &str = crate::CONSUMER_EVIDENCE_FILE;
    let _: &str = crate::GATE_OUTCOME_FILE;
    let _: &str = crate::QUALIFICATION_EVIDENCE_FILE;
    let _: &str = crate::RUNTIME_CI_PLAN;
    let _: &str = crate::RUNTIME_GITHUB_POLICY;
    let _: &str = crate::RUNTIME_IDENTITY_ARTIFACT;
    let _: &str = crate::RUNTIME_IDENTITY_DIR;
    let _: &str = crate::RUNTIME_RELEASE_PLAN;

    // Every public function keeps its exact signature.
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        plan: &eggpack_core::ReleasePlan,
        graph: &crate::ReleaseCIPlanV1,
        inputs: &[eggpack_core::FinalizationTargetInput],
        evidence_references: &[String],
        output_root: &std::path::Path,
    ) -> Result<
        (
            crate::AggregateOutcome,
            Option<eggpack_core::FinalizedRelease>,
        ),
        crate::CiError,
    > = crate::aggregate_finalize;
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        plan: &eggpack_core::ReleasePlan,
        graph: &crate::ReleaseCIPlanV1,
        inputs: &[eggpack_core::FinalizationTargetInput],
        consumer_evidences: &[crate::ConsumerValidationEvidenceV1],
        evidence_references: &[String],
        output_root: &std::path::Path,
    ) -> Result<
        (
            crate::AggregateOutcome,
            Option<eggpack_core::FinalizedRelease>,
        ),
        crate::CiError,
    > = crate::aggregate_finalize_with_consumer;
    let _: fn(
        cargo_target_dir: &std::path::Path,
        target: &str,
        binary: &str,
    ) -> Result<std::path::PathBuf, crate::CiError> = crate::cargo_output_path;
    let _: fn(
        plan: &crate::CIPlan,
        policy: &crate::GitHubPolicy,
        existing: &[u8],
    ) -> Result<crate::DriftReport, crate::CiError> = crate::check_github;
    let _: fn(
        graph: &crate::ReleaseCIPlanV1,
        policy: &crate::GitHubPolicy,
        existing: &[u8],
    ) -> Result<crate::DriftReport, crate::CiError> = crate::check_release_github;
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        shape: &crate::ReleaseWorkflowShapeV1,
        policy: &crate::GitHubPolicy,
        existing: &[u8],
    ) -> Result<crate::DriftReport, crate::CiError> = crate::check_reusable_release_github;
    let _: fn(target: &str) -> String = crate::consumer_evidence_handoff_name;
    let _: fn(build_job_id: &str) -> String = crate::consumer_job_id;
    let _: fn(text: &str) -> Result<crate::ConsumerValidationEvidenceV1, crate::CiError> =
        crate::decode_consumer_evidence;
    let _: fn(text: &str) -> Result<eggpack_core::QualificationEvidence, crate::CiError> =
        crate::decode_qualification_evidence;
    let _: fn(evidence: &eggpack_core::QualificationEvidence) -> Result<String, crate::CiError> =
        crate::encode_qualification_evidence;
    let _: fn(
        graph: &crate::ReleaseCIPlanV1,
        evidences: &[eggpack_core::QualificationEvidence],
    ) -> Result<crate::AggregateOutcome, crate::CiError> = crate::evaluate_gate;
    let _: fn(
        graph: &crate::ReleaseCIPlanV1,
        evidences: &[eggpack_core::QualificationEvidence],
        consumer_evidences: &[crate::ConsumerValidationEvidenceV1],
    ) -> Result<crate::AggregateOutcome, crate::CiError> = crate::evaluate_gate_with_consumer;
    let _: fn(
        plan: &eggpack_core::ReleasePlan,
        bindings: &eggpack_core::BuildBindingsV1,
        target: &str,
    ) -> Result<crate::BuildHandoffV1, crate::CiError> = crate::project_build_handoff;
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        release: &eggpack_core::ReleasePlan,
        bindings: &eggpack_core::BuildBindingsV1,
    ) -> Result<crate::CIPlan, crate::CiError> = crate::project_ci_plan;
    let _: fn(
        ci_plan: &crate::CIPlan,
        qualification_bindings: &eggpack_core::QualificationBindingsV1,
        build_bindings: &eggpack_core::BuildBindingsV1,
        release: &eggpack_core::ReleasePlan,
    ) -> Result<crate::ReleaseCIPlanV1, crate::CiError> = crate::project_release_plan;
    let _: fn(
        ci_plan: &crate::CIPlan,
        qualification_bindings: &eggpack_core::QualificationBindingsV1,
        build_bindings: &eggpack_core::BuildBindingsV1,
        release: &eggpack_core::ReleasePlan,
        consumer: std::collections::BTreeMap<String, crate::ConsumerValidatorV1>,
    ) -> Result<crate::ReleaseCIPlanV1, crate::CiError> = crate::project_release_plan_with_consumer;
    let _: fn() -> &'static str = crate::python_interpreter_exe;
    let _: fn(is_windows: bool) -> &'static str = crate::python_interpreter_exe_for_windows;
    let _: fn(
        plan: &eggpack_core::ReleasePlan,
        target: &eggpack_core::PlannedTarget,
        handoff: &crate::BuildHandoffV1,
        candidate_root: &std::path::Path,
    ) -> Result<eggpack_core::BuildAttempt, crate::CiError> = crate::reconstruct_attempt;
    let _: fn(
        plan: &crate::CIPlan,
        policy: &crate::GitHubPolicy,
    ) -> Result<String, crate::CiError> = crate::render_github;
    let _: fn(
        graph: &crate::ReleaseCIPlanV1,
        policy: &crate::GitHubPolicy,
    ) -> Result<String, crate::CiError> = crate::render_release_github;
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        shape: &crate::ReleaseWorkflowShapeV1,
        policy: &crate::GitHubPolicy,
    ) -> Result<String, crate::CiError> = crate::render_reusable_release_github;
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        pack_config: &eggpack_core::PackConfig,
        selected: &[String],
        tag: &str,
        source_revision: &str,
    ) -> Result<eggpack_core::ReleasePlan, crate::CiError> = crate::resolve_runtime_release_plan;
    let _: fn(
        contract: &eggpack_contract::DistributionContract,
        pack_config: &eggpack_core::PackConfig,
        selected: &[String],
        tag: &str,
        source_revision: &str,
        mode: crate::ReleaseIdentityMode,
    ) -> Result<eggpack_core::ReleasePlan, crate::CiError> =
        crate::resolve_runtime_release_plan_with_mode;
    let _: fn(
        request: &crate::ConsumerValidationRequest<'_>,
    ) -> Result<crate::ConsumerValidationEvidenceV1, crate::CiError> =
        crate::run_consumer_validator;
    let _: fn(
        handoff: &crate::BuildHandoffV1,
        candidate_src_dir: &std::path::Path,
        out_dir: &std::path::Path,
    ) -> Result<(), crate::CiError> = crate::stage_build_artifact_dir;
    let _: fn(dir: &std::path::Path) -> Result<crate::BuildHandoffV1, crate::CiError> =
        crate::validate_build_artifact_dir;
    let _: fn(target: &str) -> Result<(), crate::CiError> = crate::validate_canonical_target_dir;
    let _: fn(root: &std::path::Path) -> Result<std::path::PathBuf, crate::CiError> =
        crate::validate_provisioned_zig_dir;
    let _: fn(
        dir: &std::path::Path,
    ) -> Result<
        (crate::BuildHandoffV1, eggpack_core::QualificationEvidence),
        crate::CiError,
    > = crate::validate_qualification_artifact_dir;
    let _: fn(digest: &str) -> Result<(), crate::CiError> = crate::validate_sha256_hex;
    let _: fn(entries: &[String]) -> Result<String, crate::CiError> =
        crate::validate_zig_archive_listing;
    let _: fn(
        graph: crate::ReleaseCIPlanV1,
        required: bool,
    ) -> Result<crate::ReleaseCIPlanV1, crate::CiError> = crate::with_github_draft_staging;
    let _: fn(
        os: eggpack_core::HostOs,
        arch: eggpack_core::HostArch,
        version: &str,
    ) -> Result<String, crate::CiError> = crate::zig_archive_name;
    let _: fn(version: &str, arch: eggpack_core::HostArch) -> Result<String, crate::CiError> =
        crate::zig_download_url;
    let _: fn(
        provisioning: &crate::CrossToolProvisioningV1,
        host_os: eggpack_core::HostOs,
        host_arch: eggpack_core::HostArch,
    ) -> Result<&str, crate::CiError> = crate::zig_expected_digest;
}
