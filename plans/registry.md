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
- Release Manifest M001 handoff rebaselined and tightened after M002 closure; plan commit: `e105cd5c6a410e7d025ce411c5c501ee0485f524`;
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
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M001 ready | Contract M001/M002 closed |
| Build and qualification | proposed | `plans/subsystems/build-qualification-roadmap.md` | M001 blocked | Release Manifest M001; dist disposition C is closed |
| Bootstrap installers | proposed | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 blocked | Release Manifest M001; conformance M002 closed |
| CI/release orchestration | proposed | `plans/subsystems/ci-release-orchestration-roadmap.md` | M001 blocked | build/qualification architecture |
| Eggup interoperability | proposed | `plans/subsystems/eggup-interoperability-roadmap.md` | M001 blocked | manifest v1 |
| Ecosystem adoption | proposed | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 blocked | required core pieces |

## Dependency-ready implementation work

Current dependency-ready implementation work:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Release manifest | M001 v1 domain and deterministic serialization | ready | `plans/implementation/release-manifest/001-release-manifest-v1-domain.md` | Contract M001/M002 closure |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| Eggup distribution | M004 retire eggup-dist | ready / authorized | Eggpack Contract M002 closed; execution remains in Eggup |
| Build/qualification | M001 PackConfig/ReleasePlan | blocked | Release Manifest M001 |
| Bootstrap installers | M001 direct generator | blocked | Release Manifest M001 |
| CI orchestration | M001 CIPlan/renderer | blocked | build-plan interface |
| Eggup interoperability | M001 cross-repo interface fixtures | blocked | manifest v1 |
| Adoption | eggsact/stegoeggo | blocked | contract + required generator/CI pieces |
| Provenance/authenticity | future | planned | manifest/build evidence; trust ADR required |
| Python/wheel adapters | future | planned | native release pipeline maturity |

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
ReleaseManifest M001 [READY]
        |
        +--> future Eggup manifest-consumer seam
        +--> bootstrap/CI/build/adoption

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

`plans/implementation/release-manifest/001-release-manifest-v1-domain.md` is the sole dependency-ready Eggpack implementation handoff for the next round. It is rebaselined to the formal Contract M002 closure and now requires ProductId + ReleaseId + SourceRevision, relationship-preserving direct/bundle/archive artifact/member identity, exact size/SHA-256 evidence, deterministic JSON, and bounded non-trust-bearing evidence references. Do not pre-author Manifest M002, Build/Qualification M001, or Bootstrap Installers M001 until Manifest M001 closes and fixes the concrete schema boundary. Eggup M004 retirement is authorized independently in Eggup. The dist spike remains closed with disposition C; no backend-selection ADR or production adoption is authorized.

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
