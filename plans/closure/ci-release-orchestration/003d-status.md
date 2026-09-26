# CI and Release Orchestration Milestone 003d Closure — Consumer Release Composition Seam

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003d-consumer-release-composition-seam.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: plan baseline `8a451ba8f98cd058d861b3283af92c6412deec15`; implementation parent `5b2cad7` was the clean repository state reviewed before changes.

Implementation commits: `946dd7a` (feat: implement CI M003d consumer composition seam) and `05005bf` (test-only: gate the Windows-incompatible empty-PATH missing-interpreter simulation to non-Windows lanes; no production code changed).

First proving consumer baseline (unchanged): `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`.

## Executive finding

M003d is closed. Eggpack now exposes the three narrow composition seams required by the first real consumer without absorbing product-owned release/install semantics:

1. reusable checked-in workflow shape (`ReleaseWorkflowShapeV1`) with invocation-local runtime identity resolution (`eggpack ci _resolve-release` plus the `resolve` preflight job and preflight artifact);
2. staging-owned installer presentation (`InstallerPresentationV1`: `GeneratedDefault` preserves M003a bytes exactly; `ProductWrappers` stages product-owned public `install.sh`/`install.ps1` alongside generated exact installers);
3. one bounded post-core-qualification consumer validator (`ConsumerValidatorV1`, finite `Python3` interpreter, fixed argv, identity-linked evidence, required/optional gate semantics).

No generic command DSL, arbitrary staging surface, plugin/hook system, product-specific logic, publication authority, or tag mutation was added. No real eggsact draft was created or required. The remaining first-consumer prerequisite is Build M005 deterministic cross-tool provisioning; Ecosystem M001 then performs the outstanding M003b live-draft proof.

## Requirement-to-evidence matrix

| Requirement | Evidence and result |
|---|---|
| Static shape carries no future identity | `ReleaseWorkflowShapeV1` has no release_id/source_revision/tag/digest field; unknown identity fields reject (`m003d_shape_validation`); reusable rendering asserts the unresolved placeholders never leak into bytes (`m003d_reusable_render_is_tag_independent`). |
| Runtime preflight resolves exact identity | `_resolve-release` validates the exact tag, verifies HEAD equals the given revision via `git rev-parse`, resolves `PackConfig::resolve` with the exact tag as opaque release_id, resolves the static template for the same tag, and writes ReleasePlan/ReleaseCIPlan/GitHubDraftPolicy only to invocation-private storage (`resolve_release_emits_distinct_runtime_identity_per_tag`). |
| Same bytes serve distinct tags | Identical static files render byte-identical workflows; `v1.2.4`/`v1.2.5` resolve distinct plans/policies (`m003d_reusable_render_is_tag_independent`, `m003d_runtime_resolve_matrix`, CLI two-tag test). |
| M003c source verification intact | Every release job downloads the preflight artifact before `_verify-source` against the runtime plan; runtime source mismatch fails (`resolve_release_emits_distinct_runtime_identity_per_tag` bad-revision case; `source_verifier_accepts_tag_commit_and_rejects_other_checkout` still green). |
| Product wrappers coexist with exact installers | Four installer assets with exact names/paths/sizes/digests/media/kinds; wrapper bytes copied exactly and equal source bytes; generated `install-exact.*` bytes equal the GeneratedDefault projection (`product_wrappers_stage_four_installers_with_exact_bytes`, CLI `prepare_stage_with_product_wrappers_materializes_four_installers`). |
| Wrapper source rules | Symlink/traversal/absolute/oversize/empty/missing sources reject; source root must be a real non-symlink directory; generated-name collisions (reserved names, each other, contract artifacts) reject (`wrapper_sources_reject_symlink_traversal_size_and_collision`). |
| Exact-set reconciliation covers wrappers | Fixture adapter stages and reuses the full wrapper-inclusive set; rerun reuses without clobber (`wrapper_payload_reconciles_exact_remote_set`). |
| Bounded Python3 validator | Finite interpreter enum; host mapping `python3`/`python` with `--version` preflight identifying Python 3; hermetic PATH lookup test with fixed-argv recording (`m003d_python_mapping_and_hermetic_lookup`). |
| Validator process matrix | Success, non-zero, timeout, output-limit, missing interpreter, candidate mismatch, script symlink, candidate symlink, cancellation — each yields bounded evidence without output contents (`m003d_consumer_execution_matrix`). |
| Graph semantics | Validate-after-qualify jobs, gate waits for validators, required failure blocks, optional failure suppresses, identity mismatch is invalid, stray evidence without a validator fails closed (`m003d_consumer_gate_matrix`, `m003d_consumer_render_gates_and_detects_drift`, CLI end-to-end). |
| Local harness parity | Renderer-derived `ValidateConsumer`/`ResolveRelease` argv drive the real CLI entry points (`validate_consumer_executes_typed_command_end_to_end`, `m003d_runner_commands_cover_consumer_and_resolve`). |
| Drift detection | `ci check` (both modes) catches validator removal, staging-intent removal, tag-source switch, and one-byte edits (`m003d_reusable_check_detects_shape_drift`, render drift test, CLI shape drift test). |
| Compatibility | Extension-disabled workflows render byte-identical to M003c goldens (all `m002-*`/`m003b-*` goldens green); `GeneratedDefault` payload equals the legacy payload byte-for-byte (`generated_default_preserves_exact_m003a_behavior`); direct/bundle/archive payload and orchestration fixtures green. |

