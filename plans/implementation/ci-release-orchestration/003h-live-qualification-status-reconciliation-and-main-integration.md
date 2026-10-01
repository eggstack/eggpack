# CI and Release Orchestration Milestone 003h — Live Qualification Status Reconciliation and Main Integration

Status: ready / not started

Repository baselines:

- integration base (`main`): `3f95af43f99224755c97161e0c1102a84e713e35`;
- qualified candidate branch: `m003g-live-qualification@8507fbeebc6e6a0f8176965d8b21dfc818a03719`;
- relationship at planning time: candidate branch is 10 commits ahead of `main`, 0 behind, so a fast-forward integration is possible if `main` remains an ancestor.

Primary roadmaps:

- `plans/subsystems/ci-release-orchestration-roadmap.md`;
- `plans/subsystems/ecosystem-adoption-roadmap.md`.

Related closure evidence:

- `plans/closure/build-qualification/006-status.md`;
- `plans/closure/ci-release-orchestration/003e-status.md`;
- `plans/closure/ci-release-orchestration/003f-status.md`;
- `plans/closure/ci-release-orchestration/003g-status.md`;
- `plans/closure/ecosystem-adoption/001-status.md`.

Related consumer evidence:

- eggsact release `v1.2.7`;
- live Eggpack-generated release run `36652731202`;
- consumer-owned Windows reproducibility follow-up: eggsact M005a.

Primary class: planning/status corrective + qualified-branch integration

## 1. Why this milestone exists

The real eggsact qualification work on `m003g-live-qualification` materially
advanced Eggpack beyond current `main`:

- CI M003e corrected generated-workflow execution wiring;
- CI M003f corrected the generated `cargo install --git` command;
- CI M003g corrected live cross-tool, transfer/executable, validation, and
  staging defects;
- a maintainer-authorized eggsact `v1.2.7` release exercised the generated
  five-target workflow end to end and staged a complete 15-asset draft;
- Ecosystem M001 is closed on that live consumer evidence.

The candidate branch itself is qualified, but its planning control surfaces
were updated incrementally during live debugging. They now contain mutually
inconsistent status statements even though the closure records are clear.
Examples at baseline `8507fbe` include:

- `plans/registry.md` says CI M003c-M003g are closed in the subsystem summary,
  but separately lists M003g as `active`;
- the same registry says Ecosystem M001 is both `ready (unblocked)` and
  `closed`;
- its immediate execution graph still labels eggsact M001 `READY`;
- its M003b blocker text still says the live proof has not run and M001 must
  re-review/re-pin to M003e, despite the live `v1.2.7` evidence and later
  M003f/M003g producer pins;
- its downstream disposition still says M001 has not closed and leaves M002
  blocked on M001 closure;
- `plans/subsystems/ci-release-orchestration-roadmap.md` still describes the
  M003b live-draft condition as outstanding in narrative/table text even though
  M003g records the real draft; only exact byte-identical rerun reuse remains;
- `plans/subsystems/ecosystem-adoption-roadmap.md` retains pre-implementation
  M001 dependency/status narrative while its milestone table correctly marks
  M001 closed.

Those are planning/status defects, not new producer defects. Integrating the
branch onto `main` without reconciling them would make the canonical control
surface ambiguous immediately after a major live qualification.

## 2. Why ready

All hard dependencies are satisfied:

1. `main@3f95af43` is an ancestor of the candidate branch at planning time;
2. M006 is closed under accepted ADR-0005 Option A;
3. M003e, M003f, and M003g each have closure records and hosted cross-platform
   CI evidence;
4. candidate tip `8507fbe` passed repository CI (run `36722372028`);
5. M003g records the real eggsact `v1.2.7` live run and the complete
   15-asset draft;
6. Ecosystem M001 has a closure record on the candidate branch;
7. no open pull request or competing integration branch is required for this
   bounded reconciliation.

The remaining Windows byte-reproducibility issue is explicitly consumer-owned
by eggsact M005a. Eggpack's same-name/different-digest refusal is the expected
fail-closed behavior and is not a blocker to reconciling or integrating the
qualified Eggpack branch.

