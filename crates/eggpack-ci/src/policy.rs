//! Provider policy: action pins, runner mappings, tool provisioning, and workflow policy documents.

use crate::error::{fail, CiError};
use crate::planning::CIPlan;
use crate::validate::{safe_identifier, safe_tool_version, validate_exact_tag};
use eggpack_core::{BuildStrategy, HostArch, HostOs};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

/// A full immutable GitHub action commit reference for one fixed action repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionPin {
    /// Full `owner/repository@40-hex-sha` reference.
    pub reference: String,
}

/// One provider policy runner label for a provider-neutral host capability.
///
/// Runner labels describe host OS/architecture capability only. When explicit
/// cross-tool provisioning (`GitHubPolicy::cross_tools`) is selected, the
/// `cargo_zigbuild`/`zig` booleans are not the qualification authority; the
/// provisioned exact tools are. When provisioning is absent
/// (`PreinstalledVerified` legacy mode), both booleans must be true for
/// CargoZigbuild hosts and both exact configured versions are still checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerMapping {
    /// Required host operating system family.
    pub os: HostOs,
    /// Required host architecture family.
    pub arch: HostArch,
    /// GitHub runner label selected by provider policy.
    pub label: String,
    /// The selected runner image has a preinstalled cargo-zigbuild executable.
    pub cargo_zigbuild: bool,
    /// The selected runner image has a preinstalled Zig executable.
    pub zig: bool,
}

/// SHA-256 digests for the finite set of official Zig archives (M005 §5/§7).
///
/// The Zig version itself comes from the resolved `TargetPolicy`, never from
/// this provider policy. No arbitrary archive URL or mirror is representable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZigOfficialArchiveV1 {
    /// Exact SHA-256 of the official Linux x86-64 archive (64 lowercase hex).
    pub linux_x86_64_sha256: String,
    /// Exact SHA-256 of the official Linux AArch64 archive (64 lowercase hex).
    pub linux_aarch64_sha256: String,
}

/// Finite cross-tool provisioning policy for CargoZigbuild jobs (M005 §5).
///
/// The provider policy controls *how* the finite approved tools are
/// provisioned; `ToolchainRequirement` controls *which exact versions* are
/// required. Acquisition paths are fixed: `cargo install --locked` for
/// cargo-zigbuild and official `ziglang.org` archives for Zig. No generic
/// package manager, arbitrary URL, or arbitrary setup command exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossToolProvisioningV1 {
    /// Bound on the `cargo install cargo-zigbuild` step, 1 to 60 minutes.
    pub cargo_install_timeout_minutes: u16,
    /// Bound on curl `--connect-timeout` for the Zig download, 5 to 300 secs.
    pub zig_connect_timeout_secs: u64,
    /// Bound on curl `--max-time` for the Zig download, 60 to 3600 secs.
    pub zig_max_time_secs: u64,
    /// Exact official Zig archive digests per supported host architecture.
    pub zig: ZigOfficialArchiveV1,
}

impl ZigOfficialArchiveV1 {
    /// Validate both digests are exact 64-character lowercase hex SHA-256.
    pub fn validate(&self) -> Result<(), CiError> {
        for digest in [&self.linux_x86_64_sha256, &self.linux_aarch64_sha256] {
            validate_sha256_hex(digest)
                .map_err(|_| fail("invalid Zig official archive SHA-256"))?;
        }
        Ok(())
    }

    /// Exact expected digest for a supported Linux host architecture.
    pub fn digest_for(&self, arch: HostArch) -> Result<&str, CiError> {
        match arch {
            HostArch::X86_64 => Ok(&self.linux_x86_64_sha256),
            HostArch::Aarch64 => Ok(&self.linux_aarch64_sha256),
            HostArch::Armv7 => Err(fail("unsupported Zig provisioning host architecture")),
        }
    }
}

impl CrossToolProvisioningV1 {
    /// Validate timeout bounds and both Zig digests.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.cargo_install_timeout_minutes == 0
            || self.cargo_install_timeout_minutes > 60
            || self.zig_connect_timeout_secs < 5
            || self.zig_connect_timeout_secs > 300
            || self.zig_max_time_secs < 60
            || self.zig_max_time_secs > 3600
        {
            return Err(fail("invalid cross-tool provisioning timeout bound"));
        }
        self.zig.validate()
    }
}

