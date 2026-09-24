# Eggpack Active Planning Registry

This file is the compact control surface for active interim planning. Detailed requirements live in canonical documents, subsystem roadmaps, implementation plans, closure records, and Git history.

## Canonical direction

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`

## Status vocabulary

- **proposed** — roadmap/plan exists but is not approved for execution.
- **ready** — hard dependencies/interfaces are satisfied.
- **active** — implementation or closure work is in progress.
- **blocked** — named dependency/evidence prevents progress.
- **closing** — implementation landed; closure evidence being gathered.
- **closed** — closure record accepted.
- **conditionally closed** — implementation substantially complete with named evidence condition outstanding.
- **superseded** — replaced by another document.
- **archived** — retained only for traceability.

## Accepted architecture decisions

| ADR | Decision |
|---|---|
| `ADR-0001-producer-consumer-release-boundary.md` | Eggpack owns producer release construction/evidence; Eggup owns consumer local deployment; product code owns release/install policy. |
| `ADR-0002-contract-plan-manifest-separation.md` | DistributionContract, ReleasePlan, ReleaseManifest, and Eggup InstallReceipt are distinct authorities. |
| `ADR-0003-checked-in-generated-ci-and-publication-gate.md` | Generated CI is checked in/reviewable; release publication remains explicit. |
| `ADR-0004-first-party-native-cargo-build-adapter.md` | Initial native execution uses first-party Cargo/cargo-zigbuild adapters; no dist backend or generic command DSL is authorized. |

## Current evidence baselines

### Eggpack

- repository began empty on 2026-09-22; Contract M001 and the dist 0.33 evaluation are closed;
- last reviewed code/implementation baseline before the planning-only producer/consumer reorientation: `e3452263225fa1ea262e03b557f40b395e6a52d8`;
- producer/consumer reorientation baseline: `5793ccecf5139d9b7b250534703ec5ee84fe44b8`; subsequent planning-only corrections preserve that ownership/cutover model;
- Contract M002 implementation: `a36803a7c34cc5b273559520bf99cb2400cc183a`; closure record: `plans/closure/contract-conformance/002-status.md`;
- Release Manifest M001 implementation: `b5df057a0ab8d30664aeccc26aa7944678626930`; historical closure: `plans/closure/release-manifest/001-status.md`;
- Release Manifest M001a implementation: `20a3084bbfbc7fac03b70e0c397f9059186169c8`; closure: `plans/closure/release-manifest/001a-status.md`. Install-name collisions are target-local; release-artifact filename collisions remain manifest-global;
- Eggup interface historical baseline: `99c9040a5d106cfa46a4ba02f9fa0cee8653166c`; Eggpack-side corrective M001a implementation: `8d9264b3c224f3f05a061f4038b0f328b1c5c95e`; closure: `plans/closure/eggup-interoperability/001a-status.md`; hosted CI: `35998756491`; Eggup adapter M001 implementation: `5fbb66853bdad59aaf2bd3c7bb43a43492d0b6ef`; Eggup adapter M001a qualification implementation: `19935ec3610a5238af33a9d4f05a14925ceac25c`; closure: `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001a-status.md`; hosted CI: `36014508645`;
- prior foundation handoffs closed: Manifest M002, Build/Qualification M001, Bootstrap M001, and Eggup Interop M001/M001a;
- operational producer handoffs registered: ADR-0004 `e8bc338ab193fa8ed5fc7debb0cd68b54ebf586b`, Build/Qualification M002 plan `1666df9ac68062f7a1be4ed757a4f1ccc21aad5b`, CI Orchestration M001 plan `b7270df591efd4b6a3b8c8e02af70d3960c72f0f`, M002a Windows stability corrective `59bf3621da94b7b6ae5d64c357dee21bb27b7527`, and Build/Qualification M003 plan `fb60fe0ac0ee726e24857da661043df8cfd4454f`;
- canonical architecture commit: `2022f44f4df9b0c2518ff22a53d87fd77f63992a`.

### Eggup predecessor

Eggup's current planning assigns producer distribution authority to Eggpack. The relevant immutable predecessor is distribution M003, not current-main incidental state.

Distribution predecessor evidence:

- M001 implementation: `889a234cbe7f461d92def3df45c83c06a7d257e5`;
- M002 strict template/collision corrective: `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`;
- M003 conformance implementation: `9941c58d7039410c728860f9e4e382881d4ccf54`;
- M003 closure: `4169c8021b447fe73c8ee3ea71a80a535c940f54`;
- `eggup-dist` was unpublished predecessor evidence and has now been removed from Eggup;
- Eggup distribution M004 implementation `bc25885bd41b86bfdf2f32d1e42856e00829cd7a` is closed at `plans/closure/distribution-bootstrap/004-status.md` in `eggstack/eggup`;
- Contract M002 ported/qualified the closed M003 behavior before that retirement, so Eggpack is now the sole active producer distribution authority.

### External backend evidence

- `axodotdev/cargo-dist` / `dist` latest reviewed release: 0.33.0 (September 2026);
- upstream demonstrates generated release machinery, installers, manifest, checksums, and GitHub Artifact Attestations;
- no external production backend is adopted; ADR-0004 selects Eggpack-owned first-party Cargo/cargo-zigbuild adapters for the initial native execution slice while keeping the adapter boundary replaceable.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies / blockers |
|---|---|---|---|---|
| Contract and conformance | active | `plans/subsystems/contract-conformance-roadmap.md` | M002 closed; M003 planned | Eggup M004 retirement closed; consumer evidence gates M003 |
| External backend evaluation | closed | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 closed (C) | no production backend adopted; new evidence/plan required to reopen |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M002 closed; M003 planned | M003 requires a real consumer |
| Build and qualification | active | `plans/subsystems/build-qualification-roadmap.md` | M001/M002/M002a closed; M003 ready | registered qualification execution/evidence handoff is next |
| Bootstrap installers | active | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 closed; M002 blocked | archive content/finalization and consumer-owned extraction boundary |
| CI/release orchestration | active | `plans/subsystems/ci-release-orchestration-roadmap.md` | M001 closed; M002 remains blocked on Build M003/M004 interfaces | Build qualification interfaces and explicit staging adapter plan |
| Eggup interoperability | active | `plans/subsystems/eggup-interoperability-roadmap.md` | Eggpack M001a + Eggup adapter M001/M001a closed | M003 real-consumer adoption ready to plan |
| Ecosystem adoption | proposed | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 blocked | required core pieces |

## Dependency-ready implementation work

Current dependency-ready implementation work:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Release manifest | M002 final-artifact builder | closed | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | Closure `plans/closure/release-manifest/002-status.md` |
| Build and qualification | M001 PackConfig/ReleasePlan | closed | `plans/implementation/build-qualification/001-pack-config-and-release-plan.md` | Closure `plans/closure/build-qualification/001-status.md` |
| Build and qualification | M002 native/cross builder seam | closed (historical) | `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md` | Closure `plans/closure/build-qualification/002-status.md`; post-closure Windows stability finding tracked by M002a |
| Build and qualification | M002a Windows qualification stability | closed | `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md` | Closure `plans/closure/build-qualification/002a-status.md`; three first-attempt hosted Windows runs passed |
| Bootstrap installers | M001 generator/direct fixtures | closed | `plans/implementation/bootstrap-installers/001-direct-installer-generator.md` | Closure `plans/closure/bootstrap-installers/001-status.md` |
| CI orchestration | M001 CIPlan/GitHub renderer | closed | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | Closure `plans/closure/ci-release-orchestration/001-status.md`; hosted run 36040032768 passes all lanes |
| Eggup interoperability | M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | Closure `plans/closure/eggup-interoperability/001-status.md`; post-closure projection defect tracked by M001a |
| Eggup interoperability | M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | Closure `plans/closure/eggup-interoperability/001a-status.md`; corrected pairwise fixture baseline |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| Build and qualification | M003 qualification execution | ready to plan | M002 and M002a closed; stable candidate evidence interface |
| Build and qualification | M004 finalization/aggregation | blocked | M003 closure + Manifest M002 |
| CI orchestration | M002 qualification/aggregation gates + drift CLI | blocked | CI M001 + Build M003/M004 interfaces |
| CI orchestration | M003 draft release staging | blocked | CI M002 + explicit staging adapter plan |
| Eggup interoperability | M002 optional adapter | closed / qualified | Eggup implementation `5fbb66853bdad59aaf2bd3c7bb43a43492d0b6ef`; corrective implementation `19935ec3610a5238af33a9d4f05a14925ceac25c`; closure `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001a-status.md` |
| Eggup interoperability | M003 real-consumer adoption | ready to plan | select a real consumer currently owning duplicated manifest-to-update mapping |
| Adoption | eggsact/stegoeggo | blocked | contract + required generator/CI pieces |
| Provenance/authenticity | future | planned | manifest/build evidence; trust ADR required |
| Python/wheel adapters | future | planned | native release pipeline maturity |

Build/Qualification M002 remains historical closure evidence, and M002a closed the Windows stability corrective after three clean first-attempt hosted runs. CI Orchestration M001 remains closed. Build M003 is now registered and ready for implementation; Build M004 and CI M002/M003 remain blocked until M003/M004 evidence/finalization interfaces close.

## Immediate execution graph

```text
Eggup distribution M003 [CLOSED / FROZEN PREDECESSOR]
               |
               v
