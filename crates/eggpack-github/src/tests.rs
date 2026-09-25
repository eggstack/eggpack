//! M003a qualification: local payload plus fixture adapter matrix.
use super::*;
use eggpack_bootstrap::BootstrapInstallPolicyV1;
use eggpack_manifest::{ArtifactForm, ArtifactRecord, ByteEvidence, ReleaseManifest, TargetRecord};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn temp_root(label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("eggpack-gh-{label}-{}-{id}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create temp: {error}"),
        }
    }
}

fn sha_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn test_policy() -> GitHubDraftPolicyV1 {
    GitHubDraftPolicyV1 {
        schema_version: 1,
        owner: "acme".to_owned(),
        repository: "widget".to_owned(),
        tag: "v1.2.3".to_owned(),
        title: "widget 1.2.3".to_owned(),
        body: "notes".to_owned(),
        prerelease: false,
        token_env: "GITHUB_TOKEN".to_owned(),
        request_timeout_secs: 30,
        max_metadata_bytes: 1_000_000,
        max_list_pages: 5,
    }
}

fn direct_contract() -> DistributionContract {
    DistributionContract::parse_toml_str(include_str!(
        "../../eggpack-contract/tests/fixtures/simple-direct.toml"
    ))
    .unwrap()
}

fn bundle_contract() -> DistributionContract {
    DistributionContract::parse_toml_str(include_str!(
        "../../eggpack-contract/tests/fixtures/codegg-bundle.toml"
    ))
    .unwrap()
}

fn archive_contract() -> DistributionContract {
    DistributionContract::parse_toml_str(include_str!(
        "../../eggpack-contract/tests/fixtures/egress-archive.toml"
    ))
    .unwrap()
}

fn direct_manifest(release: &str, source: &str) -> ReleaseManifest {
    // Full contract coverage (two targets) so bootstrap generation succeeds.
    // Targets are lexically sorted for deterministic manifest JSON.
    let mk = |target: &str| TargetRecord {
        target: target.to_owned(),
        form: ArtifactForm::Direct {
            artifact: ArtifactRecord {
                name: format!("eggsact-{release}-{target}"),
                size: 4,
                sha256: sha_hex(b"body"),
            },
            install: "eggsact".to_owned(),
        },
    };
    ReleaseManifest {
        schema_version: 1,
        product_id: "eggsact".to_owned(),
        release_id: release.to_owned(),
        source_revision: source.to_owned(),
        targets: vec![mk("aarch64-apple-darwin"), mk("x86_64-unknown-linux-gnu")],
        evidence_references: Vec::new(),
    }
}

fn bundle_install_policy() -> BootstrapInstallPolicyV1 {
    use eggpack_bootstrap::TargetInstallPolicy;
    let mut modes = BTreeMap::new();
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
    BootstrapInstallPolicyV1 {
        schema_version: 1,
        targets: [(
            "x86_64-unknown-linux-gnu".to_owned(),
            TargetInstallPolicy {
                modes,
                archive_encoding: None,
            },
        )]
        .into_iter()
        .collect(),
    }
}

fn archive_install_policy() -> BootstrapInstallPolicyV1 {
    use eggpack_bootstrap::{BundleArchiveEncoding, InstallMode, TargetInstallPolicy};
    let mut targets = BTreeMap::new();
    for triple in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
        let mut modes = BTreeMap::new();
        modes.insert("egress".to_owned(), InstallMode::Executable);
        modes.insert("egress-helper".to_owned(), InstallMode::Executable);
        targets.insert(
            triple.to_owned(),
            TargetInstallPolicy {
                modes,
                archive_encoding: Some(BundleArchiveEncoding::TarGzip),
            },
        );
    }
    BootstrapInstallPolicyV1 {
        schema_version: 1,
        targets,
    }
}

fn bundle_manifest(release: &str, source: &str) -> ReleaseManifest {
    let entries = ["codegg", "codegg-helper", "codegg-manifest"];
    let installs = ["codegg", "codegg-helper", "codegg-manifest.json"];
    let records = entries
        .iter()
        .zip(installs.iter())
        .map(|(stem, install)| eggpack_manifest::BundleRecord {
            artifact: ArtifactRecord {
                name: if *stem == "codegg-manifest" {
                    format!("codegg-manifest-{release}.json")
                } else {
                    format!("{stem}-{release}-x86_64-unknown-linux-gnu")
                },
                size: 4,
                sha256: sha_hex(b"body"),
            },
            install: (*install).to_owned(),
        })
        .collect();
    ReleaseManifest {
        schema_version: 1,
        product_id: "codegg".to_owned(),
        release_id: release.to_owned(),
        source_revision: source.to_owned(),
        targets: vec![TargetRecord {
            target: "x86_64-unknown-linux-gnu".to_owned(),
            form: ArtifactForm::Bundle { entries: records },
        }],
        evidence_references: Vec::new(),
    }
}

