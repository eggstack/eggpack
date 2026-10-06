# Planning Hygiene M002 — Post-M003a Cross-Repository Handoff Reconciliation

Status: ready

Class: planning / closure hygiene / cross-repository handoff reconciliation

Roadmap: `plans/subsystems/planning-hygiene-roadmap.md`

Eggpack authoring baseline: `edd461fa887e0fe8e90ea23c285097a34efba551`

External reviewed baseline:

- `eggstack/eggsearch@33f508d87b9623b785bd151f46779f01b2409eb3`

Predecessor evidence:

- Contract M003 closure: `plans/closure/contract-conformance/003-status.md`
- Bootstrap M003 closure: `plans/closure/bootstrap-installers/003-status.md`
- Ecosystem M003a closure: `plans/closure/ecosystem-adoption/003a-status.md`

## 1. Objective

Reconcile Eggpack's active planning and agent-handoff surfaces after Contract M003, Bootstrap M003, and Ecosystem M003a all closed.

The post-closure state is materially different from the state that existed when those three plans were registered:

1. Contract M003 is closed on implementation `43fa2d7`.
2. Bootstrap M003 is closed with zero production delta and no M003a corrective.
3. Ecosystem M003a is closed with zero production delta and no Eggpack producer prerequisite.
4. Eggsearch M003b is therefore no longer blocked on architecture or producer work.
5. M003b's remaining gate is cross-repository planning discipline: an Eggpack M003b plan and a mirrored registered Eggsearch implementation plan must both exist before external implementation begins.

Some current control surfaces already reflect this, but others still contain stale pre-closure language or duplicated contradictory handoff text.

M002 makes those surfaces internally consistent. It does not implement the Eggsearch cutover and does not reopen any closed milestone.

## 2. Current drift to correct

### F1 — registry milestone index still marks M003b blocked on already-closed conditions

The active subsystem row and later handoff text correctly say M003b is ready for plan authoring, but the milestone index and planned/blocked table still retain the older blocker wording.

After paired plans are registered, every active registry occurrence must use one of two exact states:

- `ready` — when both paired plans are registered and no other hard dependency remains;
- `blocked` — only if a concrete new dependency is discovered during plan authoring.

Do not preserve "blocked until M003a closes"; M003a is already closed.

### F2 — registry summary still describes M003a as the remaining plan

A paragraph near the dependency-ready section still says one plan remains and M003b remains blocked until M003a closes. That chronology is stale.

Replace it with durable closure references and the paired-plan handoff.

### F3 — ecosystem roadmap retains a stale Contract M003 "research question"

The ecosystem roadmap still says duplicated contract parsing is tracked as Contract M003's bounded research question.

Contract M003 is now closed and `eggpack contract expand` exists. Update the statement to say the producer-side capability is closed and any cleanup is consumer-owned, separately planned.

### F4 — AGENTS.md current-handoff paragraph is duplicated and contradictory

The paragraph repeats the Bootstrap/Contract closure text and ends with the obsolete claim that Eggsearch M003b remains blocked until M003a closes.

Replace the paragraph as one compact status statement sourced from the registry.

### F5 — paired-plan registration must become the durable handoff

Once the Eggpack M003b and Eggsearch mirrored plans exist, Eggpack's control surface should point to both exact paths and exact external baseline rather than a generic "mirrored plan required" statement.

## 3. Invariants

- Do not rewrite historical closure records.
- Contract M003 remains closed.
- Bootstrap M003 remains closed.
- Ecosystem M003a remains closed.
- No production source, Cargo metadata, generated workflow, release, tag, registry package, or external repository implementation state changes in M002.
- Eggsearch work is attributed to Eggsearch.
- A paired plan is not implementation evidence.
- M003b may become `ready` only after both repositories register compatible implementation plans.
- No claim that glibc, ARMv7 runtime, or attestation parity has been re-proven during planning.
- No generalized provenance capability is implied.

## 4. In scope

