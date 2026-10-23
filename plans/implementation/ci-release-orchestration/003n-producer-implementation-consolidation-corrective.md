# CI/Release M003n — Producer Implementation Consolidation Corrective

Status: **blocked** on CI M003m.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`.
Primary class: maintainability corrective (no behaviour change intended).
Historical upstream authority: `architecture/determinism.md` and
`architecture/overview.md`, both of which require byte-identical renders and a
single canonical implementation of each rendered artifact.
Affected closed milestones: CI M001 (introduced `check_github`), CI M002
(introduced the release renderers and `check_release_github`), CI M003a
(implementation `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9`, introduced
`prepare_staging_payload`), CI M003d (implementation
`946dd7a827fcbf7552721ed162972ae5ae78b5f2`, introduced
`prepare_staging_payload_with_presentation` and
`check_reusable_release_github`). The closures remain historical and are not
rewritten.
Dependency: **CI M003m** must be closed first. This milestone reorganises the
CI crate's module structure, and doing that while the execution path is still
being changed would make the diff unreviewable.

## 1. Objective

Remove the duplicated producer implementations that a post-implementation
review found, and bring the two largest source files in the workspace back to a
reviewable structure — **without changing a single byte of any output**.

Specifically:

1. The drift-comparison logic exists three times in `eggpack-ci`; it becomes
   one function.
2. The staging-payload builder exists twice in `eggpack-github`, with 301 of
   352 lines identical; it becomes one function.
3. `render_release_github_inner` is an 845-line function; it becomes a set of
   named private helpers.
4. `crates/eggpack-ci/src/lib.rs` is 11,044 lines in one file; it becomes a
   module tree.

Every one of these is a pure consolidation. The milestone's evidence bar is
therefore simple and strict: **the outputs are byte-identical before and
after.**

## 2. Current evidence and the defects

### 2.1 The drift comparison is written three times

`crates/eggpack-ci/src/lib.rs` contains three public check functions:

| Function | Location | Renders |
|---|---|---|
| `check_github` | `:1906-1936` | `render_github` |
| `check_reusable_release_github` | `:3270-3300` | `render_reusable_release_github` |
| `check_release_github` | `:4149-4179` | `render_release_github` |

After the first line — which differs only in which renderer is called — the
bodies are **byte-identical text**: the `MAX_WORKFLOW_BYTES` bound, the
`normalize_newlines` CRLF-to-LF normalization of both sides, the equality
comparison, the first-difference byte offset computation, and the
`DriftReport` construction.

That is roughly 25 duplicated lines in three places. Drift checking is the
mechanism that guarantees a checked-in workflow matches its inputs, so it is
the last place where three copies of the same comparison should sit.

### 2.2 The staging-payload builder is written twice, 85.5% identically

`crates/eggpack-github/src/lib.rs` contains:

| Function | Location | Lines |
|---|---|---|
| `prepare_staging_payload` | `:665-1024` | 360 |
| `prepare_staging_payload_with_presentation_inner` | `:1360-…` | 409 |

A line-level diff of the two bodies: **301 of 352 lines are identical
(85.5%)**. The differing ~50 lines are the presentation-specific branch. The
identical part includes the manifest and contract validation, the product-id
cross-check, the release-identity check, the source-revision check, the
`reject_symlink_dir` call, the entire expected-inventory construction
(`expected: BTreeMap<String, (bool, u64, String)>`), the sidecar-shape checks,
the installer writes, the digest loop, the canonical sort, and the payload
assembly.

The doc comment at `crates/eggpack-github/src/lib.rs:1026-1029` claims the
`GeneratedDefault` presentation *"delegates to `prepare_staging_payload` with
byte-identical behavior."* The outer function does delegate. The inner function
does not — it duplicates. So the file simultaneously asserts and denies the
property, and the assertion is not enforced by anything.

This is the highest-risk duplication in the workspace, because a staging payload
is byte-sensitive release evidence: a future change applied to one copy and not
the other would produce two different staging payloads for the same inputs,
with no test necessarily detecting it.

### 2.3 The largest function in the workspace

`render_release_github_inner` (`crates/eggpack-ci/src/lib.rs:3302-4146`) is
**845 lines**. It performs policy validation, identity-mode resolution,
trigger cross-checks, tool-pin validation, emulated-runtime checks, and the
complete YAML emission, in one body. A change to any one of those concerns
requires reading all 845 lines to establish what else is affected.

### 2.4 The largest file in the workspace

`crates/eggpack-ci/src/lib.rs` is **11,044 lines** — 31% of the workspace's
35,383 Rust lines — holding 164 functions: the CIPlan projection, three
renderers, drift checking, Zig provisioning, consumer-validator execution,
release-identity resolution, and the entire test suite (tests begin at line
5,107).

For comparison, the other crates' largest files are `eggpack-cli/src/main.rs`
at 4,441 lines and `eggpack-github/src/lib.rs` at 3,268.

The CI crate's internal section banners (`crates/eggpack-ci/src/lib.rs:4806-4808`,
for example) already delimit logical sections, so the module boundaries are
indicated by the code itself.

## 3. Invariants

- **No output changes.** Every rendered workflow, every `DriftReport`, every
  staging payload, and every CLI output is byte-identical before and after this
  milestone. This is the milestone's primary invariant and its primary
  evidence.
- **No public API change** beyond additive internal restructuring. The
  `eggpack-ci` and `eggpack-github` public surfaces are unchanged, except
  where CI M003l already changed them.
- **No schema change.** `CIPlan` v1, `ReleaseCIPlanV1`, `ReleaseWorkflowShapeV1`,
  `GitHubPolicy`, `PackConfig` v1, and `ReleaseManifest` v1 are untouched.
- **No new dependency.**
- **Determinism is preserved.** The canonical sort, the stable serialization,
  and the byte-identical renders are unaffected. The consolidation moves code;
  it does not reorder a collection or change a format.
- The staging payload's **byte-identical equivalence** between the two
  presentations is preserved, and it becomes **enforced by construction** —
  one implementation, so there is no second copy to drift.
- The drift comparison's **CRLF-to-LF-only** normalization semantics are
  preserved exactly. This is a documented, deliberate behaviour
  (`architecture/ci-rendering.md`) and must not become a general whitespace
  tolerance.
- The `MAX_WORKFLOW_BYTES` bound is preserved in the shared comparison, so the
  bound applies uniformly to all three check functions.
- Source reorganization must not change the crate's public API, its feature
  flags, or its MSRV behaviour. `rust-version = 1.89` still holds.

## 4. Scope / non-goals

**In scope:**

- `crates/eggpack-ci/src/lib.rs` — the shared drift comparison, the
  decomposition of `render_release_github_inner`, and the split of the crate
  into modules.
- `crates/eggpack-github/src/lib.rs` — the shared staging-payload builder.
- the tests that prove byte-identity before and after.

**Out of scope:**

- CI M003l's rule convergence and CI M003m's process-execution convergence.
- Merging the three **renderers** (`render_github`,
  `render_reusable_release_github`, `render_release_github`). These produce
  genuinely different artifacts — a build workflow, a reusable release
  workflow, and a mapped release workflow — and were delivered by distinct
  milestones. **They are not duplicates and must not be merged.** Only their
  *comparison* is shared.
- Any change to what a rendered workflow contains.
- Any change to the staging payload's contents, ordering, or redaction.
- Moving code between crates.
- Any change to `eggpack-core`, `eggpack-contract`, `eggpack-manifest`,
  `eggpack-bootstrap`, or `eggpack-cli`.
- Any change to the generated workflow bytes or to the `ci _*` runner protocol.

## 5. Required production changes

### 5.1 One drift comparison

Extract the shared body of the three check functions into one private function
that takes the **already-rendered** expected bytes and the existing bytes, and
returns a `DriftReport`. The three public functions become one-liners that
render and then delegate.

The shared function keeps, in this order and unchanged:

1. the `MAX_WORKFLOW_BYTES` bound on the existing bytes;
2. `normalize_newlines` applied to **both** the expected and the existing bytes
   (CRLF to LF only);
3. the equality comparison;
4. the first-difference byte offset, computed over the normalized bytes, with
   the shorter length as the fallback;
5. the `DriftReport` construction with `expected_bytes`, `actual_bytes`, and
   `first_difference`.

Because the bound and the normalization are now applied once, all three check
functions are guaranteed to behave identically on identical inputs.

### 5.2 One staging-payload builder

Restructure `prepare_staging_payload_with_presentation_inner` so that the
shared algorithm is one function taking the presentation as a parameter, and
`prepare_staging_payload` calls it with the `GeneratedDefault` presentation.

The presentation parameter selects only the presentation-specific work. Every
other step — validation, inventory construction, sidecars, installers, digest
loop, canonical sort, payload assembly — exists once.

The doc comment at `crates/eggpack-github/src/lib.rs:1026-1029` is updated
from a claim about delegation to a statement about a single implementation.

### 5.3 Decompose `render_release_github_inner`

Split the 845-line function into named private helpers, one per concern it
currently interleaves:

- policy and shape validation;
- release-identity mode resolution;
- trigger and input cross-checks;
- tool-pin and emulated-runtime validation;
- workflow header and permission emission;
- job emission;
- step emission.

The helpers are private. The public `render_release_github` and
`render_reusable_release_github` signatures are unchanged, and the rendered
bytes are byte-identical.

### 5.4 Modularize the CI crate

Split `crates/eggpack-ci/src/lib.rs` into a module tree, using the crate's
existing section banners as the boundaries. The crate's public API is
re-exported identically from the crate root, so **no consumer of
`eggpack-ci` changes a single import**.

The split must keep the test module reachable in the same way it is today — the
crate's tests currently live inside `src/lib.rs` from line 5,107 onward, and
the `eggpack-github` crate's precedent of a separate `src/tests.rs` module
(`crates/eggpack-github/src/lib.rs:3268`, `crates/eggpack-github/src/tests.rs`)
is the model to follow.

### 5.5 No behaviour change

Every change in this milestone is a move. The closure record must show, for
each moved region, that the text is unchanged apart from the module boundary.

## 6. Ordered work packages

### WP1 — Capture the before-state as golden evidence

Before any edit, capture:

- the rendered output of `render_github`, `render_reusable_release_github`, and
  `render_release_github` for the existing fixtures, as byte-exact baselines;
- the `DriftReport` for a matching workflow, a CRLF-variant workflow, a
  workflow differing at the first byte, at the last byte, and beyond the end,
  and for an over-size workflow;
- the staging payload produced by both presentations for the existing fixtures,
  as byte-exact baselines.

These baselines become the milestone's evidence and are kept in the closure
record.

### WP2 — Failing discriminating tests first

- a test asserting that the two staging presentations produce byte-identical
  payloads. Today this passes only by coincidence of the two copies; the test
  must be written so that it would catch a divergence, and the closure record
  must show it passing **after** the consolidation.
- a test asserting that all three check functions return identical
  `DriftReport`s for identical (rendered, existing) byte pairs, across the
  CRLF, first-byte, last-byte, length-mismatch, and over-size cases.
- a test asserting that the drift comparison rejects an over-size existing
  workflow for all three functions, not only one.

### WP3 — Shared drift comparison

Extract the shared comparison and reduce the three public functions to
render-then-delegate. Verify byte-identity against WP1's baselines.

### WP4 — Shared staging-payload builder

Restructure the GitHub crate's builder. Verify byte-identity against WP1's
baselines for both presentations, and verify that the two presentations remain
byte-identical to each other.

### WP5 — Decompose `render_release_github_inner`

Extract the named private helpers. Verify byte-identity against WP1's
baselines.

### WP6 — Modularize the CI crate

Split the source file into modules, re-exporting the public API unchanged from
the crate root. Verify that `eggpack-cli`, `eggpack-github`, and the CI
crate's own tests compile and pass with **no import changes** — proven by
grepping the workspace for `eggpack_ci::` and confirming the consumer
imports are untouched by this milestone.

### WP7 — Verification and documentation

Run the full gate. Update `architecture/ci.md`, `architecture/ci-rendering.md`,
`architecture/github.md`, `architecture/overview.md` (the module map and the
line counts for the CI and GitHub crates), `crates/eggpack-ci/README.md`, and
`crates/eggpack-github/README.md`.

## 7. Failure, restart and contention semantics

- This milestone introduces no runtime behaviour, so it has no new failure
  modes. The existing failure semantics of every touched path are unchanged.
- Every consolidation step is independently revertible: each is a pure code
  move, and WP1's baselines make a bad move immediately detectable.
- A milestone that is interrupted mid-way leaves the workspace in a state where
  the baseline comparison fails loudly. It never leaves a workspace where a
  changed output is silently accepted.
- The consolidation touches no lock, no process, and no shared state.

## 8. Compatibility and migration

- **No output changes at all.** This is the milestone's defining constraint.
  Every rendered workflow, drift report, and staging payload is byte-identical.
- **No public API break.** `eggpack-ci`'s and `eggpack-github`'s public items
  are re-exported unchanged. `eggpack-ci` is not published on crates.io;
  `eggpack-github` is not published either. Both are workspace-internal.
- No schema change. No new dependency. No change to `Cargo.toml`.
- No change to `Cargo.lock`.
- `eggup-eggpack` and every consumer are unaffected.
- The `eggpack-cli` and `eggpack-github` crates' imports of `eggpack_ci` are
  unchanged; the closure record includes the grep evidence.

## 9. Required tests

- the byte-identity golden tests from WP1, for all three renderers, both
  staging presentations, and every `DriftReport` case;
- the staging-presentation equivalence test from WP2;
- the three-way drift-report equivalence test from WP2;
- the over-size bound test across all three check functions;
- the full existing `eggpack-ci` and `eggpack-github` suites, unchanged;
- a workspace-wide compile check proving no consumer import changed.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-github --all-targets --locked
./scripts/check-local.sh
```

