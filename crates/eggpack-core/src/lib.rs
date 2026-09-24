#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Producer-side release planning and finalized artifact evidence.

use eggpack_contract::{
    expected_release_files, validate_release_inventory, DistributionContract, ExpandedAssets,
    ExtrasPolicy, ReleaseInventory,
};
use eggpack_manifest::{
    ArchiveMemberRecord, ArtifactForm, ArtifactRecord, BundleRecord, ByteEvidence, ReleaseManifest,
    TargetRecord,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

/// Explicit mapping of one target's expanded contract filenames to local finalized files.
#[derive(Debug, Clone)]
pub struct FinalizedTargetInput {
    /// Contract canonical triple or unambiguous alias.
    pub target: String,
    /// Explicit flat release filename to local path mappings, including checksum sidecars.
    pub release_files: BTreeMap<String, PathBuf>,
    /// Explicit archive-member source path to finalized local member bytes.
    pub archive_members: BTreeMap<String, PathBuf>,
}

/// Caller-supplied identity and finalized file inventory for one release.
#[derive(Debug, Clone)]
pub struct FinalizedReleaseInput {
    /// Opaque product id.
    pub product_id: String,
    /// Opaque release id/version used in contract expansion.
    pub release_id: String,
    /// Opaque source revision.
    pub source_revision: String,
    /// Selected finalized target inventories.
    pub targets: Vec<FinalizedTargetInput>,
    /// Bounded producer evidence references.
    pub evidence_references: Vec<String>,
}

/// Manifest construction failure. Paths and file contents are intentionally omitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError(String);
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CoreError {}
fn err(s: impl Into<String>) -> CoreError {
    CoreError(s.into())
}
fn host_matches_target(host: HostRequirement, triple: &str) -> bool {
    let arch = match host.arch {
        HostArch::X86_64 => triple.starts_with("x86_64-"),
        HostArch::Aarch64 => triple.starts_with("aarch64-"),
        HostArch::Armv7 => triple.starts_with("armv7-"),
    };
    let os = match host.os {
        HostOs::Linux => triple.contains("-linux-"),
        HostOs::Macos => triple.ends_with("-apple-darwin"),
        HostOs::Windows => triple.contains("-windows-"),
    };
    arch && os
}
fn validate_policy(policy: &TargetPolicy, triple: &str) -> Result<(), CoreError> {
    if policy.toolchain.rust.is_empty()
        || policy.toolchain.rust.len() > 64
        || policy
            .toolchain
            .cargo_zigbuild
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() > 64)
    {
        return Err(err("toolchain requirement is empty or overlong"));
    }
    if (policy.strategy == BuildStrategy::CargoZigbuild)
        != policy.toolchain.cargo_zigbuild.is_some()
    {
        return Err(err(
            "cargo-zigbuild strategy requires an explicit tool version",
        ));
    }
    match policy.floor {
        CompatibilityFloor::Glibc { .. } if !triple.contains("-linux-") => {
            return Err(err("glibc floor applies only to Linux targets"))
        }
        CompatibilityFloor::Macos { .. } if !triple.ends_with("-apple-darwin") => {
            return Err(err("macOS floor applies only to macOS targets"))
        }
        _ => {}
    }
    if policy.qualification == Qualification::Native {
        let host = policy.qualification_host.unwrap_or(HostRequirement {
            os: policy.host_os,
            arch: policy.host_arch,
        });
        if policy.strategy == BuildStrategy::CargoZigbuild || !host_matches_target(host, triple) {
            return Err(err(
                "native qualification requires a matching native build/qualification host",
            ));
        }
    }
    Ok(())
}

