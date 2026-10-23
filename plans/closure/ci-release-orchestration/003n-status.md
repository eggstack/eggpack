# CI/Release Milestone 003n Closure — Producer Implementation Consolidation Corrective

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003n-producer-implementation-consolidation-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `6a7cb4c` (CI/Release M003m), on top of CI/Release M003l (`9cf0bf8`).

Implementation SHAs:

- `bf0acfe` — `refactor(ci): consolidate the producer's duplicated implementations` (the consolidation half: one drift comparison, one staging-payload builder, and the decomposition of `render_release_github_inner`), cherry-picked as `4692bd5`;
- `028db4a` — `refactor(ci): split the CI crate into a module tree` (the module-split half), cherry-picked as `e66c7d1`.

## Executive finding

M003n is closed. The three duplicated implementations the post-implementation review found are gone, the 845-line renderer is decomposed into named private helpers, and the CI crate's single 12,000-line source file has become a module tree.

The defining constraint of the milestone was that **no output may change**. That constraint was met by construction rather than by inspection: the baselines were captured **before any edit**, and compared after, byte for byte, for every rendered workflow, every drift report, and every staging payload the crates produce. The consolidation was not allowed to be "equivalent" — it had to be **identical**.

The public API did not change. `eggpack-cli`'s and `eggpack-github`'s `eggpack_ci::` imports are byte-identical, and the crate root exposes the same items with the same names and signatures.

## Part one — consolidation (`bf0acfe`)

### One drift comparison

The CI crate had **three** check functions whose bodies were byte-identical after the render line.

| Location | Change |
|---|---|
| `crates/eggpack-ci/src/lib.rs:1860` | `fn compare_workflow_bytes(expected: &[u8], existing: &[u8]) -> Result<DriftReport, CiError>` — the one drift comparison. It takes the **already-rendered** expected bytes, and keeps, in this order and unchanged: the `MAX_WORKFLOW_BYTES` bound on the existing bytes; `normalize_newlines` applied to **both** the expected and the existing bytes (CRLF to LF only); the equality comparison; the first-difference byte offset computed over the normalized bytes, with the shorter length as the fallback; and the `DriftReport` construction with `expected_bytes`, `actual_bytes`, and `first_difference`. |
| `crates/eggpack-ci/src/lib.rs:1887` | `pub fn check_github` — unchanged signature, now renders and delegates. |
| `crates/eggpack-ci/src/lib.rs:3255` | `pub fn check_reusable_release_github` — unchanged signature, now renders and delegates. |
| `crates/eggpack-ci/src/lib.rs:4296` | `pub fn check_release_github` — unchanged signature, now renders and delegates. |

All three are now two-line functions. Verified by reading the source: each contains exactly one call to `compare_workflow_bytes` and no duplicated comparison logic.

### `render_release_github_inner` decomposed

The 845-line function interleaved several concerns. It is now a sequence of named **private** helpers, each with one responsibility:

| Concern | Helper |
|---|---|
| render state | `ReleaseRenderContext` (`crates/eggpack-ci/src/render_release.rs:379`) |
| policy and shape validation | `validate_release_render_policy` (`crates/eggpack-ci/src/render_release.rs:417`) |
| release-identity mode resolution | `validate_release_identity_mode` (`crates/eggpack-ci/src/render_release.rs:430`) |
| trigger and input cross-checks | `validate_staging_trigger_match` (`crates/eggpack-ci/src/render_release.rs:451`) |
| tool-pin validation | `validate_release_tool_pins` (`crates/eggpack-ci/src/render_release.rs:474`) |
| emulated-runtime policy | `validate_emulated_runtime_policy` (`crates/eggpack-ci/src/render_release.rs:497`) |
| consumer-validator map | `resolve_consumer_validator_map` (`crates/eggpack-ci/src/render_release.rs:525`) |
| step emission | `emit_source_checkout_step`, `emit_runtime_identity_download_step`, `emit_tool_install_and_verify_steps` |
| header and permission emission | `emit_release_workflow_header` (`crates/eggpack-ci/src/render_release.rs:567`) |
| job emission | `emit_preflight_job`, `emit_resolve_job`, `emit_build_jobs`, `emit_qualification_jobs`, `emit_consumer_validation_jobs`, `emit_gate_job`, `emit_aggregate_job`, `emit_stage_job` |
| final shape validation | `validate_rendered_release_workflow` (`crates/eggpack-ci/src/render_release.rs:1316`) |

