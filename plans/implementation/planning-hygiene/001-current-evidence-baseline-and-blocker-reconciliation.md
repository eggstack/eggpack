# Planning Hygiene M001 — Current Evidence Baseline and Blocker Reconciliation

Status: ready

Class: planning / closure hygiene / cross-repository evidence reconciliation

Roadmap: `plans/subsystems/planning-hygiene-roadmap.md`

Eggpack authoring baseline: `cfb32de60fb678d6ad05c388ab178335f16360cf`

## 1. Objective

Bring Eggpack's active evidence baselines, blocker graph, and near-term roadmap dispositions onto the current reviewed state without changing production behavior.

The pass has five concrete outcomes:

1. replace stale snapshot-era "current evidence" prose with terminal/modern evidence;
2. revalidate and, if exact acceptance matches, discharge CI M003b's final rerun-reuse condition using the already-closed Eggsact M005a evidence;
3. bind Ecosystem M002's remaining operational condition to StegoEggo's concrete ready `0.4.3` ordinary-release milestone;
4. reclassify Contract M003 and Bootstrap M003 based on the consumer evidence that now exists, without authoring their feature implementations here;
5. leave Ecosystem M003+ and later long-term phases with exact sequencing/research dispositions rather than vague "future" text.

This is a docs/evidence pass. It MUST NOT modify Eggpack Rust, Cargo metadata, generated workflows, packages, releases, tags, crates.io state, or any external repository.

## 2. Why this is ready

All required evidence already exists.

### 2.1 Eggpack baseline

- Release Manifest M003a closed at `0b8e1f41f58a95eb9b0c4d05b930f6474109d27b`;
- observed hosted CI was recorded at `cfb32de60fb678d6ad05c388ab178335f16360cf`;
- the active registry already states that no Eggpack feature milestone is dependency-ready.

### 2.2 Eggsact M005a now discharges the recorded CI blocker

Eggpack CI M003b currently remains conditionally closed solely because exact byte-identical rerun reuse was blocked by Eggsact M005a Windows PE/PDB nondeterminism.

Eggsact has since closed that exact line:

- implementation: `eggstack/eggsact@f1352101dab748066c788e65e21e1bf303cfe995`;
- closure: `eggstack/eggsact@685fa3739562a4c3c670c28e05ee9e09b07cdbd2`;
- Windows independent double-build evidence: run `36880110434`;
- full Eggpack-generated pipeline rehearsal: run `36886042696`, attempts 1 and 2;
- attempt 2 reused the same draft and all 15 staged assets with `created: false, uploaded: 0, reused: 15`, with identical digests and zero digest-mismatch refusals;
- the Eggpack-generated workflow remained byte-identical under `eggpack ci check`; no Eggpack production change was required.

That evidence is materially the same event M003b named as its final condition. The implementation pass must compare it directly to the M003b plan/closure acceptance language before changing status. If it matches, M003b becomes closed and Phase 8's rerun-reuse exit condition is discharged. If any requirement is not actually met, retain conditional closure and record the exact residual gap.

### 2.3 StegoEggo M002 is tied to a concrete ready release

StegoEggo has registered the ordinary stable `0.4.3` release as the next release milestone:

- plan creation: `eggstack/stegoeggo@7b9c72c310d9aaa93d9e436394ef13af36e187de`;
- roadmap registration: `8f0503c671c4ad4d637be13b328641e6b3560021`;
- active-registry handoff: `a01a0222c6a5022a9eb3561f153b1e72b6e6c42a`;
- StegoEggo registry status: Release-Distribution M004 `0.4.3` is `ready`;
- that one ordinary stable release is explicitly intended to supply both the first public Eggpack-produced B evidence and the public `0.4.2 -> 0.4.3` updater proof.

Eggpack Ecosystem M002 therefore remains conditionally closed, but its blocker should point at this exact external milestone rather than an abstract future stable release.

## 3. Findings to reconcile

### F1 — stale Eggsact baseline prose

`plans/registry.md` still contains pre-closure text saying the mirrored Eggsact plan is `blocked / planned` and "flagged for re-review before implementation." Ecosystem M001 is already closed and published on `v1.2.7`.

Replace the current-baseline wording with terminal evidence and preserve the old baseline only as historical context if useful.

### F2 — stale Eggpack main/integration snapshot

`plans/registry.md` still describes `current main@404f63e` and M003h as future reconciliation. M003h is closed, M003a is closed, and current main is far beyond that snapshot.

Replace the snapshot with closure evidence, not a new ephemeral "current main" SHA that will immediately rot again.

### F3 — CI M003b / Phase 8 obsolete blocker

Current Eggpack surfaces still say exact rerun reuse is blocked on Eggsact M005a. Revalidate Eggsact closure `685fa373` and run `36886042696` against the exact Eggpack M003b condition.

If accepted:

- append a dated condition-discharge addendum to `plans/closure/ci-release-orchestration/003b-status.md`;
- mark M003b closed in the CI roadmap and registry;
- record Phase 8 exit as satisfied with the external evidence;
- remove the M005a blocker from active planning;
- do not claim Eggpack implemented the determinism fix.

