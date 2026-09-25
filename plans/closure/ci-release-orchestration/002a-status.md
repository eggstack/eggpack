# CI and Release Orchestration Milestone 002a Closure — Generated Workflow Execution Wiring

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/002a-generated-workflow-execution-wiring-corrective.md`

Coordinating closeout pass: `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Historical M002 closure: `plans/closure/ci-release-orchestration/002-status.md`

Reviewed baseline: `4d2270afe7de10bdff92563ed0f51a41ba807a04` (M002a corrective implementation; no production-code commit supersedes it — all later commits to `abd125140c763970f0658610b419b248cf41e91f` are planning-only).

Implementation commits: `4d2270a` (`feat: implement bootstrap M002a archive runtime evidence and CI M002a execution wiring`). One commit carries both M002a correctives; CI-owned paths are `.github/workflows/ci.yml` (focused steps), `crates/eggpack-ci/src/lib.rs`, `crates/eggpack-ci/tests/fixtures/m002-{direct,mixed,bundle,archive}.yml`, `crates/eggpack-ci/README.md`, `crates/eggpack-cli/src/main.rs`, and `crates/eggpack-cli/README.md`.

Implementation SHA: `4d2270afe7de10bdff92563ed0f51a41ba807a04`.

Hosted CI: [run 36154905956](https://github.com/eggstack/eggpack/actions/runs/36154905956), event `push`, attempt 1, conclusion `success`, `head_sha` exactly `4d2270afe7de10bdff92563ed0f51a41ba807a04`. All four lanes passed: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

## Executive finding

M002a is closed. The historical M002 defect — generated release workflows rendered YAML/golden-valid CI that could not execute the CLI contract it named — is corrected and proven executable. Build jobs now invoke `_capture-build` and upload the canonical per-target build directory that qualification jobs download; every generated internal CLI invocation carries all required explicit file/path arguments sourced from a bounded `GitHubReleaseInputsV1` policy; qualification artifacts carry the complete handoff/evidence/candidate layout; gate and aggregate jobs download canonical per-target qualification directories and invoke `_evaluate-gate`/`_aggregate` with explicit `--inputs-dir` plus plan/output paths; Emulated qualification without an explicit provider sysroot policy is rejected at generation time; and the finite `RunnerCommand` model shared by the GitHub renderer and the local executable orchestration tests prevents CLI/YAML drift. Direct, bundle, and archive orchestration paths execute end-to-end through M004 finalization; optional-target incompleteness suppresses the finalized release; required failures fail closed. No staging, publication, or permission change is introduced.

## Root-cause/corrective finding

Historical defect (recorded in the M002 post-closure annotation, preserved intact):

- generated M002 workflows did not create the build handoff that qualification consumed (build jobs uploaded candidate bytes only; no job produced `eggpack-build-handoff-<target>`);
- internal CLI commands were rendered without their mandatory file/path arguments (`_qualify-target` rendered only `--target`; `_evaluate-gate` and `_aggregate` rendered without required CLI inputs);
- gate/aggregate jobs did not receive the canonical per-target input layout (aggregate downloaded evidence artifacts only, while the CLI expects per-target build handoff, evidence, and candidate bytes);
- YAML/golden validity therefore did not prove operational execution.

M002a correction, as implemented at `4d2270a`:

- `GitHubReleaseInputsV1` (`crates/eggpack-ci/src/lib.rs`, repository-relative bounded paths for contract, release plan, build bindings, qualification bindings, CI plan; rejects `..`, empty segments, NUL, backslash ambiguity, drive prefixes, absolute roots; no discovery/globbing);
- finite `RunnerCommand` (`CaptureBuild` / `QualifyTarget` / `EvaluateGate` / `Aggregate`) serialized by the GitHub renderer and executed directly by local orchestration tests, with round-trip tests covering every required argument;
- canonical build handoff layout (`build-handoff.json` + `candidates/<relative-path>`, staged/validated by `stage_build_artifact_dir` / `validate_build_artifact_dir`; rejects missing/extra candidate, symlink/non-regular candidate, path escape, symlink output directory, mismatched target/binding, zero-byte candidate);
- canonical qualification layout (`build-handoff.json` + `evidence.json` + `candidates/...`, validated by `validate_qualification_artifact_dir`; bytes are the already-validated build bytes, never re-downloaded from another origin);
- explicit gate/aggregate inputs (per-target downloads into `eggpack-inputs/<target>/`; `_evaluate-gate` consumes `--ci-plan` + `--inputs-dir` + `--output`; `_aggregate` consumes contract + release-plan + CI-plan + `--inputs-dir` + output-root + summary; finalized internal artifact uploaded only on `Complete`);
- Emulated runtime policy (per-target provider sysroot required or generation rejects; no silent empty-runtime M003 invocation);
- executable orchestration tests (`m002a_generated_orchestration_executes_end_to_end` for direct/bundle/archive end-to-end, `m002a_optional_suppression_and_required_failure_close_the_gate`, `m002a_negative_matrix_for_layout_and_identity`, plus the CLI harness `generated_orchestration_cli_executes_capture_to_aggregate`).

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Build job creates canonical handoff | Build jobs install the pinned Eggpack CLI, invoke `_capture-build` with the known Cargo target root, stage `build-handoff.json + candidates/` via `stage_build_artifact_dir`, and upload the directory as one deterministic artifact. Proven by `m002a_generated_orchestration_executes_end_to_end` (build stage asserts staged handoff JSON equals projected handoff) and the regenerated goldens showing capture steps plus handoff artifact upload. |
| Qualification consumes exact build handoff | Qualification jobs download the matching per-target build artifact into a target-specific private directory, invoke `_qualify-target` with explicit contract/plan/build-bindings/qualification-bindings/target/candidate/handoff/output arguments, and reconstruct the M002 `BuildAttempt` via `reconstruct_attempt` (revalidates inventory, containment, regular non-symlink non-empty status, sizes). Covered by the e2e fixture and `m002a_runner_command_round_trip_covers_all_required_args`. |
| Every generated CLI command contains required explicit arguments | Finite `RunnerCommand::to_argv` is the single serialization used by both renderer and tests; `m002a_runner_command_round_trip_covers_all_required_args` asserts every required flag (`_capture-build`, `_qualify-target`, `_evaluate-gate` with `--ci-plan/--inputs-dir/--output`, `_aggregate` with `--inputs-dir` et al.) is present. No hidden repository discovery exists. |
| Qualification artifact includes handoff + evidence + candidates | `validate_qualification_artifact_dir` enforces the triple layout; the e2e test asserts staged evidence equals produced evidence and candidate bytes are carried through. Goldens show the complete qualification artifact upload. |
| Gate consumes structured evidence from canonical target directories | Gate job downloads every qualification artifact to `eggpack-inputs/<target>/` (one explicit download step per target) and `_evaluate-gate` loads each `evidence.json` through `decode_qualification_evidence` into `evaluate_gate`. `m002a_optional_suppression_and_required_failure_close_the_gate` proves the semantics. |
| Aggregate receives canonical target directories | Aggregate job receives the same `eggpack-inputs/<target>/` trees; the CLI reconstructs each `BuildAttempt` from its target directory and calls M004 through `aggregate_finalize`. The e2e test asserts exact final artifact bytes and manifest identity/digest after the aggregate-equivalent path. |
| Required failures fail closed | `m002a_optional_suppression_and_required_failure_close_the_gate` and `m002a_negative_matrix_for_layout_and_identity` prove required-target failure yields a failing gate and no finalized output; `m002_gate_matrix` (historical, still passing) retains Required→Passed-only semantics. |
| Optional/non-gating incomplete evidence suppresses finalized output | Same suppression test proves optional/experimental incompleteness yields `SuppressedNonGatingIncomplete` with no finalized artifact; upload steps are conditional on `Complete`. |
| Direct orchestration finalizes successfully | E2E fixture `simple-direct.toml` (eggsact 1.2.3) passes capture → qualify → gate (`Complete`) → aggregate with exact bytes/manifest. |
| Bundle orchestration finalizes successfully | E2E fixture `codegg-bundle.toml` (codegg 2.4.0) follows the same path to `Complete`. |
| Archive orchestration finalizes successfully | E2E fixture `egress-archive.toml` (egress 3.1.0) follows the same path to `Complete`. |
| Candidate/evidence identity/tamper checks remain enforced | `m002a_negative_matrix_for_layout_and_identity` (wrong target handoff, wrong release/source identity, missing/extra/symlink candidate, tampered candidate after qualification, missing artifact, layout mismatches) plus historical `m002_aggregation_finalizes_direct_bundle_archive_and_rejects_tampering` all pass. |
| Emulated qualification without provider runtime rejects | `m002a_emulated_requires_explicit_runtime_policy` proves generation rejects Emulated targets lacking an explicit sysroot policy. |
| Generated workflow remains deterministic, pinned, and read-only | `m002a_regenerate_goldens`, `m002_renderer_is_deterministic_read_only_pinned_and_gated`, `rendering_is_deterministic_parseable_read_only_and_hands_off_candidate`, and `drift_check_normalizes_only_crlf_and_detects_manual_edits` pass; permissions remain `contents: read`; actions and Eggpack tooling remain immutable-pinned. |
| Build artifact requested but never produced fails | Negative matrix covers the missing-artifact and layout-mismatch cases at the renderer/integration level. |
| Output upload gated on Complete | Regenerated goldens (`m002-direct.yml`, `m002-mixed.yml`, `m002-bundle.yml`, `m002-archive.yml`) document conditional upload on the aggregate outcome; suppression test asserts no output on non-`Complete`. |

## Production implementation evidence

- `crates/eggpack-ci/src/lib.rs` (1479 changed lines per `git show --stat 4d2270a`): `GitHubReleaseInputsV1` with bounded relative-path validation; `RunnerCommand` enum plus `to_argv` serialization; `stage/validate_build_artifact_dir`, `validate_qualification_artifact_dir`; `--inputs-dir` support in gate/aggregate paths; Emulated sysroot policy enforcement; renderer changes emitting capture steps, pinned CLI installs in build jobs, per-target downloads, and explicit gate/aggregate invocations; 6 new M002a tests (22 total in the crate with M001/M002; 21 run, 1 regenerate-goldens ignored by default).
- `crates/eggpack-cli/src/main.rs` (406 changed lines): `_capture-build` derives Cargo output locations from ReleasePlan/BuildBindings plus the known target root and stages the canonical handoff; `_qualify-target`/`_evaluate-gate`/`_aggregate` accept and require the explicit canonical arguments; CLI harness test `generated_orchestration_cli_executes_capture_to_aggregate` exercises capture → qualify → gate → aggregate through the same `RunnerCommand` argument model (2 tests total in the crate).
- Golden fixtures `m002-direct.yml`, `m002-mixed.yml`, `m002-bundle.yml`, `m002-archive.yml` regenerated to the executable shape (capture invocation, handoff upload, explicit qualify/gate/aggregate arguments, canonical inputs layout, conditional finalized upload).
- `.github/workflows/ci.yml`: Windows lane keeps the two acceptance-focused steps `Focused generated-orchestration execution (CI M002a)` and `Focused orchestration CLI harness (CI M002a)`.
- No M001 `CIPlan` semantics, M003 qualification semantics, M004 finalization semantics, contract/manifest schemas, or permission policy changed.

## Hosted evidence

Run `36154905956` (push, attempt 1, `head_sha` = `4d2270a`, conclusion `success`):

- `linux (stable)` — passed (fmt, workspace check, workspace tests, Clippy, docs).
- `linux (1.89.0)` — passed (workspace check, workspace tests).
- `portability (macos-latest)` — passed (workspace check, workspace tests).
- `portability (windows-latest)` — passed, including in order:
  1. `Verify PowerShell archive prerequisites (pwsh 7 + tar.exe)` — success (Bootstrap M002a prerequisite);
  2. `Focused PowerShell archive runtime (Bootstrap M002a)` — success;
  3. `Focused generated-orchestration execution (CI M002a)` — success (`m002a_generated_orchestration_executes_end_to_end`);
  4. `Focused orchestration CLI harness (CI M002a)` — success (`generated_orchestration_cli_executes_capture_to_aggregate`);
  5. `cargo check --workspace --all-targets --locked` and `cargo test --workspace --all-targets --all-features --locked` — success.

Windows qualification evidence is claimed from these hosted steps, not from local non-Windows execution.

## Verification executed

Local verification at closeout commit `abd1251` (planning-only delta over `4d2270a`; no production diff):

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked         21 passed, 1 ignored
cargo test -p eggpack-cli --all-targets --all-features --locked        2 passed
cargo test -p eggpack-bootstrap --all-targets --all-features --locked  8 passed
cargo test -p eggpack-ci --locked m002a_generated_orchestration_executes_end_to_end -- --nocapture  1 passed
cargo test -p eggpack-cli --locked generated_orchestration_cli_executes_capture_to_aggregate -- --nocapture  1 passed
cargo test --workspace --all-targets --all-features --locked           136 passed, 5 ignored (pre-existing ignores)
cargo doc --workspace --no-deps --locked                               passed
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                21 passed
cargo +1.89.0 test -p eggpack-cli --all-targets --locked               2 passed
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked         8 passed
./scripts/check-local.sh                                               passed (exit 0)
git diff --check                                                       passed
```

No medium-or-higher finding emerged during local verification. `git diff 4d2270a..HEAD -- crates/ .github/` is empty, so the hosted evidence at `4d2270a` still describes the production code being closed.

## Invariant review

Deterministic/reviewable generation, checked-in config/contract authority, read-only build jobs, write-permission isolation (no staging jobs exist), required-lane gating, no silent public-release replacement, and explicit publication (none performed) all hold. No repository discovery, no cross-job absolute paths, no trust in artifact transport (candidate bytes revalidated after download; identities revalidated at every boundary), no generic command DSL (`RunnerCommand` is finite to the four internal CI operations), and no M003/M004 semantic change.

## Failure/recovery review

Missing/extra/symlink/tampered candidates, wrong-target handoffs, wrong release/source identity, missing qualification artifacts, and gate/aggregate layout mismatches all fail closed with bounded diagnostics. Required-target failure blocks `Complete`; optional-target incompleteness suppresses the finalized release rather than emitting a partial one; no retry or re-run logic masks failures (hosted evidence is a first-attempt push run).

## Compatibility/migration review

M001 `CIPlan` v1 rendering is preserved; `GitHubReleaseInputsV1`, `RunnerCommand`, and the handoff directory layouts are additive internal orchestration types, not stable public contracts. No DistributionContract, ReleaseManifest, M003, or M004 semantics changed. Goldens were regenerated deliberately to the executable shape; the drift checker (`ci check`, CRLF-normalizing, non-mutating) guards them. No consumer migration is required or claimed.

## Security review

Read-only generated CI permissions retained; actions remain full-SHA pinned; Eggpack runtime tooling remains official-repository plus exact-revision pinned with `--locked` install and `--version` verification; release input paths are bounded repository-relative; candidate/evidence identity validation, outer transport revalidation, and traversal/symlink/collision guards are enforced at every boundary; no secrets, absolute paths, or raw output are uploaded; no elevation, publication, or signing authority is introduced. M002a does not stage or publish a release.

## Documentation/operations evidence

`crates/eggpack-ci/README.md` documents the end-to-end executable wiring (inputs policy, canonical layouts, `RunnerCommand` sharing, Emulated policy); `crates/eggpack-cli/README.md` documents the internal runner commands; root README executable-orchestration language is accurate and needed no change. Goldens and drift tests are checked in; `scripts/check-local.sh` covers the affected crates.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher workflow-execution finding. | M002a acceptance criteria pass. |
| Informational | Runner images remain policy-declared and are exercised only when generated release workflows run against real repositories. | Explicit policy plus preflight intent; unchanged from M002. |
| Environmental | ShellCheck/PSScriptAnalyzer availability is supplementary; local Linux lanes cannot execute the Windows-hosted qualification path, which is instead evidenced by the hosted Windows focused steps above. | Recorded here; not a product finding. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M002a | Reason |
|---|---|---|
| CI M001 | closed | Unchanged; build-only evidence still valid. |
| CI M002 | closed historically; corrective satisfied | Original library/gate/adapter evidence remains intact; the executable-wiring defect is now corrected and qualified. |
| CI M002a | closed | Implementation `4d2270a`; hosted run 36154905956 (attempt 1, all lanes green including both focused CI steps). |
| CI M003 draft release staging | blocked | M002a precondition now satisfied; still requires the separately reviewed explicit staging-adapter plan, which does not yet exist. M002a closure alone does not authorize staging implementation. |
| Bootstrap M002a | closed separately | See `plans/closure/bootstrap-installers/002a-status.md`; same SHA/run. |
| Ecosystem adoption | blocked | Still requires consumer selection plus staging/publication maturity. |

CI M003 is therefore **not ready to plan**: the executable-wiring precondition is satisfied, but the explicit staging-adapter plan remains outstanding and must be reviewed before any M003 planning begins.

## Registry updates

`plans/registry.md` reconciled: CI M002a moved to closed with this closure path; M002 annotated as historical with corrective satisfied; CI M003 blocker narrowed to the explicit staging-adapter plan; narrative, execution graph, and next-handoff paragraphs updated to the post-close state. CI roadmap `plans/subsystems/ci-release-orchestration-roadmap.md` updated to match.