## Production implementation evidence

- `crates/eggpack-github/src/lib.rs`: `InstallerPresentationV1` (+`ProductWrappers` sources, 1 MiB bound, exact byte copy), `ProductPosixWrapper`/`ProductPowershellWrapper` staging kinds, `prepare_staging_payload_with_presentation` (GeneratedDefault delegates with identical behavior), `GitHubDraftTemplateV1` with prefix-only `resolve(tag)`.
- `crates/eggpack-ci/src/lib.rs`: `ConsumerValidatorV1`/`ConsumerValidationEvidenceV1`/`ConsumerValidationOutcome`, shell-free bounded `run_consumer_validator` (cleared env with PATH-only allowlist plus `SYSTEMROOT` on Windows, null stdin, timeout/output-limit/cancellation handling), `project_release_plan_with_consumer`, `evaluate_gate_with_consumer`, `aggregate_finalize_with_consumer`, `RunnerCommand::ValidateConsumer`/`ResolveRelease`, `ReleaseWorkflowShapeV1`, `resolve_runtime_release_plan`, `render_reusable_release_github`/`check_reusable_release_github` with unresolved-identity leak assertion, validator jobs plus gate/aggregate/stage wiring that is byte-identical when disabled.
- `crates/eggpack-cli/src/main.rs`: `_resolve-release` (explicit inputs, HEAD verification against `--source-root`, three private outputs), `_validate-consumer` (handoff plus qualification-evidence identity, evidence without output contents, non-zero exit on failure), `_prepare-stage` paired `--installer-presentation`/`--source-root` flags, `--workflow-shape` generate/check mode, consumer-aware gate/aggregate with stray-evidence rejection.

## Static workflow schema and compatibility decision

`ReleaseWorkflowShapeV1` is schema version 1: canonical ordered `TargetPolicy` set, contract aliases, build/qualification bindings, consumer validator map, staging intent. It is a new additive document; historical `CIPlan` v1 / `ReleaseCIPlanV1` JSON parses and serializes unchanged (new `consumer_validators` field defaults empty and skips when empty; new optional input-path fields skip when absent). No silent reinterpretation of old documents occurs.

## Two-tag runtime identity resolution evidence

Library (`m003d_runtime_resolve_matrix`) and CLI (`resolve_release_emits_distinct_runtime_identity_per_tag`, in a disposable git repo) prove: identical static files resolve distinct ReleasePlan/GitHubDraftPolicy pairs for `v1.2.4`/`v1.2.5` with `release_id == tag`, `title == prefix + tag`, and checked-out source linkage; injection tags, non-40-hex revisions, HEAD mismatch, unknown aliases, and empty selection all fail closed with no partial outputs written.

## Proof no checked-in release-specific source SHA/tag is required

Reusable goldens contain the `resolve` job, `eggpack-runtime-identity` artifact flow, and runtime `eggpack-runtime/*.json` references only. Assertions prove the checked-in release-plan/CI-plan/draft-policy paths and both unresolved placeholders are absent. `GitHubDraftTemplateV1` has no tag field (unknown-field input rejects); the shape has no identity field (unknown-field input rejects).

## Installer-presentation schema/compatibility decision

`InstallerPresentationV1` is schema version 1 with `GeneratedDefault` / `ProductWrappers` modes; TOML and JSON checked-in forms both parse. The staging payload schema stays v1 with two additive kinds. Compatibility rule (documented in `crates/eggpack-github/README.md`): default payloads use exactly the M003a kind set and parse on existing readers; wrapper payloads fail closed on readers without the new kinds. Tested by `generated_default_preserves_exact_m003a_behavior` (payload JSON plus staged bytes identical).

## Eggsact wrapper fixture evidence

Planned eggsact configuration (`packaging/install.sh` -> `install.sh`, `packaging/install.ps1` -> `install.ps1`, generated `install-exact.sh`/`install-exact.ps1`) is exercised with representative wrapper bytes in both the github and CLI suites. No eggsact repository change was made; that is Ecosystem M001 work.

## Generated exact installer names and digests

With product wrappers enabled, generated exact installers are byte-identical to the GeneratedDefault projection for the same manifest/policy (same exact-tag origin, SHA-256/size equality asserted file-by-file). No `latest/download` origin is possible (existing exact-origin guard applies to both modes).

## Validator process/evidence matrix

