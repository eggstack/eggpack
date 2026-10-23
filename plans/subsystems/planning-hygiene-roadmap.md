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
- Ecosystem M003 Eggsearch's ordering gate is cleared for research/plan, with its preflight facts recorded;
- M002 then reconciled the post-M003a state itself: Contract M003 is described as closed wherever it appears, M003a as closed, and Ecosystem M003b as conditionally closed against the exact paired Eggpack/Eggsearch plan paths rather than a generic "mirrored plan required" note.

## 4. Milestones

### M001 — Current evidence baseline and blocker reconciliation

Class: planning / closure hygiene / cross-repository evidence reconciliation

Implementation plan:

`plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md`

Status: closed.

Closure record: `plans/closure/planning-hygiene/001-status.md`.

Outcome: reconcile the active registry and affected subsystem roadmaps with current Eggsact/StegoEggo evidence, discharge CI M003b's obsolete external blocker, and leave each future line with an exact current blocker or research disposition. All ten acceptance criteria are met, with one §11 stop condition that fired during execution and was absorbed by explicit maintainer decision rather than re-planned; the closure record documents that decision and its consequence.

### M002 — Post-M003a cross-repository handoff reconciliation

Class: planning / closure hygiene / cross-repository handoff reconciliation

Implementation plan:

`plans/implementation/planning-hygiene/002-post-m003a-cross-repository-handoff-reconciliation.md`

Status: closed.

Closure record: `plans/closure/planning-hygiene/002-status.md`.

Outcome: docs/evidence-only. It removed the stale post-closure M003/M003a/M003b control-surface text, collapsed the duplicated and self-contradicting `AGENTS.md` current-handoff paragraph into one statement sourced from the registry, and bound the handoff to the exact registered paired-plan paths. The production/package/workflow diff is empty.

### M003 — Post-implementation-review baseline reconciliation

Class: planning / documentation hygiene / architecture-citation reconciliation

Implementation plan:

`plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`

Status: closed.

Closure record: `plans/closure/planning-hygiene/003-status.md`.

Outcome: docs/evidence-only. It re-based the `architecture/` citation set and the per-crate line-count table **by symbol name**, against the measured tree SHA `fa82486`, closing the `ci-consumer-seam.md` stale anchors and the residual `cli.md` drift that `architecture/overview.md` named as outstanding. It rewrote the `AGENTS.md` current-handoff paragraph so it is a single statement sourced from `plans/registry.md`, completed `crates/eggpack-cli/README.md` into the full command reference that `docs/README.md` and `docs/quickstart.md` already promise, and registered ADR-0006 and the seven review-chain milestones in the registry, the ADR index, and the affected subsystem roadmaps. It changed no file under `crates/**` and rewrote no closure record.

## 5. Completion definition

The planning-hygiene line is complete when active control surfaces no longer contradict reviewed evidence and each blocked/future line has an exact disposition without inventing implementation work. M001 met that condition for its evidence set on 2026-10-04; M002 met it for the later Contract M003, Bootstrap M003, and Ecosystem M003a closures. M003 reopens the line because the 2026-10-23 post-implementation review found that the architecture index, the `AGENTS.md` handoff, and the CLI command reference had all drifted from the code they describe, and because the review chain it registers must appear in the registry and the affected roadmaps.

## 6. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 current evidence baseline + blocker reconciliation | closed | `plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md` | `plans/closure/planning-hygiene/001-status.md` | none; docs/evidence-only, zero production/package/workflow delta |
| M002 post-M003a cross-repository handoff reconciliation | closed | `plans/implementation/planning-hygiene/002-post-m003a-cross-repository-handoff-reconciliation.md` | `plans/closure/planning-hygiene/002-status.md` | none; docs/evidence-only, empty production/package/workflow diff, handoff bound to exact paired-plan paths |
| M003 post-implementation-review baseline reconciliation | closed | `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md` | `plans/closure/planning-hygiene/003-status.md` | none; docs/evidence-only. `architecture/` citations and line counts re-based by symbol name against tree SHA `fa82486`, the `ci-consumer-seam.md` and residual `cli.md` drift closed, the `AGENTS.md` handoff rewritten from the registry, the CLI command reference verified, and the review chain registered. Empty production/package/workflow diff |