/// Official Zig archive file name for a supported Linux host (`M005 §7`).
///
/// Maps exactly: Linux x86-64 -> `zig-x86_64-linux-<version>.tar.xz`;
/// Linux AArch64 -> `zig-aarch64-linux-<version>.tar.xz`. Any other host or
/// an out-of-bounds version fails closed.
pub fn zig_archive_name(os: HostOs, arch: HostArch, version: &str) -> Result<String, CiError> {
    if os != HostOs::Linux || !safe_tool_version(version) {
        return Err(fail("unsupported Zig provisioning host or version"));
    }
    let arch_token = match arch {
        HostArch::X86_64 => "x86_64",
        HostArch::Aarch64 => "aarch64",
        HostArch::Armv7 => return Err(fail("unsupported Zig provisioning host architecture")),
    };
    Ok(format!("zig-{arch_token}-linux-{version}.tar.xz"))
}

/// Fixed official Zig download origin (`M005 §5/§8`).
///
/// Production source is fixed to `https://ziglang.org/download/<version>/...`.
/// No caller-supplied URL, mirror, or redirect target is representable.
pub fn zig_download_url(version: &str, arch: HostArch) -> Result<String, CiError> {
    let name = zig_archive_name(HostOs::Linux, arch, version)?;
    if !safe_tool_version(version) {
        return Err(fail("invalid Zig version for download URL"));
    }
    Ok(format!("https://ziglang.org/download/{version}/{name}"))
}

/// Expected SHA-256 for the configured Zig version on a build host.
///
/// The version comes from the resolved `TargetPolicy`; the digest comes from
/// the provider provisioning policy. The checked-in digest is the authority;
/// no remotely fetched checksum file is trusted.
pub fn zig_expected_digest(
    provisioning: &CrossToolProvisioningV1,
    host_os: HostOs,
    host_arch: HostArch,
) -> Result<&str, CiError> {
    if host_os != HostOs::Linux {
        return Err(fail("Zig provisioning requires a Linux build host"));
    }
    provisioning.zig.digest_for(host_arch)
}

/// Validate one hex SHA-256 digest string without trusting remote content.
///
/// The acceptance set is owned by `eggpack-manifest` (ADR-0006) and re-exported
/// by `eggpack-core`; this crate applies it rather than restating it.
pub fn validate_sha256_hex(digest: &str) -> Result<(), CiError> {
    eggpack_core::validate_sha256_hex(digest).map_err(|_| fail("invalid SHA-256 digest"))
}

/// Pure archive top-level layout check for an official Zig archive listing.
///
/// The expected layout is exactly one top-level directory containing the `zig`
/// executable (`<top>/zig`). Rejects empty listings, absolute entries,
/// traversal components, more than one top-level member, a missing `zig`
/// entry, or a top-level `zig` file without its parent directory. Symlink and
/// traversal rejection during extraction is enforced by the rendered script
/// and by [`validate_provisioned_zig_dir`]; this function covers the layout
/// shape that can be checked from a file listing without network access.
pub fn validate_zig_archive_listing(entries: &[String]) -> Result<String, CiError> {
    if entries.is_empty() || entries.len() > 4096 {
        return Err(fail("invalid Zig archive listing"));
    }
    let mut top: Option<String> = None;
    let mut has_zig = false;
    for entry in entries {
        if entry.is_empty()
            || entry.len() > 512
            || entry.starts_with('/')
            || entry.starts_with('\\')
            || entry.contains('\0')
        {
            return Err(fail("invalid Zig archive entry"));
        }
        let normalized = entry.strip_suffix('/').unwrap_or(entry);
        let mut components = normalized.split('/');
        let first = components
            .next()
            .ok_or_else(|| fail("invalid Zig archive entry"))?;
        if first.is_empty() || first == "." || first == ".." {
            return Err(fail("invalid Zig archive top-level layout"));
        }
        for component in components {
            if component.is_empty() || component == "." || component == ".." {
                return Err(fail("invalid Zig archive entry traversal"));
            }
        }
        match &top {
            None => top = Some(first.to_owned()),
            Some(expected) if expected == first => {}
            Some(_) => return Err(fail("unexpected Zig archive top-level layout")),
        }
        if normalized == format!("{first}/zig") {
            has_zig = true;
        }
    }
    let top = top.ok_or_else(|| fail("invalid Zig archive listing"))?;
    if !has_zig {
        return Err(fail("Zig archive listing misses the zig executable"));
    }
    Ok(top)
}

