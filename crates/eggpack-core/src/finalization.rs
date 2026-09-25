//! Qualification-gated local finalization and manifest aggregation.

use crate::{
    build_manifest, err, BuildAttempt, CandidateArtifact, CoreError, LogicalOutputSelector,
    QualificationEvidence, QualificationStatus, ReleasePlan, SupportTier,
};
use eggpack_contract::{expected_release_files, DistributionContract, ExpandedAssets};
use flate2::{Compression, GzBuilder};
use sha2::Digest;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Explicit archive encoding accepted by the local finalizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveEncoding {
    /// POSIX tar stream compressed with gzip; the contract asset name must end in `.tar.gz`.
    TarGzip,
}

/// One target's exact build and qualification evidence to finalize.
#[derive(Debug, Clone)]
pub struct FinalizationTargetInput {
    /// Contract canonical triple or unambiguous alias.
    pub target: String,
    /// Build result that produced the candidate bytes.
    pub attempt: BuildAttempt,
    /// M003 qualification evidence for this exact build attempt.
    pub qualification: QualificationEvidence,
}

/// Identity and target evidence aggregated into one local release.
#[derive(Debug, Clone)]
pub struct FinalizationRequest {
    /// Opaque product id.
    pub product_id: String,
    /// Opaque release id/version used by contract expansion.
    pub release_id: String,
    /// Opaque source revision.
    pub source_revision: String,
    /// Exact selected target build/evidence pairs.
    pub targets: Vec<FinalizationTargetInput>,
    /// Bounded references to external producer evidence.
    pub evidence_references: Vec<String>,
    /// Required explicit encoding when any selected target is an archive.
    pub archive_encoding: Option<ArchiveEncoding>,
}

/// Completed local output root and manifest, returned only after complete validation.
#[derive(Debug, Clone)]
pub struct FinalizedRelease {
    /// Invocation-owned directory containing contract-named artifacts and sidecars.
    pub root: PathBuf,
    /// Manifest v1 describing the final bytes and archive members.
    pub manifest: eggpack_manifest::ReleaseManifest,
}

/// Finalize candidate bytes under an initially absent, caller-selected private directory.
///
/// Required targets must have passing qualification. Non-gating/experimental targets may
/// remain deferred, but still need complete candidate byte evidence. All outputs are written
/// under `output_root`; on any error this invocation removes only the root it created.
pub fn finalize_release(
    contract: &DistributionContract,
    plan: &ReleasePlan,
    request: &FinalizationRequest,
    output_root: &Path,
) -> Result<FinalizedRelease, CoreError> {
    if contract.product.id != request.product_id
        || plan.schema_version != 1
        || plan.release_id != request.release_id
        || plan.source_revision != request.source_revision
        || plan.targets.is_empty()
        || plan.targets.len() > 256
        || request.targets.is_empty()
        || request.targets.len() > 256
        || !output_root.is_absolute()
    {
        return Err(err(
            "finalization identity, target count, or root is invalid",
        ));
    }
    if output_root.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    }) {
        return Err(err("finalization root path contains traversal components"));
    }
    let mut canonical_targets = BTreeSet::new();
    for target in &plan.targets {
        let expanded = contract
            .expand(&target.target, &plan.release_id)
            .map_err(|_| err("ReleasePlan target is not supported by contract"))?;
        let form_matches = matches!(
            (target.artifact_form, expanded.assets),
            (crate::PlannedAssetForm::Direct, ExpandedAssets::Direct(_))
                | (crate::PlannedAssetForm::Bundle, ExpandedAssets::Bundle(_))
                | (crate::PlannedAssetForm::Archive, ExpandedAssets::Archive(_))
        );
        if target.policy.target != target.target
            || !canonical_targets.insert(target.target.as_str())
            || !form_matches
        {
            return Err(err("ReleasePlan target inventory or layout is invalid"));
        }
    }
    let parent = output_root
        .parent()
        .ok_or_else(|| err("finalization root has no parent"))?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|_| err("finalization root parent is unavailable"))?;
    if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(err("finalization root parent is not a real directory"));
    }
    let filename = output_root
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| err("finalization root name is invalid"))?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(|_| err("finalization root parent cannot be resolved"))?;
    let root = canonical_parent.join(filename);
    if fs::symlink_metadata(&root).is_ok() {
        return Err(err("finalization root must be absent"));
    }
    fs::create_dir(&root).map_err(|_| err("finalization root could not be created"))?;
    let result = (|| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|_| err("finalization root permissions could not be restricted"))?;
        }
        finalize_into(contract, plan, request, &root)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&root);
    }
    result
}

