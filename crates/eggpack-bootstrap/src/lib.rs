#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Deterministic direct-release first-install script generation.
use eggpack_contract::{DistributionContract, ExpandedAssets};
use eggpack_manifest::{ArtifactForm, ReleaseManifest};
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
            .contains(['\n', '\r', '\'', '"', '`', '$', '\\', '?', '#', ' '])
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
        if Command::new("pwsh").arg("-Version").output().is_ok() {
            let origin = serve_once(b"abc", "200 OK");
            let ps = render_powershell(
                &c,
                &m,
                &BootstrapSpec {
                    origin,
                    fixture_http: true,
                },
            )
            .unwrap();
            let ps_path = root.join("install.ps1");
            let ps_dest = root.join("powershell-destination");
            fs::write(&ps_path, ps).unwrap();
            let powershell_result = Command::new("pwsh")
                .arg("-NoProfile")
                .arg("-File")
                .arg(&ps_path)
                .arg(&ps_dest)
                .output()
                .unwrap();
            assert!(
                powershell_result.status.success(),
                "{}",
                String::from_utf8_lossy(&powershell_result.stderr)
            );
            assert_eq!(fs::read(ps_dest.join("eggsact")).unwrap(), b"abc");
        }
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
}