## 3. Objective

Produce one internally consistent, fully qualified Eggpack branch whose code,
closure records, registry, subsystem roadmaps, and immediate execution graph
all describe the same post-live-qualification state, then move `main`
forward to that exact state without a merge commit or history rewrite.

## 4. Authoritative post-reconciliation state

The implementation MUST converge on the following status model:

- Build/Qualification M001-M006: **closed**;
- CI M003e: **closed**;
- CI M003f: **closed**;
- CI M003g: **closed**;
- Ecosystem M001 eggsact adoption: **closed**;
- CI M003b: **conditionally closed**;
  - real draft creation, exact asset inventory, draft-only behavior, and
    no-clobber refusal are proven;
  - exact rerun reuse remains outstanding only because the eggsact Windows
    artifact is not byte-reproducible;
- Phase 8: **not exited / blocked on rerun-reuse evidence** owned by eggsact
  M005a, not by an unresolved Eggpack producer defect;
- Ecosystem M002 stegoeggo: **ready to plan**;
- Bootstrap M003: remains **blocked** on independent adoption
  evidence/candidate review;
- Eggup Interoperability M003: remains **ready to plan** independently;
- the M006 glibc compatibility-floor evidence boundary remains unchanged and
  unclaimed.

No status may imply that M001 still needs its consumer baseline re-review,
tool re-pin, or first live release: those events already occurred in the
candidate branch evidence.

## 5. Invariants

- do not rewrite historical closure records to hide the incremental defects
  discovered by the live run;
- do not weaken or reclassify the consumer-side Windows reproducibility finding
  as an Eggpack defect;
- do not claim full CI M003b/Phase 8 closure until an exact rerun can reuse all
  assets without clobber;
- do not claim that the `v1.2.7` rerun succeeded: attempt 2 correctly failed
  closed on the differing Windows asset;
- do not modify production Rust code as part of this milestone;
- do not regenerate CI goldens or alter release semantics;
- do not change ADR-0005, schema versions, publication authority, or the
  Eggpack/Eggup ownership boundary;
- keep all live run ids, implementation SHAs, consumer release/tag identity,
  and unresolved-condition ownership exact;
- integrate `main` only by a non-forced fast-forward to a fully verified
  descendant.

## 6. In scope

### 6.1 Registry reconciliation

Correct `plans/registry.md` so every section agrees with §4:

- active subsystem summary;
- dependency-ready implementation table;
- planned/blocked work;
- current evidence narrative;
- immediate execution graph;
- next handoff;
- downstream-unblock disposition.

Remove or replace stale statements that M003g is active, M001 is ready/not
closed, M003b still awaits its first live draft, or M002 is blocked solely on
M001 closure.

The registry should identify M003h itself as the active/ready integration
corrective until this milestone closes.

### 6.2 CI/release-orchestration roadmap reconciliation

Update `plans/subsystems/ci-release-orchestration-roadmap.md` so:

- M003e/M003f/M003g are closed consistently in both narrative and tables;
- M003b is conditionally closed with the *remaining* condition stated
  precisely as byte-identical rerun reuse;
- the real `v1.2.7` live draft evidence is referenced;
- Phase 8 remains open because the exact rerun criterion has not yet been met;
- the consumer-owned Windows determinism dependency is explicit.

### 6.3 Ecosystem-adoption roadmap reconciliation

Update `plans/subsystems/ecosystem-adoption-roadmap.md` so:

- M001 is closed consistently in narrative and tables;
- the historical M006/M003e stop/restart sequence remains traceable but is not
  presented as current state;
- M002 becomes ready to plan from the first-consumer evidence;
- later milestones remain dependency-ordered behind M002 as appropriate;
- the Windows rerun condition remains a Phase 8/M003b condition and does not
  reopen M001.

### 6.4 M003b status annotation

Do not rewrite the historical M003b closure body. Add a clearly dated
post-M003g annotation to the M003b implementation plan and/or closure record
only if needed to make the current condition discoverable:

