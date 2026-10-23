# Planning Hygiene M003 — Post-Implementation Review Baseline Reconciliation

Status: **ready for implementation**.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/planning-hygiene-roadmap.md`.
Primary class: documentation and evidence only. **No production change.**
Historical upstream authority: `AGENTS.md` §Quirks, which requires that
architecture citations be re-based **by symbol name, never by arithmetic**, and
that the current state and the outstanding gap be recorded in
`architecture/overview.md` §Citation-verification state.
Affected closed milestones: none. This milestone records and reconciles state;
it does not correct a closed milestone's behaviour and rewrites no closure
record.
Dependency: none. This milestone does not touch `crates/**` and does not
contend with CI M003k (in flight) or with any production milestone in the
review chain.

## 1. Objective

Reconcile the workspace's documentation and evidence surfaces with the code
they describe, after the post-implementation review that produced the corrective
chain registered alongside this milestone.

Four things are wrong today, all of them the same kind of thing — **the
documents state what was true at an earlier baseline, and the baseline moved**:

1. `architecture/overview.md` pins code baseline `61b2c03` and a per-crate
   line-count table measured against it. **All seven crates have drifted**, and
   the pinned baseline is 13 commits behind `HEAD`.
2. `architecture/overview.md` §Citation-verification state records a known
   outstanding gap — `ci-consumer-seam.md`'s stale anchors, and the residual
   `cli.md` drift — that has not been closed.
3. `AGENTS.md`'s current-handoff paragraph describes a state that predates CI
   M003j, M003k, and this entire review chain.
4. `docs/README.md` and `docs/quickstart.md` both promise that
   `crates/eggpack-cli/README.md` is the **full command reference**. It is 65
   lines and documents three commands; the nine `ci _*` runners' flag surface
   is described only in prose.

This milestone closes all four, and registers the review chain in the registry
and the subsystem roadmaps.

## 2. Current evidence and the gaps

### 2.1 The pinned baseline is stale

`architecture/overview.md:12-15`:

> *Code baseline `61b2c03` (the commit that bounds the `git` verification path
> recorded in [process-execution.md](process-execution.md)). Line counts below
> are measured against that baseline and were re-verified when this index was
> last touched — re-verify them against the workspace if you edit the overview
> again.*

`HEAD` is `fb81aa1400e1f333f6500075f78d944a9242b673`. `61b2c03` is
`61b2c0381494968aad441a82bbf7daa9e916d9fa`.

### 2.2 Every line count in the table has drifted

`architecture/overview.md:150-158` carries a per-crate line-count table.
Measured against `fb81aa1400e1f333f6500075f78d944a9242b673`:

| Crate | Table says | Actual | Drift |
|---|---|---|---|
| eggpack-contract | 2302 | 2449 | +147 |
| eggpack-manifest | 1354 | 1429 | +75 |
| eggpack-core | 5998 | 6237 | +239 |
| eggpack-bootstrap | 3436 | 3587 | +151 |
| eggpack-ci | 10772 | 11044 | +272 |
| eggpack-github | 4873 | 5327 | +454 |
| eggpack-cli | 3803 | 4441 | **+638** |

The `eggpack-github` and `eggpack-cli` figures are the most misleading: they
were measured before the CI M003d and Contract M003 changes respectively, and
both are now wrong by enough to mislead a reader who uses them to size a change.

### 2.3 The citation gap is recorded but not closed

`architecture/overview.md:53-58` states that `ci-consumer-seam.md` *"still
carries a known set of stale anchors into `cli/main.rs` and `core/builder.rs`,
both of which moved substantially in `61b2c03`. Treat its line references as
indicative until they are re-based."*

`architecture/overview.md:60-74` records that Contract M003 moved
`cli/src/main.rs` again, that `cli.md` was re-based for it, and that a
**residual set of pre-existing drift remains in `cli.md`** in sections M003 did
not touch — the aggregate-sidecar, `release-manifest.json` placement,
path-absolutization call-site, and end-to-end-orchestration (`1754`)
sub-anchors — which *"resolve to unrelated lines in the pre-M003 file as well,
so M003 carried them forward faithfully rather than inventing new numbers whose
intent it could not confirm."*

Those anchors are still wrong today.

### 2.4 The current handoff predates the in-flight milestone

`AGENTS.md:92` states that no Eggpack plan is `ready` and describes the state
through Ecosystem M003b (published as eggsearch `v0.4.2` on 2026-10-07) and
Planning Hygiene M002. Since then:

- CI M003j was closed (`plans/closure/ci-release-orchestration/003j-status.md`);
- CI M003k moved to **in progress** with PR `eggstack/eggpack#1` open
  (`plans/registry.md:99`, `:117`);
- ADR-0006 and the seven review-chain milestones in this pass were registered.

A grep across every `*.md` in the repository for `M003k` and `M003j` returns
hits **only under `plans/`**. Neither `AGENTS.md` nor `architecture/overview.md`
mentions either milestone. The document that agents are told to read first
describes a state that is three milestones out of date.

### 2.5 The "full command reference" is not one

`docs/README.md:29` describes `crates/eggpack-cli/README.md` as *"the full
command reference"*, and `docs/quickstart.md:212` says *"Full command
reference: `../crates/eggpack-cli/README.md`"*.

The file is 65 lines. It documents `eggpack ci generate`, `eggpack ci check`,
and `eggpack contract expand`, with one prose paragraph covering the nine
`ci _*` runners. The runners' actual flags — which are the workflow's entire
invocation protocol — are not written down anywhere.

## 3. Invariants

- **This milestone changes no production code.** No file under `crates/**` is
  modified. No behaviour, output, or test result changes.
- **Citations are re-based by symbol name, never by arithmetic.** Every
  `file:line` citation is verified by **reading the line and confirming it
  holds what the sentence claims**. A citation whose intent cannot be
  confirmed is marked indicative with a reason, or removed — it is never
  replaced by a guessed number.
- **The citation-verification section is append-only.** It is a running
  account of what was checked and when. Earlier entries stay; a new entry is
  added.
- **History is not rewritten.** No closure record is edited, reworded, or
  deleted. A closed milestone's record stays as it was written.
- Line counts are **measured**, not estimated, and the measurement is recorded
  with the tree SHA it was taken against.
- The registry remains the **single compact control surface** for milestone
  status. `AGENTS.md` points at it rather than restating it.
- Every command in the completed command reference is **run verbatim** before
  it is documented, and its real output is quoted.
- No plan, closure record, or ADR is created by this milestone beyond the
  registration it performs.

## 4. Scope / non-goals

**In scope:** `architecture/`, `AGENTS.md`, `docs/`,
`crates/eggpack-cli/README.md`, `plans/registry.md`,
`plans/subsystems/*-roadmap.md`, `plans/adrs/README.md`.

**Out of scope:**

- Any file under `crates/*/src/**` and any crate `Cargo.toml`.
- Any closure record, past plan, or accepted ADR.
- Any change to a document's **substance** — this milestone corrects what a
  document *says about the code*, not what it *decides*. Where a document's
  substance is wrong, the milestone records the discrepancy as a finding for a
  follow-up plan rather than rewriting the decision.
- The `ci-consumer-seam.md` **prose** and the constants it names. Only its
  line anchors are re-based.
- The `architecture/` documents' authority structure, the module map's
  substance, or the invariant list. Only factual claims about the code are
  corrected.
- Any new plan beyond the registration of the already-written review chain.

## 5. Required changes

### 5.1 Re-base the architecture baseline and line counts

- Set `architecture/overview.md:12` to the tree SHA the counts were measured
  against, and state plainly that it is the SHA the citations were verified
  against.
- Re-measure every line count in the `architecture/overview.md:150-158` table
  against that SHA and update it.
- Re-verify every `file:line` citation across every document in
  `architecture/`, **by symbol name**. Record the method and the result.
- Re-base `ci-consumer-seam.md`'s stale anchors.
- Re-base the residual drift in `cli.md` named at `architecture/overview.md:67-71`.
- For any citation whose intent cannot be confirmed: replace it with a
  **symbol name reference** rather than a line number, or mark it indicative
  with the reason. Do not invent a number.

### 5.2 Append to the citation-verification state

Add a new entry to `architecture/overview.md` §Citation-verification state
describing this pass: the baseline, the method, the number of anchors checked,
the number corrected, the number replaced with symbol names, and the number
left indicative with a reason. Leave the earlier entries intact.

### 5.3 Correct the false statement that M003m will falsify

`architecture/overview.md:326-328` states that *"`ci`'s independent
reimplementation is the only site without process-group kill."* CI M003m
corrects the code. This milestone updates the sentence **only after** M003m has
landed, or — if M003m has not landed — leaves it in place and records the
pending dependency in the entry added by §5.2.

The statement must never be left describing code that no longer exists, and it
must not be changed to describe code that does not exist yet.

### 5.4 Rewrite the current handoff

Rewrite `AGENTS.md:92` so that it is a **single statement sourced from the
registry**, in the style Planning Hygiene M002 established. It must name:

- the one milestone in progress (CI M003k) and its blocking relationship;
- the milestones that are ready;
- the milestones that are blocked, and on what;
- the ADR that is proposed and gates a specific milestone.

It must **not** restate milestone evidence, closure detail, or dates — those
belong to `plans/registry.md`. The paragraph must not claim that no plan is
ready if the registry lists any.

### 5.5 Complete the command reference

Rewrite `crates/eggpack-cli/README.md` as an actual full command reference:

- every command, with its class (validation / build / staging / publication /
  internal runner);
- every flag, its arity, and whether it is required;
- the exit-code discipline;
- one worked example per command family, with **real** output.

Every example is produced by **running the command** and quoting its actual
output. Nothing is written from intent.

The nine `ci _*` runners are documented with their **exact** flag surface,
because that surface is the generated workflow's invocation protocol and is
currently undocumented.

If Build/Qualification M009 has landed before this milestone, the six new
operator commands are documented here too, from their real behaviour. If it has
not, this milestone records their absence explicitly rather than leaving the
"full command reference" claim quietly false.

### 5.6 Register the review chain

Update `plans/registry.md`:

- add a row per new milestone with its status, its blockers, and its
  dependency;
- add ADR-0006 to the ADR list;
- update the dependency-ready table;
- update the planned/blocked table so the review chain is visible as a chain.

Update the affected subsystem roadmaps:

- `plans/subsystems/build-qualification-roadmap.md` — M007, M008, M009;
- `plans/subsystems/ci-release-orchestration-roadmap.md` — M003l, M003m, M003n;
- `plans/subsystems/planning-hygiene-roadmap.md` — M003.

Update `plans/adrs/README.md` to list ADR-0006.

Each roadmap row states the milestone's status and its blocker. No roadmap
narrative is rewritten.

## 6. Ordered work packages

### WP1 — Register first, so the documents describe a settled state

Register ADR-0006 and the seven review-chain milestones in the registry, the
roadmaps, and the ADR index, and rewrite `AGENTS.md:92`. Doing this first
means every later document edit is made against a registry that already
reflects the chain.

### WP2 — Re-base the architecture citations

Re-verify every citation across `architecture/` by symbol name. Update the
counts and the baseline. Append the citation-verification entry.

### WP3 — Re-base `ci-consumer-seam.md` and the residual `cli.md` drift

Close the two gaps the overview already names.

### WP4 — Complete the command reference

Rewrite the CLI README as a real full command reference, with every example run
verbatim and its real output quoted.

### WP5 — Verify and record

Confirm that `crates/**` is untouched, that no closure record changed, and
that every registered plan and ADR referenced from a document exists at the
path the document names.

## 7. Failure, restart and contention semantics

- Not applicable in the usual sense: this milestone changes no runtime
  behaviour and has no failure modes of its own.
- The milestone's own failure mode is a **wrong re-base**: a citation moved to
  a line that does not hold what the sentence claims. The defence is the
  by-symbol-name rule and the rule that an unconfirmable citation becomes a
  symbol reference or is marked indicative, never a guessed number.
- The milestone is idempotent: re-running it against an unchanged tree changes
  nothing.
- It does not contend with CI M003k, because it touches no file under
  `crates/**`. If a production milestone lands while this one is in flight,
  the counts and citations it verified must be re-verified against the new
  tree — which is the standing rule `architecture/overview.md:14-15` already
  states.

## 8. Compatibility and migration

- **No production change.** No crate is affected. No test result changes. No
  output changes.
- No schema change. No dependency change. No `Cargo.lock` change.
- The CI workflow is unaffected: this milestone edits no file the workflow
  reads.
- `AGENTS.md`'s handoff paragraph changes, and `AGENTS.md` is an agent-facing
  contract rather than a published artifact. The change makes it accurate
  rather than altering any commitment it makes.
- `crates/eggpack-cli/README.md` changes, and it is rendered on crates.io only
  for the published `eggpack-manifest` crate. `eggpack-cli` is not published,
  so the change has no downstream effect.
- The review-chain plans reference `plans/implementation/...` paths. This
  milestone confirms that every path they name exists.

## 9. Required checks

Because this milestone changes no code, the required checks are documentary:

- `git diff --stat` shows **no** change under `crates/*/src/**` and **no**
  change to any `crates/*/Cargo.toml`;
- `git diff` over `plans/closure/**` is **empty** — no closure record is
  rewritten;
- every `file:line` citation in `architecture/` is confirmed by reading the
  line, and the confirmation method is recorded;
- every plan and ADR path referenced from any updated document exists;
- every command example in the rewritten CLI README was produced by running
  the command, and the quoted output matches what the command actually prints;
- `cargo test --workspace --all-targets --all-features --locked` is unchanged
  and green, proving the milestone did not touch code.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
./scripts/check-local.sh
```

Plus the documentary checks in §9.

## 11. Documentation updates

This milestone **is** the documentation update. The documents it touches:

- `architecture/overview.md` — baseline, counts, citation-verification entry;
- `architecture/ci-consumer-seam.md` — anchors only;
- `architecture/cli.md` — the residual drift named at
  `architecture/overview.md:67-71`;
- `AGENTS.md` — the current-handoff paragraph;
- `docs/README.md` — the "full command reference" description, if the
  reference is now genuinely full;
- `docs/quickstart.md` — the "Full command reference" pointer, if it moves;
- `crates/eggpack-cli/README.md` — the full command reference;
- `plans/registry.md`, `plans/subsystems/*-roadmap.md`,
  `plans/adrs/README.md` — the registration.

## 12. Acceptance criteria

1. `architecture/overview.md`'s pinned baseline names a tree SHA that is the
   SHA the citations and counts were verified against, and every line count in
   its table matches the tree at that SHA.
2. Every `file:line` citation in `architecture/` has been verified by reading
   the line. Corrections, symbol-name replacements, and indicative markings are
   each counted in the appended citation-verification entry.
3. `ci-consumer-seam.md`'s known stale anchors and the residual `cli.md` drift
   named at `architecture/overview.md:53-58` and `:67-71` are closed, or — if
   an anchor's intent cannot be confirmed — replaced with a symbol-name
   reference with the reason recorded.
4. The citation-verification section contains a new entry for this pass, and
   every earlier entry is unchanged.
5. `AGENTS.md`'s current-handoff paragraph is a single statement sourced from
   the registry, naming the in-progress milestone, the ready milestones, the
   blocked milestones and their blockers, and the proposed ADR. It does not
   restate evidence or dates, and it does not contradict the registry.
6. `crates/eggpack-cli/README.md` documents every command and flag, with real
   output quoted from actual runs, including the `ci _*` runners' exact flag
   surface. The "full command reference" description is accurate.
7. ADR-0006 and all seven review-chain milestones are registered in
   `plans/registry.md`, the three affected subsystem roadmaps, and
   `plans/adrs/README.md`, with statuses and blockers.
8. `git diff --stat` shows no change under `crates/*/src/**`, no change to any
   `Cargo.toml`, and no change under `plans/closure/**`.
9. The full workspace gate and `./scripts/check-local.sh` are green.
10. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- re-basing a citation would require changing the **substance** of a document —
  record it as a finding instead;
- a citation's intent cannot be confirmed and no symbol-name reference is
  available — record it as indicative with a reason, and do not guess;
- the review-chain milestones it must register are changed by another
  concurrent plan — re-read the registry before registering;
- registering the chain would require editing a closure record — it must not;
- a command example cannot be produced on all four supported platforms — the
  quickstart's verified-command contract applies to `docs/quickstart.md`, and
  the CLI README must either show a platform-qualified example or record the
  platform difference explicitly.

## 14. Closure evidence

Write `plans/closure/planning-hygiene/003-status.md` with: the reviewed
baseline; the measurement method for the line counts and the SHA they were
taken against; the per-crate count table before and after; the citation
audit — how many citations were checked, corrected, replaced with symbol
names, and left indicative with reasons; the closed `ci-consumer-seam.md` and
`cli.md` gaps; the new citation-verification entry as written; the `AGENTS.md`
handoff before and after; the command-reference before and after and the
verbatim evidence that each example was run; the registry and roadmap diff;
the `git diff --stat` evidence that no code changed; the verification commands
executed and their actual results; an invariant review against §3; and
unresolved findings by severity — in particular, any citation that had to be
left indicative, and the standing obligation that any later milestone which
changes a cited file re-verifies the citations in the sections it touches.

## 15. Handoff notes

- Start from `fb81aa1400e1f333f6500075f78d944a9242b673`.
- Register first (WP1), then re-base (WP2, WP3). The registry should describe
  the chain before the documents that mention it.
- **Re-base by symbol name.** Read the line, confirm the sentence is true of
  it, and only then write the number. A blanket arithmetic shift across the
  deep dives is exactly what corrupted them the last time; the `61b2c03` pass
  recorded at `architecture/overview.md:28-33` is the precedent, and its method
  — verified anchor by anchor by name — is the method to repeat.
- The counts will drift again as the review chain lands. That is the standing
  condition, not a failure of this milestone; it is recorded in the appended
  citation-verification entry and repeated in the closure record's unresolved
  findings.
