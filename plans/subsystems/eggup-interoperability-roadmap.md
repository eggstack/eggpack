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

Eggup already has verified transaction, acquisition, ownership, rollback, and service layers. Simple consumers currently carry product-specific release mapping. Eggpack ReleaseManifest v1 is implemented, but post-closure review found a cross-target install-namespace validation defect; M001a must close before this interface is treated as stable for cross-repository consumption.

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

M001 cross-repo interface contract/fixtures.

M002 Eggup adapter implementation (in Eggup repo).

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

M001 cross-repository interface contract and fixtures is temporarily blocked on Release Manifest M001a. After that corrective re-closes the schema, Eggpack-side interoperability planning may resume; any Eggup repository changes still require a registered Eggup-side plan.