| Case | Outcome |
|---|---|
| exit 0 within bounds | `Passed` |
| non-zero exit | `Failed(NonZeroExit)` |
| timeout | `Failed(Timeout)` (prompt output-limit/cancel detection included) |
| stdout/stderr over limit | `Failed(OutputLimit)` |
| missing Python / preflight reject | `Failed(InterpreterUnavailable)` |
| size/SHA/type mismatch | `Failed(CandidateMismatch)` |
| script missing/symlink/oversize | `Failed(ScriptUnavailable)` |
| caller cancellation | `Failed(Cancelled)` |
| caller misuse (bad config, relative paths, bad digest) | `CiError` before execution |

Evidence JSON never contains `stdout`/`stderr`/output keys (asserted on serialization in both library and CLI).

## Exact candidate identity linkage

The validator selector must match a handoff output and a qualified candidate (package/binary/selector); expected size/SHA come from qualification evidence, observed size/SHA are recorded in consumer evidence, and gate comparison requires release/source/target/selector/interpreter equality.

## Required/optional gate behavior

Required validator failure (or missing consumer evidence) yields `FailedRequiredGate`; non-gating validator failure yields `SuppressedNonGatingIncomplete` with no partial release (`aggregate_finalize_with_consumer` returns no finalized output unless `Complete`). Consumer evidence for a target without a validator, duplicate targets, and selector/release swaps yield `InvalidEvidence`. Stray `consumer-evidence.json` files with a validator-free graph fail the CLI gate/aggregate before evaluation.

## Direct/bundle/archive regression results

Existing M003a direct/bundle/archive payload fixtures, M003b direct/bundle/archive staging goldens, M002a direct/bundle/archive/mixed goldens, and the direct/bundle/archive local orchestration matrices all pass unchanged. M003c tag-source/source-verifier tests pass unchanged.

## M003c invariant review

- Exact checked-out source is verified before every release job (now against the runtime plan in reusable mode).
- RefName/DispatchInput event separation and stage conditions are unchanged; reusable `resolve` carries the same finite tag-event guard as `stage`.
- Only `stage` has `contents: write` (asserted on exact and reusable renders, with and without validators).
- No OIDC, publication, tag mutation, or broad release CLI authority was added (static guards apply to both renderers).
- Upload streaming, pagination, origin, and query-encoding behavior is untouched.

## Verification executed

All verification below ran against implementation commit `946dd7a`; follow-up `05005bf` is test-only (no production code changed) and re-greened the full local suite before push:

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-github --all-targets --all-features --locked     passed (32 tests)
cargo test -p eggpack-ci --all-targets --all-features --locked         passed (42 passed, 2 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked        passed (8 tests)
cargo test -p eggpack-bootstrap --all-targets --all-features --locked  passed
cargo test --workspace --all-targets --all-features --locked           passed (195 passed, 6 ignored)
cargo doc --workspace --no-deps --locked                               passed
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                 passed (42 passed, 2 ignored)
cargo +1.89.0 test -p eggpack-github --all-targets --locked             passed (32 tests)
./scripts/check-local.sh                                               passed (exit 0, includes packaging)
git diff --check                                                       passed
```

Hosted CI (Linux stable, Linux Rust 1.89, macos-latest, windows-latest) runs on push via `.github/workflows/ci.yml`. Implementation commit `946dd7a` triggered hosted run `36219897873`; its outcome is recorded below once observed. Windows/macOS coverage depends on that hosted run: the validator execution tests use the real platform interpreter (`python3`/`python` with preflight), and the hermetic PATH test covers fixed-argv semantics on Linux while asserting only the mapping name on Windows.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher consumer-composition finding. | M003d acceptance criteria satisfied. |
| Outstanding operational condition | No real eggsact draft was created. | M003d does not require it. Live draft qualification stays the M003b/Ecosystem M001 gate after Build M005. |
| Evidence boundary | Hosted Windows/macOS lanes exercise the new validator tests on real runners only after push. | Recorded; local Linux plus MSRV evidence is complete. |
| Platform finding (closed, test-only) | Hosted Windows run `36219897873` showed the empty-PATH missing-interpreter simulation cannot work on Windows (CreateProcess searches system dirs unconditionally, so the runner still resolved `python`). | Closed by `05005bf`: that single case is gated to non-Windows lanes with a documented rationale; the spawn/preflight failure branches are platform-independent and proven on Linux/macOS, while Windows proves the mapping, preflight, and full matrix otherwise. No production code changed. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003d | Reason |
|---|---|---|
| CI M003d | closed | This record; implementation `946dd7a`. |
| CI M003b live draft qualification | ready to resume (still blocked on Build M005) | M003d corrective dependency is closed; Build M005 remains. |
| Ecosystem adoption M001 (eggsact) | blocked / planned | M003d closed; waits only on Build M005, then performs the M003b live-draft proof. Mirrored eggsact M005 plan stays registered. |
| Build M005 | ready (unchanged) | Sole remaining first-consumer prerequisite. |
| Bootstrap M003 | blocked (unchanged) | Independent adoption evidence/candidate review remains outstanding. |
| Eggup Interoperability M003 | ready to plan (unchanged) | Independent of this seam. |

Historical closure records are preserved. Registry and both affected roadmaps now identify M003d as closed, Build M005 as the sole remaining first-consumer prerequisite, and Ecosystem M001 as blocked only on M005.
