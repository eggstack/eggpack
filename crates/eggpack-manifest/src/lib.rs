#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Bounded schema-v1 evidence for finalized Eggpack release bytes."]
#![doc = "This crate parses and serializes data only; it does not access files, networks, or trust authorities."]

use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

/// The only manifest schema version currently supported.
pub const SCHEMA_V1: u32 = 1;
/// Maximum encoded JSON document size.
pub const MAX_DOCUMENT_BYTES: usize = 1_048_576;
/// Maximum target records.
pub const MAX_TARGETS: usize = 256;
/// Maximum records in any bundle/archive member list.
pub const MAX_RECORDS: usize = 256;
/// Maximum evidence references per manifest.
pub const MAX_EVIDENCE_REFERENCES: usize = 64;
const MAX_ID: usize = 128;
const MAX_NAME: usize = 255;
const MAX_REVISION: usize = 128;
const MAX_EVIDENCE: usize = 256;

/// Validation, parsing, or serialization failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManifestError {
    /// Input is malformed or violates a structural/semantic invariant.
    Invalid(String),
    /// The manifest declares an unsupported schema version.
    UnsupportedVersion(u32),
    /// JSON exceeds the documented input bound.
    TooLarge,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "invalid release manifest: {s}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported manifest schema version: {v}"),
            Self::TooLarge => f.write_str("release manifest exceeds the 1 MiB limit"),
        }
    }
}
impl std::error::Error for ManifestError {}
fn invalid(s: impl Into<String>) -> ManifestError {
    ManifestError::Invalid(s.into())
}
fn bounded(s: &str, max: usize, label: &str) -> Result<(), ManifestError> {
    if s.is_empty() || s.len() > max || s.chars().any(char::is_control) {
        return Err(invalid(format!("invalid {label}")));
    }
    Ok(())
}

/// Top-level immutable evidence for one finalized product release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    /// Schema version, currently exactly `1`.
    pub schema_version: u32,
    /// Stable opaque product identifier.
    pub product_id: String,
    /// Opaque release identifier.
    pub release_id: String,
    /// Immutable source revision associated with the release.
    pub source_revision: String,
    /// Canonical target records, emitted in lexical triple order.
    pub targets: Vec<TargetRecord>,
    /// Optional bounded evidence identity strings; these make no trust claim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_references: Vec<String>,
}

/// Finalized artifact facts for one canonical target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRecord {
    /// Canonical target triple.
    pub target: String,
    /// Explicit direct, bundle, or archive representation.
    pub form: ArtifactForm,
}

/// Discriminated artifact layout. Nested records retain semantic relationships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactForm {
    /// One release artifact corresponding to one installed file.
    Direct {
        /// Artifact facts.
        artifact: ArtifactRecord,
        /// Installed filename.
        install: String,
    },
    /// Several independently downloadable files, each paired with its install identity.
    Bundle {
        /// Paired bundle entries.
        entries: Vec<BundleRecord>,
    },
    /// One archive artifact and its required logical members.
    Archive {
        /// Archive byte facts.
        artifact: ArtifactRecord,
        /// Required archive members.
        members: Vec<ArchiveMemberRecord>,
    },
}

/// Exact byte facts for one emitted artifact or logical member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    /// Safe flat release filename.
    pub name: String,
    /// Exact non-zero byte size.
    pub size: u64,
    /// Lowercase 64-character SHA-256 hexadecimal digest.
    pub sha256: String,
}

/// A bundle artifact and its corresponding install name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleRecord {
    /// Bundle artifact facts.
    pub artifact: ArtifactRecord,
    /// Safe flat install identity associated with this artifact.
    pub install: String,
}

/// A required archive member with source and install identities preserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMemberRecord {
    /// Normalized relative source path inside the archive.
    pub source: String,
    /// Safe flat install identity.
    pub install: String,
    /// Exact member byte facts.
    pub bytes: ByteEvidence,
}

/// Exact size and digest evidence for an archive member, whose identity is
/// supplied by its source and install fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteEvidence {
    /// Exact non-zero byte size.
    pub size: u64,
    /// Lowercase 64-character SHA-256 hexadecimal digest.
    pub sha256: String,
}

