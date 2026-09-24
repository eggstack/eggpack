# Eggup Interoperability Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#16-eggup-interoperability`
- `plans/002-long-term-roadmap.md#phase-10--eggup-interoperability`

Related ADRs:

- ADR-0001;
- ADR-0002.

## 1. Purpose and ownership boundary

Define the narrow producer-to-consumer seam by which an Eggup consumer can use Eggpack release evidence without importing build/CI machinery.

Eggpack owns manifest semantics. Eggup owns adapter-to-deployment behavior.

## 2. Work classification

### Invariants

- `eggup-core` does not depend on Eggpack build/core/CLI;
- consumer policy resolves/authorizes release before manifest translation;
- manifest does not grant trust by itself;
- bundle/archive identity is preserved;
- installed-state receipt remains Eggup-owned.

### Capabilities

- parse/validate manifest in a lightweight context;
- resolve exact current platform entry;
- translate size/digest/install mapping to Eggup inputs;
- end-to-end fixture release -> verified update.

### Infrastructure

- manifest compatibility tests;
- optional `eggup-eggpack` adapter in Eggup;
- cross-repo fixture/evidence.

## 3. Non-goals

- Eggpack driving live replacement;
- Eggup learning CI/build runners;
- automatic version selection;
- mandatory Eggpack dependency for all Eggup users.

## 4. Current state

Eggup already has verified transaction, acquisition, ownership, rollback, and service layers. Simple consumers currently carry product-specific release mapping. Eggpack ReleaseManifest v1 and corrective M001a are closed; target-local installation namespaces and manifest-global release artifact names are validated as intended. The Eggpack-side M001a corrective replaces the drifted `projection-bundle.json` with an exact projection of all three CodeGG bundle entries and adds a strict pairwise projection/manifest test harness (direct, bundle, and archive) plus the full detection-gap negative matrix. The optional Eggup adapter M002 is unblocked for authoring against the corrected fixture baseline; this roadmap does not authorize Eggup implementation.

## 5. Target architecture

```text
consumer release policy
        |
        v
resolved release origin/id
        |
        +--> fetch Eggpack manifest
                  |
                  v
             eggup adapter
                  |
                  v
         artifact/acquisition plan
                  |
                  v
             eggup-core
```

## 6. Dependency graph

Hard dependency: ReleaseManifest v1 + conformance maturity. Eggup-side work also requires its own registered plan.

## 7. Milestones

M001 cross-repo interface contract/fixtures — closed historically at `plans/closure/eggup-interoperability/001-status.md`.

M001a projection fixture consistency corrective — closed at `plans/closure/eggup-interoperability/001a-status.md`; corrected bundle projection matches all three CodeGG entries, and pairwise direct/bundle/archive projection tests plus the detection-gap negative matrix now gate CI.

M002 Eggup adapter implementation (in Eggup repo) — unblocked for authoring against the corrected fixture baseline recorded in M001a closure; still requires a separately registered Eggup-side plan before implementation.

M003 one simple end-to-end consumer.

M004 bootstrap receipt compatibility only if justified.

## 8. Cross-cutting requirements

Manifest bounds, target mismatch failure, unknown schema behavior, digest propagation, no hidden network authority, clear error taxonomy.

## 9. Verification strategy

Cross-version fixture tests, wrong target, wrong digest/size, bundle consistency, archive mapping, no core dependency leakage.

## 10. Risks and decision points

If runtime manifest parsing pulls too much producer dependency graph, split/rework leaf schema crates before adoption.

## 11. Completion definition

A real Eggup consumer updates from Eggpack manifest evidence with less duplicated release mapping and unchanged deployment safety.

## 12. Milestone status

M001 is historical closure evidence. M001a is closed and is the corrected fixture baseline an Eggup adapter implementation plan must reference. Eggup repository adapter work is unblocked for authoring against the M001a closure baseline but still requires a separately registered Eggup-side plan before any Eggup code change.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | `plans/closure/eggup-interoperability/001-status.md` | post-closure projection defect resolved by M001a |
| M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | `plans/closure/eggup-interoperability/001a-status.md` | — |
| M002 Eggup adapter | ready to author / Eggup-owned | — | — | separately registered Eggup plan against M001a baseline |
| M003 simple end-to-end consumer | blocked | — | — | Eggup adapter + selected consumer |
| M004 bootstrap receipt compatibility | planned only if justified | — | — | real adoption evidence |
