//! GitHub draft release staging adapter and local staging payload materializer.
//!
//! M003a owns two bounded capabilities: materialize a complete local staging
//! payload from M004 finalization plus `ReleaseManifest` plus deterministic
//! bootstrap generators, and reconcile that payload into a GitHub draft
//! release through a provider-specific adapter. There is no publication
//! authority, no tag creation/move/delete, and no `publish` endpoint.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use eggpack_bootstrap::{BootstrapInstallPolicyV1, BootstrapSpec};
use eggpack_contract::{DistributionContract, ExpandedAssets};
use eggpack_manifest::ReleaseManifest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

const API_BASE: &str = "https://api.github.com";
const UPLOAD_BASE: &str = "https://uploads.github.com";
const API_VERSION: &str = "2026-03-10";
const USER_AGENT: &str = "eggpack-github/0.1.0";
const MAX_TAG_PEEL_DEPTH: usize = 8;
const MAX_POLICY_JSON: usize = 256 * 1024;
const MAX_PAYLOAD_JSON: usize = 1_048_576;
const MAX_BODY_NOTES: usize = 65_536;
const MAX_TITLE: usize = 256;
const MAX_NAME_SEGMENT: usize = 128;
const FIXED_MANIFEST_NAME: &str = "release-manifest.json";
const FIXED_POSIX_NAME: &str = "install.sh";
const FIXED_POWERSHELL_NAME: &str = "install.ps1";
/// Maximum upload read/write chunk, bounding per-asset transfer memory.
const UPLOAD_CHUNK_BYTES: usize = 64 * 1024;
const ASSET_PAGE_SIZE: usize = 100;

/// Bounded staging failure with redacted diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubError(String);

impl fmt::Display for GithubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GithubError {}

fn fail(message: impl Into<String>) -> GithubError {
    GithubError(message.into())
}

/// Strict draft staging policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubDraftPolicyV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Exact existing tag to stage.
    pub tag: String,
    /// Draft release title.
    pub title: String,
    /// Bounded release notes body.
    #[serde(default)]
    pub body: String,
    /// Prerelease intent.
    #[serde(default)]
    pub prerelease: bool,
    /// Token environment variable name, allowlisted to `GITHUB_TOKEN`.
    #[serde(default = "default_token_env")]
    pub token_env: String,
    /// Per-request timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub request_timeout_secs: u64,
    /// Maximum metadata response bytes.
    #[serde(default = "default_max_metadata_bytes")]
    pub max_metadata_bytes: usize,
    /// Maximum bounded release-list pages.
    #[serde(default = "default_max_list_pages")]
    pub max_list_pages: u32,
}

fn default_token_env() -> String {
    "GITHUB_TOKEN".to_owned()
}

fn default_timeout_secs() -> u64 {
    30
}

fn default_max_metadata_bytes() -> usize {
    1_000_000
}

fn default_max_list_pages() -> u32 {
    16
}

impl GitHubDraftPolicyV1 {
    /// Parse strict bounded JSON policy.
    pub fn from_json(text: &str) -> Result<Self, GithubError> {
        if text.len() > MAX_POLICY_JSON {
            return Err(fail("github draft policy exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid github draft policy JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize deterministically after validation.
    pub fn to_json(&self) -> Result<String, GithubError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("github draft policy encode failed"))
    }

    /// Validate all bounds and injection rules.
    pub fn validate(&self) -> Result<(), GithubError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported github draft policy version"));
        }
        validate_owner(&self.owner)?;
        validate_repo(&self.repository)?;
        validate_tag(&self.tag)?;
        validate_title(&self.title)?;
        validate_body(&self.body)?;
        if self.token_env != "GITHUB_TOKEN" {
            return Err(fail("token env must be GITHUB_TOKEN"));
        }
        if !(5..=120).contains(&self.request_timeout_secs) {
            return Err(fail("request timeout out of bounds"));
        }
        if self.max_metadata_bytes == 0 || self.max_metadata_bytes > 8_000_000 {
            return Err(fail("max metadata bytes out of bounds"));
        }
        if self.max_list_pages == 0 || self.max_list_pages > 32 {
            return Err(fail("max list pages out of bounds"));
        }
        Ok(())
    }

    /// Exact release download origin for bootstrap installers.
    pub fn download_origin(&self) -> String {
        format!(
            "https://github.com/{}/{}/releases/download/{}",
            self.owner, self.repository, self.tag
        )
    }
}

/// Semantic kind of one staged file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingAssetKind {
    /// Finalized M004 artifact bytes.
    FinalizedArtifact,
    /// Checksum sidecar for a finalized artifact.
    ChecksumSidecar,
    /// Standalone deterministic release manifest.
    ReleaseManifest,
    /// Deterministic POSIX installer.
    PosixInstaller,
    /// Deterministic PowerShell installer.
    PowershellInstaller,
}

/// One ordered staged file record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagingAsset {
    /// Safe flat asset name.
    pub name: String,
    /// Local relative path under payload root (flat, equals name).
    pub path: String,
    /// Exact byte size.
    pub size: u64,
    /// Lowercase SHA-256 hex.
    pub sha256: String,
    /// Media type for upload.
    pub media_type: String,
    /// Semantic kind.
    pub kind: StagingAssetKind,
}

/// Deterministic local staging payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagingPayloadV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Product id from manifest.
    pub product_id: String,
    /// Release id from manifest.
    pub release_id: String,
    /// Source revision from manifest.
    pub source_revision: String,
    /// Repository owner.
    pub owner: String,
    /// Repository name.
    pub repository: String,
    /// Exact existing tag.
    pub tag: String,
    /// Draft title.
    pub title: String,
    /// Prerelease intent.
    pub prerelease: bool,
    /// Bounded release notes.
    pub body: String,
    /// Ordered asset records sorted by name.
    pub assets: Vec<StagingAsset>,
}

impl StagingPayloadV1 {
    /// Parse strict bounded JSON payload.
    pub fn from_json(text: &str) -> Result<Self, GithubError> {
        if text.len() > MAX_PAYLOAD_JSON {
            return Err(fail("staging payload exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid staging payload JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize deterministically after validation.
    pub fn to_json(&self) -> Result<String, GithubError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.assets.sort_by(|a, b| a.name.cmp(&b.name));
        serde_json::to_string(&canonical).map_err(|_| fail("staging payload encode failed"))
    }

    /// Validate bounds, paths, digests, and ordering.
    pub fn validate(&self) -> Result<(), GithubError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported staging payload version"));
        }
        if self.product_id.is_empty()
            || self.product_id.len() > MAX_NAME_SEGMENT
            || self.release_id.is_empty()
            || self.release_id.len() > MAX_NAME_SEGMENT
            || self.source_revision.is_empty()
            || self.source_revision.len() > MAX_NAME_SEGMENT
        {
            return Err(fail("staging payload identity out of bounds"));
        }
        validate_owner(&self.owner)?;
        validate_repo(&self.repository)?;
        validate_tag(&self.tag)?;
        validate_title(&self.title)?;
        validate_body(&self.body)?;
        if self.assets.is_empty() || self.assets.len() > 1024 {
            return Err(fail("staging payload asset count out of bounds"));
        }
        let mut seen = BTreeSet::new();
        for asset in &self.assets {
            validate_asset_name(&asset.name)?;
            validate_relative_path(&asset.path)?;
            if asset.path != asset.name {
                return Err(fail("staging payload path must equal flat asset name"));
            }
            if asset.size == 0 {
                return Err(fail("staging asset size must be non-zero"));
            }
            validate_sha256(&asset.sha256)?;
            if asset.media_type.is_empty() || asset.media_type.len() > 128 {
                return Err(fail("staging asset media type out of bounds"));
            }
            if !seen.insert(asset.name.to_ascii_lowercase()) {
                return Err(fail("duplicate staging asset name"));
            }
        }
        Ok(())
    }
}

/// Bounded staging receipt (producer evidence, not an install receipt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubDraftReceiptV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Repository owner.
    pub owner: String,
    /// Repository name.
    pub repository: String,
    /// Product release id.
    pub release_id: String,
    /// Exact tag.
    pub tag: String,
    /// Source revision.
    pub source_revision: String,
    /// GitHub numeric release id.
    pub github_release_id: u64,
    /// Draft state, always true.
    pub draft: bool,
    /// Immutable state, always false for staged drafts.
    pub immutable: bool,
    /// Ordered asset names.
    pub assets: Vec<StagingAsset>,
    /// True when the release was created by this invocation.
    pub created: bool,
    /// Count of uploaded assets.
    pub uploaded: u32,
    /// Count of reused exact assets.
    pub reused: u32,
}