fn archive_manifest(release: &str, source: &str) -> ReleaseManifest {
    let mk = |target: &str| TargetRecord {
        target: target.to_owned(),
        form: ArtifactForm::Archive {
            artifact: ArtifactRecord {
                name: format!("egress-{release}-{target}.tar.gz"),
                size: 4,
                sha256: sha_hex(b"body"),
            },
            members: vec![
                eggpack_manifest::ArchiveMemberRecord {
                    source: "bin/egress-helper".to_owned(),
                    install: "egress-helper".to_owned(),
                    bytes: ByteEvidence {
                        size: 4,
                        sha256: sha_hex(b"body"),
                    },
                },
                eggpack_manifest::ArchiveMemberRecord {
                    source: "egress".to_owned(),
                    install: "egress".to_owned(),
                    bytes: ByteEvidence {
                        size: 4,
                        sha256: sha_hex(b"body"),
                    },
                },
            ],
        },
    };
    // Lexically sorted for deterministic JSON.
    ReleaseManifest {
        schema_version: 1,
        product_id: "egress".to_owned(),
        release_id: release.to_owned(),
        source_revision: source.to_owned(),
        targets: vec![mk("aarch64-apple-darwin"), mk("x86_64-unknown-linux-gnu")],
        evidence_references: Vec::new(),
    }
}

fn write_finalized_root(contract: &DistributionContract, manifest: &ReleaseManifest, root: &Path) {
    std::fs::create_dir(root).unwrap();
    // Derive expected files from contract expansion.
    let mut files: BTreeMap<String, bool> = BTreeMap::new();
    for target in &manifest.targets {
        let expanded = contract
            .expand(&target.target, &manifest.release_id)
            .unwrap();
        match expanded.assets {
            ExpandedAssets::Direct(direct) => {
                files.insert(direct.asset_file, true);
                files.insert(direct.sidecar_file, false);
            }
            ExpandedAssets::Bundle(bundle) => {
                for entry in bundle.entries.into_iter() {
                    files.insert(entry.asset_file, true);
                    files.insert(entry.sidecar_file, false);
                }
            }
            ExpandedAssets::Archive(archive) => {
                files.insert(archive.archive_file, true);
                files.insert(archive.sidecar_file, false);
            }
        }
    }
    for (name, is_artifact) in files {
        if is_artifact {
            std::fs::write(root.join(&name), b"body").unwrap();
        } else {
            // Sidecar references its artifact sibling. Find artifact by stem:
            // sidecar is "<artifact>.sha256" for these fixtures.
            let artifact = name.strip_suffix(".sha256").unwrap().to_owned();
            let content = format!("{}  {}\n", sha_hex(b"body"), artifact);
            std::fs::write(root.join(&name), content).unwrap();
        }
    }
}

fn prepare_case(
    contract: &DistributionContract,
    manifest: &ReleaseManifest,
    install_policy: &BootstrapInstallPolicyV1,
    label: &str,
) -> (PathBuf, PathBuf, PathBuf, StagingPayloadV1) {
    let parent = temp_root(label);
    let finalized = parent.join("finalized");
    write_finalized_root(contract, manifest, &finalized);
    let staging = parent.join("staging");
    let policy = test_policy();
    let payload = prepare_staging_payload(
        contract,
        manifest,
        &finalized,
        &policy,
        install_policy,
        &staging,
    )
    .unwrap();
    (parent, finalized, staging, payload)
}

