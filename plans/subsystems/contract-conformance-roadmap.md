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

Predecessor implementation exists in `eggstack/eggup`. The terminal producer-side predecessor is distribution M003: implementation `9941c58d7039410c728860f9e4e382881d4ccf54`, closure `4169c8021b447fe73c8ee3ea71a80a535c940f54`. Current Eggup planning assigns all future producer distribution work to Eggpack.

Relevant predecessor closure evidence:

- distribution M001 implementation `889a234cbe7f461d92def3df45c83c06a7d257e5`;
- distribution M002 corrective `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`;
- distribution M003 implementation `9941c58d7039410c728860f9e4e382881d4ccf54`;
- distribution M003 closure `4169c8021b447fe73c8ee3ea71a80a535c940f54`;
- `eggup-dist` was unpublished/frozen predecessor evidence and has now been removed from Eggup;
- runtime Eggup crates do not depend on Eggpack producer crates;
- Eggup distribution M004 closed at implementation `bc25885bd41b86bfdf2f32d1e42856e00829cd7a` after this subsystem's M002 closure.

The corrected predecessor schema covers simple direct, CodeGG-like bundle, and Egress-like archive fixtures. Closed M003 additionally provides the expected-release-file model, release/archive inventories, observed mappings, deterministic findings/report surface, bounds, and positive/negative conformance fixtures.

M002 has ported the closed API and intended conformance behavior without changing schema v1. Review also found and fixed a predecessor comparison blind spot for crossed bundle entry mappings; see `plans/closure/contract-conformance/002-status.md` for the matrix. Eggup subsequently executed and closed M004, removing `eggup-dist`; Eggpack is now the sole active producer-side contract/conformance authority.

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

M001 is closed; its schema/expansion import is qualified and retained as the first migration stage.

M002 is closed with the terminal Eggup M003 predecessor behavior qualified. M003's consumer-evidence gate was satisfied on 2026-10-04 (Planning Hygiene M001) by two independent adopted consumers. The bounded decision is now resolved into a ready implementation plan: expose existing direct-contract expansion semantics through one narrow local CLI projection without adding a second schema or a general CLI framework.

## 7. Milestones

### M001 — Bootstrap workspace and faithfully import DistributionContract v1

Class: invariant / infrastructure

Objective: create the Rust workspace and `eggpack-contract` while reproducing every corrected predecessor invariant.

Dependencies: canonical planning/ADRs; Eggup source/closure baselines.

Deliverable boundary: parser/validation/expansion + fixtures only.

Value: Eggpack obtains a proven portable contract without redesign risk.

Exit conditions: stable + MSRV tests, package qualification, deterministic fixtures, exact semantic comparison to predecessor.

Deferred from M001 only: removing `eggup-dist` is handled by the already-registered Eggup distribution M004 after this subsystem's M002 closes.

### M002 — Release/archive/mapping conformance validators

Class: capability / infrastructure

Objective: port the closed Eggup M003 validator implementation and independently qualify it as Eggpack-owned behavior.

Dependencies: M001 closed; Eggup M003 implementation/closure available as immutable predecessor evidence.

Deliverable boundary: pure expected/observed comparisons, reports, fixtures.

Value: CI/installers/runtime mappings can detect drift against one authority.

Exit conditions: all direct/bundle/archive positive/negative matrices pass; predecessor public behaviors/fixtures are accounted for; no network/process dependency; Eggup M004 retirement is unblocked.

### M003 — Conformance consumer polish

Class: polish

Objective: add thin local-file CLI/fixture interfaces only if real adoption shows they reduce consumer glue.

Dependencies: M002 + first consumer evidence. **Satisfied and delivered.** M003 closed on `plans/closure/contract-conformance/003-status.md`.

Evidence gathered 2026-10-04 (Planning Hygiene M001): two independent direct-binary consumers each carry a product-side contract-check script that loads the checked-in `release/eggpack/distribution.toml` with Python `tomllib` and re-implements the producer's `{product}`/`{target}` asset expansion in order to compare it against a frozen product-owned public-name table:

