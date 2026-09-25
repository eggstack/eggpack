# Bootstrap Installers Milestone 002 Closure — Bundle/Archive Bootstrap Safety

Status: closed

Source plan: `plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md`

Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Reviewed baseline: `ca0537b5ec6f7996ee9f0a1eaa04256752515eaa` (CI M002 handoff registration; Bootstrap M001 + Build M004 already closed).

Implementation commits: `12eb2c8`, `3afa8e1`, `df0120c`. Final implementation SHA: `df0120cfcea3bf582390807639f99d84ad080391`.

Hosted CI: [run 36142013486](https://github.com/eggstack/eggpack/actions/runs/36142013486), success on all lanes (Linux stable, Linux Rust 1.89, macOS, Windows). Two prior runs (`36140450329`, `36141499463`) exposed host-mismatch and Windows PowerShell 5.1 `Get-FileHash` failures; both were corrected in `3afa8e1` (host-aware fixtures) and `df0120c` (pwsh 7 runtime) before the passing run.

## Executive finding

`eggpack-bootstrap` now generates deterministic transactional first-install POSIX and PowerShell scripts for direct, bundle, and archive releases. Bundle installers verify every member before any placement and roll back only invocation-created paths. Archive installers support tar+gzip only, verify archive bytes, check exact member inventory before extraction, validate extracted regular files by size and SHA-256, flatten nested sources to contract install names, and place atomically under explicit caller-owned executable/data modes. Direct M001 behavior is preserved via the original renderers. No overwrite, update, receipt, service, elevation, or discovery authority is introduced.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Strict BootstrapInstallPolicyV1 | Versioned policy with per-target install-name to mode maps and TarGzip-only archive encoding; unknown fields, versions, counts, and character violations reject; validated against contract/manifest pairwise. |
| Projection model | Direct/Bundle/Archive internal forms validate contract/manifest pairwise including all relationships; manifest members are matched by exact source, never basename alone; target sets, runtime mappings, and policy coverage are checked before emitting text. |
| Bundle download/verification | Per-platform unrolled downloads use fixed origin plus manifest filenames, `curl` with connect/total bounds and no redirect following, exact size then SHA-256 checks for every member before placement. |
| Bundle transactional placement | All destinations checked absent before and after verification; modes applied in temp; same-filesystem hard-link (POSIX) or `File.Move` (PowerShell) without overwrite; unrolled `rm -f`/created-list rollback removes only invocation-created files. |
| Archive verification | Archive size/hash verified; `tar -tzf` listing checked for empty/absolute/backslash/colon/`..` names, sorted and compared exactly to contract sources; missing/extras reject before extraction into private temp. |
| Archive tool boundary | POSIX requires `tar`; PowerShell requires `tar.exe`; both fail with bounded messages before placement; no downloaded extractor or caller-supplied command. |
| Extracted member validation | `! -L` plus `-f`/`PathType Leaf` checks reject symlinks/directories/devices; exact size and SHA-256 verified against manifest; source mapped to install name only after verification; modes applied explicitly. |
| Install modes | POSIX Executable 0755 / Data 0644 from policy; PowerShell preserves bytes with inherited ACL while still requiring and validating policy; no mode guessing, chown, or ACL synthesis. |
| Existing-install safety | First-install only; pre-existing files fail closed before and during placement; races fail on atomic creation; losers roll back only their own paths. |
| Origin/download policy | HTTPS production origin, loopback-only fixture mode, exact URLs, no redirects, timeouts, no credentials, no latest/version API; applied independently per bundle member. |
| Determinism | Same contract + manifest + spec + policy renders byte-identical scripts; no timestamps or random IDs in text (runtime temp names only). |

## Production implementation evidence

- Added `InstallMode`, `BundleArchiveEncoding::TarGzip`, `TargetInstallPolicy`, and `BootstrapInstallPolicyV1` with TOML/JSON parsing, shape validation, and pairwise `project_all` validation.
- Added `render_posix_with_policy` and `render_powershell_with_policy` with per-platform unrolled branches for direct/bundle/archive, preserving M001 `render_posix`/`render_powershell` byte-identical output for direct-only releases.
- `eggpack-bootstrap` depends only on contract, manifest, serde, serde_json, and toml; no network, process, GitHub, or Eggup dependencies. `cargo tree -p eggpack-bootstrap` confirms no external backend.
- Docs updated in root README, crate README, and plan-required closure/roadmap/registry (see below).

## Verification executed

Passed locally and on hosted run 36142013486:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-bootstrap --all-targets --all-features --locked  (7 passed)
cargo test --workspace --all-targets --all-features --locked  (128 passed, 4 ignored)
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-bootstrap --locked
cargo package -p eggpack-bootstrap --locked --allow-dirty [with local path patches]
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Runtime fixture coverage executed POSIX bundle/archive paths on Unix (Linux and macOS lanes) and PowerShell bundle paths via pwsh 7 on Linux, macOS, and Windows lanes. Shell `sh -n` succeeds for all generated POSIX fixtures; PowerShell parser succeeds via `powershell.exe` (Windows 5.1) and `pwsh` (all lanes). ShellCheck and PSScriptAnalyzer were not installed locally and were not required; no sudo/elevation/update/latest/version/redirect-following/arbitrary-extraction strings appear in generated text. Fault-injected `ln` failure proves rollback removes partial placement; missing-tar test proves bounded `tar is required` failure before placement with no installed files.

## Invariant, failure, compatibility, and security review

Generation is pure with no external effects. All target/artifact/digest/install data derives from validated inputs. Path traversal, symlink/hardlink, extra-member, destination-race, rollback-scope, mode-correctness, checksum-fallback, temp-permission, redirect, and secret/elevation behaviors were explicitly audited per plan section 10. SHA-256 remains integrity, not authenticity. No unresolved medium-or-higher extraction/placement finding remains.

DistributionContract v1 and ReleaseManifest v1 are unchanged. Direct M001 scripts remain supported via original functions. `BootstrapInstallPolicyV1` is generator input, not a portable release identity schema. No Eggup dependency. No persistent lockfile was introduced; atomic no-overwrite placement plus rollback preserves safety per plan.

## Documentation, roadmap, and dependency transitions

Root and crate READMEs document first-install-only, all-or-nothing bundle/archive, caller-owned modes, tar-gzip-only, no-overwrite, integrity vs authenticity, required tar tooling, and Eggup/application-owned updates. Bootstrap M002 is closed.

| Milestone | Status after M002 | Reason |
|---|---|---|
| Bootstrap M001 | closed | Prior closure remains valid; direct behavior preserved. |
| Bootstrap M002 | closed | Local and hosted acceptance evidence above; run 36142013486. |
| Bootstrap M003 two-consumer adoption | blocked | Still requires real adoption evidence and candidate review. M002 closure alone does not authorize M003 and no M003 plan is authored here. |
| CI M002 | closed separately | See `plans/closure/ci-release-orchestration/002-status.md`; same implementation SHA and hosted run. |
| Ecosystem adoption | blocked | Still requires core pieces plus real-consumer selection; M002 completion is necessary but not sufficient. |

Bootstrap M003 is therefore **not ready to plan**: the bundle/archive safety precondition is now satisfied, but real two-consumer adoption evidence and receipt-handoff review are still outstanding.
