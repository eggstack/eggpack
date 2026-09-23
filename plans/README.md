# Eggpack Planning System

This directory separates durable architectural direction from temporary execution planning.

Eggpack is the developer-side distribution and release-construction layer for Eggstack. It defines, validates, plans, builds, qualifies, packages, and describes releases. Eggup remains the consumer-side deployment mechanism responsible for acquisition, verification, local replacement, rollback, recovery, and optional service lifecycle.

## Canonical long-term documents

The following files define the intended product and architecture and MUST NOT be edited as part of ordinary implementation work:

- `000-long-term-specification.md` — normative end-state specification and invariants.
- `001-terminology-and-domain-model.md` — normative language and identity model.
- `002-long-term-roadmap.md` — dependency-ordered long-term capability roadmap.
- `003-planning-process.md` — rules for deriving and managing interim plans.

The first three documents are stable architectural references. Changes require an explicit long-term architecture decision, not an implementation convenience. Interim plans MUST reference them rather than silently revising their requirements.

## Planning hierarchy

```text
Long-term specification and terminology
        |
        v
Architecture decision records
        |
        v
Master long-term roadmap
        |
        v
Subsystem roadmaps
        |
        v
Milestone implementation plans
        |
        v
Implementation and verification
        |
        v
Closure records and archive
```

## Directory roles

- `adrs/` — durable architecture decisions. Accepted decisions are superseded, not rewritten.
- `subsystems/` — subsystem specifications and dependency-ordered roadmaps.
- `implementation/` — bounded milestone plans handed to implementation agents.
- `closure/` — verification, evidence, residual-risk, and completion records.
- `archive/` — completed or superseded interim planning retained for traceability.
- `registry.md` — compact control surface for active work and dependency gates.

## Core rule

Long-term documents state **what Eggpack is becoming and what must remain true**. Interim documents state **what an implementation agent should do next against a specific repository baseline**.

Eggpack planning MUST preserve the producer/consumer boundary with Eggup. A convenience that causes Eggpack to perform live machine deployment, or Eggup to become a release-production/build system, is an architecture change and requires an ADR.

## Required classification

Every subsystem roadmap and implementation plan MUST distinguish:

- **Invariant** — a property that must always remain true.
- **Capability** — developer- or release-operator-visible behavior.
- **Infrastructure** — internal machinery required by capabilities.
- **Polish** — ergonomics, diagnostics, performance tuning, cleanup, or documentation.

Infrastructure and polish MUST NOT be presented as completed capability without the corresponding contract-level acceptance evidence.

## Naming conventions

- ADR: `adrs/ADR-NNNN-short-title.md`
- Subsystem roadmap: `subsystems/<subsystem>-roadmap.md`
- Milestone implementation plan: `implementation/<subsystem>/NNN-short-title.md`
- Closure record: `closure/<subsystem>/NNN-status.md`
- Archived document: retain its original relative structure beneath `archive/`

Use stable subsystem names. Do not encode dates in filenames unless the document is inherently time-bound.

## Planning lifecycle

1. Identify the relevant long-term specification sections and invariants.
2. Record unresolved architectural decisions in `adrs/`.
3. Create or update a subsystem roadmap in `subsystems/`.
4. Select one dependency-ready milestone.
5. Write a bounded handoff plan under `implementation/`.
6. Implement and verify.
7. Write a closure record under `closure/`.
8. Update `registry.md` and the subsystem roadmap.
9. Archive completed or superseded interim planning when it is no longer active.

No milestone is complete merely because code landed. Completion requires the closure evidence defined by its implementation plan and roadmap.

## Upstream baseline

Eggpack begins from proven distribution-contract work currently implemented in `eggstack/eggup` as the unpublished `eggup-dist` crate. The initial migration MUST preserve its schema-v1 behavior and closure evidence before extending the model. Eggup remains the source of truth for consumer-side deployment semantics; Eggpack becomes the source of truth for producer-side distribution semantics.
