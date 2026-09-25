# Ecosystem Adoption Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#23-consumer-adoption-requirement`
- `plans/002-long-term-roadmap.md#phase-9--simple-consumer-adoption`
- `plans/002-long-term-roadmap.md#phase-11--broader-native-adoption`

Related ADRs:

- ADR-0001;
- ADR-0003.

## 1. Purpose and ownership boundary

Prove Eggpack abstractions against real repositories and retire duplicated producer-side release authority.

Each consumer retains product-specific smoke semantics, release source policy, install destinations, fallback policy, and package-registry policy.

## 2. Work classification

### Invariants

- migration cannot weaken current release coverage/qualification;
- existing installer/update path remains available until replacement qualifies;
- product policy is not generalized without multi-consumer evidence;
- every adoption records before/after authority.

### Capabilities

- shared contract;
- shared manifest;
- generated installers/CI;
- optional Eggup interoperability.

### Infrastructure

- consumer fixtures;
- compatibility tests;
- migration guides.

## 3. Non-goals

- simultaneous flag-day migration;
- forcing every consumer into identical packaging;
- deleting specialized Python/package work prematurely.

## 4. Current state

Evidence reviewed:

- eggsact: direct binaries, five-target release workflow, shell/PowerShell installers;
- stegoeggo: similar direct-binary pattern;
- eggsearch: expands to ARMv7 and Windows ARM64 with qualification/release modes;
- Gregg: sibling `gregg` + `greggd` assets and duplicated target table;
- Egress: `eggress` + `pproxy` archive pair plus Python wheels;
- Eggserve: sophisticated declarative wheel matrix and many release validators.

## 5. Target adoption sequence

1. eggsact;
2. stegoeggo;
3. eggsearch;
4. Gregg;
5. CodeGG;
6. Egress;
7. Eggserve/Python packaging only after native Eggpack contracts are stable.

## 6. Dependency graph

Core simple-direct producer prerequisites are closed, but the first consumer's full release-workflow parity still depends on Phase 8 draft staging. Eggsact baseline review is complete at `eggstack/eggsact@43971e7c1af7f936acfd876f9bff246e72866f2d`; implementation waits for CI M003b closure and a mirrored eggsact distribution/release plan.

Complex adoption additionally depends on the release form/target diversity required by each consumer; bundle/archive producer paths are now qualified, but later milestones still require per-consumer evidence before planning.

Eggup interoperability is optional per consumer and has its own gate.

## 7. Milestones

M001 eggsact direct release adoption.

M002 stegoeggo direct release adoption.

M003 eggsearch target/qualification diversity.

M004 Gregg sibling bundle.

M005 CodeGG runfile bundle.

M006 Egress archive pair.

M007 specialized wheel/package evaluation.

## 8. Cross-cutting requirements

Each adoption closure records:

- removed duplicate files/tables;
- retained product-specific policy;
- artifact-name parity;
- target coverage;
- qualification parity;
- generated workflow/installer diffs;
- rollback path.

## 9. Verification strategy

Compare old/new expected artifact sets and run real local/hosted qualification before deleting predecessor machinery.

## 10. Risks and decision points

Premature adoption can force schema design around one repo. Simple consumers must prove shared behavior before complex extensions.

## 11. Completion definition

Multiple independent repos use Eggpack as producer authority with measurable reduction in release duplication and no safety/coverage regression.

## 12. Milestone status

Core build/finalization/bootstrap/generated-CI prerequisites are closed. M001 eggsact research is complete enough to identify the migration boundary, but implementation remains blocked on the Phase 8 staging interface and a mirrored eggsact handoff.

Reviewed eggsact baseline: `eggstack/eggsact@43971e7c1af7f936acfd876f9bff246e72866f2d`.

Current eggsact release authority still duplicated across:

- five-target `.github/workflows/release-binaries.yml`;
- `packaging/install.sh` / `packaging/install.ps1`;
- `scripts/check-release-contract.py`;
- `src/update.rs` target/asset mapping.

Adoption must preserve product-owned behavior that Eggpack intentionally does not own:

- crates.io-first publication and tag-after-publish ordering;
- installer `--version` / latest selection;
- Cargo fallback on unsupported host or exact asset 404;
- self-update release selection/fallback and Eggup transaction semantics.

The first migration should therefore replace producer target/artifact/checksum/qualification/workflow authority while composing around, not absorbing, those product policies.

| Milestone | Status | Implementation plan | Closure record | Blockers / sequencing |
|---|---|---|---|---|
| M001 eggsact direct release adoption | blocked / research complete | — | — | CI M003b full staging closure + mirrored eggsact distribution/release adoption plan; preserve Cargo fallback/latest/update product policy |
| M002 stegoeggo direct release adoption | blocked | — | — | M001 closure; use second-consumer evidence to avoid one-repo schema overfitting |
| M003 eggsearch target/qualification diversity | blocked | — | — | M001/M002 direct adoption evidence + eggsearch target/qualification review |
| M004 Gregg sibling bundle | blocked | — | — | prior adoption evidence + Gregg bundle/service review |
| M005 CodeGG runfile bundle | blocked | — | — | prior bundle evidence + CodeGG runfile review |
| M006 Egress archive pair | blocked | — | — | prior adoption evidence + Egress archive/Python boundary review |
| M007 specialized wheel/package evaluation | blocked | — | — | native adoption maturity; separate package-adapter planning |
