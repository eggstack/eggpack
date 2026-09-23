use eggpack_contract::{
    expected_release_files, validate_archive_member_inventory, validate_observed_mapping,
    validate_release_inventory, ArchiveMemberInventory, ConformanceReport, DistributionContract,
    ExpandedAssets, ExtrasPolicy, FindingKind, ObservedArchiveMapping, ObservedDirectMapping,
    ObservedTargetAssets, ObservedTargetMapping, ReleaseInventory, MAX_OBSERVED_ENTRIES,
};

fn load(name: &str) -> DistributionContract {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).expect("fixture readable");
    DistributionContract::parse_toml_str(&text).expect("fixture valid")
}

fn load_observation(name: &str) -> ObservedTargetMapping {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).expect("observation fixture readable");
    toml::from_str(&text).expect("observation fixture valid")
}

fn names(files: &[eggpack_contract::ExpectedReleaseFile]) -> Vec<String> {
    files.iter().map(|file| file.file_name.clone()).collect()
}

fn report_kinds(report: &ConformanceReport) -> Vec<FindingKind> {
    report.findings.iter().map(|finding| finding.kind).collect()
}

#[test]
fn direct_release_inventory_accepts_required_files_and_default_extras() {
    assert_eq!(ExtrasPolicy::default(), ExtrasPolicy::AllowExtras);
    let contract = load("simple-direct.toml");
    let expected = expected_release_files(&contract, "linux-x64", "1.2.6").unwrap();
    assert_eq!(expected[0].label, "asset");
    assert_eq!(expected[1].label, "sidecar");
    let mut observed = names(&expected);
    observed.push("source.tar.gz".to_string());
    let inventory = ReleaseInventory::new(observed).unwrap();
    let report = validate_release_inventory(&expected, &inventory, ExtrasPolicy::AllowExtras);
    assert!(report.is_conformant());
}

#[test]
fn golden_observation_fixtures_conform_for_all_layouts() {
    let cases = [
        ("simple-direct.toml", "observed-simple.toml", "1.2.6"),
        ("codegg-bundle.toml", "observed-codegg.toml", "0.9.0"),
        ("egress-archive.toml", "observed-egress.toml", "2.1.0"),
    ];
    for (contract_name, observation_name, version) in cases {
        let contract = load(contract_name);
        let observation = load_observation(observation_name);
        let report = validate_observed_mapping(&contract, version, &observation);
        assert!(report.is_conformant(), "{observation_name}: {report:?}");
    }
}

#[test]
fn release_validation_reports_missing_files_and_exact_extras() {
    let contract = load("simple-direct.toml");
    let expected = expected_release_files(&contract, "linux-x64", "1.2.6").unwrap();
    let inventory = ReleaseInventory::new([expected[0].file_name.as_str(), "README.txt"]).unwrap();
    let report = validate_release_inventory(&expected, &inventory, ExtrasPolicy::Exact);
    assert_eq!(
        report_kinds(&report),
        [
            FindingKind::MissingReleaseFile,
            FindingKind::UnexpectedReleaseFile
        ]
    );
    assert_eq!(report.findings[0].label, "sidecar");
}

#[test]
fn bundle_release_inventory_requires_each_asset_and_sidecar() {
    let contract = load("codegg-bundle.toml");
    let expected = expected_release_files(&contract, "x86_64-unknown-linux-gnu", "0.9.0").unwrap();
    assert_eq!(expected.len(), 6);
    assert_eq!(expected[0].label, "entries[0].asset");
    let complete = ReleaseInventory::new(names(&expected)).unwrap();
    assert!(validate_release_inventory(&expected, &complete, ExtrasPolicy::Exact).is_conformant());
    let incomplete = ReleaseInventory::new(names(&expected)[1..].to_vec()).unwrap();
    let report = validate_release_inventory(&expected, &incomplete, ExtrasPolicy::AllowExtras);
    assert_eq!(report.findings[0].kind, FindingKind::MissingReleaseFile);
    assert_eq!(report.findings[0].label, "entries[0].asset");
}