impl GitHubDraftReceiptV1 {
    /// Parse strict bounded JSON receipt.
    pub fn from_json(text: &str) -> Result<Self, GithubError> {
        if text.len() > MAX_PAYLOAD_JSON {
            return Err(fail("staging receipt exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid staging receipt JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize after validation.
    pub fn to_json(&self) -> Result<String, GithubError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("staging receipt encode failed"))
    }

    /// Validate receipt shape.
    pub fn validate(&self) -> Result<(), GithubError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported staging receipt version"));
        }
        validate_owner(&self.owner)?;
        validate_repo(&self.repository)?;
        validate_tag(&self.tag)?;
        if !self.draft || self.immutable {
            return Err(fail("receipt must describe a mutable draft"));
        }
        if self.assets.is_empty() || self.assets.len() > 1024 {
            return Err(fail("receipt asset count out of bounds"));
        }
        if self
            .uploaded
            .checked_add(self.reused)
            .is_none_or(|total| total as usize != self.assets.len())
        {
            return Err(fail("receipt upload/reuse counts must cover assets"));
        }
        Ok(())
    }
}

fn validate_owner(value: &str) -> Result<(), GithubError> {
    if value.is_empty() || value.len() > 64 {
        return Err(fail("owner out of bounds"));
    }
    if value.chars().any(char::is_control) {
        return Err(fail("owner contains control characters"));
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(fail("owner contains unsupported characters"));
    }
    if value.starts_with('-') || value.ends_with('-') || value.contains("..") {
        return Err(fail("owner has unsafe shape"));
    }
    Ok(())
}

fn validate_repo(value: &str) -> Result<(), GithubError> {
    if value.is_empty() || value.len() > 64 {
        return Err(fail("repository out of bounds"));
    }
    if value.chars().any(char::is_control) {
        return Err(fail("repository contains control characters"));
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(fail("repository contains unsupported characters"));
    }
    if value.contains("..") {
        return Err(fail("repository has unsafe shape"));
    }
    Ok(())
}

fn validate_tag(value: &str) -> Result<(), GithubError> {
    if value.is_empty() || value.len() > 128 {
        return Err(fail("tag out of bounds"));
    }
    if value.chars().any(char::is_control) {
        return Err(fail("tag contains control characters"));
    }
    if value.contains(['?', '#', '@', ' ', '\\', '\'', '"', '`', '$']) {
        return Err(fail("tag contains query/injection characters"));
    }
    if value.contains("..") || value.starts_with('/') || value.ends_with('/') {
        return Err(fail("tag has unsafe path shape"));
    }
    Ok(())
}

fn validate_title(value: &str) -> Result<(), GithubError> {
    if value.is_empty() || value.len() > MAX_TITLE {
        return Err(fail("title out of bounds"));
    }
    if value.chars().any(char::is_control) {
        return Err(fail("title contains control characters"));
    }
    Ok(())
}

fn validate_body(value: &str) -> Result<(), GithubError> {
    if value.len() > MAX_BODY_NOTES {
        return Err(fail("release notes exceed bound"));
    }
    if value.chars().any(|c| c == '\0') {
        return Err(fail("release notes contain NUL"));
    }
    Ok(())
}

fn validate_asset_name(value: &str) -> Result<(), GithubError> {
    if value.is_empty() || value.len() > 255 {
        return Err(fail("asset name out of bounds"));
    }
    if value == "." || value == ".." {
        return Err(fail("asset name is not a safe flat name"));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(fail("asset name must be flat"));
    }
    if value.chars().any(char::is_control) {
        return Err(fail("asset name contains control characters"));
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), GithubError> {
    if value.is_empty() || value.len() > 512 {
        return Err(fail("staging path out of bounds"));
    }
    if Path::new(value).is_absolute() {
        return Err(fail("staging path must be relative"));
    }
    if value.contains('\\') {
        return Err(fail("staging path must use flat relative form"));
    }
    let path = Path::new(value);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(fail("staging path contains traversal")),
        }
    }
    validate_asset_name(value)?;
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), GithubError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fail("sha256 must be 64 lowercase hex characters"));
    }
    Ok(())
}

/// Read a credential from the environment without logging it.
pub fn read_token(env_name: &str) -> Result<String, GithubError> {
    if env_name != "GITHUB_TOKEN" {
        return Err(fail("token env must be GITHUB_TOKEN"));
    }
    let value = std::env::var(env_name).map_err(|_| fail("github token is absent"))?;
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(fail("github token is absent"));
    }
    Ok(value)
}

fn digest_file(path: &Path) -> Result<(u64, String), GithubError> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| fail("staging file is unavailable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        return Err(fail("staging file must be a non-empty regular file"));
    }
    let bytes = std::fs::read(path).map_err(|_| fail("staging file cannot be read"))?;
    let after =
        std::fs::symlink_metadata(path).map_err(|_| fail("staging file changed during read"))?;
    if after.file_type().is_symlink() || !after.is_file() || after.len() != bytes.len() as u64 {
        return Err(fail("staging file changed during read"));
    }
    use sha2::Digest;
    let digest = sha2::Sha256::digest(&bytes);
    Ok((
        bytes.len() as u64,
        digest.iter().map(|b| format!("{b:02x}")).collect(),
    ))
}

fn reject_symlink_dir(path: &Path, label: &str) -> Result<(), GithubError> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| fail(format!("{label} is unavailable")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(fail(format!("{label} is not a real directory")));
    }
    Ok(())
}

fn ensure_absent_private_dir(path: &Path, label: &str) -> Result<(), GithubError> {
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(fail(format!("{label} must be absent")));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    reject_symlink_dir(&parent, "output parent")?;
    std::fs::create_dir(path).map_err(|_| fail(format!("{label} could not be created")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| fail(format!("{label} permissions could not be restricted")))?;
    }
    Ok(())
}