fn finalize_into(
    contract: &DistributionContract,
    plan: &ReleasePlan,
    request: &FinalizationRequest,
    root: &Path,
) -> Result<FinalizedRelease, CoreError> {
    let has_archive = request.targets.iter().any(|input| {
        contract
            .expand(&input.target, &request.release_id)
            .is_ok_and(|expanded| matches!(expanded.assets, ExpandedAssets::Archive(_)))
    });
    if has_archive != request.archive_encoding.is_some()
        || (has_archive && request.archive_encoding != Some(ArchiveEncoding::TarGzip))
    {
        return Err(err(
            "archive encoding must be explicit and match selected layouts",
        ));
    }

    let expected_targets: BTreeSet<_> = plan.targets.iter().map(|t| t.target.as_str()).collect();
    let mut actual_targets = BTreeSet::new();
    let mut manifest_targets = Vec::new();
    let staging_members = root.join(".eggpack-members");
    let mut staging_created = false;
    for input in &request.targets {
        let canonical_target = contract
            .resolve(&input.target)
            .map_err(|_| err("finalization target is not supported by contract"))?
            .triple
            .clone();
        let planned = plan
            .targets
            .iter()
            .find(|target| target.target == canonical_target)
            .ok_or_else(|| err("finalization target is not in ReleasePlan"))?;
        if input.attempt.target != planned.target
            || input.attempt.release_id != request.release_id
            || input.attempt.source_revision != request.source_revision
            || input.attempt.strategy != planned.policy.strategy
            || input.attempt.process.outcome != crate::CommandOutcome::Success
            || !actual_targets.insert(planned.target.as_str())
        {
            return Err(err("build attempt identity or target inventory is invalid"));
        }
        input
            .qualification
            .validate_for(plan, planned, &input.attempt)
            .map_err(|_| err("qualification evidence does not match build attempt"))?;
        match input.qualification.status {
            QualificationStatus::Passed => {}
            QualificationStatus::Deferred if planned.policy.support != SupportTier::Required => {}
            _ => return Err(err("target qualification does not permit finalization")),
        }
        let candidate_map = validated_candidates(&input.attempt, &input.qualification)?;
        let expanded = contract
            .expand(&input.target, &request.release_id)
            .map_err(|_| err("target expansion failed"))?;
        validate_layout_selectors(&candidate_map, &expanded.assets)?;
        let expected_files = expected_release_files(contract, &input.target, &request.release_id)
            .map_err(|_| err("expected contract inventory unavailable"))?;
        let mut release_files = BTreeMap::new();
        let mut archive_members = BTreeMap::new();
        match expanded.assets {
            ExpandedAssets::Direct(direct) => {
                let candidate = get_candidate(&candidate_map, &LogicalOutputSelector::Direct)?;
                copy_verified(candidate, &input.qualification, &direct.asset_file, root)?;
                let path = root.join(&direct.asset_file);
                write_checksum(&path, &direct.sidecar_file, root)?;
                release_files.insert(direct.asset_file, path);
                release_files.insert(direct.sidecar_file.clone(), root.join(direct.sidecar_file));
            }
            ExpandedAssets::Bundle(bundle) => {
                for (index, entry) in bundle.entries.iter().enumerate() {
                    let selector = LogicalOutputSelector::BundleEntry { index };
                    let candidate = get_candidate(&candidate_map, &selector)?;
                    copy_verified(candidate, &input.qualification, &entry.asset_file, root)?;
                    let path = root.join(&entry.asset_file);
                    write_checksum(&path, &entry.sidecar_file, root)?;
                    release_files.insert(entry.asset_file.clone(), path);
                    release_files
                        .insert(entry.sidecar_file.clone(), root.join(&entry.sidecar_file));
                }
            }
            ExpandedAssets::Archive(archive) => {
                if !archive.archive_file.ends_with(".tar.gz") {
                    return Err(err(
                        "TarGzip encoding requires a `.tar.gz` contract filename",
                    ));
                }
                if !staging_created {
                    fs::create_dir(&staging_members)
                        .map_err(|_| err("archive member staging root could not be created"))?;
                    staging_created = true;
                }
                let mut staged_candidates = Vec::with_capacity(archive.members.len());
                for (index, member) in archive.members.iter().enumerate() {
                    let selector = LogicalOutputSelector::ArchiveMember {
                        source: member.source.clone(),
                    };
                    let candidate = get_candidate(&candidate_map, &selector)?;
                    let staged_name = format!("member-{index}");
                    copy_verified(
                        candidate,
                        &input.qualification,
                        &staged_name,
                        &staging_members,
                    )?;
                    let staged = CandidateArtifact {
                        path: staging_members.join(staged_name),
                        ..candidate.clone()
                    };
                    archive_members.insert(member.source.clone(), staged.path.clone());
                    staged_candidates.push(staged);
                }
                let members: Vec<_> = archive
                    .members
                    .iter()
                    .zip(staged_candidates.iter())
                    .map(|(member, candidate)| (member.source.as_str(), candidate))
                    .collect();
                create_tar_gzip(&root.join(&archive.archive_file), &members)?;
                let archive_path = root.join(&archive.archive_file);
                write_checksum(&archive_path, &archive.sidecar_file, root)?;
                release_files.insert(archive.archive_file, archive_path);
                release_files.insert(
                    archive.sidecar_file.clone(),
                    root.join(archive.sidecar_file),
                );
            }
        }
        let actual_files: BTreeSet<_> = release_files.keys().cloned().collect();
        let expected_names: BTreeSet<_> = expected_files
            .into_iter()
            .map(|item| item.file_name)
            .collect();
        if actual_files != expected_names {
            return Err(err(
                "finalized files do not exactly match contract inventory",
            ));
        }
        manifest_targets.push(crate::FinalizedTargetInput {
            target: planned.target.clone(),
            release_files,
            archive_members,
        });
    }
    if expected_targets != actual_targets {
        return Err(err("finalization targets do not exactly cover ReleasePlan"));
    }
    manifest_targets.sort_by(|left, right| left.target.cmp(&right.target));
    let manifest_input = crate::FinalizedReleaseInput {
        product_id: request.product_id.clone(),
        release_id: request.release_id.clone(),
        source_revision: request.source_revision.clone(),
        targets: manifest_targets,
        evidence_references: request.evidence_references.clone(),
    };
    let manifest = build_manifest(contract, &manifest_input)?;
    if staging_created {
        fs::remove_dir_all(&staging_members)
            .map_err(|_| err("archive member staging cleanup failed"))?;
    }
    Ok(FinalizedRelease {
        root: root.to_path_buf(),
        manifest,
    })
}