`render_release_github_inner` (`crates/eggpack-ci/src/render_release.rs:1370`) remains the single entry point, and the public `render_release_github` and `render_reusable_release_github` signatures are unchanged.

### One staging-payload builder

| Location | Change |
|---|---|
| `crates/eggpack-github/src/lib.rs:1012` | `fn prepare_staging_payload_with_presentation_inner` is the **single** builder. It takes the presentation as a parameter, and the presentation selects **only** the presentation-specific work — installer naming and wrapper surface. Every other step — validation, inventory construction, sidecars, installers, the digest loop, the canonical sort, and the payload assembly — exists **once**. |
| `crates/eggpack-github/src/lib.rs:663` | `pub fn prepare_staging_payload` — a thin entry point calling the builder with the `GeneratedDefault` presentation. |
| `crates/eggpack-github/src/lib.rs:690` | `pub fn prepare_staging_payload_with_presentation` — a thin entry point. |

The old doc comment claiming delegation to a shared implementation is replaced by a statement about a single implementation, and the `GeneratedDefault` "dispatch mismatch" branch that only existed to keep the two copies in step is gone.

### The byte-identity evidence

Baselines were captured from `6a7cb4c` **before any edit**, regenerated after, and compared with `cmp`:

| Baseline | Size |
|---|---|
| `render_github.yml` | 1,833 B |
| `render_release_github.yml` | 6,883 B |
| `render_release_github_staging.yml` | 20,796 B |
| `render_reusable_release_github_refname.yml` | 28,591 B |
| `render_reusable_release_github_dispatch.yml` | 28,735 B |
| `drift_reports.txt` (matching / CRLF / first-byte / last-byte / shorter × all three checkers, plus all three over-size refusals) | 1,160 B |
| `staging_generated_default.json` | 1,826 B |
| `staging_product_wrappers.json` | 2,242 B |
| `staging_files.txt` (per-file size and SHA-256 for both presentations) | 1,876 B |

All identical. The committed tests pin these: the CI crate reuses the existing `native-direct.yml`, `m002-direct.yml`, and `m003b-direct-staging.yml` fixtures plus a pinned SHA-256 and length for the reusable workflow; the GitHub crate embeds the two golden staging JSONs.

Two new cross-cutting tests make the duplication a thing that cannot silently return:

- both staging presentations must produce **byte-identical** payloads — today this held only by coincidence of the two copies;
- all three check functions must return **identical** `DriftReport`s for identical byte pairs, across the CRLF, first-byte, last-byte, length-mismatch, and over-size cases — and all three must refuse an over-size workflow, not only one.

## Part two — the module split (`028db4a`, cherry-picked `e66c7d1`)

`crates/eggpack-ci/src/lib.rs` was a single **12,221-line** source file. It is now a **14-file module tree**, with the crate root reduced to **59 lines** of crate docs, module declarations, and `pub use` re-exports.

| Module | Lines | Responsibility |
|---|---|---|
| `src/lib.rs` | 59 | crate docs, `mod` declarations, `pub use` re-exports |
| `src/error.rs` | 19 | `CiError` and `fail` |
| `src/validate.rs` | 60 | bounded identifier, metadata, and release-identity validators |
| `src/planning.rs` | 370 | `CIPlan` types, projection, validation |
| `src/policy.rs` | 739 | pins, runner mappings, tool provisioning, workflow policy |
| `src/runner.rs` | 475 | `RunnerCommand` and its `argv` / `to_shell` |
| `src/render_github.rs` | 256 | the build-workflow renderer and shared render helpers |
| `src/drift.rs` | 106 | `DriftReport`, the one comparison, the three check functions |
| `src/handoff.rs` | 390 | build-handoff projection, validation, staging |
| `src/release_plan.rs` | 655 | `ReleaseCIPlanV1`, gate, aggregation, staging intent |
| `src/render_release.rs` | 1,394 | the release renderers and the `emit_*` helpers |
| `src/consumer_validator.rs` | 560 | consumer-validator config, evidence, bounded run |
| `src/identity.rs` | 215 | static shape and runtime identity resolution |
| `src/tests.rs` | 7,538 | the test module, moved out of `lib.rs` |

The split used the crate's existing section banners and the actual content as the boundaries. The test module's move to `src/tests.rs` follows the `eggpack-github` crate's precedent, as the plan requires.

### The public API did not change

The split is the part of a module tree most likely to leak: a private item in a submodule is not visible to its parent, so a naive split either fails to compile or widens visibility to make it work.

