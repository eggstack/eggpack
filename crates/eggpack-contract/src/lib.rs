#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Versioned distribution contract schema for release-time tooling."]
#![doc = ""]
#![doc = "A `DistributionContract` describes one product's release layout once:"]
#![doc = "target triples, platform aliases, asset file names, checksum sidecar"]
#![doc = "names, archive members, and install names. It does not fetch releases,"]
#![doc = "order versions, verify checksums, extract archives, generate installers,"]
#![doc = "or become a runtime dependency of ordinary consumers."]

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Schema version 1. The only version understood by this crate.
pub const SCHEMA_V1: u32 = 1;

/// Maximum lengths to keep diagnostics bounded.
const MAX_ID_LEN: usize = 64;
const MAX_NAME_LEN: usize = 128;
const MAX_TEMPLATE_LEN: usize = 256;
const MAX_DETAIL_LEN: usize = 512;
/// Maximum entries accepted in caller-supplied inventories and observations.
pub const MAX_OBSERVED_ENTRIES: usize = 256;
const MAX_CONFORMANCE_FINDINGS: usize = MAX_OBSERVED_ENTRIES * 2;

/// Namespace in which an expanded filename collision occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameNamespace {
    /// Release assets and checksum sidecars share one flat namespace.
    ReleaseFiles,
    /// Installed filenames share one flat namespace.
    InstallNames,
}

impl fmt::Display for NameNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReleaseFiles => f.write_str("release-file"),
            Self::InstallNames => f.write_str("install-name"),
        }
    }
}

/// Typed distribution errors. Parsing never guesses or falls back.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DistError {
    /// Input violates the schema contract.
    InvalidInput(String),
    /// The document declares an unsupported schema version.
    UnsupportedVersion {
        /// The declared version.
        found: u32,
    },
    /// Two targets share a canonical triple.
    DuplicateTarget(String),
    /// An alias collides with another alias or triple.
    DuplicateAlias(String),
    /// No target matches the requested triple or alias.
    UnknownTarget(String),
    /// A template contains an unknown placeholder.
    UnknownPlaceholder(String),
    /// A template requires an input that was not supplied.
    MissingInput(String),
    /// An archive member path is unsafe.
    InvalidMemberPath(String),
    /// Two logical filename fields collide after portable case folding.
    NameCollision {
        /// The filename namespace that contains the collision.
        namespace: NameNamespace,
        /// Bounded logical field label; no arbitrary filename is included.
        first: String,
        /// Bounded logical field label; no arbitrary filename is included.
        second: String,
    },
    /// A caller-supplied flat release inventory is invalid.
    InvalidReleaseInventory(String),
    /// A caller-supplied archive-member inventory is invalid.
    InvalidArchiveMemberInventory(String),
}

impl DistError {
    fn invalid(detail: impl Into<String>) -> Self {
        Self::InvalidInput(bound(detail.into()))
    }
}

impl fmt::Display for DistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(d) => write!(f, "invalid distribution contract: {d}"),
            Self::UnsupportedVersion { found } => {
                write!(f, "unsupported distribution schema version: {found}")
            }
            Self::DuplicateTarget(t) => write!(f, "duplicate distribution target: {t}"),
            Self::DuplicateAlias(a) => write!(f, "duplicate distribution alias: {a}"),
            Self::UnknownTarget(t) => write!(f, "unsupported distribution target: {t}"),
            Self::UnknownPlaceholder(p) => {
                write!(f, "unknown name-template placeholder: {p}")
            }
            Self::MissingInput(m) => write!(f, "missing template input: {m}"),
            Self::InvalidMemberPath(p) => write!(f, "unsafe archive member path: {p}"),
            Self::NameCollision {
                namespace,
                first,
                second,
            } => write!(
                f,
                "expanded {namespace} filename collision between {first} and {second}"
            ),
            Self::InvalidReleaseInventory(detail) => {
                write!(f, "invalid release inventory: {detail}")
            }
            Self::InvalidArchiveMemberInventory(detail) => {
                write!(f, "invalid archive member inventory: {detail}")
            }
        }
    }
}

impl std::error::Error for DistError {}

fn bound(mut s: String) -> String {
    if s.len() > MAX_DETAIL_LEN {
        s.truncate(MAX_DETAIL_LEN);
    }
    s
}

/// Opaque product identity. Version semantics remain caller-owned.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductIdentity {
    /// Opaque product id (for example `eggsact`). Used for `{product}` expansion.
    pub id: String,
    /// Optional human-readable display name. Never participates in expansion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Checksum sidecar naming. Integrity metadata only; never authenticity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecksumSpec {
    /// Sidecar file-name template. May use `{product}`, `{version}`,
    /// `{target}`, `{alias}`, and `{asset}` (the expanded asset file name).
    pub sidecar: String,
}

/// One bundle entry: a single asset file plus its install name.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleEntry {
    /// Release asset file-name template (flat, no directories).
    pub asset: String,
    /// Install/display file-name template (flat, no directories).
    pub install: String,
}

/// One archive member: a literal relative source path plus its install name.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMember {
    /// Literal relative path inside the archive. No templates; must be
    /// traversal-free (`a/b/c`, never `/abs`, `..`, or empty components).
    pub source: String,
    /// Install/display file-name template (flat, no directories).
    pub install: String,
}

/// Asset layout for one target. A small tagged enum, not a generic package language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetForm {
    /// One release asset corresponds to one member.
    Direct {
        /// Release asset file-name template.
        asset: String,
        /// Install file-name template.
        install: String,
    },
    /// Multiple release assets together form one logical release.
    Bundle {
        /// Each direct asset in the bundle.
        entries: Vec<BundleEntry>,
    },
    /// One release asset contains multiple required members.
    Archive {
        /// Archive file-name template.
        asset: String,
        /// Required members inside the archive.
        members: Vec<ArchiveMember>,
    },
}

/// One supported target entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetEntry {
    /// Canonical Rust target triple.
    pub triple: String,
    /// Zero or more consumer/platform aliases (globally unambiguous).
    pub aliases: Vec<String>,
    /// Asset mapping for this target.
    pub asset: AssetForm,
    /// Checksum sidecar naming for this target.
    pub checksum: ChecksumSpec,
}

/// A versioned distribution contract (schema v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionContract {
    /// Always [`SCHEMA_V1`].
    pub schema_version: u32,
    /// Opaque product identity.
    pub product: ProductIdentity,
    /// Supported targets (non-empty, deterministic order as written).
    pub targets: Vec<TargetEntry>,
}