impl ReleaseManifest {
    /// Find exactly one canonical target record. This never resolves aliases or guesses.
    pub fn target(&self, canonical_triple: &str) -> Result<&TargetRecord, ManifestError> {
        self.validate()?;
        self.targets
            .iter()
            .find(|t| t.target == canonical_triple)
            .ok_or_else(|| invalid("canonical target not found"))
    }
    /// Parse strict schema-v1 JSON and validate all bounds and relationships.
    pub fn from_json(input: &str) -> Result<Self, ManifestError> {
        if input.len() > MAX_DOCUMENT_BYTES {
            return Err(ManifestError::TooLarge);
        }
        let value: Self = serde_json::from_str(input).map_err(|e| invalid(format!("JSON: {e}")))?;
        value.validate()?;
        Ok(value)
    }

    /// Validate a manifest without performing I/O.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != SCHEMA_V1 {
            return Err(ManifestError::UnsupportedVersion(self.schema_version));
        }
        bounded(&self.product_id, MAX_ID, "product id")?;
        bounded(&self.release_id, MAX_ID, "release id")?;
        bounded(&self.source_revision, MAX_REVISION, "source revision")?;
        if self.targets.is_empty() || self.targets.len() > MAX_TARGETS {
            return Err(invalid("target count out of bounds"));
        }
        if self.evidence_references.len() > MAX_EVIDENCE_REFERENCES {
            return Err(invalid("evidence reference count out of bounds"));
        }
        let mut target_seen = HashSet::new();
        let mut release_names = HashSet::new();
        for t in &self.targets {
            bounded(&t.target, MAX_ID, "target")?;
            if !target_seen.insert(t.target.as_str()) {
                return Err(invalid("duplicate canonical target"));
            }
            let mut installs = HashSet::new();
            match &t.form {
                ArtifactForm::Direct { artifact, install } => {
                    artifact.validate(&mut release_names)?;
                    valid_name(install, "install name", &mut installs)?;
                }
                ArtifactForm::Bundle { entries } => {
                    if entries.is_empty() || entries.len() > MAX_RECORDS {
                        return Err(invalid("bundle entry count out of bounds"));
                    }
                    let mut seen = HashSet::new();
                    for e in entries {
                        e.artifact.validate(&mut release_names)?;
                        valid_name(&e.install, "install name", &mut installs)?;
                        if !seen.insert(e.artifact.name.as_str()) {
                            return Err(invalid("duplicate bundle artifact"));
                        }
                    }
                }
                ArtifactForm::Archive { artifact, members } => {
                    artifact.validate(&mut release_names)?;
                    if members.is_empty() || members.len() > MAX_RECORDS {
                        return Err(invalid("archive member count out of bounds"));
                    }
                    let mut sources = HashSet::new();
                    for m in members {
                        valid_path(&m.source)?;
                        valid_name(&m.install, "install name", &mut installs)?;
                        m.bytes.validate()?;
                        if !sources.insert(m.source.as_str()) {
                            return Err(invalid("duplicate archive member source"));
                        }
                    }
                }
            }
        }
        for r in &self.evidence_references {
            bounded(r, MAX_EVIDENCE, "evidence reference")?;
        }
        Ok(())
    }

    /// Serialize as deterministic compact JSON with canonical lexical ordering.
    /// This is stable application serialization, not signing-grade canonical JSON.
    pub fn to_json(&self) -> Result<String, ManifestError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.targets.sort_by(|a, b| a.target.cmp(&b.target));
        for t in &mut canonical.targets {
            match &mut t.form {
                ArtifactForm::Bundle { entries } => entries.sort_by(|a, b| {
                    (&a.artifact.name, &a.install).cmp(&(&b.artifact.name, &b.install))
                }),
                ArtifactForm::Archive { members, .. } => {
                    members.sort_by(|a, b| (&a.source, &a.install).cmp(&(&b.source, &b.install)))
                }
                ArtifactForm::Direct { .. } => {}
            }
        }
        serde_json::to_string(&canonical).map_err(|e| invalid(format!("JSON serialization: {e}")))
    }
}