/// Build and hash a complete schema-v1 manifest from explicitly supplied files.
pub fn build_manifest(
    contract: &DistributionContract,
    input: &FinalizedReleaseInput,
) -> Result<ReleaseManifest, CoreError> {
    if contract.product.id != input.product_id {
        return Err(err("product identity does not match contract"));
    }
    if input.targets.is_empty() || input.targets.len() > 256 {
        return Err(err("target count out of bounds"));
    }
    let mut target_records = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for target in &input.targets {
        let expanded = contract
            .expand(&target.target, &input.release_id)
            .map_err(|_| err("target expansion failed"))?;
        if !seen.insert(expanded.triple.clone()) {
            return Err(err("duplicate canonical target"));
        }
        let expected = expected_release_files(contract, &target.target, &input.release_id)
            .map_err(|_| err("expected inventory derivation failed"))?;
        let inventory = ReleaseInventory::new(target.release_files.keys().cloned())
            .map_err(|_| err("invalid release inventory"))?;
        if !validate_release_inventory(&expected, &inventory, ExtrasPolicy::Exact).is_conformant() {
            return Err(err("release inventory is incomplete or contains extras"));
        }
        let artifact = |name: &str| -> Result<ArtifactRecord, CoreError> {
            let path = target
                .release_files
                .get(name)
                .ok_or_else(|| err("required artifact path missing"))?;
            let (size, sha256) = digest_file(path)?;
            Ok(ArtifactRecord {
                name: name.to_string(),
                size,
                sha256,
            })
        };
        let record_form = match expanded.assets {
            ExpandedAssets::Direct(d) => {
                if !target.archive_members.is_empty() {
                    return Err(err("unexpected archive member inputs"));
                }
                ArtifactForm::Direct {
                    artifact: artifact(&d.asset_file)?,
                    install: d.install_name,
                }
            }
            ExpandedAssets::Bundle(b) => {
                if !target.archive_members.is_empty() {
                    return Err(err("unexpected archive member inputs"));
                }
                ArtifactForm::Bundle {
                    entries: b
                        .entries
                        .iter()
                        .map(|e| {
                            Ok(BundleRecord {
                                artifact: artifact(&e.asset_file)?,
                                install: e.install_name.clone(),
                            })
                        })
                        .collect::<Result<_, CoreError>>()?,
                }
            }
            ExpandedAssets::Archive(a) => {
                if target.archive_members.len() != a.members.len() {
                    return Err(err("archive member inventory incomplete or has extras"));
                }
                let members = a
                    .members
                    .iter()
                    .map(|m| {
                        let p = target
                            .archive_members
                            .get(&m.source)
                            .ok_or_else(|| err("required archive member path missing"))?;
                        let (size, sha256) = digest_file(p)?;
                        Ok(ArchiveMemberRecord {
                            source: m.source.clone(),
                            install: m.install_name.clone(),
                            bytes: ByteEvidence { size, sha256 },
                        })
                    })
                    .collect::<Result<_, CoreError>>()?;
                ArtifactForm::Archive {
                    artifact: artifact(&a.archive_file)?,
                    members,
                }
            }
        };
        target_records.push(TargetRecord {
            target: expanded.triple,
            form: record_form,
        });
    }
    let manifest = ReleaseManifest {
        schema_version: 1,
        product_id: input.product_id.clone(),
        release_id: input.release_id.clone(),
        source_revision: input.source_revision.clone(),
        targets: target_records,
        evidence_references: input.evidence_references.clone(),
    };
    manifest
        .validate()
        .map_err(|_| err("constructed manifest violates schema-v1 bounds"))?;
    Ok(manifest)
}

fn digest_file(path: &Path) -> Result<(u64, String), CoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| err("finalized file is unavailable"))?;
    if !metadata.file_type().is_file() {
        return Err(err("finalized input is not a regular non-symlink file"));
    }
    let mut reader =
        BufReader::new(File::open(path).map_err(|_| err("finalized file cannot be opened"))?);
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buf = [0u8; 65536];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|_| err("finalized file read failed"))?;
        if n == 0 {
            break;
        }
        size = size
            .checked_add(n as u64)
            .ok_or_else(|| err("file size overflow"))?;
        hasher.update(&buf[..n]);
    }
    if size == 0 {
        return Err(err("finalized files must be non-empty"));
    }
    let digest = hasher.finalize();
    Ok((size, digest.iter().map(|b| format!("{b:02x}")).collect()))
}

