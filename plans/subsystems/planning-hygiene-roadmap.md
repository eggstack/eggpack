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

M001 is closed. It reconciled the drift found after Release Manifest M003a and, in doing so, closed three milestones and cleared the ordering gate on a fourth. Its closure record is `plans/closure/planning-hygiene/001-status.md`.

Resolved:

- stale snapshot-era Eggsact/Eggpack baseline prose in the registry and the ecosystem roadmap is replaced with terminal evidence and closure references;
- CI M003b's exact rerun-reuse condition is discharged from eggsact M005a evidence, so M003b is closed and Phase 8 is satisfied;
- Ecosystem M002's live condition was met by stegoeggo `v0.5.0` and M002 is closed. The concrete owner is no longer a future `0.4.3` milestone — `0.4.3` was stopped pre-publication on consumer semver grounds and superseded by `0.5.0`;
- Contract M003's consumer-evidence gate is satisfied, with the duplicated producer-fact parsing precisely identified across 2 repositories and 3 implementations;
- Bootstrap M003's two-consumer adoption evidence now exists and it is unblocked for candidate review;
- Ecosystem M003 Eggsearch's ordering gate is cleared for research/plan, with its preflight facts recorded.

## 4. Milestones

### M001 — Current evidence baseline and blocker reconciliation

Class: planning / closure hygiene / cross-repository evidence reconciliation

Implementation plan:

`plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md`

Status: closed.

Closure record: `plans/closure/planning-hygiene/001-status.md`.

Outcome: reconcile the active registry and affected subsystem roadmaps with current Eggsact/StegoEggo evidence, discharge CI M003b's obsolete external blocker, and leave each future line with an exact current blocker or research disposition. All ten acceptance criteria are met, with one §11 stop condition that fired during execution and was absorbed by explicit maintainer decision rather than re-planned; the closure record documents that decision and its consequence.

## 5. Completion definition

The planning-hygiene line is complete when active control surfaces no longer contradict reviewed evidence and each blocked/future line has an exact disposition without inventing implementation work. M001 met that condition on 2026-10-04.

## 6. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 current evidence baseline + blocker reconciliation | closed | `plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md` | `plans/closure/planning-hygiene/001-status.md` | none; docs/evidence-only, zero production/package/workflow delta |