// ---- Raw TOML shapes (deny unknown fields for typo safety) ----

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContract {
    schema_version: u32,
    product: ProductIdentity,
    targets: Vec<RawTarget>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    triple: String,
    #[serde(default)]
    aliases: Vec<String>,
    asset: RawAsset,
    checksum: ChecksumSpec,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAsset {
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    asset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    install: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<BundleEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    members: Option<Vec<ArchiveMember>>,
}

// ---- Parsing and validation ----

impl DistributionContract {
    /// Parses and structurally validates a TOML distribution contract.
    ///
    /// Returns typed validation errors; never guesses, falls back, touches
    /// the network, or rewrites the source. Unknown future schema versions
    /// fail with [`DistError::UnsupportedVersion`].
    pub fn parse_toml_str(s: &str) -> Result<Self, DistError> {
        let raw: RawContract =
            toml::from_str(s).map_err(|e| DistError::invalid(format!("TOML parse: {e}")))?;
        Self::from_raw(raw)
    }

    /// Serializes the contract deterministically to TOML.
    ///
    /// Field and target order are preserved as stored, so golden outputs are
    /// stable for a given contract value.
    pub fn to_toml_string(&self) -> Result<String, DistError> {
        let raw = self.to_raw();
        toml::to_string(&raw).map_err(|e| DistError::invalid(format!("TOML render: {e}")))
    }

    /// Resolves a canonical triple or alias to its target entry.
    ///
    /// Aliases are globally unambiguous; unknown inputs fail with
    /// [`DistError::UnknownTarget`] instead of guessing a nearby architecture.
    pub fn resolve(&self, target_or_alias: &str) -> Result<&TargetEntry, DistError> {
        for t in &self.targets {
            if t.triple == target_or_alias {
                return Ok(t);
            }
            if t.aliases.iter().any(|a| a == target_or_alias) {
                return Ok(t);
            }
        }
        Err(DistError::UnknownTarget(bound(target_or_alias.to_string())))
    }

    /// Expands all file names for one target and opaque version string.
    ///
    /// `version` is opaque (no SemVer ordering) but must be filesystem-safe
    /// (non-empty, no `/`, `\\`, or control characters). `{alias}` templates
    /// require looking up via that alias; looking up via the triple when the
    /// template needs `{alias}` fails with [`DistError::MissingInput`]. The
    /// expanded result is rejected if any release or install filename
    /// collides, including ASCII case-only collisions.
    pub fn expand(
        &self,
        target_or_alias: &str,
        version: &str,
    ) -> Result<ExpandedTarget, DistError> {
        validate_version(version)?;
        let target = self.resolve(target_or_alias)?;
        // `{alias}` expands to the alias used for lookup, when the lookup was
        // via an alias. A triple lookup leaves alias empty.
        let alias_used: Option<&str> = if target.triple == target_or_alias {
            None
        } else {
            Some(target_or_alias)
        };
        let ctx = ExpansionCtx {
            product: &self.product.id,
            version,
            target: &target.triple,
            alias: alias_used,
        };
        let assets = match &target.asset {
            AssetForm::Direct { asset, install } => {
                let asset_file = expand_template(asset, &ctx, None)?;
                let install_name = expand_template(install, &ctx, None)?;
                let sidecar_file =
                    expand_template(&target.checksum.sidecar, &ctx, Some(&asset_file))?;
                ExpandedAssets::Direct(ExpandedDirect {
                    asset_file,
                    install_name,
                    sidecar_file,
                })
            }
            AssetForm::Bundle { entries } => {
                let mut out = Vec::with_capacity(entries.len());
                for e in entries {
                    let asset_file = expand_template(&e.asset, &ctx, None)?;
                    let install_name = expand_template(&e.install, &ctx, None)?;
                    let sidecar_file =
                        expand_template(&target.checksum.sidecar, &ctx, Some(&asset_file))?;
                    out.push(ExpandedDirect {
                        asset_file,
                        install_name,
                        sidecar_file,
                    });
                }
                ExpandedAssets::Bundle(ExpandedBundle { entries: out })
            }
            AssetForm::Archive { asset, members } => {
                let archive_file = expand_template(asset, &ctx, None)?;
                let sidecar_file =
                    expand_template(&target.checksum.sidecar, &ctx, Some(&archive_file))?;
                let mut expanded_members = Vec::with_capacity(members.len());
                for m in members {
                    let install_name = expand_template(&m.install, &ctx, None)?;
                    expanded_members.push(ExpandedArchiveMember {
                        source: m.source.clone(),
                        install_name,
                    });
                }
                ExpandedAssets::Archive(ExpandedArchive {
                    archive_file,
                    sidecar_file,
                    members: expanded_members,
                })
            }
        };
        let expanded = ExpandedTarget {
            triple: target.triple.clone(),
            assets,
        };
        validate_expanded_names(&expanded)?;
        Ok(expanded)
    }

    fn from_raw(raw: RawContract) -> Result<Self, DistError> {
        if raw.schema_version != SCHEMA_V1 {
            return Err(DistError::UnsupportedVersion {
                found: raw.schema_version,
            });
        }
        validate_product_id(&raw.product.id)?;
        if let Some(display) = &raw.product.display_name {
            validate_display_name(display)?;
        }
        if raw.targets.is_empty() {
            return Err(DistError::invalid("contract declares no targets"));
        }
        let all_triples: HashSet<&str> = raw.targets.iter().map(|t| t.triple.as_str()).collect();
        // Duplicate triples fail without guessing.
        {
            let mut seen: HashSet<&str> = HashSet::new();
            for t in &raw.targets {
                if !seen.insert(t.triple.as_str()) {
                    return Err(DistError::DuplicateTarget(bound(t.triple.clone())));
                }
            }
        }
        // Aliases must be valid, must not collide with any triple (even their
        // own) to keep triple-vs-alias resolution unambiguous, and must not
        // map one alias to different targets.
        let mut alias_owner: HashMap<&str, &str> = HashMap::new();
        for t in &raw.targets {
            validate_triple(&t.triple)?;
            for a in &t.aliases {
                validate_alias(a)?;
                if all_triples.contains(a.as_str()) {
                    return Err(DistError::DuplicateAlias(bound(format!(
                        "{a} collides with a target triple"
                    ))));
                }
                if let Some(owner) = alias_owner.get(a.as_str()) {
                    if *owner != t.triple.as_str() {
                        return Err(DistError::DuplicateAlias(bound(a.clone())));
                    }
                } else {
                    alias_owner.insert(a.as_str(), t.triple.as_str());
                }
            }
        }
        let mut out_targets = Vec::with_capacity(raw.targets.len());
        for t in raw.targets {
            let asset = convert_asset(t.asset)?;
            validate_checksum_template(&asset, &t.checksum.sidecar)?;
            out_targets.push(TargetEntry {
                triple: t.triple,
                aliases: t.aliases,
                asset,
                checksum: t.checksum,
            });
        }
        Ok(Self {
            schema_version: SCHEMA_V1,
            product: raw.product,
            targets: out_targets,
        })
    }

    fn to_raw(&self) -> RawContract {
        let targets = self
            .targets
            .iter()
            .map(|t| {
                let (kind, asset, install, entries, members) = match &t.asset {
                    AssetForm::Direct { asset, install } => (
                        "direct".to_string(),
                        Some(asset.clone()),
                        Some(install.clone()),
                        None,
                        None,
                    ),
                    AssetForm::Bundle { entries } => (
                        "bundle".to_string(),
                        None,
                        None,
                        Some(entries.clone()),
                        None,
                    ),
                    AssetForm::Archive { asset, members } => (
                        "archive".to_string(),
                        Some(asset.clone()),
                        None,
                        None,
                        Some(members.clone()),
                    ),
                };
                RawTarget {
                    triple: t.triple.clone(),
                    aliases: t.aliases.clone(),
                    asset: RawAsset {
                        kind,
                        asset,
                        install,
                        entries,
                        members,
                    },
                    checksum: t.checksum.clone(),
                }
            })
            .collect();
        RawContract {
            schema_version: self.schema_version,
            product: self.product.clone(),
            targets,
        }
    }
}

// ---- Asset conversion ----

fn convert_asset(raw: RawAsset) -> Result<AssetForm, DistError> {
    match raw.kind.as_str() {
        "direct" => {
            let asset = raw
                .asset
                .ok_or_else(|| DistError::invalid("direct asset requires `asset`"))?;
            let install = raw
                .install
                .ok_or_else(|| DistError::invalid("direct asset requires `install`"))?;
            if raw.entries.is_some() || raw.members.is_some() {
                return Err(DistError::invalid(
                    "direct asset must not declare `entries` or `members`",
                ));
            }
            validate_file_template(&asset, false)?;
            validate_file_template(&install, false)?;
            Ok(AssetForm::Direct { asset, install })
        }
        "bundle" => {
            let entries = raw
                .entries
                .ok_or_else(|| DistError::invalid("bundle asset requires `entries`"))?;
            if raw.asset.is_some() || raw.install.is_some() || raw.members.is_some() {
                return Err(DistError::invalid(
                    "bundle asset must declare only `entries`",
                ));
            }
            if entries.is_empty() {
                return Err(DistError::invalid("bundle asset declares no entries"));
            }
            if entries.len() > 64 {
                return Err(DistError::invalid("bundle asset declares too many entries"));
            }
            for e in &entries {
                validate_file_template(&e.asset, false)?;
                validate_file_template(&e.install, false)?;
            }
            ensure_unique_raw_templates(
                NameNamespace::ReleaseFiles,
                entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (e.asset.as_str(), format!("entries[{i}].asset"))),
            )?;
            ensure_unique_raw_templates(
                NameNamespace::InstallNames,
                entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (e.install.as_str(), format!("entries[{i}].install"))),
            )?;
            Ok(AssetForm::Bundle { entries })
        }
        "archive" => {
            let asset = raw
                .asset
                .ok_or_else(|| DistError::invalid("archive asset requires `asset`"))?;
            let members = raw
                .members
                .ok_or_else(|| DistError::invalid("archive asset requires `members`"))?;
            if raw.install.is_some() || raw.entries.is_some() {
                return Err(DistError::invalid(
                    "archive asset must declare only `asset` and `members`",
                ));
            }
            validate_file_template(&asset, false)?;
            if members.is_empty() {
                return Err(DistError::invalid("archive asset declares no members"));
            }
            if members.len() > 64 {
                return Err(DistError::invalid(
                    "archive asset declares too many members",
                ));
            }
            let mut seen: HashSet<&str> = HashSet::new();
            for m in &members {
                validate_member_source(&m.source)?;
                validate_file_template(&m.install, false)?;
                if !seen.insert(m.source.as_str()) {
                    return Err(DistError::invalid(format!(
                        "duplicate archive member: {}",
                        m.source
                    )));
                }
            }
            ensure_unique_raw_templates(
                NameNamespace::InstallNames,
                members
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (m.install.as_str(), format!("members[{i}].install"))),
            )?;
            Ok(AssetForm::Archive { asset, members })
        }
        other => Err(DistError::invalid(format!(
            "unknown asset kind: {other} (expected direct|bundle|archive)"
        ))),
    }
}

