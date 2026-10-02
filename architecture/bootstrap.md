# `eggpack-bootstrap` — Deep Dive

Pure deterministic **first-install script generator**: input
`DistributionContract + ReleaseManifest + BootstrapSpec +
BootstrapInstallPolicyV1` → output text (`sh` / `ps1`). Exact-release only, no
`latest`/discovery/mirrors; first-install only (missing destinations, no
overwrite/elevate/update). M001 direct-only; M002/M002a transactional
bundle/archive safety. All in `crates/eggpack-bootstrap/src/lib.rs`
(~3436 lines, `#![forbid(unsafe_code)]`); tests inline (`:1050-3436`).

## Key types / functions (`src/lib.rs`)

- `BootstrapSpec{origin, fixture_http}` (`:12-17`) — caller-owned fixed base
  URL, no trailing `/`.
- `BootstrapError(String)` + `fail()` (`:19-29`) — bounded path-free diagnostics.
- M001 projection: `Item{os, arch, name, install, size, sha}` (`:30-37`),
  `project(c,m,s)` (`:38-112`, manifest order preserved),
  `safe_segment()` (`:113-117`),
  `runtime_pair()` (`:118-138`), `shq()` (`:189-191`), `psq()` (`:239-241`).
- M001 renderers: `render_posix` (`:139-188`), `render_powershell` (`:192-238`)
  — kept for direct regression.
- M002 policy: `InstallMode::{Executable, Data}` (`:247-255`),
  `BundleArchiveEncoding::{TarGzip}` (`:257-263`),
  `TargetInstallPolicy{modes, archive_encoding}` (`:265-275`),
  `BootstrapInstallPolicyV1{schema_version:1, targets}` (`:277-286`) +
  `empty()`, `from_toml()`, `from_json()`, `validate_shape()` with
  `MAX_POLICY_JSON = 256KiB`, targets/entries ≤256. Shape rules: target key
  1–128 chars, mode keys pass `safe_segment`, `deny_unknown_fields`.
- M002 projection: `BundleEntryProjection` (`:357-364`),
  `ArchiveMemberProjection` (`:366-373`),
  `TargetProjection::{Direct,Bundle,Archive}` (`:375-398`),
  `project_all(c,m,s,policy)` (`:400-622`, sorted by `(os,arch)`),
  `posix_mode()` (`:624-629`),
  `validate_origin()` (`:340-355`).
- M002 renderers: `render_posix_with_policy()` (`:656-860`),
  `render_powershell_with_policy()` (`:871-1048`) + verify/download blocks
  (`POSIX_SHA_SNIPPET`, `posix_verify/download_block`, `ps_verify_block`).

## Shell vs PowerShell × direct/bundle/archive

**Direct:** POSIX maps `uname -s/m` → `linux|macos|windows : x86_64|aarch64|
armv7`, double absent-check, `mktemp -d` + trap, `curl --fail --silent
--show-error` (no `-L`), `wc -c` size, `sha256sum|shasum|openssl` digest,
`chmod 755` + `ln`. PowerShell: `$IsMacOS/$env:OS/RuntimeInformation` mapping,
`Test-Path`, `Invoke-WebRequest -MaximumRedirection 0`, `Get-FileHash`,
`[IO.File]::Move`, `finally` cleanup.

**Bundle** (`_with_policy` only, no `archive_encoding`): pre-check all
destinations absent; download + verify each member; re-check absent;
POSIX `chmod 755|644` per `InstallMode` (PowerShell inherits ACL, no `chmod`);
sequential link/move with rollback (`rm -f` priors / `$created` list).

**Archive** (TarGzip only, `.tar.gz` suffix): require `tar`/`tar.exe`;
download + verify outer bytes; write expected inventory; `tar -tzf` listing;
reject empty/absolute/traversal/backslash/colon; `sort+cmp` exact inventory;
extract to temp dir (never dest); regular-file check (`[ ! -L ] + [ -f ]` /
`LinkType` + `PathType Leaf` — device/dir rejected implicitly); per-member
size/SHA + mode; re-check dest absent; link/move with rollback. Nested `source`
(e.g. `bin/host-helper`) flattens to `install` (e.g. `host-helper`); `bin/` is
never created in dest. Windows preserves bytes with inherited ACL; policy still
validated for cross-renderer explicitness.

Direct-with-policy rule: if a policy is present for a direct target, exactly
one mode (`install -> Executable`) and no encoding.

## Safety / integrity

- Generation pure + deterministic: same inputs → byte-identical output
  (`project_all` sorted `os,arch`; legacy `project` preserves manifest order;
  runtime tmp via `mktemp`/GUID only).
- Origin: `https://` only, or loopback `http://` iff `fixture_http:true`;
  rejects newline/quote/`$\?# @`, trailing `/`; no redirects; bounded timeouts.
- Contract/manifest gate before emit: manifest validated, product match,
  canonical unique targets, `resolve+expand` equality, full target-set equality,
  unambiguous runtime pair, `safe_segment = [A-Za-z0-9._-+]+`, strict policy
  shape.
- Integrity before use: every bundle member, outer archive bytes, then every
  extracted member checked for exact size + SHA-256. SHA-256 = integrity, not
  authenticity (no signing).
- Placement: all destinations absent before + race re-check, same-filesystem
  `ln` / same-volume `Move` (tmp under dest), invocation-only rollback, never
  delete pre-existing, never replace/merge. `Executable = 0755` vs `Data = 0644`
  explicit on POSIX; PS inherits ACL. No `sudo/RunAs/latest/unzip/arbitrary
  extractor`.

## Boundaries

Never: release selection/latest, overwrite/update, Eggup receipts, service
lifecycle, privilege escalation, package-manager fallback, GitHub API, redirect
chains, signatures, arbitrary extract commands, publication, ownership inference,
non-tar.gz encodings. Normal updates/service stay Eggup/application-owned.

## Dependencies / dependents

- Deps (`Cargo.toml:14-19`): `eggpack-contract`, `eggpack-manifest`, `serde`,
  `serde_json`, `toml`. Dev-only `sha2,tar,flate2` for tests. No `eggpack-core`,
  no Eggup, no HTTP/async/build backends.
- Dependents: `eggpack-github` (staging payload exact installers),
  `eggpack-cli` (direct dep), `eggpack-ci` (dev-dep for renderer tests).