/// Validate an extracted provisioned Zig directory without network access.
///
/// Requires `<dir>/zig` to be a regular non-symlink file (executable bit is
/// enforced on Unix; on Windows a regular file suffices since the execute bit
/// is not tracked). Rejects missing entries, symlinked executables,
/// directories, empty files, and unexpected extra top-level members. The
/// caller passes the private extraction root whose single child is the
/// archive top-level directory.
pub fn validate_provisioned_zig_dir(root: &Path) -> Result<std::path::PathBuf, CiError> {
    let metadata =
        std::fs::symlink_metadata(root).map_err(|_| fail("provisioned Zig root unavailable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(fail("provisioned Zig root is not a private directory"));
    }
    let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(root)
        .map_err(|_| fail("provisioned Zig root unreadable"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|_| fail("provisioned Zig root unreadable"))?;
    if entries.len() != 1 {
        return Err(fail("unexpected Zig archive top-level layout"));
    }
    entries.sort();
    let top = entries
        .into_iter()
        .next()
        .ok_or_else(|| fail("unexpected Zig archive top-level layout"))?;
    let top_metadata = std::fs::symlink_metadata(&top)
        .map_err(|_| fail("unexpected Zig archive top-level layout"))?;
    if top_metadata.file_type().is_symlink() || !top_metadata.is_dir() {
        return Err(fail("unexpected Zig archive top-level layout"));
    }
    let zig = top.join("zig");
    let zig_metadata =
        std::fs::symlink_metadata(&zig).map_err(|_| fail("provisioned zig executable missing"))?;
    if zig_metadata.file_type().is_symlink() || !zig_metadata.is_file() || zig_metadata.len() == 0 {
        return Err(fail("provisioned zig executable is not a regular file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if zig_metadata.permissions().mode() & 0o111 == 0 {
            return Err(fail("provisioned zig executable is not executable"));
        }
    }
    Ok(zig)
}

/// Finite workflow trigger supported by the M001 renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTrigger {
    /// Generate on pushes.
    Push,
    /// Allow deliberate manual generation.
    WorkflowDispatch,
}

/// Finite checked-in policy for mapping an exact source tag to manifest ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseIdentityMode {
    /// Historical opaque exact tag behavior.
    ExactTag,
    /// Map exactly `vMAJOR.MINOR.PATCH` to `MAJOR.MINOR.PATCH`.
    VPrefixedStableSemver,
}

impl ReleaseIdentityMode {
    pub(crate) fn release_id(self, tag: &str) -> Result<String, CiError> {
        validate_exact_tag(tag)?;
        match self {
            Self::ExactTag => Ok(tag.to_owned()),
            Self::VPrefixedStableSemver => {
                let version = tag
                    .strip_prefix('v')
                    .ok_or_else(|| fail("stable semantic tag must start with v"))?;
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() != 3
                    || parts.iter().any(|part| {
                        part.is_empty()
                            || !part.bytes().all(|byte| byte.is_ascii_digit())
                            || (part.len() > 1 && part.starts_with('0'))
                            || part.parse::<u64>().is_err()
                    })
                {
                    return Err(fail("tag is not v-prefixed stable semantic version"));
                }
                Ok(version.to_owned())
            }
        }
    }
}

/// Bounded GitHub Actions policy supplied separately from CIPlan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubPolicy {
    /// Runner label used for lightweight graph preflight.
    pub preflight_runner: String,
    /// Exact host capability to GitHub label mappings.
    pub runners: Vec<RunnerMapping>,
    /// Immutable pin for `actions/checkout`.
    pub checkout: ActionPin,
    /// Immutable pin for `dtolnay/rust-toolchain`.
    pub rust_toolchain: ActionPin,
    /// Immutable pin for `actions/upload-artifact`.
    pub upload_artifact: ActionPin,
    /// Immutable pin for `actions/download-artifact`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_artifact: Option<ActionPin>,
    /// Finite workflow event set.
    pub triggers: Vec<WorkflowTrigger>,
    /// Per-job timeout in minutes, from 1 to 360.
    pub timeout_minutes: u16,
    /// Whether a newer ref run cancels an in-progress run in the same group.
    pub cancel_in_progress: bool,
    /// Internal candidate artifact retention in days, from 1 to 90.
    pub artifact_retention_days: u8,
    /// Pinned Eggpack runtime tool provisioning for M002 qualify/aggregate jobs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eggpack_tool: Option<EggpackToolPolicy>,
    /// Explicit repository-relative input paths for M002a executable wiring.
    ///
    /// Required for `render_release_github`; absent for M001 build-only
    /// rendering. Paths live in provider policy, not portable release identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_inputs: Option<GitHubReleaseInputsV1>,
    /// Finite explicit QEMU sysroot policy per canonical target for Emulated
    /// qualification (M002a section 5H). Repository-relative paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emulated_sysroots: Option<BTreeMap<String, String>>,
    /// Explicit staging settings, present only when the graph requests
    /// GitHub draft staging (M003b). Absent for M002a build-only rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<GitHubStagingPolicyV1>,
    /// Finite deterministic cross-tool provisioning for CargoZigbuild jobs
    /// (M005). Absent means `PreinstalledVerified` legacy mode: both exact
    /// configured versions are still checked but no installation occurs.
    /// Present means `Provisioned` mode: exact cargo-zigbuild and
    /// SHA-256-verified official Zig are installed into invocation-private
    /// directories before any product build. Historical documents without
    /// this field remain parseable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_tools: Option<CrossToolProvisioningV1>,
    /// Optional finite source-tag to manifest-id mapping for reusable release
    /// workflows. Absence preserves exact-tag identity and historical output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_identity_mode: Option<ReleaseIdentityMode>,
}

