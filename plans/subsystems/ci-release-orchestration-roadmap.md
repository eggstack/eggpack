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

Hard/interface dependencies: Build/qualification M001 ReleasePlan + contract/manifest are closed. ADR-0004 and Build M002 define the shared build-binding/command seam. M002 implementation has landed, but Windows native execution qualification is blocked because the current hosted runner lacks a usable MSVC linker. CI M001 is blocked until that platform execution boundary is resolved; do not duplicate builder semantics or begin the renderer against an unqualified Windows lane.

## 7. Milestones

M001 CIPlan + deterministic GitHub renderer (blocked pending Build M002 Windows native execution qualification).

Implementation plan: `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md`.

M002 qualification/aggregation gates and drift check.

M003 draft GitHub Release staging adapter.

## 8. Cross-cutting requirements

Least privilege, pinned actions, explicit timeouts, concurrency controls, artifact provenance, no secret leakage in generated summaries.

## 9. Verification strategy

Golden workflows, parse/lint, deterministic regeneration, mutation tests proving `ci check` catches drift, permission audits, fixture aggregate failures.

## 10. Risks and decision points

GitHub syntax evolves; keep provider layer isolated. Avoid encoding all Actions features.

## 11. Completion definition

At least two repos replace hand-maintained release workflow matrices with generated checked-in CI while retaining equal or stronger qualification evidence.

## 12. Milestone status

Build/Qualification M001 fixes the provider-neutral ReleasePlan interface. Build M002 and ADR-0004 define the shared native build-binding/command boundary, but required Windows native Cargo and timeout/cancellation execution evidence is blocked by the hosted runner's missing MSVC linker. CI Orchestration M001 remains blocked until M002 can close with that platform evidence. No publication/staging authority is authorized.


| Milestone | Status | Implementation plan | Closure record | Blockers / sequencing |
|---|---|---|---|---|
| M001 CIPlan + GitHub renderer | blocked | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | — | Build M002 Windows native execution qualification; hosted runner lacks usable MSVC linker |
| M002 qualification/aggregation gates + drift CLI | blocked | — | — | M001 closure + Build/Qualification M003/M004 interfaces |
| M003 draft release staging | blocked | — | — | M002 + explicit staging adapter plan |