// ---- Validation helpers ----

fn is_safe_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

fn validate_product_id(id: &str) -> Result<(), DistError> {
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return Err(DistError::invalid("product id is empty or overlong"));
    }
    if id.chars().any(|c| c.is_control()) {
        return Err(DistError::invalid("product id has control characters"));
    }
    if !id.chars().all(is_safe_name_char) {
        return Err(DistError::invalid(
            "product id must use [A-Za-z0-9-_.] only",
        ));
    }
    if id == "." || id == ".." {
        return Err(DistError::invalid("product id must name a product"));
    }
    Ok(())
}

fn validate_display_name(name: &str) -> Result<(), DistError> {
    if name.is_empty() || name.len() > 256 {
        return Err(DistError::invalid("display name is empty or overlong"));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(DistError::invalid("display name has control characters"));
    }
    Ok(())
}

fn validate_triple(triple: &str) -> Result<(), DistError> {
    if triple.is_empty() || triple.len() > MAX_NAME_LEN {
        return Err(DistError::invalid("target triple is empty or overlong"));
    }
    if triple.chars().any(|c| c.is_control()) {
        return Err(DistError::invalid("target triple has control characters"));
    }
    if triple.contains(['/', '\\', ' ', '@', '?', '#', ':']) {
        return Err(DistError::invalid(
            "target triple must be a bare triple without separators",
        ));
    }
    if !triple.chars().all(is_safe_name_char) {
        return Err(DistError::invalid(
            "target triple must use [A-Za-z0-9-_.] only",
        ));
    }
    if !triple.contains('-') {
        return Err(DistError::invalid(
            "target triple must contain '-' (canonical Rust triple)",
        ));
    }
    Ok(())
}

fn validate_alias(alias: &str) -> Result<(), DistError> {
    if alias.is_empty() || alias.len() > MAX_NAME_LEN {
        return Err(DistError::invalid("alias is empty or overlong"));
    }
    if alias.chars().any(|c| c.is_control()) {
        return Err(DistError::invalid("alias has control characters"));
    }
    if alias.contains(['/', '\\', ' ', '@', '?', '#', ':']) {
        return Err(DistError::invalid("alias must not contain separators"));
    }
    if !alias.chars().all(is_safe_name_char) {
        return Err(DistError::invalid("alias must use [A-Za-z0-9-_.] only"));
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<(), DistError> {
    if version.is_empty() || version.len() > MAX_NAME_LEN {
        return Err(DistError::invalid(
            "version is empty or overlong (opaque but filesystem-safe)",
        ));
    }
    if version.chars().any(|c| c.is_control()) {
        return Err(DistError::invalid("version has control characters"));
    }
    if version.contains(['/', '\\']) {
        return Err(DistError::invalid(
            "version must not contain path separators",
        ));
    }
    Ok(())
}

/// Validates a flat file-name template (asset, install, or sidecar).
///
/// `allow_asset` permits the `{asset}` placeholder (checksum sidecars only).
/// Rejects path separators, control characters, and unknown placeholders.
/// Archive member sources are literals and validated separately.
fn validate_file_template(template: &str, allow_asset: bool) -> Result<(), DistError> {
    if template.is_empty() || template.len() > MAX_TEMPLATE_LEN {
        return Err(DistError::invalid("name template is empty or overlong"));
    }
    if template.chars().any(|c| c.is_control()) {
        return Err(DistError::invalid("name template has control characters"));
    }
    if template.contains(['/', '\\']) {
        return Err(DistError::invalid(
            "name template must be a flat file name without directories",
        ));
    }
    parse_template(template, allow_asset)?;
    Ok(())
}

fn validate_checksum_template(_asset: &AssetForm, sidecar: &str) -> Result<(), DistError> {
    validate_file_template(sidecar, true)
}

/// Validates a literal archive member source path (no templates).
fn validate_member_source(source: &str) -> Result<(), DistError> {
    if source.is_empty() || source.len() > MAX_TEMPLATE_LEN {
        return Err(DistError::InvalidMemberPath(bound(
            "member path is empty or overlong".to_string(),
        )));
    }
    if source.chars().any(|c| c.is_control()) {
        return Err(DistError::InvalidMemberPath(bound(
            "member path has control characters".to_string(),
        )));
    }
    if source.contains('{') || source.contains('}') {
        return Err(DistError::InvalidMemberPath(bound(
            "member source must be literal (no templates)".to_string(),
        )));
    }
    if source.contains('\\') {
        return Err(DistError::InvalidMemberPath(bound(
            "member path must use '/' separators".to_string(),
        )));
    }
    if source.starts_with('/') {
        return Err(DistError::InvalidMemberPath(bound(format!(
            "absolute member path: {source}"
        ))));
    }
    if source.contains(':') {
        return Err(DistError::InvalidMemberPath(bound(format!(
            "member path must not contain drive prefixes: {source}"
        ))));
    }
    for comp in source.split('/') {
        if comp.is_empty() {
            return Err(DistError::InvalidMemberPath(bound(format!(
                "empty member path component: {source}"
            ))));
        }
        if comp == "." || comp == ".." {
            return Err(DistError::InvalidMemberPath(bound(format!(
                "member path escapes root: {source}"
            ))));
        }
    }
    Ok(())
}

// ---- Template expansion ----

#[derive(Debug, Clone, Copy)]
enum Placeholder {
    Product,
    Version,
    Target,
    Alias,
    Asset,
}

enum TemplatePart<'a> {
    Literal(&'a str),
    Placeholder(Placeholder),
}

/// Parses the complete v1 grammar. Braces are reserved exclusively for exact
/// placeholder names; there is no escaping, nesting, or deferred syntax.
fn parse_template(template: &str, allow_asset: bool) -> Result<Vec<TemplatePart<'_>>, DistError> {
    let bytes = template.as_bytes();
    let mut parts = Vec::new();
    let mut cursor = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                if cursor < i {
                    parts.push(TemplatePart::Literal(&template[cursor..i]));
                }
                let Some(relative_end) = template[i + 1..].find('}') else {
                    return Err(DistError::invalid(
                        "unmatched opening brace in name template",
                    ));
                };
                let end = i + 1 + relative_end;
                let name = &template[i + 1..end];
                if name.is_empty() {
                    return Err(DistError::invalid("empty placeholder in name template"));
                }
                if name.contains(['{', '}']) {
                    return Err(DistError::invalid("nested braces in name template"));
                }
                if !name.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    return Err(DistError::invalid("malformed placeholder in name template"));
                }
                let placeholder = match name {
                    "product" => Placeholder::Product,
                    "version" => Placeholder::Version,
                    "target" => Placeholder::Target,
                    "alias" => Placeholder::Alias,
                    "asset" if allow_asset => Placeholder::Asset,
                    _ => return Err(DistError::UnknownPlaceholder(bound(name.to_string()))),
                };
                parts.push(TemplatePart::Placeholder(placeholder));
                i = end + 1;
                cursor = i;
            }
            b'}' => return Err(DistError::invalid("stray closing brace in name template")),
            byte => {
                if !is_safe_name_char(char::from(byte)) {
                    return Err(DistError::invalid(
                        "name template literal must use [A-Za-z0-9-_.] only",
                    ));
                }
                i += 1;
            }
        }
    }
    if cursor < bytes.len() {
        parts.push(TemplatePart::Literal(&template[cursor..]));
    }
    Ok(parts)
}

struct ExpansionCtx<'a> {
    product: &'a str,
    version: &'a str,
    target: &'a str,
    alias: Option<&'a str>,
}