### F4 — Ecosystem M002 operational blocker is now concrete

Keep M002 conditionally closed until the actual public `0.4.3` event occurs, but cite StegoEggo Release-Distribution M004 and its exact plan/registration commits as the owner of the remaining evidence.

Do not pre-close M002 based on a ready release plan.

### F5 — Contract M003 consumer evidence now exists

Contract M003's only gating evidence was real consumer adoption. That evidence exists.

Two independent direct-binary consumers now carry substantially parallel product-side contract-check scripts:

- Eggsact `scripts/check-release-contract.py` parses `release/eggpack/distribution.toml` with Python `tomllib`, reconstructs target -> expanded asset mapping, and compares it to product-owned frozen public names;
- StegoEggo performs the same parse/expand comparison independently.

Eggpack itself already owns `DistributionContract::parse_toml_str` / expansion semantics, while the public CLI currently exposes only the `eggpack ci ...` surface.

This is concrete evidence that Contract M003's question ("add thin local-file CLI/fixture interfaces only if real adoption shows they reduce consumer glue") is now worth a bounded research/decision pass. M001 MUST NOT implement that polish. It should update the roadmap from "planned pending consumer evidence" to "consumer evidence satisfied; research/decision needed" and identify the duplicated glue precisely.

Do not authorize a general CLI framework. Any future Contract M003 plan should stay narrowly focused on local bounded contract parse/expand/inspection output that can replace duplicated producer-fact parsing while leaving product-specific invariants in consumer scripts.

### F6 — Bootstrap M003 remains legitimately blocked, but the blocker is narrower

Both Eggsact and StegoEggo have adopted Eggpack-generated exact installers in their producer cutovers, but Bootstrap M003's completion definition is real two-consumer adoption evidence. StegoEggo's second-consumer closure remains operationally conditional until the public `0.4.3` release.

Therefore Bootstrap M003 should remain blocked, but its blocker becomes:

`StegoEggo Release-Distribution M004 public 0.4.3 live receipt -> Eggpack Ecosystem M002 full closure -> Bootstrap M003 candidate review`.

Do not author Bootstrap M003 implementation in this pass.

### F7 — Ecosystem M003 Eggsearch is the next ordered adoption, but not yet executable

Eggsearch is a meaningful next diversity consumer, not another direct five-target clone:

- seven release targets;
- Linux ARMv7 with QEMU runtime qualification;
- Windows ARM64;
- Linux x86-64/AArch64 with glibc 2.17 floors;
- current release workflow uses Zig `0.13.0` and cargo-zigbuild `0.20.1`;
- current draft assembly uses `gh release upload --clobber`;
- its installers and release target matrix remain hand-maintained;
- immutable `v0.4.1` release/provenance evidence exists.

Eggpack already has Armv7 target classification and an explicit `Qualification::Emulated` / QEMU-sysroot path, so M003 is a credible next research line once Ecosystem M002 fully closes. But adoption must first reconcile Eggsearch's older toolchain versions, Windows ARM64 runner/host mapping, QEMU qualification semantics, and the current clobber-on-draft-rerun behavior with Eggpack's fail-closed staging policy.

Record M003 as "blocked on StegoEggo M002 live closure; preflight research warranted" rather than authoring an implementation plan here.

## 4. Future-line dispositions

The implementation pass should make the following sequencing explicit.

### Near-term

1. Planning Hygiene M001 — execute now.
2. StegoEggo `0.4.3` live release — external, already ready in StegoEggo.
3. Ecosystem M002 — full-close from that live evidence if acceptance passes.
4. Bootstrap M003 — re-review immediately after M002 full closure.
5. Ecosystem M003 Eggsearch — research/plan after M002 full closure; preflight research may begin before the event but implementation remains blocked.
6. Contract M003 — independent research/decision can begin now because first-consumer evidence exists.

### Later ordered adoption

- Ecosystem M004 Gregg sibling bundle remains after Eggsearch target-diversity evidence;
- Ecosystem M005 CodeGG runfile bundle remains after a qualified sibling-bundle consumer;
- Ecosystem M006 Egress archive pair remains after prior native adoption evidence and requires an explicit archive/Python boundary review;
- Ecosystem M007 specialized wheel/package evaluation remains blocked on native-adoption maturity.

### Long-term phases

- Phase 12 provenance/authenticity is researchable but not implementation-ready: it requires a dedicated trust ADR. Eggsearch's existing GitHub Artifact Attestation release provides useful prior art, but it does not select Eggpack's trust model.
- Phase 13 specialized package adapters should remain behind broader native adoption; registry publication remains explicit.
- Phase 14 public API/format stabilization remains premature until the intended contracts have multiple independent consumers and the broader adoption line has exercised the remaining release forms.

## 5. Invariants

- no production source change;
- no Cargo or lockfile change;
- no generated workflow change;
- no package/release/tag/registry mutation;
- no external repository mutation;
- preserve historical closure chronology;
- attribute Eggsact and StegoEggo evidence to those repositories;
- do not mark StegoEggo live evidence complete before the release happens;
- do not mark Contract M003 or Ecosystem M003 implementation-ready without their own researched implementation plans;
- do not weaken Eggpack's no-clobber policy to mirror Eggsearch's current workflow.