#[test]
fn archive_release_and_member_inventories_validate_without_opening_archive() {
    let contract = load("egress-archive.toml");
    let expected_files = expected_release_files(&contract, "macos-arm64", "2.1.0").unwrap();
    assert_eq!(expected_files.len(), 2);
    let release = ReleaseInventory::new(names(&expected_files)).unwrap();
    assert!(
        validate_release_inventory(&expected_files, &release, ExtrasPolicy::Exact).is_conformant()
    );

    let expanded = contract.expand("macos-arm64", "2.1.0").unwrap();
    let ExpandedAssets::Archive(archive) = &expanded.assets else {
        panic!("fixture should be archive");
    };
    let inventory =
        ArchiveMemberInventory::new(archive.members.iter().map(|member| member.source.clone()))
            .unwrap();
    assert!(
        validate_archive_member_inventory(&expanded, &inventory, ExtrasPolicy::Exact)
            .unwrap()
            .is_conformant()
    );

    let incomplete = ArchiveMemberInventory::new(["egress"]).unwrap();
    let report =
        validate_archive_member_inventory(&expanded, &incomplete, ExtrasPolicy::AllowExtras)
            .unwrap();
    assert_eq!(report.findings[0].kind, FindingKind::MissingArchiveMember);
    assert_eq!(report.findings[0].label, "bin/egress-helper");

    let with_extra =
        ArchiveMemberInventory::new(["egress", "bin/egress-helper", "doc/readme"]).unwrap();
    assert!(
        validate_archive_member_inventory(&expanded, &with_extra, ExtrasPolicy::AllowExtras)
            .unwrap()
            .is_conformant()
    );
    assert_eq!(
        validate_archive_member_inventory(&expanded, &with_extra, ExtrasPolicy::Exact)
            .unwrap()
            .findings[0]
            .kind,
        FindingKind::UnexpectedArchiveMember
    );
}

#[test]
fn inventories_reject_bad_names_duplicates_collisions_and_large_inputs() {
    assert!(ReleaseInventory::new(["asset", "asset"]).is_err());
    assert!(ReleaseInventory::new(["asset", "ASSET"]).is_err());
    assert!(ReleaseInventory::new(["../asset"]).is_err());
    assert!(ReleaseInventory::new(["dir/asset"]).is_err());
    assert!(ReleaseInventory::new(["bad\nname"]).is_err());
    assert!(ReleaseInventory::new(["x".repeat(257)]).is_err());
    let too_many = (0..=MAX_OBSERVED_ENTRIES).map(|n| format!("asset-{n}"));
    assert!(ReleaseInventory::new(too_many).is_err());

    assert!(ArchiveMemberInventory::new(["bin/tool", "bin/tool"]).is_err());
    assert!(ArchiveMemberInventory::new(["bin/tool", "BIN/TOOL"]).is_err());
    for path in [
        "../tool",
        "/tool",
        "C:/tool",
        "bin\\tool",
        "bin//tool",
        "bin/./tool",
    ] {
        assert!(ArchiveMemberInventory::new([path]).is_err(), "{path}");
    }
    let nested = ArchiveMemberInventory::new(["bin/tool", "share/app/config.toml"]).unwrap();
    assert_eq!(nested.members()[0], "bin/tool");
}

#[test]
fn direct_mapping_accepts_alias_and_reports_exact_mapping_drift() {
    let contract = load("simple-direct.toml");
    let expanded = contract.expand("linux-x64", "1.2.6").unwrap();
    let ExpandedAssets::Direct(direct) = expanded.assets else {
        panic!("fixture should be direct");
    };
    let mut observation = ObservedTargetMapping {
        target: "linux-x64".to_string(),
        canonical_target: expanded.triple,
        assets: ObservedTargetAssets::Direct(ObservedDirectMapping {
            asset_file: direct.asset_file,
            sidecar_file: direct.sidecar_file,
            install_name: direct.install_name,
        }),
    };
    let encoded = toml::to_string(&observation).unwrap();
    let decoded: ObservedTargetMapping = toml::from_str(&encoded).unwrap();
    assert_eq!(observation, decoded);
    assert!(validate_observed_mapping(&contract, "1.2.6", &observation).is_conformant());

    observation.canonical_target = "aarch64-unknown-linux-gnu".to_string();
    let report = validate_observed_mapping(&contract, "1.2.6", &observation);
    assert!(report
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::TargetMismatch));
}

