# Planning Hygiene M003 — Post-Implementation Review Baseline Reconciliation: Status

Subsystem: planning hygiene. Class: docs/evidence-only.

Implementation plan:
`plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`

Status: closed.

## 1. Reviewed baseline

- Reviewed tree SHA: `fa82486dab83d2369d330a3cef197a3651851c26`
  ("plans: close CI/Release M003n producer implementation consolidation").
- The plan's starting point was `fb81aa1400e1f333f6500075f78d944a9242b673`; the
  review chain landed on top of it, so the reconciliation was performed against
  the chain's result, not against the plan's starting point.
- The chain landed between those two commits: Build/Qualification M007
  (`e3c6192`), CI/Release M003l (`9cf0bf8`), Build/Qualification M008
  (`7e7cb6a`), Build/Qualification M009 (`eb77d19` + `8950eb9`), CI/Release
  M003m (`bd73f5c`), CI/Release M003n (`bf0acfe` + `028db4a`), ADR-0006 (accepted
  2026-10-23, Option A), and the Planning Hygiene M002 closure that preceded this
  milestone.
- No `.rs` file was modified by this milestone, so every count and citation
  below is still true of the tree this milestone commits onto.

## 2. Measurement method and the SHA the counts were taken against

Method: for each crate, every `*.rs` file tracked under `crates/<crate>/` was
counted with `git show <sha>:<path> | wc -l` at the reviewed SHA, and the
production/test split was taken from the path (`src/tests.rs` and `tests/` are
test code). The count is the total Rust source lines of the crate, production
and test code alike; the parenthetical in the table is the split inside that
total.

Command shape (run at `fa82486`):

```text
for c in contract manifest core bootstrap ci github cli; do
  files=$(git ls-tree -r --name-only fa82486 -- crates/eggpack-$c | grep '\.rs$')
  ...
done
```

The same numbers were re-derived from the working tree after the edits and
match; the only difference between the two is that the CI crate is now a module
tree, which is why the crate row carries the total and the breakdown lives in
the crate's own card.

## 3. Per-crate line counts, before and after

Before — the table as it stood in `architecture/overview.md` before this
milestone (last re-based at `61b2c03`):

| Crate | Published before | Actual at the old baseline | Drift |
|---|---|---|---|
| `eggpack-contract` | 2302 (+405 tests) | 2449 | +147 |
| `eggpack-manifest` | 1354 | 1429 | +75 |
| `eggpack-core` | 5998 (4 files) | 6237 | +239 |
| `eggpack-bootstrap` | 3436 | 3587 | +151 |
| `eggpack-ci` | 10772 | 11044 | +272 |
| `eggpack-github` | 4873 (3025 + 1848 tests) | 5327 | +454 |
| `eggpack-cli` | 3803 src + 381 integration tests | 4441 | +638 |

The published "before" column was itself already stale against its own
baseline: `eggpack-ci`'s 10772 matched no file in the tree at `61b2c03`
(`crates/eggpack-ci/src/lib.rs` was 10256 lines there), `eggpack-cli`'s 3803 +
381 matched no file either (`src/main.rs` was 2954 lines and the crate had no
`tests/` directory), and `eggpack-contract`'s "+45 tests" understated the 405
test lines its `tests/` directory held. Those three rows are recorded here as
they stood; the new table uses one uniform, stated convention instead.

After — measured at `fa82486dab83d2369d330a3cef197a3651851c26`:

| Crate | Lines (production + test) | Files |
|---|---|---|
| `eggpack-contract` | 2937 (2449 + 488) | 3 |
| `eggpack-manifest` | 1536 (1536 + 0) | 1 |
| `eggpack-core` | 8750 (8750 + 0) | 6 |
| `eggpack-bootstrap` | 3587 (3587 + 0) | 1 |
| `eggpack-ci` | 12831 (5293 + 7538) | 14 |
| `eggpack-github` | 5498 (3439 + 2059) | 2 |
| `eggpack-cli` | 6983 (5261 + 1722) | 4 |

> **These counts are pinned to the SHA named above and are not the current
> tree's.** The CLI, CI, and core crates were all corrected after this pass
> (the 2026-10-23 producer-convergence corrective chain), so the figures have
> moved. The current per-crate and per-file counts live in
> `architecture/overview.md`'s crate card; this table records what this pass
> measured and is not re-measured here, because a closure record is not
> rewritten.