impl ArtifactRecord {
    /// Decode validated lowercase SHA-256 hexadecimal evidence into bytes.
    pub fn sha256_bytes(&self) -> Result<[u8; 32], ManifestError> {
        decode_sha256(&self.sha256)
    }
    fn validate(&self, names: &mut HashSet<String>) -> Result<(), ManifestError> {
        valid_name(&self.name, "artifact filename", names)?;
        if self.size == 0 {
            return Err(invalid("size must be non-zero"));
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "sha256 must be 64 lowercase hexadecimal characters",
            ));
        }
        Ok(())
    }
}
impl ByteEvidence {
    /// Decode validated lowercase SHA-256 hexadecimal evidence into bytes.
    pub fn sha256_bytes(&self) -> Result<[u8; 32], ManifestError> {
        decode_sha256(&self.sha256)
    }
    fn validate(&self) -> Result<(), ManifestError> {
        if self.size == 0 {
            return Err(invalid("size must be non-zero"));
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "sha256 must be 64 lowercase hexadecimal characters",
            ));
        }
        Ok(())
    }
}
fn decode_sha256(hex: &str) -> Result<[u8; 32], ManifestError> {
    if hex.len() != 64 {
        return Err(invalid("sha256 must contain 64 lowercase hex characters"));
    }
    let mut out = [0u8; 32];
    for (i, pair) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let digit = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        };
        out[i] = digit(pair[0])
            .and_then(|h| digit(pair[1]).map(|l| (h << 4) | l))
            .ok_or_else(|| invalid("invalid lowercase sha256"))?;
    }
    Ok(out)
}
fn valid_name(name: &str, label: &str, names: &mut HashSet<String>) -> Result<(), ManifestError> {
    bounded(name, MAX_NAME, label)?;
    if name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
    {
        return Err(invalid(format!("unsafe {label}")));
    }
    if !names.insert(name.to_ascii_lowercase()) {
        return Err(invalid(format!(
            "duplicate or ASCII-case-colliding {label}"
        )));
    }
    Ok(())
}
fn valid_path(p: &str) -> Result<(), ManifestError> {
    bounded(p, 1024, "archive member path")?;
    if p.starts_with('/')
        || p.contains('\\')
        || p.contains(':')
        || p.split('/').any(|c| c.is_empty() || c == "." || c == "..")
    {
        return Err(invalid("unsafe archive member path"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn artifact(name: &str) -> ArtifactRecord {
        ArtifactRecord {
            name: name.into(),
            size: 3,
            sha256: "ab".repeat(32),
        }
    }
    fn manifest(form: ArtifactForm) -> ReleaseManifest {
        ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".into(),
            release_id: "1.0.0".into(),
            source_revision: "a".repeat(40),
            targets: vec![TargetRecord {
                target: "x86_64-unknown-linux-gnu".into(),
                form,
            }],
            evidence_references: vec![],
        }
    }
    fn multi_target(forms: Vec<(&str, ArtifactForm)>) -> ReleaseManifest {
        let mut m = manifest(forms[0].1.clone());
        m.targets = forms
            .into_iter()
            .map(|(target, form)| TargetRecord {
                target: target.into(),
                form,
            })
            .collect();
        m
    }
    #[test]
    fn direct_round_trip_stable() {
        let m = manifest(ArtifactForm::Direct {
            artifact: artifact("eggsact"),
            install: "eggsact".into(),
        });
        let j = m.to_json().unwrap();
        let expected = format!(
            r#"{{"schema_version":1,"product_id":"eggsact","release_id":"1.0.0","source_revision":"{}","targets":[{{"target":"x86_64-unknown-linux-gnu","form":{{"kind":"direct","artifact":{{"name":"eggsact","size":3,"sha256":"{}"}},"install":"eggsact"}}}}]}}"#,
            "a".repeat(40),
            "ab".repeat(32)
        );
        assert_eq!(j, expected);
        assert_eq!(
            ReleaseManifest::from_json(&j).unwrap().to_json().unwrap(),
            j
        );
    }
    #[test]
    fn bundle_pairing_round_trip() {
        let m = manifest(ArtifactForm::Bundle {
            entries: vec![
                BundleRecord {
                    artifact: artifact("helper"),
                    install: "helper-bin".into(),
                },
                BundleRecord {
                    artifact: artifact("main"),
                    install: "main-bin".into(),
                },
            ],
        });
        assert_eq!(
            ReleaseManifest::from_json(&m.to_json().unwrap()).unwrap(),
            m
        );
    }
    #[test]
    fn archive_relationship_round_trip() {
        let m = manifest(ArtifactForm::Archive {
            artifact: artifact("release.tar"),
            members: vec![ArchiveMemberRecord {
                source: "bin/tool".into(),
                install: "tool".into(),
                bytes: ByteEvidence {
                    size: 3,
                    sha256: "ab".repeat(32),
                },
            }],
        });
        assert_eq!(
            ReleaseManifest::from_json(&m.to_json().unwrap()).unwrap(),
            m
        );
    }
    #[test]
    fn rejects_unknown_fields_version_and_bad_digest_or_size() {
        let m = manifest(ArtifactForm::Direct {
            artifact: artifact("a"),
            install: "a".into(),
        });
        let j = m.to_json().unwrap();
        assert!(ReleaseManifest::from_json(
            &j.replace("\"schema_version\":1", "\"schema_version\":2")
        )
        .is_err());
        assert!(ReleaseManifest::from_json(
            &j.replace("\"schema_version\":1", "\"schema_version\":1,\"extra\":0")
        )
        .is_err());
        assert!(ReleaseManifest::from_json(&j.replace(&"ab".repeat(32), "xx")).is_err());
        assert!(ReleaseManifest::from_json(&j.replace("\"size\":3", "\"size\":0")).is_err());
    }
    #[test]
    fn rejects_case_collision_and_crossed_install() {
        let mut m = manifest(ArtifactForm::Bundle {
            entries: vec![
                BundleRecord {
                    artifact: artifact("App"),
                    install: "one".into(),
                },
                BundleRecord {
                    artifact: artifact("app"),
                    install: "two".into(),
                },
            ],
        });
        assert!(m.validate().is_err());
        m = manifest(ArtifactForm::Bundle {
            entries: vec![
                BundleRecord {
                    artifact: artifact("one"),
                    install: "same".into(),
                },
                BundleRecord {
                    artifact: artifact("two"),
                    install: "same".into(),
                },
            ],
        });
        assert!(m.validate().is_err());
    }
    #[test]
    fn direct_install_names_are_target_local_like_eggsact_contract() {
        let m = multi_target(vec![
            (
                "x86_64-unknown-linux-gnu",
                ArtifactForm::Direct {
                    artifact: artifact("eggsact-linux"),
                    install: "eggsact".into(),
                },
            ),
            (
                "aarch64-apple-darwin",
                ArtifactForm::Direct {
                    artifact: artifact("eggsact-macos"),
                    install: "eggsact".into(),
                },
            ),
        ]);
        // Mirrors crates/eggpack-contract/tests/fixtures/simple-direct.toml.
        assert!(m.validate().is_ok());
    }
    #[test]
    fn archive_install_names_are_target_local_like_egress_contract() {
        let archive = |filename: &str| ArtifactForm::Archive {
            artifact: artifact(filename),
            members: vec![
                ArchiveMemberRecord {
                    source: "egress".into(),
                    install: "egress".into(),
                    bytes: ByteEvidence {
                        size: 3,
                        sha256: "ab".repeat(32),
                    },
                },
                ArchiveMemberRecord {
                    source: "bin/egress-helper".into(),
                    install: "egress-helper".into(),
                    bytes: ByteEvidence {
                        size: 4,
                        sha256: "cd".repeat(32),
                    },
                },
            ],
        };
        let m = multi_target(vec![
            ("x86_64-unknown-linux-gnu", archive("egress-linux.tar")),
            ("aarch64-apple-darwin", archive("egress-macos.tar")),
        ]);
        // Mirrors crates/eggpack-contract/tests/fixtures/egress-archive.toml.
        assert!(m.validate().is_ok());
    }
    #[test]
    fn bundle_install_names_are_target_local() {
        let bundle = |main: &str, helper: &str| ArtifactForm::Bundle {
            entries: vec![
                BundleRecord {
                    artifact: artifact(main),
                    install: "app".into(),
                },
                BundleRecord {
                    artifact: artifact(helper),
                    install: "app-helper".into(),
                },
            ],
        };
        let m = multi_target(vec![
            (
                "x86_64-unknown-linux-gnu",
                bundle("app-linux", "helper-linux"),
            ),
            ("aarch64-apple-darwin", bundle("app-macos", "helper-macos")),
        ]);
        assert!(m.validate().is_ok());
    }
    #[test]
    fn install_collisions_remain_rejected_within_target() {
        for installs in [["same", "same"], ["App", "app"]] {
            let m = manifest(ArtifactForm::Bundle {
                entries: vec![
                    BundleRecord {
                        artifact: artifact("one"),
                        install: installs[0].into(),
                    },
                    BundleRecord {
                        artifact: artifact("two"),
                        install: installs[1].into(),
                    },
                ],
            });
            assert!(m.validate().is_err());
        }

        for installs in [["same", "same"], ["App", "app"]] {
            let m = manifest(ArtifactForm::Archive {
                artifact: artifact("release.tar"),
                members: vec![
                    ArchiveMemberRecord {
                        source: "bin/one".into(),
                        install: installs[0].into(),
                        bytes: ByteEvidence {
                            size: 3,
                            sha256: "ab".repeat(32),
                        },
                    },
                    ArchiveMemberRecord {
                        source: "bin/two".into(),
                        install: installs[1].into(),
                        bytes: ByteEvidence {
                            size: 3,
                            sha256: "cd".repeat(32),
                        },
                    },
                ],
            });
            assert!(m.validate().is_err());
        }
    }
    #[test]
    fn release_artifact_collisions_remain_manifest_global() {
        for filenames in [["same", "same"], ["App.tar", "app.tar"]] {
            let m = multi_target(vec![
                (
                    "x86_64-unknown-linux-gnu",
                    ArtifactForm::Direct {
                        artifact: artifact(filenames[0]),
                        install: "app".into(),
                    },
                ),
                (
                    "aarch64-apple-darwin",
                    ArtifactForm::Direct {
                        artifact: artifact(filenames[1]),
                        install: "app".into(),
                    },
                ),
            ]);
            assert!(m.validate().is_err());
        }
    }
    #[test]
    fn exact_target_and_sha_helpers_are_non_wire_additions() {
        let m = manifest(ArtifactForm::Direct {
            artifact: artifact("a"),
            install: "a".into(),
        });
        let before = m.to_json().unwrap();
        let target = m.target("x86_64-unknown-linux-gnu").unwrap();
        if let ArtifactForm::Direct { artifact, .. } = &target.form {
            assert_eq!(artifact.sha256_bytes().unwrap(), [0xab; 32]);
        }
        assert!(m.target("linux-x64").is_err());
        assert_eq!(m.to_json().unwrap(), before);
    }
    #[test]
    fn eggup_interoperability_manifest_fixtures_are_valid_v1() {
        for fixture in [
            include_str!(
                "../../../plans/closure/eggup-interoperability/fixtures/direct-manifest.json"
            ),
            include_str!(
                "../../../plans/closure/eggup-interoperability/fixtures/bundle-manifest.json"
            ),
            include_str!(
                "../../../plans/closure/eggup-interoperability/fixtures/archive-manifest.json"
            ),
        ] {
            let parsed = ReleaseManifest::from_json(fixture).unwrap();
            assert_eq!(parsed.to_json().unwrap(), fixture.trim());
        }
        assert!(ReleaseManifest::from_json(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/unknown-schema.json"
        ))
        .is_err());
        assert!(ReleaseManifest::from_json(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/corrupt-digest.json"
        ))
        .is_err());
        assert!(ReleaseManifest::from_json(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/corrupt-size.json"
        ))
        .is_err());
        let direct = ReleaseManifest::from_json(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/direct-manifest.json"
        ))
        .unwrap();
        assert!(direct.target("x86_64-pc-windows-gnu").is_err());
        assert_eq!(direct.product_id, "eggsact");
        assert_eq!(direct.release_id, "1.2.6");
        let bundle = ReleaseManifest::from_json(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/bundle-manifest.json"
        ))
        .unwrap();
        match &bundle.target("x86_64-unknown-linux-gnu").unwrap().form {
            ArtifactForm::Bundle { entries } => assert_eq!(entries.len(), 3),
            _ => panic!("bundle fixture changed form"),
        }
        let archive = ReleaseManifest::from_json(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/archive-manifest.json"
        ))
        .unwrap();
        match &archive.target("x86_64-unknown-linux-gnu").unwrap().form {
            ArtifactForm::Archive { artifact, members } => {
                assert_eq!(artifact.size, 3);
                assert_eq!(members.len(), 2);
                assert_eq!(members[0].source, "bin/egress-helper");
            }
            _ => panic!("archive fixture changed form"),
        }
        let projection: serde_json::Value = serde_json::from_str(include_str!(
            "../../../plans/closure/eggup-interoperability/fixtures/projection-archive.json"
        ))
        .unwrap();
        assert_eq!(projection["extraction_required"], true);
        assert_eq!(projection["acquisition_units"].as_array().unwrap().len(), 1);
        assert_eq!(projection["members"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn canonical_order_is_independent_of_input_order() {
        let mut a = manifest(ArtifactForm::Bundle {
            entries: vec![
                BundleRecord {
                    artifact: artifact("z"),
                    install: "z".into(),
                },
                BundleRecord {
                    artifact: artifact("a"),
                    install: "a".into(),
                },
            ],
        });
        let mut b = a.clone();
        if let ArtifactForm::Bundle { entries } = &mut b.targets[0].form {
            entries.reverse();
        }
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
        a.targets.reverse();
    }
}
