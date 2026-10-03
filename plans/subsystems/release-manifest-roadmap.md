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

M003 is closed. Eggup interoperability M003 closed the real-consumer gate on Eggsact adoption (`eggstack/eggup@538e3e5`, consumer `eggstack/eggsact@65c916b`, hosted CI `36902758482` + drift `36902758396`), and Eggup M004a then proved that `eggpack-manifest 0.1.0` was the remaining Eggpack-owned registry prerequisite for its package/API promotion.

Publication completed on `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`: `eggpack-manifest 0.1.0` is published, non-yanked, and registry-resolvable at the exact version Eggup's qualified adapter pins. No `crates/eggpack-manifest/` delta existed between the consumer-qualified pin `678bbf04f5a02827003a1d9ab83ba4f0e6360e41` and the publication source commit, and the published `src/lib.rs` is byte-identical to that pin, so the published artifact is the consumer-qualified source rather than merely version-compatible. Closure: `plans/closure/release-manifest/003-status.md`. Formal JSON Schema export remains non-blocking polish rather than a prerequisite for Eggup.

Only `eggpack-manifest` was published. `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-ci`, `eggpack-github`, and `eggpack-cli` remain unpublished; no workspace-wide release is implied by the single-package publication.

Post-publication downstream review found that Eggup has since completed the registry promotion that consumed this handoff: Eggup M004 closed at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`, with formal blocked-state cleanup at `3b82d5397e728649a666690868a1e2d0fe42460d`. Eggpack still contains pre-M004 present-tense planning text, so M003a is registered as a docs-only downstream-closure reconciliation. It does not reopen M003 or authorize package/runtime changes.

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
    +--> manifest M003 consumer compatibility + `eggpack-manifest 0.1.0` publication [CLOSED]
    |        |
    |        +--> Eggup M004 package/API promotion [CLOSED DOWNSTREAM]
    |        |
    |        `--> manifest M003a post-publication downstream closure reconciliation [READY; DOCS ONLY]
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

Status: **closed**. Closure: `plans/closure/release-manifest/003-status.md`.

The consumer-qualified schema-v1 source/API is the publication compatibility baseline. Strict fixture/API/package qualification passed on stable and MSRV 1.89.0, `cargo package` and `cargo publish --dry-run` passed from a clean tree, and only the leaf `eggpack-manifest 0.1.0` crate was published through the explicit manual crates.io boundary. crates.io checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629` at publication source `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`, tagged `eggpack-manifest-v0.1.0`. An external registry-only consumer resolved exact `=0.1.0` with no Git or path source and passed a schema-v1 parse/round-trip/determinism smoke plus fail-closed negatives, closing the handoff back to Eggup.

The existing strict parser/serializer tests, direct/bundle/archive fixtures, cross-repo projection evidence, and real Eggsact consumer were the required compatibility harness for this publication. Formal JSON Schema export remains available later polish, but it was not and is not a publication or Eggup-unblock prerequisite.

### M003a — Post-publication downstream closure reconciliation

Class: planning / closure hygiene / cross-repository evidence reconciliation

Hard dependency: M003 closure plus reviewed downstream Eggup M004 closure. Satisfied by Eggpack M003 closure and Eggup `ea1f1c5` / `3b82d53`.

Status: **ready**. Implementation plan: `plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md`.

Reconcile Eggpack's active registry, Release Manifest roadmap, Eggup interoperability roadmap, and M003 post-closure receipt with the fact that Eggup consumed `eggpack-manifest 0.1.0` and closed its M004 registry promotion. Preserve M003's historical closure body, append rather than rewrite historical evidence, and prove an empty production/package/workflow diff. No Rust, Cargo, workflow, tag, release, crates.io, or external-repository mutation is authorized.

## 8. Cross-cutting requirements

Schema changes require compatibility rules. Unknown schema majors fail typed. Unknown security-sensitive semantics fail closed.

Manifest contents must be bounded before runtime consumers rely on them.

## 9. Verification strategy

Round trip, deterministic bytes/order, duplicate/collision negatives, altered digest/size negatives, mixed ReleaseId/SourceRevision rejection, direct/bundle/archive goldens.

## 10. Risks and decision points

Signing exact manifest bytes may require a canonical JSON choice. Do not lock a signing format until manifest semantics are proven.

## 11. Completion definition

At least Eggpack bootstrap/CI and one Eggup adapter consume the same manifest v1 without backend-specific types.

Satisfied: Eggpack's own crates consume the crate as a workspace member, the real Eggsact updater consumes it through Eggup's bounded adapter, and M003 added a fourth independent consumer path — an external registry-only project resolving exact `=0.1.0` from crates.io with no Git or path source.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 | closed (historical) | `plans/implementation/release-manifest/001-release-manifest-v1-domain.md` | `plans/closure/release-manifest/001-status.md` | post-closure defect tracked by M001a |
| M001a | closed | `plans/implementation/release-manifest/001a-cross-target-install-namespace-corrective.md` | `plans/closure/release-manifest/001a-status.md` | M001 implementation/closure |
| M002 | closed | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | `plans/closure/release-manifest/002-status.md` | implementation and hosted CI passed |
| M003 | closed | `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md` | `plans/closure/release-manifest/003-status.md` | published `eggpack-manifest 0.1.0` from `8d661e4`, hosted run 37064833069 green, tag `eggpack-manifest-v0.1.0`, registry-only consumer proof green |
| M003a | ready | `plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md` | — | docs-only reconciliation of Eggup M004 downstream closure (`ea1f1c5`; formal status cleanup `3b82d53`); M003 remains closed; no production/package/workflow changes |
