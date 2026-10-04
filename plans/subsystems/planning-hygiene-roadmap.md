# Planning Hygiene Roadmap

Status: active

Long-term references:

- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`

## 1. Purpose

Keep Eggpack's active control surfaces synchronized with already-reviewed implementation, closure, hosted-CI, and cross-repository evidence.

This line is documentation/evidence reconciliation only. It does not own runtime behavior, package publication, consumer implementation, or feature scope.

## 2. Invariants

- historical closure records are preserved rather than rewritten to hide chronology;
- current-state prose must not retain blockers that have already been discharged;
- another repository's work is attributed to that repository and exact evidence;
- evidence-only status transitions do not imply production changes;
- no new implementation milestone is marked ready merely because research can begin;
- the registry remains the compact control surface, with detailed evidence in source roadmaps/plans/closures.

## 3. Current state

Release Manifest M003a closed the post-Eggup-M004 reconciliation and left no dependency-ready Eggpack implementation work. A follow-up review found broader pre-M003a drift outside that corrective's scope:

- `plans/registry.md` still contains snapshot-era Eggsact/Eggpack baseline prose from before Ecosystem M001 and CI M003h closure;
- CI M003b / Phase 8 still cite Eggsact M005a Windows nondeterminism as open, but Eggsact M005a is now closed with exact rerun-reuse evidence;
- Ecosystem M002's remaining operational condition is now owned by a concrete ready StegoEggo `0.4.3` release milestone rather than an unspecified future stable release;
- Contract M003 has the consumer evidence it requested and now needs a bounded polish decision based on observed consumer glue;
- Bootstrap M003 still needs the second consumer's live public receipt and remains blocked until StegoEggo's ordinary `0.4.3` release closes that evidence condition.

## 4. Milestones

### M001 — Current evidence baseline and blocker reconciliation

Class: planning / closure hygiene / cross-repository evidence reconciliation

Implementation plan:

`plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md`

Status: ready.

Objective: reconcile the active registry and affected subsystem roadmaps with current Eggsact/StegoEggo evidence, discharge CI M003b's obsolete external blocker if its recorded acceptance condition is met, and leave each future line with an exact current blocker or research disposition.

## 5. Completion definition

The planning-hygiene line is complete when active control surfaces no longer contradict reviewed evidence and each blocked/future line has an exact disposition without inventing implementation work.

## 6. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 current evidence baseline + blocker reconciliation | ready | `plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md` | — | none; evidence is already available read-only |
