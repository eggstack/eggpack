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


## Proposed records

- ADR-0006 — release evidence digest acceptance set. Decides whether the
  all-zeros SHA-256 sentinel is refused by every producer path and whether the
  published `ReleaseManifest` acceptance set narrows, which is a compatibility
  change to the published `eggpack-manifest` crate. Gates the
  digest-acceptance work package of CI M003l only.

## Accepted records

- ADR-0001 — producer/consumer release boundary;
- ADR-0002 — contract/plan/manifest separation;
- ADR-0003 — checked-in generated CI and explicit publication gate;
- ADR-0004 — first-party native Cargo/cargo-zigbuild adapter boundary;
- ADR-0005 — native qualification is independent of build strategy when the qualification host matches the target OS/architecture.
