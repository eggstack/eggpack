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

## Current evidence baselines

### Eggpack

- repository began empty on 2026-09-22; Contract M001 and the dist 0.33 evaluation are closed;
- last reviewed code/implementation baseline before the planning-only producer/consumer reorientation: `e3452263225fa1ea262e03b557f40b395e6a52d8`;
- producer/consumer reorientation baseline: `5793ccecf5139d9b7b250534703ec5ee84fe44b8`; subsequent planning-only corrections preserve that ownership/cutover model;
- Contract M002 implementation: `a36803a7c34cc5b273559520bf99cb2400cc183a`; closure record: `plans/closure/contract-conformance/002-status.md`;
- Release Manifest M001 implementation: `b5df057a0ab8d30664aeccc26aa7944678626930`; historical closure: `plans/closure/release-manifest/001-status.md`;
- Release Manifest M001a implementation: `20a3084bbfbc7fac03b70e0c397f9059186169c8`; closure: `plans/closure/release-manifest/001a-status.md`. Install-name collisions are target-local; release-artifact filename collisions remain manifest-global;
- Eggup interface historical baseline: `99c9040a5d106cfa46a4ba02f9fa0cee8653166c`; corrective M001a implementation: `8d9264b3c224f3f05a061f4038b0f328b1c5c95e`; closure: `plans/closure/eggup-interoperability/001a-status.md`; hosted CI: `35998756491`; pinned Eggup API source: `eggstack/eggup@2cab1f97ef30fa347c2030da321462459672c521`;
- next-batch implementation handoffs registered: Manifest M002 `14b88e7c7de1caac9958d821ba0ebe7cb4350117`, Build/Qualification M001 `a6e33a71efc7bdbb2cce10b905c8a6e3f7dfeb68`, Bootstrap M001 `7101cd241f5c9dec5931817850b22361feed32e5`, Eggup Interop M001 `916c93a1ad2aed7e033bea6d961b35e7008d805e`;
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
- no production Eggpack backend decision has been made.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies / blockers |
|---|---|---|---|---|
| Contract and conformance | active | `plans/subsystems/contract-conformance-roadmap.md` | M002 closed; M003 planned | Eggup M004 retirement closed; consumer evidence gates M003 |
| External backend evaluation | closed | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 closed (C) | no production backend adopted; new evidence/plan required to reopen |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M002 closed; M003 planned | M003 requires a real consumer |
| Build and qualification | active | `plans/subsystems/build-qualification-roadmap.md` | M001 closed; M002 blocked | backend/adapter decision at implementation boundary |
| Bootstrap installers | active | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 closed; M002 blocked | archive content/finalization and consumer-owned extraction boundary |
| CI/release orchestration | active | `plans/subsystems/ci-release-orchestration-roadmap.md` | M001 ready to plan | Build/Qualification M001 ReleasePlan interface closed |
| Eggup interoperability | active | `plans/subsystems/eggup-interoperability-roadmap.md` | M001a closed; Eggup adapter plan registered | Eggup handoff: `eggstack/eggup@2ad8ce3968128cbc8d1065d8334a106f82b3a9bd`, `plans/implementation/eggpack-manifest-interoperability/001-release-manifest-v1-adapter.md` |
| Ecosystem adoption | proposed | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 blocked | required core pieces |

## Dependency-ready implementation work

Current dependency-ready implementation work:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Release manifest | M002 final-artifact builder | closed | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | Closure `plans/closure/release-manifest/002-status.md` |
| Build and qualification | M001 PackConfig/ReleasePlan | closed | `plans/implementation/build-qualification/001-pack-config-and-release-plan.md` | Closure `plans/closure/build-qualification/001-status.md` |
| Bootstrap installers | M001 generator/direct fixtures | closed | `plans/implementation/bootstrap-installers/001-direct-installer-generator.md` | Closure `plans/closure/bootstrap-installers/001-status.md` |
| Eggup interoperability | M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | Closure `plans/closure/eggup-interoperability/001-status.md`; post-closure projection defect tracked by M001a |
| Eggup interoperability | M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | Closure `plans/closure/eggup-interoperability/001a-status.md`; corrected pairwise fixture baseline |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| CI orchestration | M001 CIPlan/renderer | ready to plan | Build/Qualification M001 ReleasePlan interface closed; register an implementation handoff before execution |
| Eggup interoperability | M002 optional adapter | ready for Eggup implementation | Eggup-owned plan registered at `eggstack/eggup@2ad8ce3968128cbc8d1065d8334a106f82b3a9bd` |
| Adoption | eggsact/stegoeggo | blocked | contract + required generator/CI pieces |
| Provenance/authenticity | future | planned | manifest/build evidence; trust ADR required |
| Python/wheel adapters | future | planned | native release pipeline maturity |

The ready implementation milestones have registered handoffs; CI Orchestration M001 is only ready for a plan to be authored. Follow the batch sequencing below.

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
        |                         `--> CI Orchestration M001 [READY TO PLAN]
        +--> bootstrap installers M001 [CLOSED; direct first-install generator]
        `--> Eggup interoperability M001 [CLOSED HISTORICALLY]
                       |
                       v
             Eggup interoperability M001a [CLOSED]
                       |
                       `--> Eggup adapter M001 [READY FOR IMPLEMENTATION; Eggup plan registered]

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
- no external backend is production-authorized yet.
- Eggup core must remain independent of producer tooling.

## Next handoff

Manifest M002, Build/Qualification M001, Bootstrap Installers M001, and Eggup Interoperability M001a are closed. Eggup Interoperability M001 remains historical closure evidence, with the M001a corrective closure now documenting the corrected pairwise direct/bundle/archive projection/manifest baseline. The Eggup-owned adapter implementation plan is now registered at `eggstack/eggup@2ad8ce3968128cbc8d1065d8334a106f82b3a9bd` and is ready for implementation against the M001a closure baseline. CI Orchestration M001 is independently ready for plan authoring and is unaffected by this corrective. Bootstrap M002 remains blocked on qualified archive finalization/extraction boundaries. Ecosystem adoption remains blocked on the required producer/installer/CI capabilities. The dist spike remains closed disposition C; no backend-selection ADR or production adoption is authorized.

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
