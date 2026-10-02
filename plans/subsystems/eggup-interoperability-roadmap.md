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

Eggup already has verified transaction, acquisition, ownership, rollback, and service layers. Eggpack ReleaseManifest v1 and corrective M001a are closed; the optional Eggup adapter and its M001a qualification corrective are also closed. The real Eggsact consumer path has now closed in Eggup: `eggstack/eggup@538e3e5605cf3c315c10e5be200c8896de7379b1` records M003 closure on consumer `eggstack/eggsact@65c916b`, with hosted CI `36902758482` and release-drift run `36902758396` green.

The producer convention used by that consumer remains Eggpack Ecosystem M001 / Eggsact Distribution M005 `v1.2.7`: `release/eggpack/distribution.toml` owns the unversioned release artifact names and the published release includes `release-manifest.json`. Eggpack interoperability M003 is therefore **closed downstream**, not ready-to-resume. The next cross-repo prerequisite is no longer runtime adoption: Eggup M004a proved that package/API promotion now waits on registry publication of the already-qualified leaf crate `eggpack-manifest 0.1.0`. That producer-side publication is tracked by Release Manifest M003 at `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md`.

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

M003 Eggsact end-to-end runtime consumer adoption — closed in Eggup/Eggsact (`eggstack/eggup@538e3e5`, consumer `eggstack/eggsact@65c916b`, hosted CI `36902758482` + drift `36902758396`). The next producer-side prerequisite for Eggup package promotion is Release Manifest M003 publication of `eggpack-manifest 0.1.0`.

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

M001 is historical Eggpack-side closure evidence and Eggpack M001a is closed as the corrected producer fixture baseline. The Eggup adapter and its M001a qualification corrective are both closed. Eggpack retains only the producer/interface side of this seam. Eggup interoperability M003 is closed on the real Eggsact consumer path; no consumer runtime work remains under this Eggpack milestone. Eggup M004a has moved the active cross-repo dependency to Release Manifest M003, which must publish the already-qualified `eggpack-manifest 0.1.0` leaf crate before Eggup can execute its registry-only promotion sequence.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | `plans/closure/eggup-interoperability/001-status.md` | post-closure projection defect resolved by M001a |
| M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | `plans/closure/eggup-interoperability/001a-status.md` | — |
| M002 Eggup adapter | closed / qualified in Eggup | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/001-release-manifest-v1-adapter.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001-status.md`; corrective closure `plans/closure/eggpack-manifest-interoperability/001a-status.md` | — |
| M003 Eggsact end-to-end runtime consumer | closed / Eggup-owned | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/003-eggsact-real-consumer-manifest-adoption.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/003-status.md` | consumer `eggstack/eggsact@65c916b`; hosted CI `36902758482` + drift `36902758396` green; next producer prerequisite is Release Manifest M003 `eggpack-manifest 0.1.0` publication |
| M004 bootstrap receipt compatibility | planned only if justified | — | — | real adoption evidence |
