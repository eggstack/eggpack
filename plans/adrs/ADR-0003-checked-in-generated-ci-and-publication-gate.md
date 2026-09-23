# ADR-0003: Checked-In Generated CI and Explicit Publication Gate

Status: accepted

Date: 2026-09-22

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#14-ci-and-workflow-generation`
- `plans/000-long-term-specification.md#15-publication-policy`

Affected subsystem roadmaps:

- CI/release orchestration;
- ecosystem adoption.

## Context

Eggstack repositories currently maintain large release workflows with duplicated target matrices and guards. Central reusable workflows can reduce duplication but may hide policy behind a remote mutable dependency. Fully hand-written workflows preserve visibility but retain drift.

The current repository practice also favors qualification/draft assembly followed by deliberate human publication.

## Decision drivers

- reviewability;
- deterministic drift detection;
- minimized hidden remote policy;
- safe staging before publication;
- compatibility with current GitHub Actions usage.

## Considered options

### Option A — Central remote reusable workflow only

Low duplication but policy can change outside the consumer repository.

### Option B — Generated checked-in workflows

Some generated-file churn, but exact release logic remains visible in each repository.

### Option C — Keep hand-written workflows

Maximum flexibility but preserves duplication Eggpack exists to remove.

## Decision

Select Option B as the default.

Eggpack will generate deterministic, checked-in provider workflows and provide a drift check. Reusable workflows MAY implement stable leaf operations but MUST NOT become the sole hidden release authority.

Build/qualification/aggregation MAY stage a draft release. Final public publication remains an explicit maintainer action unless a later ADR deliberately changes policy.

Registry publication (crates.io/PyPI/etc.) is separately authorized and is not implied by release qualification.

## Consequences

### Positive

- code review sees exact release graph;
- target/config drift can be detected;
- repositories are resilient to mutable remote workflow behavior;
- publication stays deliberate.

### Negative

- generated workflow diffs need maintenance;
- generator compatibility becomes a public operational contract.

### Neutral or deferred

- exact action pinning policy is defined in implementation/consumer policy;
- non-GitHub CI providers are future adapters.

## Compatibility and migration

Existing hand-written workflows remain until a consumer adoption plan proves generated parity. Eggpack MUST not overwrite a workflow without a deterministic diff/check mode.

## Security and reliability implications

Generated workflows must keep permissions least-privilege and separate read/build jobs from write/staging jobs. Public immutable release assets must not be silently replaced.

## Verification

- `eggpack ci generate` is deterministic;
- `eggpack ci check` detects edits/drift;
- generated workflows expose explicit permissions;
- staging can complete without final publication;
- consumer migration tests compare old/new artifact sets.

## Supersession

None.