/// Explicit finite build strategy. Planning never executes either strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStrategy {
    /// Native Cargo target build.
    NativeCargo,
    /// Cargo zigbuild cross-target build.
    CargoZigbuild,
}
/// Host operating system family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOs {
    /// Linux.
    Linux,
    /// macOS.
    Macos,
    /// Windows.
    Windows,
}
/// Host CPU architecture family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostArch {
    /// x86-64.
    X86_64,
    /// 64-bit ARM.
    Aarch64,
    /// 32-bit ARM.
    Armv7,
}
/// Qualification classification stored as intent, not proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Qualification {
    /// Run on matching native host.
    Native,
    /// Qualification is deferred.
    DeferredNative,
    /// Run under emulation.
    Emulated,
    /// Structural checks only.
    Structural,
}
/// Support and release gating tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    /// Required release target.
    Required,
    /// Supported without gating.
    NonGating,
    /// Experimental target.
    Experimental,
}
/// Provider-neutral OS/architecture capability required of a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRequirement {
    /// Host operating system family.
    pub os: HostOs,
    /// Host architecture family.
    pub arch: HostArch,
}
/// Explicit compiler/toolchain requirements; no version is inferred from the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainRequirement {
    /// Exact or policy-bounded Rust toolchain string.
    pub rust: String,
    /// Optional exact cargo-zigbuild version when that strategy is selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_zigbuild: Option<String>,
}
/// Explicit compatibility floor for produced binaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompatibilityFloor {
    /// No additional deployment floor.
    None,
    /// Minimum GNU libc version.
    Glibc {
        /// Major version.
        major: u16,
        /// Minor version.
        minor: u16,
    },
    /// Minimum macOS deployment version.
    Macos {
        /// Major version.
        major: u16,
        /// Minor version.
        minor: u16,
    },
}
/// One target policy in PackConfig v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetPolicy {
    /// Canonical contract triple.
    pub target: String,
    /// Build strategy.
    pub strategy: BuildStrategy,
    /// Build host OS.
    pub host_os: HostOs,
    /// Build host architecture.
    pub host_arch: HostArch,
    /// Optional separate machine capability for qualification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualification_host: Option<HostRequirement>,
    /// Required compiler and optional cross-build tool versions.
    pub toolchain: ToolchainRequirement,
    /// Compatibility/deployment floor asserted by producer policy.
    pub floor: CompatibilityFloor,
    /// Qualification intent.
    pub qualification: Qualification,
    /// Release support tier.
    pub support: SupportTier,
}
/// Strict PackConfig v1 document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackConfig {
    /// Schema version exactly 1.
    pub schema_version: u32,
    /// Per-target producer policies.
    pub targets: Vec<TargetPolicy>,
}
/// Canonically ordered pure release planning result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleasePlan {
    /// Schema version exactly 1.
    pub schema_version: u32,
    /// Opaque release id.
    pub release_id: String,
    /// Opaque source revision.
    pub source_revision: String,
    /// Target plans sorted by canonical triple.
    pub targets: Vec<PlannedTarget>,
}
/// One selected target's canonical policy and contract-derived artifact form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTarget {
    /// Canonical contract triple.
    pub target: String,
    /// Producer policy resolved for this canonical target.
    pub policy: TargetPolicy,
    /// Artifact identity derived exclusively from the contract.
    pub artifact_form: PlannedAssetForm,
}
/// Contract-derived release artifact form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedAssetForm {
    /// One direct release artifact.
    Direct,
    /// Multiple sibling artifacts.
    Bundle,
    /// One archive artifact with required members.
    Archive,
}
impl ReleasePlan {
    /// Serialize the canonically ordered plan as deterministic compact JSON.
    pub fn to_json(&self) -> Result<String, CoreError> {
        if self.schema_version != 1
            || self.release_id.is_empty()
            || self.source_revision.is_empty()
            || self.targets.is_empty()
        {
            return Err(err("invalid ReleasePlan bounds"));
        }
        serde_json::to_string(self).map_err(|_| err("ReleasePlan serialization failed"))
    }
}
impl PackConfig {
    /// Parse strict TOML PackConfig v1.
    pub fn from_toml(s: &str) -> Result<Self, CoreError> {
        let c: Self = toml::from_str(s).map_err(|_| err("invalid PackConfig TOML"))?;
        if c.schema_version != 1 || c.targets.is_empty() {
            return Err(err("invalid PackConfig version or empty targets"));
        }
        let mut seen = std::collections::HashSet::new();
        if c.targets.iter().any(|p| {
            p.target.is_empty()
                || !seen.insert(&p.target)
                || p.toolchain.rust.is_empty()
                || p.toolchain.rust.len() > 64
                || (p.strategy == BuildStrategy::CargoZigbuild)
                    != p.toolchain.cargo_zigbuild.is_some()
        }) {
            return Err(err("invalid or duplicate PackConfig target"));
        }
        Ok(c)
    }
    /// Resolve selected targets against the distribution contract without side effects.
    pub fn resolve(
        &self,
        contract: &DistributionContract,
        release_id: &str,
        source_revision: &str,
        selected: &[String],
    ) -> Result<ReleasePlan, CoreError> {
        if self.schema_version != 1 {
            return Err(err("unsupported PackConfig version"));
        }
        let mut policies = BTreeMap::new();
        for p in &self.targets {
            let t = contract
                .resolve(&p.target)
                .map_err(|_| err("PackConfig contains unsupported target"))?;
            validate_policy(p, &t.triple)?;
            if policies.insert(t.triple.clone(), p).is_some() {
                return Err(err("duplicate canonical policy target"));
            }
        }
        let mut targets = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for s in selected {
            let t = contract
                .resolve(s)
                .map_err(|_| err("unknown selected target"))?;
            if !seen.insert(t.triple.clone()) {
                return Err(err("duplicate selected target"));
            }
            let mut policy = policies
                .get(&t.triple)
                .ok_or_else(|| err("selected target has no policy"))?
                .to_owned()
                .clone();
            policy.target = t.triple.clone();
            let artifact_form = match &t.asset {
                eggpack_contract::AssetForm::Direct { .. } => PlannedAssetForm::Direct,
                eggpack_contract::AssetForm::Bundle { .. } => PlannedAssetForm::Bundle,
                eggpack_contract::AssetForm::Archive { .. } => PlannedAssetForm::Archive,
            };
            targets.push(PlannedTarget {
                target: t.triple.clone(),
                policy,
                artifact_form,
            });
        }
        if targets.is_empty() {
            return Err(err("no selected targets"));
        }
        targets.sort_by(|a, b| a.target.cmp(&b.target));
        Ok(ReleasePlan {
            schema_version: 1,
            release_id: release_id.into(),
            source_revision: source_revision.into(),
            targets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pack_config_resolves_targets_in_canonical_order() {
        let contract = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let config = PackConfig {
            schema_version: 1,
            targets: vec![TargetPolicy {
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
            }],
        };
        let selected = vec!["linux-x64".into()];
        assert_eq!(
            config
                .resolve(&contract, "1.2.6", "abc", &selected)
                .unwrap()
                .targets[0]
                .target,
            "x86_64-unknown-linux-gnu"
        );
        assert!(config
            .resolve(&contract, "1.2.6", "abc", &["nope".into()])
            .is_err());
        let mut invalid = config.clone();
        invalid.targets[0].qualification = Qualification::Emulated;
        assert!(invalid
            .resolve(&contract, "1.2.6", "abc", &selected)
            .is_ok());
        invalid.targets[0].qualification = Qualification::Native;
        invalid.targets[0].host_arch = HostArch::Aarch64;
        assert!(invalid
            .resolve(&contract, "1.2.6", "abc", &selected)
            .is_err());
    }
    #[test]
    fn pack_config_toml_is_versioned_bounded_and_rejects_unknown_fields() {
        let source = r#"schema_version=1
[[targets]]
target="x86_64-unknown-linux-gnu"
strategy="native_cargo"
host_os="linux"
host_arch="x86_64"
toolchain={rust="1.89.0"}
floor={kind="none"}
qualification="native"
support="required"
"#;
        assert!(PackConfig::from_toml(source).is_ok());
        assert!(
            PackConfig::from_toml(&source.replace("support=", "unexpected=1\nsupport=")).is_err()
        );
        assert!(
            PackConfig::from_toml(&source.replace("schema_version=1", "schema_version=2")).is_err()
        );
    }
    #[test]
    fn manifest_builder_hashes_only_explicit_contract_paths() {
        let contract = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let dir = std::env::temp_dir().join(format!("eggpack-core-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let names = [
            "eggsact-1.2.6-x86_64-unknown-linux-gnu",
            "eggsact-1.2.6-x86_64-unknown-linux-gnu.sha256",
        ];
        let mut files = BTreeMap::new();
        for n in names {
            let p = dir.join(n);
            fs::write(&p, b"abc").unwrap();
            files.insert(n.to_string(), p);
        }
        let input = FinalizedReleaseInput {
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: "a".repeat(40),
            targets: vec![FinalizedTargetInput {
                target: "linux-x64".into(),
                release_files: files,
                archive_members: BTreeMap::new(),
            }],
            evidence_references: vec![],
        };
        let m = build_manifest(&contract, &input).unwrap();
        assert_eq!(m.targets[0].target, "x86_64-unknown-linux-gnu");
        assert_eq!(
            m.targets[0].form,
            ArtifactForm::Direct {
                artifact: ArtifactRecord {
                    name: names[0].into(),
                    size: 3,
                    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                        .into()
                },
                install: "eggsact".into()
            }
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn bundle_and_archive_inputs_preserve_contract_relationships() {
        for (fixture, release, product, target) in [
            (
                include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml"),
                "2.4.0",
                "codegg",
                "linux-x64",
            ),
            (
                include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml"),
                "3.1.0",
                "egress",
                "linux-x64",
            ),
        ] {
            let contract = DistributionContract::parse_toml_str(fixture).unwrap();
            let expected = expected_release_files(&contract, target, release).unwrap();
            let dir = std::env::temp_dir().join(format!(
                "eggpack-layout-test-{}-{product}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            let mut files = BTreeMap::new();
            for item in expected {
                let path = dir.join(&item.file_name);
                fs::write(&path, b"asset").unwrap();
                files.insert(item.file_name, path);
            }
            let expanded = contract.expand(target, release).unwrap();
            let members = match expanded.assets {
                ExpandedAssets::Archive(a) => a
                    .members
                    .into_iter()
                    .map(|m| {
                        let path = dir.join(format!("member-{}", m.source.replace('/', "_")));
                        fs::write(&path, b"member").unwrap();
                        (m.source, path)
                    })
                    .collect(),
                _ => BTreeMap::new(),
            };
            let input = FinalizedReleaseInput {
                product_id: product.into(),
                release_id: release.into(),
                source_revision: "c".repeat(40),
                targets: vec![FinalizedTargetInput {
                    target: target.into(),
                    release_files: files,
                    archive_members: members,
                }],
                evidence_references: vec![],
            };
            let manifest = build_manifest(&contract, &input).unwrap();
            match &manifest.targets[0].form {
                ArtifactForm::Bundle { .. } if product == "codegg" => (),
                ArtifactForm::Archive { members, .. } if product == "egress" => {
                    assert_eq!(members.len(), 2)
                }
                _ => panic!("contract form was not preserved"),
            }
            fs::remove_dir_all(dir).unwrap();
        }
    }
    #[test]
    fn manifest_builder_rejects_missing_or_extra_contract_inventory() {
        let contract = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let dir =
            std::env::temp_dir().join(format!("eggpack-core-negative-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut files = BTreeMap::new();
        for name in [
            "eggsact-1.2.6-x86_64-unknown-linux-gnu",
            "eggsact-1.2.6-x86_64-unknown-linux-gnu.sha256",
        ] {
            let p = dir.join(name);
            fs::write(&p, b"x").unwrap();
            files.insert(name.to_string(), p);
        }
        let mut input = FinalizedReleaseInput {
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: "a".repeat(40),
            targets: vec![FinalizedTargetInput {
                target: "linux-x64".into(),
                release_files: files,
                archive_members: BTreeMap::new(),
            }],
            evidence_references: vec![],
        };
        input.targets[0]
            .release_files
            .remove("eggsact-1.2.6-x86_64-unknown-linux-gnu.sha256");
        assert!(build_manifest(&contract, &input).is_err());
        input.targets[0]
            .release_files
            .insert("unexpected.file".into(), dir.join("unused"));
        assert!(build_manifest(&contract, &input).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