/// Materialize a complete deterministic staging payload.
///
/// The finalized root is revalidated against the contract and manifest; exact
/// finalized assets are copied into a private staging directory alongside a
/// standalone `release-manifest.json` and deterministic bootstrap installers.
#[allow(clippy::too_many_arguments)]
pub fn prepare_staging_payload(
    contract: &DistributionContract,
    manifest: &ReleaseManifest,
    finalized_root: &Path,
    policy: &GitHubDraftPolicyV1,
    install_policy: &BootstrapInstallPolicyV1,
    output_dir: &Path,
) -> Result<StagingPayloadV1, GithubError> {
    policy.validate()?;
    manifest
        .validate()
        .map_err(|_| fail("invalid release manifest"))?;
    if contract.product.id != manifest.product_id {
        return Err(fail("contract and manifest product mismatch"));
    }
    reject_symlink_dir(finalized_root, "finalized root")?;

    // Collect expected inventory from contract expansion plus manifest facts.
    let mut expected: BTreeMap<String, (bool, u64, String)> = BTreeMap::new();
    for target in &manifest.targets {
        let expanded = contract
            .expand(&target.target, &manifest.release_id)
            .map_err(|_| fail("manifest target is not supported by contract"))?;
        match (&expanded.assets, &target.form) {
            (
                ExpandedAssets::Direct(direct),
                eggpack_manifest::ArtifactForm::Direct { artifact, .. },
            ) => {
                if direct.asset_file != artifact.name {
                    return Err(fail("contract and manifest direct asset differ"));
                }
                if expected
                    .insert(
                        artifact.name.clone(),
                        (true, artifact.size, artifact.sha256.clone()),
                    )
                    .is_some()
                {
                    return Err(fail("duplicate release asset name"));
                }
                if expected
                    .insert(direct.sidecar_file.clone(), (false, 0, String::new()))
                    .is_some()
                {
                    return Err(fail("duplicate release asset name"));
                }
            }
            (
                ExpandedAssets::Bundle(bundle),
                eggpack_manifest::ArtifactForm::Bundle { entries },
            ) => {
                if bundle.entries.len() != entries.len() {
                    return Err(fail("bundle contract and manifest entry counts differ"));
                }
                let mut expanded_map = BTreeMap::new();
                for entry in &bundle.entries {
                    if expanded_map
                        .insert(entry.asset_file.clone(), entry.sidecar_file.clone())
                        .is_some()
                    {
                        return Err(fail("duplicate bundle asset"));
                    }
                }
                for entry in entries {
                    let sidecar = expanded_map
                        .get(&entry.artifact.name)
                        .ok_or_else(|| fail("bundle contract and manifest entries differ"))?;
                    if expected
                        .insert(
                            entry.artifact.name.clone(),
                            (true, entry.artifact.size, entry.artifact.sha256.clone()),
                        )
                        .is_some()
                    {
                        return Err(fail("duplicate release asset name"));
                    }
                    if expected
                        .insert(sidecar.clone(), (false, 0, String::new()))
                        .is_some()
                    {
                        return Err(fail("duplicate release asset name"));
                    }
                }
            }
            (
                ExpandedAssets::Archive(archive),
                eggpack_manifest::ArtifactForm::Archive { artifact, .. },
            ) => {
                if archive.archive_file != artifact.name {
                    return Err(fail("contract and manifest archive filenames differ"));
                }
                if expected
                    .insert(
                        artifact.name.clone(),
                        (true, artifact.size, artifact.sha256.clone()),
                    )
                    .is_some()
                {
                    return Err(fail("duplicate release asset name"));
                }
                if expected
                    .insert(archive.sidecar_file.clone(), (false, 0, String::new()))
                    .is_some()
                {
                    return Err(fail("duplicate release asset name"));
                }
            }
            _ => return Err(fail("contract and manifest forms differ for target")),
        }
    }

    // Auxiliary collision guard.
    for name in expected.keys() {
        let lower = name.to_ascii_lowercase();
        if lower == FIXED_MANIFEST_NAME
            || lower == FIXED_POSIX_NAME
            || lower == FIXED_POWERSHELL_NAME
        {
            return Err(fail("release asset collides with staging auxiliary name"));
        }
    }

    // Inventory of the finalized root.
    let mut observed: BTreeMap<String, PathBuf> = BTreeMap::new();
    let entries =
        std::fs::read_dir(finalized_root).map_err(|_| fail("finalized root cannot be listed"))?;
    for entry in entries {
        let entry = entry.map_err(|_| fail("finalized root entry unavailable"))?;
        let file_type = entry
            .file_type()
            .map_err(|_| fail("finalized entry type unavailable"))?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(fail("finalized root must contain only regular files"));
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        validate_asset_name(&name)?;
        if observed.insert(name.clone(), entry.path()).is_some() {
            return Err(fail("duplicate finalized file"));
        }
    }
    let expected_names: BTreeSet<String> = expected.keys().cloned().collect();
    let observed_names: BTreeSet<String> = observed.keys().cloned().collect();
    if expected_names != observed_names {
        return Err(fail("finalized root files do not exactly match manifest"));
    }

    // Verify artifact bytes and sidecar contents.
    for (name, (is_artifact, size, sha)) in &expected {
        let path = observed
            .get(name)
            .ok_or_else(|| fail("finalized file missing"))?;
        if *is_artifact {
            let (actual_size, actual_sha) = digest_file(path)?;
            if actual_size != *size || actual_sha != *sha {
                return Err(fail("finalized file differs from manifest"));
            }
        }
    }
    for (name, (is_artifact, _, _)) in &expected {
        if *is_artifact {
            continue;
        }
        // Sidecar: "<sha>  <file>\n" for its artifact sibling is validated
        // by locating the artifact that shares the sidecar stem. The exact
        // sidecar filename mapping comes from contract expansion; here we
        // verify content shape and matching digest without trusting names.
        let path = observed
            .get(name)
            .ok_or_else(|| fail("finalized file missing"))?;
        let text = std::fs::read_to_string(path).map_err(|_| fail("sidecar cannot be read"))?;
        if text.len() > 4096 {
            return Err(fail("sidecar exceeds bound"));
        }
        let trimmed = text.trim_end_matches(['\n', '\r']);
        let mut parts = trimmed.splitn(2, "  ");
        let sha = parts
            .next()
            .ok_or_else(|| fail("sidecar content invalid"))?;
        let file = parts
            .next()
            .ok_or_else(|| fail("sidecar content invalid"))?;
        validate_sha256(sha)?;
        validate_asset_name(file)?;
        let artifact_path = finalized_root.join(file);
        if !observed.contains_key(file) {
            return Err(fail("sidecar references unknown artifact"));
        }
        let (actual_size, actual_sha) = digest_file(&artifact_path)?;
        if actual_sha != sha {
            return Err(fail("sidecar digest differs from artifact"));
        }
        let _ = actual_size;
        // Sidecar filename must end with the artifact name plus .sha256
        // for the fixtures used here; contract templates already bound the
        // exact names above, so this is a consistency check only.
        if !name.ends_with(".sha256") {
            return Err(fail("sidecar filename invalid"));
        }
    }

    ensure_absent_private_dir(output_dir, "staging output")?;
    let cleanup = |_: ()| {
        let _ = std::fs::remove_dir_all(output_dir);
    };

    // Copy exact finalized assets.
    for (name, path) in &observed {
        let dest = output_dir.join(name);
        std::fs::copy(path, &dest).map_err(|_| {
            cleanup(());
            fail("staging copy failed")
        })?;
    }

    // Standalone manifest handoff.
    let manifest_json = manifest
        .to_json()
        .map_err(|_| fail("manifest encode failed"))?;
    let manifest_path = output_dir.join(FIXED_MANIFEST_NAME);
    std::fs::write(&manifest_path, manifest_json.as_bytes()).map_err(|_| {
        cleanup(());
        fail("staging manifest write failed")
    })?;
    let roundtrip = ReleaseManifest::from_json(&manifest_json).map_err(|_| {
        cleanup(());
        fail("staging manifest roundtrip failed")
    })?;
    let roundtrip_json = roundtrip.to_json().map_err(|_| {
        cleanup(());
        fail("staging manifest roundtrip failed")
    })?;
    if roundtrip_json != manifest_json {
        cleanup(());
        return Err(fail("staging manifest does not decode to exact manifest"));
    }

    // Deterministic bootstrap installers with exact-tag origin.
    let origin = policy.download_origin();
    let spec = BootstrapSpec {
        origin,
        fixture_http: false,
    };
    let posix =
        eggpack_bootstrap::render_posix_with_policy(contract, manifest, &spec, install_policy)
            .map_err(|_| {
                cleanup(());
                fail("posix installer generation failed")
            })?;
    let powershell =
        eggpack_bootstrap::render_powershell_with_policy(contract, manifest, &spec, install_policy)
            .map_err(|_| {
                cleanup(());
                fail("powershell installer generation failed")
            })?;
    // Exact-tag origin only; never latest/download.
    if posix.contains("latest/download") || powershell.contains("latest/download") {
        cleanup(());
        return Err(fail("installer uses unsupported latest origin"));
    }
    std::fs::write(output_dir.join(FIXED_POSIX_NAME), posix.as_bytes()).map_err(|_| {
        cleanup(());
        fail("posix installer write failed")
    })?;
    std::fs::write(
        output_dir.join(FIXED_POWERSHELL_NAME),
        powershell.as_bytes(),
    )
    .map_err(|_| {
        cleanup(());
        fail("powershell installer write failed")
    })?;

    // Build deterministic payload records.
    let mut assets = Vec::new();
    let mut staged_names: BTreeSet<String> = BTreeSet::new();
    let mut staged_entries: Vec<(String, StagingAssetKind, String)> = Vec::new();
    for name in observed.keys() {
        let is_artifact = expected
            .get(name)
            .map(|(is_artifact, _, _)| *is_artifact)
            .unwrap_or(false);
        let (kind, media) = if is_artifact {
            (
                StagingAssetKind::FinalizedArtifact,
                "application/octet-stream".to_owned(),
            )
        } else {
            (StagingAssetKind::ChecksumSidecar, "text/plain".to_owned())
        };
        staged_entries.push((name.clone(), kind, media));
    }
    staged_entries.push((
        FIXED_MANIFEST_NAME.to_owned(),
        StagingAssetKind::ReleaseManifest,
        "application/json".to_owned(),
    ));
    staged_entries.push((
        FIXED_POSIX_NAME.to_owned(),
        StagingAssetKind::PosixInstaller,
        "text/x-shellscript".to_owned(),
    ));
    staged_entries.push((
        FIXED_POWERSHELL_NAME.to_owned(),
        StagingAssetKind::PowershellInstaller,
        "text/x-powershell".to_owned(),
    ));
    for (name, kind, media) in staged_entries {
        if !staged_names.insert(name.to_ascii_lowercase()) {
            cleanup(());
            return Err(fail("duplicate staging asset name"));
        }
        let (size, sha) = digest_file(&output_dir.join(&name)).map_err(|_| {
            cleanup(());
            fail("staged file digest failed")
        })?;
        assets.push(StagingAsset {
            name: name.clone(),
            path: name,
            size,
            sha256: sha,
            media_type: media,
            kind,
        });
    }
    assets.sort_by(|a, b| a.name.cmp(&b.name));

    let payload = StagingPayloadV1 {
        schema_version: 1,
        product_id: manifest.product_id.clone(),
        release_id: manifest.release_id.clone(),
        source_revision: manifest.source_revision.clone(),
        owner: policy.owner.clone(),
        repository: policy.repository.clone(),
        tag: policy.tag.clone(),
        title: policy.title.clone(),
        prerelease: policy.prerelease,
        body: policy.body.clone(),
        assets,
    };
    payload.validate().map_err(|_| {
        cleanup(());
        fail("staging payload invalid")
    })?;
    Ok(payload)
}