fn validated_candidates<'a>(
    attempt: &'a BuildAttempt,
    qualification: &crate::QualificationEvidence,
) -> Result<BTreeMap<LogicalOutputSelector, &'a CandidateArtifact>, CoreError> {
    if qualification.candidates.len() != attempt.candidates.len() {
        return Err(err("qualification candidate evidence is incomplete"));
    }
    let qualified: BTreeMap<_, _> = qualification
        .candidates
        .iter()
        .map(|item| (item.selector.clone(), item))
        .collect();
    let mut candidates = BTreeMap::new();
    for candidate in &attempt.candidates {
        let evidence = qualified
            .get(&candidate.selector)
            .ok_or_else(|| err("candidate has no qualification byte evidence"))?;
        let (size, digest) = digest_regular(&candidate.path)?;
        if evidence.package != candidate.package
            || evidence.binary != candidate.binary
            || evidence.size != candidate.size
            || size != evidence.size
            || digest != evidence.sha256
            || candidates
                .insert(candidate.selector.clone(), candidate)
                .is_some()
        {
            return Err(err(
                "candidate bytes or identity differ from qualification evidence",
            ));
        }
    }
    Ok(candidates)
}

fn get_candidate<'a>(
    candidates: &'a BTreeMap<LogicalOutputSelector, &'a CandidateArtifact>,
    selector: &LogicalOutputSelector,
) -> Result<&'a CandidateArtifact, CoreError> {
    candidates
        .get(selector)
        .copied()
        .ok_or_else(|| err("required logical candidate is missing"))
}

