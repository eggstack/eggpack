# Ecosystem Adoption Roadmap

Status: proposed

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

Simple adoption waits for contract/conformance + installer/CI pieces needed by the selected repo.

Complex adoption additionally waits for build/finalization and bundle/archive proof.

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

All milestones blocked on core producer capabilities.
