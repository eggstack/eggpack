# Architecture Decision Records

This directory contains durable decisions affecting Eggpack architecture across milestones or subsystems.

## Naming

`ADR-NNNN-short-title.md`

Numbers are monotonically increasing and never reused.

## Status lifecycle

```text
proposed -> accepted -> deprecated or superseded
         `-> rejected
```

Accepted ADRs are historical records. Create a new ADR to supersede an accepted decision.

## Required structure

Each ADR records:

- status/date/owners;
- related canonical sections and subsystem roadmaps;
- context and decision drivers;
- considered options;
- precise decision;
- consequences;
- compatibility/migration;
- security/reliability implications;
- verification;
- supersession.

Use an ADR when a decision establishes ownership, a public compatibility contract, a durable external backend, publication authority, or a trust/signing model.
