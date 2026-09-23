# ADR-0001: Producer/Consumer Release Boundary

Status: accepted

Date: 2026-09-22

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#4-producerconsumer-ownership-rule`
- `plans/000-long-term-specification.md#16-eggup-interoperability`

Affected subsystem roadmaps:

- all

## Context

Eggup was created to consolidate security-sensitive installation/update mechanics. During that work, `eggup-dist` began accumulating release-layout schema/tooling because bootstrap installers and runtime updaters duplicated target/asset mapping.

Continuing to expand release production inside Eggup would mix two different lifecycle domains: producer build/release construction and consumer local deployment. Eggstack repositories also duplicate release workflows, cross-build targets, checksum generation, installer mapping, and artifact validation.

## Decision drivers

- keep Eggup small and consumer-oriented;
- centralize producer-side duplication;
- permit either project to be useful independently;
- avoid runtime dependencies on build/CI tooling;
- support a narrow stable interoperability seam.

## Considered options

### Option A — Put all distribution machinery in Eggup

Simpler repository count, but couples build/CI/release publishing concerns to runtime update infrastructure and encourages Eggup to become a package manager/build system.

### Option B — Separate Eggpack producer layer and Eggup consumer layer

Adds a repository/interface boundary but makes ownership explicit and allows each side to remain independently reusable.

### Option C — Use only external release tooling

Reduces initial code but does not by itself provide Eggstack-specific contract/manifests, multi-artifact semantics, or the desired Eggup interoperability boundary.

## Decision

Select Option B.

Eggpack owns release production/evidence. Eggup owns local deployment mechanism. Application/product code owns release selection and install policy.

Eggpack MAY use external build/release backends. Eggup MAY consume Eggpack manifests through an optional adapter. Neither core depends on the other's heavy machinery.

## Consequences

### Positive

- clear lifecycle ownership;
- smaller Eggup runtime boundary;
- producer tooling can evolve independently;
- concrete manifest can serve installers, Eggup, CI, and external tooling.

### Negative

- cross-repository interface/versioning must be maintained;
- migration of `eggup-dist` requires staged duplication before cleanup;
- end-to-end release/update tests span repositories.

### Neutral or deferred

- exact Eggup adapter crate name/API is deferred;
- exact publication provider remains adapter-specific.

## Compatibility and migration

`eggup-dist` is frozen after Eggup distribution M003, which closed the schema-v1 conformance surface in addition to the earlier schema work. Eggpack Contract M002 must port and independently qualify that closed M003 behavior. Eggup has a separate blocked M004 retirement plan that removes the duplicate only after Eggpack M002 closes. A future manifest consumer adapter is separate and waits for Eggpack ReleaseManifest v1.

## Security and reliability implications

Security-sensitive runtime mutation remains in Eggup. Build hooks, CI permissions, release staging, and provenance belong to Eggpack. The separation prevents producer dependencies from silently entering updater core.

## Verification

- Eggpack Contract M002 closure demonstrates equivalence to the closed Eggup M003 conformance surface;
- the subsequent Eggup M004 retirement removes the duplicate producer crate;
- dependency trees show Eggup core has no Eggpack build dependency;
- Eggpack can produce a manifest consumed by an optional Eggup adapter;
- Eggpack can be used without Eggup;
- Eggup can install non-Eggpack artifacts.

## Supersession

None.
