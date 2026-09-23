# External Backend Evaluation Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#17-external-backend-policy`
- `plans/002-long-term-roadmap.md#phase-e1--external-dist-capabilityinteroperability-spike`

Related ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- any future backend-selection ADR.

## 1. Purpose and ownership boundary

This subsystem determines whether existing release tooling can implement part of Eggpack's producer pipeline without becoming Eggpack's canonical domain model.

The first candidate is `axodotdev/cargo-dist` / `dist`.

## 2. Work classification

### Invariants

- external backend types never become canonical Eggpack identity;
- backend use cannot replace Eggup updater ownership;
- a backend cannot weaken qualification truthfulness or publication gates;
- backend-specific missing capabilities remain explicit.

### Capabilities

- compare backend behavior to Eggpack requirements;
- normalize backend output experimentally;
- produce a disposition before committing to a large native engine.

### Infrastructure

- fixture repos/configs;
- comparison matrix;
- optional throwaway adapter prototypes.

### Polish

- none initially.

## 3. Non-goals

- adopting a production dependency during the spike;
- rewriting consumers;
- benchmarking every release tool;
- using an external updater.

## 4. Current state

Verified initial external baseline: `dist` 0.33.0, released 2026-09-10/11, with current upstream repository activity. Its own release demonstrates multiple Rust targets, checksums, installers, a `dist-manifest.json`, and GitHub Artifact Attestations.

The closed M001 spike found useful archive/build/install/CI prior art, but no production backend fit without substantial Eggpack-owned normalization and qualification. Disposition C is recorded in `plans/closure/external-backend-evaluation/001-status.md`. Eggpack has not adopted a backend.

## 5. Target architecture

No production backend architecture is selected. M001 recommends prior-art-only use of dist's target/archive/checksum/manifest patterns and an Eggpack-owned normalization boundary if a separately planned adapter is proposed.

Possible accepted shapes:

```text
Eggpack plan -> native backend
Eggpack plan -> dist backend -> normalize outputs -> Eggpack manifest
Eggpack plan -> mixed backend by target/package kind
```

## 6. Dependency graph

M001 spike was independent of Eggpack code foundation and is closed.

Production backend adoption remains unapproved. An ADR is required only if a future registered evaluation recommends adoption or partial adoption (M001 disposition A/B).

## 7. Milestones

### M001 — dist 0.33 capability/interoperability spike

Class: infrastructure / architecture evidence

Scenarios:

- eggsact/stegoeggo direct;
- Gregg sibling bundle;
- Egress archive pair;
- eggsearch ARMv7/Windows ARM64;
- explicit glibc floor;
- native/deferred/QEMU qualification;
- generated CI;
- installer generation;
- manifest inspection/normalization;
- attestations;
- disabling updater behavior.

Exit conditions: factual capability matrix, prototype evidence where needed, dependency/maintenance assessment, recommended disposition without production adoption.

### M002 — Backend selection ADR (conditional)

Class: invariant; not triggered by disposition C

Create only if a new evidence plan recommends adopting or partially adopting an external backend.

## 8. Cross-cutting requirements

No live production release publication in the spike. Use local/dry-run/sample fixtures.

Pin exact external version when measuring behavior.

## 9. Verification strategy

Capture commands, generated plans/workflows/manifests, target support, and incompatibilities. Prefer small synthetic fixtures derived from real repository layouts.

## 10. Risks and decision points

A highly capable tool can still be the wrong abstraction if Eggpack must wrap most behavior. Conversely, implementing generic release mechanics unnecessarily would create long-term maintenance debt.

## 11. Completion definition

The subsystem closes when maintainers can make an evidence-backed backend decision and the chosen path is recorded in an ADR or explicit rejection disposition.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 | closed | `plans/implementation/external-backend-evaluation/001-dist-0.33-capability-and-interoperability-spike.md` | `plans/closure/external-backend-evaluation/001-status.md` | — |
