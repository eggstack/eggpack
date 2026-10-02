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

External repositories may consume already-closed producer interfaces without being inserted into this ordered qualification sequence. Eggwork Operations M003 is one such producer-only adoption: its release configuration remains Eggwork-owned and it does not advance or consume M001-M007 ecosystem milestone numbering.

## 6. Dependency graph

Core direct build/finalization/staging prerequisites are closed, including Build M005/M006 and CI M003e-M003g. Historically, M001 stopped at its §20 condition until ADR-0005 Option A allowed host-matched native qualification for cross-tool-built targets. It then resumed after consumer baseline re-review and successive exact tool re-pins, culminating in eggsact `v1.2.7` live run 36652731202 (producer pin M003g `e5c81f2`). Ecosystem M001 is closed. M003b's sole remaining exact rerun-reuse condition belongs to Phase 8 and consumer eggsact M005a Windows byte reproducibility; it does not reopen M001. Ecosystem M002 is now conditionally closed. StegoEggo implementation `3b96fae` landed the five-target Eggpack producer cutover without any new Eggpack production primitive, and consumer closure `c75132a` records green standard CI plus the release-drift guard. The pre-existing consumer updater nested-runtime panic was resolved by StegoEggo Release-Distribution M003 implementation `e611c91` and closure `f80eebe3`; its updater rehearsal, CI, and release-drift checks are green. Full M002 closure now waits only for the next ordinary stable B > 0.4.2 live release evidence.

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

M001 research and mirrored planning are complete. Implementation historically stopped at §20 before code, configuration, or workflow changes because Eggpack rejected `Qualification::Native` for a `CargoZigbuild` target; Build M006 under ADR-0005 Option A closed that gap. M001 then passed consumer baseline re-review, re-pinned through M003e/M003f/M003g, and completed on the maintainer-authorized eggsact `v1.2.7` release. The closure is recorded at `plans/closure/ecosystem-adoption/001-status.md`. The historical M006/M003e stop-and-restart sequence is retained as history, not current status.

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
| M001 eggsact direct release adoption | closed | `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md` | `plans/closure/ecosystem-adoption/001-status.md`; consumer closure `eggstack/eggsact: plans/closure/distribution-update-release/005-status.md` | Implemented and published on release `v1.2.7` (live run 36652731202, 15-asset draft, producer pin M003g `e5c81f2`). Open condition: consumer-side Windows byte-reproducibility (eggsact M005a) for byte-identical rerun reuse |
| M002 stegoeggo direct release adoption | conditionally closed | `plans/implementation/ecosystem-adoption/002-stegoeggo-direct-release-adoption-and-second-consumer-qualification.md` | `plans/closure/ecosystem-adoption/002-status.md`; consumer closure `eggstack/stegoeggo: plans/closure/release-distribution/002-status.md` | consumer cutover landed at `3b96fae`; updater corrective M003 closed at `f80eebe3`; no new Eggpack primitive required; full live closure waits only for ordinary stable B > 0.4.2 |
| M003 eggsearch target/qualification diversity | blocked | — | — | M001/M002 direct adoption evidence + eggsearch target/qualification review |
| M004 Gregg sibling bundle | blocked | — | — | prior adoption evidence + Gregg bundle/service review |
| M005 CodeGG runfile bundle | blocked | — | — | prior bundle evidence + CodeGG runfile review |
| M006 Egress archive pair | blocked | — | — | prior adoption evidence + Egress archive/Python boundary review |
| M007 specialized wheel/package evaluation | blocked | — | — | native adoption maturity; separate package-adapter planning |