fn expand_template(
    template: &str,
    ctx: &ExpansionCtx<'_>,
    asset: Option<&str>,
) -> Result<String, DistError> {
    let parts = parse_template(template, asset.is_some())?;
    let mut out = String::with_capacity(template.len());
    for part in parts {
        match part {
            TemplatePart::Literal(literal) => out.push_str(literal),
            TemplatePart::Placeholder(Placeholder::Product) => out.push_str(ctx.product),
            TemplatePart::Placeholder(Placeholder::Version) => out.push_str(ctx.version),
            TemplatePart::Placeholder(Placeholder::Target) => out.push_str(ctx.target),
            TemplatePart::Placeholder(Placeholder::Alias) => {
                let alias = ctx.alias.ok_or_else(|| {
                    DistError::MissingInput(bound(
                        "`{alias}` requires lookup via that alias".to_string(),
                    ))
                })?;
                out.push_str(alias);
            }
            TemplatePart::Placeholder(Placeholder::Asset) => {
                let asset = asset.ok_or_else(|| {
                    DistError::MissingInput(bound(
                        "`{asset}` requires a checksum context".to_string(),
                    ))
                })?;
                out.push_str(asset);
            }
        }
    }
    if out.is_empty() || out.len() > MAX_TEMPLATE_LEN {
        return Err(DistError::invalid("expanded name is empty or overlong"));
    }
    if out.contains(['{', '}']) {
        return Err(DistError::invalid("expanded name contains reserved braces"));
    }
    if out.contains(['/', '\\']) {
        return Err(DistError::invalid(
            "expanded name must remain a flat file name",
        ));
    }
    Ok(out)
}

fn ensure_unique_names(
    namespace: NameNamespace,
    names: impl IntoIterator<Item = (String, String)>,
) -> Result<(), DistError> {
    let mut seen: HashMap<String, String> = HashMap::new();
    for (name, field) in names {
        let key = name.to_ascii_lowercase();
        if let Some(first) = seen.get(&key) {
            return Err(DistError::NameCollision {
                namespace,
                first: bound(first.clone()),
                second: bound(field),
            });
        }
        seen.insert(key, field);
    }
    Ok(())
}

fn ensure_unique_raw_templates<'a>(
    namespace: NameNamespace,
    templates: impl IntoIterator<Item = (&'a str, String)>,
) -> Result<(), DistError> {
    ensure_unique_names(
        namespace,
        templates
            .into_iter()
            .map(|(template, field)| (template.to_string(), field)),
    )
}

fn validate_expanded_names(target: &ExpandedTarget) -> Result<(), DistError> {
    let mut release_names = Vec::new();
    let mut install_names = Vec::new();
    match &target.assets {
        ExpandedAssets::Direct(direct) => {
            release_names.push((direct.asset_file.clone(), "asset_file".to_string()));
            release_names.push((direct.sidecar_file.clone(), "sidecar_file".to_string()));
            install_names.push((direct.install_name.clone(), "install_name".to_string()));
        }
        ExpandedAssets::Bundle(bundle) => {
            for (index, entry) in bundle.entries.iter().enumerate() {
                release_names.push((
                    entry.asset_file.clone(),
                    format!("entries[{index}].asset_file"),
                ));
                release_names.push((
                    entry.sidecar_file.clone(),
                    format!("entries[{index}].sidecar_file"),
                ));
                install_names.push((
                    entry.install_name.clone(),
                    format!("entries[{index}].install_name"),
                ));
            }
        }
        ExpandedAssets::Archive(archive) => {
            release_names.push((archive.archive_file.clone(), "archive_file".to_string()));
            release_names.push((archive.sidecar_file.clone(), "sidecar_file".to_string()));
            for (index, member) in archive.members.iter().enumerate() {
                install_names.push((
                    member.install_name.clone(),
                    format!("members[{index}].install_name"),
                ));
            }
        }
    }
    ensure_unique_names(NameNamespace::ReleaseFiles, release_names)?;
    ensure_unique_names(NameNamespace::InstallNames, install_names)
}

// ---- Expanded model ----

/// One expanded direct asset (file, install name, and sidecar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedDirect {
    /// Expanded release asset file name.
    pub asset_file: String,
    /// Expanded install/display file name.
    pub install_name: String,
    /// Expanded checksum sidecar file name.
    pub sidecar_file: String,
}

/// Expanded multi-asset bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedBundle {
    /// Each entry in the bundle.
    pub entries: Vec<ExpandedDirect>,
}

/// One expanded archive member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedArchiveMember {
    /// Literal source path inside the archive.
    pub source: String,
    /// Expanded install/display file name.
    pub install_name: String,
}

/// Expanded archive asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedArchive {
    /// Expanded archive file name.
    pub archive_file: String,
    /// Expanded checksum sidecar file name.
    pub sidecar_file: String,
    /// Required members.
    pub members: Vec<ExpandedArchiveMember>,
}

/// Expanded asset forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandedAssets {
    /// Single direct asset.
    Direct(ExpandedDirect),
    /// Multi-asset bundle.
    Bundle(ExpandedBundle),
    /// Archive with members.
    Archive(ExpandedArchive),
}

/// A resolved and expanded target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedTarget {
    /// Canonical triple.
    pub triple: String,
    /// Expanded file names.
    pub assets: ExpandedAssets,
}

/// Caller policy for observed names that the contract does not require.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExtrasPolicy {
    /// Permit unrelated release files or archive members.
    #[default]
    AllowExtras,
    /// Require the observed set to contain only names declared by the contract.
    Exact,
}

/// One required flat release file and its stable diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct ExpectedReleaseFile {
    /// Stable contract-field label such as `asset`, `sidecar`, or `entries[0].asset`.
    pub label: String,
    /// Exact release inventory filename.
    pub file_name: String,
}

/// Derive the required release file names for a concrete target and version.
///
/// Target aliases are resolved using the contract's normal expansion rules.
/// Results are ordered by contract declaration and stable within each asset
/// form. This function does not inspect a release or touch the network.
pub fn expected_release_files(
    contract: &DistributionContract,
    target_or_alias: &str,
    version: &str,
) -> Result<Vec<ExpectedReleaseFile>, DistError> {
    let expanded = contract.expand(target_or_alias, version)?;
    let mut files = Vec::new();
    match expanded.assets {
        ExpandedAssets::Direct(direct) => {
            files.push(expected_file("asset", direct.asset_file));
            files.push(expected_file("sidecar", direct.sidecar_file));
        }
        ExpandedAssets::Bundle(bundle) => {
            for (index, entry) in bundle.entries.into_iter().enumerate() {
                files.push(expected_file(
                    &format!("entries[{index}].asset"),
                    entry.asset_file,
                ));
                files.push(expected_file(
                    &format!("entries[{index}].sidecar"),
                    entry.sidecar_file,
                ));
            }
        }
        ExpandedAssets::Archive(archive) => {
            files.push(expected_file("archive", archive.archive_file));
            files.push(expected_file("sidecar", archive.sidecar_file));
        }
    }
    Ok(files)
}

fn expected_file(label: &str, file_name: String) -> ExpectedReleaseFile {
    ExpectedReleaseFile {
        label: label.to_string(),
        file_name,
    }
}

/// Bounded, validated list of flat names supplied by a release client or fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInventory {
    files: Vec<String>,
}

impl ReleaseInventory {
    /// Construct an inventory from flat release filenames.
    ///
    /// Empty, overlong, control/separator-containing, exact-duplicate, and
    /// ASCII-case-colliding names are rejected. Names are sorted so reports
    /// do not depend on provider response order.
    pub fn new<I, S>(files: I) -> Result<Self, DistError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut names = Vec::new();
        let mut seen = HashSet::new();
        for value in files {
            if names.len() == MAX_OBSERVED_ENTRIES {
                return Err(DistError::InvalidReleaseInventory(bound(format!(
                    "more than {MAX_OBSERVED_ENTRIES} entries"
                ))));
            }
            let name = value.into();
            validate_observed_flat_name(&name)
                .map_err(|detail| DistError::InvalidReleaseInventory(bound(detail)))?;
            let key = name.to_ascii_lowercase();
            if !seen.insert(key) {
                return Err(DistError::InvalidReleaseInventory(bound(format!(
                    "duplicate or ASCII-case-colliding filename: {name}"
                ))));
            }
            names.push(name);
        }
        names.sort();
        Ok(Self { files: names })
    }

    /// The validated filenames in deterministic order.
    pub fn files(&self) -> &[String] {
        &self.files
    }
}

fn validate_observed_flat_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_TEMPLATE_LEN {
        return Err("filename is empty or overlong".to_string());
    }
    if name.chars().any(|c| c.is_control()) {
        return Err("filename contains control characters".to_string());
    }
    if name.contains(['/', '\\']) {
        return Err("filename must be flat and contain no path separators".to_string());
    }
    Ok(())
}