fn validate_layout_selectors(
    candidates: &BTreeMap<LogicalOutputSelector, &CandidateArtifact>,
    form: &ExpandedAssets,
) -> Result<(), CoreError> {
    let expected: BTreeSet<_> = match form {
        ExpandedAssets::Direct(_) => [LogicalOutputSelector::Direct].into_iter().collect(),
        ExpandedAssets::Bundle(bundle) => (0..bundle.entries.len())
            .map(|index| LogicalOutputSelector::BundleEntry { index })
            .collect(),
        ExpandedAssets::Archive(archive) => archive
            .members
            .iter()
            .map(|member| LogicalOutputSelector::ArchiveMember {
                source: member.source.clone(),
            })
            .collect(),
    };
    if candidates.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err(err(
            "candidate selectors do not exactly match contract layout",
        ));
    }
    Ok(())
}

fn copy_verified(
    candidate: &CandidateArtifact,
    qualification: &QualificationEvidence,
    filename: &str,
    root: &Path,
) -> Result<(), CoreError> {
    let evidence = qualification
        .candidates
        .iter()
        .find(|item| item.selector == candidate.selector)
        .ok_or_else(|| err("candidate qualification digest is missing"))?;
    let destination = root.join(filename);
    let mut source = File::open(&candidate.path)
        .map_err(|_| err("qualified candidate could not be reopened"))?;
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|_| err("contract output path already exists"))?;
    std::io::copy(&mut source, &mut target).map_err(|_| err("candidate copy failed"))?;
    target
        .flush()
        .map_err(|_| err("finalized file flush failed"))?;
    drop(target);
    let (size, digest) = digest_regular(&destination)?;
    if size != evidence.size || digest != evidence.sha256 {
        return Err(err("candidate changed while being finalized"));
    }
    Ok(())
}

fn create_tar_gzip(path: &Path, members: &[(&str, &CandidateArtifact)]) -> Result<(), CoreError> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| err("archive output path already exists"))?;
    let encoder = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.mode(tar::HeaderMode::Deterministic);
    for (member_name, candidate) in members {
        let (size, _) = digest_regular(&candidate.path)?;
        let mut source = File::open(&candidate.path)
            .map_err(|_| err("archive member candidate could not be opened"))?;
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header
            .set_path(member_name)
            .map_err(|_| err("archive member path rejected"))?;
        header.set_size(size);
        header.set_mode(0o755);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        archive
            .append(&header, &mut source)
            .map_err(|_| err("archive member assembly failed"))?;
    }
    let encoder = archive
        .into_inner()
        .map_err(|_| err("archive stream finalization failed"))?;
    encoder
        .finish()
        .map_err(|_| err("gzip stream finalization failed"))?;
    Ok(())
}

fn write_checksum(artifact: &Path, filename: &str, root: &Path) -> Result<(), CoreError> {
    let (_, digest) = digest_regular(artifact)?;
    let sidecar = root.join(filename);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(sidecar)
        .map_err(|_| err("checksum sidecar path already exists"))?;
    writeln!(
        file,
        "{digest}  {}",
        artifact.file_name().unwrap_or_default().to_string_lossy()
    )
    .map_err(|_| err("checksum sidecar write failed"))?;
    file.flush()
        .map_err(|_| err("checksum sidecar flush failed"))?;
    Ok(())
}