/// Finite Eggpack runtime tool provisioning for generated qualify/aggregate jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EggpackToolPolicy {
    /// Exact official Eggpack repository URL.
    pub repo: String,
    /// Exact 40-hex commit revision.
    pub revision: String,
    /// Cargo package providing the `eggpack` binary (exactly `eggpack-cli`).
    pub package: String,
    /// Bound install timeout in minutes, from 1 to 60.
    pub install_timeout_minutes: u16,
}

impl EggpackToolPolicy {
    /// Validate official repo, immutable revision, package, and timeout bound.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.repo != "https://github.com/eggstack/eggpack"
            || self.revision.len() != 40
            || !self.revision.bytes().all(|b| b.is_ascii_hexdigit())
            || self.package != "eggpack-cli"
            || self.install_timeout_minutes == 0
            || self.install_timeout_minutes > 60
        {
            return Err(fail("invalid Eggpack tool provisioning policy"));
        }
        Ok(())
    }
}

/// Explicit repository-relative input paths for generated release workflow CLI
/// invocations (M002a execution wiring corrective).
///
/// Every generated `eggpack ci` command references its file inputs explicitly;
/// no hidden repository discovery or globbing is permitted. Paths live in
/// renderer/provider policy, never in portable release identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubReleaseInputsV1 {
    /// Repository-relative DistributionContract TOML path.
    pub contract: String,
    /// Repository-relative ReleasePlan JSON path.
    pub release_plan: String,
    /// Repository-relative BuildBindingsV1 (TOML or JSON) path.
    pub build_bindings: String,
    /// Repository-relative QualificationBindingsV1 (TOML or JSON) path.
    pub qualification_bindings: String,
    /// Repository-relative ReleaseCIPlanV1 JSON path.
    pub ci_plan: String,
    /// Repository-relative PackConfig (TOML or JSON) path for M003d runtime
    /// identity resolution. Required only for reusable workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_config: Option<String>,
    /// Repository-relative GitHubDraftTemplateV1 JSON path for M003d runtime
    /// identity resolution. Required only for reusable workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_template: Option<String>,
    /// Repository-relative InstallerPresentationV1 (TOML or JSON) path for
    /// M003d product-wrapper staging. Required only when the stage job
    /// passes `--installer-presentation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_presentation: Option<String>,
    /// Repository-relative consumer validator map (TOML or JSON) path.
    /// Required only when the graph configures consumer validators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_validators: Option<String>,
}

/// Explicit repository-relative staging input paths for the generated draft
/// staging job (M003b). All paths are bounded relative paths resolved from the
/// repository root after checkout; no discovery or globbing is permitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubStagingInputsV1 {
    /// Repository-relative DistributionContract TOML path.
    pub contract: String,
    /// Repository-relative BootstrapInstallPolicyV1 (TOML or JSON) path.
    pub install_policy: String,
    /// Repository-relative GitHubDraftPolicyV1 JSON path.
    pub github_policy: String,
    /// Optional repository-relative InstallerPresentationV1 path (M003d).
    /// Absent for M003c-compatible staging (GeneratedDefault).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_presentation: Option<String>,
}

