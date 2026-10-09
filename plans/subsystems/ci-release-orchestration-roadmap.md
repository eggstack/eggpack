# CI and Release Orchestration Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#14-ci-and-workflow-generation`
- `plans/000-long-term-specification.md#15-publication-policy`
- `plans/002-long-term-roadmap.md#phase-7--checked-in-ci-generation`
- `plans/002-long-term-roadmap.md#phase-8--release-staging-and-human-publication-gate`

Related ADRs:

- `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`;
- `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`

## 1. Purpose and ownership boundary

Own provider-neutral release graph projection, deterministic checked-in GitHub Actions generation, drift checking, aggregation gates, and later draft-release staging.

It must not become a general CI language or automatically publish package registries.

## 2. Work classification

### Invariants

- generated workflow is deterministic/reviewable;
- checked-in config/contract remains authority;
- build jobs default read-only;
- write permissions isolated to explicit staging jobs;
- required qualification lanes gate aggregation;
- public immutable releases are not silently replaced;
- publication is explicit.

### Capabilities

- `eggpack ci generate`;
- `eggpack ci check`;
- preflight/build/qualify/aggregate graph;
- release artifact collection;
- draft staging.

### Infrastructure

- CIPlan;
- GitHub Actions renderer;
- action/tool pin data;
- static workflow guards.

## 3. Non-goals

- replacing ordinary project CI;
- arbitrary user-authored workflow language;
- hidden central workflow authority;
- automatic crates.io/PyPI publication.

## 4. Current state

Eggstack repositories contain duplicated release workflows. Eggserve's large matrix and validator scripts demonstrate why target authority and workflow guards should be generated from data; eggsact/stegoeggo demonstrate simpler repeated release graphs.

## 5. Target architecture

```text
ReleasePlan -> CIPlan -> GitHub renderer -> checked-in release.yml
                         |
                         +-> ci check (re-render/diff)
```

## 6. Dependency graph

Hard/interface dependencies: Build/qualification M001 ReleasePlan + contract/manifest are closed. ADR-0004 and closed Build M002/M002a define the stable build-binding/command seam. Build M003 and M004 are closed at `plans/closure/build-qualification/003-status.md` and `plans/closure/build-qualification/004-status.md`; CI M002 is historical with its M002a corrective closed at `plans/closure/ci-release-orchestration/002a-status.md`.

## 7. Milestones

M001 CIPlan + deterministic GitHub renderer.

Implementation plan: `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md`.

M002 qualification/aggregation gates and drift check.

Implementation plan: `plans/implementation/ci-release-orchestration/002-qualification-aggregation-gates-and-drift-cli.md`.

M003 draft GitHub Release staging adapter, split into M003a provider/payload machinery and M003b generated staging-job integration/operational qualification.

M003a implementation plan: `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md`.

M003b implementation plan: `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md`.

M003c corrective plan: `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md`.

M003d consumer composition plan: `plans/implementation/ci-release-orchestration/003d-consumer-release-composition-seam.md`.

M003e execution-wiring corrective plan: `plans/implementation/ci-release-orchestration/003e-generated-release-execution-wiring-corrective.md`.

M003f tool-install corrective plan: `plans/implementation/ci-release-orchestration/003f-generated-tool-install-command-corrective.md`.

M003g live-qualification corrective plan: `plans/implementation/ci-release-orchestration/003g-live-qualification-corrective.md`.

M003h status-reconciliation/main-integration corrective plan: `plans/implementation/ci-release-orchestration/003h-live-qualification-status-reconciliation-and-main-integration.md`.

M003i identity-separation corrective: `plans/implementation/ci-release-orchestration/003i-explicit-tag-and-manifest-release-identity-corrective.md` (**closed** at `plans/closure/ci-release-orchestration/003i-status.md`; implementation `3ddac9d84fc2e10b8451d38672f66b02e06724df`, hosted run `37813044035`, all four lanes green). The immutable producer pin unblocked the separate wg-basic Distribution M003 for its consumer-owned implementation and qualification.

