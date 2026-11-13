# CI/Release M003o Corrective — Renderer Panic and Digest Diagnostics

Status: **closed**.

Subsystem: CI and release orchestration.

Predecessor: `plans/implementation/ci-release-orchestration/003n-producer-implementation-consolidation.md` (M003n, implementations `4692bd5` and `e66c7d1`, closure `plans/closure/ci-release-orchestration/003n-status.md`).

Triggered by: the 2026-10-23 post-implementation review of the M003n chain, which found one reachable panic in the renderer and two diagnostics that had become inaccurate under ADR-0006.

M003n's closure record remains historical and is not rewritten. This corrective is a new plan, per the repository's closure discipline.

## 1. Objective

Close the one reachable panic in the release-workflow renderer, and make the two digest diagnostics name the reason they actually refused.

The review of the CI crate returned **PASS** on the substance: no rule was weakened by the M003l convergence, no rendered byte changed by the M003n consolidation, the CI crate's public API was byte-identical before and after the module split, and the CI production region contains no process-spawning code. The two defects below are the exceptions, and both are small.

## 2. The defects

### 2.1 A reachable panic in the renderer

`mkdir_step` asserted that every generated working-directory name is a fixed safe relative path of at most 128 characters. The check was sound as an invariant statement; using an assertion to enforce it was not, because the directory name is derived from a caller-supplied document.

The consumer-evidence directory is `./eggpack-consumer/{build_job_id}`, and the build job id is derived from the contract's target triple. A contract may legally declare a triple at the contract's own name bound of 128 characters — `validate_triple` accepts it, since `MAX_NAME_LEN` is 128 and the triple uses only `[A-Za-z0-9-_.]` and contains a `-`. That target produces a 134-character job id and a **153-character** directory name, which trips the assertion.

So a caller-supplied contract could panic the renderer, rather than the renderer refusing a document it cannot represent safely. The same input is refused upstream by the release-plan projection, which is why the panic was not reachable through the full CLI path — but the renderer is a public function and must not panic on its input.

### 2.2 Two diagnostics that misdescribe the refusal

Two messages said a refused digest "must be lowercase hex" or "must be 64 lowercase hex characters". Under ADR-0006 the acceptance set was narrowed, so a refused digest is **not always** a formatting problem: the all-zeros digest is perfectly good lowercase hex and is refused for a different reason. The diagnostic told an operator that a well-formed digest was malformed.

## 3. Invariants

- The renderer MUST NOT panic on a caller-supplied document. A document it cannot represent safely MUST be refused with an error.
- A refusal diagnostic MUST name the reason the refusal happened. It MUST NOT misdescribe a well-formed value as malformed.
- The rendered bytes MUST NOT change for every input that was accepted before. The refusal applies only to inputs that were previously refused by an assertion.
- The CI crate's public API MUST NOT change.
- Fail-closed behaviour is preserved: the refusal is an error, not a skip and not a sanitisation.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-ci/src/{render_release.rs,policy.rs,consumer_validator.rs}`.

**Out of scope:**

- The rendered workflow's content, structure, step order, pins, or gates. They are unchanged.
- The CI crate's public API surface. It is unchanged: the 85 public items are identical before and after the module split.
- The process-execution convergence from M003m. The runner's outcome mapping, its bounds, and its environment allowlist are unchanged.
- The contract crate's own name bound. The bound stays at 128; the renderer now refuses what it cannot represent.
- `crates/eggpack-core/**`, `crates/eggpack-manifest/**`, and the published `eggpack-manifest` crate.

## 5. Required production changes

1. `mkdir_step` returns `Result<(), CiError>` and refuses an unrepresentable directory instead of asserting. Its four call sites propagate the error.

2. The CI crate's `validate_sha256_hex` and the two consumer-validator digest checks name the actual refusal reason, using the reason phrase `eggpack-manifest` already publishes on `Sha256Error`.

## 6. Required tests

1. A contract with a target triple at the contract's name bound is projected into a graph whose build job id is 134 characters, and the renderer refuses it with an error rather than panicking. The test is confirmed to fail against the previous assertion.

2. The CI crate's existing rendering tests continue to pass unchanged, which is what proves the rendered bytes did not move.

## 7. Verification

`cargo fmt --all -- --check`, `cargo clippy -p eggpack-ci --all-targets --all-features --locked -- -D warnings`, and `cargo test -p eggpack-ci --all-targets --all-features --locked`.

The workspace gate is not the acceptance criterion for this corrective; it runs once for the whole review line.

## 8. Closure

Closed at `plans/closure/ci-release-orchestration/003o-status.md`.