## 6. In scope

- `plans/registry.md`;
- `plans/subsystems/planning-hygiene-roadmap.md`;
- `plans/subsystems/ci-release-orchestration-roadmap.md`;
- `plans/subsystems/ecosystem-adoption-roadmap.md`;
- `plans/subsystems/contract-conformance-roadmap.md`;
- `plans/subsystems/bootstrap-installers-roadmap.md`;
- append-only update to `plans/closure/ci-release-orchestration/003b-status.md` if Eggsact M005a fully satisfies the condition;
- `AGENTS.md` only if its "next handoff" summary becomes false after the reconciliation;
- final closure `plans/closure/planning-hygiene/001-status.md`.

## 7. Out of scope

- Rust/code changes;
- changes to consumer repositories;
- executing StegoEggo `0.4.3`;
- writing Contract M003 feature implementation;
- writing Bootstrap M003 feature implementation;
- writing Ecosystem M003 Eggsearch implementation;
- adopting attestations/signing;
- package publication.

## 8. Ordered work packages

### WP1 — Revalidate exact evidence

Read-only verify:

- Eggpack M003b source plan and historical closure condition;
- Eggsact M005a closure `685fa373`, implementation `f135210`, runs `36880110434` and `36886042696`;
- StegoEggo Release-Distribution M004 ready registration `7b9c72c` / `8f0503c` / `a01a022`;
- Eggsact and StegoEggo contract-check scripts as current examples of duplicated local parse/expand glue;
- Eggsearch current seven-target release workflow and installer mapping.

### WP2 — Refresh current evidence baselines

Remove snapshot wording that presents closed work as future work. Prefer durable closure references over continuously changing "current main" SHAs.

### WP3 — Discharge or retain CI M003b condition

Compare the Eggsact rerun rehearsal directly to M003b acceptance.

If fully matched, close the condition via append-only historical addendum and active-state updates. If not, document the smallest remaining gap.

### WP4 — Reconcile blocked/future subsystem statuses

Apply F4-F7 without pre-authorizing feature work.

### WP5 — Consistency sweep

Search active planning for:

- Eggsact M005a still described as open;
- CI M003b/Phase 8 still described as blocked on Windows nondeterminism after condition discharge;
- Ecosystem M002 described without the concrete StegoEggo M004 owner;
- Contract M003 still described as lacking first-consumer evidence;
- Bootstrap M003 presented as generally blocked instead of specifically waiting on the live second-consumer receipt;
- stale `current main@404f63e` or `blocked / planned` Eggsact snapshot text.

### WP6 — Close Planning Hygiene M001

Create `plans/closure/planning-hygiene/001-status.md` with the evidence matrix, changed files, status transitions, and exact remaining blockers.

## 9. Verification

Required scope proof:

```text
git diff --check
git status --short
git diff <implementation-parent>..HEAD -- crates/ Cargo.toml Cargo.lock .github/
```

The production/package/workflow diff MUST be empty.

The closure must also record targeted text-search results for each stale blocker named in WP5.

Hosted CI is optional/incidental for this docs-only pass. If it runs, record the observed result without treating it as runtime qualification evidence.

## 10. Acceptance criteria

M001 closes only when:

1. current evidence baselines no longer present pre-M001/pre-M003h snapshots as current;
2. CI M003b's external condition is either formally discharged from exact Eggsact evidence or retained with a specific evidence gap;
3. Phase 8 status agrees with M003b;
4. Ecosystem M002 points at StegoEggo's ready `0.4.3` milestone but remains conditional until the public event;
5. Contract M003 records consumer evidence as satisfied and a bounded research/decision disposition;
6. Bootstrap M003 retains only the live second-consumer receipt blocker;
7. Ecosystem M003 records Eggsearch as the next ordered adoption with implementation still blocked on M002 full closure;
8. later adoption/provenance/package-adapter phases remain sequenced and unauthorised;
9. no production/package/workflow/external-repository mutation occurred;
10. `plans/closure/planning-hygiene/001-status.md` records exact evidence and remaining work.

## 11. Stop conditions

Stop and re-plan if:

- Eggsact M005a rehearsal does not actually satisfy M003b's exact rerun requirement;
- StegoEggo `0.4.3` has already published and changes Ecosystem M002 from pending to closable during execution;
- consumer evidence reveals a medium-or-higher Eggpack runtime defect;
- the reconciliation requires changing code, workflow generation, package state, or external repositories.

## 12. Closure evidence

Record:

- final Eggpack reconciliation commit;
- Eggsact M005a implementation/closure + run IDs;
- StegoEggo M004 registration commits;
- whether M003b/Phase 8 closed and why;
- exact Contract M003 and Bootstrap M003 dispositions;
- Eggsearch preflight facts;
- empty production/package/workflow diff;
- unresolved findings by severity;
- next handoff after cleanup.
