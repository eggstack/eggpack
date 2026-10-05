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
- `architecture/` line citations rot silently: editing a cited `.rs` file invalidates them, and a blanket re-base corrupts as much as it fixes because documents are often only *partly* updated. Re-base by symbol name, never arithmetic. Current state and the outstanding gap are recorded in `architecture/overview.md` §Citation-verification state.

## Skills

Read the relevant skill before starting the matching kind of work. Each is derived
guidance that routes to a normative document; where a skill and `AGENTS.md` or
`architecture/` disagree, the deep dive wins and the skill is what gets fixed.

| Skill | Use it when |
|---|---|
| `.skills/pre-submit-gate/` | Finishing a change, before committing, or asked "is this ready to submit?" — the exact gate, the `cargo package` patch flags, and the Windows-only steps a local green run does not cover |
| `.skills/architecture-routing/` | Before reading source — which deep dive is normative, which crate owns which boundary, and whether a suspected bug is doc drift |
| `.skills/planning-and-closure/` | Opening, planning, correcting, or closing a milestone; updating the registry |
| `.skills/generated-ci-and-draft-staging/` | Touching `ci generate`/`ci check`, the `ci _*` runner commands, the handoff file names, or draft staging/publication |
| `.skills/contract-and-manifest-surface/` | Adding or changing a target, asset form, install name, or either schema-v1 document |

## Docs index

- `docs/` — user-facing guides, verified commands only. Start with `quickstart.md` (resolve → generate → check); every command and expected output there was run verbatim, so keep them exact when editing.
- `plans/registry.md` — control surface (status, blockers, execution order, next handoff). Canonical direction: `plans/000/001/002/003`.
- `architecture/overview.md` — module map, producer pipeline, key invariants, and the index into every deep dive. Pins its verified baseline; re-verify its line counts and `file:line` citations against the workspace whenever you touch a cited file.
- `architecture/eggup-manifest-consumer-v1.md` — Eggup consumer mapping; interface note, not a wire format.

There is deliberately no `opencode.json` or other agent-runtime config here.
`AGENTS.md` plus `architecture/overview.md` is the whole entry path.

### Architecture deep dives

Read the index in `overview.md` and follow one link. Grouped by what they cover:

| Area | Document |
|---|---|
| Vocabulary, authority, phases, ADRs 0001–0005, `dist` (disposition C: prior art only) | [principles-roadmap.md](architecture/principles-roadmap.md) |
| Planning process, closure discipline, registry | [planning-and-governance.md](architecture/planning-and-governance.md) |
| Layout authority, schema-v1, conformance validators | [contract.md](architecture/contract.md) |
| Final-bytes evidence, bounds, collision rules, publication status | [manifest.md](architecture/manifest.md) |
| Pipeline orientation and cross-file contracts | [core.md](architecture/core.md) |
| Planning stage, policy, `validate_policy` | [core-planning.md](architecture/core-planning.md) |
| Build stage, bindings, bounded execution, MSVC | [core-build.md](architecture/core-build.md) |
| Qualification methods, host matching, what native does not prove | [core-qualification.md](architecture/core-qualification.md) |
| Finalization, gate, archives, sidecars, manifest aggregation | [core-finalization.md](architecture/core-finalization.md) |
| Installer render API, install modes, PowerShell specifics | [bootstrap.md](architecture/bootstrap.md) |
| Job-graph projection, gates, aggregation, CLI seam | [ci.md](architecture/ci.md) |
| Renderers, caller-supplied policy, drift checking | [ci-rendering.md](architecture/ci-rendering.md) |
| Handoff formats, consumer validator, runtime identity | [ci-consumer-seam.md](architecture/ci-consumer-seam.md) |
| Staging payload, `GithubApi` seam, reconciliation, redaction | [github.md](architecture/github.md) |
| Commands, `ci _*` runners, file-safety helpers, exit discipline | [cli.md](architecture/cli.md) |
| Determinism and stable serialization, and where it is not claimed | [determinism.md](architecture/determinism.md) |
| Fail-closed validation model | [validation-model.md](architecture/validation-model.md) |
| Process execution, env allowlist, executable allowlist | [process-execution.md](architecture/process-execution.md) |
| Test topology, lane matrix, MSRV | [testing-and-portability.md](architecture/testing-and-portability.md) |

## Planning conventions

- Closure discipline: `plans/closure/<subsystem>/NNN-status.md`. Compilation ≠ closure. History never rewritten — correctives are new `NNNa` plans; superseded material goes to `plans/archive/`.
- Current handoff (per registry): three plans are `ready`. Contract M003 (`plans/implementation/contract-conformance/003-bounded-direct-contract-expansion-cli.md`) is the only production-code handoff and stays a narrow local scalar projection over existing schema-v1 semantics. Bootstrap M003 (`plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md`) is an evidence/ownership closeout with no assumed code change. Ecosystem M003a (`plans/implementation/ecosystem-adoption/003a-eggsearch-seven-target-compatibility-preflight-and-migration-design.md`) is read-only/scratch and must resolve toolchain, ARMv7 runtime, Windows ARM64, attestation-preservation, and no-clobber choices. Eggsearch M003b remains blocked until M003a closes and a mirrored Eggsearch plan is registered.
