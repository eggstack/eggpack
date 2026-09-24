# Bootstrap Installers Milestone 001 Closure — Direct Installer Generator

Status: closed

Source plan: `plans/implementation/bootstrap-installers/001-direct-installer-generator.md`

Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Reviewed baseline: `a161e4362499f1e07a4a8f0fe1e3d3a9cbf6e10b` (Build/Qualification M001 closure; direct renderer dependencies already closed).

Implementation commits: `4eba6ec9422ab01a675ef34cdfc2b9d6f9c0944d`, `fae149ebab0023e1b9fd765d6429718b612ab383`, `f1f06387157966a0288e00c04aa455daa50699e9`, `4654c19b138ab2644674438dd1172749baca73d7`, `f57ec4bebd36d473d59ca9a06bbb2f4a73b95a09`.

Hosted CI: [run 35955647024](https://github.com/eggstack/eggpack/actions/runs/35955647024), all Linux stable, Linux Rust 1.89, macOS, and Windows jobs passed.

## Executive finding

`eggpack-bootstrap` now validates exact contract/manifest consistency and renders deterministic release-specific POSIX and PowerShell direct first-install scripts. Both renderers use explicit fixed origins, bounded downloads, exact expected sizes and SHA-256, private temporary staging, and no-clobber placement. Unsupported or ambiguous mappings fail generation. Existing destinations are preserved. Release selection, update policy, elevation, service lifecycle, and receipts remain outside the generated installer.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Isolated renderer dependency direction | `crates/eggpack-bootstrap` depends only on contract and manifest; `cargo tree -p eggpack-bootstrap --locked`. |
| Contract/manifest identity agreement and direct-only gate | `project` checks product identity, exact canonical target set, exact expanded artifact/install pairing, and direct form; mismatch regression. |
| Fail-closed platform mapping | OS/architecture pairs are derived from canonical target triples. Duplicate runtime pairs and unsupported triples fail. |
| Fixed origin and bounded transport | HTTPS by default; explicit loopback-only fixture mode; origin rejects credential-like and quoting/query characters. `curl` and `Invoke-WebRequest` have total/connect bounds and redirects are not followed. |
| Exact integrity before installation | Local shell and PowerShell fake-release matrix covers success, wrong size, wrong digest, 404, existing destination, and empty destination after failure. Exact local byte count precedes SHA-256 verification. |
| No-clobber placement and cleanup | POSIX stages under destination then uses atomic hard-link creation, which cannot replace an existing name; PowerShell uses `File.Move` without overwrite. Temporary staging is removed on handled exit. |
| Readable deterministic output and generation-only authority | Repeat render equality; shell syntax check; PowerShell AST parse; generated scripts contain no release lookup, elevation, service, or Eggup receipt logic. |
| PowerShell execution/parse evidence | Local `pwsh` fake-release execution covers success and failure matrix. Hosted Windows executes the PowerShell parser check as part of the workspace test suite. |

## Production implementation evidence

- Added a dedicated renderer crate with no network/process runtime dependency; only the generated text performs acquisition when deliberately run.
- Rejects asset/install characters outside a bounded portable filename set so URL segments and destination names cannot be reinterpreted by the target platform.
- Production origin is HTTPS-only; test HTTP is limited to loopback addresses with an explicit fixture switch.
- Does not overwrite existing files and does not request elevation. Direct-only M001 does not generalize into a script DSL.

## Verification executed

Passed locally:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-bootstrap --locked
cargo package -p eggpack-bootstrap --locked --allow-dirty [with local path patches for unpublished workspace dependencies]
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
git diff --check
```

The workspace suite passed 60 tests. On the local Unix host, generated shell syntax and loopback execution passed; `pwsh` ran the generated PowerShell script through success, overwrite, size, digest, and 404 cases. ShellCheck and PSScriptAnalyzer were not required and were not used. Hosted CI passed on macOS and Windows as well as both Linux toolchains.

## Invariant, failure, compatibility, and security review

Generation performs no external effects. Generated scripts derive all target/artifact/digest/install data from validated inputs and never decide which release to install. Origin and path injection is constrained; redirects are disabled; timeouts are explicit. Verification failure and existing destinations leave the destination artifact absent/unchanged. Atomic no-clobber placement fails closed when the filesystem cannot create the POSIX hard link. SHA-256 is integrity evidence, not authenticity. No unresolved medium-or-higher installer safety finding remains.

No existing consumer installer was removed. ReleaseManifest v1 and DistributionContract schemas are unchanged. The generator is an optional release-specific output and does not establish runtime update policy.

## Documentation, roadmap, and dependency transitions

Root and crate READMEs now document first-install-only semantics, fixed exact-release generation, and integrity versus authenticity. Bootstrap M001 is closed. Bootstrap M002 remains blocked: the current builder reports archive and member facts but does not prove the archive contains those exact members, and no qualified consumer extraction path exists here. M003 adoption therefore remains blocked pending M002 and actual consumer evidence. Eggup Interoperability M001 remains ready and is the next registered plan in this execution sequence.