Eggpack contract M001 [CLOSED]
               |
               v
contract conformance M002 [CLOSED: M003 PORT QUALIFIED]
        |                      |
        |                      +--> Eggup M004 retire eggup-dist [CLOSED]
        v
ReleaseManifest M001 [CLOSED HISTORICALLY]
        |
        v
Manifest M001a corrective [CLOSED: target-local installs, global artifact filenames]
        |
        +--> Manifest M002 [CLOSED; establishes eggpack-core and final-byte manifest builder]
        |        |
        |        `--> build/qualification M001 [CLOSED; ReleasePlan interface established]
        |                         |
        |                         v
        |              ADR-0004 [ACCEPTED: first-party Cargo/zigbuild]
        |                         |
        |                         v
        |              build/qualification M002 [CLOSED HISTORICALLY]
        |                         |
        |                         v
        |              build/qualification M002a [CLOSED: WINDOWS STABILITY QUALIFIED]
        |                         |
        |                         +--> qualification M003 [READY; PLAN REGISTERED]
        |                         `--> CI Orchestration M001 [CLOSED; consumes M002 interface]
        |                                      |
        |                                      `--> CI M002 [BLOCKED on qualification/finalization]
        +--> bootstrap installers M001 [CLOSED; direct first-install generator]
        `--> Eggup interoperability M001 [CLOSED HISTORICALLY]
                       |
                       v
             Eggup interoperability M001a [CLOSED]
                       |
                       `--> Eggup adapter M001 [CLOSED HISTORICALLY]
                                  |
                                  v
                           Eggup adapter M001a [CLOSED]
                                  |
                                  `--> consumer adoption [READY TO PLAN]

External dist 0.33 spike [CLOSED, disposition C] --> prior art only
```

## Initial architecture constraints

- Rust baseline: 1.89 unless changed by ADR.
- `eggpack-contract` migration is fidelity-first; no opportunistic schema redesign.
- `eggup-dist` was frozen through the migration and has now been removed by closed Eggup M004 after Eggpack Contract M002 qualification.
- concrete release manifests describe final bytes only.
- generated CI is checked in and drift-checked.
- staging/qualification does not automatically publish.
- checksums are integrity, not authenticity.
- no external backend is production-authorized; ADR-0004 authorizes only the first-party Cargo/cargo-zigbuild adapter boundary.
- Eggup core must remain independent of producer tooling.

## Next handoff

Manifest M002, Build/Qualification M001/M002/M002a, CI Orchestration M001, Bootstrap Installers M001, and Eggup Interoperability M001a retain their closure records. Build/Qualification M003 is registered at `plans/implementation/build-qualification/003-qualification-execution-and-evidence.md` and is the next producer implementation handoff. Build M004 and CI M002/M003, staging/publication, and ecosystem adoption remain blocked until qualification/finalization interfaces close. Eggup M003 real-consumer adapter adoption remains an independent ready branch, not the producer critical path.

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
