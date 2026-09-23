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

- repository began empty on 2026-09-22; first production workspace/crate implementation is underway;
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
| External backend evaluation | active | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 ready | none |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M001 blocked | contract M001 + conformance interface |
| Build and qualification | proposed | `plans/subsystems/build-qualification-roadmap.md` | M001 blocked | contract/manifest + backend disposition |
| Bootstrap installers | proposed | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 blocked | conformance + manifest |
| CI/release orchestration | proposed | `plans/subsystems/ci-release-orchestration-roadmap.md` | M001 blocked | build/qualification architecture |
| Eggup interoperability | proposed | `plans/subsystems/eggup-interoperability-roadmap.md` | M001 blocked | manifest v1 |
| Ecosystem adoption | proposed | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 blocked | required core pieces |

## Dependency-ready implementation work

Two initial workstreams are dependency-ready and SHOULD proceed in parallel:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Contract and conformance | M001 workspace + DistributionContract v1 import | closed | `plans/implementation/contract-conformance/001-workspace-and-distribution-contract-v1-import.md` | — |
| Contract and conformance | M002 release and installer conformance validators | ready | `plans/implementation/contract-conformance/002-release-and-installer-conformance-validators.md` | M001 closure |
| External backend evaluation | M001 dist 0.33 capability spike | ready | `plans/implementation/external-backend-evaluation/001-dist-0.33-capability-and-interoperability-spike.md` | none |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| Release manifest | M001 schema/domain | blocked | conformance M002 expected-file interface |
| Build/qualification | M001 PackConfig/ReleasePlan | blocked | manifest/contract + backend M001 disposition |
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
Eggpack contract M001 [READY]
               |
               v
contract conformance M002
               |
               v
ReleaseManifest v1
       |               |
       v               v
 Eggup seam       bootstrap/CI
       \               /
        \             /
         build/adoption

External dist 0.33 spike [READY, parallel]
               |
               v
backend-selection disposition/ADR
               |
               +----> build/qualification architecture
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

The contract M001 implementation is closing; the independent backend spike remains next in the requested execution order:

1. `plans/implementation/contract-conformance/001-workspace-and-distribution-contract-v1-import.md`;
2. `plans/implementation/external-backend-evaluation/001-dist-0.33-capability-and-interoperability-spike.md`.

Contract M001 is closed; its M002 validator successor is ready. The external backend spike is independent and is the next execution handoff. Any production external-backend adoption remains blocked until the dist spike closes and a backend-selection ADR is reviewed.

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
