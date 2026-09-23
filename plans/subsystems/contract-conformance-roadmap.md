# Contract and Conformance Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#6-portable-distribution-contract`
- `plans/000-long-term-specification.md#7-artifact-forms`
- `plans/002-long-term-roadmap.md#phase-1--repository-bootstrap-and-distributioncontract-v1-migration`
- `plans/002-long-term-roadmap.md#phase-2--contract-conformance-engine`

Related ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

## 1. Purpose and ownership boundary

This subsystem owns the smallest portable release-layout contract and pure conformance validation against it.

It consumes predecessor evidence from Eggup's unpublished `eggup-dist` crate.

It must not own build runners, network release discovery, archive extraction execution, publication, live installation, service policy, or version selection.

## 2. Work classification

### Invariants

- schema versions are explicit;
- unknown future schema majors fail typed;
- unknown fields fail rather than silently drift;
- target triples/aliases are globally unambiguous;
- unsupported target lookup never guesses;
- template grammar remains small and non-executable;
- final expanded release/install namespaces reject exact and ASCII-case collisions;
- archive member sources are relative, normalized, traversal-free literals;
- direct/bundle/archive remain the only v1 layout forms;
- checksum metadata is integrity-only;
- pure contract/conformance operations perform no network/process/filesystem mutation.

### Capabilities

- describe a release layout once;
- expand one target/release deterministically;
- derive required release files;
- validate observed release inventories;
- validate archive-member inventories;
- validate bootstrap/runtime mapping observations.

### Infrastructure

- `eggpack-contract`;
- parser/validator;
- expected-file model;
- inventory/report types;
- direct/bundle/archive fixtures.

### Polish

- diagnostics;
- fixture documentation;
- optional thin local-only CLI once library semantics settle.

## 3. Non-goals

- builder configuration;
- GitHub API;
- checksum byte computation;
- signatures;
- archive extraction;
- installer generation;
- arbitrary shell/source parsing;
- version ordering.

## 4. Current state

Predecessor implementation exists in `eggstack/eggup` at current reviewed baseline `cf5b3d3819c168eb2dbf841daa8332f3eb28c915`.

Relevant predecessor closure evidence:

- distribution M001 implementation `889a234cbe7f461d92def3df45c83c06a7d257e5`;
- distribution M002 corrective `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`;
- `eggup-dist` remains unpublished;
- runtime Eggup crates do not depend on it.

The corrected predecessor schema already covers simple direct, CodeGG-like bundle, and Egress-like archive fixtures.

Eggup also contains an implementation-ready M003 validator plan. Eggpack should adopt that intent after the migration closes instead of implementing it in Eggup first.

## 5. Target architecture

```text
distribution contract TOML
          |
          v
   eggpack-contract
      |        |
      |        +--> deterministic target expansion
      |
      +--> expected inventory
      |
      +--> pure conformance validators
               |
               v
       structured report
```

The crate stays usable independently from Eggpack's future build engine.

## 6. Dependency graph

```text
M001 faithful predecessor import
        |
        v
M002 conformance validators
        |
        v
M003 consumer fixture/CLI polish
```

M001 has only interface/evidence dependencies on Eggup and is ready.

M002 is hard-blocked on M001 closure.

## 7. Milestones

### M001 — Bootstrap workspace and faithfully import DistributionContract v1

Class: invariant / infrastructure

Objective: create the Rust workspace and `eggpack-contract` while reproducing every corrected predecessor invariant.

Dependencies: canonical planning/ADRs; Eggup source/closure baselines.

Deliverable boundary: parser/validation/expansion + fixtures only.

Value: Eggpack obtains a proven portable contract without redesign risk.

Exit conditions: stable + MSRV tests, package qualification, deterministic fixtures, exact semantic comparison to predecessor.

Deferred: removing `eggup-dist` from Eggup.

### M002 — Release/archive/mapping conformance validators

Class: capability / infrastructure

Objective: move the already-designed validator milestone to Eggpack.

Dependencies: M001 closed.

Deliverable boundary: pure expected/observed comparisons, reports, fixtures.

Value: CI/installers/runtime mappings can detect drift against one authority.

Exit conditions: all direct/bundle/archive positive/negative matrices pass; no network/process dependency.

### M003 — Conformance consumer polish

Class: polish

Objective: add thin local-file CLI/fixture interfaces only if real adoption shows they reduce consumer glue.

Dependencies: M002 + first consumer evidence.

## 8. Cross-cutting requirements

### Compatibility

M001 MUST preserve valid schema-v1 interpretation. If semantics need expansion, create a new schema/version/ADR rather than silently changing v1.

### Security

Bound all strings/counts. Keep traversal/collision rules. Forbid arbitrary execution.

### Performance

Parsing/expansion should be trivial relative to release build work; avoid heavy dependency graphs.

### Documentation

Public schema examples and migration provenance are required.

## 9. Verification strategy

- import predecessor unit/fixture tests;
- differential/golden expansion against predecessor fixtures;
- property/fuzz tests later for template/path parsers;
- stable + Rust 1.89;
- `cargo package`;
- dependency-tree proof of no build/network stack.

## 10. Risks and decision points

The main risk is "improving" the schema during migration and losing proof continuity. M001 is intentionally fidelity-first.

A future combined Eggpack producer configuration must reference this contract rather than create another artifact-name authority.

## 11. Completion definition

The subsystem is complete when contract v1 is Eggpack-owned, conformance is deterministic, at least two consumers depend on the same contract, and Eggup no longer owns a competing release-layout implementation.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 | ready | to be registered in initial handoff | — | — |
| M002 | blocked | not authored until M001 closure | — | M001 |
| M003 | planned | — | — | M002 + consumer evidence |