- `plans/registry.md`
- `plans/subsystems/planning-hygiene-roadmap.md`
- `plans/subsystems/ecosystem-adoption-roadmap.md`
- `AGENTS.md`
- `plans/closure/planning-hygiene/002-status.md`
- consistency references to the paired Eggpack/Eggsearch M003b plans after they are registered

## 5. Out of scope

- Eggpack Rust changes
- Eggsearch production/config/workflow changes
- glibc re-proof
- ARMv7 validator implementation
- attestation API verification
- generated workflow changes
- release/tag/publication activity
- Phase 12 provenance design
- consumer-side Contract M003 cleanup outside the Eggsearch adoption plan

## 6. Ordered work packages

### WP1 — Freeze paired-plan baselines

Record the exact Eggpack and Eggsearch commits at which the paired M003b plans were registered.

### WP2 — Reconcile Eggpack registry

Remove every stale pre-M003a blocker statement and make the active row, milestone index, planned/blocked table, execution graph, next handoff, and downstream disposition agree.

### WP3 — Reconcile subsystem roadmap

Update stale Contract M003 language and make M003b's state/paired-plan pointers exact.

### WP4 — Reconcile AGENTS.md

Replace the duplicated current-handoff paragraph with one non-duplicated statement.

### WP5 — Stale-state sweep

Search at least for:

```text
M003b remains blocked until M003a closes
M003b.*blocked.*M003a
one plan remains.*M003a
Contract M003.*research question
research/decision ready
unblocked for research/plan
mirrored implementation plan.*required
```

A match is allowed only when clearly historical or when the current paired-plan state actually requires it.

### WP6 — Close M002

Write `plans/closure/planning-hygiene/002-status.md` with:

- exact paired-plan commits/paths;
- files reconciled;
- stale statements removed;
- empty production/package/workflow diff;
- remaining M003b implementation obligations;
- next handoff.

## 7. Verification

Required:

```bash
git diff --check
git status --short
git diff <implementation-parent>..HEAD -- crates/ Cargo.toml Cargo.lock .github/
```

The production/package/workflow diff MUST be empty.

Also verify:

- every current Eggpack M003b control-surface reference agrees on status;
- Contract M003 is described as closed wherever it is used as current evidence;
- Bootstrap M003 is described as closed with no remaining producer work;
- M003a is described as closed;
- both paired M003b plan paths resolve;
- Eggsearch remains unmodified by the reconciliation pass itself.

Hosted CI is not required solely for this docs-only pass.

## 8. Acceptance criteria

M002 closes when:

1. no active Eggpack surface says M003a is still open;
2. no active Eggpack surface says M003b is blocked on M003a;
3. no active surface describes Contract M003 as still a research question;
4. AGENTS.md contains exactly one current-handoff statement;
5. registry, ecosystem roadmap, and AGENTS.md agree on paired M003b readiness;
6. both paired plans are referenced by exact path;
7. no historical closure was rewritten;
8. no production/package/workflow/external implementation changed;
9. unresolved M003b obligations remain explicit: toolchain/glibc proof, ARMv7 runtime validator, attestation-subject parity, seven-target/no-clobber cutover evidence;
10. `plans/closure/planning-hygiene/002-status.md` records the final state.

## 9. Stop conditions

Stop and re-plan if:

- either paired plan cannot be registered without an unresolved architecture choice;
- Eggsearch release-relevant source/config/workflow materially changes from the reviewed baseline before registration;
- M003b plan authoring discovers a required Eggpack producer feature;
- the attestation-preservation design requires Eggpack-generated jobs to request OIDC;
- any reconciliation would require rewriting historical closure evidence.

## 10. Closure evidence

Record:

- implementation/reconciliation commit;
- paired Eggpack plan path + registration commit;
- paired Eggsearch plan path + registration commit;
- exact Eggsearch baseline;
- final M003b status;
- empty production/package/workflow diff;
- stale-state search result;
- unresolved findings;
- next handoff.
