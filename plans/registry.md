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
- next-batch implementation handoffs registered: Manifest M002 `14b88e7c7de1caac9958d821ba0ebe7cb4350117`, Build/Qualification M001 `a6e33a71efc7bdbb2cce10b905c8a6e3f7dfeb68`, Bootstrap M001 `7101cd241f5c9dec5931817850b22361feed32e5`, Eggup Interop M001 `916c93a1ad2aed7e033bea6d961b35e7008d805e`;
- canonical architecture commit: `2022f44f4df9b0c2518ff22a53d87fd77f63992a`.

### Eggup predecessor

Eggup's current planning assigns producer distribution authority to Eggpack. The relevant immutable predecessor is distribution M003, not current-main incidental state.

Distribution predecessor evidence:

- M001 implementation: `889a234cbe7f461d92def3df45c83c06a7d257e5`;
- M002 strict template/collision corrective: `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`;
- M003 conformance implementation: `9941c58d7039410c728860f9e4e382881d4ccf54`;
- M003 closure: `4169c8021b447fe73c8ee3ea71a80a535c940f54`;
- `eggup-dist` is unpublished and frozen;
- Eggup distribution M004 was a blocked retirement plan and may execute only after Eggpack Contract M002 closes;
- Contract M002 has now ported the closed M003 behavior, authorizing Eggup M004 retirement.

### External backend evidence

- `axodotdev/cargo-dist` / `dist` latest reviewed release: 0.33.0 (September 2026);
- upstream demonstrates generated release machinery, installers, manifest, checksums, and GitHub Artifact Attestations;
- no production Eggpack backend decision has been made.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies / blockers |
|---|---|---|---|---|
| Contract and conformance | active | `plans/subsystems/contract-conformance-roadmap.md` | M002 closed; M003 planned | Eggup M004 retirement authorized |
| External backend evaluation | closed | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 closed (C) | no production backend adopted; new evidence/plan required to reopen |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M001a closed; M002 ready | M001/M001a and conformance M002 closed |
| Build and qualification | active | `plans/subsystems/build-qualification-roadmap.md` | M001 ready | Contract M001/M002, manifest M001/M001a, backend evaluation closed |
| Bootstrap installers | active | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 ready | Contract conformance M002 and manifest M001/M001a closed |
| CI/release orchestration | active | `plans/subsystems/ci-release-orchestration-roadmap.md` | M001 blocked | Build/qualification M001 interface |
| Eggup interoperability | active | `plans/subsystems/eggup-interoperability-roadmap.md` | M001 ready (Eggpack side) | Manifest M001/M001a and conformance maturity closed; Eggup-side plan required for Eggup changes |
| Ecosystem adoption | proposed | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 blocked | required core pieces |

## Dependency-ready implementation work

Current dependency-ready implementation work:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Release manifest | M002 final-artifact builder | ready | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | Contract conformance M002; Manifest M001/M001a closed |
| Build and qualification | M001 PackConfig/ReleasePlan | ready / ordered after core bootstrap | `plans/implementation/build-qualification/001-pack-config-and-release-plan.md` | semantic dependencies closed; extend `eggpack-core` after/with Manifest M002 bootstrap |
| Bootstrap installers | M001 generator/direct fixtures | ready | `plans/implementation/bootstrap-installers/001-direct-installer-generator.md` | Contract conformance M002; Manifest M001/M001a closed |
| Eggup interoperability | M001 Eggpack interface/fixtures | ready | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | Manifest M001/M001a + conformance M002 closed; Eggup changes require later Eggup plan |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| Eggup distribution | M004 retire eggup-dist | ready / authorized | Eggpack Contract M002 closed; execution remains in Eggup |
| CI orchestration | M001 CIPlan/renderer | blocked | Build/qualification M001 interface |
| Adoption | eggsact/stegoeggo | blocked | contract + required generator/CI pieces |
| Provenance/authenticity | future | planned | manifest/build evidence; trust ADR required |
| Python/wheel adapters | future | planned | native release pipeline maturity |

Milestones above with status `ready` have satisfied hard/interface dependencies; detailed implementation plans still need registration before execution handoff.

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
        |                      +--> Eggup M004 retire eggup-dist [AUTHORIZED]
        v
ReleaseManifest M001 [CLOSED HISTORICALLY]
        |
        v
Manifest M001a corrective [CLOSED: target-local installs, global artifact filenames]
        |
        +--> Manifest M002 [READY; establishes eggpack-core]
        |        |
        |        `--> build/qualification M001 [READY; extend core after bootstrap]
        +--> bootstrap installers M001 [READY; independent renderer crate]
        `--> Eggup interoperability M001, Eggpack side [READY; fixtures/interface only]

External dist 0.33 spike [CLOSED, disposition C] --> prior art only
```

## Initial architecture constraints

- Rust baseline: 1.89 unless changed by ADR.
- `eggpack-contract` migration is fidelity-first; no opportunistic schema redesign.
- `eggup-dist` is frozen after closed Eggup M003 and is removed only after Eggpack Contract M002 migration closure through the already-registered Eggup M004 retirement plan.
- concrete release manifests describe final bytes only.
- generated CI is checked in and drift-checked.
- staging/qualification does not automatically publish.
- checksums are integrity, not authenticity.
- no external backend is production-authorized yet.
- Eggup core must remain independent of producer tooling.

## Next handoff

The next implementation batch is fully registered. Execute Manifest M002 first (or first within a coordinated branch) because it establishes the canonical `eggpack-core` package; then apply Build/Qualification M001 to extend that core planning surface. Bootstrap Installers M001 and Eggpack-side Eggup Interoperability M001 are independently ready and may proceed in parallel because they consume the already-closed Contract/Manifest interfaces. Do not author CI Orchestration M001 until Build/Qualification M001 closes and fixes the ReleasePlan interface. Do not author the Eggup adapter implementation in Eggup until Eggpack Interoperability M001 closes and its fixture/mapping contract is reviewed. Ecosystem adoption remains blocked on the required producer/installer/CI capabilities. The dist spike remains closed disposition C; no backend-selection ADR or production adoption is authorized.

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