/// Validate release completeness for an expanded target.
///
/// Required files are always errors when missing. Unrelated files are ignored
/// by [`ExtrasPolicy::AllowExtras`] and reported by [`ExtrasPolicy::Exact`].
pub fn validate_release_inventory(
    expected: &[ExpectedReleaseFile],
    observed: &ReleaseInventory,
    extras: ExtrasPolicy,
) -> ConformanceReport {
    if expected.len() > MAX_OBSERVED_ENTRIES
        || expected.iter().any(|file| {
            file.label.is_empty()
                || file.label.len() > MAX_DETAIL_LEN
                || validate_observed_flat_name(&file.file_name).is_err()
        })
    {
        return ConformanceReport::new(vec![ConformanceFinding::new(
            FindingKind::InvalidObservation,
            "expected release files",
            None,
            Some("invalid or over limit"),
        )]);
    }
    let mut expected_names = HashSet::new();
    if expected
        .iter()
        .any(|file| !expected_names.insert(file.file_name.to_ascii_lowercase()))
    {
        return ConformanceReport::new(vec![ConformanceFinding::new(
            FindingKind::InvalidObservation,
            "expected release files",
            None,
            Some("duplicate or ASCII-case-colliding expected filename"),
        )]);
    }
    let required: HashSet<&str> = expected.iter().map(|f| f.file_name.as_str()).collect();
    let observed_set: HashSet<&str> = observed.files.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for file in expected {
        if !observed_set.contains(file.file_name.as_str()) {
            findings.push(ConformanceFinding::new(
                FindingKind::MissingReleaseFile,
                &file.label,
                Some(&file.file_name),
                None,
            ));
        }
    }
    if extras == ExtrasPolicy::Exact {
        for name in &observed.files {
            if !required.contains(name.as_str()) {
                findings.push(ConformanceFinding::new(
                    FindingKind::UnexpectedReleaseFile,
                    "release inventory",
                    None,
                    Some(name),
                ));
            }
        }
    }
    ConformanceReport::new(findings)
}

/// Bounded, validated archive member paths supplied by a listing tool or fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveMemberInventory {
    members: Vec<String>,
}

impl ArchiveMemberInventory {
    /// Construct an inventory from caller-supplied archive member paths.
    ///
    /// Paths use the same traversal-free, forward-slash rules as schema v1;
    /// exact and ASCII-case duplicates fail. Members are sorted deterministically.
    pub fn new<I, S>(members: I) -> Result<Self, DistError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut paths = Vec::new();
        let mut seen = HashSet::new();
        for value in members {
            if paths.len() == MAX_OBSERVED_ENTRIES {
                return Err(DistError::InvalidArchiveMemberInventory(bound(format!(
                    "more than {MAX_OBSERVED_ENTRIES} entries"
                ))));
            }
            let path = value.into();
            validate_member_source(&path).map_err(|error| {
                DistError::InvalidArchiveMemberInventory(bound(error.to_string()))
            })?;
            let key = path.to_ascii_lowercase();
            if !seen.insert(key) {
                return Err(DistError::InvalidArchiveMemberInventory(bound(format!(
                    "duplicate or ASCII-case-colliding member path: {path}"
                ))));
            }
            paths.push(path);
        }
        paths.sort();
        Ok(Self { members: paths })
    }

    /// The validated member paths in deterministic order.
    pub fn members(&self) -> &[String] {
        &self.members
    }
}

/// Validate required archive members for an expanded archive target.
///
/// This function compares supplied names only. It never opens or extracts an archive.
pub fn validate_archive_member_inventory(
    expected: &ExpandedTarget,
    observed: &ArchiveMemberInventory,
    extras: ExtrasPolicy,
) -> Result<ConformanceReport, DistError> {
    let ExpandedAssets::Archive(archive) = &expected.assets else {
        return Err(DistError::invalid(
            "archive member validation requires an archive target",
        ));
    };
    if archive.members.len() > MAX_OBSERVED_ENTRIES
        || archive
            .members
            .iter()
            .any(|member| validate_member_source(&member.source).is_err())
    {
        return Ok(ConformanceReport::new(vec![ConformanceFinding::new(
            FindingKind::InvalidObservation,
            "expected archive members",
            None,
            Some("invalid or over limit"),
        )]));
    }
    let required: HashSet<&str> = archive.members.iter().map(|m| m.source.as_str()).collect();
    let observed_set: HashSet<&str> = observed.members.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for member in &archive.members {
        if !observed_set.contains(member.source.as_str()) {
            findings.push(ConformanceFinding::new(
                FindingKind::MissingArchiveMember,
                &member.source,
                Some(&member.source),
                None,
            ));
        }
    }
    if extras == ExtrasPolicy::Exact {
        for path in &observed.members {
            if !required.contains(path.as_str()) {
                findings.push(ConformanceFinding::new(
                    FindingKind::UnexpectedArchiveMember,
                    "archive inventory",
                    None,
                    Some(path),
                ));
            }
        }
    }
    Ok(ConformanceReport::new(findings))
}

/// One observed direct asset mapping reported by consumer-owned runtime/bootstrap tests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedDirectMapping {
    /// Release asset filename selected by the consumer.
    pub asset_file: String,
    /// Checksum sidecar filename selected by the consumer.
    pub sidecar_file: String,
    /// Installed filename selected by the consumer.
    pub install_name: String,
}

/// One observed archive-member-to-install-name mapping.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedArchiveMapping {
    /// Member path selected by the consumer.
    pub source: String,
    /// Installed filename selected by the consumer.
    pub install_name: String,
}

/// Consumer-observed release layout for one target.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedTargetAssets {
    /// One direct asset.
    Direct(ObservedDirectMapping),
    /// Several direct assets forming one logical release.
    Bundle {
        /// Observed entries.
        entries: Vec<ObservedDirectMapping>,
    },
    /// One archive and its required member mappings.
    Archive {
        /// Archive filename selected by the consumer.
        archive_file: String,
        /// Checksum sidecar filename selected by the consumer.
        sidecar_file: String,
        /// Observed member mappings.
        members: Vec<ObservedArchiveMapping>,
    },
}

/// Machine-readable mapping facts extracted by consumer-owned tests or tooling.
///
/// This type deliberately contains no URLs, version policy, commands, privileges,
/// scripts, or service behavior. It can be serialized as TOML by a consumer;
/// Eggup compares the data and does not parse consumer source code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedTargetMapping {
    /// Target triple or alias the consumer claims to use.
    pub target: String,
    /// Canonical triple the consumer claims that target resolved to.
    pub canonical_target: String,
    /// Observed file/install mapping.
    pub assets: ObservedTargetAssets,
}

/// Stable kind for one conformance finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// A required release filename was absent.
    MissingReleaseFile,
    /// Exact release-set validation found an unrelated filename.
    UnexpectedReleaseFile,
    /// Target resolution or canonical target differed.
    TargetMismatch,
    /// Release asset filenames differed.
    AssetMismatch,
    /// Checksum sidecar filenames differed.
    SidecarMismatch,
    /// Install filenames differed.
    InstallNameMismatch,
    /// A required archive member path was absent.
    MissingArchiveMember,
    /// Exact archive-set validation found an undeclared member path.
    UnexpectedArchiveMember,
    /// An archive member/install-name pair differed.
    ArchiveMappingMismatch,
    /// Observed mapping data violated an input bound or path/name rule.
    InvalidObservation,
}

/// One deterministic conformance finding with bounded expected/observed values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct ConformanceFinding {
    /// Finding category.
    pub kind: FindingKind,
    /// Stable logical label or inventory category.
    pub label: String,
    /// Expected value, when applicable.
    pub expected: Option<String>,
    /// Observed value, when applicable.
    pub observed: Option<String>,
}

impl ConformanceFinding {
    fn new(kind: FindingKind, label: &str, expected: Option<&str>, observed: Option<&str>) -> Self {
        Self {
            kind,
            label: bound(label.to_string()),
            expected: expected.map(|value| bound(value.to_string())),
            observed: observed.map(|value| bound(value.to_string())),
        }
    }
}

/// Deterministic, bounded result of a conformance check.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ConformanceReport {
    /// Findings sorted by kind, label, expected value, then observed value.
    pub findings: Vec<ConformanceFinding>,
    /// True if findings were omitted at the report bound.
    pub truncated: bool,
}

