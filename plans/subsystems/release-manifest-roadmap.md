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

`eggpack-contract` exists and Contract M001/M002 are closed. `eggpack-manifest` schema v1 is implemented, and corrective M001a closed the cross-target install-namespace defect while preserving manifest-global artifact filename uniqueness. Manifest M002 is closed with an explicit-file final artifact builder in `eggpack-core`. The `dist` 0.33 evaluation remains disposition C: its manifest is design prior art/backend observation only and is not Eggpack's canonical format.

The former M003 real-consumer gate is now satisfied. Eggup interoperability M003 closed on real Eggsact adoption (`eggstack/eggup@538e3e5`, consumer `eggstack/eggsact@65c916b`, hosted CI `36902758482` + drift `36902758396`). Eggup M004a then proved that `eggpack-manifest 0.1.0` is the remaining Eggpack-owned registry prerequisite for its package/API promotion. Authoring review found no `crates/eggpack-manifest/` path delta between the consumer-qualified pin `678bbf04f5a02827003a1d9ab83ba4f0e6360e41` and Eggpack baseline `ed1bef885eb3e396a945165d680ed3073decf2a5`. M003 is therefore ready under `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md`: freeze/requalify the consumer-compatible leaf crate, publish only `eggpack-manifest 0.1.0` manually, and prove exact registry-only resolution. Formal JSON Schema export remains non-blocking polish rather than a prerequisite for Eggup.

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
manifest M001 schema/domain [CLOSED HISTORICALLY]
    |
    v
manifest M001a namespace corrective
    |
    v
manifest M002 final-artifact builder
    |
    +--> manifest M003 consumer compatibility + `eggpack-manifest 0.1.0` publication [READY]
    |        |
    |        `--> Eggup M004 package/API promotion prerequisite
    +--> Eggup interoperability
    +--> bootstrap/CI aggregation
    +--> provenance/signing
```

## 7. Milestones

### M001 — Manifest v1 domain and deterministic serialization

Class: invariant / infrastructure

Hard dependency: Contract M002 closure, now satisfied; it stabilizes the expected-file/conformance interface on top of already-closed Contract M001.

Define bounded ProductId/ReleaseId/SourceRevision and evidence-reference types, JSON shape, deterministic ordering, relationship-preserving direct/bundle/archive artifact/member identity, validation, and fixtures.

### M001a — Cross-target install namespace corrective

Class: corrective / invariant

Hard dependency: M001 implementation/closure.

Correct install-name uniqueness to be target-local while preserving manifest-global release-artifact uniqueness. Add multi-target direct/bundle/archive regressions and re-close planning state before downstream work resumes.

### M002 — Final artifact manifest builder

Class: capability

Hard dependency: conformance M002 + M001 + M001a closure.

Build a manifest only from a complete, validated final inventory with computed artifact/member size/digest and bounded qualification/provenance evidence references.

### M003 — Consumer compatibility baseline and `eggpack-manifest 0.1.0` publication

Class: compatibility / infrastructure / manual publication prerequisite

Hard dependency: a real v1 consumer. Satisfied by closed Eggup interoperability M003 / Eggsact real-consumer adoption.

Freeze the consumer-qualified schema-v1 source/API as the publication compatibility baseline, re-run strict fixture/API/package qualification, publish only the leaf `eggpack-manifest 0.1.0` crate through the explicit manual crates.io boundary, and prove exact registry-only resolution before handing back to Eggup. The implementation plan is `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md`.

The existing strict parser/serializer tests, direct/bundle/archive fixtures, cross-repo projection evidence, and real Eggsact consumer are the required compatibility harness for this publication. Formal JSON Schema export may be added later if useful, but it is not a publication or Eggup-unblock prerequisite.

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
| M001 | closed (historical) | `plans/implementation/release-manifest/001-release-manifest-v1-domain.md` | `plans/closure/release-manifest/001-status.md` | post-closure defect tracked by M001a |
| M001a | closed | `plans/implementation/release-manifest/001a-cross-target-install-namespace-corrective.md` | `plans/closure/release-manifest/001a-status.md` | M001 implementation/closure |
| M002 | closed | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | `plans/closure/release-manifest/002-status.md` | implementation and hosted CI passed |
| M003 | ready | `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md` | — | real-consumer gate satisfied; execute package/dry-run/manual crates.io publication + registry-only proof |