- A `cargo public-api` dump is **byte-identical** before and after — 2,173 lines.
- A committed test, `m003n_public_api_surface_is_unchanged`, pins the crate root's re-export set (85 names), 38 types, 10 constants, 37 function signatures, and the enum variants.
- **27 items were widened to `pub(crate)`** so sibling modules can reach them, and `CiError`'s payload became `pub(crate)` so sibling modules can construct it. **Nothing became `pub`.** The crate's public surface is exactly what it was.

### The consumers did not change

`eggpack-cli` and `eggpack-github` together hold **160** `eggpack_ci::` references, and the normalized-import digest is **identical** before and after (`9feae34e012bbbcf2449d901c51aee31`). `git diff --name-only` over `Cargo.toml`, `Cargo.lock`, and `crates/*/Cargo.toml` returns **zero** files, as it does for every crate outside `eggpack-ci`.

### The byte-identity evidence

Baselines were captured **before any edit** into `/tmp/eggpack-m003n-before`: the `render_github.yml`, `render_release_github_m002.yml`, `render_release_github_m003b_staging.yml`, and both `render_reusable_release_github` variants; a `drift_reports.txt` covering matching, CRLF, first-byte, last-byte, length-mismatch, and over-size cases across all three check functions; and an `artifacts.txt` holding the `CIPlan` and `ReleaseCIPlan` JSON, the gate and consumer-gate outcomes, the qualification and consumer evidence, and the handoff JSON.

After the split, `diff -r` reports **no differences** and the SHA-256 digests are identical — the reusable-dispatch workflow at `1eb55594…` and the release-m002 workflow at `39800e0b…`.

### The invariants from the two preceding milestones survived the move

- CI M003m's invariant holds: the CI crate's **production region contains no** `Command::new`, `.spawn()`, `group_spawn`, `.kill()`, `thread::spawn`, `JoinHandle`, `Stdio::`, or `env_clear`.
- CI M003l's delegation is intact: no crate-side copy of the host-match, floor-applicability, exact-tag, SHA-256 acceptance, or file-digest rule was reintroduced.

### Documentation

`architecture/ci.md` and `architecture/ci-rendering.md` had their citations re-based **by symbol name** to `module.rs:line`, with a convention line stating that they are relative to `crates/eggpack-ci/src/`. `crates/eggpack-ci/README.md` gained the module map. The pre-existing stale baseline SHA in `architecture/overview.md` was deliberately left for Planning Hygiene M003, which is the milestone that owns it.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §5.1 — one drift comparison, with the order of the bound, the normalization, the comparison, the first-difference fallback, and the report unchanged | `compare_workflow_bytes` (`:1860`); the three check functions are two-line delegators. |
| §5.2 — one staging-payload builder, with the presentation selecting only the presentation-specific work | `prepare_staging_payload_with_presentation_inner` (`:1012`); two thin entry points. |
| §5.3 — `render_release_github_inner` decomposed by concern, with unchanged public signatures | The helper table above; `render_release_github` and `render_reusable_release_github` unchanged. |
| §5.4 — the CI crate becomes a module tree, with the public API re-exported identically so no consumer changes an import | Recorded in the module-split section below. |
| §5.5 — no behaviour change | The byte-identity evidence above and below. |
| §8 — no public API break; `eggpack-ci` and `eggpack-github` are workspace-internal and not published | The public-item diff is identical for both crates — 112 identical public items in `eggpack-ci`, 61 in `eggpack-github` — and the 60 distinct `eggpack_ci::` paths imported by `eggpack-cli` and `eggpack-github` diff clean against `6a7cb4c`. |
| §8 — no new dependency; no `Cargo.toml` change; no `Cargo.lock` change | `Cargo.toml` and `Cargo.lock` are absent from the commit. |
| §8 — no schema change | `CIPlan v1` and every other document type are unchanged. |
| §3 — the CI crate's delegation to the canonical producer rules stays intact | The M003l delegation is unchanged; no crate-side copy of the host-match, floor-applicability, exact-tag, SHA-256 acceptance, or file-digest rule was reintroduced. |
| §3 — the CI crate's production region contains no process-spawning code | The M003m invariant holds: no process or exec construct above the test module. |
| §3 — least privilege in the generated workflows is unchanged | Only the staging job has write scope and there is no publish or release step; the rendered bytes are byte-identical, so the permissions block is unchanged. |

## Verification executed

All verification below ran against the consolidated implementation, scoped to the actual change as the maintainer directed for this stage:

