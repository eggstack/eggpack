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

Core direct build/finalization/staging prerequisites are closed, CI M003d closed the static-workflow/runtime-identity plus product-wrapper/consumer-validator composition seam, and Build M005 closed deterministic Zig/cargo-zigbuild provisioning (`plans/closure/build-qualification/005-status.md`). Eggsact baseline review is complete at `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`. The mirrored eggsact M005 plan is registered; M001 then stopped at its §20 condition because Eggpack cannot express eggsact's two CargoZigbuild + native-qualified Linux targets. The gap and its options are recorded in `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md`, and the conditional corrective is proposed at `plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md`. The outstanding M003b live-draft evidence is therefore still blocked on M001.

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

Core build/finalization/bootstrap/staging prerequisites through M003d plus Build M005 are closed. M001 eggsact research and mirrored planning are complete, but implementation stopped at the plan's §20 condition before any code, configuration, or workflow change landed in either repository. The blocking capability gap is that Eggpack rejects `Qualification::Native` for a `CargoZigbuild` target, while eggsact must cross-build both Linux targets for a glibc 2.17 floor and natively qualify them on matching native runners. Decision pending in `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md`.

Reviewed eggsact baseline: `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`. The consumer baseline has since advanced to `34aed3ab36da2637c22412f7ca65d35f1ca5021d`; the five-target matrix, exact cross-tool versions and digests, and drafts-only assembly intent are unchanged, and the updater transport has since moved to the qualified `eggup-eggfetch` / `eggfetch-core` crates. Per planning process §2 the baseline is flagged for re-review before implementation rather than applied mechanically.

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
| M001 eggsact direct release adoption | blocked | `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md` | — | §20 stop condition hit before implementation: Eggpack cannot declare native qualification for a CargoZigbuild build. Pending `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md`; conditional corrective proposed as `plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md`. M001 itself performs the real M003b draft qualification once unblocked |
| M002 stegoeggo direct release adoption | blocked | — | — | M001 closure; use second-consumer evidence to avoid one-repo schema overfitting. Also inherits the same qualification gap, since stegoeggo is a direct-binary consumer |
| M003 eggsearch target/qualification diversity | blocked | — | — | M001/M002 direct adoption evidence + eggsearch target/qualification review |
| M004 Gregg sibling bundle | blocked | — | — | prior adoption evidence + Gregg bundle/service review |
| M005 CodeGG runfile bundle | blocked | — | — | prior bundle evidence + CodeGG runfile review |
| M006 Egress archive pair | blocked | — | — | prior adoption evidence + Egress archive/Python boundary review |
| M007 specialized wheel/package evaluation | blocked | — | — | native adoption maturity; separate package-adapter planning |