M003j dispatch-tag shell-injection corrective: `plans/implementation/ci-release-orchestration/003j-dispatch-tag-shell-injection-corrective.md` (**closed** at `plans/closure/ci-release-orchestration/003j-status.md`; implementation `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`, hosted run `37987890305`, all four lanes green). Dispatch tags are bounded canonical data validated before resolver/stage checkout; direct dispatch-tag expressions in `run:` are rejected. wg-basic Release Readiness R001 is unblocked to repin/regenerate under its own plan.

## 8. Cross-cutting requirements

Least privilege, pinned actions, explicit timeouts, concurrency controls, artifact provenance, no secret leakage in generated summaries.

## 9. Verification strategy

Golden workflows, parse/lint, deterministic regeneration, mutation tests proving `ci check` catches drift, permission audits, fixture aggregate failures.

## 10. Risks and decision points

GitHub syntax evolves; keep provider layer isolated. Avoid encoding all Actions features.

## 11. Completion definition

At least two repos replace hand-maintained release workflow matrices with generated checked-in CI while retaining equal or stronger qualification evidence.

## 12. Milestone status

Build/Qualification M001-M006 and CI M001/M002/M002a/M003c-M003g are closed on the live-qualified candidate branch. M003h reconciles post-live status and integrates the qualified branch onto `main`; its closure record captures the integration evidence. Historical M003a/M003b closure statements remain intact. Eggsact M001 completed the maintainer-authorized live qualification: run `36652731202` created and later published release `v1.2.7` with 15 assets. M003b is **closed** as of Planning Hygiene M001 (2026-10-04): live draft creation, exact inventory, draft-only behavior, and no-clobber refusal were already proven, and the final exact byte-identical rerun-reuse condition is now discharged by eggsact Distribution M005a — rehearsal run `36886042696` attempt 2 reused draft `401132612` with `created: false, uploaded: 0, reused: 15` and identical digests. The Windows PE/PDB nondeterminism was consumer-owned throughout and was fixed product-side in eggsact, not by an Eggpack change. **Phase 8 is satisfied**; publication remained a separate human action in every case. See `plans/closure/ci-release-orchestration/003b-status.md` §6.


| Milestone | Status | Implementation plan | Closure record | Blockers / sequencing |
|---|---|---|---|---|
| M001 CIPlan + GitHub renderer | closed | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | `plans/closure/ci-release-orchestration/001-status.md` | Closed after package, local, and Linux stable/MSRV/macOS/Windows hosted checks |
| M002 qualification/aggregation gates + drift CLI | closed historically; corrective closed | `plans/implementation/ci-release-orchestration/002-qualification-aggregation-gates-and-drift-cli.md` | `plans/closure/ci-release-orchestration/002-status.md` | Post-closure generated-workflow execution defect corrected and qualified by M002a |
| M002a generated workflow execution wiring | closed | `plans/implementation/ci-release-orchestration/002a-generated-workflow-execution-wiring-corrective.md` | `plans/closure/ci-release-orchestration/002a-status.md` | Closed on implementation `4d2270a` + hosted run 36154905956 (attempt 1, all lanes green) via closeout pass `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md` |
| M003a local staging payload + GitHub draft adapter | closed historically; M003c corrective closed | `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md` | `plans/closure/ci-release-orchestration/003a-status.md` | Historical closure retained; bounded transfer, pagination, exact origin, and query encoding corrected by M003c |
| M003b generated draft staging job + operational qualification | closed | `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md` | `plans/closure/ci-release-orchestration/003b-status.md` | All §13 criteria met. Draft/inventory/draft-only/no-clobber proven by eggsact `v1.2.7` run 36652731202; exact byte-identical rerun reuse proven by eggsact M005a rehearsal run 36886042696 attempt 2 (`created: false, uploaded: 0, reused: 15`). Phase 8 satisfied; discharge recorded in the closure record §6 |

| M003c staging source identity + bounded transfer corrective | closed | `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md` | `plans/closure/ci-release-orchestration/003c-status.md`; implementation `5c28099`; hosted run 36213316240 passed all lanes | Source identity, bounded transfer, pagination, exact origin, and query encoding all closed; M003b and Phase 8 have since closed on the rerun-reuse receipt |

