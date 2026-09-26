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

## 8. Cross-cutting requirements

Least privilege, pinned actions, explicit timeouts, concurrency controls, artifact provenance, no secret leakage in generated summaries.

## 9. Verification strategy

Golden workflows, parse/lint, deterministic regeneration, mutation tests proving `ci check` catches drift, permission audits, fixture aggregate failures.

## 10. Risks and decision points

GitHub syntax evolves; keep provider layer isolated. Avoid encoding all Actions features.

## 11. Completion definition

At least two repos replace hand-maintained release workflow matrices with generated checked-in CI while retaining equal or stronger qualification evidence.

## 12. Milestone status

Build/Qualification M001-M004, CI M001/M002/M002a, and corrective M003c are closed. M003a and M003b retain historical closure evidence with corrective annotations. M003b live draft qualification is ready to resume; Phase 8 exit and consumer adoption remain blocked on that operational evidence.


| Milestone | Status | Implementation plan | Closure record | Blockers / sequencing |
|---|---|---|---|---|
| M001 CIPlan + GitHub renderer | closed | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | `plans/closure/ci-release-orchestration/001-status.md` | Closed after package, local, and Linux stable/MSRV/macOS/Windows hosted checks |
| M002 qualification/aggregation gates + drift CLI | closed historically; corrective closed | `plans/implementation/ci-release-orchestration/002-qualification-aggregation-gates-and-drift-cli.md` | `plans/closure/ci-release-orchestration/002-status.md` | Post-closure generated-workflow execution defect corrected and qualified by M002a |
| M002a generated workflow execution wiring | closed | `plans/implementation/ci-release-orchestration/002a-generated-workflow-execution-wiring-corrective.md` | `plans/closure/ci-release-orchestration/002a-status.md` | Closed on implementation `4d2270a` + hosted run 36154905956 (attempt 1, all lanes green) via closeout pass `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md` |
| M003a local staging payload + GitHub draft adapter | closed historically; M003c corrective closed | `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md` | `plans/closure/ci-release-orchestration/003a-status.md` | Historical closure retained; bounded transfer, pagination, exact origin, and query encoding corrected by M003c |
| M003b generated draft staging job + operational qualification | conditionally closed; M003c corrective closed | `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md` | `plans/closure/ci-release-orchestration/003b-status.md` | Exact-source and tag-source defects corrected; live draft is the only remaining M003b condition |

| M003c staging source identity + bounded transfer corrective | closed | `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md` | `plans/closure/ci-release-orchestration/003c-status.md`; implementation `5c28099`; hosted run 36213316240 passed all lanes | M003b live draft qualification is ready to resume; Phase 8 and consumer adoption still require its evidence |
