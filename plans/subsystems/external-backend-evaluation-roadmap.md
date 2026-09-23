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

That makes it too capable to ignore, but Eggstack has requirements that need explicit testing: sibling bundles, archive-member semantics, ARMv7/SBC targets, glibc floors, deferred-native/QEMU qualification, readable generated CI, manifest normalization, and separation from updater ownership.

## 5. Target architecture

No target backend architecture is selected yet.

Possible accepted shapes:

```text
Eggpack plan -> native backend
Eggpack plan -> dist backend -> normalize outputs -> Eggpack manifest
Eggpack plan -> mixed backend by target/package kind
```

## 6. Dependency graph

M001 spike is independent of Eggpack code foundation and may run in parallel with contract migration.

A production backend decision is hard-blocked on spike closure and requires an ADR.

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

### M002 — Backend selection ADR

Class: invariant

Created only if M001 supports adopting or partially adopting an external backend.

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
| M001 | ready | `plans/implementation/external-backend-evaluation/001-dist-0.33-capability-and-interoperability-spike.md` | — | — |
| M002 | blocked | — | — | M001 |