/// Finite staging runner label plus repository identity for the generated
/// draft staging job (M003b). The exact tag itself lives in the checked-in
/// GitHub draft policy file and at runtime in `github.ref_name` (tag push) or
/// the explicit `release_tag` dispatch input; the renderer records how the tag
/// is obtained, never a branch or `latest` mapping, and never a publish flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubStagingPolicyV1 {
    /// Runner label for the staging job (finite, safe).
    pub runner: String,
    /// Exact repository owner.
    pub owner: String,
    /// Exact repository name.
    pub repository: String,
    /// Explicit tag source mapping.
    pub tag_source: StagingTagSource,
    /// Optional GitHub Environment that gates draft staging with configured
    /// repository reviewers. This is a reference only; Eggpack does not
    /// create or configure the environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    /// Repository-relative staging input paths.
    pub inputs: GitHubStagingInputsV1,
    /// Staging receipt artifact retention in days, from 1 to 90.
    pub receipt_retention_days: u8,
}

/// Explicit tag source mapping for the generated staging job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingTagSource {
    /// Tag-push workflows: candidate tag from `github.ref_name` with ref type tag.
    RefName,
    /// Manual dispatch: explicit existing tag input (no latest/branch default).
    DispatchInput,
}

impl GitHubReleaseInputsV1 {
    /// Validate all required paths plus any configured M003d extension
    /// paths as bounded relative paths without escapes.
    pub fn validate(&self) -> Result<(), CiError> {
        for path in [
            &self.contract,
            &self.release_plan,
            &self.build_bindings,
            &self.qualification_bindings,
            &self.ci_plan,
        ] {
            validate_release_input_path(path)?;
        }
        for path in [
            self.pack_config.as_ref(),
            self.draft_template.as_ref(),
            self.installer_presentation.as_ref(),
            self.consumer_validators.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_release_input_path(path)?;
        }
        Ok(())
    }
}

pub(crate) fn validate_release_input_path(path: &str) -> Result<(), CiError> {
    if path.is_empty() || path.len() > 512 || path.contains('\0') || path.contains('\\') {
        return Err(fail("release input path is not a bounded relative path"));
    }
    if path.starts_with('/')
        || path.contains(':')
        || path.split('/').any(|segment| {
            segment.is_empty() || segment == "." || segment == ".." || segment.len() > 128
        })
    {
        return Err(fail("release input path escapes its root"));
    }
    // No drive prefixes (`C:` covered by colon check) or absolute roots.
    Ok(())
}

/// Canonical per-target build handoff artifact layout (M002a section 5B).
pub const BUILD_HANDOFF_FILE: &str = "build-handoff.json";

/// Canonical per-target qualification evidence file (M002a section 5C).
pub const QUALIFICATION_EVIDENCE_FILE: &str = "evidence.json";

/// Canonical candidate byte directory inside build/qualification artifacts.
pub const CANDIDATES_DIR: &str = "candidates";

/// Canonical gate/aggregate outcome file name.
pub const GATE_OUTCOME_FILE: &str = "gate-outcome.json";

/// Validate that a target directory name is a safe canonical target triple
/// (no path separators or escapes; used for `eggpack-inputs/<target>/`).
pub fn validate_canonical_target_dir(target: &str) -> Result<(), CiError> {
    if target.is_empty()
        || target.len() > 128
        || target.contains('/')
        || target.contains('\\')
        || target.contains(':')
        || target.contains('\0')
        || target.contains("..")
    {
        return Err(fail("target directory escapes its root"));
    }
    Ok(())
}