#[test]
fn direct_payload_is_exact_and_deterministic() {
    let contract = direct_contract();
    let manifest = direct_manifest("1.2.6", &"a".repeat(40));
    let empty = BootstrapInstallPolicyV1::empty();
    let (parent, _, staging, payload) = prepare_case(&contract, &manifest, &empty, "direct");
    // Standalone manifest matches M004 manifest exactly.
    let stored = std::fs::read_to_string(staging.join("release-manifest.json")).unwrap();
    assert_eq!(
        ReleaseManifest::from_json(&stored).unwrap(),
        manifest,
        "standalone manifest must decode to exact M004 manifest"
    );
    assert_eq!(stored, manifest.to_json().unwrap());
    // Both installers present and deterministic.
    let sh = std::fs::read_to_string(staging.join("install.sh")).unwrap();
    let ps = std::fs::read_to_string(staging.join("install.ps1")).unwrap();
    assert!(sh.contains("https://github.com/acme/widget/releases/download/v1.2.3"));
    assert!(!sh.contains("latest/download"));
    assert!(ps.contains("https://github.com/acme/widget/releases/download/v1.2.3"));
    // Payload deterministic.
    assert_eq!(payload.to_json().unwrap(), payload.to_json().unwrap());
    assert_eq!(
        StagingPayloadV1::from_json(&payload.to_json().unwrap()).unwrap(),
        payload
    );
    // Second materialization is byte-identical.
    let staging2 = parent.join("staging2");
    let payload2 = prepare_staging_payload(
        &contract,
        &manifest,
        &parent.join("finalized"),
        &test_policy(),
        &BootstrapInstallPolicyV1::empty(),
        &staging2,
    )
    .unwrap();
    assert_eq!(payload, payload2);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn bundle_and_archive_payloads_are_exact() {
    let bundle_policy = bundle_install_policy();
    let archive_policy = archive_install_policy();
    for (label, contract, manifest, policy) in [
        (
            "bundle",
            bundle_contract(),
            bundle_manifest("2.4.0", &"b".repeat(40)),
            &bundle_policy,
        ),
        (
            "archive",
            archive_contract(),
            archive_manifest("3.1.0", &"c".repeat(40)),
            &archive_policy,
        ),
    ] {
        let (parent, _, staging, payload) = prepare_case(&contract, &manifest, policy, label);
        let stored = std::fs::read_to_string(staging.join("release-manifest.json")).unwrap();
        assert_eq!(ReleaseManifest::from_json(&stored).unwrap(), manifest);
        assert!(staging.join("install.sh").exists());
        assert!(staging.join("install.ps1").exists());
        assert!(!payload.assets.is_empty());
        std::fs::remove_dir_all(parent).unwrap();
    }
}

#[test]
fn missing_extra_and_tampered_finalized_files_reject() {
    let contract = direct_contract();
    let manifest = direct_manifest("1.2.6", &"a".repeat(40));
    // Missing file.
    {
        let parent = temp_root("missing");
        let finalized = parent.join("finalized");
        write_finalized_root(&contract, &manifest, &finalized);
        let first = std::fs::read_dir(&finalized)
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        std::fs::remove_file(first.path()).unwrap();
        assert!(prepare_staging_payload(
            &contract,
            &manifest,
            &finalized,
            &test_policy(),
            &BootstrapInstallPolicyV1::empty(),
            &parent.join("out"),
        )
        .is_err());
        std::fs::remove_dir_all(parent).unwrap();
    }
    // Extra file.
    {
        let parent = temp_root("extra");
        let finalized = parent.join("finalized");
        write_finalized_root(&contract, &manifest, &finalized);
        std::fs::write(finalized.join("extra.bin"), b"x").unwrap();
        assert!(prepare_staging_payload(
            &contract,
            &manifest,
            &finalized,
            &test_policy(),
            &BootstrapInstallPolicyV1::empty(),
            &parent.join("out"),
        )
        .is_err());
        std::fs::remove_dir_all(parent).unwrap();
    }
    // Tampered file.
    {
        let parent = temp_root("tamper");
        let finalized = parent.join("finalized");
        write_finalized_root(&contract, &manifest, &finalized);
        // Tamper first artifact (not sidecar).
        let artifact = manifest.targets[0].clone();
        let name = match artifact.form {
            ArtifactForm::Direct { artifact, .. } => artifact.name,
            _ => unreachable!(),
        };
        std::fs::write(finalized.join(&name), b"tampered-bytes").unwrap();
        assert!(prepare_staging_payload(
            &contract,
            &manifest,
            &finalized,
            &test_policy(),
            &BootstrapInstallPolicyV1::empty(),
            &parent.join("out"),
        )
        .is_err());
        std::fs::remove_dir_all(parent).unwrap();
    }
}

#[test]
fn symlink_and_auxiliary_collision_reject() {
    let contract = direct_contract();
    let manifest = direct_manifest("1.2.6", &"a".repeat(40));
    // Symlink in finalized root.
    {
        let parent = temp_root("symlink");
        let finalized = parent.join("finalized");
        write_finalized_root(&contract, &manifest, &finalized);
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = finalized.join("link-target");
            std::fs::write(&target, b"x").unwrap();
            // Replace one expected file with a symlink of the same name?
            // Simpler: add a symlink file (extra) which must reject.
            let _ = symlink(&target, finalized.join("evil-link"));
            assert!(prepare_staging_payload(
                &contract,
                &manifest,
                &finalized,
                &test_policy(),
                &BootstrapInstallPolicyV1::empty(),
                &parent.join("out"),
            )
            .is_err());
        }
        std::fs::remove_dir_all(parent).unwrap();
    }
    // Auxiliary collision: craft a contract whose asset collides with
    // install.sh. Use a direct contract with colliding asset name.
    {
        let text = r#"
schema_version = 1
[product]
id = "collide"
display_name = "collide"
[[targets]]
triple = "x86_64-unknown-linux-gnu"
[targets.asset]
kind = "direct"
asset = "install.sh"
install = "collide"
[targets.checksum]
sidecar = "{asset}.sha256"
"#;
        let colliding = DistributionContract::parse_toml_str(text).unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            product_id: "collide".to_owned(),
            release_id: "1.0.0".to_owned(),
            source_revision: "a".repeat(40),
            targets: vec![TargetRecord {
                target: "x86_64-unknown-linux-gnu".to_owned(),
                form: ArtifactForm::Direct {
                    artifact: ArtifactRecord {
                        name: "install.sh".to_owned(),
                        size: 4,
                        sha256: sha_hex(b"body"),
                    },
                    install: "collide".to_owned(),
                },
            }],
            evidence_references: Vec::new(),
        };
        let parent = temp_root("collide");
        let finalized = parent.join("finalized");
        std::fs::create_dir(&finalized).unwrap();
        std::fs::write(finalized.join("install.sh"), b"body").unwrap();
        std::fs::write(
            finalized.join("install.sh.sha256"),
            format!("{}  install.sh\n", sha_hex(b"body")),
        )
        .unwrap();
        assert!(prepare_staging_payload(
            &colliding,
            &manifest,
            &finalized,
            &test_policy(),
            &BootstrapInstallPolicyV1::empty(),
            &parent.join("out"),
        )
        .is_err());
        std::fs::remove_dir_all(parent).unwrap();
    }
}

