# Release Manifest Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#12-concrete-release-manifest`
- `plans/001-terminology-and-domain-model.md#11-release-manifest`
- `plans/002-long-term-roadmap.md#phase-3--concrete-releasemanifest-v1`

Related ADR:

- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

## 1. Purpose and ownership boundary

This subsystem owns the machine-readable description of finalized release bytes.

It does not own build strategy, release selection, hosting authority, live installation, or trust policy.

## 2. Work classification

### Invariants

- manifest describes finalized bytes, never pre-finalization candidates;
- one manifest identifies one ProductId + ReleaseId;
- entries use canonical target identity;
- sizes/digests are exact;
- direct/bundle/archive identity is unambiguous;
- serialization is deterministic;
- no secrets/credentials;
- no installed-state claims;
- integrity is not labeled authenticity.

### Capabilities

- emit/parse/validate one release manifest;
- inspect target/artifact entries;
- use manifest as backend-independent Eggpack output and Eggup input.

### Infrastructure

- `eggpack-manifest`;
- schema v1;
- deterministic JSON;
- fixtures;
- manifest builder from validated release inventory.

### Polish

- JSON schema export;
- human-readable inspection.

## 3. Non-goals

- deciding which release is latest;
- acquiring artifacts;
- signing standard selection in v1;
- storing Eggup receipts;
- embedding arbitrary CI logs.

## 4. Current state

`eggpack-contract` exists and Contract M001/M002 are closed. M002 established the expected-file/conformance interface this subsystem consumes. No Eggpack release-manifest crate exists yet. The `dist` 0.33 evaluation closed with disposition C: its manifest is design prior art/backend observation only and is not Eggpack's canonical format.

## 5. Target architecture

```text
validated contract + release identity
           +
finalized observed artifacts
           +
qualification summaries
           |
           v
    ManifestBuilder
           |
           v
 eggpack-release.json
```

A manifest parser must remain usable without Eggpack build tooling.

## 6. Dependency graph

```text
contract M001
    |
conformance M002
    |
    v
manifest M001 schema/domain
    |
    v
manifest M002 final-artifact builder
    |
    +--> Eggup interoperability
    +--> bootstrap/CI aggregation
    +--> provenance/signing
```

## 7. Milestones

### M001 — Manifest v1 domain and deterministic serialization

Class: invariant / infrastructure

Hard dependency: Contract M002 closure, now satisfied; it stabilizes the expected-file/conformance interface on top of already-closed Contract M001.

Define bounded ProductId/ReleaseId/SourceRevision and evidence-reference types, JSON shape, deterministic ordering, relationship-preserving direct/bundle/archive artifact/member identity, validation, and fixtures.

### M002 — Final artifact manifest builder

Class: capability

Hard dependency: conformance M002 + M001.

Build a manifest only from a complete, validated final inventory with computed artifact/member size/digest and bounded qualification/provenance evidence references.

### M003 — Schema export and compatibility harness

Class: polish / infrastructure

Add JSON Schema or equivalent compatibility fixtures only after v1 has a real consumer.

## 8. Cross-cutting requirements

Schema changes require compatibility rules. Unknown schema majors fail typed. Unknown security-sensitive semantics fail closed.

Manifest contents must be bounded before runtime consumers rely on them.

## 9. Verification strategy

Round trip, deterministic bytes/order, duplicate/collision negatives, altered digest/size negatives, mixed ReleaseId/SourceRevision rejection, direct/bundle/archive goldens.

## 10. Risks and decision points

Signing exact manifest bytes may require a canonical JSON choice. Do not lock a signing format until manifest semantics are proven.

## 11. Completion definition

At least Eggpack bootstrap/CI and one Eggup adapter consume the same manifest v1 without backend-specific types.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 | closed | `plans/implementation/release-manifest/001-release-manifest-v1-domain.md` | `plans/closure/release-manifest/001-status.md` | — |
| M002 | ready | — | — | Manifest M001 + conformance M002 closed |
| M003 | planned | — | — | real consumer |