impl GitHubPolicy {
    /// Validate action identity, immutable SHA syntax, runner uniqueness, and bounds.
    pub fn validate(&self, plan: &CIPlan) -> Result<(), CiError> {
        plan.validate()?;
        if !safe_runner_label(&self.preflight_runner)
            || self.timeout_minutes == 0
            || self.timeout_minutes > 360
            || self.artifact_retention_days == 0
            || self.artifact_retention_days > 90
            || self.triggers.is_empty()
            || self.triggers.len() > 2
            || self.runners.len() > 128
        {
            return Err(fail("invalid or out-of-bounds GitHub policy"));
        }
        validate_pin(&self.checkout, "actions/checkout")?;
        validate_pin(&self.rust_toolchain, "dtolnay/rust-toolchain")?;
        validate_pin(&self.upload_artifact, "actions/upload-artifact")?;
        if let Some(download) = &self.download_artifact {
            validate_pin(download, "actions/download-artifact")?;
        }
        if let Some(tool) = &self.eggpack_tool {
            tool.validate()?;
        }
        if let Some(inputs) = &self.release_inputs {
            inputs.validate()?;
        }
        if let Some(sysroots) = &self.emulated_sysroots {
            if sysroots.len() > 256 {
                return Err(fail("emulated sysroot policy exceeds bound"));
            }
            for (target, path) in sysroots {
                validate_canonical_target_dir(target)?;
                validate_release_input_path(path)?;
            }
        }
        if let Some(staging) = &self.staging {
            if !safe_runner_label(&staging.runner)
                || staging.owner.is_empty()
                || staging.owner.len() > 64
                || staging.repository.is_empty()
                || staging.repository.len() > 64
                || staging
                    .environment
                    .as_deref()
                    .is_some_and(|name| !safe_identifier(name))
                || staging.receipt_retention_days == 0
                || staging.receipt_retention_days > 90
            {
                return Err(fail("invalid GitHub staging policy"));
            }
            // No publish flag exists by construction; tag source is explicit by
            // enum; owner/repository must be safe segments (no control/path).
            for value in [&staging.owner, &staging.repository] {
                if value.chars().any(char::is_control)
                    || value.contains('/')
                    || value.contains("..")
                {
                    return Err(fail("invalid GitHub staging repository identity"));
                }
            }
            for path in [
                &staging.inputs.contract,
                &staging.inputs.install_policy,
                &staging.inputs.github_policy,
            ] {
                validate_release_input_path(path)?;
            }
            if let Some(presentation) = &staging.inputs.installer_presentation {
                validate_release_input_path(presentation)?;
            }
            let required_trigger = match staging.tag_source {
                StagingTagSource::RefName => WorkflowTrigger::Push,
                StagingTagSource::DispatchInput => WorkflowTrigger::WorkflowDispatch,
            };
            if !self.triggers.contains(&required_trigger) {
                return Err(fail(
                    "staging tag source requires its matching workflow trigger",
                ));
            }
        }
        let mut hosts = BTreeSet::new();
        for mapping in &self.runners {
            if !safe_runner_label(&mapping.label)
                || !hosts.insert((os_code(mapping.os), arch_code(mapping.arch)))
            {
                return Err(fail("invalid or duplicate GitHub runner mapping"));
            }
        }
        let mut triggers = BTreeSet::new();
        for trigger in &self.triggers {
            let code = match trigger {
                WorkflowTrigger::Push => 0,
                WorkflowTrigger::WorkflowDispatch => 1,
            };
            if !triggers.insert(code) {
                return Err(fail("duplicate workflow trigger"));
            }
        }
        if let Some(provisioning) = &self.cross_tools {
            provisioning.validate()?;
        }
        for job in &plan.targets {
            let host = &job.planned.policy;
            let mapping = self
                .runners
                .iter()
                .find(|mapping| mapping.os == host.host_os && mapping.arch == host.host_arch)
                .ok_or_else(|| fail("GitHub runner mapping missing for build host"))?;
            if job.planned.policy.strategy != BuildStrategy::CargoZigbuild {
                continue;
            }
            if self.cross_tools.is_some() {
                // Provisioned mode: tools are installed into invocation-private
                // directories; runner labels describe host capability only.
                // Only the finite Linux x86-64/AArch64 provider hosts are
                // supported for official Zig archives.
                if host.host_os != HostOs::Linux
                    || !matches!(host.host_arch, HostArch::X86_64 | HostArch::Aarch64)
                {
                    return Err(fail(
                        "cross-tool provisioning supports only Linux x86-64/AArch64 hosts",
                    ));
                }
            } else if !mapping.cargo_zigbuild || !mapping.zig {
                return Err(fail(
                    "cross-build runner lacks preinstalled cargo-zigbuild or Zig",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_pin(pin: &ActionPin, expected_repo: &str) -> Result<(), CiError> {
    let Some((repo, sha)) = pin.reference.split_once('@') else {
        return Err(fail(
            "GitHub action pin must include an immutable commit SHA",
        ));
    };
    if repo != expected_repo || sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(fail(
            "GitHub action pin is not an approved repository commit SHA",
        ));
    }
    Ok(())
}

fn safe_runner_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 64
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn os_code(os: HostOs) -> u8 {
    match os {
        HostOs::Linux => 1,
        HostOs::Macos => 2,
        HostOs::Windows => 3,
    }
}

fn arch_code(arch: HostArch) -> u8 {
    match arch {
        HostArch::X86_64 => 1,
        HostArch::Aarch64 => 2,
        HostArch::Armv7 => 3,
    }
}
