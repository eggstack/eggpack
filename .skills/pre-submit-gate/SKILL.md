---
name: pre-submit-gate
description: Run Eggpack's mandatory pre-submit verification gate before any commit or push. Use when finishing a change, before committing, or when asked "is this ready to submit?". Covers scripts/check-local.sh step by step, the cargo package patch flags that must be copied verbatim, the Windows-only steps the local gate does not run, and what a green local gate still does not prove.
---

# Pre-submit gate

The canonical gate is one script. Run it; do not hand-assemble a subset and call
it the gate.

```bash
scripts/check-local.sh
```

It is `set -euo pipefail`, so it stops at the first failure. Start it early and
read other work while it runs — the workspace test suite and the `cargo package`
round are the slow parts.

## What it actually runs, in order

| Step | Command | Notes |
|---|---|---|
| format | `cargo fmt --all -- --check` | default rustfmt; there is no `rustfmt.toml` |
| check | `cargo check --workspace --all-targets --locked` | |
| lint | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | `all = "warn"` and `unsafe_code = "deny"` are workspace lints |
| per-crate tests | `-p eggpack-contract`, `-p eggpack-github`, `-p eggpack-ci`, `-p eggpack-cli` | a fast localized lane; these four are the crates with the most Windows-hostile surface |
| workspace tests | `cargo test --workspace --all-targets --all-features --locked` | supersedes the per-crate lane; the split is for faster failure localization |
| docs | `cargo doc --workspace --no-deps --locked` | |
| trees | `cargo tree -p <each of the 7 crates> --locked` | dependency-direction eyeball |
| packages | `cargo package -p <each of the 7 crates> --locked --allow-dirty` | see the flags below |
| MSRV lane | `cargo +1.89.0 check --workspace --all-targets --locked`, then `cargo +1.89.0 test -p <each crate> --all-targets --locked` | needs the `1.89.0` toolchain installed |

Fast inner loop while you are still editing, before the full gate:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test -p <crate> --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Single test:

```bash
cargo test -p <crate> --all-targets --all-features --locked <filter> -- --nocapture
```

`--locked` is load-bearing on `check`, `clippy`, and `test`. Without it a
dependency or version drift silently rewrites `Cargo.lock` and you find out at
`--locked` time in CI, or during a release, instead of now.

## `cargo package` needs the patch flags — copy them, do not invent them

`eggpack-contract` and `eggpack-manifest` are not published to crates.io yet, so
packaging any crate that depends on them fails unless each unpublished
dependency is patched to its local path:

```bash
cargo package -p eggpack-ci --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"' \
  --config 'patch.crates-io.eggpack-core.path="crates/eggpack-core"' \
  --config 'patch.crates-io.eggpack-bootstrap.path="crates/eggpack-bootstrap"' \
  --config 'patch.crates-io.eggpack-github.path="crates/eggpack-github"'
```

The set is exactly the transitive in-workspace dependency closure of the crate
being packaged. The authoritative copy for all seven is `scripts/check-local.sh` —
read it rather than reconstructing it. **Never bare `cargo publish`**: the only
crate published from this workspace is `eggpack-manifest` 0.1.0, publication is a
deliberate milestone, and `--allow-dirty` on `package` is for local verification
only.

`--allow-dirty` is safe *here* because packaging is a local verification step and
publishing is a separate human action. Do not carry that flag into a release.

## What a green local gate does not cover

These run only in hosted CI, and their absence from a green local run is the
most common false sense of security:

| Step | Why it is hosted-only |
|---|---|
| macOS lane | `cargo test --workspace` on `macos-latest` |
| Windows lane | `cargo test --workspace` on `windows-latest` |
| MSVC linker environment verification | requires `ilammy/msvc-dev-cmd`; asserts `link.exe` resolves under `VCToolsInstallDir` |
| `real_local_cargo_fixture_builds_a_direct_candidate` | real Cargo build on Windows |
| `timeout_kills_and_waits_for_the_process_group` | process-group kill on Windows |
| Windows core tests with `-- --test-threads=1` | serialization for diagnosis only |
| `m002a_powershell_archive_runtime` | needs pwsh 7 **and** `tar.exe` |
| `m002a_generated_orchestration_executes_end_to_end` | end-to-end orchestration |
| `generated_orchestration_cli_executes_capture_to_aggregate` | CLI harness through aggregate |

A closure record that says "verified locally" for a milestone whose evidence
depends on any of these is incomplete — `plans/closure/README.md` requires
honest recording of unavailable evidence, and hosted run IDs are the evidence
that counts. See the `planning-and-closure` skill.

## Diagnosing a Windows-only failure

Append `-- --test-threads=1` to core tests so output interleaving stops hiding the
cause. The focused CI filters above exist because the full Windows lane is slow;
when triaging, run the narrow filter first and widen only if it passes.

## Reference

- [AGENTS.md](../../AGENTS.md) — invariants and fast loops
- [architecture/testing-and-portability.md](../../architecture/testing-and-portability.md)
  — test topology, lane matrix, and what each layer proves
- [architecture/planning-and-governance.md](../../architecture/planning-and-governance.md)
  — what a closure record must state about evidence
- [scripts/check-local.sh](../../scripts/check-local.sh) — the authority
- [.github/workflows/ci.yml](../../.github/workflows/ci.yml) — the hosted lanes