```text
cargo fmt --all -- --check                                                  passed
cargo check --workspace --all-targets --all-features --locked               passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked               passed (78 passed, 0 failed, 3 ignored; 73/3 before M003n, 77/3 after the consolidation half)
cargo test -p eggpack-github --all-targets --all-features --locked          passed (43 passed, 0 failed)
cargo test -p eggpack-cli --all-targets --all-features --locked              passed (27 + 8 + 18 = 53 passed, 0 failed)
git diff --check                                                            passed
```

**Not run, by the maintainer's scope direction and by the absence of a push:** `./scripts/check-local.sh`, the `cargo +1.89.0` MSRV lane, the `cargo package` steps, and hosted CI. Plan §12 requires the full gate and a hosted run; neither was performed and no hosted run is claimed here. The full local gate runs once at the end of the chain.

## Compatibility and no-regression evidence

- **No rendered output changed** — proven by before/after byte comparison, not by inspection.
- **No public API changed** — proven by an identical public-item diff and an unchanged import diff.
- **No schema version bump.**
- **No dependency added** in any crate; `Cargo.toml` and `Cargo.lock` untouched.
- **No historical closure rewritten.**
- **Pre-existing tests stay green** across CI, GitHub, and CLI.

## Security and reliability review

- The change is a **refactor with byte-identical output**, so it introduces no new surface and removes none. The security posture of the rendered workflows, the staging payloads, and the drift checks is exactly what it was.
- The consolidation removes three ways for the drift comparison to drift apart: the bound, the normalization, and the first-difference computation now exist once.
- The staging-payload consolidation removes the failure mode where the two presentations disagreed because one copy was updated and the other was not. The new cross-presentation byte-identity test makes that divergence a test failure rather than a silent inconsistency.
- No network, credential, privilege, archive, or publication surface is touched.

## Failure and recovery review

- The refactor introduces no new state, no new I/O, and no new failure mode. Every failure path is the one that already existed.
- The byte-identity comparison is the failure detector: if a future edit changes any rendered byte, the golden baselines and the committed tests fail.
- Rollback is a revert of this milestone's commits; the pre-consolidation code had the duplicated implementations, which is the defect this milestone removes.
- No contention or restart hazard: the milestone adds no external service, cache, or shared state.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, maintenance) | Three byte-identical drift-check bodies, two staging-payload builders that were 85.5% identical, and an 845-line renderer that interleaved several concerns. | All consolidated, with byte-identical output proven by before/after comparison. |
| Low (test coverage) | The reusable-workflow golden is pinned by SHA-256 and length rather than by a fixture file. | Deliberate: the milestone's stated file allow-list did not include a new fixture file, and a digest pins the same bytes. Recorded here rather than left implicit. |
| Low (documentation debt, carried to Planning Hygiene M003) | `architecture/overview.md`, `architecture/cli.md`, `architecture/determinism.md`, `architecture/validation-model.md`, and `architecture/ci-consumer-seam.md` carry **pre-existing stale citations** — for example `determinism.md` cites `normalize_newlines` at `:1818-1831` where it now lives at `:1895-1908`, and `cli.md` cites `prepare_staging_payload` at `:1550` where it now lives at `:663`. | Out of this milestone's scope by design: the consolidation half re-based only the citations in the sections it moved. The full by-symbol-name re-base is Planning Hygiene M003's job, and it is the next milestone. |
| Operational boundary | Verification ran on Linux x86-64 only. The macOS and Windows lanes are covered by hosted CI only, and no hosted run was performed because nothing was pushed. | Accepted; the same boundary every prior milestone recorded. |
| None | No unresolved medium-or-higher defect in the corrected producer structure. | M003n acceptance criteria satisfied. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003n | Reason |
|---|---|---|
| CI/Release M003n | closed | This record. |
| Planning Hygiene M003 | ready | Its docs-only baseline reconciliation now runs against the final shape of the tree, and it is the milestone that closes the stale-citation debt recorded above. |

## Downstream-unblock answer

Closing M003n unblocks **Planning Hygiene M003**, which is the last milestone in this review chain. Because the whole chain is now closed, nothing in it remains blocked.

## Eggstack references

- Closed source plan: `plans/implementation/ci-release-orchestration/003n-producer-implementation-consolidation-corrective.md`
- Predecessor in this chain: `plans/closure/ci-release-orchestration/003m-status.md` (`bd73f5c`)
- Successor: `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`
