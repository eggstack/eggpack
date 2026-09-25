#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Deterministic first-install script generation for direct, bundle, and archive releases.
use eggpack_contract::{DistributionContract, ExpandedAssets};
use eggpack_manifest::{ArtifactForm, ReleaseManifest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Caller-owned exact release origin and fixture HTTP switch.
#[derive(Debug, Clone)]
pub struct BootstrapSpec {
    /// Fixed base URL without trailing slash.
    pub origin: String,
    /// Permit loopback HTTP fixtures only.
    pub fixture_http: bool,
}
/// Renderer failure with bounded, path-free diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapError(String);
impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for BootstrapError {}
fn fail(s: &str) -> BootstrapError {
    BootstrapError(s.into())
}
struct Item {
    os: String,
    arch: String,
    name: String,
    install: String,
    size: u64,
    sha: String,
}
fn project(
    c: &DistributionContract,
    m: &ReleaseManifest,
    s: &BootstrapSpec,
) -> Result<Vec<Item>, BootstrapError> {
    let loopback = s.origin.starts_with("http://127.0.0.1")
        || s.origin.starts_with("http://localhost")
        || s.origin.starts_with("http://[::1]");
    if !(s.origin.starts_with("https://") || (s.fixture_http && loopback))
        || s.origin
            .contains(['\n', '\r', '\'', '"', '`', '$', '\\', '?', '#', ' ', '@'])
        || s.origin.ends_with('/')
        || s.origin.ends_with("https://")
    {
        return Err(fail(
            "origin must be a fixed HTTPS URL (or explicit loopback fixture URL)",
        ));
    }
    m.validate().map_err(|_| fail("invalid release manifest"))?;
    if c.product.id != m.product_id {
        return Err(fail("contract and manifest product mismatch"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in &m.targets {
        let x = c
            .resolve(&t.target)
            .map_err(|_| fail("manifest target absent from contract"))?;
        if x.triple != t.target || !seen.insert(t.target.as_str()) {
            return Err(fail("manifest target is not unique and canonical"));
        }
        let exp = c
            .expand(&t.target, &m.release_id)
            .map_err(|_| fail("contract expansion failed"))?;
        let (name, install, size, sha) = match (&exp.assets, &t.form) {
            (ExpandedAssets::Direct(d), ArtifactForm::Direct { artifact, install })
                if d.asset_file == artifact.name && d.install_name == *install =>
            {
                (
                    artifact.name.clone(),
                    install.clone(),
                    artifact.size,
                    artifact.sha256.clone(),
                )
            }
            _ => return Err(fail("M001 requires matching direct artifact targets")),
        };
        if !safe_segment(&name) || !safe_segment(&install) {
            return Err(fail(
                "artifact and install basenames contain unsupported URL or platform characters",
            ));
        }
        let (os, arch) = runtime_pair(&t.target)
            .ok_or_else(|| fail("target has no unambiguous runtime OS/architecture mapping"))?;
        out.push(Item {
            os: os.into(),
            arch: arch.into(),
            name,
            install,
            size,
            sha,
        });
    }
    if out.len() != c.targets.len() {
        return Err(fail("contract and manifest target sets differ"));
    }
    let mut pairs = std::collections::HashSet::new();
    if out
        .iter()
        .any(|x| !pairs.insert((x.os.clone(), x.arch.clone())))
    {
        return Err(fail("runtime platform mapping is ambiguous"));
    }
    Ok(out)
}
fn safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'+'))
}
fn runtime_pair(t: &str) -> Option<(&'static str, &'static str)> {
    let arch = if t.starts_with("x86_64-") {
        "x86_64"
    } else if t.starts_with("aarch64-") {
        "aarch64"
    } else if t.starts_with("armv7-") {
        "armv7"
    } else {
        return None;
    };
    let os = if t.contains("-linux-") {
        "linux"
    } else if t.ends_with("-apple-darwin") {
        "macos"
    } else if t.contains("-windows-") {
        "windows"
    } else {
        return None;
    };
    Some((os, arch))
}
/// Render deterministic POSIX shell for an exact release.
pub fn render_posix(
    c: &DistributionContract,
    m: &ReleaseManifest,
    s: &BootstrapSpec,
) -> Result<String, BootstrapError> {
    let items = project(c, m, s)?;
    let mut cases = String::new();
    for i in items {
        cases.push_str(&format!(
            "  {}:{}) name={} install={} size={} sha={} ;;\n",
            i.os,
            i.arch,
            shq(&i.name),
            shq(&i.install),
            i.size,
            i.sha
        ));
    }
    Ok(format!(
        r##"#!/bin/sh
set -eu
os=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$os" in darwin) os=macos ;; esac
arch=$(uname -m)
case "$arch" in amd64) arch=x86_64 ;; arm64) arch=aarch64 ;; armv7l) arch=armv7 ;; esac
case "$os:$arch" in
{cases}  *) echo 'unsupported platform' >&2; exit 2 ;;
esac
origin={origin}
dest=${{1:-.}}
mkdir -p "$dest"
file="$dest/$install"
[ ! -e "$file" ] || {{ echo 'destination already exists' >&2; exit 3; }}
tmp=$(mktemp -d "$dest/.eggpack.XXXXXX") || exit 4
trap 'rm -rf "$tmp"' 0
trap 'exit 1' HUP INT TERM
if command -v curl >/dev/null 2>&1; then curl --fail --silent --show-error --connect-timeout 10 --max-time 120 "$origin/$name" -o "$tmp/payload"; else echo 'curl is required' >&2; exit 5; fi
actual=$(wc -c < "$tmp/payload" | tr -d ' ')
[ "$actual" = "$size" ] || {{ echo 'size mismatch' >&2; exit 6; }}
if command -v sha256sum >/dev/null 2>&1; then actual=$(sha256sum "$tmp/payload" | cut -d ' ' -f 1); elif command -v shasum >/dev/null 2>&1; then actual=$(shasum -a 256 "$tmp/payload" | cut -d ' ' -f 1); elif command -v openssl >/dev/null 2>&1; then actual=$(openssl dgst -sha256 "$tmp/payload" | sed 's/^.*= //'); else echo 'SHA-256 tool required' >&2; exit 7; fi
[ "$actual" = "$sha" ] || {{ echo 'SHA-256 mismatch' >&2; exit 8; }}
[ ! -e "$file" ] || {{ echo 'destination already exists' >&2; exit 3; }}
chmod 755 "$tmp/payload"
ln "$tmp/payload" "$file" || {{ echo 'destination already exists or filesystem does not support safe placement' >&2; exit 3; }}
"##,
        cases = cases,
        origin = shq(&s.origin)
    ))
}
fn shq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
/// Render deterministic PowerShell for an exact release.
pub fn render_powershell(
    c: &DistributionContract,
    m: &ReleaseManifest,
    s: &BootstrapSpec,
) -> Result<String, BootstrapError> {
    let items = project(c, m, s)?;
    let mut cases = String::new();
    for i in items {
        cases.push_str(&format!(
            "  '{}:{}' {{ $name={}; $install={}; $size={}; $sha='{}' }}\n",
            i.os,
            i.arch,
            psq(&i.name),
            psq(&i.install),
            i.size,
            i.sha
        ));
    }
    Ok(format!(
        r#"$ErrorActionPreference = 'Stop'
$os = if ($env:OS -eq 'Windows_NT') {{ 'windows' }} elseif ($IsMacOS) {{ 'macos' }} else {{ 'linux' }}
$arch = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
switch ($arch) {{ 'x64' {{ $arch='x86_64' }} 'arm64' {{ $arch='aarch64' }} 'arm' {{ $arch='armv7' }} }}
switch ("$os`:$arch") {{
{cases}  default {{ throw 'unsupported platform' }}
}}
$origin = {origin}
$dest = if ($args.Count -gt 0) {{ $args[0] }} else {{ '.' }}
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$file = Join-Path $dest $install
if (Test-Path -LiteralPath $file) {{ throw 'destination already exists' }}
$tmp = Join-Path $dest ('.eggpack-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {{
  $payload = Join-Path $tmp 'payload'
  Invoke-WebRequest -Uri "$origin/$name" -OutFile $payload -TimeoutSec 120 -MaximumRedirection 0
  if ((Get-Item -LiteralPath $payload).Length -ne $size) {{ throw 'size mismatch' }}
  if ((Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sha) {{ throw 'SHA-256 mismatch' }}
  if (Test-Path -LiteralPath $file) {{ throw 'destination already exists' }}
  [IO.File]::Move($payload, $file)
}} finally {{ Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue }}
"#,
        cases = cases,
        origin = psq(&s.origin)
    ))
}
fn psq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

// ---------------------------------------------------------------------------
// M002 — bundle/archive bootstrap safety.
// ---------------------------------------------------------------------------

/// Finite caller-owned install mode for one bundle/archive entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    /// POSIX mode 0755; Windows preserves bytes with inherited ACL.
    Executable,
    /// POSIX mode 0644; Windows preserves bytes with inherited ACL.
    Data,
}

/// Finite archive encoding accepted by generated bootstrap installers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleArchiveEncoding {
    /// POSIX tar stream compressed with gzip, matching M004 producer convention.
    TarGzip,
}

/// Per-target caller install policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInstallPolicy {
    /// Exact install-name to mode map for bundle/archive targets.
    #[serde(default)]
    pub modes: BTreeMap<String, InstallMode>,
    /// Required TarGzip encoding for archive targets; must be absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_encoding: Option<BundleArchiveEncoding>,
}

/// Strict versioned caller-owned bootstrap install policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapInstallPolicyV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Per-canonical-target install policy. May be empty for direct-only releases.
    #[serde(default)]
    pub targets: BTreeMap<String, TargetInstallPolicy>,
}

const MAX_POLICY_JSON: usize = 256 * 1024;
const MAX_POLICY_TARGETS: usize = 256;
const MAX_POLICY_ENTRIES: usize = 256;

impl BootstrapInstallPolicyV1 {
    /// Empty policy for direct-only first-install releases (M001 behavior).
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            targets: BTreeMap::new(),
        }
    }

    /// Parse strict TOML policy.
    pub fn from_toml(text: &str) -> Result<Self, BootstrapError> {
        if text.len() > MAX_POLICY_JSON {
            return Err(fail("install policy exceeds size bound"));
        }
        let value: Self = toml::from_str(text).map_err(|_| fail("invalid install policy TOML"))?;
        value.validate_shape()?;
        Ok(value)
    }

    /// Parse strict JSON policy.
    pub fn from_json(text: &str) -> Result<Self, BootstrapError> {
        if text.len() > MAX_POLICY_JSON {
            return Err(fail("install policy exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid install policy JSON"))?;
        value.validate_shape()?;
        Ok(value)
    }

    fn validate_shape(&self) -> Result<(), BootstrapError> {
        if self.schema_version != 1 || self.targets.len() > MAX_POLICY_TARGETS {
            return Err(fail("invalid install policy version or target count"));
        }
        for (target, policy) in &self.targets {
            if target.is_empty() || target.len() > 128 || policy.modes.len() > MAX_POLICY_ENTRIES {
                return Err(fail("invalid install policy target or entry count"));
            }
            for name in policy.modes.keys() {
                if !safe_segment(name) {
                    return Err(fail("install policy names contain unsupported characters"));
                }
            }
        }
        Ok(())
    }
}