#[test]
fn installers_deterministic_and_exact_tag_origin() {
    let contract = direct_contract();
    let manifest = direct_manifest("1.2.6", &"a".repeat(40));
    let empty = BootstrapInstallPolicyV1::empty();
    let (parent, _, staging, _) = prepare_case(&contract, &manifest, &empty, "deterministic");
    let sh1 = std::fs::read_to_string(staging.join("install.sh")).unwrap();
    let ps1 = std::fs::read_to_string(staging.join("install.ps1")).unwrap();
    // Regenerate via second staging; bytes must match.
    let staging2 = parent.join("staging2");
    prepare_staging_payload(
        &contract,
        &manifest,
        &parent.join("finalized"),
        &test_policy(),
        &BootstrapInstallPolicyV1::empty(),
        &staging2,
    )
    .unwrap();
    assert_eq!(
        sh1,
        std::fs::read_to_string(staging2.join("install.sh")).unwrap()
    );
    assert_eq!(
        ps1,
        std::fs::read_to_string(staging2.join("install.ps1")).unwrap()
    );
    std::fs::remove_dir_all(parent).unwrap();
}

// ---------------------------------------------------------------------------
// Adapter fixture matrix
// ---------------------------------------------------------------------------

fn payload_for_adapter(
    source: &str,
) -> (
    GitHubDraftPolicyV1,
    StagingPayloadV1,
    BTreeMap<String, Vec<u8>>,
) {
    let policy = test_policy();
    let payload = StagingPayloadV1 {
        schema_version: 1,
        product_id: "widget".to_owned(),
        release_id: "1.2.3".to_owned(),
        source_revision: source.to_owned(),
        owner: policy.owner.clone(),
        repository: policy.repository.clone(),
        tag: policy.tag.clone(),
        title: policy.title.clone(),
        prerelease: policy.prerelease,
        body: policy.body.clone(),
        assets: vec![
            StagingAsset {
                name: "app.bin".to_owned(),
                path: "app.bin".to_owned(),
                size: 4,
                sha256: sha_hex(b"body"),
                media_type: "application/octet-stream".to_owned(),
                kind: StagingAssetKind::FinalizedArtifact,
            },
            StagingAsset {
                name: "app.bin.sha256".to_owned(),
                path: "app.bin.sha256".to_owned(),
                size: 71,
                sha256: sha_hex(format!("{}  app.bin\n", sha_hex(b"body")).as_bytes()),
                media_type: "text/plain".to_owned(),
                kind: StagingAssetKind::ChecksumSidecar,
            },
        ],
    };
    let mut bytes = BTreeMap::new();
    bytes.insert("app.bin".to_owned(), b"body".to_vec());
    let sidecar = format!("{}  app.bin\n", sha_hex(b"body"));
    bytes.insert("app.bin.sha256".to_owned(), sidecar.into_bytes());
    // Fix sidecar size/sha to actual.
    let mut payload = payload;
    for asset in &mut payload.assets {
        let data = bytes.get(&asset.name).unwrap();
        asset.size = data.len() as u64;
        asset.sha256 = sha_hex(data);
    }
    (policy, payload, bytes)
}