| M003d consumer release composition seam | closed | `plans/implementation/ci-release-orchestration/003d-consumer-release-composition-seam.md` | `plans/closure/ci-release-orchestration/003d-status.md` | reusable static workflow/runtime identity, product wrapper presentation, and bounded consumer-validator seam landed; Build M005 and Build M006 are both closed, so every first-consumer prerequisite is now satisfied and Ecosystem M001 is the only remaining step |

| M003e generated release execution wiring corrective | closed | `plans/implementation/ci-release-orchestration/003e-generated-release-execution-wiring-corrective.md` | `plans/closure/ci-release-orchestration/003e-status.md`; implementation `b9062d4`; hosted run 36572608484 passed all lanes | first-consumer end-to-end review proved no generated workflow can execute; renderer installs tool before use and creates output dirs, CLI absolutizes generated relative paths; post-closure live dispatch exposed the rejected `-p` install flag (M003f) |

| M003f generated tool-install command corrective | closed | `plans/implementation/ci-release-orchestration/003f-generated-tool-install-command-corrective.md` | `plans/closure/ci-release-orchestration/003f-status.md`; implementation `c190e77`; hosted run 36632209736 passed all lanes | `cargo install --git` rejected `-p`; package now positional; live dispatch made no mutation; the second dispatch exposed F9/F10 (M003g) |

| M003g live qualification failures corrective | closed | `plans/implementation/ci-release-orchestration/003g-live-qualification-corrective.md` | `plans/closure/ci-release-orchestration/003g-status.md`; implementations `5acda73`/`4b28820`/`5ac5b83`/`e5c81f2`; hosted runs 36638321253, 36642184204, 36646814734, 36652203168 all lanes | cross-tool PATH, invalid zigbuild check, artifact exec-bit stripping, consumer validation against failed evidence, stage arity, swallowed staging error, and boolean `make_latest` all corrected; live run 36652731202 staged a complete 15-asset draft for eggsact `v1.2.7`, which the maintainer then published |
| M003h live qualification status reconciliation + main integration | closed | `plans/implementation/ci-release-orchestration/003h-live-qualification-status-reconciliation-and-main-integration.md` | `plans/closure/ci-release-orchestration/003h-status.md` | current `main@404f63ec` was merged by `609d5fb`; status reconciled, full verification passed, and `main` fast-forwarded without force |
| M003i explicit tag and manifest release identity corrective | closed | `plans/implementation/ci-release-orchestration/003i-explicit-tag-and-manifest-release-identity-corrective.md` | `plans/closure/ci-release-orchestration/003i-status.md`; implementation `3ddac9d84fc2e10b8451d38672f66b02e06724df`; hosted run `37813044035` passed all four lanes | Finite checked-in opt-in stable-tag mapping with explicitly versioned runtime envelope; legacy exact-tag workflow bytes and ReleaseManifest v1 remain unchanged; wg-basic M003 may resume against the immutable producer SHA |
| M003j dispatch tag shell-injection corrective | closed | `plans/implementation/ci-release-orchestration/003j-dispatch-tag-shell-injection-corrective.md` | `plans/closure/ci-release-orchestration/003j-status.md`; implementation `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`; hosted run `37987890305` passed Linux stable/MSRV, macOS, and Windows | wg-basic Release Readiness R001 may resume with this producer pin; consumer regeneration and authorization remain separate |

## 13. 2026-10-08 corrective addendum — M003i

**M003i closed** (`plans/closure/ci-release-orchestration/003i-status.md`; implementation `3ddac9d84fc2e10b8451d38672f66b02e06724df`, hosted run `37813044035`). The corrective addresses `dbowm91/wg-basic@125a6a7975a36c65f9b380d8a00cda3604a9241e` Phase 10 M003: exact source tag `vX.Y.Z` is now independently bound to manifest identity `X.Y.Z` through reusable resolution, per-job source checks, finalization, and exact-tag staging. Default exact-tag behavior and ReleaseManifest v1 remain unchanged. The downstream wg-basic M003 may resume with this immutable Eggpack pin; its implementation, native qualification, and closure remain separately owned. Earlier CI M003a–M003h historical closures are unaffected.