#[test]
fn direct_mapping_reports_asset_sidecar_and_install_name_drift() {
    let contract = load("simple-direct.toml");
    let observation = ObservedTargetMapping {
        target: "x86_64-unknown-linux-gnu".to_string(),
        canonical_target: "x86_64-unknown-linux-gnu".to_string(),
        assets: ObservedTargetAssets::Direct(ObservedDirectMapping {
            asset_file: "wrong-asset".to_string(),
            sidecar_file: "wrong-sidecar".to_string(),
            install_name: "wrong-install".to_string(),
        }),
    };
    let report = validate_observed_mapping(&contract, "1.2.6", &observation);
    assert_eq!(
        report_kinds(&report),
        [
            FindingKind::AssetMismatch,
            FindingKind::SidecarMismatch,
            FindingKind::InstallNameMismatch
        ]
    );
}

#[test]
fn bundle_and_archive_mapping_conformance_cover_every_member() {
    let bundle_contract = load("codegg-bundle.toml");
    let bundle = bundle_contract
        .expand("x86_64-unknown-linux-gnu", "0.9.0")
        .unwrap();
    let ExpandedAssets::Bundle(bundle_assets) = bundle.assets else {
        panic!("fixture should be bundle");
    };
    let observation = ObservedTargetMapping {
        target: bundle.triple.clone(),
        canonical_target: bundle.triple,
        assets: ObservedTargetAssets::Bundle {
            entries: bundle_assets
                .entries
                .into_iter()
                .map(|entry| ObservedDirectMapping {
                    asset_file: entry.asset_file,
                    sidecar_file: entry.sidecar_file,
                    install_name: entry.install_name,
                })
                .collect(),
        },
    };
    assert!(validate_observed_mapping(&bundle_contract, "0.9.0", &observation).is_conformant());

    let archive_contract = load("egress-archive.toml");
    let archive = archive_contract.expand("macos-arm64", "2.1.0").unwrap();
    let ExpandedAssets::Archive(archive_assets) = archive.assets else {
        panic!("fixture should be archive");
    };
    let mut observation = ObservedTargetMapping {
        target: "macos-arm64".to_string(),
        canonical_target: archive.triple,
        assets: ObservedTargetAssets::Archive {
            archive_file: archive_assets.archive_file,
            sidecar_file: archive_assets.sidecar_file,
            members: archive_assets
                .members
                .into_iter()
                .map(|member| ObservedArchiveMapping {
                    source: member.source,
                    install_name: member.install_name,
                })
                .collect(),
        },
    };
    assert!(validate_observed_mapping(&archive_contract, "2.1.0", &observation).is_conformant());
    let ObservedTargetAssets::Archive { members, .. } = &mut observation.assets else {
        unreachable!();
    };
    members[0].install_name = "wrong-name".to_string();
    assert_eq!(
        validate_observed_mapping(&archive_contract, "2.1.0", &observation).findings[0].kind,
        FindingKind::ArchiveMappingMismatch
    );
}

#[test]
fn observations_reject_oversized_or_unsafe_inputs_and_sort_findings() {
    let contract = load("simple-direct.toml");
    let oversized = ObservedTargetMapping {
        target: "x86_64-unknown-linux-gnu".to_string(),
        canonical_target: "x86_64-unknown-linux-gnu".to_string(),
        assets: ObservedTargetAssets::Bundle {
            entries: (0..MAX_OBSERVED_ENTRIES)
                .map(|_| ObservedDirectMapping {
                    asset_file: "asset".to_string(),
                    sidecar_file: "sidecar".to_string(),
                    install_name: "install".to_string(),
                })
                .collect(),
        },
    };
    assert_eq!(
        validate_observed_mapping(&contract, "1.0.0", &oversized).findings[0].kind,
        FindingKind::InvalidObservation
    );
    let unsafe_observation = ObservedTargetMapping {
        target: "../linux".to_string(),
        canonical_target: "x86_64-unknown-linux-gnu".to_string(),
        assets: ObservedTargetAssets::Direct(ObservedDirectMapping {
            asset_file: "asset".to_string(),
            sidecar_file: "sidecar".to_string(),
            install_name: "install".to_string(),
        }),
    };
    assert_eq!(
        validate_observed_mapping(&contract, "1.0.0", &unsafe_observation).findings[0].kind,
        FindingKind::TargetMismatch
    );

    let expected = expected_release_files(&contract, "linux-x64", "1.2.6").unwrap();
    let empty = ReleaseInventory::new(Vec::<String>::new()).unwrap();
    let report = validate_release_inventory(&expected, &empty, ExtrasPolicy::Exact);
    let mut sorted = report.findings.clone();
    sorted.sort();
    assert_eq!(report.findings, sorted);
}