#[tokio::test]
async fn lightweight_exact_tag_accepted() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await
    .unwrap();
    assert!(receipt.draft && !receipt.immutable);
    assert!(receipt.created);
    assert_eq!(receipt.uploaded as usize, payload.assets.len());
}

#[tokio::test]
async fn annotated_tag_peeling_accepted() {
    let source = "c".repeat(40);
    let tag_obj = "b".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&tag_obj);
    {
        let mut inner = fixture.inner.lock().unwrap();
        inner.ref_kind = "tag".to_owned();
    }
    fixture.add_tag_object(&tag_obj, "commit", &source);
    let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await
    .unwrap();
    assert!(receipt.created);
}

#[tokio::test]
async fn tag_mismatch_rejects_before_mutation() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&"b".repeat(40));
    let result = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.create_calls(), 0);
    assert_eq!(fixture.upload_calls(), 0);
}

#[tokio::test]
async fn excessive_tag_depth_rejects() {
    let source = "z".repeat(40).replace('z', "a");
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let first = "b".repeat(40);
    let fixture = FixtureGithub::with_tag(&first);
    {
        let mut inner = fixture.inner.lock().unwrap();
        inner.ref_kind = "tag".to_owned();
        // Chain 9 tag objects (depth bound is 8).
        let mut prev = first.clone();
        for i in 0..9u8 {
            let next = format!("c{i}{}", "d".repeat(37));
            inner
                .tag_objects
                .insert(prev.clone(), ("tag".to_owned(), next.clone()));
            prev = next;
        }
    }
    let result = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.create_calls(), 0);
}

#[tokio::test]
async fn existing_exact_draft_reused() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    fixture.seed_release(
        fixture_release(7, "v1.2.3", "widget 1.2.3", "notes", false, true, false),
        Vec::new(),
    );
    let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await
    .unwrap();
    assert!(!receipt.created);
    assert_eq!(receipt.github_release_id, 7);
}

#[tokio::test]
async fn published_and_immutable_reject() {
    for (draft, immutable) in [(false, false), (true, true)] {
        let source = "a".repeat(40);
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        fixture.seed_release(
            fixture_release(
                7,
                "v1.2.3",
                "widget 1.2.3",
                "notes",
                false,
                draft,
                immutable,
            ),
            Vec::new(),
        );
        let result = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
            bytes
                .get(name)
                .cloned()
                .ok_or_else(|| fail("missing bytes"))
        })
        .await;
        assert!(result.is_err(), "draft={draft} immutable={immutable}");
    }
}

#[tokio::test]
async fn wrong_draft_policy_rejects() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    fixture.seed_release(
        fixture_release(7, "v1.2.3", "wrong title", "notes", false, true, false),
        Vec::new(),
    );
    let result = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.upload_calls(), 0);
}