impl ConformanceReport {
    fn new(mut findings: Vec<ConformanceFinding>) -> Self {
        findings.sort();
        let truncated = findings.len() > MAX_CONFORMANCE_FINDINGS;
        findings.truncate(MAX_CONFORMANCE_FINDINGS);
        Self {
            findings,
            truncated,
        }
    }

    /// Whether no conformance mismatch was found.
    pub fn is_conformant(&self) -> bool {
        self.findings.is_empty()
    }

    /// Return success for a conformant report, otherwise return the report itself.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_conformant() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

/// Compare consumer-observed target facts against one contract expansion.
///
/// `target` may be a canonical triple or declared alias. Aliases are resolved
/// by the contract with no nearest-target fallback. Observation collections
/// and every reported value are bounded; malformed observations produce a
/// structured `InvalidObservation` finding.
pub fn validate_observed_mapping(
    contract: &DistributionContract,
    version: &str,
    observed: &ObservedTargetMapping,
) -> ConformanceReport {
    if !valid_observation(observed) {
        return ConformanceReport::new(vec![ConformanceFinding::new(
            FindingKind::InvalidObservation,
            "observation",
            None,
            Some("invalid or over limit"),
        )]);
    }
    let expected = match contract.expand(&observed.target, version) {
        Ok(expanded) => expanded,
        Err(_) => {
            return ConformanceReport::new(vec![ConformanceFinding::new(
                FindingKind::TargetMismatch,
                "target",
                Some("declared target or alias"),
                Some(&observed.target),
            )]);
        }
    };
    let mut findings = Vec::new();
    if expected.triple != observed.canonical_target {
        findings.push(ConformanceFinding::new(
            FindingKind::TargetMismatch,
            "canonical target",
            Some(&expected.triple),
            Some(&observed.canonical_target),
        ));
    }
    compare_observed_assets(&expected.assets, &observed.assets, &mut findings);
    ConformanceReport::new(findings)
}

fn valid_observation(observed: &ObservedTargetMapping) -> bool {
    if observed.target.is_empty()
        || observed.target.len() > MAX_NAME_LEN
        || observed.canonical_target.is_empty()
        || observed.canonical_target.len() > MAX_NAME_LEN
        || observed.target.chars().any(char::is_control)
        || observed.canonical_target.chars().any(char::is_control)
    {
        return false;
    }
    let mut count = 0usize;
    let valid_names = |count: &mut usize, names: &[&str]| {
        *count = (*count).saturating_add(names.len());
        *count <= MAX_OBSERVED_ENTRIES
            && names
                .iter()
                .all(|name| validate_observed_flat_name(name).is_ok())
    };
    match &observed.assets {
        ObservedTargetAssets::Direct(entry) => {
            valid_names(
                &mut count,
                &[&entry.asset_file, &entry.sidecar_file, &entry.install_name],
            ) && unique_ascii_names(&[&entry.asset_file, &entry.sidecar_file])
        }
        ObservedTargetAssets::Bundle { entries } => {
            let fields_valid = entries.iter().all(|entry| {
                valid_names(
                    &mut count,
                    &[&entry.asset_file, &entry.sidecar_file, &entry.install_name],
                )
            });
            let release_files: Vec<&str> = entries
                .iter()
                .flat_map(|entry| [&entry.asset_file[..], &entry.sidecar_file[..]])
                .collect();
            fields_valid && unique_ascii_names(&release_files)
        }
        ObservedTargetAssets::Archive {
            archive_file,
            sidecar_file,
            members,
        } => {
            if !valid_names(&mut count, &[archive_file, sidecar_file]) {
                return false;
            }
            let mut seen = HashSet::new();
            let fields_valid = members.iter().all(|member| {
                count = count.saturating_add(2);
                count <= MAX_OBSERVED_ENTRIES
                    && validate_member_source(&member.source).is_ok()
                    && validate_observed_flat_name(&member.install_name).is_ok()
                    && seen.insert(member.source.to_ascii_lowercase())
            });
            fields_valid && unique_ascii_names(&[archive_file, sidecar_file])
        }
    }
}

fn unique_ascii_names(names: &[&str]) -> bool {
    let mut seen = HashSet::new();
    names
        .iter()
        .all(|name| seen.insert(name.to_ascii_lowercase()))
}

fn compare_observed_assets(
    expected: &ExpandedAssets,
    observed: &ObservedTargetAssets,
    findings: &mut Vec<ConformanceFinding>,
) {
    match (expected, observed) {
        (ExpandedAssets::Direct(expected), ObservedTargetAssets::Direct(observed)) => {
            compare_names(
                FindingKind::AssetMismatch,
                "asset",
                &[expected.asset_file.as_str()],
                &[observed.asset_file.as_str()],
                findings,
            );
            compare_names(
                FindingKind::SidecarMismatch,
                "sidecar",
                &[expected.sidecar_file.as_str()],
                &[observed.sidecar_file.as_str()],
                findings,
            );
            compare_names(
                FindingKind::InstallNameMismatch,
                "install",
                &[expected.install_name.as_str()],
                &[observed.install_name.as_str()],
                findings,
            );
        }
        (ExpandedAssets::Bundle(expected), ObservedTargetAssets::Bundle { entries }) => {
            compare_names(
                FindingKind::AssetMismatch,
                "bundle assets",
                &expected
                    .entries
                    .iter()
                    .map(|e| e.asset_file.as_str())
                    .collect::<Vec<_>>(),
                &entries
                    .iter()
                    .map(|e| e.asset_file.as_str())
                    .collect::<Vec<_>>(),
                findings,
            );
            compare_names(
                FindingKind::SidecarMismatch,
                "bundle sidecars",
                &expected
                    .entries
                    .iter()
                    .map(|e| e.sidecar_file.as_str())
                    .collect::<Vec<_>>(),
                &entries
                    .iter()
                    .map(|e| e.sidecar_file.as_str())
                    .collect::<Vec<_>>(),
                findings,
            );
            compare_names(
                FindingKind::InstallNameMismatch,
                "bundle installs",
                &expected
                    .entries
                    .iter()
                    .map(|e| e.install_name.as_str())
                    .collect::<Vec<_>>(),
                &entries
                    .iter()
                    .map(|e| e.install_name.as_str())
                    .collect::<Vec<_>>(),
                findings,
            );
        }
        (
            ExpandedAssets::Archive(expected),
            ObservedTargetAssets::Archive {
                archive_file,
                sidecar_file,
                members,
            },
        ) => {
            compare_names(
                FindingKind::AssetMismatch,
                "archive asset",
                &[expected.archive_file.as_str()],
                &[archive_file.as_str()],
                findings,
            );
            compare_names(
                FindingKind::SidecarMismatch,
                "archive sidecar",
                &[expected.sidecar_file.as_str()],
                &[sidecar_file.as_str()],
                findings,
            );
            let expected_members: HashMap<&str, &str> = expected
                .members
                .iter()
                .map(|member| (member.source.as_str(), member.install_name.as_str()))
                .collect();
            let observed_members: HashMap<&str, &str> = members
                .iter()
                .map(|member| (member.source.as_str(), member.install_name.as_str()))
                .collect();
            let mut sources: Vec<&str> = expected_members
                .keys()
                .chain(observed_members.keys())
                .copied()
                .collect();
            sources.sort_unstable();
            sources.dedup();
            for source in sources {
                let expected_install = expected_members.get(source).copied();
                let observed_install = observed_members.get(source).copied();
                if expected_install != observed_install {
                    findings.push(ConformanceFinding::new(
                        FindingKind::ArchiveMappingMismatch,
                        source,
                        expected_install,
                        observed_install,
                    ));
                }
            }
        }
        (expected, observed) => findings.push(ConformanceFinding::new(
            FindingKind::AssetMismatch,
            "asset form",
            Some(asset_form_name(expected)),
            Some(observed_form_name(observed)),
        )),
    }
}

fn compare_names(
    kind: FindingKind,
    label: &str,
    expected: &[&str],
    observed: &[&str],
    findings: &mut Vec<ConformanceFinding>,
) {
    let mut expected = expected.to_vec();
    let mut observed = observed.to_vec();
    expected.sort();
    observed.sort();
    for index in 0..expected.len().max(observed.len()) {
        let expected_value = expected.get(index).copied();
        let observed_value = observed.get(index).copied();
        if expected_value != observed_value {
            findings.push(ConformanceFinding::new(
                kind,
                label,
                expected_value,
                observed_value,
            ));
        }
    }
}

fn asset_form_name(assets: &ExpandedAssets) -> &'static str {
    match assets {
        ExpandedAssets::Direct(_) => "direct",
        ExpandedAssets::Bundle(_) => "bundle",
        ExpandedAssets::Archive(_) => "archive",
    }
}