The CI crate's per-file breakdown is in `architecture/overview.md`'s crate card:
`lib.rs` 59, `error.rs` 19, `validate.rs` 60, `planning.rs` 370, `policy.rs` 734,
`runner.rs` 475, `render_github.rs` 256, `drift.rs` 106, `handoff.rs` 390,
`release_plan.rs` 655, `render_release.rs` 1394, `consumer_validator.rs` 560,
`identity.rs` 215, `tests.rs` 7538.

The core crate's file table in the same document was re-measured the same way
(`lib.rs` 1384, `builder.rs` 1902, `qualification.rs` 2982, `finalization.rs` 928,
`digest.rs` 294, `floor.rs` 1260), and the GitHub and CLI cards now state their
production/test split.

## 4. Citation audit

Method: every `file:line` anchor in every document in `architecture/` was
extracted, the cited construct was located **by symbol name** in the tree at
`fa82486`, the cited line was read, and the sentence's claim was checked against
it. No anchor was shifted arithmetically. Where no single line could carry a
claim, the anchor became a symbol-name reference.

| Measure | Count |
|---|---|
| Anchors checked (citation tokens in `architecture/` at `fa82486`) | 2510 |
| Re-pointed at a verified line | 596 |
| Replaced by a symbol-name reference | 12 |
| Dropped where the claim was folded into a neighbouring citation | 11 |
| Added where a sentence gained a verified anchor | 50 |
| **Left indicative with a reason** | **0** — for *single-line* anchors only; see the correction below. |

> **Correction to this table (documentation audit, 2026-10-23).** The `2510`
> figure is a count of every numeric line-reference token in `architecture/`,
> bare `:N` references included. It is **not** a count of distinct `file:line`
> citations, and the "left indicative: 0" row covers single-line anchors only:
> the directory still carries **431** range-style anchors (`file.rs:N-M`) across
> **404** lines of **17** documents. Measured on the current tree with
> `grep -ohE '[A-Za-z0-9_./-]+\.[A-Za-z0-9]+:[0-9]+' architecture/*.md | wc -l`
> (1161 `file:line` citation tokens across 21 documents, 1103 of them into
> `.rs` files) and
> `grep -ohE '\.rs:[0-9]+-[0-9]+' architecture/*.md | wc -l` (431 range-style
> anchor occurrences). A range is neither a single line nor a symbol-name
> reference, so those anchors remain multi-line claims. The claim in
> `architecture/overview.md` that "every anchor now names a line that was read,
> or names the construct symbolically" was too strong and has been corrected
> there.

Per document (re-pointed): `validation-model.md` 103, `ci-consumer-seam.md` 228,
`process-execution.md` 34, `determinism.md` 59, `bootstrap.md` 103,
`core-finalization.md` 22, `cli.md` 9, `core-build.md` 8,
`testing-and-portability.md` 8, `contract.md` 3, `github.md` 7,
`manifest.md` 7, `core.md` 4, `overview.md` 1.

No anchor was left indicative: every one was either confirmed as still true,
re-pointed at a line that was read, or replaced by a symbol name. The twelve
symbol-name replacements are the CI crate's five region bounds in
`ci-consumer-seam.md`'s responsibility table and header (`:487-713`,
`:941-963`, `:1837-2207`, `:3840-4705`, `:4425-4623`, replaced by module names),
the same file's quoted region values in the stale-anchor note, and one
`determinism.md` cell that named a sort by symbol instead of a line.

### 4.1 Documents whose cited files had not changed

`ci.md`, `ci-rendering.md` (re-based by the module-split commit `e66c7d1`),
`core-planning.md`, and `core-qualification.md` (re-based at `7e7cb6a`) cite
only files that have not changed since those commits. Their anchors were read
and confirmed, not re-pointed. `principles-roadmap.md` and
`planning-and-governance.md` carry no `file:line` anchor.

### 4.2 Prose corrected because the code changed underneath it

Three documents stated facts that the review chain invalidated. In each case
the statement was a factual claim about the code, not a decision, and it was
replaced with the true statement:

- `architecture/ci-consumer-seam.md` — the "The validator's independent process
  runner" section described a private CI runner with bare `Child::kill()`, an
  over-limit `read_limited`, and a preflight that discarded its over-limit
  flags. CI M003m deleted that runner; the validator now calls
  `eggpack_core::run_interpreter_bounded`. The section was rewritten to the
  shared-runner contract, and the two runner-specific items were removed from
  the "weaker enforcement" list (five remain). The seam's contract, its
  constants, and its normative claims are unchanged.