- `eggstack/eggsact: scripts/check-release-contract.py`;
- `eggstack/stegoeggo: scripts/check-release-contract.py`, plus a second independent expansion in `scripts/release-check-assets.sh`.

That is three near-parallel implementations of the same bounded producer-fact parse/expand step across two repositories. Eggpack already owns the semantics in `DistributionContract::parse_toml_str` and its expansion, but the public CLI exposes only the `eggpack ci ...` surface, so consumers cannot reuse them.

Bounded disposition: M003 is **closed**. The delivered surface is exactly the narrow one registered: `eggpack contract expand` reads one bounded local schema-v1 contract through the shared non-symlink reader, expands one target/alias through the existing `DistributionContract::parse_toml_str` and `::expand` APIs, and emits exactly one scalar (`canonical-target`, or direct-artifact `asset`, `sidecar`, or `install`) plus one newline. Bundle/archive list projection, repository/network discovery, release selection, a new serialized expansion schema, and a general CLI framework stayed out of scope, and each omission is now recorded with its reason in `architecture/cli.md`. Product-specific invariants stay in consumer scripts.

Delivery notes worth carrying forward:

- the command uses a **command-local exact argument parser** rather than the shared `get_flag` helper, because that helper silently keeps the first occurrence of a repeated flag — unacceptable for output a consumer script captures. The historical `ci` family was left byte-for-byte unchanged;
- `canonical-target` resolves for every asset form while `asset`/`sidecar`/`install` fail closed for bundle and archive targets. The first implementation had this inverted and the test lane caught it (closure finding C-1);
- consumer-shaped parity was proven by running the binary against the real `eggsact@d4e6e5c` and `stegoeggo@v0.5.0` contracts for every target, every alias, and all four fields — 80 queries, matching both consumers' frozen public names. A byte-identical copy of the adopted eggsact contract is checked in as a regression fixture;
- the documented CLI README example is executed verbatim by a guard test, so it cannot repeat the earlier unguarded-example drift;
- one optional consumer-side consequence: each consumer can now delete its hand-written asset-name reconstruction, under a plan in that consumer's own repository. That work is not authorized here.

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

The main risk is "improving" the schema or conformance semantics during migration and losing proof continuity. M001 and M002 are intentionally fidelity-first. Any deliberate semantic change belongs after migration closure and requires versioning/ADR treatment.

A future combined Eggpack producer configuration must reference this contract rather than create another artifact-name authority.

## 11. Completion definition

The authority-transfer portion is complete: contract v1/conformance are Eggpack-owned and Eggup M004 removed the competing producer crate. The first-consumer evidence condition for M003 was satisfied, and M003 is now closed (`plans/closure/contract-conformance/003-status.md`) on implementation `43fa2d7`: the bounded local scalar projection exists, is tested at the process boundary, and is proven against both adopted consumers' real contracts. Both parts of the completion definition are met, so the contract-conformance subsystem is **complete** and no further contract milestone is registered. Remaining contract-shaped duplication is consumer-side cleanup, owned by those repositories.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 | closed | `plans/implementation/contract-conformance/001-workspace-and-distribution-contract-v1-import.md` | `plans/closure/contract-conformance/001-status.md` | — |
| M002 | closed | `plans/implementation/contract-conformance/002-release-and-installer-conformance-validators.md` | `plans/closure/contract-conformance/002-status.md` | — |
| M003 | closed | `plans/implementation/contract-conformance/003-bounded-direct-contract-expansion-cli.md` | `plans/closure/contract-conformance/003-status.md` | Closed on `43fa2d7`; bounded local scalar projection over existing schema-v1 semantics; 18 new tests including a process-boundary stdout contract; parity proven against eggsact `d4e6e5c` and stegoeggo `v0.5.0` for every target/alias/field; no new serialized schema, network or repository discovery, bundle/archive list projection, or CLI framework. Optional consumer-side asset-name cleanup becomes eligible, under plans in those repositories |
