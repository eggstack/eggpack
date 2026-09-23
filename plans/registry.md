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
- current reviewed planning/implementation baseline before this registry update: `6ace467d1c3ecd9f76e223799017039504989eac`;
- canonical architecture commit: `2022f44f4df9b0c2518ff22a53d87fd77f63992a`.

### Eggup predecessor

Current reviewed Eggup main: `cf5b3d3819c168eb2dbf841daa8332f3eb28c915`.

Distribution predecessor evidence:

- M001 implementation: `889a234cbe7f461d92def3df45c83c06a7d257e5`;
- M002 strict template/collision corrective: `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`;
- `eggup-dist` is unpublished;
- Eggup M003 conformance validator plan is ready but should migrate to Eggpack after Eggpack M001 rather than expand producer ownership in Eggup.

### External backend evidence

- `axodotdev/cargo-dist` / `dist` latest reviewed release: 0.33.0 (September 2026);
- upstream demonstrates generated release machinery, installers, manifest, checksums, and GitHub Artifact Attestations;
- no production Eggpack backend decision has been made.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies / blockers |
|---|---|---|---|---|
| Contract and conformance | active | `plans/subsystems/contract-conformance-roadmap.md` | M002 ready | M001 closed with hosted CI |
| External backend evaluation | closed | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 closed (C) | no production backend adopted; new evidence/plan required to reopen |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M001 blocked | contract M002 expected-file interface |
| Build and qualification | proposed | `plans/subsystems/build-qualification-roadmap.md` | M001 blocked | Contract M002 + manifest M001; dist disposition C is closed |
| Bootstrap installers | proposed | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 blocked | conformance + manifest |
| CI/release orchestration | proposed | `plans/subsystems/ci-release-orchestration-roadmap.md` | M001 blocked | build/qualification architecture |
| Eggup interoperability | proposed | `plans/subsystems/eggup-interoperability-roadmap.md` | M001 blocked | manifest v1 |
| Ecosystem adoption | proposed | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 blocked | required core pieces |

## Dependency-ready implementation work

Current dependency-ready implementation work:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Contract and conformance | M002 release and installer conformance validators | ready | `plans/implementation/contract-conformance/002-release-and-installer-conformance-validators.md` | M001 closure |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| Release manifest | M001 schema/domain | blocked | conformance M002 expected-file interface |
| Build/qualification | M001 PackConfig/ReleasePlan | blocked | Contract M002 + Release Manifest M001 |
| Bootstrap installers | M001 direct generator | blocked | conformance + manifest |
| CI orchestration | M001 CIPlan/renderer | blocked | build-plan interface |
| Eggup interoperability | M001 cross-repo interface fixtures | blocked | manifest v1 |
| Adoption | eggsact/stegoeggo | blocked | contract + required generator/CI pieces |
| Provenance/authenticity | future | planned | manifest/build evidence; trust ADR required |
| Python/wheel adapters | future | planned | native release pipeline maturity |

## Immediate execution graph

```text
Eggup eggup-dist M001/M002 evidence
               |
               v
Eggpack contract M001 [CLOSED]
               |
               v
contract conformance M002 [READY]
               |
               v
ReleaseManifest M001 [BLOCKED]
       |               |
       v               v
 Eggup seam       bootstrap/CI
       \               /
        \             /
         build/adoption

External dist 0.33 spike [CLOSED, disposition C] --> prior art only
```

## Initial architecture constraints

- Rust baseline: 1.89 unless changed by ADR.
- `eggpack-contract` migration is fidelity-first; no opportunistic schema redesign.
- `eggup-dist` is not removed until Eggpack migration closure and a separate Eggup cleanup plan.
- concrete release manifests describe final bytes only.
- generated CI is checked in and drift-checked.
- staging/qualification does not automatically publish.
- checksums are integrity, not authenticity.
- no external backend is production-authorized yet.
- Eggup core must remain independent of producer tooling.

## Next handoff

`plans/implementation/contract-conformance/002-release-and-installer-conformance-validators.md` is the sole dependency-ready implementation handoff for the next round and is rebaselined to the current closed-M001/closed-dist state. Do not pre-author Release Manifest M001 until Contract M002 closes and proves the expected-file/conformance interface. The dist spike is closed with disposition C; no backend-selection ADR or production adoption is authorized.

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
