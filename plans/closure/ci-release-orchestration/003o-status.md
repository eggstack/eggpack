# CI/Release Milestone 003o Closure — Renderer Panic and Digest Diagnostics

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003o-renderer-panic-and-digest-diagnostics.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md` (registered there)

Triggered by: the 2026-10-23 post-implementation review of the M003n chain.

Reviewed baseline: `e0e1510` (the tip of the review chain), on top of M003n's implementations `4692bd5` and `e66c7d1`.

## Executive finding

CI/Release 003o is closed. The release-workflow renderer no longer panics on a caller-supplied contract, and the two digest diagnostics name the reason they actually refused.

The review of the CI crate returned **PASS** on the substance. The evidence for that verdict is recorded below, because a corrective milestone that fixes two small things must not imply that the milestone it corrects was otherwise unsound.

## What the review confirmed about the CI crate

- **No rule was weakened by the M003l convergence.** The CI crate's local copies of the SHA-256 hex validator, the host-match rule, the policy validator, the floor-applicability rule, and the exact-tag-safety rule are gone; the survivors are thin delegations to `eggpack_core` and `eggpack_manifest`, and each delegation is semantically identical to the copy it replaced.
- **No rendered byte changed by the M003n consolidation.** The consolidated drift comparison, the nineteen-helper decomposition of the release-workflow renderer, and the merged staging-payload builder all produce the same bytes.
- **The public API is unchanged.** The 85 public items of `eggpack-ci` are identical before and after the module split — nothing became `pub`, nothing was removed.
- **The CI production region contains no process-spawning code.** `grep` for `Command::new`, `.spawn()`, and `std::process` across `crates/eggpack-ci/src/` returns nothing outside the test module. The two `run_interpreter_bounded` call sites are the only process-execution entry points, as M003m intended.
- **The `try_wait` failure classification** was examined and confirmed as a diagnostic-accuracy drift only: both the old and the new classification fail the consumer validation, so no pass/fail behaviour changed.

## Defect 1 — a reachable panic in the renderer

`mkdir_step` asserted that every generated working-directory name is a fixed safe relative path of at most 128 characters. The invariant was right; enforcing it with an assertion was not, because the directory name is derived from a caller-supplied document.

The consumer-evidence directory is `./eggpack-consumer/{build_job_id}`, and the build job id is derived from the contract's target triple. A contract may legally declare a triple at the contract's own name bound of 128 characters: `validate_triple` accepts it, because `MAX_NAME_LEN` is 128 and such a triple uses only `[A-Za-z0-9-_.]` and contains a `-`. That target produces a 134-character job id and a **153-character** directory name.

A caller-supplied contract could therefore panic the renderer, where the renderer must refuse a document it cannot represent safely.

### Production implementation evidence

`crates/eggpack-ci/src/render_release.rs`:

- `mkdir_step` returns `Result<(), CiError>` and refuses an unrepresentable directory with `generated mkdir directory is not a fixed safe relative path`, instead of asserting.
- Its four call sites propagate the error.
- The doc comment now states why the check is a refusal rather than an assertion.

### Test evidence

`crates/eggpack-ci/src/tests.rs` — `m003n_review_refuses_an_over_long_generated_directory_instead_of_panicking`:

- Builds a real contract whose triple is 128 characters, projects the release graph through the ordinary pipeline, and asserts the build job id is 134 characters.
- Asserts the renderer returns an error.

The test was confirmed to be a genuine regression test: with the previous assertion restored, it fails with

```text
thread 'tests::m003n_review_refuses_an_over_long_generated_directory_instead_of_panicking' panicked at
  crates/eggpack-ci/src/render_release.rs:40:5:
test result: FAILED. 0 passed; 1 failed
```

and passes with the refusal.

## Defect 2 — two diagnostics that misdescribed the refusal

Two messages said a refused digest "must be lowercase hex" or "must be 64 lowercase hex characters". Under ADR-0006 the acceptance set was narrowed, so a refused digest is not always a formatting problem: the all-zeros digest is perfectly good lowercase hex and is refused for a different reason. The diagnostic told an operator that a well-formed digest was malformed.

### Production implementation evidence

- `crates/eggpack-ci/src/policy.rs` — `validate_sha256_hex` now reports `SHA-256 digest {reason}`, using the reason phrase `eggpack-manifest` already publishes on `Sha256Error`.
- `crates/eggpack-ci/src/consumer_validator.rs` — the two digest checks report `expected candidate digest: {reason}` and `consumer evidence digest: {reason}`.

The `eggpack-github` crate already distinguished these two reasons correctly and was not changed.

## Verification

```text
$ cargo fmt --all -- --check
(clean)

$ cargo clippy -p eggpack-ci --all-targets --all-features --locked -- -D warnings
(clean)

$ cargo test -p eggpack-ci --all-targets --all-features --locked
79 passed; 0 failed; 3 ignored
```

## Invariants re-checked

- The renderer does not panic on a caller-supplied document; it refuses.
- A refusal diagnostic names the reason it refused.
- The rendered bytes did not change for every input that was accepted before. The refusal applies only to inputs an assertion previously turned into a panic.
- The CI crate's public API is unchanged: 85 items, identical to `origin/main`.
- Fail-closed behaviour is preserved: the refusal is an error, not a skip and not a sanitisation.
- The contract crate's name bound is unchanged. The renderer refuses what it cannot represent; the contract does not get looser.

## Historical record

M003n's own closure record (`plans/closure/ci-release-orchestration/003n-status.md`) remains as written. The defects it did not cover are recorded here and in the M003o plan; no history was rewritten.