fn digest_regular(path: &Path) -> Result<(u64, String), CoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| err("candidate file is unavailable"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() == 0 {
        return Err(err("candidate must be a non-empty regular file"));
    }
    let mut file = File::open(path).map_err(|_| err("candidate file cannot be opened"))?;
    let mut size = 0u64;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| err("candidate file read failed"))?;
        if count == 0 {
            break;
        }
        size = size
            .checked_add(count as u64)
            .ok_or_else(|| err("candidate size overflow"))?;
        hasher.update(&buffer[..count]);
    }
    let after = fs::symlink_metadata(path).map_err(|_| err("candidate changed during read"))?;
    if after.file_type().is_symlink() || !after.is_file() || after.len() != size {
        return Err(err("candidate changed during read"));
    }
    let digest = hasher.finalize();
    Ok((
        size,
        digest.iter().map(|byte| format!("{byte:02x}")).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArtifactForm, BuildStrategy, CandidateArchitecture, CandidateFormat, CommandOutcome,
        CompatibilityFloor, HostArch, HostOs, HostRequirement, PackConfig, Qualification,
        QualificationMethod, QualifiedCandidateEvidence, SupportTier, TargetPolicy,
        ToolchainRequirement,
    };
    use eggpack_contract::DistributionContract;
    use sha2::Digest;

    fn elf() -> Vec<u8> {
        let mut bytes = vec![0; 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
        bytes
    }

    fn digest(bytes: &[u8]) -> String {
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn fixture(
        contract_text: &str,
        target_alias: &str,
        release_id: &str,
        root: &Path,
    ) -> (
        DistributionContract,
        ReleasePlan,
        FinalizationRequest,
        PathBuf,
    ) {
        let contract = DistributionContract::parse_toml_str(contract_text).unwrap();
        let config = PackConfig {
            schema_version: 1,
            targets: vec![TargetPolicy {
                target: target_alias.into(),
                strategy: BuildStrategy::NativeCargo,
                host_os: HostOs::Linux,
                host_arch: HostArch::X86_64,
                qualification_host: None,
                toolchain: ToolchainRequirement {
                    rust: "1.89.0".into(),
                    cargo_zigbuild: None,
                },
                floor: CompatibilityFloor::None,
                qualification: Qualification::Structural,
                support: SupportTier::Required,
            }],
        };
        let plan = config
            .resolve(
                &contract,
                release_id,
                &"a".repeat(40),
                &[target_alias.into()],
            )
            .unwrap();
        let planned = &plan.targets[0];
        let expanded = contract.expand(target_alias, release_id).unwrap();
        let selectors: Vec<_> = match expanded.assets {
            ExpandedAssets::Direct(_) => vec![LogicalOutputSelector::Direct],
            ExpandedAssets::Bundle(bundle) => (0..bundle.entries.len())
                .map(|index| LogicalOutputSelector::BundleEntry { index })
                .collect(),
            ExpandedAssets::Archive(archive) => archive
                .members
                .into_iter()
                .map(|member| LogicalOutputSelector::ArchiveMember {
                    source: member.source,
                })
                .collect(),
        };
        let source_root = root.join("candidates");
        fs::create_dir(&source_root).unwrap();
        let bytes = elf();
        let candidates: Vec<_> = selectors
            .into_iter()
            .enumerate()
            .map(|(index, selector)| {
                let path = source_root.join(format!("candidate-{index}"));
                fs::write(&path, &bytes).unwrap();
                CandidateArtifact {
                    target: planned.target.clone(),
                    selector,
                    package: "fixture".into(),
                    binary: format!("fixture-{index}"),
                    path,
                    size: bytes.len() as u64,
                }
            })
            .collect();
        let attempt = BuildAttempt {
            release_id: plan.release_id.clone(),
            source_revision: plan.source_revision.clone(),
            target: planned.target.clone(),
            strategy: BuildStrategy::NativeCargo,
            tool_summary: "fixture".into(),
            process: crate::ProcessEvidence {
                outcome: CommandOutcome::Success,
                stdout_bytes: 0,
                stderr_bytes: 0,
            },
            candidates: candidates.clone(),
        };
        let mut qualified = candidates
            .iter()
            .map(|candidate| QualifiedCandidateEvidence {
                selector: candidate.selector.clone(),
                package: candidate.package.clone(),
                binary: candidate.binary.clone(),
                size: bytes.len() as u64,
                sha256: digest(&bytes),
                format: CandidateFormat::Elf,
                architecture: CandidateArchitecture::X86_64,
            })
            .collect::<Vec<_>>();
        qualified.sort_by(|a, b| a.selector.cmp(&b.selector));
        let qualification = QualificationEvidence {
            schema_version: 1,
            release_id: plan.release_id.clone(),
            source_revision: plan.source_revision.clone(),
            target: planned.target.clone(),
            planned_classification: Qualification::Structural,
            method: QualificationMethod::Structural,
            actual_host: HostRequirement {
                os: HostOs::Linux,
                arch: HostArch::X86_64,
            },
            support: SupportTier::Required,
            status: QualificationStatus::Passed,
            candidates: qualified,
            smoke_selector: None,
            processes: vec![],
        };
        let request = FinalizationRequest {
            product_id: contract.product.id.clone(),
            release_id: plan.release_id.clone(),
            source_revision: plan.source_revision.clone(),
            targets: vec![FinalizationTargetInput {
                target: target_alias.into(),
                attempt,
                qualification,
            }],
            evidence_references: vec!["qualification:fixture".into()],
            archive_encoding: matches!(planned.artifact_form, crate::PlannedAssetForm::Archive)
                .then_some(ArchiveEncoding::TarGzip),
        };
        (contract, plan, request, source_root)
    }

    #[test]
    fn direct_and_bundle_finalization_produce_contract_inventory_and_final_hashes() {
        for (name, contract_text, product, target, release) in [
            (
                "direct",
                include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
                "eggsact",
                "linux-x64",
                "1.2.6",
            ),
            (
                "bundle",
                include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
                "codegg",
                "linux-x64",
                "2.4.0",
            ),
        ] {
            let parent = crate::test_temp_dir(name);
            let (contract, plan, request, candidates) =
                fixture(contract_text, target, release, &parent);
            assert_eq!(contract.product.id, product);
            let output = parent.join("release");
            let finalized = finalize_release(&contract, &plan, &request, &output).unwrap();
            assert_eq!(
                fs::canonicalize(&finalized.root).unwrap(),
                fs::canonicalize(&output).unwrap()
            );
            assert_eq!(finalized.manifest.release_id, release);
            match &finalized.manifest.targets[0].form {
                ArtifactForm::Direct { artifact, .. } => {
                    assert_eq!(artifact.size, 64);
                    assert_eq!(artifact.sha256, digest(&elf()));
                    assert_eq!(
                        fs::read_to_string(
                            finalized.root.join(format!("{}.sha256", artifact.name))
                        )
                        .unwrap()
                        .trim(),
                        format!("{}  {}", artifact.sha256, artifact.name)
                    );
                }
                ArtifactForm::Bundle { entries } => assert_eq!(entries.len(), 3),
                _ => panic!("unexpected finalized form"),
            }
            assert!(!finalized.root.join("unused").exists());
            fs::remove_dir_all(parent).unwrap();
            let _ = candidates;
        }
    }

    #[test]
    fn archive_is_explicit_deterministic_and_contains_exact_contract_members() {
        let parent = crate::test_temp_dir("finalize-archive");
        let (contract, plan, request, _) = fixture(
            include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
            "linux-x64",
            "3.1.0",
            &parent,
        );
        let first_root = parent.join("first");
        let second_root = parent.join("second");
        let first = finalize_release(&contract, &plan, &request, &first_root).unwrap();
        let second = finalize_release(&contract, &plan, &request, &second_root).unwrap();
        assert_eq!(first.manifest, second.manifest);
        let first_archive =
            fs::read(first_root.join("egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz")).unwrap();
        let second_archive =
            fs::read(second_root.join("egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz")).unwrap();
        assert_eq!(first_archive, second_archive);
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(first_archive.as_slice()));
        let entries = archive
            .entries()
            .unwrap()
            .map(|entry| {
                let mut entry = entry.unwrap();
                let name = entry.path().unwrap().to_string_lossy().into_owned();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                (name, bytes)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            entries,
            vec![
                ("egress".into(), elf()),
                ("bin/egress-helper".into(), elf())
            ]
        );
        assert!(
            matches!(&first.manifest.targets[0].form, ArtifactForm::Archive { members, .. } if members.len() == 2)
        );
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn incomplete_qualification_tampering_and_existing_roots_fail_without_partial_success() {
        let parent = crate::test_temp_dir("finalize-negative");
        let (contract, plan, mut request, source_root) = fixture(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "linux-x64",
            "1.2.6",
            &parent,
        );
        let existing = parent.join("existing");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("preserve"), b"sentinel").unwrap();
        assert!(finalize_release(&contract, &plan, &request, &existing).is_err());
        assert_eq!(fs::read(existing.join("preserve")).unwrap(), b"sentinel");

        request.targets[0].qualification.status = QualificationStatus::Deferred;
        assert!(finalize_release(
            &contract,
            &plan,
            &request,
            &parent.join("required-deferred")
        )
        .is_err());
        request.targets[0].qualification.status = QualificationStatus::Passed;
        fs::write(&request.targets[0].attempt.candidates[0].path, b"changed").unwrap();
        let rejected_root = parent.join("rejected");
        assert!(finalize_release(&contract, &plan, &request, &rejected_root).is_err());
        assert!(!rejected_root.exists());
        assert!(source_root.exists());
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn finalization_rejects_mixed_identity_and_extra_candidate_slots() {
        let parent = crate::test_temp_dir("finalize-inventory");
        let (contract, plan, mut request, _) = fixture(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "linux-x64",
            "1.2.6",
            &parent,
        );
        request.targets[0].attempt.source_revision = "b".repeat(40);
        assert!(
            finalize_release(&contract, &plan, &request, &parent.join("mixed-source")).is_err()
        );

        request.targets[0].attempt.source_revision = plan.source_revision.clone();
        let mut extra = request.targets[0].attempt.candidates[0].clone();
        extra.selector = LogicalOutputSelector::BundleEntry { index: 0 };
        request.targets[0].attempt.candidates.push(extra.clone());
        let original = request.targets[0].qualification.candidates[0].clone();
        request.targets[0]
            .qualification
            .candidates
            .push(QualifiedCandidateEvidence {
                selector: extra.selector,
                ..original
            });
        request.targets[0]
            .qualification
            .candidates
            .sort_by(|a, b| a.selector.cmp(&b.selector));
        assert!(finalize_release(&contract, &plan, &request, &parent.join("extra-slot")).is_err());
        assert!(!parent.join("mixed-source").exists());
        assert!(!parent.join("extra-slot").exists());
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn concurrent_invocations_with_distinct_roots_do_not_share_outputs() {
        let parent = crate::test_temp_dir("finalize-concurrent");
        let (contract, plan, request, _) = fixture(
            include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml"),
            "linux-x64",
            "1.2.6",
            &parent,
        );
        let contract_a = contract.clone();
        let plan_a = plan.clone();
        let request_a = request.clone();
        let root_a = parent.join("release-a");
        let root_b = parent.join("release-b");
        let a = std::thread::spawn(move || {
            finalize_release(&contract_a, &plan_a, &request_a, &root_a).unwrap()
        });
        let b = std::thread::spawn(move || {
            finalize_release(&contract, &plan, &request, &root_b).unwrap()
        });
        let a = a.join().unwrap();
        let b = b.join().unwrap();
        assert_eq!(a.manifest, b.manifest);
        let asset = "eggsact-1.2.6-x86_64-unknown-linux-gnu";
        assert_eq!(
            fs::read(a.root.join(asset)).unwrap(),
            fs::read(b.root.join(asset)).unwrap()
        );
        assert!(a.root != b.root);
        fs::remove_dir_all(parent).unwrap();
    }
}
