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

Core direct build/finalization/staging prerequisites are closed, including Build M005/M006 and CI M003e-M003g. Historically, M001 stopped at its §20 condition until ADR-0005 Option A allowed host-matched native qualification for cross-tool-built targets. It then resumed after consumer baseline re-review and successive exact tool re-pins, culminating in eggsact `v1.2.7` live run 36652731202 (producer pin M003g `e5c81f2`). Ecosystem M001 is closed. CI M003b's exact rerun-reuse condition belonged to Phase 8 and to consumer eggsact M005a; M005a closed it product-side, M003b and Phase 8 are now closed (Planning Hygiene M001, 2026-10-04), and M001 was not reopened. **Ecosystem M002 is now closed.** StegoEggo implementation `3b96fae` landed the five-target Eggpack producer cutover without any new Eggpack production primitive, and consumer closure `c75132a` records green standard CI plus the release-drift guard. The pre-existing consumer updater nested-runtime panic was resolved by StegoEggo Release-Distribution M003 implementation `e611c91` and closure `f80eebe3`. The remaining live condition was then satisfied by StegoEggo's ordinary stable `v0.5.0` (Eggpack run `37181914252`, published 2026-10-04T06:34:19Z, source `57ca94c9…`, exact 15-asset inventory, manual publication, real public `0.4.2 -> 0.5.0` updater transition). M002's live evidence is complete and Ecosystem M003 is no longer blocked on it.

Complex adoption additionally depends on the release form/target diversity required by each consumer; bundle/archive producer paths are now qualified, but later milestones still require per-consumer evidence before planning.

Eggsearch is split into M003a/M003b because current research found real compatibility choices that must not be hidden inside a cross-repository implementation pass. **M003a has now resolved all four of those choices with evidence** (`plans/closure/ecosystem-adoption/003a-status.md`), and the answer turned out to be better for M003b than the split anticipated: **no new Eggpack producer capability is required.**

What the four gaps actually were, now measured rather than assumed:

- the Zig 0.13.0 archive-layout gap is real — Eggpack's provisioner emits `zig-<arch>-linux-<version>.tar.xz`, which 404s for 0.13.0 and 200s for 0.14.1 — but the pin appears in exactly one Eggsearch file and in no public document, while the guaranteed artefact is the glibc 2.17 floor asserted post-build by `readelf`. Migrating to the already-qualified pair is therefore implementation detail, not a compatibility change, and M003b must re-prove the floor against the real build graph before cutover;
- the ARMv7 gap is a provisioning gap, not a schema gap: `Structural` core qualification plus a required product-owned consumer validator is expressible today and still gates aggregation, while `Emulated` renders `--qemu-sysroot` while provisioning nothing;
- the attestation gap is a permission-placement gap: a narrow read-only product-owned workflow can attest exact already-staged bytes, so Eggpack's no-OIDC generated workflow is preserved untouched;
- the clobber gap is not a gap: Eggpack's exact-reuse/refusal staging is strictly stronger, and operator recovery is a three-step procedure.

M003b is now blocked on one *process* step only — a mirrored implementation plan registered in `eggstack/eggsearch` — and on the M003b-side re-proof obligations recorded in that closure.

Eggup interoperability is optional per consumer and has its own gate.

## 7. Milestones

M001 eggsact direct release adoption.

M002 stegoeggo direct release adoption.

M003a eggsearch seven-target compatibility preflight and migration design — **closed**.

Implementation plan: `plans/implementation/ecosystem-adoption/003a-eggsearch-seven-target-compatibility-preflight-and-migration-design.md`.

Closure record: `plans/closure/ecosystem-adoption/003a-status.md`.

M003b eggsearch target/qualification diversity adoption — **ready for plan authoring**. All compatibility choices are resolved and no producer prerequisite milestone was required. The single remaining condition is the mirrored implementation plan registered in `eggstack/eggsearch` before any external edit, per planning process §9.

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

Historical pre-adoption eggsact baseline, retained for traceability only: `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25` (later `34aed3ab36da2637c22412f7ca65d35f1ca5021d`). The "flagged for re-review before implementation" note applied to the pre-cutover baseline and was discharged when M001 passed consumer baseline re-review and landed. It is not a current blocker.