- live draft evidence is now satisfied by eggsact `v1.2.7`;
- exact rerun reuse remains outstanding because one consumer artifact changes
  bytes;
- M003b therefore remains conditionally closed;
- the producer behavior correctly refuses clobber.

This annotation must preserve the historical statement that no live draft
existed at the original M003b conditional close.

### 6.5 Integration verification and `main` fast-forward

After documentation/status reconciliation:

1. assert current `main` is still an ancestor of the candidate branch;
2. inspect the complete `main...candidate` diff and commit list;
3. prove the M003e/M003f/M003g production commits and closure commits are all
   present exactly once;
4. run/observe the full repository verification on the final candidate tip;
5. require hosted CI success on that exact tip;
6. update `main` to the exact candidate tip with a non-forced fast-forward;
7. verify `main` resolves to the same SHA and its CI is green.

If `main` advances independently before step 1, stop. Rebase/merge
reconciliation requires a new reviewed baseline; do not force-update
`main`.

## 7. Out of scope

- fixing eggsact Windows PE/PDB reproducibility (eggsact M005a);
- changing M003b's exact-rerun acceptance criterion;
- new release functionality or provider behavior;
- new target/build/qualification capability;
- independent ELF GLIBC-symbol-floor verification;
- stegoeggo implementation itself;
- deleting historical branches after integration;
- squashing or rewriting the qualified M003e/M003f/M003g history.

## 8. Required work packages

### WP1 — Freeze and audit the candidate

Record:

- current `main` SHA;
- current candidate SHA;
- compare result (`ahead_by`, `behind_by`);
- candidate-tip CI;
- all M003e/M003f/M003g implementation and closure SHAs;
- live eggsact run/tag evidence.

Stop if the candidate is no longer a strict descendant of `main`.

### WP2 — Build a status contradiction matrix

For each affected file, enumerate every stale statement and its corrected
state. At minimum cover:

- `plans/registry.md`;
- `plans/subsystems/ci-release-orchestration-roadmap.md`;
- `plans/subsystems/ecosystem-adoption-roadmap.md`;
- M003b current-condition annotation if needed.

The matrix belongs in the M003h closure record so later reviewers can see
which statements were corrected without diff archaeology.

### WP3 — Apply documentation-only reconciliation

Make only planning/documentation edits. Verify:

- `git diff --stat` contains no Rust source, Cargo manifest/lockfile,
  workflow, fixture/golden, or script changes;
- closure records M003e/M003f/M003g and Ecosystem M001 remain intact;
- historical statements are preserved or explicitly annotated rather than
  silently rewritten.

### WP4 — Static consistency checks

Search the final tree for stale state phrases and require zero unqualified
matches for current-state claims such as:

- M003g `active`;
- M001 `ready` / `unblocked` as its current status;
- M001 `has not closed`;
- M003b `live draft outstanding` without the rerun-reuse qualification;
- M002 `blocked on M001 closure`;
- instructions to re-pin to M003e/M003f as future work.

Historical narrative may contain those strings only when explicitly marked as
historical.

### WP5 — Repository verification

Because the candidate includes production changes not yet on `main`, run the
normal full verification even though M003h itself is documentation-only:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Also require hosted CI on the final candidate tip to pass all repository lanes.

Do not re-run a public release merely to close M003h. The live release evidence
is already recorded by M003g/M001; this milestone validates integration and
status correctness.

### WP6 — Fast-forward `main`

Immediately before updating `main`:

- fetch both refs again;
- require `main` to be an ancestor of the final candidate;
- require `behind_by == 0` for the candidate relative to `main`;
- require the final candidate CI to be green.

Then advance `main` to the exact candidate SHA with force disabled.

After the ref move:

- verify `main` equals the candidate SHA;
- verify the branch-to-main compare reports no divergence;
- verify the planning files on `main` carry the reconciled statuses;
- observe the `main` push CI if a new run is emitted.

## 9. Failure, restart, and contention semantics

- **Main advanced independently:** stop before any ref move. Re-review the new
  main baseline and reconcile normally; never force.
