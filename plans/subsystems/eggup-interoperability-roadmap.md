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

The producer convention used by that consumer remains Eggpack Ecosystem M001 / Eggsact Distribution M005 `v1.2.7`: `release/eggpack/distribution.toml` owns the unversioned release artifact names and the published release includes `release-manifest.json`. Eggpack interoperability M003 is therefore **closed downstream**, not ready-to-resume. The cross-repo prerequisite that Eggup M004a identified is no longer runtime adoption or registry availability: Release Manifest M003 published the already-qualified leaf crate `eggpack-manifest 0.1.0` to crates.io from `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0` (checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`, tag `eggpack-manifest-v0.1.0`, hosted run 37064833069 green), and an external registry-only consumer resolved exact `=0.1.0` with no Git or path source. The published `src/lib.rs` is byte-identical to the consumer-qualified pin `678bbf04f5a02827003a1d9ab83ba4f0e6360e41`. Closure: `plans/closure/release-manifest/003-status.md`.

No Eggpack work remains under this seam. The Eggup-owned registry promotion that was waiting on that prerequisite has since been executed and closed downstream: Eggup M004 published `eggup-acquisition 0.1.2` -> `eggup-eggfetch 0.1.2` -> `eggup-eggpack 0.1.2` in that dependency order from publication source `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28` (hosted run `37090397398`, green on Stable, MSRV, macOS, and Windows) and closed at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`; `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d` then formally reconciled Eggup's own stale blocked-state prose. `eggup-eggpack 0.1.2` now consumes registry `eggpack-manifest =0.1.0` with no Git or path edge, and Eggup's adapter-only and Eggsact-shaped registry-only external graphs both resolve and pass smoke. Eggup closure record: `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/004-status.md`. That work is Eggup's and is not claimed here; Eggpack's only remaining task was recording the receipt, which Release Manifest M003a closed at `plans/closure/release-manifest/003a-status.md`.

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

M003 Eggsact end-to-end runtime consumer adoption — closed in Eggup/Eggsact (`eggstack/eggup@538e3e5`, consumer `eggstack/eggsact@65c916b`, hosted CI `36902758482` + drift `36902758396`). The producer-side prerequisite for Eggup package promotion is satisfied: Release Manifest M003 published `eggpack-manifest 0.1.0` to crates.io from Eggpack `8d661e4` with published bytes identical to the consumer-qualified pin.

M004 Eggup registry package/API promotion — **closed in Eggup**, not Eggpack's to close. Closure `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/004-status.md` at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`; formal status cleanup `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`; publication source `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28`; hosted run `37090397398` green. The Eggpack prerequisite it consumed is Release Manifest M003 (`eggpack-manifest 0.1.0`, closed at `plans/closure/release-manifest/003-status.md`). Its chain is no longer a blocker on anything Eggpack owns.

M004 bootstrap receipt compatibility only if justified. Eggpack's separately named placeholder remains `planned`; Eggup M004 above is a different, already-closed downstream milestone and is not repurposed.

## 8. Cross-cutting requirements

Manifest bounds, target mismatch failure, unknown schema behavior, digest propagation, no hidden network authority, clear error taxonomy.

## 9. Verification strategy

Cross-version fixture tests, wrong target, wrong digest/size, bundle consistency, archive mapping, no core dependency leakage.

## 10. Risks and decision points

If runtime manifest parsing pulls too much producer dependency graph, split/rework leaf schema crates before adoption.

## 11. Completion definition

A real Eggup consumer updates from Eggpack manifest evidence with less duplicated release mapping and unchanged deployment safety.

## 12. Milestone status

M001 is historical Eggpack-side closure evidence and Eggpack M001a is closed as the corrected producer fixture baseline. The Eggup adapter and its M001a qualification corrective are both closed. Eggpack retains only the producer/interface side of this seam. Eggup interoperability M003 is closed on the real Eggsact consumer path; no consumer runtime work remains under this Eggpack milestone. The Release Manifest M003 dependency Eggup M004a moved here is closed: `eggpack-manifest 0.1.0` is registry-resolvable at the exact version Eggup's qualified adapter pins, with published bytes identical to the consumer-qualified source. Eggup has since executed and closed its own registry-only promotion sequence at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`, so the seam is closed end to end. Eggpack owes nothing further on it, and no Eggup milestone is blocked on Eggpack. The next downstream action is Eggsact's own Git-to-registry migration, which is Eggsact-owned and separately authorized in `eggstack/eggsact`; it is not an Eggpack blocker or handoff.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | `plans/closure/eggup-interoperability/001-status.md` | post-closure projection defect resolved by M001a |
| M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | `plans/closure/eggup-interoperability/001a-status.md` | — |
| M002 Eggup adapter | closed / qualified in Eggup | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/001-release-manifest-v1-adapter.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001-status.md`; corrective closure `plans/closure/eggpack-manifest-interoperability/001a-status.md` | — |
| M003 Eggsact end-to-end runtime consumer | closed / Eggup-owned | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/003-eggsact-real-consumer-manifest-adoption.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/003-status.md` | consumer `eggstack/eggsact@65c916b`; hosted CI `36902758482` + drift `36902758396` green; producer prerequisite satisfied by Release Manifest M003 `plans/closure/release-manifest/003-status.md` |
| M004 Eggup registry package/API promotion | closed in Eggup; not Eggpack's to close | `eggstack/eggup: plans/implementation/eggpack-manifest-interoperability/004-registry-package-api-promotion-and-publication.md` | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/004-status.md`; closure `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`; status cleanup `3b82d5397e728649a666690868a1e2d0fe42460d`; publication source `02a1d32931be29cc3d8980833643b2cd822f2d28`; hosted run `37090397398` green | none remaining; the Eggpack prerequisite (`eggpack-manifest 0.1.0`, Release Manifest M003) was consumed and the `0.1.2` publication chain is complete |
| M004 bootstrap receipt compatibility | planned only if justified | — | — | real adoption evidence |