Add a direct narrowed run for the new tests, e.g.
`cargo test -p eggpack-ci --all-targets --all-features --locked <new_test_name> -- --nocapture`.
Append `-- --test-threads=1` when diagnosing Windows serialization.

## 11. Documentation updates

- `architecture/ci.md` — the CI crate's module map.
- `architecture/ci-rendering.md` — one drift comparison; the three renderers
  remain three renderers, and that distinction is now explicit.
- `architecture/github.md` — one staging-payload builder.
- `architecture/overview.md` — the module map and the CI and GitHub line
  counts. Note that `architecture/overview.md`'s existing line counts and
  baseline SHA are already stale; this milestone updates only the counts for
  the crates it actually changes, and the full re-base remains the
  responsibility of
  `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`.
  This milestone re-verifies the citations in the sections it touches, by
  symbol name.
- `crates/eggpack-ci/README.md`, `crates/eggpack-github/README.md`.

## 12. Acceptance criteria

1. Exactly one drift-comparison implementation exists in `eggpack-ci`, used by
   all three check functions. Grep evidence is in the closure record.
2. Exactly one staging-payload builder exists in `eggpack-github`, used by
   both presentations.
3. `render_release_github_inner` is decomposed into named private helpers, each
   with a single concern.
4. `crates/eggpack-ci/src/lib.rs` is reduced to a crate root that re-exports
   its public API and contains no implementation body beyond the re-exports.