fn validate_origin(s: &BootstrapSpec) -> Result<(), BootstrapError> {
    let loopback = s.origin.starts_with("http://127.0.0.1")
        || s.origin.starts_with("http://localhost")
        || s.origin.starts_with("http://[::1]");
    if !(s.origin.starts_with("https://") || (s.fixture_http && loopback))
        || s.origin
            .contains(['\n', '\r', '\'', '"', '`', '$', '\\', '?', '#', ' ', '@'])
        || s.origin.ends_with('/')
        || s.origin.ends_with("https://")
    {
        return Err(fail(
            "origin must be a fixed HTTPS URL (or explicit loopback fixture URL)",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct BundleEntryProjection {
    name: String,
    install: String,
    size: u64,
    sha: String,
    mode: InstallMode,
}

#[derive(Debug, Clone)]
struct ArchiveMemberProjection {
    source: String,
    install: String,
    size: u64,
    sha: String,
    mode: InstallMode,
}

#[derive(Debug, Clone)]
enum TargetProjection {
    Direct {
        os: String,
        arch: String,
        name: String,
        install: String,
        size: u64,
        sha: String,
    },
    Bundle {
        os: String,
        arch: String,
        entries: Vec<BundleEntryProjection>,
    },
    Archive {
        os: String,
        arch: String,
        archive_name: String,
        archive_size: u64,
        archive_sha: String,
        members: Vec<ArchiveMemberProjection>,
    },
}

fn project_all(
    c: &DistributionContract,
    m: &ReleaseManifest,
    s: &BootstrapSpec,
    policy: &BootstrapInstallPolicyV1,
) -> Result<Vec<TargetProjection>, BootstrapError> {
    validate_origin(s)?;
    policy.validate_shape()?;
    m.validate().map_err(|_| fail("invalid release manifest"))?;
    if policy.schema_version != 1 {
        return Err(fail("invalid install policy version"));
    }
    if c.product.id != m.product_id {
        return Err(fail("contract and manifest product mismatch"));
    }
    let mut seen_targets = BTreeSet::new();
    let mut seen_platforms = BTreeSet::new();
    let mut out = Vec::new();
    for t in &m.targets {
        let resolved = c
            .resolve(&t.target)
            .map_err(|_| fail("manifest target absent from contract"))?;
        if resolved.triple != t.target || !seen_targets.insert(t.target.as_str()) {
            return Err(fail("manifest target is not unique and canonical"));
        }
        let expanded = c
            .expand(&t.target, &m.release_id)
            .map_err(|_| fail("contract expansion failed"))?;
        let target_policy = policy.targets.get(&t.target);
        let (os, arch) = runtime_pair(&t.target)
            .ok_or_else(|| fail("target has no unambiguous runtime OS/architecture mapping"))?;
        if !seen_platforms.insert((os.to_string(), arch.to_string())) {
            return Err(fail("runtime platform mapping is ambiguous"));
        }
        match (&expanded.assets, &t.form) {
            (
                eggpack_contract::ExpandedAssets::Direct(d),
                ArtifactForm::Direct { artifact, install },
            ) if d.asset_file == artifact.name && d.install_name == *install => {
                if !safe_segment(&artifact.name) || !safe_segment(install) {
                    return Err(fail(
                        "artifact and install basenames contain unsupported URL or platform characters",
                    ));
                }
                if let Some(p) = target_policy {
                    if p.archive_encoding.is_some() {
                        return Err(fail("archive policy on non-archive target"));
                    }
                    if p.modes.len() != 1 || p.modes.get(install) != Some(&InstallMode::Executable)
                    {
                        return Err(fail(
                            "direct install policy must name the single executable install",
                        ));
                    }
                }
                out.push(TargetProjection::Direct {
                    os: os.into(),
                    arch: arch.into(),
                    name: artifact.name.clone(),
                    install: install.clone(),
                    size: artifact.size,
                    sha: artifact.sha256.clone(),
                });
            }
            (eggpack_contract::ExpandedAssets::Bundle(b), ArtifactForm::Bundle { entries }) => {
                let p =
                    target_policy.ok_or_else(|| fail("bundle target is missing install policy"))?;
                if p.archive_encoding.is_some() {
                    return Err(fail("archive policy on non-archive target"));
                }
                if b.entries.len() != entries.len() {
                    return Err(fail("bundle contract and manifest entry counts differ"));
                }
                // Pair contract entries with manifest entries by exact asset/install names.
                let mut manifest_by_asset: BTreeMap<(&str, &str), _> = BTreeMap::new();
                for e in entries {
                    if !safe_segment(&e.artifact.name) || !safe_segment(&e.install) {
                        return Err(fail(
                            "artifact and install basenames contain unsupported URL or platform characters",
                        ));
                    }
                    if manifest_by_asset
                        .insert((e.artifact.name.as_str(), e.install.as_str()), e)
                        .is_some()
                    {
                        return Err(fail("duplicate bundle manifest entry"));
                    }
                }
                let mut projected = Vec::with_capacity(b.entries.len());
                for contract_entry in &b.entries {
                    let key = (
                        contract_entry.asset_file.as_str(),
                        contract_entry.install_name.as_str(),
                    );
                    let manifest_entry = manifest_by_asset
                        .remove(&key)
                        .ok_or_else(|| fail("bundle contract and manifest entries differ"))?;
                    let mode = p.modes.get(&contract_entry.install_name).ok_or_else(|| {
                        fail("bundle install policy is missing or has extra install names")
                    })?;
                    projected.push(BundleEntryProjection {
                        name: manifest_entry.artifact.name.clone(),
                        install: manifest_entry.install.clone(),
                        size: manifest_entry.artifact.size,
                        sha: manifest_entry.artifact.sha256.clone(),
                        mode: *mode,
                    });
                }
                if !manifest_by_asset.is_empty() {
                    return Err(fail("bundle contract and manifest entries differ"));
                }
                if p.modes.len() != projected.len() {
                    return Err(fail(
                        "bundle install policy is missing or has extra install names",
                    ));
                }
                out.push(TargetProjection::Bundle {
                    os: os.into(),
                    arch: arch.into(),
                    entries: projected,
                });
            }
            (
                eggpack_contract::ExpandedAssets::Archive(a),
                ArtifactForm::Archive { artifact, members },
            ) => {
                let p = target_policy
                    .ok_or_else(|| fail("archive target is missing install policy"))?;
                if p.archive_encoding != Some(BundleArchiveEncoding::TarGzip) {
                    return Err(fail(
                        "archive target requires explicit TarGzip install policy",
                    ));
                }
                if !a.archive_file.ends_with(".tar.gz") {
                    return Err(fail("TarGzip archive filename must end in .tar.gz"));
                }
                if a.archive_file != artifact.name {
                    return Err(fail("archive contract and manifest filenames differ"));
                }
                if !safe_segment(&artifact.name) {
                    return Err(fail(
                        "artifact and install basenames contain unsupported URL or platform characters",
                    ));
                }
                if a.members.len() != members.len() {
                    return Err(fail("archive contract and manifest member counts differ"));
                }
                let mut manifest_by_source: BTreeMap<&str, _> = BTreeMap::new();
                for member in members {
                    // Do not accept a manifest member merely because its basename matches.
                    if manifest_by_source
                        .insert(member.source.as_str(), member)
                        .is_some()
                    {
                        return Err(fail("duplicate archive manifest member"));
                    }
                }
                let mut projected = Vec::with_capacity(a.members.len());
                for contract_member in &a.members {
                    let manifest_member = manifest_by_source
                        .remove(contract_member.source.as_str())
                        .ok_or_else(|| {
                            fail("archive contract and manifest member sources differ")
                        })?;
                    if manifest_member.install != contract_member.install_name {
                        return Err(fail("archive contract and manifest install names differ"));
                    }
                    if !safe_segment(&manifest_member.install) {
                        return Err(fail(
                            "artifact and install basenames contain unsupported URL or platform characters",
                        ));
                    }
                    let mode = p.modes.get(&contract_member.install_name).ok_or_else(|| {
                        fail("archive install policy is missing or has extra install names")
                    })?;
                    projected.push(ArchiveMemberProjection {
                        source: manifest_member.source.clone(),
                        install: manifest_member.install.clone(),
                        size: manifest_member.bytes.size,
                        sha: manifest_member.bytes.sha256.clone(),
                        mode: *mode,
                    });
                }
                if !manifest_by_source.is_empty() {
                    return Err(fail("archive contract and manifest member sources differ"));
                }
                if p.modes.len() != projected.len() {
                    return Err(fail(
                        "archive install policy is missing or has extra install names",
                    ));
                }
                out.push(TargetProjection::Archive {
                    os: os.into(),
                    arch: arch.into(),
                    archive_name: artifact.name.clone(),
                    archive_size: artifact.size,
                    archive_sha: artifact.sha256.clone(),
                    members: projected,
                });
            }
            _ => {
                return Err(fail("contract and manifest forms differ for target"));
            }
        }
    }
    if out.len() != c.targets.len() {
        return Err(fail("contract and manifest target sets differ"));
    }
    for extra in policy.targets.keys() {
        if !seen_targets.contains(extra.as_str()) {
            return Err(fail("install policy names an unknown target"));
        }
    }
    out.sort_by(|a, b| {
        let key = |p: &TargetProjection| match p {
            TargetProjection::Direct { os, arch, .. }
            | TargetProjection::Bundle { os, arch, .. }
            | TargetProjection::Archive { os, arch, .. } => (os.clone(), arch.clone()),
        };
        key(a).cmp(&key(b))
    });
    Ok(out)
}

fn posix_mode(mode: InstallMode) -> &'static str {
    match mode {
        InstallMode::Executable => "755",
        InstallMode::Data => "644",
    }
}

const POSIX_SHA_SNIPPET: &str = "if command -v sha256sum >/dev/null 2>&1; then actual=$(sha256sum \"$file_arg\" | cut -d ' ' -f 1); elif command -v shasum >/dev/null 2>&1; then actual=$(shasum -a 256 \"$file_arg\" | cut -d ' ' -f 1); elif command -v openssl >/dev/null 2>&1; then actual=$(openssl dgst -sha256 \"$file_arg\" | sed 's/^.*= //'); else echo 'SHA-256 tool required' >&2; exit 7; fi";

fn posix_verify_block(tmp_file: &str, size: u64, sha: &str) -> String {
    let size_check = format!(
        "actual=$(wc -c < {tmp} | tr -d ' ')\n    [ \"$actual\" = \"{size}\" ] || {{ echo 'size mismatch' >&2; exit 6; }}\n",
        tmp = tmp_file,
        size = size
    );
    let sha_check = POSIX_SHA_SNIPPET.replace("$file_arg", tmp_file)
        + &format!(
            "\n    [ \"$actual\" = {sha} ] || {{ echo 'SHA-256 mismatch' >&2; exit 8; }}\n",
            sha = shq(sha)
        );
    size_check + &sha_check
}

fn posix_download_block(origin: &str, name: &str, tmp_file: &str) -> String {
    format!(
        "    if command -v curl >/dev/null 2>&1; then curl --fail --silent --show-error --connect-timeout 10 --max-time 120 {origin}/{name} -o {tmp}; else echo 'curl is required' >&2; exit 5; fi\n",
        origin = shq(origin),
        name = shq(name),
        tmp = tmp_file
    )
}

/// Render deterministic POSIX shell for direct, bundle, and archive releases.
pub fn render_posix_with_policy(
    c: &DistributionContract,
    m: &ReleaseManifest,
    s: &BootstrapSpec,
    policy: &BootstrapInstallPolicyV1,
) -> Result<String, BootstrapError> {
    let targets = project_all(c, m, s, policy)?;
    let mut branches = String::new();
    for target in &targets {
        match target {
            TargetProjection::Direct {
                os,
                arch,
                name,
                install,
                size,
                sha,
            } => {
                let tmp_file = "\"$tmp/payload\"";
                branches.push_str(&format!(
                    "  {os}:{arch})\n    file=\"$dest\"/{install}\n    [ ! -e \"$file\" ] || {{ echo 'destination already exists' >&2; exit 3; }}\n{download}{verify}    [ ! -e \"$file\" ] || {{ echo 'destination already exists' >&2; exit 3; }}\n    chmod 755 {payload}\n    ln {payload} \"$file\" || {{ echo 'destination already exists or filesystem does not support safe placement' >&2; exit 3; }}\n    exit 0\n    ;;\n",
                    os = os,
                    arch = arch,
                    install = shq(install),
                    download = posix_download_block(&s.origin, name, tmp_file),
                    verify = posix_verify_block(tmp_file, *size, sha),
                    payload = tmp_file,
                ));
            }
            TargetProjection::Bundle { os, arch, entries } => {
                let mut block = format!("  {os}:{arch})\n");
                for (index, entry) in entries.iter().enumerate() {
                    block.push_str(&format!(
                        "    file{index}=\"$dest\"/{install}\n",
                        index = index,
                        install = shq(&entry.install)
                    ));
                }
                for index in 0..entries.len() {
                    block.push_str(&format!(
                        "    [ ! -e \"$file{index}\" ] || {{ echo 'destination already exists' >&2; exit 3; }}\n",
                        index = index
                    ));
                }
                for (index, entry) in entries.iter().enumerate() {
                    let tmp_file = format!("\"$tmp/bundle-{index}\"", index = index);
                    block.push_str(&posix_download_block(&s.origin, &entry.name, &tmp_file));
                    block.push_str(&format!(
                        "    {}",
                        posix_verify_block(&tmp_file, entry.size, &entry.sha)
                            .replace('\n', "\n    ")
                            .trim_end()
                    ));
                    block.push('\n');
                }
                for index in 0..entries.len() {
                    block.push_str(&format!(
                        "    [ ! -e \"$file{index}\" ] || {{ echo 'destination already exists' >&2; exit 3; }}\n",
                        index = index
                    ));
                }
                for (index, entry) in entries.iter().enumerate() {
                    block.push_str(&format!(
                        "    chmod {mode} \"$tmp/bundle-{index}\"\n",
                        mode = posix_mode(entry.mode),
                        index = index
                    ));
                }
                for (index, _) in entries.iter().enumerate() {
                    if index == 0 {
                        block.push_str(&format!(
                            "    ln \"$tmp/bundle-{index}\" \"$file{index}\" || {{ echo 'destination already exists or filesystem does not support safe placement' >&2; exit 3; }}\n",
                            index = index
                        ));
                    } else {
                        let mut rollback = String::new();
                        for prior in 0..index {
                            rollback.push_str(&format!(" \"$file{prior}\"", prior = prior));
                        }
                        block.push_str(&format!(
                            "    ln \"$tmp/bundle-{index}\" \"$file{index}\" || {{ rm -f{rollback}; echo 'destination already exists or filesystem does not support safe placement' >&2; exit 3; }}\n",
                            index = index,
                            rollback = rollback
                        ));
                    }
                }
                block.push_str("    exit 0\n    ;;\n");
                branches.push_str(&block);
            }
            TargetProjection::Archive {
                os,
                arch,
                archive_name,
                archive_size,
                archive_sha,
                members,
            } => {
                let mut block = format!("  {os}:{arch})\n");
                for (index, member) in members.iter().enumerate() {
                    block.push_str(&format!(
                        "    dest{index}=\"$dest\"/{install}\n",
                        index = index,
                        install = shq(&member.install)
                    ));
                }
                for index in 0..members.len() {
                    block.push_str(&format!(
                        "    [ ! -e \"$dest{index}\" ] || {{ echo 'destination already exists' >&2; exit 3; }}\n",
                        index = index
                    ));
                }
                block.push_str("    if ! command -v tar >/dev/null 2>&1; then echo 'tar is required' >&2; exit 5; fi\n");
                let archive_tmp = "\"$tmp/archive\"";
                block.push_str(&posix_download_block(&s.origin, archive_name, archive_tmp));
                block.push_str(&format!(
                    "    {}",
                    posix_verify_block(archive_tmp, *archive_size, archive_sha)
                        .replace('\n', "\n    ")
                        .trim_end()
                ));
                block.push('\n');
                let mut expected_printf = String::from("    printf '%s\\n'");
                for member in members {
                    expected_printf.push_str(&format!(" {}", shq(&member.source)));
                }
                expected_printf.push_str(" > \"$tmp/expected\"\n");
                block.push_str(&expected_printf);
                block.push_str("    if ! tar -tzf \"$tmp/archive\" > \"$tmp/listing\"; then echo 'archive listing failed' >&2; exit 6; fi\n");
                block.push_str("    while IFS= read -r line; do case \"$line\" in '') echo 'unsafe archive member' >&2; exit 6 ;; /*) echo 'unsafe archive member' >&2; exit 6 ;; *\\\\*) echo 'unsafe archive member' >&2; exit 6 ;; *:* ) echo 'unsafe archive member' >&2; exit 6 ;; esac; case \"$line\" in *'..'*) echo 'unsafe archive member' >&2; exit 6 ;; esac; done < \"$tmp/listing\"\n");
                block.push_str("    if ! LC_ALL=C sort \"$tmp/expected\" > \"$tmp/expected.sorted\"; then echo 'archive inventory failed' >&2; exit 6; fi\n");
                block.push_str("    if ! LC_ALL=C sort \"$tmp/listing\" > \"$tmp/listing.sorted\"; then echo 'archive inventory failed' >&2; exit 6; fi\n");
                block.push_str("    if ! cmp -s \"$tmp/expected.sorted\" \"$tmp/listing.sorted\"; then echo 'archive member inventory mismatch' >&2; exit 6; fi\n");
                block.push_str("    mkdir -p \"$tmp/extracted\"\n");
                block.push_str("    if ! tar -xzf \"$tmp/archive\" -C \"$tmp/extracted\"; then echo 'archive extraction failed' >&2; exit 6; fi\n");
                for member in members {
                    block.push_str(&format!(
                        "    member=\"$tmp/extracted\"/{source}\n    [ ! -L \"$member\" ] || {{ echo 'symlink member rejected' >&2; exit 6; }}\n    [ -f \"$member\" ] || {{ echo 'member is not a regular file' >&2; exit 6; }}\n",
                        source = shq(&member.source)
                    ));
                    let member_var = "\"$member\"";
                    block.push_str(&format!(
                        "    {}",
                        posix_verify_block(member_var, member.size, &member.sha)
                            .replace('\n', "\n    ")
                            .trim_end()
                    ));
                    block.push('\n');
                    block.push_str(&format!(
                        "    chmod {mode} \"$member\"\n",
                        mode = posix_mode(member.mode)
                    ));
                }
                for index in 0..members.len() {
                    block.push_str(&format!(
                        "    [ ! -e \"$dest{index}\" ] || {{ echo 'destination already exists' >&2; exit 3; }}\n",
                        index = index
                    ));
                }
                for (index, member) in members.iter().enumerate() {
                    if index == 0 {
                        block.push_str(&format!(
                            "    ln \"$tmp/extracted\"/{source} \"$dest{index}\" || {{ echo 'destination already exists or filesystem does not support safe placement' >&2; exit 3; }}\n",
                            source = shq(&member.source),
                            index = index
                        ));
                    } else {
                        let mut rollback = String::new();
                        for prior in 0..index {
                            rollback.push_str(&format!(" \"$dest{prior}\"", prior = prior));
                        }
                        block.push_str(&format!(
                            "    ln \"$tmp/extracted\"/{source} \"$dest{index}\" || {{ rm -f{rollback}; echo 'destination already exists or filesystem does not support safe placement' >&2; exit 3; }}\n",
                            source = shq(&member.source),
                            index = index,
                            rollback = rollback
                        ));
                    }
                }
                block.push_str("    exit 0\n    ;;\n");
                branches.push_str(&block);
            }
        }
    }
    Ok(format!(
        r##"#!/bin/sh
set -eu
os=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$os" in darwin) os=macos ;; esac
arch=$(uname -m)
case "$arch" in amd64) arch=x86_64 ;; arm64) arch=aarch64 ;; armv7l) arch=armv7 ;; esac
origin={origin}
dest=${{1:-.}}
mkdir -p "$dest"
tmp=$(mktemp -d "$dest/.eggpack.XXXXXX") || exit 4
trap 'rm -rf "$tmp"' 0
trap 'exit 1' HUP INT TERM
case "$os:$arch" in
{branches}  *) echo 'unsupported platform' >&2; exit 2 ;;
esac
"##,
        origin = shq(&s.origin),
        branches = branches
    ))
}

