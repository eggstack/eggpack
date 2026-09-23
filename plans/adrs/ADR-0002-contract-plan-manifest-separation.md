# ADR-0002: Separate Distribution Contract, Release Plan, Release Manifest, and Install Receipt

Status: accepted

Date: 2026-09-22

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#6-portable-distribution-contract`
- `plans/000-long-term-specification.md#8-release-planning`
- `plans/000-long-term-specification.md#12-concrete-release-manifest`

Affected subsystem roadmaps:

- contract and conformance;
- release manifest;
- Eggup interoperability;
- bootstrap/CI generation.

## Context

Release systems often conflate desired layout, build plan, emitted artifacts, and installed state. That makes it difficult to detect drift and encourages runtime consumers to duplicate producer policy.

## Decision drivers

- distinguish intent from evidence;
- make manifests backend-independent;
- keep installed-state ownership in Eggup;
- support deterministic testing;
- provide a future signing/attestation unit.

## Considered options

### Option A — One large release document

Simple superficially, but mixes checked-in policy with invocation state and final evidence.

### Option B — Four distinct objects

DistributionContract, ReleasePlan, ReleaseManifest, Eggup Install Receipt.

## Decision

Select Option B.

```text
DistributionContract = portable expected release layout
ReleasePlan          = one producer invocation's resolved intent
ReleaseManifest      = exact finalized release bytes/evidence
Eggup InstallReceipt = installed state on one consumer machine
```

PackConfig is producer-only configuration and may contribute to ReleasePlan but is not a consumer contract.

## Consequences

### Positive

- clear authority;
- manifests remain usable regardless of build backend;
- tests can separately validate planning and actual output;
- signing can bind final release state.

### Negative

- several versioned schemas/types must be maintained;
- conversion boundaries require explicit validation.

### Neutral or deferred

- exact canonical JSON scheme for manifest signing is deferred until manifest v1 implementation.

## Compatibility and migration

The existing `eggup-dist` schema becomes the initial DistributionContract. It is not expanded ad hoc into a combined build/update document during migration.

## Security and reliability implications

Final hashes/sizes are produced only after byte-changing finalization. A plan cannot be mistaken for evidence that artifacts exist or are trusted.

## Verification

Tests must demonstrate that:

- a plan exists without artifacts;
- an incomplete/mismatched inventory cannot produce a successful manifest;
- a manifest cannot claim installed state;
- Eggup receipts remain independently owned.

## Supersession

None.