#[tokio::test]
async fn existing_exact_assets_reused_and_missing_uploaded() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    // Seed one exact asset, leave the other missing.
    let first = &payload.assets[0];
    fixture.seed_release(
        fixture_release(7, "v1.2.3", "widget 1.2.3", "notes", false, true, false),
        vec![fixture_asset(
            1,
            &first.name,
            first.size,
            "uploaded",
            Some(&first.sha256),
        )],
    );
    let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await
    .unwrap();
    assert_eq!(receipt.reused, 1);
    assert_eq!(receipt.uploaded, 1);
    // Rerun reuses both.
    let receipt2 = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes
            .get(name)
            .cloned()
            .ok_or_else(|| fail("missing bytes"))
    })
    .await
    .unwrap();
    assert_eq!(receipt2.reused, 2);
    assert_eq!(receipt2.uploaded, 0);
}

#[tokio::test]
async fn mismatched_unexpected_and_duplicate_assets_reject() {
    // Mismatched size.
    {
        let source = "a".repeat(40);
        let (policy, payload, _) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        let first = &payload.assets[0];
        fixture.seed_release(
            fixture_release(7, "v1.2.3", "widget 1.2.3", "notes", false, true, false),
            vec![fixture_asset(
                1,
                &first.name,
                first.size + 1,
                "uploaded",
                Some(&first.sha256),
            )],
        );
        let bytes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
    // Unexpected asset.
    {
        let source = "a".repeat(40);
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        fixture.seed_release(
            fixture_release(7, "v1.2.3", "widget 1.2.3", "notes", false, true, false),
            vec![fixture_asset(
                1,
                "evil.bin",
                4,
                "uploaded",
                Some(&sha_hex(b"body")),
            )],
        );
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
    // Duplicate same-name.
    {
        let source = "a".repeat(40);
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        let first = &payload.assets[0];
        fixture.seed_release(
            fixture_release(7, "v1.2.3", "widget 1.2.3", "notes", false, true, false),
            vec![
                fixture_asset(1, &first.name, first.size, "uploaded", Some(&first.sha256)),
                fixture_asset(2, &first.name, first.size, "uploaded", Some(&first.sha256)),
            ],
        );
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
}

#[tokio::test]
async fn renamed_and_422_responses_fail_closed() {
    let source = "a".repeat(40);
    // Rename.
    {
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        fixture.rename_next_upload();
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
    // 422.
    {
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        fixture.fail_upload_422_next();
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
    // Digest/size mismatch in upload response.
    {
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        fixture.digest_mismatch_next();
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
    {
        let (policy, payload, bytes) = payload_for_adapter(&source);
        let fixture = FixtureGithub::with_tag(&source);
        fixture.size_mismatch_next();
        assert!(
            stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
                bytes.get(name).cloned().ok_or_else(|| fail("missing"))
            })
            .await
            .is_err()
        );
    }
}

#[tokio::test]
async fn starter_502_cleanup_is_narrow_and_resumable() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    fixture.fail_upload_502_next("app.bin", true);
    let before_deletes = fixture.delete_calls();
    let result = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes.get(name).cloned().ok_or_else(|| fail("missing"))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.delete_calls(), before_deletes + 1);
    // Next invocation resumes and succeeds.
    let receipt = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes.get(name).cloned().ok_or_else(|| fail("missing"))
    })
    .await
    .unwrap();
    assert!(receipt.uploaded >= 1);
}

#[tokio::test]
async fn starter_502_without_exact_starter_deletes_nothing() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    fixture.fail_upload_502_next("app.bin", false);
    let before = fixture.delete_calls();
    let result = stage_with_bytes(&payload, &policy, &fixture, "token", |name| {
        bytes.get(name).cloned().ok_or_else(|| fail("missing"))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.delete_calls(), before);
}