fn ps_verify_block(payload_var: &str, size: u64, sha: &str) -> String {
    format!(
        "  if ((Get-Item -LiteralPath {payload}).Length -ne {size}) {{ throw 'size mismatch' }}\n  if ((Get-FileHash -LiteralPath {payload} -Algorithm SHA256).Hash.ToLowerInvariant() -ne {sha}) {{ throw 'SHA-256 mismatch' }}\n",
        payload = payload_var,
        size = size,
        sha = shq(&sha.to_lowercase())
    )
}

/// Render deterministic PowerShell for direct, bundle, and archive releases.
pub fn render_powershell_with_policy(
    c: &DistributionContract,
    m: &ReleaseManifest,
    s: &BootstrapSpec,
    policy: &BootstrapInstallPolicyV1,
) -> Result<String, BootstrapError> {
    let targets = project_all(c, m, s, policy)?;
    let mut branches = String::new();
    for target in &targets {
        match target {
            TargetProjection::Direct {
                os,
                arch,
                name,
                install,
                size,
                sha,
            } => {
                branches.push_str(&format!(
                    "    '{os}:{arch}' {{\n      $file = Join-Path $dest {install}\n      if (Test-Path -LiteralPath $file) {{ throw 'destination already exists' }}\n      $payload = Join-Path $tmp 'payload'\n      Invoke-WebRequest -Uri \"$origin/{name}\" -OutFile $payload -TimeoutSec 120 -MaximumRedirection 0\n{verify}      if (Test-Path -LiteralPath $file) {{ throw 'destination already exists' }}\n      [IO.File]::Move($payload, $file)\n      $created += $file\n    }}\n",
                    os = os,
                    arch = arch,
                    install = psq(install),
                    name = name,
                    verify = ps_verify_block("$payload", *size, sha)
                        .lines()
                        .map(|l| format!("      {l}\n"))
                        .collect::<String>(),
                ));
            }
            TargetProjection::Bundle { os, arch, entries } => {
                let mut block = format!("    '{os}:{arch}' {{\n");
                for (index, entry) in entries.iter().enumerate() {
                    block.push_str(&format!(
                        "      $file{index} = Join-Path $dest {install}\n",
                        index = index,
                        install = psq(&entry.install)
                    ));
                }
                for index in 0..entries.len() {
                    block.push_str(&format!(
                        "      if (Test-Path -LiteralPath $file{index}) {{ throw 'destination already exists' }}\n",
                        index = index
                    ));
                }
                for (index, entry) in entries.iter().enumerate() {
                    block.push_str(&format!(
                        "      $payload{index} = Join-Path $tmp 'bundle-{index}'\n      Invoke-WebRequest -Uri \"$origin/{name}\" -OutFile $payload{index} -TimeoutSec 120 -MaximumRedirection 0\n",
                        index = index,
                        name = entry.name
                    ));
                    block.push_str(
                        &ps_verify_block(
                            &format!("$payload{index}", index = index),
                            entry.size,
                            &entry.sha,
                        )
                        .lines()
                        .map(|l| format!("      {l}\n"))
                        .collect::<String>(),
                    );
                }
                for index in 0..entries.len() {
                    block.push_str(&format!(
                        "      if (Test-Path -LiteralPath $file{index}) {{ throw 'destination already exists' }}\n",
                        index = index
                    ));
                }
                for (index, _) in entries.iter().enumerate() {
                    block.push_str(&format!(
                        "      [IO.File]::Move($payload{index}, $file{index})\n      $created += $file{index}\n",
                        index = index
                    ));
                }
                block.push_str("    }\n");
                branches.push_str(&block);
            }
            TargetProjection::Archive {
                os,
                arch,
                archive_name,
                archive_size,
                archive_sha,
                members,
            } => {
                let mut block = format!("    '{os}:{arch}' {{\n");
                for (index, member) in members.iter().enumerate() {
                    block.push_str(&format!(
                        "      $dest{index} = Join-Path $dest {install}\n",
                        index = index,
                        install = psq(&member.install)
                    ));
                }
                for index in 0..members.len() {
                    block.push_str(&format!(
                        "      if (Test-Path -LiteralPath $dest{index}) {{ throw 'destination already exists' }}\n",
                        index = index
                    ));
                }
                block.push_str("      if (-not (Get-Command tar.exe -ErrorAction SilentlyContinue)) { throw 'tar.exe is required' }\n");
                block.push_str(&format!(
                    "      $archive = Join-Path $tmp 'archive'\n      Invoke-WebRequest -Uri \"$origin/{name}\" -OutFile $archive -TimeoutSec 120 -MaximumRedirection 0\n",
                    name = archive_name
                ));
                block.push_str(
                    &ps_verify_block("$archive", *archive_size, archive_sha)
                        .lines()
                        .map(|l| format!("      {l}\n"))
                        .collect::<String>(),
                );
                let expected_list = members
                    .iter()
                    .map(|member| psq(&member.source))
                    .collect::<Vec<_>>()
                    .join(", ");
                block.push_str(&format!(
                    "      $expected = @({expected})\n      $actual = & tar.exe -tzf $archive\n      if ($LASTEXITCODE -ne 0) {{ throw 'archive listing failed' }}\n",
                    expected = expected_list
                ));
                block.push_str("      foreach ($line in $actual) { if ([string]::IsNullOrEmpty($line)) { throw 'unsafe archive member' }; if ($line.StartsWith('/')) { throw 'unsafe archive member' }; if ($line.Contains('\\')) { throw 'unsafe archive member' }; if ($line.Contains(':')) { throw 'unsafe archive member' }; if ($line -match '\\.\\.') { throw 'unsafe archive member' } }\n");
                block.push_str("      $missing = @($expected | Where-Object { $actual -notcontains $_ }); if ($missing.Count -gt 0) { throw 'archive member inventory mismatch' }\n");
                block.push_str("      $extra = @($actual | Where-Object { $expected -notcontains $_ }); if ($extra.Count -gt 0) { throw 'archive member inventory mismatch' }\n");
                block.push_str("      $extractDir = Join-Path $tmp 'extracted'\n      New-Item -ItemType Directory -Path $extractDir | Out-Null\n      & tar.exe -xzf $archive -C $extractDir\n      if ($LASTEXITCODE -ne 0) { throw 'archive extraction failed' }\n");
                for member in members {
                    block.push_str(&format!(
                        "      $member = Join-Path $extractDir {source}\n      if ((Get-Item -LiteralPath $member).LinkType) {{ throw 'symlink member rejected' }}\n      if (-not (Test-Path -LiteralPath $member -PathType Leaf)) {{ throw 'member is not a regular file' }}\n",
                        source = psq(&member.source)
                    ));
                    block.push_str(
                        &ps_verify_block("$member", member.size, &member.sha)
                            .lines()
                            .map(|l| format!("      {l}\n"))
                            .collect::<String>(),
                    );
                }
                for index in 0..members.len() {
                    block.push_str(&format!(
                        "      if (Test-Path -LiteralPath $dest{index}) {{ throw 'destination already exists' }}\n",
                        index = index
                    ));
                }
                for (index, member) in members.iter().enumerate() {
                    block.push_str(&format!(
                        "      $member{index} = Join-Path $extractDir {source}\n      [IO.File]::Move($member{index}, $dest{index})\n      $created += $dest{index}\n",
                        index = index,
                        source = psq(&member.source)
                    ));
                }
                block.push_str("    }\n");
                branches.push_str(&block);
            }
        }
    }
    Ok(format!(
        r#"$ErrorActionPreference = 'Stop'
$os = if ($env:OS -eq 'Windows_NT') {{ 'windows' }} elseif ($IsMacOS) {{ 'macos' }} else {{ 'linux' }}
$arch = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
switch ($arch) {{ 'x64' {{ $arch='x86_64' }} 'arm64' {{ $arch='aarch64' }} 'arm' {{ $arch='armv7' }} }}
$origin = {origin}
$dest = if ($args.Count -gt 0) {{ $args[0] }} else {{ '.' }}
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$tmp = Join-Path $dest ('.eggpack-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
$created = @()
try {{
  switch ("$os`:$arch") {{
{branches}    default {{ throw 'unsupported platform' }}
  }}
}} catch {{
  foreach ($p in $created) {{ Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue }}
  throw
}} finally {{ Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue }}
"#,
        origin = psq(&s.origin),
        branches = branches
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    fn serve_once(body: &'static [u8], status: &'static str) -> String {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
        });
        format!("http://{address}/releases")
    }
    #[test]
    fn exact_direct_maps_render_stable_scripts_and_reject_bad_origin() {
        use std::process::Command;
        let c = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let m = ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: "a".repeat(40),
            targets: vec![
                eggpack_manifest::TargetRecord {
                    target: "x86_64-unknown-linux-gnu".into(),
                    form: ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-x86_64-unknown-linux-gnu".into(),
                            size: 3,
                            sha256:
                                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                                    .into(),
                        },
                        install: "eggsact".into(),
                    },
                },
                eggpack_manifest::TargetRecord {
                    target: "aarch64-apple-darwin".into(),
                    form: ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-aarch64-apple-darwin".into(),
                            size: 3,
                            sha256:
                                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                                    .into(),
                        },
                        install: "eggsact".into(),
                    },
                },
            ],
            evidence_references: vec![],
        };
        let spec = BootstrapSpec {
            origin: "https://example.invalid/releases".into(),
            fixture_http: false,
        };
        let a = render_posix(&c, &m, &spec).unwrap();
        assert_eq!(a, render_posix(&c, &m, &spec).unwrap());
        assert!(a.contains("--connect-timeout 10 --max-time 120"));
        assert!(!a.contains("sudo"));
        #[cfg(unix)]
        assert!(std::process::Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(&a)
            .status()
            .unwrap()
            .success());
        let ps = render_powershell(&c, &m, &spec).unwrap();
        assert!(ps.contains("Get-FileHash"));
        let ps_path =
            std::env::temp_dir().join(format!("eggpack-ps-parse-{}.ps1", std::process::id()));
        std::fs::write(&ps_path, &ps).unwrap();
        let shell = if cfg!(windows) {
            "powershell.exe"
        } else {
            "pwsh"
        };
        if Command::new(shell).arg("-Version").output().is_ok() {
            let parser = format!("$tokens=$null;$errors=$null;[System.Management.Automation.Language.Parser]::ParseFile({},[ref]$tokens,[ref]$errors)|Out-Null;if($errors.Count -gt 0){{exit 1}}", psq(&ps_path.to_string_lossy()));
            assert!(Command::new(shell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &parser])
                .status()
                .unwrap()
                .success());
        }
        let _ = std::fs::remove_file(ps_path);
        let bad = BootstrapSpec {
            origin: "http://example.invalid".into(),
            fixture_http: true,
        };
        assert!(render_posix(&c, &m, &bad).is_err());
    }
    #[test]
    fn mismatch_fails_before_text_is_emitted() {
        let c = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let m = ReleaseManifest {
            schema_version: 1,
            product_id: "other".into(),
            release_id: "1".into(),
            source_revision: "a".repeat(40),
            targets: vec![],
            evidence_references: vec![],
        };
        assert!(render_posix(
            &c,
            &m,
            &BootstrapSpec {
                origin: "https://x".into(),
                fixture_http: false
            }
        )
        .is_err());
    }
    #[cfg(unix)]
    #[test]
    fn generated_shell_installs_verified_bytes_and_refuses_existing_file() {
        use std::{fs, process::Command};
        let c = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let m = ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: "a".repeat(40),
            targets: vec![
                eggpack_manifest::TargetRecord {
                    target: "x86_64-unknown-linux-gnu".into(),
                    form: ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-x86_64-unknown-linux-gnu".into(),
                            size: 3,
                            sha256:
                                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                                    .into(),
                        },
                        install: "eggsact".into(),
                    },
                },
                eggpack_manifest::TargetRecord {
                    target: "aarch64-apple-darwin".into(),
                    form: ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-aarch64-apple-darwin".into(),
                            size: 3,
                            sha256:
                                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                                    .into(),
                        },
                        install: "eggsact".into(),
                    },
                },
            ],
            evidence_references: vec![],
        };
        let origin = serve_once(b"abc", "200 OK");
        let script = render_posix(
            &c,
            &m,
            &BootstrapSpec {
                origin,
                fixture_http: true,
            },
        )
        .unwrap();
        let root =
            std::env::temp_dir().join(format!("eggpack-bootstrap-test-{}", std::process::id()));
        let dest = root.join("destination");
        fs::create_dir_all(&root).unwrap();
        let script_path = root.join("install.sh");
        fs::write(&script_path, script).unwrap();
        assert!(Command::new("sh")
            .arg(&script_path)
            .arg(&dest)
            .status()
            .unwrap()
            .success());
        assert_eq!(fs::read(dest.join("eggsact")).unwrap(), b"abc");
        assert!(!Command::new("sh")
            .arg(&script_path)
            .arg(&dest)
            .output()
            .unwrap()
            .status
            .success());
        let host_target = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => "aarch64-apple-darwin",
            ("macos", _) => "x86_64-apple-darwin",
            (_, "aarch64") => "aarch64-unknown-linux-gnu",
            _ => "x86_64-unknown-linux-gnu",
        };
        let selected = m
            .targets
            .iter()
            .position(|target| target.target == host_target)
            .unwrap();
        if Command::new("pwsh").arg("-Version").output().is_ok() {
            let run_ps = |label: &str,
                          manifest: &ReleaseManifest,
                          body: &'static [u8],
                          response: &'static str| {
                let origin = serve_once(body, response);
                let script = render_powershell(
                    &c,
                    manifest,
                    &BootstrapSpec {
                        origin,
                        fixture_http: true,
                    },
                )
                .unwrap();
                let script_path = root.join(format!("{label}.ps1"));
                let destination = root.join(format!("{label}-destination"));
                fs::write(&script_path, script).unwrap();
                let result = Command::new("pwsh")
                    .arg("-NoProfile")
                    .arg("-File")
                    .arg(&script_path)
                    .arg(&destination)
                    .output()
                    .unwrap();
                (result.status.success(), destination, script_path)
            };
            let (ok, ps_dest, ps_path) = run_ps("install", &m, b"abc", "200 OK");
            assert!(ok);
            assert_eq!(fs::read(ps_dest.join("eggsact")).unwrap(), b"abc");
            let repeat = Command::new("pwsh")
                .arg("-NoProfile")
                .arg("-File")
                .arg(&ps_path)
                .arg(&ps_dest)
                .output()
                .unwrap();
            assert!(!repeat.status.success());
            let mut bad_size_ps = m.clone();
            if let ArtifactForm::Direct { artifact, .. } = &mut bad_size_ps.targets[selected].form {
                artifact.size = 4;
            }
            let (ok, dest, _) = run_ps("bad-size-powershell", &bad_size_ps, b"abc", "200 OK");
            assert!(!ok);
            assert_eq!(fs::read_dir(dest).unwrap().count(), 0);
            let mut bad_digest_ps = m.clone();
            if let ArtifactForm::Direct { artifact, .. } = &mut bad_digest_ps.targets[selected].form
            {
                artifact.sha256 = "00".repeat(32);
            }
            let (ok, dest, _) = run_ps("bad-digest-powershell", &bad_digest_ps, b"abc", "200 OK");
            assert!(!ok);
            assert_eq!(fs::read_dir(dest).unwrap().count(), 0);
            let (ok, dest, _) = run_ps("missing-powershell", &m, b"", "404 Not Found");
            assert!(!ok);
            assert_eq!(fs::read_dir(dest).unwrap().count(), 0);
        }
        let mut bad_size = m.clone();
        if let ArtifactForm::Direct { artifact, .. } = &mut bad_size.targets[selected].form {
            artifact.size = 4;
        }
        let origin = serve_once(b"abc", "200 OK");
        let bad_script = render_posix(
            &c,
            &bad_size,
            &BootstrapSpec {
                origin,
                fixture_http: true,
            },
        )
        .unwrap();
        let bad_path = root.join("bad-size.sh");
        let bad_dest = root.join("bad-size-dest");
        fs::write(&bad_path, bad_script).unwrap();
        assert!(!Command::new("sh")
            .arg(&bad_path)
            .arg(&bad_dest)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(fs::read_dir(&bad_dest).unwrap().count(), 0);
        let mut bad_digest = m.clone();
        if let ArtifactForm::Direct { artifact, .. } = &mut bad_digest.targets[selected].form {
            artifact.sha256 = "00".repeat(32);
        }
        let origin = serve_once(b"abc", "200 OK");
        let bad_script = render_posix(
            &c,
            &bad_digest,
            &BootstrapSpec {
                origin,
                fixture_http: true,
            },
        )
        .unwrap();
        let bad_path = root.join("bad-digest.sh");
        let bad_dest = root.join("bad-digest-dest");
        fs::write(&bad_path, bad_script).unwrap();
        assert!(!Command::new("sh")
            .arg(&bad_path)
            .arg(&bad_dest)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(fs::read_dir(&bad_dest).unwrap().count(), 0);
        let origin = serve_once(b"", "404 Not Found");
        let missing_script = render_posix(
            &c,
            &m,
            &BootstrapSpec {
                origin,
                fixture_http: true,
            },
        )
        .unwrap();
        let missing_path = root.join("missing.sh");
        let missing_dest = root.join("missing-dest");
        fs::write(&missing_path, missing_script).unwrap();
        assert!(!Command::new("sh")
            .arg(&missing_path)
            .arg(&missing_dest)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(fs::read_dir(&missing_dest).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    // -----------------------------------------------------------------------
    // M002 bundle/archive safety.
    // -----------------------------------------------------------------------
    use std::collections::HashMap;

    fn sha_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn serve_map(routes: HashMap<String, (Vec<u8>, String)>) -> String {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut request = [0u8; 8192];
                let Ok(n) = stream.read(&mut request) else {
                    continue;
                };
                let text = String::from_utf8_lossy(&request[..n]).to_string();
                let path = text
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                // Strip query and leading "/releases/" prefix handling: routes are keyed by
                // full path after host, e.g. "/releases/<filename>".
                let key = path.split('?').next().unwrap_or("").to_string();
                let (body, status) = routes
                    .get(&key)
                    .cloned()
                    .unwrap_or((vec![], "404 Not Found".into()));
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        format!("http://{address}/releases")
    }

    fn bundle_contract() -> DistributionContract {
        DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/codegg-bundle.toml"
        ))
        .unwrap()
    }

    fn host_triple() -> &'static str {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
            ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
            ("linux", _) => "x86_64-unknown-linux-gnu",
            ("macos", "aarch64") => "aarch64-apple-darwin",
            ("macos", _) => "x86_64-apple-darwin",
            ("windows", "x86_64") => "x86_64-pc-windows-msvc",
            ("windows", "aarch64") => "aarch64-pc-windows-msvc",
            _ => "x86_64-unknown-linux-gnu",
        }
    }

    fn host_bundle_case() -> (
        DistributionContract,
        ReleaseManifest,
        BootstrapInstallPolicyV1,
        Vec<(String, Vec<u8>)>,
    ) {
        let triple = host_triple();
        let contract = DistributionContract::parse_toml_str(&format!(
            r#"schema_version = 1
[product]
id = "hostbundle"
[[targets]]
triple = "{triple}"
aliases = []
[targets.asset]
kind = "bundle"
[[targets.asset.entries]]
asset = "{{product}}-{{version}}-{{target}}-main"
install = "host-main"
[[targets.asset.entries]]
asset = "{{product}}-helper-{{version}}-{{target}}"
install = "host-helper"
[[targets.asset.entries]]
asset = "{{product}}-manifest-{{version}}.json"
install = "host-manifest.json"
[targets.checksum]
sidecar = "{{asset}}.sha256"
"#
        ))
        .unwrap();
        let bodies = [
            b"host-main-bytes".to_vec(),
            b"host-helper-bytes".to_vec(),
            b"{\"manifest\":true}".to_vec(),
        ];
        let expanded = contract.expand(triple, "1.0.0").unwrap();
        let bundle = match expanded.assets {
            eggpack_contract::ExpandedAssets::Bundle(b) => b,
            _ => panic!("bundle expected"),
        };
        let mut entries = Vec::new();
        let mut modes = BTreeMap::new();
        for (index, contract_entry) in bundle.entries.iter().enumerate() {
            let body = &bodies[index];
            entries.push(eggpack_manifest::BundleRecord {
                artifact: eggpack_manifest::ArtifactRecord {
                    name: contract_entry.asset_file.clone(),
                    size: body.len() as u64,
                    sha256: sha_hex(body),
                },
                install: contract_entry.install_name.clone(),
            });
            let mode = if index == 2 {
                InstallMode::Data
            } else {
                InstallMode::Executable
            };
            modes.insert(contract_entry.install_name.clone(), mode);
        }
        let manifest = ReleaseManifest {
            schema_version: 1,
            product_id: "hostbundle".into(),
            release_id: "1.0.0".into(),
            source_revision: "a".repeat(40),
            targets: vec![eggpack_manifest::TargetRecord {
                target: triple.into(),
                form: ArtifactForm::Bundle { entries },
            }],
            evidence_references: vec![],
        };
        manifest.validate().unwrap();
        let policy = BootstrapInstallPolicyV1 {
            schema_version: 1,
            targets: [(
                triple.to_string(),
                TargetInstallPolicy {
                    modes,
                    archive_encoding: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        let routes = bundle
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.asset_file.clone(), bodies[index].clone()))
            .collect();
        (contract, manifest, policy, routes)
    }

    #[allow(clippy::type_complexity)]
    fn host_archive_case() -> (
        DistributionContract,
        ReleaseManifest,
        BootstrapInstallPolicyV1,
        Vec<u8>,
        Vec<(String, Vec<u8>)>,
    ) {
        let triple = host_triple();
        let contract = DistributionContract::parse_toml_str(&format!(
            r#"schema_version = 1
[product]
id = "hostarchive"
[[targets]]
triple = "{triple}"
aliases = []
[targets.asset]
kind = "archive"
asset = "{{product}}-{{version}}-{{target}}.tar.gz"
[[targets.asset.members]]
source = "hostbin"
install = "hostbin"
[[targets.asset.members]]
source = "bin/host-helper"
install = "host-helper"
[targets.checksum]
sidecar = "{{asset}}.sha256"
"#
        ))
        .unwrap();
        let member_bodies = vec![
            ("hostbin".to_string(), b"hostbin-bytes".to_vec()),
            ("bin/host-helper".to_string(), b"host-helper-bytes".to_vec()),
        ];
        let archive_bytes = build_deterministic_tar_gz(&[
            ("hostbin", member_bodies[0].1.as_slice()),
            ("bin/host-helper", member_bodies[1].1.as_slice()),
        ]);
        let expanded = contract.expand(triple, "1.0.0").unwrap();
        let archive = match expanded.assets {
            eggpack_contract::ExpandedAssets::Archive(a) => a,
            _ => panic!("archive expected"),
        };
        let members = archive
            .members
            .iter()
            .map(|contract_member| {
                let body = member_bodies
                    .iter()
                    .find(|(source, _)| source == &contract_member.source)
                    .unwrap();
                eggpack_manifest::ArchiveMemberRecord {
                    source: contract_member.source.clone(),
                    install: contract_member.install_name.clone(),
                    bytes: eggpack_manifest::ByteEvidence {
                        size: body.1.len() as u64,
                        sha256: sha_hex(&body.1),
                    },
                }
            })
            .collect::<Vec<_>>();
        let manifest = ReleaseManifest {
            schema_version: 1,
            product_id: "hostarchive".into(),
            release_id: "1.0.0".into(),
            source_revision: "b".repeat(40),
            targets: vec![eggpack_manifest::TargetRecord {
                target: triple.into(),
                form: ArtifactForm::Archive {
                    artifact: eggpack_manifest::ArtifactRecord {
                        name: archive.archive_file.clone(),
                        size: archive_bytes.len() as u64,
                        sha256: sha_hex(&archive_bytes),
                    },
                    members,
                },
            }],
            evidence_references: vec![],
        };
        manifest.validate().unwrap();
        let policy = BootstrapInstallPolicyV1 {
            schema_version: 1,
            targets: [(
                triple.to_string(),
                TargetInstallPolicy {
                    modes: [
                        ("hostbin".into(), InstallMode::Executable),
                        ("host-helper".into(), InstallMode::Data),
                    ]
                    .into_iter()
                    .collect(),
                    archive_encoding: Some(BundleArchiveEncoding::TarGzip),
                },
            )]
            .into_iter()
            .collect(),
        };
        (contract, manifest, policy, archive_bytes, member_bodies)
    }

    fn archive_contract() -> DistributionContract {
        DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/egress-archive.toml"
        ))
        .unwrap()
    }

    fn bundle_manifest_and_bodies() -> (
        ReleaseManifest,
        Vec<(String, Vec<u8>)>,
        BootstrapInstallPolicyV1,
    ) {
        // codegg-bundle: three entries, single target x86_64-unknown-linux-gnu.
        let bodies = vec![
            (b"codegg-main-bytes".to_vec(), "codegg"),
            (b"helper-bytes".to_vec(), "codegg-helper"),
            (b"{\"manifest\":true}".to_vec(), "codegg-manifest.json"),
        ];
        let contract = bundle_contract();
        let expanded = contract.expand("linux-x64", "2.4.0").unwrap();
        let bundle = match expanded.assets {
            eggpack_contract::ExpandedAssets::Bundle(b) => b,
            _ => panic!("bundle fixture changed"),
        };
        assert_eq!(bundle.entries.len(), 3);
        let mut manifest_entries = Vec::new();
        let mut policy_modes = BTreeMap::new();
        for (index, contract_entry) in bundle.entries.iter().enumerate() {
            let body = &bodies[index].0;
            manifest_entries.push(eggpack_manifest::BundleRecord {
                artifact: eggpack_manifest::ArtifactRecord {
                    name: contract_entry.asset_file.clone(),
                    size: body.len() as u64,
                    sha256: sha_hex(body),
                },
                install: contract_entry.install_name.clone(),
            });
            let mode = if index == 2 {
                InstallMode::Data
            } else {
                InstallMode::Executable
            };
            policy_modes.insert(contract_entry.install_name.clone(), mode);
        }
        let manifest = ReleaseManifest {
            schema_version: 1,
            product_id: "codegg".into(),
            release_id: "2.4.0".into(),
            source_revision: "a".repeat(40),
            targets: vec![eggpack_manifest::TargetRecord {
                target: "x86_64-unknown-linux-gnu".into(),
                form: ArtifactForm::Bundle {
                    entries: manifest_entries,
                },
            }],
            evidence_references: vec![],
        };
        manifest.validate().unwrap();
        let policy = BootstrapInstallPolicyV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                TargetInstallPolicy {
                    modes: policy_modes,
                    archive_encoding: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        let route_bodies = bundle_entries_route_bodies(&contract, &manifest, &bodies);
        (manifest, route_bodies, policy)
    }

    fn bundle_entries_route_bodies(
        contract: &DistributionContract,
        manifest: &ReleaseManifest,
        bodies: &[(Vec<u8>, &str)],
    ) -> Vec<(String, Vec<u8>)> {
        let expanded = contract.expand("linux-x64", &manifest.release_id).unwrap();
        let bundle = match expanded.assets {
            eggpack_contract::ExpandedAssets::Bundle(b) => b,
            _ => panic!("bundle expected"),
        };
        bundle
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.asset_file.clone(), bodies[index].0.clone()))
            .collect()
    }

    fn build_deterministic_tar_gz(members: &[(&str, &[u8])]) -> Vec<u8> {
        let file = Vec::new();
        let enc = flate2::GzBuilder::new()
            .mtime(0)
            .write(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(enc);
        archive.mode(tar::HeaderMode::Deterministic);
        for (name, bytes) in members {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_path(name).unwrap();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_cksum();
            archive.append(&header, *bytes).unwrap();
        }
        let enc = archive.into_inner().unwrap();
        enc.finish().unwrap()
    }

    /// Test-only minimal ustar writer capable of raw unsafe member names.
    ///
    /// The Rust `tar` builder validates paths, so traversal/absolute/backslash
    /// fixtures that must reach the generated consumer guard are built here
    /// byte-by-byte. Production validation is never weakened by this helper.
    struct RawTarEntry<'a> {
        name: &'a str,
        kind: u8,
        linkname: &'a str,
        data: &'a [u8],
    }

    fn build_raw_tar_gz(entries: &[RawTarEntry<'_>]) -> Vec<u8> {
        fn octal(value: u64, width: usize) -> Vec<u8> {
            let text = format!("{value:0width$o}", width = width - 1);
            let mut out = text.into_bytes();
            out.push(0);
            debug_assert_eq!(out.len(), width);
            out
        }
        let mut raw = Vec::new();
        for entry in entries {
            let mut header = [0u8; 512];
            let name = entry.name.as_bytes();
            assert!(
                !name.is_empty() && name.len() < 100,
                "raw name out of bounds"
            );
            header[..name.len()].copy_from_slice(name);
            header[100..108].copy_from_slice(&octal(0o755, 8));
            header[108..116].copy_from_slice(&octal(0, 8));
            header[116..124].copy_from_slice(&octal(0, 8));
            let size = if entry.kind == b'5' {
                0
            } else {
                entry.data.len() as u64
            };
            header[124..136].copy_from_slice(&octal(size, 12));
            header[136..148].copy_from_slice(&octal(0, 12));
            // Checksum field is spaces during computation.
            for b in header[148..156].iter_mut() {
                *b = b' ';
            }
            header[156] = entry.kind;
            let link = entry.linkname.as_bytes();
            assert!(link.len() < 100, "raw linkname out of bounds");
            header[157..157 + link.len()].copy_from_slice(link);
            header[257..262].copy_from_slice(b"ustar");
            header[263..265].copy_from_slice(b"00");
            let sum: u32 = header.iter().map(|b| *b as u32).sum();
            let text = format!("{sum:06o}\0 ");
            header[148..156].copy_from_slice(text.as_bytes());
            raw.extend_from_slice(&header);
            if entry.kind == b'0' {
                raw.extend_from_slice(entry.data);
                while raw.len() % 512 != 0 {
                    raw.push(0);
                }
            }
        }
        raw.extend_from_slice(&[0u8; 1024]);
        let file = Vec::new();
        let mut enc = flate2::GzBuilder::new()
            .mtime(0)
            .write(file, flate2::Compression::default());
        use std::io::Write;
        enc.write_all(&raw).unwrap();
        enc.finish().unwrap()
    }

    fn manifest_with_archive_bytes(manifest: &ReleaseManifest, bytes: &[u8]) -> ReleaseManifest {
        let mut updated = manifest.clone();
        if let ArtifactForm::Archive { artifact, .. } = &mut updated.targets[0].form {
            artifact.size = bytes.len() as u64;
            artifact.sha256 = sha_hex(bytes);
        } else {
            panic!("archive expected");
        }
        updated.validate().unwrap();
        updated
    }

    fn pwsh_available() -> bool {
        std::process::Command::new("pwsh")
            .arg("-Version")
            .output()
            .is_ok()
    }

    fn tar_exe_available() -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("tar.exe")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || std::process::Command::new("pwsh")
                    .args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "(Get-Command tar.exe -ErrorAction SilentlyContinue) -ne $null",
                    ])
                    .output()
                    .map(|o| {
                        o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "True"
                    })
                    .unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            // PowerShell installers shell to tar.exe; on Unix lanes pwsh can still
            // execute the script only where a tar.exe shim exists on PATH.
            std::process::Command::new("pwsh")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "(Get-Command tar.exe -ErrorAction SilentlyContinue) -ne $null",
                ])
                .output()
                .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "True")
                .unwrap_or(false)
        }
    }

    #[allow(clippy::type_complexity)]
    fn archive_manifest_policy_and_archive() -> (
        ReleaseManifest,
        BootstrapInstallPolicyV1,
        Vec<u8>,
        Vec<(String, Vec<u8>)>,
    ) {
        // egress-archive linux target: members "egress" and "bin/egress-helper".
        let member_bodies = vec![
            ("egress".to_string(), b"egress-binary-bytes".to_vec()),
            (
                "bin/egress-helper".to_string(),
                b"helper-binary-bytes".to_vec(),
            ),
        ];
        let archive_bytes = build_deterministic_tar_gz(&[
            ("egress", member_bodies[0].1.as_slice()),
            ("bin/egress-helper", member_bodies[1].1.as_slice()),
        ]);
        let contract = archive_contract();
        let expanded = contract.expand("linux-x64", "3.1.0").unwrap();
        let archive = match expanded.assets {
            eggpack_contract::ExpandedAssets::Archive(a) => a,
            _ => panic!("archive expected"),
        };
        assert_eq!(archive.members.len(), 2);
        let members = archive
            .members
            .iter()
            .map(|contract_member| {
                let body = member_bodies
                    .iter()
                    .find(|(source, _)| source == &contract_member.source)
                    .unwrap();
                eggpack_manifest::ArchiveMemberRecord {
                    source: contract_member.source.clone(),
                    install: contract_member.install_name.clone(),
                    bytes: eggpack_manifest::ByteEvidence {
                        size: body.1.len() as u64,
                        sha256: sha_hex(&body.1),
                    },
                }
            })
            .collect::<Vec<_>>();
        let manifest = ReleaseManifest {
            schema_version: 1,
            product_id: "egress".into(),
            release_id: "3.1.0".into(),
            source_revision: "b".repeat(40),
            targets: vec![eggpack_manifest::TargetRecord {
                target: "x86_64-unknown-linux-gnu".into(),
                form: ArtifactForm::Archive {
                    artifact: eggpack_manifest::ArtifactRecord {
                        name: archive.archive_file.clone(),
                        size: archive_bytes.len() as u64,
                        sha256: sha_hex(&archive_bytes),
                    },
                    members,
                },
            }],
            evidence_references: vec![],
        };
        manifest.validate().unwrap();
        let policy = BootstrapInstallPolicyV1 {
            schema_version: 1,
            targets: [(
                "x86_64-unknown-linux-gnu".into(),
                TargetInstallPolicy {
                    modes: [
                        ("egress".into(), InstallMode::Executable),
                        ("egress-helper".into(), InstallMode::Data),
                    ]
                    .into_iter()
                    .collect(),
                    archive_encoding: Some(BundleArchiveEncoding::TarGzip),
                },
            )]
            .into_iter()
            .collect(),
        };
        (manifest, policy, archive_bytes, member_bodies)
    }

    #[test]
    fn m002_policy_projection_matrix() {
        let bundle_contract = bundle_contract();
        let (bundle_manifest, _, bundle_policy) = bundle_manifest_and_bodies();
        let spec = BootstrapSpec {
            origin: "https://example.invalid/releases".into(),
            fixture_http: false,
        };
        // Bundle exact relationship.
        let projected =
            project_all(&bundle_contract, &bundle_manifest, &spec, &bundle_policy).unwrap();
        assert_eq!(projected.len(), 1);
        assert!(matches!(projected[0], TargetProjection::Bundle { .. }));

        // Archive exact relationship.
        let archive_contract = archive_contract();
        // Archive contract has two targets; build a two-target manifest for projection test.
        let (single_manifest, single_policy, _) = {
            let (m, p, _, _) = archive_manifest_policy_and_archive();
            (m, p, ())
        };
        // Single-target manifest vs two-target contract must reject (target-set mismatch).
        assert!(project_all(&archive_contract, &single_manifest, &spec, &single_policy).is_err());

        // Unknown target rejects.
        let mut unknown = single_manifest.clone();
        unknown.targets[0].target = "x86_64-pc-windows-gnu".into();
        assert!(project_all(&archive_contract, &unknown, &spec, &single_policy).is_err());

        // Missing policy rejects.
        assert!(project_all(
            &bundle_contract,
            &bundle_manifest,
            &spec,
            &BootstrapInstallPolicyV1::empty()
        )
        .is_err());

        // Extra policy rejects.
        let mut extra = bundle_policy.clone();
        extra.targets.insert(
            "aarch64-apple-darwin".into(),
            TargetInstallPolicy {
                modes: [("x".into(), InstallMode::Data)].into_iter().collect(),
                archive_encoding: None,
            },
        );
        assert!(project_all(&bundle_contract, &bundle_manifest, &spec, &extra).is_err());

        // Invalid version / unknown field rejects.
        assert!(
            BootstrapInstallPolicyV1::from_json(r#"{"schema_version":2,"targets":{}}"#).is_err()
        );
        assert!(BootstrapInstallPolicyV1::from_json(
            r#"{"schema_version":1,"targets":{},"extra":1}"#
        )
        .is_err());
        assert!(BootstrapInstallPolicyV1::from_toml("schema_version = 1\nunknown = 1").is_err());

        // Archive without TarGzip rejects.
        let mut no_encoding = single_policy.clone();
        no_encoding
            .targets
            .get_mut("x86_64-unknown-linux-gnu")
            .unwrap()
            .archive_encoding = None;
        // Need a manifest covering both archive targets for this check; use single-target
        // contract view instead: build a single-target contract for focused negative.
        let single_target_contract = DistributionContract::parse_toml_str(
            r#"schema_version = 1
[product]
id = "egress"
[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = []
[targets.asset]
kind = "archive"
asset = "{product}-{version}-{target}.tar.gz"
[[targets.asset.members]]
source = "egress"
install = "egress"
[targets.checksum]
sidecar = "{asset}.sha256"
"#,
        )
        .unwrap();
        let single_manifest_min = ReleaseManifest {
            schema_version: 1,
            product_id: "egress".into(),
            release_id: "3.1.0".into(),
            source_revision: "b".repeat(40),
            targets: vec![eggpack_manifest::TargetRecord {
                target: "x86_64-unknown-linux-gnu".into(),
                form: ArtifactForm::Archive {
                    artifact: eggpack_manifest::ArtifactRecord {
                        name: "egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz".into(),
                        size: 10,
                        sha256: "ab".repeat(32),
                    },
                    members: vec![eggpack_manifest::ArchiveMemberRecord {
                        source: "egress".into(),
                        install: "egress".into(),
                        bytes: eggpack_manifest::ByteEvidence {
                            size: 3,
                            sha256: "ab".repeat(32),
                        },
                    }],
                },
            }],
            evidence_references: vec![],
        };
        assert!(project_all(
            &single_target_contract,
            &single_manifest_min,
            &spec,
            &no_encoding
        )
        .is_err());

        // Archive policy on non-archive target rejects.
        let mut archive_on_bundle = bundle_policy.clone();
        archive_on_bundle
            .targets
            .get_mut("x86_64-unknown-linux-gnu")
            .unwrap()
            .archive_encoding = Some(BundleArchiveEncoding::TarGzip);
        assert!(project_all(
            &bundle_contract,
            &bundle_manifest,
            &spec,
            &archive_on_bundle
        )
        .is_err());

        // Mode map cannot rename an install.
        let mut renamed = bundle_policy.clone();
        let modes = &mut renamed
            .targets
            .get_mut("x86_64-unknown-linux-gnu")
            .unwrap()
            .modes;
        let first_key = modes.keys().next().unwrap().clone();
        let first_mode = modes.remove(&first_key).unwrap();
        modes.insert("renamed-binary".into(), first_mode);
        assert!(project_all(&bundle_contract, &bundle_manifest, &spec, &renamed).is_err());

        // Direct M001 projection unchanged: old renderers still succeed for direct-only.
        let direct_contract = DistributionContract::parse_toml_str(include_str!(
            "../../eggpack-contract/tests/fixtures/simple-direct.toml"
        ))
        .unwrap();
        let direct_manifest = ReleaseManifest {
            schema_version: 1,
            product_id: "eggsact".into(),
            release_id: "1.2.6".into(),
            source_revision: "a".repeat(40),
            targets: vec![
                eggpack_manifest::TargetRecord {
                    target: "x86_64-unknown-linux-gnu".into(),
                    form: ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-x86_64-unknown-linux-gnu".into(),
                            size: 3,
                            sha256:
                                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                                    .into(),
                        },
                        install: "eggsact".into(),
                    },
                },
                eggpack_manifest::TargetRecord {
                    target: "aarch64-apple-darwin".into(),
                    form: ArtifactForm::Direct {
                        artifact: eggpack_manifest::ArtifactRecord {
                            name: "eggsact-1.2.6-aarch64-apple-darwin".into(),
                            size: 3,
                            sha256:
                                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                                    .into(),
                        },
                        install: "eggsact".into(),
                    },
                },
            ],
            evidence_references: vec![],
        };
        let old_posix = render_posix(&direct_contract, &direct_manifest, &spec).unwrap();
        assert!(old_posix.contains("curl --fail"));
        // New renderers accept direct-only with empty policy.
        let new_posix = render_posix_with_policy(
            &direct_contract,
            &direct_manifest,
            &spec,
            &BootstrapInstallPolicyV1::empty(),
        )
        .unwrap();
        assert!(new_posix.contains("curl --fail"));
        assert_eq!(
            new_posix,
            render_posix_with_policy(
                &direct_contract,
                &direct_manifest,
                &spec,
                &BootstrapInstallPolicyV1::empty()
            )
            .unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn m002_bundle_posix_runtime_matrix() {
        use std::{fs, os::unix::fs::PermissionsExt, process::Command};
        let (contract, manifest, policy, route_bodies) = host_bundle_case();
        let mut routes = HashMap::new();
        for (name, body) in &route_bodies {
            routes.insert(format!("/releases/{name}"), (body.clone(), "200 OK".into()));
        }
        let origin = serve_map(routes.clone());
        let spec = BootstrapSpec {
            origin: origin.clone(),
            fixture_http: true,
        };
        let script = render_posix_with_policy(&contract, &manifest, &spec, &policy).unwrap();
        // Determinism.
        assert_eq!(
            script,
            render_posix_with_policy(&contract, &manifest, &spec, &policy).unwrap()
        );
        // Static checks.
        assert!(Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(&script)
            .status()
            .unwrap()
            .success());
        for forbidden in [
            "sudo",
            "Start-Process",
            "RunAs",
            "latest",
            "curl -L",
            "--location",
            "MaximumRedirection 1",
        ] {
            assert!(!script.contains(forbidden), "forbidden: {forbidden}");
        }
        assert!(!script
            .contains("curl --fail --silent --show-error --connect-timeout 10 --max-time 120 -L"));

        let root = std::env::temp_dir().join(format!("eggpack-m002-bundle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script_path = root.join("install.sh");
        fs::write(&script_path, &script).unwrap();

        // Success installs all members with explicit modes.
        let dest = root.join("dest-ok");
        assert!(Command::new("sh")
            .arg(&script_path)
            .arg(&dest)
            .status()
            .unwrap()
            .success());
        let main_bytes = fs::read(dest.join("host-main")).unwrap();
        assert_eq!(main_bytes, b"host-main-bytes");
        assert_eq!(
            fs::read(dest.join("host-helper")).unwrap(),
            b"host-helper-bytes"
        );
        assert_eq!(
            fs::read(dest.join("host-manifest.json")).unwrap(),
            b"{\"manifest\":true}"
        );
        assert_eq!(
            fs::metadata(dest.join("host-main"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(dest.join("host-helper"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(dest.join("host-manifest.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );
        // No overwrite.
        assert!(!Command::new("sh")
            .arg(&script_path)
            .arg(&dest)
            .output()
            .unwrap()
            .status
            .success());

        // Wrong size leaves no files.
        let mut bad_size = manifest.clone();
        if let ArtifactForm::Bundle { entries } = &mut bad_size.targets[0].form {
            entries[0].artifact.size += 1;
        }
        let bad_script = render_posix_with_policy(&contract, &bad_size, &spec, &policy).unwrap();
        let bad_path = root.join("bad-size.sh");
        let bad_dest = root.join("bad-size-dest");
        fs::write(&bad_path, bad_script).unwrap();
        assert!(!Command::new("sh")
            .arg(&bad_path)
            .arg(&bad_dest)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(fs::read_dir(&bad_dest).unwrap().count(), 0);

        // Wrong SHA leaves no files.
        let mut bad_sha = manifest.clone();
        if let ArtifactForm::Bundle { entries } = &mut bad_sha.targets[0].form {
            entries[1].artifact.sha256 = "00".repeat(32);
        }
        let bad_script = render_posix_with_policy(&contract, &bad_sha, &spec, &policy).unwrap();
        let bad_path = root.join("bad-sha.sh");
        let bad_dest = root.join("bad-sha-dest");
        fs::write(&bad_path, bad_script).unwrap();
        assert!(!Command::new("sh")
            .arg(&bad_path)
            .arg(&bad_dest)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(fs::read_dir(&bad_dest).unwrap().count(), 0);

        // Missing asset leaves no files.
        let mut missing_routes = routes.clone();
        let missing_name = route_bodies[0].0.clone();
        missing_routes.remove(&format!("/releases/{missing_name}"));
        let missing_origin = serve_map(missing_routes);
        let missing_spec = BootstrapSpec {
            origin: missing_origin,
            fixture_http: true,
        };
        let missing_script =
            render_posix_with_policy(&contract, &manifest, &missing_spec, &policy).unwrap();
        let missing_path = root.join("missing.sh");
        let missing_dest = root.join("missing-dest");
        fs::write(&missing_path, missing_script).unwrap();
        assert!(!Command::new("sh")
            .arg(&missing_path)
            .arg(&missing_dest)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(fs::read_dir(&missing_dest).unwrap().count(), 0);

        // Existing first and later destinations prevent placement.
        for existing in ["host-main", "host-manifest.json"] {
            let dest = root.join(format!("existing-{existing}"));
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join(existing), b"sentinel").unwrap();
            assert!(!Command::new("sh")
                .arg(&script_path)
                .arg(&dest)
                .output()
                .unwrap()
                .status
                .success());
            assert_eq!(fs::read(dest.join(existing)).unwrap(), b"sentinel");
            // No other files placed.
            assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
        }

        // Rollback after first placement via fault-injected ln.
        let fake_bin = root.join("fake-bin");
        fs::create_dir_all(&fake_bin).unwrap();
        fs::write(
            fake_bin.join("ln"),
            "#!/bin/sh\ncount_file=\"$FAKE_LN_COUNT\"\ncount=$(cat \"$count_file\" 2>/dev/null || echo 0)\ncount=$((count+1))\necho \"$count\" > \"$count_file\"\nif [ \"$count\" -ge 2 ]; then echo 'injected ln failure' >&2; exit 1; fi\nexec /bin/ln \"$@\"\n",
        )
        .unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(fake_bin.join("ln")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(fake_bin.join("ln"), perms).unwrap();
        }
        // Symlink required tools into fake bin (except ln which is faulted).
        for tool in [
            "curl",
            "sha256sum",
            "wc",
            "cut",
            "tr",
            "mktemp",
            "rm",
            "mkdir",
            "chmod",
            "cmp",
            "sort",
            "printf",
            "tar",
            "uname",
            "sh",
        ] {
            let src = format!("/usr/bin/{tool}");
            if std::path::Path::new(&src).exists() {
                let _ = std::os::unix::fs::symlink(&src, fake_bin.join(tool));
            }
            let sbin = format!("/bin/{tool}");
            if !std::path::Path::new(&fake_bin.join(tool)).exists()
                && std::path::Path::new(&sbin).exists()
            {
                let _ = std::os::unix::fs::symlink(&sbin, fake_bin.join(tool));
            }
        }
        let count_file = root.join("ln-count");
        fs::write(&count_file, b"0").unwrap();
        let rollback_dest = root.join("rollback-dest");
        let output = Command::new("sh")
            .arg(&script_path)
            .arg(&rollback_dest)
            .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
            .env("FAKE_LN_COUNT", &count_file)
            .output()
            .unwrap();
        assert!(!output.status.success());
        // Only invocation-created files are rolled back; dest contains no partial files.
        if rollback_dest.exists() {
            let count = fs::read_dir(&rollback_dest).map(|d| d.count()).unwrap_or(0);
            assert_eq!(count, 0, "rollback must remove partial placement");
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    #[allow(clippy::type_complexity)]
    fn m002_archive_posix_runtime_matrix() {
        use std::{fs, os::unix::fs::PermissionsExt, process::Command};
        // Host-specific single-target archive case so Unix lanes (Linux/macOS) execute.
        let (contract, manifest, policy, archive_bytes, member_bodies) = host_archive_case();
        let member_main = member_bodies[0].1.clone();
        let member_helper = member_bodies[1].1.clone();
        let archive_name = match &manifest.targets[0].form {
            ArtifactForm::Archive { artifact, .. } => artifact.name.clone(),
            _ => panic!("archive expected"),
        };
        let mut routes = HashMap::new();
        routes.insert(
            format!("/releases/{archive_name}"),
            (archive_bytes.clone(), "200 OK".into()),
        );
        let origin = serve_map(routes.clone());
        let spec = BootstrapSpec {
            origin,
            fixture_http: true,
        };
        let script = render_posix_with_policy(&contract, &manifest, &spec, &policy).unwrap();
        assert_eq!(
            script,
            render_posix_with_policy(&contract, &manifest, &spec, &policy).unwrap()
        );
        assert!(Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(&script)
            .status()
            .unwrap()
            .success());
        assert!(!script.contains("sudo"));
        assert!(!script.contains("unzip"));

        let root =
            std::env::temp_dir().join(format!("eggpack-m002-archive-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script_path = root.join("install.sh");
        fs::write(&script_path, &script).unwrap();

        // Success flattens nested source to install name.
        let dest = root.join("dest-ok");
        assert!(Command::new("sh")
            .arg(&script_path)
            .arg(&dest)
            .status()
            .unwrap()
            .success());
        assert_eq!(fs::read(dest.join("hostbin")).unwrap(), member_main);
        assert_eq!(fs::read(dest.join("host-helper")).unwrap(), member_helper);
        assert!(!dest.join("bin").exists());
        assert_eq!(
            fs::metadata(dest.join("hostbin"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(dest.join("host-helper"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );

        // Archive size / SHA mismatch rejects without placement.
        for mutate_size in [true, false] {
            let mut bad = manifest.clone();
            if let ArtifactForm::Archive { artifact, .. } = &mut bad.targets[0].form {
                if mutate_size {
                    artifact.size += 1;
                } else {
                    artifact.sha256 = "00".repeat(32);
                }
            }
            let bad_script = render_posix_with_policy(&contract, &bad, &spec, &policy).unwrap();
            let bad_path = root.join(format!("bad-archive-{mutate_size}.sh"));
            let bad_dest = root.join(format!("bad-archive-{mutate_size}-dest"));
            fs::write(&bad_path, bad_script).unwrap();
            assert!(!Command::new("sh")
                .arg(&bad_path)
                .arg(&bad_dest)
                .output()
                .unwrap()
                .status
                .success());
            assert_eq!(fs::read_dir(&bad_dest).unwrap().count(), 0);
        }

        // Member size / SHA mismatch rejects.
        for mutate_size in [true, false] {
            let mut bad = manifest.clone();
            if let ArtifactForm::Archive { members, .. } = &mut bad.targets[0].form {
                if mutate_size {
                    members[0].bytes.size += 1;
                } else {
                    members[1].bytes.sha256 = "11".repeat(32);
                }
            }
            let bad_script = render_posix_with_policy(&contract, &bad, &spec, &policy).unwrap();
            let bad_path = root.join(format!("bad-member-{mutate_size}.sh"));
            let bad_dest = root.join(format!("bad-member-{mutate_size}-dest"));
            fs::write(&bad_path, bad_script).unwrap();
            assert!(!Command::new("sh")
                .arg(&bad_path)
                .arg(&bad_dest)
                .output()
                .unwrap()
                .status
                .success());
            // Destination may exist (mkdir before download) but must contain no installed files.
            if bad_dest.exists() {
                let installed = fs::read_dir(&bad_dest)
                    .unwrap()
                    .filter(|e| {
                        let name = e
                            .as_ref()
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .into_owned();
                        !name.starts_with(".eggpack.")
                    })
                    .count();
                assert_eq!(installed, 0);
            }
        }

        // Missing / extra / traversal / absolute / backslash / symlink / type
        // members reject at the inner inventory/path/type defense. Every case
        // first updates the manifest outer archive size/SHA to the exact
        // tampered bytes so outer verification passes; the expected member
        // records remain authoritative.
        let tampered_cases: Vec<(&str, Vec<u8>, &str)> = vec![
            (
                "missing",
                build_deterministic_tar_gz(&[("hostbin", member_main.as_slice())]),
                "archive member inventory mismatch",
            ),
            (
                "extra",
                build_deterministic_tar_gz(&[
                    ("hostbin", member_main.as_slice()),
                    ("bin/host-helper", member_helper.as_slice()),
                    ("extra-file", b"extra" as &[u8]),
                ]),
                "archive member inventory mismatch",
            ),
            (
                "traversal",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "../evil",
                        kind: b'0',
                        linkname: "",
                        data: member_helper.as_slice(),
                    },
                ]),
                "unsafe archive member",
            ),
            (
                "absolute",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "/abs",
                        kind: b'0',
                        linkname: "",
                        data: member_helper.as_slice(),
                    },
                ]),
                "unsafe archive member",
            ),
            (
                "backslash",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "bin\\evil",
                        kind: b'0',
                        linkname: "",
                        data: member_helper.as_slice(),
                    },
                ]),
                "unsafe archive member",
            ),
            (
                "symlink-inner",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "bin/host-helper",
                        kind: b'2',
                        linkname: "hostbin",
                        data: &[],
                    },
                ]),
                "symlink member rejected",
            ),
            (
                "directory",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "bin/host-helper",
                        kind: b'5',
                        linkname: "",
                        data: &[],
                    },
                ]),
                "member is not a regular file",
            ),
        ];
        for (label, bytes, expected_guard) in &tampered_cases {
            let tampered_manifest = manifest_with_archive_bytes(&manifest, bytes);
            let mut tampered_routes = HashMap::new();
            tampered_routes.insert(
                format!("/releases/{archive_name}"),
                (bytes.clone(), "200 OK".into()),
            );
            let tampered_origin = serve_map(tampered_routes);
            let tampered_spec = BootstrapSpec {
                origin: tampered_origin,
                fixture_http: true,
            };
            let tampered_script =
                render_posix_with_policy(&contract, &tampered_manifest, &tampered_spec, &policy)
                    .unwrap();
            let tampered_path = root.join(format!("tampered-{label}.sh"));
            let tampered_dest = root.join(format!("tampered-{label}-dest"));
            fs::write(&tampered_path, &tampered_script).unwrap();
            let output = Command::new("sh")
                .arg(&tampered_path)
                .arg(&tampered_dest)
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                !output.status.success(),
                "tampered archive {label} must fail"
            );
            // Outer archive verification must have passed: the failure is the
            // intended inner member defense, not an outer size/SHA mismatch.
            assert!(
                !stderr.contains("size mismatch") && !stderr.contains("SHA-256 mismatch"),
                "tampered archive {label} must pass outer digest, got: {stderr}"
            );
            assert!(
                stderr.contains(expected_guard),
                "tampered archive {label} must reach '{expected_guard}', got: {stderr}"
            );
            // No final installed files exist.
            if tampered_dest.exists() {
                let installed = fs::read_dir(&tampered_dest)
                    .unwrap()
                    .filter(|e| {
                        let name = e
                            .as_ref()
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .into_owned();
                        !name.starts_with(".eggpack.")
                    })
                    .count();
                assert_eq!(
                    installed, 0,
                    "tampered archive {label} must install nothing"
                );
            }
            // Destination remains unchanged when it pre-exists.
            let preserved = root.join(format!("tampered-{label}-preserved"));
            fs::create_dir_all(&preserved).unwrap();
            fs::write(preserved.join("hostbin"), b"sentinel").unwrap();
            assert!(!Command::new("sh")
                .arg(&tampered_path)
                .arg(&preserved)
                .output()
                .unwrap()
                .status
                .success());
            assert_eq!(fs::read(preserved.join("hostbin")).unwrap(), b"sentinel");
        }

        // Symlink member rejects at the inner type defense (outer digest valid).
        {
            let file = Vec::new();
            let enc = flate2::GzBuilder::new()
                .mtime(0)
                .write(file, flate2::Compression::default());
            let mut archive = tar::Builder::new(enc);
            archive.mode(tar::HeaderMode::Deterministic);
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_path("hostbin").unwrap();
            header.set_size(member_main.len() as u64);
            header.set_mode(0o755);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_cksum();
            archive.append(&header, member_main.as_slice()).unwrap();
            let mut link_header = tar::Header::new_gnu();
            link_header.set_entry_type(tar::EntryType::Symlink);
            link_header.set_path("bin/host-helper").unwrap();
            link_header.set_link_name("hostbin").unwrap();
            link_header.set_mode(0o777);
            link_header.set_uid(0);
            link_header.set_gid(0);
            link_header.set_mtime(0);
            link_header.set_cksum();
            archive.append(&link_header, &[][..]).unwrap();
            let enc = archive.into_inner().unwrap();
            let symlink_bytes = enc.finish().unwrap();
            let symlink_manifest = manifest_with_archive_bytes(&manifest, &symlink_bytes);
            let mut symlink_routes = HashMap::new();
            symlink_routes.insert(
                format!("/releases/{archive_name}"),
                (symlink_bytes, "200 OK".into()),
            );
            let symlink_origin = serve_map(symlink_routes);
            let symlink_spec = BootstrapSpec {
                origin: symlink_origin,
                fixture_http: true,
            };
            let symlink_script =
                render_posix_with_policy(&contract, &symlink_manifest, &symlink_spec, &policy)
                    .unwrap();
            let symlink_path = root.join("symlink.sh");
            let symlink_dest = root.join("symlink-dest");
            fs::write(&symlink_path, symlink_script).unwrap();
            let output = Command::new("sh")
                .arg(&symlink_path)
                .arg(&symlink_dest)
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success());
            assert!(
                !stderr.contains("size mismatch") && !stderr.contains("SHA-256 mismatch"),
                "symlink case must pass outer digest, got: {stderr}"
            );
            assert!(
                stderr.contains("symlink member rejected"),
                "symlink case must reach type guard, got: {stderr}"
            );
        }

        // Archive placement rollback: fault-injected ln fails on the second
        // final move; the first invocation-created destination must be removed.
        {
            let fake_bin = root.join("archive-fake-bin");
            fs::create_dir_all(&fake_bin).unwrap();
            fs::write(
                fake_bin.join("ln"),
                "#!/bin/sh\ncount_file=\"$FAKE_LN_COUNT\"\ncount=$(cat \"$count_file\" 2>/dev/null || echo 0)\ncount=$((count+1))\necho \"$count\" > \"$count_file\"\nif [ \"$count\" -ge 2 ]; then echo 'injected ln failure' >&2; exit 1; fi\nexec /bin/ln \"$@\"\n",
            )
            .unwrap();
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(fake_bin.join("ln")).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(fake_bin.join("ln"), perms).unwrap();
            }
            for tool in [
                "curl",
                "sha256sum",
                "wc",
                "cut",
                "tr",
                "mktemp",
                "rm",
                "mkdir",
                "chmod",
                "cmp",
                "sort",
                "printf",
                "tar",
                "uname",
                "sh",
            ] {
                for prefix in ["/usr/bin", "/bin"] {
                    let src = format!("{prefix}/{tool}");
                    if std::path::Path::new(&src).exists() && !fake_bin.join(tool).exists() {
                        let _ = std::os::unix::fs::symlink(&src, fake_bin.join(tool));
                    }
                }
            }
            let count_file = root.join("archive-ln-count");
            fs::write(&count_file, b"0").unwrap();
            // Pre-existing second install name is not owned by the installer;
            // use a fresh dest and let the injected ln failure trigger rollback
            // of the first placed file instead.
            let rollback_dest = root.join("archive-rollback-dest");
            let output = Command::new("sh")
                .arg(&script_path)
                .arg(&rollback_dest)
                .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
                .env("FAKE_LN_COUNT", &count_file)
                .output()
                .unwrap();
            assert!(!output.status.success());
            if rollback_dest.exists() {
                let count = fs::read_dir(&rollback_dest)
                    .map(|d| {
                        d.filter(|e| {
                            !e.as_ref()
                                .unwrap()
                                .file_name()
                                .to_string_lossy()
                                .starts_with(".eggpack.")
                        })
                        .count()
                    })
                    .unwrap_or(0);
                assert_eq!(count, 0, "archive rollback must remove partial placement");
            }
        }

        // Unavailable tar rejects before placement.
        {
            let empty_bin = root.join("empty-bin");
            fs::create_dir_all(&empty_bin).unwrap();
            for tool in [
                "curl",
                "sha256sum",
                "wc",
                "cut",
                "tr",
                "mktemp",
                "rm",
                "mkdir",
                "chmod",
                "cmp",
                "sort",
                "printf",
                "uname",
            ] {
                for prefix in ["/usr/bin", "/bin"] {
                    let src = format!("{prefix}/{tool}");
                    if std::path::Path::new(&src).exists() && !empty_bin.join(tool).exists() {
                        let _ = std::os::unix::fs::symlink(&src, empty_bin.join(tool));
                    }
                }
            }
            // Ensure tar is absent.
            assert!(!empty_bin.join("tar").exists());
            let no_tar_dest = root.join("no-tar-dest");
            let output = Command::new("/bin/sh")
                .arg(&script_path)
                .arg(&no_tar_dest)
                .env("PATH", &empty_bin)
                .output()
                .unwrap();
            assert!(!output.status.success());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("tar is required"));
            if no_tar_dest.exists() {
                let installed = fs::read_dir(&no_tar_dest)
                    .unwrap()
                    .filter(|e| {
                        let name = e
                            .as_ref()
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .into_owned();
                        !name.starts_with(".eggpack.")
                    })
                    .count();
                assert_eq!(installed, 0);
            }
        }

        // Pre-existing destination preserved.
        {
            let dest = root.join("preexisting");
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("hostbin"), b"sentinel").unwrap();
            assert!(!Command::new("sh")
                .arg(&script_path)
                .arg(&dest)
                .output()
                .unwrap()
                .status
                .success());
            assert_eq!(fs::read(dest.join("hostbin")).unwrap(), b"sentinel");
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn m002_powershell_static_and_runtime() {
        use std::{fs, process::Command};
        let (contract, manifest, policy, route_bodies) = host_bundle_case();
        let mut routes = HashMap::new();
        for (name, body) in &route_bodies {
            routes.insert(format!("/releases/{name}"), (body.clone(), "200 OK".into()));
        }
        let origin = serve_map(routes);
        let spec = BootstrapSpec {
            origin,
            fixture_http: true,
        };
        let ps_bundle =
            render_powershell_with_policy(&contract, &manifest, &spec, &policy).unwrap();
        assert_eq!(
            ps_bundle,
            render_powershell_with_policy(&contract, &manifest, &spec, &policy).unwrap()
        );
        for forbidden in [
            "sudo",
            "Start-Process",
            "RunAs",
            "latest",
            "-MaximumRedirection 1",
        ] {
            assert!(!ps_bundle.contains(forbidden));
        }
        assert!(ps_bundle.contains("-MaximumRedirection 0"));
        assert!(!ps_bundle.contains("curl"));

        // PowerShell parser succeeds where pwsh/powershell is available.
        let shell = if cfg!(windows) {
            "powershell.exe"
        } else {
            "pwsh"
        };
        let parser_available = Command::new(shell).arg("-Version").output().is_ok();
        if parser_available {
            let ps_path =
                std::env::temp_dir().join(format!("eggpack-m002-{}.ps1", std::process::id()));
            fs::write(&ps_path, &ps_bundle).unwrap();
            let parser = format!("$tokens=$null;$errors=$null;[System.Management.Automation.Language.Parser]::ParseFile({},[ref]$tokens,[ref]$errors)|Out-Null;if($errors.Count -gt 0){{exit 1}}", psq(&ps_path.to_string_lossy()));
            assert!(Command::new(shell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &parser])
                .status()
                .unwrap()
                .success());
            // Runtime bundle install via pwsh 7 (Get-FileHash/tar.exe behavior).
            // Windows powershell.exe 5.1 lacks reliable Get-FileHash in CI, so use
            // pwsh where available; static/parser coverage still runs on 5.1.
            if Command::new("pwsh").arg("-Version").output().is_ok() {
                let root =
                    std::env::temp_dir().join(format!("eggpack-m002-ps-{}", std::process::id()));
                let _ = fs::remove_dir_all(&root);
                fs::create_dir_all(&root).unwrap();
                let script_path = root.join("install.ps1");
                fs::write(&script_path, &ps_bundle).unwrap();
                let dest = root.join("dest-ok");
                let result = Command::new("pwsh")
                    .arg("-NoProfile")
                    .arg("-File")
                    .arg(&script_path)
                    .arg(&dest)
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "pwsh bundle install failed: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(
                    fs::read(dest.join("host-main")).unwrap(),
                    b"host-main-bytes"
                );
                // No overwrite.
                let repeat = Command::new("pwsh")
                    .arg("-NoProfile")
                    .arg("-File")
                    .arg(&script_path)
                    .arg(&dest)
                    .output()
                    .unwrap();
                assert!(!repeat.status.success());
                let _ = fs::remove_dir_all(root);
            }
            let _ = fs::remove_file(ps_path);
        }

        // Archive PowerShell static checks (host-specific so parser runs everywhere).
        let (single_contract, archive_manifest, archive_policy, _, _) = host_archive_case();
        let archive_spec = BootstrapSpec {
            origin: "https://example.invalid/releases".into(),
            fixture_http: false,
        };
        let ps_archive = render_powershell_with_policy(
            &single_contract,
            &archive_manifest,
            &archive_spec,
            &archive_policy,
        )
        .unwrap();
        assert!(ps_archive.contains("tar.exe"));
        assert!(!ps_archive.contains("sudo"));
        // Archive installers roll back only invocation-created files.
        assert!(ps_archive.contains("foreach ($p in $created)"));
        if parser_available {
            let ps_path =
                std::env::temp_dir().join(format!("eggpack-m002-arch-{}.ps1", std::process::id()));
            fs::write(&ps_path, &ps_archive).unwrap();
            let parser = format!("$tokens=$null;$errors=$null;[System.Management.Automation.Language.Parser]::ParseFile({},[ref]$tokens,[ref]$errors)|Out-Null;if($errors.Count -gt 0){{exit 1}}", psq(&ps_path.to_string_lossy()));
            assert!(Command::new(shell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &parser])
                .status()
                .unwrap()
                .success());
            let _ = fs::remove_file(ps_path);
        }
    }

    #[test]
    fn m002a_powershell_archive_runtime() {
        use std::{fs, process::Command};
        // Qualification lane: Windows must provide pwsh 7 + tar.exe and may not
        // silently skip. Other platforms run when pwsh + tar.exe are present
        // and skip otherwise (regression guard, not qualification evidence).
        if cfg!(windows) {
            assert!(pwsh_available(), "pwsh 7 is required on the Windows lane");
            assert!(
                tar_exe_available(),
                "tar.exe is required on the Windows lane"
            );
        } else if !pwsh_available() || !tar_exe_available() {
            eprintln!("skip: pwsh + tar.exe unavailable on non-Windows lane");
            return;
        }
        let (contract, manifest, policy, archive_bytes, member_bodies) = host_archive_case();
        let member_main = member_bodies[0].1.clone();
        let member_helper = member_bodies[1].1.clone();
        let archive_name = match &manifest.targets[0].form {
            ArtifactForm::Archive { artifact, .. } => artifact.name.clone(),
            _ => panic!("archive expected"),
        };
        let mut routes = HashMap::new();
        routes.insert(
            format!("/releases/{archive_name}"),
            (archive_bytes.clone(), "200 OK".into()),
        );
        let origin = serve_map(routes);
        let spec = BootstrapSpec {
            origin,
            fixture_http: true,
        };
        let script = render_powershell_with_policy(&contract, &manifest, &spec, &policy).unwrap();
        assert_eq!(
            script,
            render_powershell_with_policy(&contract, &manifest, &spec, &policy).unwrap()
        );
        assert!(script.contains("tar.exe"));
        assert!(!script.contains("sudo"));
        assert!(!script.contains("Start-Process"));
        assert!(script.contains("-MaximumRedirection 0"));

        let root =
            std::env::temp_dir().join(format!("eggpack-m002a-ps-arch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script_path = root.join("install.ps1");
        fs::write(&script_path, &script).unwrap();

        fn pwsh_run(script: &std::path::Path, dest: &std::path::Path) -> std::process::Output {
            Command::new("pwsh")
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-File")
                .arg(script)
                .arg(dest)
                .output()
                .unwrap()
        }
        fn combined(output: &std::process::Output) -> String {
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        }
        fn installed_files(dest: &std::path::Path) -> usize {
            if !dest.exists() {
                return 0;
            }
            fs::read_dir(dest)
                .unwrap()
                .filter(|e| {
                    let name = e
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .into_owned();
                    !name.starts_with(".eggpack-")
                })
                .count()
        }

        // Positive: exact bytes, nested-source flattening, no-overwrite.
        let dest = root.join("dest-ok");
        let ok = pwsh_run(&script_path, &dest);
        assert!(
            ok.status.success(),
            "pwsh archive install failed: {}",
            combined(&ok)
        );
        assert_eq!(fs::read(dest.join("hostbin")).unwrap(), member_main);
        assert_eq!(fs::read(dest.join("host-helper")).unwrap(), member_helper);
        assert!(!dest.join("bin").exists());
        // Pre-existing destination preservation.
        let repeat = pwsh_run(&script_path, &dest);
        assert!(!repeat.status.success());
        assert_eq!(fs::read(dest.join("hostbin")).unwrap(), member_main);
        assert_eq!(installed_files(&dest), 2);

        // Archive integrity negatives fail before member listing/extraction.
        for (label, mut bad) in [("size", manifest.clone()), ("sha", manifest.clone())] {
            if let ArtifactForm::Archive { artifact, .. } = &mut bad.targets[0].form {
                if label == "size" {
                    artifact.size += 1;
                } else {
                    artifact.sha256 = "00".repeat(32);
                }
            }
            let bad_script =
                render_powershell_with_policy(&contract, &bad, &spec, &policy).unwrap();
            let bad_path = root.join(format!("bad-archive-{label}.ps1"));
            let bad_dest = root.join(format!("bad-archive-{label}-dest"));
            fs::write(&bad_path, &bad_script).unwrap();
            let out = pwsh_run(&bad_path, &bad_dest);
            assert!(!out.status.success(), "wrong archive {label} must fail");
            assert_eq!(installed_files(&bad_dest), 0);
        }

        // Inner archive negatives: valid outer digest, intended guard rejects.
        let inner_cases: Vec<(&str, Vec<u8>, &str)> = vec![
            (
                "missing",
                build_deterministic_tar_gz(&[("hostbin", member_main.as_slice())]),
                "archive member inventory mismatch",
            ),
            (
                "extra",
                build_deterministic_tar_gz(&[
                    ("hostbin", member_main.as_slice()),
                    ("bin/host-helper", member_helper.as_slice()),
                    ("extra-file", b"extra" as &[u8]),
                ]),
                "archive member inventory mismatch",
            ),
            (
                "traversal",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "../evil",
                        kind: b'0',
                        linkname: "",
                        data: member_helper.as_slice(),
                    },
                ]),
                "unsafe archive member",
            ),
            (
                "absolute",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "/abs",
                        kind: b'0',
                        linkname: "",
                        data: member_helper.as_slice(),
                    },
                ]),
                "unsafe archive member",
            ),
            (
                "backslash",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "bin\\evil",
                        kind: b'0',
                        linkname: "",
                        data: member_helper.as_slice(),
                    },
                ]),
                "unsafe archive member",
            ),
            (
                "symlink",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "bin/host-helper",
                        kind: b'2',
                        linkname: "hostbin",
                        data: &[],
                    },
                ]),
                "symlink member rejected",
            ),
            (
                "directory",
                build_raw_tar_gz(&[
                    RawTarEntry {
                        name: "hostbin",
                        kind: b'0',
                        linkname: "",
                        data: member_main.as_slice(),
                    },
                    RawTarEntry {
                        name: "bin/host-helper",
                        kind: b'5',
                        linkname: "",
                        data: &[],
                    },
                ]),
                "member is not a regular file",
            ),
        ];
        for (label, bytes, expected_guard) in &inner_cases {
            let tampered_manifest = manifest_with_archive_bytes(&manifest, bytes);
            let mut tampered_routes = HashMap::new();
            tampered_routes.insert(
                format!("/releases/{archive_name}"),
                (bytes.clone(), "200 OK".into()),
            );
            let tampered_origin = serve_map(tampered_routes);
            let tampered_spec = BootstrapSpec {
                origin: tampered_origin,
                fixture_http: true,
            };
            let tampered_script = render_powershell_with_policy(
                &contract,
                &tampered_manifest,
                &tampered_spec,
                &policy,
            )
            .unwrap();
            let tampered_path = root.join(format!("tampered-{label}.ps1"));
            let tampered_dest = root.join(format!("tampered-{label}-dest"));
            fs::write(&tampered_path, &tampered_script).unwrap();
            let out = pwsh_run(&tampered_path, &tampered_dest);
            let text = combined(&out);
            assert!(!out.status.success(), "tampered archive {label} must fail");
            assert!(
                !text.contains("size mismatch") || text.contains(expected_guard),
                "tampered archive {label} must pass outer digest, got: {text}"
            );
            assert!(
                text.contains(expected_guard),
                "tampered archive {label} must reach '{expected_guard}', got: {text}"
            );
            assert_eq!(
                installed_files(&tampered_dest),
                0,
                "tampered archive {label} must install nothing"
            );
        }

        // Member evidence negatives reach member checks after extraction.
        for (label, mut bad) in [
            ("member-size", manifest.clone()),
            ("member-sha", manifest.clone()),
        ] {
            if let ArtifactForm::Archive { members, .. } = &mut bad.targets[0].form {
                if label == "member-size" {
                    members[0].bytes.size += 1;
                } else {
                    members[1].bytes.sha256 = "11".repeat(32);
                }
            }
            let bad_script =
                render_powershell_with_policy(&contract, &bad, &spec, &policy).unwrap();
            let bad_path = root.join(format!("{label}.ps1"));
            let bad_dest = root.join(format!("{label}-dest"));
            fs::write(&bad_path, &bad_script).unwrap();
            let out = pwsh_run(&bad_path, &bad_dest);
            assert!(!out.status.success(), "{label} must fail");
            assert_eq!(installed_files(&bad_dest), 0);
        }

        // Tool boundary: tar.exe unavailable fails before placement.
        {
            let no_tar_dest = root.join("no-tar-dest");
            let probe = Command::new("pwsh")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "$env:PATH=''; & {} {}",
                        script_path.to_string_lossy(),
                        no_tar_dest.to_string_lossy()
                    ),
                ])
                .output()
                .unwrap();
            // On lanes where PATH='' still resolves tar.exe (Windows app-alias
            // path), fall back to asserting the static tool boundary: the
            // generated script requires tar.exe before extraction/placement.
            let text = combined(&probe);
            if text.contains("tar.exe is required") {
                assert!(!probe.status.success());
                assert_eq!(installed_files(&no_tar_dest), 0);
            } else {
                assert!(script.contains("tar.exe is required"));
            }
        }

        // Placement failure / rollback: pre-existing second install name is not
        // owned by the installer. The first move succeeds, the second fails,
        // and the first invocation-created destination is rolled back while the
        // pre-existing sentinel remains untouched. This exercises the shared
        // `$created` rollback primitive for archives without adding a test hook
        // to generated installers.
        {
            let dest = root.join("rollback-dest");
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("host-helper"), b"sentinel").unwrap();
            let out = pwsh_run(&script_path, &dest);
            assert!(
                !out.status.success(),
                "archive placement conflict must fail"
            );
            assert_eq!(fs::read(dest.join("host-helper")).unwrap(), b"sentinel");
            assert!(
                !dest.join("hostbin").exists(),
                "first archive file must be rolled back"
            );
        }

        // Pre-existing first destination is preserved and installs nothing else.
        {
            let dest = root.join("preexisting");
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("hostbin"), b"sentinel").unwrap();
            let out = pwsh_run(&script_path, &dest);
            assert!(!out.status.success());
            assert_eq!(fs::read(dest.join("hostbin")).unwrap(), b"sentinel");
            assert_eq!(installed_files(&dest), 1);
        }

        fs::remove_dir_all(root).unwrap();
    }
}
