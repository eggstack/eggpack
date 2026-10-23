# Build/Qualification M007 — Evidence Digest Unification Corrective

Status: **ready for implementation**.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/build-qualification-roadmap.md`.
Primary class: invariant / integrity-evidence corrective.
Historical upstream authority: ADR-0002 contract/plan/manifest separation; ADR-0005 §9 compatibility-floor evidence boundary (the rule that a digest must describe the bytes it claims to describe).
Affected closed milestones: Build/Qualification M001 (implementation `4eba6ec9422ab01a675ef34cdfc2b9d6f9c0944d`, introduced `crates/eggpack-core/src/lib.rs:291`), Build/Qualification M004 (implementation `661288cee39df3aa81ed5d8a5c3b530ced64788e`, introduced `crates/eggpack-core/src/finalization.rs:482`). Both closures remain historical and are not rewritten.
Dependency: none. This milestone is confined to `crates/eggpack-core/**` and does not touch `crates/eggpack-ci/**`, so it does not contend with CI M003k (in flight, PR `eggstack/eggpack#1`).

## 1. Objective

Replace `eggpack-core`'s two divergent file-digest implementations with **one**
canonical bounded digest helper that has exactly one defined behaviour for
symlink rejection, empty-file rejection, size arithmetic, and post-read
stability, and make every core digest call site use it.

The milestone establishes the canonical helper that
`plans/implementation/ci-release-orchestration/003l-producer-rule-authority-convergence-corrective.md`
later consumes from `eggpack-ci` and `eggpack-github`. It does **not** itself
change any other crate.

## 2. Current evidence and the defect

`eggpack-core` contains two file-digest helpers that were written for different
milestones and never reconciled.

| | `digest_file` | `digest_regular` |
|---|---|---|
| Location | `crates/eggpack-core/src/lib.rs:291-324` | `crates/eggpack-core/src/finalization.rs:482-512` |
| Introduced by | Build M001, `4eba6ec` | Build M004, `661288c` |
| Symlink rejected before open | yes | yes |
| Empty file rejected **before** reading | no — rejected after reading, at `:319-321` | yes, at `:484` |
| Post-read stability re-check | **no** | yes, at `:503-506` |
| Size arithmetic | `checked_add` | `checked_add` |
| Error type | `CoreError` | `CoreError` |

The post-read re-check is the material difference. `digest_regular` re-stats the
path after the read and refuses the file if it became a symlink, stopped being a
regular file, or changed length while being read. `digest_file` does not, so a
path replaced between the pre-open `symlink_metadata` check and the open can
produce a digest that describes bytes other than the file that was inspected —
the exact TOCTOU hazard that `digest_file`'s own comment at
`crates/eggpack-core/src/lib.rs:292-296` claims to guard against.

Two consequences:

- The manifest that `build_manifest` aggregates at
  `crates/eggpack-core/src/lib.rs:181` is built from `digest_file` results, so
  the final-bytes evidence record is produced by the weaker of the two helpers.
- The stronger helper already exists in the same crate, so this is a divergence
  to be closed, not a new capability to be built.

## 3. Invariants

- A recorded digest MUST describe the bytes of the exact regular file that was
  inspected. The recorded size and digest MUST be mutually consistent.
- A symlink MUST never be followed, and a file that becomes a symlink, changes
  type, or changes length during the read MUST be refused, not digested.
- An empty file MUST be refused before it is read, so a zero-length input can
  never produce a digest record.
- Size accumulation MUST be overflow-checked; a size overflow is an error, never
  a wrap.
- Digest computation MUST stream through a fixed-size buffer and MUST NOT hold
  the whole artifact in memory. A finalized release bundle can be far larger
  than the process should grow to accommodate.
- The digest helper MUST NOT change what bytes are hashed. This milestone
  changes only *which* helper is used and *what it refuses*, never the hash
  function, the encoding, or the lowercase hex representation.
- Determinism is preserved: identical bytes MUST produce an identical digest
  and size record, and no error message may embed artifact content.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-core/**` only. The canonical helper, the two
retirements, the call sites, and the tests.

**Out of scope:**

- the SHA-256 *hex acceptance set* (the validator rule, not the digest
  implementation). That is decided by
  `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` and
  implemented by CI M003l. M007 changes **no** acceptance set.
- `crates/eggpack-ci/src/lib.rs:4663` `hash_candidate_file` and
  `crates/eggpack-github/src/lib.rs:597` `digest_file`. These are consumers of
  the canonical helper and are converged by CI M003l.
  *(The two locations are as of this plan's repository baseline (`fb81aa1`,
  2026-10-23). The CI crate was afterwards split into a module tree by
  CI/Release M003n: `hash_candidate_file` now lives at
  `crates/eggpack-ci/src/consumer_validator.rs:477` and `digest_file` at
  `crates/eggpack-github/src/lib.rs:604`.)*
- Any change to `ReleaseManifest` v1's fields, `schema_version`, or the
  published `eggpack-manifest` crate.
- Any change to archive assembly, sidecar content, or the finalization order.

## 5. Required production changes

1. Define one canonical bounded file-digest helper in `eggpack-core`, carrying
   the **stricter** of the two current semantics:
   - `symlink_metadata` before open; refuse anything that is not a regular
     non-symlink file;
   - refuse an empty file **before** reading;
   - read through a fixed-size buffer; accumulate size with `checked_add`;
   - `symlink_metadata` again after the read; refuse if the path became a
     symlink, stopped being a regular file, or its length differs from the
     number of bytes read;
   - return the size and the lowercase hex SHA-256 digest.

   The helper MUST be `pub` so that `eggpack-ci` and `eggpack-github` can
   consume it in CI M003l, and MUST return a typed error rather than a
   sentinel value.

2. Retire `crates/eggpack-core/src/lib.rs:291` `digest_file` and
   `crates/eggpack-core/src/finalization.rs:482` `digest_regular`. Every core
   call site routes to the canonical helper.

3. The canonical helper's error type MUST distinguish the refusal reasons the
   current helpers distinguish, so that no diagnostic becomes vaguer than it
   is today. Diagnostics remain single-line and bounded, per the workspace's
   diagnostic discipline.

4. The helper MUST be documented with the TOCTOU rationale that
   `crates/eggpack-core/src/lib.rs:292-296` currently carries, so the
   protection survives the move.

## 6. Ordered work packages

### WP1 — Failing discriminating tests first

Add tests that fail against the current tree and pass after the change:

- **Post-read replacement is refused.** Build a file, digest it with the
  canonical helper, and prove that the helper's post-read re-check is what
  refuses a path that changed length during the read. On platforms where the
  race cannot be staged deterministically, assert the *presence* of the
  re-check by unit-testing the post-read comparison directly rather than by
  racing a thread, and say so in the test comment.
- **A symlink is refused** for both the digest path and the finalization path.
- **An empty file is refused before any read occurs.**
- **Size overflow is an error**, not a wrapped value.
- **Identical bytes produce identical digests** and different bytes produce
  different digests, proving the helper did not change the hash semantics.

Each test MUST fail against the pre-change tree for the reason it exists, not
for an incidental reason.

### WP2 — Canonical helper

Implement the canonical helper with the semantics in §5. Keep it in a location
that makes the ownership explicit to `eggpack-ci` and `eggpack-github`, and
document it as the **single** producer-side file-digest authority.

### WP3 — Retire the duplicates

Replace both call sites. Grep for remaining uses of the retired names and
confirm zero remain. Confirm no second file-digest implementation survives
inside `eggpack-core`.

### WP4 — Verification and documentation

Run the full gate. Update `architecture/core-finalization.md`,
`architecture/core.md`, and `crates/eggpack-core/README.md` to name the
canonical helper and to state the TOCTOU rule once, instead of describing it
in two places.

## 7. Failure, restart and contention semantics

- A refused digest is a **hard failure** of the enclosing operation. It is
  never downgraded to a warning, never retried against the same path, and
  never recorded as an empty digest.
- No partial or placeholder digest is ever emitted. There is no sentinel
  success value.
- The helper performs no writes and holds no locks, so concurrent callers are
  independent. Two callers digesting the same path concurrently may observe
  different results if the file legitimately changes between them; each
  observes a self-consistent (size, digest) pair or an error.
- Restarting a failed finalization re-runs the digest from the beginning. There
  is no resumable state to corrupt.

## 8. Compatibility and migration

- No schema change. `ReleaseManifest` v1 fields, `schema_version`, and the
  published `eggpack-manifest` crate are untouched.
- No checked-in document changes meaning. A manifest produced before this
  milestone still validates, because the tightened behaviour refuses only
  inputs that were previously refused by one of the two helpers anyway.
- The only behavioural narrowing is: a file that changes during the read is now
  refused by `build_manifest`'s path as well as by the finalization path.
  That is the intended correction and it fails closed.
- No new dependency. `sha2` is already a `eggpack-core` dependency.
- No public binary or library API break beyond the removal of two private
  helpers. `eggpack-core` is not published.

## 9. Required tests

- unit tests for the canonical helper: regular file, empty file, symlink,
  directory, missing path, post-read length change, post-read type change;
- a regression test proving `build_manifest` refuses a finalized file that
  changes during the read, where the platform allows deterministic staging;
- a regression test proving the finalization path still produces the same
  digest and size for unchanged bytes as before the change — the golden
  digests already present in the finalization fixtures MUST be unchanged;
- an explicit test that the digest and size records agree for every artifact in
  an aggregated manifest.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
```

Add a direct narrowed run for the new tests, e.g.
`cargo test -p eggpack-core --all-targets --all-features --locked <new_test_name> -- --nocapture`.
On Windows, append `-- --test-threads=1` when diagnosing serialization.

## 11. Documentation updates

- `architecture/core-finalization.md` — name the canonical helper and the
  TOCTOU rule.
- `architecture/core.md` — update the file/crate line counts if they move.
- `crates/eggpack-core/README.md` — record that core owns the single
  producer-side file-digest authority.
- `architecture/overview.md` — its line counts and baseline SHA are already
  stale; **do not** re-base them here. Planning Hygiene M003 performs the full
  re-base against a measured baseline; this milestone re-verifies the
  citations in the sections it touches, by symbol name. See
  `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`.
## 12. Acceptance criteria

1. Exactly one file-digest implementation exists in `eggpack-core`, and it is
   the one used by `build_manifest` and by finalization.
2. That helper refuses symlinks, empty files, non-regular files, and files that
   change during the read; accumulates size with overflow checking; streams
   through a fixed-size buffer; and returns a typed error rather than a
   sentinel.
3. Every in-tree golden digest in the existing fixtures is unchanged.
4. The regression tests in WP1 fail against the pre-change tree for the reason
   they exist, and pass after the change.
5. No acceptance set, schema field, `schema_version`, or published crate
   changed.
6. The full workspace gate and `./scripts/check-local.sh` are green, and hosted
   CI is green on all lanes.
7. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- unifying the helpers would change any recorded digest or size for unchanged
  bytes;
- the post-read re-check proves unstably racy on a supported platform, such
  that the helper would refuse a file that did not actually change;
- the canonical helper cannot be exposed to `eggpack-ci` and `eggpack-github`
  without changing a published contract;
- any change to `ReleaseManifest` v1, the `eggpack-manifest` crate, or the
  archive/sidecar formats appears to be required;
- the work would require touching `crates/eggpack-ci/**` or
  `crates/eggpack-github/**`, which belong to CI M003l.

## 14. Closure evidence

Write `plans/closure/build-qualification/007-status.md` with: the reviewed
baseline and implementation SHA; the before/after helper inventory; the
regression evidence with exact test names and commands; the golden-digest
unchanged proof; the verification commands executed and their actual results;
hosted CI run IDs and the lane matrix spelled out; an invariant review against
§3; an explicit statement that no acceptance set, schema field, or published
artifact changed; unresolved findings by severity; and the CI M003l unblock.

The M001 and M004 closure records remain historical and unchanged.

## 15. Handoff notes

- Start from `fb81aa1400e1f333f6500075f78d944a9242b673`.
- This milestone is **independent of CI M003k** and may be executed while the
  M003k PR is open, because it does not touch `crates/eggpack-ci/**`.
- The canonical helper's public surface is the contract that CI M003l consumes.
  Land it with that in mind: name it, document it, and make it the obvious
  thing for a downstream crate to call.
- The digest **acceptance set** is not decided here. If ADR-0006 is accepted
  before this milestone lands, do **not** implement its rule here; CI M003l
  owns it.