5. **Every rendered workflow, `DriftReport`, and staging payload is
   byte-identical** to the WP1 baselines. This is proven by test, and the
   baselines are reproduced in the closure record.
6. The public API of `eggpack-ci` and `eggpack-github` is unchanged; no
   consumer import changed.
7. No schema change, no dependency change, no `Cargo.lock` change.
8. The full workspace gate and `./scripts/check-local.sh` are green, and hosted
   CI is green on all lanes.
9. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- any output byte changes;
- the two staging presentations are found to be **intentionally** different in
  more than the presentation-specific region — that would mean the two
  functions encode different policies, not two renderings of one policy, and
  it must be understood before consolidating;
- the public API of either crate would change;
- the module split would require a consumer import change;
- the decomposition of `render_release_github_inner` would require changing
  the emitted YAML;
- a new dependency, a new schema, or a new command would be required.

## 14. Closure evidence

Write `plans/closure/ci-release-orchestration/003n-status.md` with: the
reviewed baseline and implementation SHA; the before/after duplication
inventory with the measured identical-line ratios; the WP1 golden baselines
reproduced in full; the byte-identity proof for every renderer, drift report,
and staging payload; the staging-presentation equivalence proof; the before and
after line counts for `crates/eggpack-ci/src/lib.rs` and
`crates/eggpack-github/src/lib.rs`; the grep output proving each retired
duplicate is gone; the grep output proving no consumer import changed; the
verification commands executed and their actual results; hosted CI run IDs and
the lane matrix; an invariant review against §3; unresolved findings by
severity; and the residual size of `crates/eggpack-cli/src/main.rs`, which this
milestone does **not** address and which the closure record names as a
remaining item.

The CI M001, M002, M003a, and M003d closure records remain historical and
unchanged.

## 15. Handoff notes

- Start from the CI M003m closure SHA.
- Capture WP1's baselines **before** the first edit and keep them out of the
  working tree if they would otherwise be picked up by `cargo fmt` or the
  linters. The closure record is where they belong.
- The three renderers are **not** duplicates. Do not merge them. The milestone
  shares their comparison, not their output.
- `crates/eggpack-cli/src/main.rs` (4,441 lines) and
  `crates/eggpack-core/src/qualification.rs` (2,621 lines, with a 372-line
  `qualify_target_for_host_with_runner`) remain unaddressed by this chain.
  Name them in the closure record's unresolved findings so the next planning
  pass sees them.
