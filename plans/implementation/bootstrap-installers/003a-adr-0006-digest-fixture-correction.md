# Bootstrap Installers Milestone 003a Corrective — ADR-0006 Adversarial Digest Fixture Correction

Status: closed

Subsystem: Bootstrap installers.

Predecessor: `plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md`.

Triggered by: CI/Release M003l (`9cf0bf8`), which implemented accepted ADR-0006 and tightened the SHA-256 hex acceptance set. The tightening was correct; the **fixtures** that exercised the bootstrap installers' own integrity checks were not.

## 1. Objective

Restore the bootstrap installers' adversarial digest coverage.

Five bootstrap test fixtures used the all-zeros digest (`"00".repeat(32)`) as the stand-in for "a digest that does not match the bytes being installed". Under accepted ADR-0006 the all-zeros sentinel is refused by the release-manifest acceptance rule, so those fixtures now fail **at manifest validation**, before the installer's own size-and-digest integrity check ever runs.

The fix is to use a **well-formed but wrong** digest in those fixtures, so the manifest still validates and the installer's own check is what rejects the artifact. The tests keep testing what they were written to test.

## 2. Current evidence and the gaps

`cargo test -p eggpack-bootstrap --all-targets --all-features --locked` reports **8 passed, 3 failed**:

```text
---- tests::m002_archive_posix_runtime_matrix stdout ----
thread 'tests::m002_archive_posix_runtime_matrix' panicked at crates/eggpack-bootstrap/src/lib.rs:2659:88:
called `Result::unwrap()` on an `Err` value: BootstrapError("invalid release manifest")
---- tests::m002_bundle_posix_runtime_matrix stdout ----
thread 'tests::m002_bundle_posix_runtime_matrix' panicked at crates/eggpack-bootstrap/src/lib.rs:2337:88:
called `Result::unwrap()` on an `Err` value: BootstrapError("invalid release manifest")
```

The failing unwraps are on `render_posix_with_policy(...).unwrap()` calls, which validate the manifest before rendering.

The five affected fixture sites, all with the same intent — *the installer must refuse a digest that does not match the served bytes*:

| Site | Test intent |
|---|---|
| `crates/eggpack-bootstrap/src/lib.rs:1331` | a PowerShell installer refuses a mismatched digest and leaves no files |
| `crates/eggpack-bootstrap/src/lib.rs:1367` | a POSIX installer refuses a mismatched digest and leaves no files |
| `crates/eggpack-bootstrap/src/lib.rs:2335` | "wrong SHA leaves no files" for a bundle target |
| `crates/eggpack-bootstrap/src/lib.rs:2656` | "archive size / SHA mismatch rejects without placement" for an archive target |
| `crates/eggpack-bootstrap/src/lib.rs:3330` | "archive integrity negatives fail before member listing/extraction" for an archive target |

## 3. Invariants

- The **installer-side** integrity check stays under test. The fix must not turn these fixtures into tests of the manifest validator.
- The fixtures' digests must remain **well-formed**: exactly 64 lowercase hex characters, so the manifest's acceptance rule admits them.
- The fixtures' digests must remain **wrong**: they must not equal the digest of the bytes the fixture serves.
- The digest chosen must not collide with any digest the bootstrap crate already uses in a fixture (`"ab".repeat(32)`, `"11".repeat(32)`, and the SHA-256 of `b"abc"`).
- **No production behaviour changes.** Only test fixtures change.
- `#![forbid(unsafe_code)]` holds. `unsafe_code = "deny"`, `clippy::all = warn`, verification denies warnings.
- Rust edition 2021, `rust-version = 1.89`.
- No new dependency. No `Cargo.toml` change. No `Cargo.lock` change.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-bootstrap/src/lib.rs` test fixtures only.

**Out of scope:** the bootstrap crate's production code; the `eggpack-manifest` acceptance rule; ADR-0006 itself; every other crate.

**Non-goal:** weakening the acceptance rule. The all-zeros digest stays refused everywhere. The point is that the installer must still catch a *well-formed* mismatch, which is the realistic case.

## 5. Required production changes

None. This is a fixture correction.

## 6. Ordered work packages

1. **WP1** — Replace the all-zeros placeholder with a well-formed, wrong digest at each of the five sites, chosen so it does not collide with any digest the crate already uses in a fixture.
2. **WP2** — Re-run the bootstrap suite and confirm the three failing tests pass and the coverage they provided is intact: each must still assert that the installer refuses the artifact and leaves nothing behind.
3. **WP3** — Confirm the whole workspace suite is green.

## 7. Failure, restart and contention semantics

A fixture change cannot fail closed against production. If a test still fails after the change, the cause is something other than the digest placeholder — stop and report it rather than adjusting the expectation.

## 8. Compatibility and migration

None. Test fixtures are not a published surface, and no crate's behaviour changes.

## 9. Required tests

- the three previously failing bootstrap tests pass;
- the rest of the bootstrap suite is unchanged and green;
- the full workspace suite is green;
- a grep confirms no `"00".repeat(32)` remains in the bootstrap crate, and that the replacement digest appears at exactly the five intended sites.

## 10. Verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
```

## 11. Documentation updates

None required. The `eggpack-manifest` acceptance rule is unchanged, so no architecture document describes different behaviour.

## 12. Acceptance criteria

1. The three failing bootstrap tests pass, and each still asserts that the installer refuses the artifact rather than that the manifest is malformed.
2. The bootstrap crate contains no all-zeros digest placeholder.
3. The replacement digest is well-formed and does not collide with any digest already used in the crate's fixtures.
4. No production code, no `Cargo.toml`, and no `Cargo.lock` changed.
5. The full workspace test suite is green.

## 13. Stop conditions

Stop and re-plan if:

- the installer-side integrity check cannot be exercised with a well-formed wrong digest — that would mean the installer's own check is unreachable, which is a different and more serious defect;
- the failure turns out to have a cause other than the digest placeholder.

## 14. Closure evidence

Write `plans/closure/bootstrap-installers/003a-status.md` with: the reviewed baseline; the failing test output before the change and the passing output after; the exact sites changed and the digest chosen, with the argument for its well-formedness and non-collision; the requirement-to-evidence matrix; the `git diff --stat` evidence that only test fixtures changed; the verification commands and their actual results; and the invariant review.

## 15. Handoff notes

- Start from the branch that carries CI/Release M003l.
- The all-zeros sentinel stays refused everywhere. Do not "fix" this by relaxing the acceptance rule.
- `crates/eggpack-manifest` and the `eggpack-manifest` crate are out of scope: the rule is right and must not move.
