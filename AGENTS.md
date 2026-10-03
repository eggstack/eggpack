# AGENTS.md — eggpack

Producer-side release construction/distribution for Eggstack. Does NOT replace Eggup (consumer-side install/update/rollback).

## Workspace

- Cargo workspace, resolver 2, 7 members: `eggpack-{contract,manifest,core,bootstrap,ci,github,cli}`. Edition 2021, `rust-version = 1.89`, MIT.
- `#![forbid(unsafe_code)]` workspace-wide (`unsafe_code = deny`); clippy `all = warn`, but verification denies warnings.
- Leaf crates are sync, side-effect-free: `contract` (layout authority + validators), `manifest` (JSON parser/serializer, no I/O). `core` owns planning/build/qualify/finalize. `bootstrap` renders installers. `ci` renders workflows. `github` stages drafts. `cli` is deterministic wiring only (hand-rolled args, no `clap`).

## Verify (trust these over docs)

- Full local gate: `scripts/check-local.sh` — `fmt --check` → `check` → `clippy -D warnings` → per-crate tests → workspace tests → `doc` → `tree` → `package` → MSRV lane (`cargo +1.89.0 check/test`).
- Fast loop: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test -p <crate> --all-targets --all-features --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- Single test: `cargo test -p <crate> --all-targets --all-features --locked <filter> -- --nocapture` (e.g. `-p eggpack-core real_local_cargo_fixture_builds_a_direct_candidate`).
- Windows-only serialization when diagnosing: append `-- --test-threads=1` to core tests.
- `cargo package` for non-leaf crates requires `--config 'patch.crates-io.<dep>.path="crates/<dep>"'` flags — copy the exact invocation from `scripts/check-local.sh`. Never bare `cargo publish`.
- CI (`.github/workflows/ci.yml`, `contents: read`): `linux` job = ubuntu × stable/1.89.0 (fmt only on stable, clippy/doc only on stable); `portability` = macOS/Windows stable with focused Windows builder, process-group, bootstrap-archive, and orchestration tests.

## Invariants (do not violate)

- Authority separation: Contract owns layout/names; PackConfig references them but never redefines; ReleasePlan = intent (not evidence); ReleaseManifest = final-bytes evidence only; Eggup receipts = installed state.
- Determinism: canonical sort, stable serialization, sorted inventories, byte-identical renders. Preserve it in any change.
- Fail-closed validation: `deny_unknown_fields`, `schema_version == 1`, bounded counts/sizes, exact-match resolution, `AllowExtras` default with opt-in `Exact`.
- Integrity ≠ authenticity: SHA-256 + sizes are integrity facts only. No signature/provenance claims.
- Staging is draft-only on the exact existing tag. No `--clobber`, no tag mutation, no auto-publish, no immutable overwrite. Publication is a separate human action.
- `Qualification::Native` is host-matched proof, independent of builder (`NativeCargo` or `CargoZigbuild`); mismatch records `HostMismatch` fail, never pass/skip. Native run does NOT prove a declared glibc/macOS floor.
- Archive output: `TarGzip` only, filename must end `.tar.gz`, deterministic timestamps/ownership.
- Finalizer writes only into a new output root under a caller-secured parent; `release-manifest.json` goes beside (never inside) the finalized root.

## Quirks

- Process execution: bounded time/output, `command-group` process groups/jobs, cleared env except small toolchain/path allowlist, never return captured output contents.
- Windows Cargo builds require initialized MSVC env (`ilammy/msvc-dev-cmd`, `link.exe` under `VCToolsInstallDir`); bootstrap archive tests need pwsh 7 + `tar.exe`.
- `eggpack ci generate` creates/atomically replaces only the explicit `--output` path, rejects symlinks. `eggpack ci check` never writes; CRLF→LF-only drift comparison.
- Internal `ci _*` runner commands are file-in/file-out wrappers, not a scripting interface; no shell, no arbitrary commands.

## Docs index

- No `.skills/` or `opencode.json` in this repo — don't invent any. User-facing guides live in `docs/` (verified commands only); internals live in `architecture/`.
- `architecture/overview.md` — module map, producer pipeline, key invariants; pins its verified commit (re-verify line counts against workspace when touching it).
- `architecture/{contract,manifest,core,bootstrap,ci,github,cli}.md` — per-crate deep dives with type/function line refs. Read `contract.md` + `manifest.md` first, then `core.md`.
- `architecture/principles-roadmap.md` — domain model, phases, ADRs 0001–0005, tooling, Eggup boundary, `dist` evaluation (disposition C: prior art only).
- `architecture/eggup-manifest-consumer-v1.md` — Eggup consumer mapping; interface note, not a wire format.
- `plans/registry.md` — control surface (status, blockers, execution order, next handoff). Canonical direction: `plans/000/001/002/003`.
- `docs/quickstart.md` — verified CLI walkthrough (resolve → generate → check). Every command/output there was run verbatim; keep them exact when editing.

## Planning conventions

- Closure discipline: `plans/closure/<subsystem>/NNN-status.md`. Compilation ≠ closure. History never rewritten — correctives are new `NNNa` plans; superseded material goes to `plans/archive/`.
- Next handoff (per registry): none pending. Release Manifest M003 is closed — `eggpack-manifest 0.1.0` is published to crates.io from `8d661e4` (checksum `2a08f24b…b629`, tag `eggpack-manifest-v0.1.0`), and the published bytes are identical to the consumer-qualified source. Eggup consumed it and closed its own M004 registry promotion (`eggstack/eggup@ea1f1c5`, status cleanup `3b82d53`); M003a recorded that receipt and closed. No Eggpack-owned prerequisite remains outstanding for Eggup; new Eggpack work needs new evidence and a new plan.