fn observed_form_name(assets: &ObservedTargetAssets) -> &'static str {
    match assets {
        ObservedTargetAssets::Direct(_) => "direct",
        ObservedTargetAssets::Bundle { .. } => "bundle",
        ObservedTargetAssets::Archive { .. } => "archive",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = r#"
schema_version = 1

[product]
id = "eggsact"

[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = ["linux-x64"]

[targets.asset]
kind = "direct"
asset = "{product}-{version}-{target}"
install = "{product}"

[targets.checksum]
sidecar = "{asset}.sha256"
"#;

    const BUNDLE: &str = r#"
schema_version = 1

[product]
id = "codegg"

[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = []

[targets.asset]
kind = "bundle"

[[targets.asset.entries]]
asset = "{product}-{version}-{target}"
install = "{product}"

[[targets.asset.entries]]
asset = "{product}-helper-{version}-{target}"
install = "{product}-helper"

[[targets.asset.entries]]
asset = "{product}-manifest-{version}.json"
install = "{product}-manifest.json"

[targets.checksum]
sidecar = "{asset}.sha256"
"#;

    const ARCHIVE: &str = r#"
schema_version = 1

[product]
id = "egress"

[[targets]]
triple = "aarch64-apple-darwin"
aliases = ["macos-arm64"]

[targets.asset]
kind = "archive"
asset = "{product}-{version}-{target}.tar.gz"

[[targets.asset.members]]
source = "egress"
install = "egress"

[[targets.asset.members]]
source = "bin/egress-helper"
install = "egress-helper"

[targets.checksum]
sidecar = "{asset}.sha256"
"#;

    fn bundle_with(
        asset0: &str,
        asset1: &str,
        install0: &str,
        install1: &str,
        sidecar: &str,
    ) -> Result<DistributionContract, DistError> {
        let source = BUNDLE
            .replacen(
                "asset = \"{product}-{version}-{target}\"",
                &format!("asset = \"{asset0}\""),
                1,
            )
            .replacen(
                "asset = \"{product}-helper-{version}-{target}\"",
                &format!("asset = \"{asset1}\""),
                1,
            )
            .replacen(
                "install = \"{product}\"",
                &format!("install = \"{install0}\""),
                1,
            )
            .replacen(
                "install = \"{product}-helper\"",
                &format!("install = \"{install1}\""),
                1,
            )
            .replace(
                "sidecar = \"{asset}.sha256\"",
                &format!("sidecar = \"{sidecar}\""),
            );
        DistributionContract::parse_toml_str(&source)
    }

    fn archive_with(install0: &str, install1: &str) -> Result<DistributionContract, DistError> {
        let source = ARCHIVE
            .replacen(
                "install = \"egress\"",
                &format!("install = \"{install0}\""),
                1,
            )
            .replacen(
                "install = \"egress-helper\"",
                &format!("install = \"{install1}\""),
                1,
            );
        DistributionContract::parse_toml_str(&source)
    }

    #[test]
    fn parse_minimal_valid_v1() {
        let c = DistributionContract::parse_toml_str(SIMPLE).unwrap();
        assert_eq!(c.schema_version, 1);
        assert_eq!(c.product.id, "eggsact");
        assert_eq!(c.targets.len(), 1);
    }

    #[test]
    fn unknown_schema_version_fails_typed() {
        let s = SIMPLE.replace("schema_version = 1", "schema_version = 2");
        let err = DistributionContract::parse_toml_str(&s).unwrap_err();
        assert!(matches!(err, DistError::UnsupportedVersion { found: 2 }));
    }

    #[test]
    fn duplicate_target_is_rejected() {
        let s = format!("{SIMPLE}\n[[targets]]\ntriple = \"x86_64-unknown-linux-gnu\"\n[targets.asset]\nkind = \"direct\"\nasset = \"a\"\ninstall = \"b\"\n[targets.checksum]\nsidecar = \"{{asset}}.sha256\"\n");
        let err = DistributionContract::parse_toml_str(&s).unwrap_err();
        assert!(matches!(err, DistError::DuplicateTarget(_)));
    }

    #[test]
    fn duplicate_alias_is_rejected() {
        let s = r#"
schema_version = 1
[product]
id = "p"
[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = ["same"]
[targets.asset]
kind = "direct"
asset = "a"
install = "b"
[targets.checksum]
sidecar = "{asset}.sha256"
[[targets]]
triple = "aarch64-apple-darwin"
aliases = ["same"]
[targets.asset]
kind = "direct"
asset = "a"
install = "b"
[targets.checksum]
sidecar = "{asset}.sha256"
"#;
        let err = DistributionContract::parse_toml_str(s).unwrap_err();
        assert!(matches!(err, DistError::DuplicateAlias(_)));
    }

    #[test]
    fn alias_colliding_with_triple_is_rejected() {
        let s = r#"
schema_version = 1
[product]
id = "p"
[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = ["aarch64-apple-darwin"]
[targets.asset]
kind = "direct"
asset = "a"
install = "b"
[targets.checksum]
sidecar = "{asset}.sha256"
[[targets]]
triple = "aarch64-apple-darwin"
aliases = []
[targets.asset]
kind = "direct"
asset = "a"
install = "b"
[targets.checksum]
sidecar = "{asset}.sha256"
"#;
        let err = DistributionContract::parse_toml_str(s).unwrap_err();
        assert!(matches!(err, DistError::DuplicateAlias(_)));
    }

    #[test]
    fn unsupported_target_lookup_fails_without_guessing() {
        let c = DistributionContract::parse_toml_str(SIMPLE).unwrap();
        let err = c.resolve("riscv64-unknown-linux-gnu").unwrap_err();
        assert!(matches!(err, DistError::UnknownTarget(_)));
        // Alias lookup works; wrong alias does not guess.
        assert!(c.resolve("linux-x64").is_ok());
        assert!(c.resolve("linux-arm64").is_err());
    }

    #[test]
    fn unknown_placeholder_is_rejected() {
        let s = SIMPLE.replace("{product}-{version}-{target}", "{product}-{env}-x");
        let err = DistributionContract::parse_toml_str(&s).unwrap_err();
        assert!(matches!(err, DistError::UnknownPlaceholder(_)));
    }

    #[test]
    fn malformed_template_grammar_is_rejected_during_parse() {
        for bad in [
            "{product",
            "product}",
            "{}",
            "{product{version}}",
            "{ bad}",
            "{{product}}",
            "file name",
            "{asset}",
        ] {
            let source = SIMPLE.replace(
                "asset = \"{product}-{version}-{target}\"",
                &format!("asset = \"{bad}\""),
            );
            assert!(
                DistributionContract::parse_toml_str(&source).is_err(),
                "accepted malformed template {bad:?}"
            );
        }
    }

    #[test]
    fn repeated_known_placeholders_remain_valid() {
        let source = SIMPLE.replace(
            "asset = \"{product}-{version}-{target}\"",
            "asset = \"{product}-{product}-{version}\"",
        );
        let contract = DistributionContract::parse_toml_str(&source).unwrap();
        let expanded = contract
            .expand("x86_64-unknown-linux-gnu", "1.2.3")
            .unwrap();
        match expanded.assets {
            ExpandedAssets::Direct(direct) => {
                assert_eq!(direct.asset_file, "eggsact-eggsact-1.2.3")
            }
            _ => panic!("expected direct"),
        }
    }

    #[test]
    fn bundle_expansion_rejects_release_asset_and_sidecar_collisions() {
        let contract = bundle_with("same", "same", "app", "helper", "{asset}.sha256")
            .expect_err("identical raw assets should fail early");
        assert!(matches!(
            contract,
            DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            }
        ));

        let contract =
            bundle_with("{version}", "1.2.3", "app", "helper", "{asset}.sha256").unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            })
        ));

        let contract = bundle_with("one", "two", "app", "helper", "constant.sha256").unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            })
        ));

        let contract = bundle_with("first", "second", "app", "helper", "first").unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            })
        ));
    }

    #[test]
    fn direct_and_archive_asset_sidecar_collisions_are_rejected() {
        let direct = SIMPLE.replace(
            "sidecar = \"{asset}.sha256\"",
            "sidecar = \"{product}-{version}-{target}\"",
        );
        let contract = DistributionContract::parse_toml_str(&direct).unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            })
        ));

        let archive = ARCHIVE.replace("sidecar = \"{asset}.sha256\"", "sidecar = \"{asset}\"");
        let contract = DistributionContract::parse_toml_str(&archive).unwrap();
        assert!(matches!(
            contract.expand("aarch64-apple-darwin", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            })
        ));
    }

    #[test]
    fn bundle_and_archive_install_names_must_be_unique_and_portable() {
        let contract = bundle_with("one", "two", "same", "same", "{asset}.sha256")
            .expect_err("identical raw install names should fail early");
        assert!(matches!(
            contract,
            DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            }
        ));

        let contract = bundle_with("one", "two", "App", "app", "{asset}.sha256")
            .expect_err("ASCII case-only raw install names should fail early");
        assert!(matches!(
            contract,
            DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            }
        ));

        let contract = bundle_with("one", "two", "{version}", "APP", "{asset}.sha256").unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "app"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            })
        ));

        let contract = bundle_with(
            "one",
            "two",
            "member-{version}",
            "member-1.2.3",
            "{asset}.sha256",
        )
        .unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            })
        ));

        let contract = archive_with("tool", "tool").expect_err("duplicate archive install name");
        assert!(matches!(
            contract,
            DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            }
        ));
        let contract = archive_with("Tool", "tool")
            .expect_err("ASCII case-only raw archive install names should fail early");
        assert!(matches!(
            contract,
            DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            }
        ));
        let contract = archive_with("{version}", "TOOL").unwrap();
        assert!(matches!(
            contract.expand("aarch64-apple-darwin", "tool"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            })
        ));
    }

    #[test]
    fn release_and_install_case_only_collisions_are_rejected() {
        let source = SIMPLE.replace(
            "sidecar = \"{asset}.sha256\"",
            "sidecar = \"EGGSACT-1.2.3-X86_64-UNKNOWN-LINUX-GNU\"",
        );
        let contract = DistributionContract::parse_toml_str(&source).unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "1.2.3"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::ReleaseFiles,
                ..
            })
        ));

        let contract = bundle_with("one", "two", "{version}", "APP", "{asset}.sha256").unwrap();
        assert!(matches!(
            contract.expand("x86_64-unknown-linux-gnu", "app"),
            Err(DistError::NameCollision {
                namespace: NameNamespace::InstallNames,
                ..
            })
        ));
    }

    #[test]
    fn missing_alias_input_fails() {
        let c = DistributionContract::parse_toml_str(
            &SIMPLE.replace("install = \"{product}\"", "install = \"{alias}\""),
        )
        .unwrap();
        // Lookup via triple leaves `{alias}` without input.
        let err = c.expand("x86_64-unknown-linux-gnu", "1.0.0").unwrap_err();
        assert!(matches!(err, DistError::MissingInput(_)));
        // Lookup via the alias supplies it.
        let ok = c.expand("linux-x64", "1.0.0").unwrap();
        match ok.assets {
            ExpandedAssets::Direct(d) => assert_eq!(d.install_name, "linux-x64"),
            _ => panic!("expected direct"),
        }
    }

    #[test]
    fn direct_single_member_expansion() {
        let c = DistributionContract::parse_toml_str(SIMPLE).unwrap();
        let e = c.expand("x86_64-unknown-linux-gnu", "1.2.6").unwrap();
        match e.assets {
            ExpandedAssets::Direct(d) => {
                assert_eq!(d.asset_file, "eggsact-1.2.6-x86_64-unknown-linux-gnu");
                assert_eq!(d.install_name, "eggsact");
                assert_eq!(
                    d.sidecar_file,
                    "eggsact-1.2.6-x86_64-unknown-linux-gnu.sha256"
                );
            }
            _ => panic!("expected direct"),
        }
    }

    #[test]
    fn multi_direct_bundle_expansion() {
        let c = DistributionContract::parse_toml_str(BUNDLE).unwrap();
        let e = c.expand("x86_64-unknown-linux-gnu", "0.9.0").unwrap();
        match e.assets {
            ExpandedAssets::Bundle(b) => {
                assert_eq!(b.entries.len(), 3);
                assert_eq!(
                    b.entries[0].asset_file,
                    "codegg-0.9.0-x86_64-unknown-linux-gnu"
                );
                assert_eq!(b.entries[1].install_name, "codegg-helper");
                assert_eq!(b.entries[2].asset_file, "codegg-manifest-0.9.0.json");
                assert!(b.entries[0].sidecar_file.ends_with(".sha256"));
            }
            _ => panic!("expected bundle"),
        }
    }

    #[test]
    fn archive_asset_expansion() {
        let c = DistributionContract::parse_toml_str(ARCHIVE).unwrap();
        let e = c.expand("macos-arm64", "2.1.0").unwrap();
        assert_eq!(e.triple, "aarch64-apple-darwin");
        match e.assets {
            ExpandedAssets::Archive(a) => {
                assert_eq!(a.archive_file, "egress-2.1.0-aarch64-apple-darwin.tar.gz");
                assert_eq!(
                    a.sidecar_file,
                    "egress-2.1.0-aarch64-apple-darwin.tar.gz.sha256"
                );
                assert_eq!(a.members.len(), 2);
                assert_eq!(a.members[0].source, "egress");
                assert_eq!(a.members[1].source, "bin/egress-helper");
            }
            _ => panic!("expected archive"),
        }
    }

    #[test]
    fn archive_member_absolute_path_is_rejected() {
        let s = ARCHIVE.replace("source = \"egress\"", "source = \"/egress\"");
        let err = DistributionContract::parse_toml_str(&s).unwrap_err();
        assert!(matches!(err, DistError::InvalidMemberPath(_)));
    }

    #[test]
    fn archive_member_traversal_is_rejected() {
        for bad in ["../evil", "a/../../b", "a/../b", "bin/..", ".", ".."] {
            let s = ARCHIVE.replace("source = \"egress\"", &format!("source = \"{bad}\""));
            let err = DistributionContract::parse_toml_str(&s).unwrap_err();
            assert!(matches!(err, DistError::InvalidMemberPath(_)), "{bad}");
        }
    }

    #[test]
    fn duplicate_normalized_archive_members_rejected() {
        let s = ARCHIVE.replace("source = \"bin/egress-helper\"", "source = \"egress\"");
        let err = DistributionContract::parse_toml_str(&s).unwrap_err();
        assert!(matches!(err, DistError::InvalidInput(_)));
    }

    #[test]
    fn checksum_sidecar_naming_is_explicit() {
        let c = DistributionContract::parse_toml_str(SIMPLE).unwrap();
        let e = c.expand("linux-x64", "1.0.0").unwrap();
        match e.assets {
            ExpandedAssets::Direct(d) => {
                assert!(d.sidecar_file.contains(&d.asset_file));
                assert!(!d.sidecar_file.contains("signature"));
            }
            _ => panic!("expected direct"),
        }
    }

    #[test]
    fn product_version_is_opaque_but_filesystem_safe() {
        let c = DistributionContract::parse_toml_str(SIMPLE).unwrap();
        // Opaque: dotted versions pass through unchanged.
        let e = c.expand("x86_64-unknown-linux-gnu", "1.2.6-rc.1").unwrap();
        match e.assets {
            ExpandedAssets::Direct(d) => assert!(d.asset_file.contains("1.2.6-rc.1")),
            _ => panic!("expected direct"),
        }
        // Unsafe versions fail instead of escaping.
        assert!(c.expand("x86_64-unknown-linux-gnu", "../evil").is_err());
        assert!(c.expand("x86_64-unknown-linux-gnu", "a/b").is_err());
        assert!(c.expand("x86_64-unknown-linux-gnu", "").is_err());
    }

    #[test]
    fn deterministic_round_trip_golden() {
        for fixture in [SIMPLE, BUNDLE, ARCHIVE] {
            let c = DistributionContract::parse_toml_str(fixture).unwrap();
            let rendered = c.to_toml_string().unwrap();
            let reparsed = DistributionContract::parse_toml_str(&rendered).unwrap();
            assert_eq!(c, reparsed);
            // Golden stability: rendering twice yields identical bytes.
            assert_eq!(rendered, reparsed.to_toml_string().unwrap());
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let s = SIMPLE.replace(
            "[product]\nid = \"eggsact\"",
            "[product]\nid = \"eggsact\"\ntypo_field = 1",
        );
        assert!(DistributionContract::parse_toml_str(&s).is_err());
    }
}