- **Candidate CI red:** do not integrate. Any production defect requires a new
  corrective plan; a documentation-only defect may remain within M003h if it
  does not alter producer behavior.
- **Status disagreement with closure evidence:** closure evidence and actual
  Git/live-run history win; correct the registry/roadmap, do not rewrite the
  evidence to match stale summaries.
- **Unexpected production diff during WP3:** stop and split it into a new
  corrective milestone.
- **Ref update races:** re-fetch refs and re-check ancestry; never retry with
  force.
- **Partial planning edit:** continue on the candidate branch only; `main`
  remains untouched until all acceptance criteria pass.

There is no shared runtime state, migration, or service restart in this
milestone.

## 10. Compatibility and migration

No public schema/API/runtime behavior is intentionally changed by M003h.

The integration brings already-qualified production changes from the
M003e/M003f/M003g chain onto `main` without squashing. Git history remains a
linear audit trail from `main@3f95af43` through each live corrective and its
closure evidence.

No consumer migration is performed by this plan. Eggsact M001 is already
closed on the candidate branch; stegoeggo M002 becomes the next planning
candidate after integration.

## 11. Security and release-safety review

The reconciliation MUST preserve the security conclusions already qualified by
M003g:

- artifact-transfer exec-bit restoration occurs only after byte identity
  verification and does not alter digest-covered bytes;
- non-`Passed` qualification evidence is refused by consumer validation;
- staging remains draft-only in generated automation;
- same-name/different-digest remote assets fail closed rather than clobber;
- only staging receives release write permission;
- staging diagnostics do not expose token/body/URL secrets;
- the Windows rerun mismatch remains visible and unresolved rather than being
  waived for status closure.

The main ref update itself must be non-forced.

## 12. Acceptance criteria

M003h may close only when all are true:

- the final candidate remains a descendant of the reviewed main baseline;
- no production file changes are introduced by M003h itself;
- registry, CI roadmap, ecosystem roadmap, and immediate execution graph all
  agree with §4;
- M003g is closed everywhere;
- Ecosystem M001 is closed everywhere;
- M003b is consistently conditionally closed with exact rerun reuse as the
  remaining condition;
- Phase 8 remains open for that same condition;
- Ecosystem M002 is ready to plan;
- Bootstrap M003 and Eggup Interop M003 retain their independent statuses;
- stale current-state phrases in WP4 are eliminated or explicitly historical;
- full local verification passes;
- hosted CI passes on the final candidate tip;
- `main` is advanced by non-forced fast-forward to exactly that tip;
- post-integration compare shows no divergence;
- a closure record is written at
  `plans/closure/ci-release-orchestration/003h-status.md` with the
  contradiction matrix, verification results, final SHA, ref-move evidence,
  and remaining external/consumer conditions.

## 13. Stop conditions

Stop and re-plan if:

- `main` is no longer an ancestor of the candidate branch;
- reconciling status requires changing an accepted ADR or canonical ownership
  model;
- the live `v1.2.7` evidence is found not to support a closure currently
  claimed by M003g or M001;
- a medium-or-higher Eggpack production defect is discovered;
- full verification or candidate-tip hosted CI fails;
- integration would require a force push, squash, or history rewrite.

## 14. Closure evidence

Create `plans/closure/ci-release-orchestration/003h-status.md` recording:

- reviewed main and candidate baselines;
- exact pre-integration compare relation;
- status contradiction matrix and corrected dispositions;
- paths changed by M003h, proving documentation-only scope;
- retained M003e/M003f/M003g and M001 closure evidence;
- live eggsact `v1.2.7` run/tag/asset evidence references;
- full local verification results;
- final candidate hosted CI run id and lane results;
- non-forced main ref update evidence;
- post-integration main SHA and branch compare;
- unresolved findings by severity and owner;
- next dependency-ready handoff (expected Ecosystem M002 / stegoeggo);
- confirmation that Phase 8 remains blocked only on exact rerun reuse / eggsact
  Windows byte reproducibility, unless newer evidence legitimately closes it.
