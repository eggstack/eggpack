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

Eggup already has verified transaction, acquisition, ownership, rollback, and service layers. Simple consumers currently carry product-specific release mapping. Eggpack ReleaseManifest v1 and corrective M001a are closed; target-local installation namespaces and manifest-global release artifact names are validated as intended. Eggpack-side M001 interface contract and fixtures closed historically, but post-closure review found that `projection-bundle.json` does not match its paired CodeGG bundle manifest and was not mechanically compared in tests. Corrective M001a is the active gate. The optional Eggup adapter M002 is blocked until M001a closes; this roadmap does not authorize Eggup implementation.

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

M001a projection fixture consistency corrective — ready at `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md`; fixes bundle projection drift and adds manifest↔projection regression validation.

M002 Eggup adapter implementation (in Eggup repo) — blocked until M001a closes and a corrected fixture baseline is recorded.

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

M001 is historical closure evidence. M001a is now the sole dependency-ready interoperability handoff. Any Eggup repository adapter work remains blocked until M001a closes and still requires a separately registered Eggup-side plan.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | `plans/closure/eggup-interoperability/001-status.md` | post-closure projection defect tracked by M001a |
| M001a projection fixture consistency corrective | ready | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | — | M001 implementation/closure |
| M002 Eggup adapter | blocked / Eggup-owned | — | — | Eggpack M001a corrective closure + separately registered Eggup plan |
| M003 simple end-to-end consumer | blocked | — | — | Eggup adapter + selected consumer |
| M004 bootstrap receipt compatibility | planned only if justified | — | — | real adoption evidence |