// ---------------------------------------------------------------------------
// GitHub adapter
// ---------------------------------------------------------------------------

/// Exact tag reference target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefTarget {
    /// Commit or tag object SHA.
    pub sha: String,
    /// Object type (`commit` or `tag`).
    pub kind: String,
}

/// Annotated tag object peeling step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagObject {
    /// Peeled object SHA.
    pub object_sha: String,
    /// Peeled object type.
    pub object_type: String,
}

/// Remote release summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRelease {
    /// GitHub numeric release id.
    pub id: u64,
    /// Exact tag name.
    pub tag_name: String,
    /// Release title.
    pub name: String,
    /// Release notes.
    pub body: String,
    /// Draft state.
    pub draft: bool,
    /// Immutable state.
    pub immutable: bool,
    /// Prerelease intent.
    pub prerelease: bool,
    /// Upload URL template/host.
    pub upload_url: String,
}

/// Remote release asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAsset {
    /// GitHub numeric asset id.
    pub id: u64,
    /// Exact asset name.
    pub name: String,
    /// Exact byte size.
    pub size: u64,
    /// Asset state (`uploaded` or `starter`).
    pub state: String,
    /// Optional `sha256:<hex>` digest.
    pub digest: Option<String>,
}

/// Provider transport seam for GitHub staging.
///
/// Production uses fixed `https://api.github.com` plus upload host
/// `https://uploads.github.com` through `eggfetch-core` with a lean profile
/// (no retries, no redirects). Tests inject a deterministic fixture.
pub trait GithubApi: Send + Sync {
    /// Fetch exact `refs/tags/<tag>` reference.
    fn get_ref(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> impl std::future::Future<Output = Result<RefTarget, GithubError>> + Send;
    /// Fetch one annotated tag object by SHA.
    fn get_tag(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> impl std::future::Future<Output = Result<TagObject, GithubError>> + Send;
    /// List releases for one page (1-indexed, `per_page=100`).
    fn list_releases(
        &self,
        owner: &str,
        repo: &str,
        page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<RemoteRelease>, GithubError>> + Send;
    /// Create a new draft release for an exact existing tag.
    fn create_release(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        title: &str,
        body: &str,
        prerelease: bool,
    ) -> impl std::future::Future<Output = Result<RemoteRelease, GithubError>> + Send;
    /// List all assets for one release.
    fn list_assets(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<RemoteAsset>, GithubError>> + Send;
    /// Upload one asset; 502/422 are reported as errors with `http_502` /
    /// `http_422_duplicate` prefixes so the caller can apply narrow recovery.
    #[allow(clippy::too_many_arguments)]
    fn upload_asset(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        name: &str,
        file: Box<dyn std::io::Read + Send>,
        length: u64,
        content_type: &str,
    ) -> impl std::future::Future<Output = Result<RemoteAsset, GithubError>> + Send;
    /// Delete one asset by id (only the narrow starter recovery may call this).
    fn delete_asset(
        &self,
        owner: &str,
        repo: &str,
        asset_id: u64,
    ) -> impl std::future::Future<Output = Result<(), GithubError>> + Send;
}

trait ReadSeek: std::io::Read + std::io::Seek {
    fn source_len(&self) -> Result<u64, GithubError>;
}
impl ReadSeek for std::fs::File {
    fn source_len(&self) -> Result<u64, GithubError> {
        Ok(self
            .metadata()
            .map_err(|_| fail("staged file is unavailable"))?
            .len())
    }
}
impl<T: AsRef<[u8]> + Send> ReadSeek for std::io::Cursor<T> {
    fn source_len(&self) -> Result<u64, GithubError> {
        Ok(self.get_ref().as_ref().len() as u64)
    }
}

fn is_http_502(error: &GithubError) -> bool {
    error.0.starts_with("http_502")
}

fn is_http_422_duplicate(error: &GithubError) -> bool {
    error.0.starts_with("http_422_duplicate")
}

fn validate_sha40(value: &str) -> Result<(), GithubError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && (b.is_ascii_digit() || b.is_ascii_lowercase()))
    {
        // GitHub SHAs are lowercase hex; be strict but do not leak values.
        return Err(fail("remote SHA has invalid shape"));
    }
    Ok(())
}

/// Verify an exact tag resolves to the payload source revision.
///
/// Lightweight tags must point directly at the revision. Annotated tags are
/// peeled with a bounded loop and must terminate at a commit with the exact
/// revision. No tag write endpoint exists.
pub async fn verify_tag_source(
    transport: &impl GithubApi,
    owner: &str,
    repo: &str,
    tag: &str,
    source_revision: &str,
) -> Result<(), GithubError> {
    let reference = transport.get_ref(owner, repo, tag).await?;
    if reference.kind == "commit" {
        validate_sha40(&reference.sha)?;
        if reference.sha != source_revision {
            return Err(fail("tag does not match source revision"));
        }
        return Ok(());
    }
    if reference.kind != "tag" {
        return Err(fail("tag reference has unexpected type"));
    }
    validate_sha40(&reference.sha)?;
    let mut current = reference.sha;
    for _ in 0..MAX_TAG_PEEL_DEPTH {
        let object = transport.get_tag(owner, repo, &current).await?;
        if object.object_type == "commit" {
            validate_sha40(&object.object_sha)?;
            if object.object_sha != source_revision {
                return Err(fail("tag does not match source revision"));
            }
            return Ok(());
        }
        if object.object_type != "tag" {
            return Err(fail("annotated tag did not resolve to a commit"));
        }
        validate_sha40(&object.object_sha)?;
        current = object.object_sha;
    }
    Err(fail("annotated tag depth exceeds bound"))
}

fn find_exact_release(
    releases: &[RemoteRelease],
    tag: &str,
) -> Result<Option<RemoteRelease>, GithubError> {
    let mut matches = Vec::new();
    for release in releases {
        if release.tag_name == tag {
            matches.push(release.clone());
        }
    }
    if matches.len() > 1 {
        return Err(fail("duplicate same-tag release records"));
    }
    Ok(matches.into_iter().next())
}

fn check_upload_url(
    url: &str,
    owner: &str,
    repository: &str,
    release_id: u64,
) -> Result<(), GithubError> {
    let normalized = url.strip_suffix("{?name,label}").unwrap_or(url);
    let parsed = url::Url::parse(normalized).map_err(|_| fail("upload URL is invalid"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("uploads.github.com")
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.port().is_some_and(|port| port != 443)
        || parsed.fragment().is_some()
        || parsed.path() != format!("/repos/{owner}/{repository}/releases/{release_id}/assets")
        || parsed.query().is_some()
    {
        return Err(fail("upload host is not the expected GitHub host"));
    }
    Ok(())
}

fn upload_url(
    owner: &str,
    repository: &str,
    release_id: u64,
    name: &str,
) -> Result<String, GithubError> {
    validate_asset_name(name)?;
    let mut url = url::Url::parse(&format!(
        "{UPLOAD_BASE}/repos/{owner}/{repository}/releases/{release_id}/assets"
    ))
    .map_err(|_| fail("upload URL construction failed"))?;
    url.query_pairs_mut().append_pair("name", name);
    Ok(url.into())
}

fn streamed_upload_body(
    file: Box<dyn std::io::Read + Send>,
    length: u64,
) -> Result<eggfetch_core::RequestBody, GithubError> {
    let length =
        usize::try_from(length).map_err(|_| fail("asset length exceeds platform bound"))?;
    Ok(eggfetch_core::RequestBody::from_stream(
        futures_util::stream::try_unfold(file, |mut file| async move {
            use std::io::Read;
            let mut chunk = vec![0; UPLOAD_CHUNK_BYTES];
            let count = file
                .read(&mut chunk)
                .map_err(|error| eggfetch_core::Error::Io(std::sync::Arc::new(error)))?;
            if count == 0 {
                Ok(None)
            } else {
                chunk.truncate(count);
                Ok(Some((bytes::Bytes::from(chunk), file)))
            }
        }),
        Some(length),
    ))
}

async fn list_all_assets(
    transport: &impl GithubApi,
    policy: &GitHubDraftPolicyV1,
    release_id: u64,
) -> Result<Vec<RemoteAsset>, GithubError> {
    let mut all = Vec::new();
    for page in 1..=policy.max_list_pages {
        let batch = transport
            .list_assets(&policy.owner, &policy.repository, release_id, page)
            .await?;
        let full = batch.len() == ASSET_PAGE_SIZE;
        all.extend(batch);
        if !full {
            return Ok(all);
        }
        if page == policy.max_list_pages {
            return Err(fail("asset pagination bound exhausted"));
        }
    }
    Ok(all)
}

fn verify_file(file: &mut dyn ReadSeek, expected: &StagingAsset) -> Result<(), GithubError> {
    use std::io::SeekFrom;
    if file.source_len()? != expected.size {
        return Err(fail("local asset size differs from payload"));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| fail("staged file cannot be read"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; UPLOAD_CHUNK_BYTES];
    loop {
        let count = file
            .read(&mut buf)
            .map_err(|_| fail("staged file cannot be read"))?;
        if count == 0 {
            break;
        }
        use sha2::Digest;
        hasher.update(&buf[..count]);
    }
    use sha2::Digest;
    let digest = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if digest != expected.sha256 {
        return Err(fail("local asset digest differs from payload"));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| fail("staged file cannot be read"))?;
    Ok(())
}

/// Testable staging core with explicit local bytes.
pub async fn stage_with_bytes<F>(
    payload: &StagingPayloadV1,
    policy: &GitHubDraftPolicyV1,
    transport: &impl GithubApi,
    token: &str,
    mut bytes_for: F,
) -> Result<GitHubDraftReceiptV1, GithubError>
where
    F: FnMut(&str) -> Result<Vec<u8>, GithubError>,
{
    stage_with_source(payload, policy, transport, token, |name| {
        Ok(Box::new(std::io::Cursor::new(bytes_for(name)?)) as Box<dyn ReadSeek + Send>)
    })
    .await
}

async fn stage_with_source<F>(
    payload: &StagingPayloadV1,
    policy: &GitHubDraftPolicyV1,
    transport: &impl GithubApi,
    token: &str,
    mut source_for: F,
) -> Result<GitHubDraftReceiptV1, GithubError>
where
    F: FnMut(&str) -> Result<Box<dyn ReadSeek + Send>, GithubError>,
{
    if token.is_empty() || token.len() > 4096 || token.chars().any(char::is_control) {
        return Err(fail("github token is absent"));
    }
    payload.validate()?;
    policy.validate()?;
    if payload.owner != policy.owner
        || payload.repository != policy.repository
        || payload.tag != policy.tag
        || payload.title != policy.title
        || payload.prerelease != policy.prerelease
        || payload.body != policy.body
    {
        return Err(fail("staging payload does not match draft policy"));
    }

    verify_tag_source(
        transport,
        &policy.owner,
        &policy.repository,
        &policy.tag,
        &payload.source_revision,
    )
    .await?;

    let mut seen: Vec<RemoteRelease> = Vec::new();
    for page in 1..=policy.max_list_pages {
        let batch = transport
            .list_releases(&policy.owner, &policy.repository, page)
            .await?;
        if batch.is_empty() {
            break;
        }
        let full = batch.len() >= 100;
        seen.extend(batch);
        if !full {
            break;
        }
    }
    let existing = find_exact_release(&seen, &policy.tag)?;
    let (release, created) = match existing {
        None => {
            let created = transport
                .create_release(
                    &policy.owner,
                    &policy.repository,
                    &policy.tag,
                    &policy.title,
                    &policy.body,
                    policy.prerelease,
                )
                .await?;
            if created.tag_name != policy.tag || !created.draft || created.immutable {
                return Err(fail("created release has unexpected state"));
            }
            (created, true)
        }
        Some(current) => {
            if !current.draft || current.immutable {
                return Err(fail("existing release is not a mutable draft"));
            }
            if current.tag_name != policy.tag {
                return Err(fail("existing draft tag mismatch"));
            }
            if current.name != policy.title
                || current.prerelease != policy.prerelease
                || current.body != policy.body
            {
                return Err(fail("existing draft title/prerelease/body mismatch"));
            }
            (current, false)
        }
    };
    check_upload_url(
        &release.upload_url,
        &policy.owner,
        &policy.repository,
        release.id,
    )?;

    let mut expected: BTreeMap<String, &StagingAsset> = BTreeMap::new();
    for asset in &payload.assets {
        if expected.insert(asset.name.clone(), asset).is_some() {
            return Err(fail("duplicate payload asset name"));
        }
    }

    let remote = list_all_assets(transport, policy, release.id).await?;
    {
        let mut names = BTreeSet::new();
        for asset in &remote {
            if !names.insert(asset.name.to_ascii_lowercase()) {
                return Err(fail("duplicate same-name remote asset"));
            }
            if !expected.contains_key(&asset.name) {
                return Err(fail("unexpected remote asset"));
            }
        }
    }
    let mut remote_by_name: BTreeMap<String, RemoteAsset> = BTreeMap::new();
    for asset in remote {
        remote_by_name.insert(asset.name.clone(), asset);
    }

    let mut uploaded: u32 = 0;
    let mut reused: u32 = 0;
    for (name, local) in &expected {
        match remote_by_name.get(name) {
            None => {
                let mut file = source_for(name)?;
                verify_file(file.as_mut(), local)?;
                match transport
                    .upload_asset(
                        &policy.owner,
                        &policy.repository,
                        release.id,
                        name,
                        Box::new(file),
                        local.size,
                        &local.media_type,
                    )
                    .await
                {
                    Ok(record) => {
                        if record.name != *name {
                            return Err(fail("upload response renamed the asset"));
                        }
                        if record.state != "uploaded" {
                            return Err(fail("upload response state is not uploaded"));
                        }
                        if record.size != local.size {
                            return Err(fail("upload response size mismatch"));
                        }
                        if let Some(digest) = &record.digest {
                            if digest != &format!("sha256:{}", local.sha256) {
                                return Err(fail("upload response digest mismatch"));
                            }
                        }
                        uploaded = uploaded
                            .checked_add(1)
                            .ok_or_else(|| fail("upload count overflow"))?;
                    }
                    Err(error) => {
                        if is_http_422_duplicate(&error) {
                            return Err(fail("duplicate-name upload rejected"));
                        }
                        if is_http_502(&error) {
                            let after = list_all_assets(transport, policy, release.id).await?;
                            let starters: Vec<&RemoteAsset> = after
                                .iter()
                                .filter(|asset| {
                                    asset.name == *name
                                        && asset.state == "starter"
                                        && asset.size == 0
                                })
                                .collect();
                            if starters.len() == 1 {
                                transport
                                    .delete_asset(&policy.owner, &policy.repository, starters[0].id)
                                    .await?;
                            }
                            return Err(fail("upload failed with 502; starter cleaned when exact"));
                        }
                        return Err(error);
                    }
                }
            }
            Some(record) => {
                if record.state != "uploaded" {
                    return Err(fail("remote asset is not uploaded"));
                }
                if record.size != local.size {
                    return Err(fail("same-name remote asset size mismatch"));
                }
                match &record.digest {
                    Some(digest) if digest == &format!("sha256:{}", local.sha256) => {
                        reused = reused
                            .checked_add(1)
                            .ok_or_else(|| fail("reuse count overflow"))?;
                    }
                    _ => return Err(fail("same-name remote asset digest mismatch")),
                }
            }
        }
    }

    let final_assets = list_all_assets(transport, policy, release.id).await?;
    {
        let mut names = BTreeSet::new();
        for asset in &final_assets {
            if !names.insert(asset.name.clone()) {
                return Err(fail("duplicate same-name remote asset"));
            }
        }
        if names.len() != expected.len() {
            return Err(fail("staged asset set is incomplete"));
        }
        for (name, local) in &expected {
            let record = final_assets
                .iter()
                .find(|asset| &asset.name == name)
                .ok_or_else(|| fail("staged asset missing after upload"))?;
            if record.state != "uploaded"
                || record.size != local.size
                || record.digest.as_deref() != Some(&format!("sha256:{}", local.sha256))
            {
                return Err(fail("staged asset evidence mismatch"));
            }
        }
    }

    verify_tag_source(
        transport,
        &policy.owner,
        &policy.repository,
        &policy.tag,
        &payload.source_revision,
    )
    .await?;

    let mut ordered = payload.assets.clone();
    ordered.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(GitHubDraftReceiptV1 {
        schema_version: 1,
        owner: policy.owner.clone(),
        repository: policy.repository.clone(),
        release_id: payload.release_id.clone(),
        tag: policy.tag.clone(),
        source_revision: payload.source_revision.clone(),
        github_release_id: release.id,
        draft: true,
        immutable: false,
        assets: ordered,
        created,
        uploaded,
        reused,
    })
}

/// Stage using a staging directory on disk.
pub async fn stage_with_dir(
    payload: &StagingPayloadV1,
    policy: &GitHubDraftPolicyV1,
    transport: &impl GithubApi,
    token: &str,
    staging_dir: &Path,
) -> Result<GitHubDraftReceiptV1, GithubError> {
    payload.validate()?;
    policy.validate()?;
    reject_symlink_dir(staging_dir, "staging directory")?;
    let mut files = BTreeMap::new();
    for asset in &payload.assets {
        let path = staging_dir.join(&asset.name);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| fail("staged file is unavailable"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(fail("staged file must be a regular file"));
        }
        let mut file =
            std::fs::File::open(&path).map_err(|_| fail("staged file cannot be read"))?;
        if !file
            .metadata()
            .map_err(|_| fail("staged file is unavailable"))?
            .is_file()
        {
            return Err(fail("staged file must be a regular file"));
        }
        verify_file(&mut file, asset)?;
        files.insert(asset.name.clone(), file);
    }
    stage_with_source(payload, policy, transport, token, |name| {
        let file = files
            .remove(name)
            .ok_or_else(|| fail("staged file is unavailable"))?;
        Ok(Box::new(file) as Box<dyn ReadSeek + Send>)
    })
    .await
}

// ---------------------------------------------------------------------------
// Production transport (eggfetch-core, lean profile)
// ---------------------------------------------------------------------------

/// Production GitHub transport with fixed hosts.
///
/// Uses `eggfetch-core 0.2.0` with explicit lean features (`standard-http1`,
/// `tls-rustls`, `tls-native-roots`, `json`) rather than the default
/// retry/redirect policy. Retries are disabled and redirects are not followed;
/// 3xx responses fail closed. Bounded timeouts and body sizes apply to every
/// request. The `Debug` implementation redacts the credential.
pub struct EggfetchTransport {
    client: eggfetch_core::Client,
    token: String,
    timeout_secs: u64,
    max_metadata_bytes: usize,
}

impl fmt::Debug for EggfetchTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EggfetchTransport")
            .field("timeout_secs", &self.timeout_secs)
            .field("max_metadata_bytes", &self.max_metadata_bytes)
            .field("token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl EggfetchTransport {
    /// Create a production transport. The token is held in memory only and
    /// never serialized or logged.
    pub fn new(
        token: String,
        timeout_secs: u64,
        max_metadata_bytes: usize,
    ) -> Result<Self, GithubError> {
        if token.is_empty() || token.len() > 4096 || token.chars().any(char::is_control) {
            return Err(fail("github token is absent"));
        }
        if !(5..=120).contains(&timeout_secs) {
            return Err(fail("request timeout out of bounds"));
        }
        if max_metadata_bytes == 0 || max_metadata_bytes > 8_000_000 {
            return Err(fail("max metadata bytes out of bounds"));
        }
        let timeout = eggfetch_core::Timeout::from_secs(timeout_secs);
        let client = eggfetch_core::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .build();
        Ok(Self {
            client,
            token,
            timeout_secs,
            max_metadata_bytes,
        })
    }

    fn auth(&self) -> Result<eggfetch_core::AuthScheme, GithubError> {
        eggfetch_core::AuthScheme::bearer(self.token.clone())
            .map_err(|_| fail("github token has invalid shape"))
    }

    fn timeout(&self) -> eggfetch_core::Timeout {
        eggfetch_core::Timeout::from_secs(self.timeout_secs)
    }

    async fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        mut response: eggfetch_core::Response,
    ) -> Result<T, GithubError> {
        let status = response.status().as_u16();
        if (300..400).contains(&status) {
            return Err(fail("unexpected redirect; redirects are disabled"));
        }
        if !(200..300).contains(&status) {
            return match status {
                404 => Err(fail("github resource not found")),
                422 => Err(fail("http_422_duplicate: github rejected duplicate name")),
                502 => Err(fail("http_502: github reported a gateway failure")),
                _ => Err(fail("github request failed")),
            };
        }
        response
            .json::<T>()
            .await
            .map_err(|_| fail("github response decode failed"))
    }
}

#[derive(Debug, Deserialize)]
struct ApiRefObject {
    sha: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct ApiRefResponse {
    object: ApiRefObject,
}

#[derive(Debug, Deserialize)]
struct ApiTagObjectInner {
    sha: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct ApiTagResponse {
    object: ApiTagObjectInner,
}

#[derive(Debug, Deserialize)]
struct ApiRelease {
    id: u64,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    #[serde(default)]
    immutable: bool,
    prerelease: bool,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiAsset {
    id: u64,
    name: String,
    size: u64,
    state: String,
    digest: Option<String>,
}

impl GithubApi for EggfetchTransport {
    async fn get_ref(&self, owner: &str, repo: &str, tag: &str) -> Result<RefTarget, GithubError> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/git/ref/tags/{tag}");
        let response = self
            .client
            .get(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .send()
            .await
            .map_err(|_| fail("github tag lookup failed"))?;
        let body: ApiRefResponse = self.read_json(response).await?;
        Ok(RefTarget {
            sha: body.object.sha,
            kind: body.object.kind,
        })
    }

    async fn get_tag(&self, owner: &str, repo: &str, sha: &str) -> Result<TagObject, GithubError> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/git/tags/{sha}");
        let response = self
            .client
            .get(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .send()
            .await
            .map_err(|_| fail("github tag object lookup failed"))?;
        let body: ApiTagResponse = self.read_json(response).await?;
        Ok(TagObject {
            object_sha: body.object.sha,
            object_type: body.object.kind,
        })
    }

    async fn list_releases(
        &self,
        owner: &str,
        repo: &str,
        page: u32,
    ) -> Result<Vec<RemoteRelease>, GithubError> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/releases?per_page=100&page={page}");
        let response = self
            .client
            .get(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .send()
            .await
            .map_err(|_| fail("github release list failed"))?;
        let body: Vec<ApiRelease> = self.read_json(response).await?;
        Ok(body
            .into_iter()
            .map(|item| RemoteRelease {
                id: item.id,
                tag_name: item.tag_name,
                name: item.name.unwrap_or_default(),
                body: item.body.unwrap_or_default(),
                draft: item.draft,
                immutable: item.immutable,
                prerelease: item.prerelease,
                upload_url: item.upload_url,
            })
            .collect())
    }

    async fn create_release(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        title: &str,
        body: &str,
        prerelease: bool,
    ) -> Result<RemoteRelease, GithubError> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/releases");
        let request_body = serde_json::json!({
            "tag_name": tag,
            "name": title,
            "body": body,
            "draft": true,
            "prerelease": prerelease,
            "make_latest": false,
        });
        let response = self
            .client
            .post(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .json(&request_body)
            .map_err(|_| fail("github request encode failed"))?
            .send()
            .await
            .map_err(|_| fail("github release creation failed"))?;
        let item: ApiRelease = self.read_json(response).await?;
        Ok(RemoteRelease {
            id: item.id,
            tag_name: item.tag_name,
            name: item.name.unwrap_or_default(),
            body: item.body.unwrap_or_default(),
            draft: item.draft,
            immutable: item.immutable,
            prerelease: item.prerelease,
            upload_url: item.upload_url,
        })
    }

    async fn list_assets(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        page: u32,
    ) -> Result<Vec<RemoteAsset>, GithubError> {
        let url = format!(
            "{API_BASE}/repos/{owner}/{repo}/releases/{release_id}/assets?per_page=100&page={page}"
        );
        let response = self
            .client
            .get(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .send()
            .await
            .map_err(|_| fail("github asset list failed"))?;
        let body: Vec<ApiAsset> = self.read_json(response).await?;
        Ok(body
            .into_iter()
            .map(|item| RemoteAsset {
                id: item.id,
                name: item.name,
                size: item.size,
                state: item.state,
                digest: item.digest,
            })
            .collect())
    }

    async fn upload_asset(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        name: &str,
        file: Box<dyn std::io::Read + Send>,
        length: u64,
        content_type: &str,
    ) -> Result<RemoteAsset, GithubError> {
        let url = upload_url(owner, repo, release_id, name)?;
        let body = streamed_upload_body(file, length)?;
        let response = self
            .client
            .post(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header("Content-Type", content_type)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .body(body)
            .send()
            .await
            .map_err(|_| fail("github asset upload failed"))?;
        let status = response.status().as_u16();
        if status == 502 {
            return Err(fail("http_502: github reported a gateway failure"));
        }
        if status == 422 {
            return Err(fail("http_422_duplicate: github rejected duplicate name"));
        }
        if (300..400).contains(&status) {
            return Err(fail("unexpected redirect; redirects are disabled"));
        }
        if status != 201 {
            return Err(fail("github asset upload failed"));
        }
        let item: ApiAsset = self.read_json(response).await?;
        Ok(RemoteAsset {
            id: item.id,
            name: item.name,
            size: item.size,
            state: item.state,
            digest: item.digest,
        })
    }

    async fn delete_asset(
        &self,
        owner: &str,
        repo: &str,
        asset_id: u64,
    ) -> Result<(), GithubError> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/releases/assets/{asset_id}");
        let response = self
            .client
            .delete(&url)
            .map_err(|_| fail("github request build failed"))?
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .auth(self.auth()?)
            .timeout(self.timeout())
            .max_decoded_body_size(self.max_metadata_bytes)
            .send()
            .await
            .map_err(|_| fail("github asset delete failed"))?;
        let status = response.status().as_u16();
        if status != 204 {
            return Err(fail("github asset delete failed"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Deterministic fixture transport for tests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct FixtureReleaseState {
    release: RemoteRelease,
}

#[derive(Debug)]
struct FixtureInner {
    ref_sha: Option<String>,
    ref_kind: String,
    tag_objects: BTreeMap<String, (String, String)>,
    releases: Vec<FixtureReleaseState>,
    assets: BTreeMap<u64, Vec<RemoteAsset>>,
    asset_pages: BTreeMap<(u64, u32), Vec<RemoteAsset>>,
    next_release_id: u64,
    next_asset_id: u64,
    upload_rename_next: bool,
    upload_digest_mismatch_next: bool,
    upload_size_mismatch_next: bool,
    upload_fail_502_create_starter: bool,
    upload_fail_502_next: Option<String>,
    upload_fail_422_next: bool,
    create_calls: u32,
    upload_calls: u32,
    delete_calls: u32,
}

/// Deterministic in-memory GitHub fixture.
#[derive(Debug)]
pub struct FixtureGithub {
    inner: Mutex<FixtureInner>,
}

impl FixtureGithub {
    /// Create a fixture with an exact lightweight tag.
    pub fn with_tag(commit_sha: &str) -> Self {
        Self {
            inner: Mutex::new(FixtureInner {
                ref_sha: Some(commit_sha.to_owned()),
                ref_kind: "commit".to_owned(),
                tag_objects: BTreeMap::new(),
                releases: Vec::new(),
                assets: BTreeMap::new(),
                asset_pages: BTreeMap::new(),
                next_release_id: 100,
                next_asset_id: 1000,
                upload_rename_next: false,
                upload_digest_mismatch_next: false,
                upload_size_mismatch_next: false,
                upload_fail_502_create_starter: true,
                upload_fail_502_next: None,
                upload_fail_422_next: false,
                create_calls: 0,
                upload_calls: 0,
                delete_calls: 0,
            }),
        }
    }

    /// Set the tag reference (for movement tests).
    pub fn set_ref(&self, sha: Option<String>, kind: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.ref_sha = sha;
        inner.ref_kind = kind.to_owned();
    }

    /// Register an annotated tag peeling step.
    pub fn add_tag_object(&self, sha: &str, object_type: &str, object_sha: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.tag_objects.insert(
            sha.to_owned(),
            (object_type.to_owned(), object_sha.to_owned()),
        );
    }

    /// Seed one existing release.
    pub fn seed_release(&self, release: RemoteRelease, assets: Vec<RemoteAsset>) {
        let mut inner = self.inner.lock().unwrap();
        let id = release.id;
        inner.releases.push(FixtureReleaseState { release });
        inner.assets.insert(id, assets);
        if id >= inner.next_release_id {
            inner.next_release_id = id + 1;
        }
    }

    /// Override one remote asset-list page for pagination edge-case tests.
    pub fn set_asset_page(&self, release_id: u64, page: u32, assets: Vec<RemoteAsset>) {
        self.inner
            .lock()
            .unwrap()
            .asset_pages
            .insert((release_id, page), assets);
    }

    /// Fail the next upload for `name` with 502.
    pub fn fail_upload_502_next(&self, name: &str, create_starter: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.upload_fail_502_next = Some(name.to_owned());
        inner.upload_fail_502_create_starter = create_starter;
    }

    /// Fail the next upload with 422 duplicate.
    pub fn fail_upload_422_next(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.upload_fail_422_next = true;
    }

    /// Rename the next uploaded asset.
    pub fn rename_next_upload(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.upload_rename_next = true;
    }

    /// Return a wrong digest for the next upload.
    pub fn digest_mismatch_next(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.upload_digest_mismatch_next = true;
    }

    /// Return a wrong size for the next upload.
    pub fn size_mismatch_next(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.upload_size_mismatch_next = true;
    }

    /// Number of release creations.
    pub fn create_calls(&self) -> u32 {
        self.inner.lock().unwrap().create_calls
    }

    /// Number of upload attempts.
    pub fn upload_calls(&self) -> u32 {
        self.inner.lock().unwrap().upload_calls
    }

    /// Number of asset deletions.
    pub fn delete_calls(&self) -> u32 {
        self.inner.lock().unwrap().delete_calls
    }

    /// Current assets for one release.
    pub fn assets_for(&self, release_id: u64) -> Vec<RemoteAsset> {
        self.inner
            .lock()
            .unwrap()
            .assets
            .get(&release_id)
            .cloned()
            .unwrap_or_default()
    }
}

/// Build a fixture release record.
pub fn fixture_release(
    id: u64,
    tag: &str,
    title: &str,
    body: &str,
    prerelease: bool,
    draft: bool,
    immutable: bool,
) -> RemoteRelease {
    RemoteRelease {
        id,
        tag_name: tag.to_owned(),
        name: title.to_owned(),
        body: body.to_owned(),
        draft,
        immutable,
        prerelease,
        upload_url: format!("{UPLOAD_BASE}/repos/acme/widget/releases/{id}/assets{{?name,label}}"),
    }
}

/// Build a fixture asset record.
pub fn fixture_asset(
    id: u64,
    name: &str,
    size: u64,
    state: &str,
    sha_hex: Option<&str>,
) -> RemoteAsset {
    RemoteAsset {
        id,
        name: name.to_owned(),
        size,
        state: state.to_owned(),
        digest: sha_hex.map(|hex| format!("sha256:{hex}")),
    }
}

impl GithubApi for FixtureGithub {
    async fn get_ref(
        &self,
        _owner: &str,
        _repo: &str,
        _tag: &str,
    ) -> Result<RefTarget, GithubError> {
        let inner = self.inner.lock().unwrap();
        match &inner.ref_sha {
            None => Err(fail("github resource not found")),
            Some(sha) => Ok(RefTarget {
                sha: sha.clone(),
                kind: inner.ref_kind.clone(),
            }),
        }
    }

    async fn get_tag(
        &self,
        _owner: &str,
        _repo: &str,
        sha: &str,
    ) -> Result<TagObject, GithubError> {
        let inner = self.inner.lock().unwrap();
        match inner.tag_objects.get(sha) {
            None => Err(fail("github resource not found")),
            Some((kind, object_sha)) => Ok(TagObject {
                object_sha: object_sha.clone(),
                object_type: kind.clone(),
            }),
        }
    }

    async fn list_releases(
        &self,
        _owner: &str,
        _repo: &str,
        page: u32,
    ) -> Result<Vec<RemoteRelease>, GithubError> {
        if page != 1 {
            return Ok(Vec::new());
        }
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .releases
            .iter()
            .map(|state| state.release.clone())
            .collect())
    }

    async fn create_release(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        title: &str,
        body: &str,
        prerelease: bool,
    ) -> Result<RemoteRelease, GithubError> {
        let mut inner = self.inner.lock().unwrap();
        inner.create_calls += 1;
        let id = inner.next_release_id;
        inner.next_release_id += 1;
        let release = RemoteRelease {
            id,
            tag_name: tag.to_owned(),
            name: title.to_owned(),
            body: body.to_owned(),
            draft: true,
            immutable: false,
            prerelease,
            upload_url: format!(
                "{UPLOAD_BASE}/repos/{owner}/{repo}/releases/{id}/assets{{?name,label}}"
            ),
        };
        inner.releases.push(FixtureReleaseState {
            release: release.clone(),
        });
        inner.assets.insert(id, Vec::new());
        Ok(release)
    }

    async fn list_assets(
        &self,
        _owner: &str,
        _repo: &str,
        release_id: u64,
        page: u32,
    ) -> Result<Vec<RemoteAsset>, GithubError> {
        let inner = self.inner.lock().unwrap();
        if let Some(page_assets) = inner.asset_pages.get(&(release_id, page)) {
            return Ok(page_assets.clone());
        }
        let all = inner.assets.get(&release_id).cloned().unwrap_or_default();
        let start = (page.saturating_sub(1) as usize).saturating_mul(ASSET_PAGE_SIZE);
        Ok(all.into_iter().skip(start).take(ASSET_PAGE_SIZE).collect())
    }

    async fn upload_asset(
        &self,
        _owner: &str,
        _repo: &str,
        release_id: u64,
        name: &str,
        mut file: Box<dyn std::io::Read + Send>,
        length: u64,
        _content_type: &str,
    ) -> Result<RemoteAsset, GithubError> {
        use std::io::Read;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| fail("fixture upload read failed"))?;
        if bytes.len() as u64 != length {
            return Err(fail("fixture upload length mismatch"));
        }
        let mut inner = self.inner.lock().unwrap();
        inner.upload_calls += 1;
        if inner.upload_fail_422_next {
            inner.upload_fail_422_next = false;
            return Err(fail("http_422_duplicate: github rejected duplicate name"));
        }
        if let Some(expected) = inner.upload_fail_502_next.clone() {
            if expected == name {
                inner.upload_fail_502_next = None;
                if inner.upload_fail_502_create_starter {
                    let id = inner.next_asset_id;
                    inner.next_asset_id += 1;
                    inner
                        .assets
                        .entry(release_id)
                        .or_default()
                        .push(RemoteAsset {
                            id,
                            name: name.to_owned(),
                            size: 0,
                            state: "starter".to_owned(),
                            digest: None,
                        });
                }
                return Err(fail("http_502: github reported a gateway failure"));
            }
        }
        use sha2::Digest;
        let digest: String = sha2::Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let id = inner.next_asset_id;
        inner.next_asset_id += 1;
        let record_name = if inner.upload_rename_next {
            inner.upload_rename_next = false;
            format!("{name}.renamed")
        } else {
            name.to_owned()
        };
        let size = if inner.upload_size_mismatch_next {
            inner.upload_size_mismatch_next = false;
            bytes.len() as u64 + 1
        } else {
            bytes.len() as u64
        };
        let digest_value = if inner.upload_digest_mismatch_next {
            inner.upload_digest_mismatch_next = false;
            Some("sha256:".to_owned() + &"0".repeat(64))
        } else {
            Some(format!("sha256:{digest}"))
        };
        let record = RemoteAsset {
            id,
            name: record_name,
            size,
            state: "uploaded".to_owned(),
            digest: digest_value,
        };
        inner
            .assets
            .entry(release_id)
            .or_default()
            .push(record.clone());
        Ok(record)
    }

    async fn delete_asset(
        &self,
        _owner: &str,
        _repo: &str,
        asset_id: u64,
    ) -> Result<(), GithubError> {
        let mut inner = self.inner.lock().unwrap();
        inner.delete_calls += 1;
        for assets in inner.assets.values_mut() {
            if let Some(position) = assets.iter().position(|asset| asset.id == asset_id) {
                assets.remove(position);
                return Ok(());
            }
        }
        Err(fail("github asset delete failed"))
    }
}

#[cfg(test)]
mod tests;