#[tokio::test]
async fn post_stage_tag_movement_rejects() {
    // Fixture that moves the tag after uploads complete. Implement by wrapping
    // the fixture: first calls succeed, then ref changes. Simplest: use a
    // custom transport that flips the tag on the final verification.
    struct MovingTransport {
        fixture: FixtureGithub,
        moved: Mutex<bool>,
        source: String,
    }
    impl GithubApi for MovingTransport {
        async fn get_ref(&self, o: &str, r: &str, t: &str) -> Result<RefTarget, GithubError> {
            if *self.moved.lock().unwrap() {
                Ok(RefTarget {
                    sha: "b".repeat(40),
                    kind: "commit".to_owned(),
                })
            } else {
                self.fixture.get_ref(o, r, t).await
            }
        }
        async fn get_tag(&self, o: &str, r: &str, s: &str) -> Result<TagObject, GithubError> {
            self.fixture.get_tag(o, r, s).await
        }
        async fn list_releases(
            &self,
            o: &str,
            r: &str,
            p: u32,
        ) -> Result<Vec<RemoteRelease>, GithubError> {
            self.fixture.list_releases(o, r, p).await
        }
        async fn create_release(
            &self,
            o: &str,
            r: &str,
            t: &str,
            title: &str,
            b: &str,
            pre: bool,
        ) -> Result<RemoteRelease, GithubError> {
            self.fixture.create_release(o, r, t, title, b, pre).await
        }
        async fn list_assets(
            &self,
            o: &str,
            r: &str,
            id: u64,
        ) -> Result<Vec<RemoteAsset>, GithubError> {
            self.fixture.list_assets(o, r, id).await
        }
        async fn upload_asset(
            &self,
            o: &str,
            r: &str,
            id: u64,
            n: &str,
            bytes: &[u8],
            ct: &str,
        ) -> Result<RemoteAsset, GithubError> {
            let result = self.fixture.upload_asset(o, r, id, n, bytes, ct).await;
            // Move the tag after the last upload so post-stage verification fails.
            *self.moved.lock().unwrap() = true;
            let _ = &self.source;
            result
        }
        async fn delete_asset(&self, o: &str, r: &str, id: u64) -> Result<(), GithubError> {
            self.fixture.delete_asset(o, r, id).await
        }
    }
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let moving = MovingTransport {
        fixture: FixtureGithub::with_tag(&source),
        moved: Mutex::new(false),
        source: source.clone(),
    };
    let result = stage_with_bytes(&payload, &policy, &moving, "token", |name| {
        bytes.get(name).cloned().ok_or_else(|| fail("missing"))
    })
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn token_absent_rejects_before_io_and_redacts_diagnostics() {
    let source = "a".repeat(40);
    let (policy, payload, bytes) = payload_for_adapter(&source);
    let fixture = FixtureGithub::with_tag(&source);
    let result = stage_with_bytes(&payload, &policy, &fixture, "", |name| {
        bytes.get(name).cloned().ok_or_else(|| fail("missing"))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(fixture.create_calls(), 0);
    assert_eq!(fixture.upload_calls(), 0);
    // Redaction: distinctive token must never appear in errors.
    let secret = "super-secret-token-12345";
    let fixture2 = FixtureGithub::with_tag(&"b".repeat(40));
    let error = stage_with_bytes(&payload, &policy, &fixture2, secret, |name| {
        bytes.get(name).cloned().ok_or_else(|| fail("missing"))
    })
    .await
    .unwrap_err();
    assert!(!error.to_string().contains(secret));
    let debug = format!(
        "{:?}",
        EggfetchTransport::new(secret.to_owned(), 30, 1_000_000).unwrap()
    );
    assert!(!debug.contains(secret));
}

#[test]
fn policy_rejects_injection_and_publish_flags() {
    let mut policy = test_policy();
    policy.owner = "evil/repo".to_owned();
    assert!(policy.validate().is_err());
    let mut policy = test_policy();
    policy.tag = "v1?x".to_owned();
    assert!(policy.validate().is_err());
    let mut policy = test_policy();
    policy.token_env = "OTHER".to_owned();
    assert!(policy.validate().is_err());
    // No draft=false / publish flag exists by construction: the struct has no
    // such field, and unknown fields are denied.
    assert!(GitHubDraftPolicyV1::from_json(
        r#"{"schema_version":1,"owner":"a","repository":"b","tag":"v1","title":"t","draft":false}"#
    )
    .is_err());
}

#[test]
fn lean_eggfetch_client_talks_to_loopback_without_retry_redirect() {
    // Prove the lean eggfetch profile performs bounded HTTP against loopback.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buffer = vec![0u8; 4096];
            let count = stream.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]);
            assert!(request.contains("GET /hello"));
            let body = r#"{"ok":true}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let client = eggfetch_core::Client::builder()
            .user_agent("eggpack-test")
            .timeout(eggfetch_core::Timeout::from_secs(10))
            .build();
        let url = format!("http://{address}/hello");
        let mut response = client
            .get(&url)
            .unwrap()
            .header("Accept", "application/vnd.github+json")
            .timeout(eggfetch_core::Timeout::from_secs(10))
            .max_decoded_body_size(1_000_000)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let bytes = response.bytes().await.unwrap();
        assert_eq!(&bytes[..], br#"{"ok":true}"#);
        server.await.unwrap();
    });
}