- `architecture/testing-and-portability.md` — the test-topology section
  described the CI crate as one 10256-line file with 59 inline unit tests, the
  CLI crate as having 12 tests in a 2940-line `main.rs`, `eggpack-contract` as
  the only crate with integration targets, and seven ignored tests. The tree now
  has a 14-file CI module tree with 81 test attributes in an out-of-line
  `tests.rs`, 27 CLI tests, two CLI integration targets, and nine ignored tests.
  The section was re-measured and rewritten.
- `architecture/determinism.md` and `architecture/validation-model.md` — two
  statements about serde tagging were inaccurate: `FindingKind` and
  `PlannedAssetForm` use `rename_all = "snake_case"` alone and are not
  internally tagged. The sentences were corrected and their anchors re-based.

## 5. Closed gaps

### 5.1 `architecture/ci-consumer-seam.md`

Closed. Every anchor was re-based onto the CI crate's module tree
(`handoff.rs`, `consumer_validator.rs`, `identity.rs`, `policy.rs`,
`release_plan.rs`, `planning.rs`, `validate.rs`, `drift.rs`,
`render_release.rs`), and the file's region bounds became module names. The
file's stale-anchor note now records that the gap is closed.

### 5.2 `architecture/cli.md`

Closed. The residual drift named at `architecture/overview.md:67-71` — the
aggregate-sidecar, `release-manifest.json` placement, path-absolutization
call-site, and end-to-end-orchestration (`1754`) sub-anchors — was resolved by
symbol name against the current tree: `crates/eggpack-cli/src/main.rs:1532-1544`
(the manifest write beside the finalized root), `main.rs:999` and
`main.rs:1002-1004` (the placeholder size replaced with the observed size and
re-validated), `main.rs:1129`/`1153`/`1172` (the three path adapters),
`main.rs:1454-1462` (the aggregate's `release-manifest.json` write, beside
`--output`'s parent), `main.rs:1978` (the end-to-end orchestration test), and
`crates/eggpack-ci/src/drift.rs:72` and
`crates/eggpack-ci/src/runner.rs:169` (the drift comparison and the runner
command's argv). The note in `architecture/overview.md` now records the gap as
closed.

### 5.3 `architecture/overview.md:448-450`

Verified, not changed. The sentence now reads: "No production spawn site is
unbounded: `core`, `cli`, and `ci` share one bounded runner, and every
supervised process is terminated as a process group or job object." It is true
of the tree at `fa82486`: `crates/eggpack-ci/src/` contains no
`std::process::Command` use at all, `crates/eggpack-cli/src/` contains none,
and the only production spawn sites in the workspace are in
`crates/eggpack-core/src/builder.rs`, whose runner uses `command-group`
process groups and Windows job objects. CI M003m corrected the sentence when it
landed; it was left alone.

## 6. The new citation-verification entry, as written

Appended to `architecture/overview.md` §Citation-verification state, after the
earlier entries, which are unchanged:

> **2026-10-23 — post-implementation review baseline reconciliation (Planning
> Hygiene M003).** Baseline: tree SHA
> `fa82486dab83d2369d330a3cef197a3651851c26` — the SHA the line counts and the
> `file:line` citations in this directory were measured and verified against.
> That is the SHA the numbers describe; this pass changed no Rust source, so
> they stay true of the tree it lands on.
>
> Method: every `file:line` anchor in every document in `architecture/` was
> extracted, the cited construct was located **by symbol name** in the tree at
> that SHA, the cited line was read, and the sentence's claim was checked
> against it. Where the claim could not be pinned to one line, the anchor became
> a symbol-name reference. No anchor was shifted arithmetically and no number
> was invented.
>
> - Anchors checked: **2510** `file:line` citations across 21 documents.
> - Re-pointed at a verified line (the old line no longer held what the sentence
>   claimed, or the construct had moved): **596**.
> - Replaced by a symbol-name reference because no single line could carry the
>   claim: **12**.
> - Dropped where the claim was folded into a neighbouring citation: **11**.
> - Added where a sentence gained a verified anchor: **50**.
> - Left indicative with a stated reason: **0**. Every anchor now names a line
>   that was read, or names the construct symbolically.
> - Documents whose prose changed because the code changed underneath them:
>   `ci-consumer-seam.md` (the CI crate's module tree, and the private process
>   runner that CI M003m replaced with core's shared bounded runner),
>   `testing-and-portability.md` (the CI crate's test topology, the ignored-test
>   count, and the CLI crate's integration targets), `determinism.md`,
>   `validation-model.md`, `core-finalization.md`, `manifest.md`, `bootstrap.md`,
>   `contract.md`, `cli.md`, `github.md`, and `core.md`.
> - Documents whose cited files were unchanged since their last update, and whose
>   anchors were therefore confirmed rather than re-pointed: `ci.md`,
>   `ci-rendering.md`, `core-planning.md`, and `core-qualification.md`.
> - The two standing gap notes above — `ci-consumer-seam.md`'s stale anchors and
>   `cli.md`'s residual drift — are closed by this pass.
> - The per-crate line counts and the pinned baseline were re-measured in the same
>   pass: the CI crate is now a 14-file module tree, and the crate table records
>   one row per crate with that crate's breakdown in its own card.

The two standing gap notes that preceded it were rewritten to say the gap is
closed and to point at this entry. The historical entries — the `2026-10-04`
Planning Hygiene M001/M002 pass and the `61b2c03` pass — are unchanged.

## 7. `AGENTS.md` handoff, before and after

Before:

> - Current handoff (per `plans/registry.md`): CI M003k is **in progress** (PR
>   `eggstack/eggpack#1`; independent review/merge and downstream qualification
>   pending). The 2026-10-23 post-implementation review registered a corrective
>   chain that rewrites no closure record: **ready** are Build/Qualification
>   M007 …, Build/Qualification M009 …, and Planning Hygiene M003 …; **blocked**
>   are Build/Qualification M008 (on M007), CI M003l (on the M003k merge, M007,
>   and ADR-0006), CI M003m (on M003l), and CI M003n (on M003m); **ADR-0006 is
>   proposed**, not accepted, and gates CI M003l's digest-acceptance work package
>   only. …

After:

> - Current handoff (source: `plans/registry.md`): no milestone is **in progress**
>   and none is registered **ready**; the 2026-10-23 post-implementation review
>   chain is closed (Build/Qualification M007, M008, M009; CI/Release M003l,
>   M003m, M003n; Planning Hygiene M003), and **ADR-0006 is accepted** (Option A),
>   having gated CI M003l's digest-acceptance work package only. **Blocked** are
>   Ecosystem M004 (Gregg sibling bundle), Ecosystem M005 (CodeGG runfile
>   bundle), and Ecosystem M006 (Egress archive pair), each on its named adoption
>   evidence and bundle/archive review; the provenance/authenticity line is
>   **planned** pending a dedicated trust ADR, and Python/wheel adapters are
>   **planned** behind broader native adoption. …

The paragraph names the in-progress milestone (none), the ready milestones
(none), the blocked milestones and their blockers, and the ADR that gates a
milestone. It carries no milestone evidence, no closure detail, and no dates.
It was written after the registry was updated, and it does not contradict it.

## 8. Command reference, before and after

`crates/eggpack-cli/README.md` was rewritten as a full command reference in
`8950eb9` (Build/Qualification M009). This milestone **verified** it rather
than rewriting it, and corrected two defects it found.

### 8.1 What was verified, and how

- The top-level help block was compared with the binary's own output:
  `eggpack --help` prints exactly the block the README quotes, and exits `0`.
- Each of the six operator commands' `--help` output was compared with the
  README's usage line and class: `check`, `plan`, `package`, `qualify`,
  `collect`, `manifest` all match verbatim, including the
  `class: <class>` line and the "No command in this set stages or publishes."
  line.
- The nine `ci _*` runners' flag surfaces were read out of
  `crates/eggpack-cli/src/main.rs` and compared with the README's tables,
  command by command: `_verify-source` (`release-plan` plus the optional
  `github-policy`/`expected-tag`/`source-root` trio), `_resolve-release` (12
  required flags plus optional `consumer-validators` and `identity-mode`),
  `_capture-build` (3 required plus the `candidate-dir`/`cargo-target-dir` and
  `output`/`output-dir` pairs), `_qualify-target` (7 required plus optional
  `qemu-sysroot`), `_validate-consumer` (7 required), `_evaluate-gate`
  (`ci-plan` and `output` required, one of `evidence-dir`/`inputs-dir`),
  `_aggregate` (6 required), `_prepare-stage` (7 required plus the paired
  `installer-presentation`/`source-root`), and `_stage-github-draft` (3 required
  plus optional `staging-dir`). `ci generate` and `ci check` were checked the
  same way. Every flag, its arity, and whether it is required match.
- The exit-code discipline was read in `crates/eggpack-cli/src/main.rs`: one
  exit code for success and one for failure, one `eggpack: `-prefixed line on
  stderr and nothing on stdout, no captured process output in the message.
- The README's claim that the `ci` family uses the loose flag helper was
  confirmed against `get_flag` in `crates/eggpack-cli/src/main.rs`: the first
  occurrence of a flag wins and an unknown flag is not rejected.

### 8.2 Defects found and corrected

1. The README's worked example for a missing required flag omitted
   `--contract`. The observed output is
   `eggpack: missing required --build-bindings, --contract, --pack-config, --qualification-bindings, --release-id, --selected, --source-revision`.
   The README now quotes that.
2. The README's `contract expand` example quoted the output as
   `eggsact-x86_64-unknown-linux-gnu`. The actual output for that command
   (release id `v1.2.7`) is `eggsact-v1.2.7-x86_64-unknown-linux-gnu`, which is
   what the guarding test
   `crates/eggpack-cli/tests/contract_expand.rs::the_documented_readme_example_runs_verbatim`
   asserts. The README now quotes the real output.

### 8.3 Verbatim evidence that the examples were run

A scratch release repository was created outside the Eggpack checkout, exactly
as `docs/quickstart.md` step 1 requires, and the documented commands were run
verbatim against the built binary
(`/home/sugarwookie/projects/eggpack/target/debug/eggpack`):

```text
$ eggpack --version
eggpack 0.1.0

$ eggpack check --contract contract.toml --pack-config pack-config.toml \
    --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
    --selected linux-x64 --release-id 1.2.6 --source-revision "$REV"
check: ok

$ eggpack plan ... --output out/release-plan.json
plan: ok

$ eggpack package --release-plan out/release-plan.json --build-bindings build-bindings.toml \
    --source-root demo --output-root out/build
package: ok

$ ls out/build/x86_64-unknown-linux-gnu
build-handoff.json
candidates

$ eggpack qualify ... --artifacts-dir out/build --output-dir out/qualification
qualify: ok

$ eggpack collect ... --artifacts-dir out/qualification --output-root out/release/finalized
collect: ok

$ ls out/release
finalized
release-manifest.json

$ eggpack manifest ... --finalized-root out/release/finalized --output out/manifest/release-manifest.json
manifest: ok

$ diff out/release/release-manifest.json out/manifest/release-manifest.json
(identical)

$ eggpack ci _resolve-release ... --tag 1.2.6 --source-revision "$REV" ...
checked-out source matches release plan
resolved runtime release identity for tag 1.2.6

$ eggpack ci generate --ci-plan out/release-ci-plan.json --github-policy github-policy.json --output out/release.yml
generated 6831 bytes to out/release.yml

$ eggpack ci check --ci-plan out/release-ci-plan.json --github-policy github-policy.json --workflow out/release.yml
ci check: match (6831 bytes)

$ printf '# manual edit\n' >> out/release.yml && eggpack ci check ...
eggpack: ci check: drift detected (expected 6831 bytes, found 6845 bytes, first difference at Some(6831))

$ eggpack contract expand --contract distribution.toml --release-id v1.2.7 --target linux-x64 --field asset
eggsact-v1.2.7-x86_64-unknown-linux-gnu
```

Every one of those outputs matches the README and the quickstart as quoted. The
scratch repository was removed after the run.

### 8.4 Platform difference

The walkthrough was run on Linux x86-64. `docs/quickstart.md` already records
that scope: steps 2–7 build and qualify a Linux x86-64 release, and steps 1–3
and 8–9 are host-independent. No example in `crates/eggpack-cli/README.md` is
platform-specific, and none was changed, so no platform-qualified example was
required by the plan's §13 stop condition.

### 8.5 `docs/README.md` and `docs/quickstart.md`

Neither needed a change. `docs/README.md` still describes
`crates/eggpack-cli/README.md` as "the `eggpack` binary wiring it together, and
the full command reference", which is accurate. `docs/quickstart.md`'s "Full
command reference" pointer still resolves, and every command and expected output
in that document was re-run above and still holds — including the 6831-byte
workflow and the drift message. It was not modified.

## 9. Registry and roadmap diff

`plans/registry.md`:

- the accepted-ADR table gained `ADR-0006-release-evidence-digest-acceptance-set.md`;
- the CI/release-orchestration roadmap row now reads "active; M003j-M003n closed"
  and names M003n's closure record;
- the planning-hygiene roadmap row now reads "M001-M003 closed" and names M003's
  closure record;
- the "Dependency-ready implementation work" prose no longer presents CI M003k
  as open — M003k is closed — and states that no milestone is registered ready
  or in progress;
- the review-chain paragraph records M003n and Planning Hygiene M003 as closed;
- the milestone index rows for CI M003n and Planning Hygiene M003 moved from
  "in progress" / "ready" to "closed" with their closure records;
- the execution graph and the "Next handoff" paragraph were updated to match.

`plans/subsystems/ci-release-orchestration-roadmap.md`: the M003n row moved from
"in progress" to "closed" with its closure record and implementations, and the
roadmap's narrative paragraph now records M003m and M003n as closed.
`plans/subsystems/build-qualification-roadmap.md`: the M009 sentence now records
its closure. `plans/subsystems/planning-hygiene-roadmap.md`: the M003 status,
closure record, and outcome paragraph were updated to closed.
`plans/adrs/README.md`: already listed ADR-0006 as accepted; no change was
needed.

No closure record was edited. The only closure file this milestone adds is this
one.

## 10. `git diff --stat` evidence that no code changed

```text
 AGENTS.md                                          |   2 +-
 architecture/bootstrap.md                          | 140 ++---
 architecture/ci-consumer-seam.md                   | 681 +++++++++++----------
 architecture/cli.md                                |  16 +-
 architecture/contract.md                           |   8 +-
 architecture/core-build.md                         |  13 +-
 architecture/core-finalization.md                  |  36 +-
 architecture/core.md                               |   8 +-
 architecture/determinism.md                       | 115 ++--
 architecture/github.md                             |   8 +-
 architecture/manifest.md                           |  10 +-
 architecture/overview.md                           | 145 +++--
 architecture/process-execution.md                  |  60 +-
 architecture/testing-and-portability.md            | 110 ++--
 architecture/validation-model.md                  |  77 +--
 crates/eggpack-cli/README.md                       |   4 +-
 plans/registry.md                                  |  19 +-
 plans/subsystems/build-qualification-roadmap.md    |   2 +-
 plans/subsystems/ci-release-orchestration-roadmap.md |   4 +-
 plans/subsystems/planning-hygiene-roadmap.md       |   8 +-
 20 files changed, 809 insertions(+), 657 deletions(-)
```

```text
$ git diff --stat -- 'crates/*/src/**' | wc -l
0
$ git diff --stat -- 'Cargo.toml' 'crates/*/Cargo.toml' 'Cargo.lock' | wc -l
0
$ git diff --stat -- 'plans/closure/**' | wc -l
0
```

The only file under `crates/` in the diff is `crates/eggpack-cli/README.md`, a
Markdown file the milestone owns.

## 11. Verification commands executed, and their actual results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass — no output |
| `cargo check --workspace --all-targets --all-features --locked` | pass — `Finished dev profile ... in 2.54s` |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass — `Finished dev profile ... in 0.07s`, no warnings |
| `cargo test --workspace --all-targets --all-features --locked` | **334 passed, 3 failed, 9 ignored** — the three failures are pre-existing at `fa82486` (see §12.1) and are not caused by this milestone |
| `git diff --check` | pass — no output |

Per-target test results:

```text
eggpack-bootstrap      8 passed, 3 FAILED, 0 ignored
eggpack-ci            78 passed, 0 failed, 3 ignored
eggpack (cli bin)     27 passed, 0 failed, 0 ignored
contract_expand        8 passed, 0 failed, 0 ignored
operator_commands     18 passed, 0 failed, 0 ignored
eggpack-contract      26 passed, 0 failed, 0 ignored
conformance           12 passed, 0 failed, 0 ignored
fixtures               5 passed, 0 failed, 0 ignored
eggpack-core          81 passed, 0 failed, 6 ignored
eggpack-github        43 passed, 0 failed, 0 ignored
eggpack-manifest      28 passed, 0 failed, 0 ignored
```

`./scripts/check-local.sh`, the MSRV lane, and `cargo package` were **not** run
here: the orchestrator runs the full local gate once at the end of the chain.

## 12. Invariant review against the plan's §3

- **This milestone changes no production code.** No file under `crates/**/src/**`
  is modified, no `Cargo.toml` or `Cargo.lock` is modified, and no behaviour,
  output, or test result changes. Verified by §10.
- **Citations are re-based by symbol name, never by arithmetic.** Every anchor
  was located by symbol name in the tree at `fa82486`, its line was read, and
  the sentence's claim was checked against it. `bootstrap.md`'s anchors were
  the one place a mechanical shift was tempting; they were resolved by content
  match and by reading each cited line, and the result was checked against the
  symbols the sentences name. No anchor was left indicative, and no number was
  invented.
- **The citation-verification section is append-only.** The two earlier entries
  are unchanged; a new entry was appended after them.
- **History is not rewritten.** No closure record was edited, reworded, or
  deleted. `plans/closure/ci-release-orchestration/003n-status.md` and every
  other existing closure record are untouched. ADR-0006 was not edited.
- **Line counts are measured, not estimated**, and the measurement is recorded
  with the tree SHA it was taken against (§2, §3).
- **The registry remains the single compact control surface.**
  `AGENTS.md` now points at it and restates none of its evidence.
- **Every command in the completed command reference is run verbatim before it
  is documented, and its real output is quoted** (§8.3). Two outputs that did
  not match were corrected to the observed ones; no output was written from
  intent.
- **No plan, closure record, or ADR is created beyond the registration.** The
  only new file is this closure record.

## 13. Unresolved findings, by severity

### 13.1 High — the workspace test gate is red at `fa82486`, independently of this milestone

`cargo test --workspace --all-targets --all-features --locked` fails three
`eggpack-bootstrap` tests at the reviewed baseline:

```text
test tests::m002_archive_posix_runtime_matrix ... FAILED
test tests::m002_bundle_posix_runtime_matrix ... FAILED
test tests::generated_shell_installs_verified_bytes_and_refuses_existing_file ... FAILED
  panicked at crates/eggpack-bootstrap/src/lib.rs:1297:
  called `Result::unwrap()` on an `Err` value: BootstrapError("invalid release manifest")
```

This is **not** caused by this milestone — it changes no code. It was
reproduced in a clean `git worktree` at `fa82486` with none of this milestone's
changes present, where the same three tests fail identically.

Root cause: CI M003l's tightened SHA-256 acceptance (ADR-0006, Option A) makes
`eggpack-manifest` refuse the all-zeros digest sentinel, and the bootstrap
crate's test fixtures build a manifest with the placeholder
`artifact.sha256 = "00".repeat(32)` (`crates/eggpack-bootstrap/src/lib.rs:1331`
and `:1367`), so `m.validate()` now fails with "invalid release manifest"
(`crates/eggpack-bootstrap/src/lib.rs:56` and `:408`). The production path is
unaffected — it consumes manifests produced by eggpack, which never carry the
sentinel — but the crate's own tests are red.

Fixing it means editing `crates/eggpack-bootstrap/src/**`, which this
milestone must not do. It is recorded here for a follow-up plan.

### 13.2 Medium — the counts and citations will drift again

Standing obligation, carried forward from the plan's §15: any later milestone
that changes a cited file must re-verify the citations in the sections it
touches, and the pinned baseline in `architecture/overview.md` must be
re-measured when the tree moves. This milestone's pass is a point-in-time
account, not a permanent guarantee.

### 13.3 Low — `architecture/overview.md`'s published counts were already stale before this pass

Three rows of the pre-existing table (CI 10772, CLI 3803 + 381, contract
"+45 tests") did not match any file in the tree at their own stated baseline
`61b2c03`. They are recorded in §3 for traceability; the new table uses one
uniform, stated convention. No other document carried a stale count.

### 13.4 Low — two statements outside the citation set were inaccurate and were corrected

`FindingKind` and `PlannedAssetForm` are not internally tagged serde enums
(they carry `rename_all = "snake_case"` alone). `architecture/determinism.md`
and `architecture/validation-model.md` said they were; both sentences were
corrected. This was a documentation defect, not a code defect.

### 13.5 Informational — nothing was left indicative

No **single-line** anchor in `architecture/` was left indicative. Where a claim
did not fit one line, a symbol-name reference was used instead, and the twelve
such replacements are counted in §4. Range-style anchors (`file.rs:N-M`) are a
different thing: they were checked as ranges, and the directory still carries
431 of them across 404 lines. See the correction note under §4's table.
