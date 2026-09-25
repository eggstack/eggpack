# CI and Release Orchestration Milestone 002 Closure — Qualification/Aggregation Gates and Drift CLI

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/002-qualification-aggregation-gates-and-drift-cli.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `66f818a67f8681010d07301f4a1d7ca5aadf2f5b` (Build M004 closure; CI M001 + Build M003/M004 interfaces closed).

Implementation commits: `12eb2c8`, `3afa8e1`, `df0120c`. Final implementation SHA: `df0120cfcea3bf582390807639f99d84ad080391` (shared with Bootstrap M002; bootstrap host-aware fixes are included in the same SHA).

Hosted CI: [run 36142013486](https://github.com/eggstack/eggpack/actions/runs/36142013486), success on all lanes (Linux stable, Linux Rust 1.89, macOS, Windows). This run covers the `eggpack-ci` library (15 tests), `eggpack-cli` (1 test), workspace (128 passed, 4 ignored), docs, Clippy, formatting, packaging, and MSRV checks.

## Executive finding

Closed CI M001 now extends to executable release orchestration: `ReleaseCIPlanV1` layers qualification jobs, a required-evidence gate, and an aggregate/finalize node on the validated M001 graph; M003 qualification and M004 finalization are invoked through narrow file-in/file-out CLI wrappers, never copied into YAML or shell. Required targets gate on `Passed`; missing/corrupt evidence fails closed; optional failures suppress the finalized release instead of producing a partial one. Generated release workflows are deterministic, read-only, pinned to immutable actions and the official Eggpack tool revision, and upload completed finalized releases as internal workflow artifacts only. No GitHub Release, publication, signing, or staging is performed. `eggpack ci generate`/`check` provide deterministic non-mutating drift control.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Preserve M001 CIPlan | `CIPlan` schema v1, validation, `render_github`, `check_github`, and golden fixtures are unchanged; new `ReleaseCIPlanV1` layers executable state without altering M001 meaning. Historical M001 evidence remains truthful. |
| Executable graph | `ReleaseCIPlanV1` validates M001 plan plus `QualificationBindingsV1` via the M003 API, preserves canonical ordering/hosts/tiers, assigns deterministic build/evidence handoff names, and carries explicit finalization settings. |
| Qualification projection | One job per target with canonical IDs, build dependencies, handoff names, classification/host/support/required state; validated through `QualificationBindingsV1::validate_for`, not copied logic; execution delegates to `eggpack_core::qualify_target` via CLI. |
| Build handoff | `BuildHandoffV1` carries schema/release/source/target/strategy plus selector/package/binary/relative-path/size/identity entries; rejects absolute/traversal/colon/null/`..`/empty segments, duplicates, and swaps; `project_build_handoff` derives relative names deterministically; `reconstruct_attempt` revalidates inventory, containment, regular non-symlink non-empty status, and sizes before returning an in-memory `BuildAttempt`. |
| Evidence handoff | `encode/decode_qualification_evidence` transport M003 JSON with size bounds; aggregate revalidates candidate bytes against evidence after download; transport is never trusted as proof. |
| Required gate | `evaluate_gate` enforces Required→Passed only (Deferred/Failed/missing/identity-mismatch fail), reads structured evidence never logs, and returns `Complete`/`SuppressedNonGatingIncomplete`/`FailedRequiredGate`/`InvalidEvidence` without dropping targets. |
| Non-gating suppression | Optional/experimental missing/failed/unqualified evidence yields suppression with no finalized artifact; later staging may proceed only from `Complete`. |
| Aggregate/finalize | `aggregate_finalize` gates via `evaluate_gate`, maps wrapper encoding to `ArchiveEncoding::TarGzip`, calls `eggpack_core::finalize_release` (not a copy), and returns outcome plus finalized release only on `Complete`. Direct/bundle/archive complete paths produce internal artifacts; tampered/mixed/extra-slot inputs reject. |
| Pinned tool policy | `EggpackToolPolicy` allows only `https://github.com/eggstack/eggpack` at exact 40-hex revision with `eggpack-cli` package and 1–60 min timeout; no arbitrary URL/package or curl-pipe-shell; installed with `cargo install --git --rev --locked -p` plus `eggpack --version` verification. |
| Generated graph | `render_release_github` reuses M001 build prefix then appends qualify→gate→aggregate jobs with read-only permissions, exact dependencies, deterministic handoff/final names, bounded retention, and pinned tool steps; `always`-style collection is not used to bypass required failures. |
| Permissions/artifacts | Contents read only, no id-token/write/release API; candidate/evidence/final artifacts use deterministic names and bounded retention; no credentials, absolute paths, or raw output uploaded. |
| Minimal CLI | New `eggpack-cli` crate with `eggpack` binary depends only on workspace crates plus serde_json/toml; no GitHub client or async runtime. `ci generate`/`check` plus internal `_capture-build`/`_qualify-target`/`_evaluate-gate`/`_aggregate` are deterministic file-in/file-out wrappers, not a scripting DSL. |
| Generate/check | Generate creates/replaces only the explicit workflow path, rejects symlinks, writes temp then atomically replaces, no network/discovery, bounded summary. Check renders in memory, normalizes CRLF only, exits 0 on match else nonzero with bounded diagnostic, never modifies. |
| Golden/drift/tamper tests | Goldens for direct, mixed native/cross, bundle, and archive release workflows parse with independent YAML; drift tests prove one-byte edits fail; tamper/missing-evidence/gating matrices prove closed gates and no partial releases. |

## Production implementation evidence

- `crates/eggpack-ci` adds handoff schemas, executable graph, gate/aggregate adapter, tool policy (optional `download_artifact` and `eggpack_tool` fields preserve M001 JSON compatibility), release renderer/drift checker, and 7 new tests (15 total with M001).
- `crates/eggpack-cli` adds the `eggpack` binary with manual bounded argument parsing, atomic output handling, and 1 integration test covering generate-equals-library, check pass/CRLF/drift/never-modifies/symlink-reject/bounded-diagnostic behavior.
- Golden fixtures: `m002-direct.yml`, `m002-mixed.yml`, `m002-bundle.yml`, `m002-archive.yml` under `crates/eggpack-ci/tests/fixtures/`.
- `scripts/check-local.sh`, workspace membership, and `Cargo.lock` include the new crate. Root, `eggpack-ci`, and `eggpack-cli` READMEs document internal-artifact-only, suppression, staging separation, pinning, and non-mutating check behavior.
- `cargo tree -p eggpack-ci/eggpack-cli` confirms no GitHub API client, async runtime, or external backend; only workspace crates plus serialization/hash/TOML support already used in the workspace.

## Verification executed

- `./scripts/check-local.sh` — passed (exit 0), covering fmt, workspace check, strict Clippy, contract/CI/CLI/workspace tests, docs, tree/package checks, and Rust 1.89 workspace check plus contract/manifest/core/bootstrap/CI/CLI tests.
- Focused results: `cargo test -p eggpack-ci` 15 passed; `cargo test -p eggpack-cli` 1 passed; `cargo test --workspace` 128 passed, 4 ignored (pre-existing ignores in core).
- `cargo package -p eggpack-ci` and `-p eggpack-cli` with local path patches — passed.
- `cargo +1.89.0 check/test` for workspace and affected crates — passed locally and on hosted 1.89 lane.
- `git diff --check` — passed.
- Hosted run 36142013486 — Linux stable, Linux 1.89, macOS, Windows all passed.

## Invariant, failure/recovery, and security review

Build success never implies qualification; qualification status comes only from M003 evidence; final bytes/manifests come only from M004. Required non-Passed prevents aggregation; missing/corrupt evidence prevents aggregation; optional incompleteness suppresses output instead of partial release; aggregation never drops selected targets; handoffs bind release/source/target/selector identity; permissions stay read-only; actions stay full-SHA pinned; Eggpack tooling stays pinned; no generic DSL, mutable reusable workflow, or repository discovery. `ci check` never rewrites. M004 revalidates final bytes after transport. Action-SHA, official-repo-plus-revision, command/YAML injection, quoting, traversal/symlink, name-collision, evidence-swap, candidate-tamper, gate-bypass, permission, and leakage vectors were audited with no unresolved medium-or-higher finding.

## Compatibility/migration review

M001 `CIPlan` v1 and `render_github`/`check_github` are unchanged; existing policy JSON without `download_artifact`/`eggpack_tool` still parses (optional fields) and renders build-only workflows as before. New `ReleaseCIPlanV1`, handoff, and CLI formats are additive internal orchestration types, not a stable public contract. No DistributionContract, ReleaseManifest, M003, or M004 semantics changed. No consumer migration is required or claimed.

## Documentation/operations evidence

Root README, `crates/eggpack-ci/README.md`, and new `crates/eggpack-cli/README.md` describe executable graph, handoffs, gating/suppression, internal-artifact-only, pinning, and generate/check behavior. Goldens and drift tests are checked in. `scripts/check-local.sh` covers the new crate.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher orchestration/supply-chain finding. | M002 acceptance criteria pass. |
| Informational | Runner images are policy-declared (including preinstalled cargo-zigbuild/Zig) and not validated by the generator; actual hosted images are exercised only when release workflows run. | Tool capability remains explicit policy plus preflight intent; execution belongs to build/qualification and future runner operations. |
| Environmental | ShellCheck/PSScriptAnalyzer were not installed where noted; PowerShell runtime uses pwsh 7 (Windows `powershell.exe` 5.1 parses but lacks reliable `Get-FileHash` in CI, so runtime prefers `pwsh`). | Documented here; static parse plus pwsh runtime on all lanes provides closure evidence without those linters. |

## Roadmap disposition and dependency transitions

CI M002 is closed. CI M001 remains closed and truthful. Build M003/M004 remain closed.

| Milestone | Status after M002 | Reason |
|---|---|---|
| CI M001 | closed | Unchanged; build-only evidence still valid. |
| CI M002 | closed | Local and hosted acceptance evidence above; run 36142013486. |
| CI M003 draft release staging | blocked | Still requires CI M002 (now satisfied) **plus** a separately reviewed staging adapter plan, which does not yet exist. M002 closure alone does not authorize staging implementation. |
| Bootstrap M002 | closed separately | Same SHA/run; see `plans/closure/bootstrap-installers/002-status.md`. |
| Ecosystem adoption | blocked | Still requires consumer selection plus staging/publication maturity. |

CI M003 is therefore **not ready to plan**: the gating/aggregation precondition is satisfied, but the explicit staging adapter plan remains outstanding and must be reviewed before any M003 planning begins.
