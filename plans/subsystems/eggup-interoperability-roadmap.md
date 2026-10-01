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

Eggup already has verified transaction, acquisition, ownership, rollback, and service layers. Eggpack ReleaseManifest v1 and corrective M001a are closed; the optional Eggup adapter and its M001a qualification corrective are also closed. Eggsact is already the selected real consumer in Eggup under `plans/implementation/eggpack-manifest-interoperability/003-eggsact-real-consumer-manifest-adoption.md`, and Eggup completed the bounded JSON parse/project API qualification before stopping on missing producer authority.

That producer-side gate is now satisfied. Eggpack Ecosystem M001 and Eggsact Distribution M005 closed on the real `v1.2.7` release: `eggstack/eggsact: release/eggpack/distribution.toml` is the producer authority for the existing unversioned `eggsact-{target}[.exe]` assets, and the generated release staged a 15-asset set including `release-manifest.json`, which the maintainer then published. Eggup interoperability M003 is therefore **ready to resume in Eggup/Eggsact**, not ready to author from scratch. The runtime updater still has no manifest-consumption path, so M003 is not closed; execution must refresh the current Eggup/Eggsact baselines and then complete the already-registered consumer plan and verification matrix.

Eggwork Operations M003 (`eggstack/eggwork: plans/implementation/operations-distribution/003-eggpack-producer-packaging-integration.md`) remains explicitly producer-side adoption only and does not satisfy or block this runtime interoperability milestone.

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

M002 Eggup adapter implementation (in Eggup repo) — implemented/closed historically at `eggstack/eggup@5fbb66853bdad59aaf2bd3c7bb43a43492d0b6ef`; Eggup corrective M001a closed at `19935ec3610a5238af33a9d4f05a14925ceac25c` with full regression evidence.

M003 Eggsact end-to-end runtime consumer adoption — selected and already planned in Eggup; producer gate satisfied by Ecosystem M001 / Eggsact M005; resume the Eggup-owned implementation and consumer verification.

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

M001 is historical Eggpack-side closure evidence and Eggpack M001a is closed as the corrected producer fixture baseline. The Eggup adapter and its M001a qualification corrective are both closed. Eggpack retains only the producer/interface side of this seam. Eggsact is already selected and the Eggup M003 plan/status record already exist; after Ecosystem M001 / Eggsact M005 established the live contract and published manifest convention, M003 is ready to resume in the consumer repositories.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | `plans/closure/eggup-interoperability/001-status.md` | post-closure projection defect resolved by M001a |
| M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | `plans/closure/eggup-interoperability/001a-status.md` | — |
| M002 Eggup adapter | closed / qualified in Eggup | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/001-release-manifest-v1-adapter.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001-status.md`; corrective closure `plans/closure/eggpack-manifest-interoperability/001a-status.md` | — |
| M003 Eggsact end-to-end runtime consumer | ready to resume / Eggup-owned | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/003-eggsact-real-consumer-manifest-adoption.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/003-status.md` (bounded pass; milestone not closed) | producer gate satisfied by Eggpack Ecosystem M001 + Eggsact M005 / `v1.2.7`; refresh consumer/Eggup baselines and complete runtime adoption; Eggwork Operations M003 is producer-only |
| M004 bootstrap receipt compatibility | planned only if justified | — | — | real adoption evidence |