Current eggsact release authority is no longer duplicated. Since M001, producer target/artifact/checksum/qualification/workflow authority comes from checked-in `release/eggpack/` configuration and the generated workflow, drift-guarded by `eggpack ci check` at the pinned tool revision. `scripts/check-release-contract.py` and `src/update.rs` now *compare against* the Eggpack contract rather than restating producer facts.

What genuinely remains duplicated across both adopted consumers is a specific, bounded piece of producer-fact parsing: each repository independently loads `release/eggpack/distribution.toml` with Python `tomllib` and re-implements the `{product}`/`{target}` asset expansion to compare it against a frozen product-owned name table. That residue is tracked as Contract M003's bounded research question, not as a failed adoption.

Product-owned behavior that Eggpack intentionally does not own, and which both consumers preserved:

- crates.io-first publication and tag-after-publish ordering;
- installer `--version` / latest selection;
- Cargo fallback on unsupported host or exact asset 404;
- self-update release selection/fallback and Eggup transaction semantics.

The migration replaced producer authority while composing around, not absorbing, those product policies.

| Milestone | Status | Implementation plan | Closure record | Blockers / sequencing |
|---|---|---|---|---|
| M001 eggsact direct release adoption | closed | `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md` | `plans/closure/ecosystem-adoption/001-status.md`; consumer closure `eggstack/eggsact: plans/closure/distribution-update-release/005-status.md` | Implemented and published on release `v1.2.7` (live run 36652731202, 15-asset draft, producer pin M003g `e5c81f2`). The consumer-side Windows byte-reproducibility condition is closed by eggsact M005a (`685fa373`) |
| M002 stegoeggo direct release adoption | closed | `plans/implementation/ecosystem-adoption/002-stegoeggo-direct-release-adoption-and-second-consumer-qualification.md` | `plans/closure/ecosystem-adoption/002-status.md` §9; consumer closure `eggstack/stegoeggo: plans/closure/release-distribution/005-status.md` (`7bde933b`) | consumer cutover `3b96fae`; updater corrective M003 closed `f80eebe3`; no new Eggpack primitive required. Live evidence landed as `v0.5.0` (run `37181914252`, 15 assets, manual publication, real `0.4.2 -> 0.5.0` updater transition). The rerun-reuse item was not exercised on this release and is substituted by eggsact M005a evidence — see closure §9.3 |
| M003a eggsearch compatibility preflight + migration design | closed | `plans/implementation/ecosystem-adoption/003a-eggsearch-seven-target-compatibility-preflight-and-migration-design.md` | `plans/closure/ecosystem-adoption/003a-status.md` | Read-only/scratch preflight against `eggstack/eggsearch@ec437cb`, zero production delta. All seven targets rendered (75723-byte workflow, `ci check: match`, byte-identical re-render); Windows ARM64 proven on `windows-11-arm`; G1 selected Option A (toolchain pin is implementation detail, glibc floor is policy); G2 selected Option A (Structural + required consumer validator); G3 selected Option A design-only (read-only provenance seam, no OIDC in generated jobs); G4 accepted (no-clobber strictly stronger). **No producer prerequisite plan registered** |
| M003b eggsearch target/qualification diversity adoption | ready for plan authoring | — | — | All four compatibility choices resolved in M003a with no producer prerequisite. Remaining: a mirrored implementation plan registered in `eggstack/eggsearch` before any external edit, the post-bump glibc 2.17 re-proof for both floored Linux targets and ARMv7, and an attestation-subject parity check with a token that can read attestations. If the re-proof fails, M003b stops and returns to M003a |
| M004 Gregg sibling bundle | blocked | — | — | remains after M003b Eggsearch target-diversity adoption evidence; prior adoption evidence + Gregg bundle/service review |
| M005 CodeGG runfile bundle | blocked | — | — | remains after a qualified sibling-bundle consumer; prior bundle evidence + CodeGG runfile review |
| M006 Egress archive pair | blocked | — | — | remains after prior native adoption evidence; requires an explicit Egress archive/Python boundary review |
| M007 specialized wheel/package evaluation | blocked | — | — | remains behind broader native adoption; separate package-adapter planning |
