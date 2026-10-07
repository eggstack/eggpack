# Planning Hygiene Milestone 002 — Post-M003a Cross-Repository Handoff Reconciliation

Status: **closed**

> **Subsequent state (2026-10-07).** This record's body describes Ecosystem M003b as *conditionally closed* with publication outstanding. That was true when M002 closed and is left unrewritten, because history is not revised. M003b has since been **closed**: provenance run `37561960304` attested the staged bytes and eggsearch `v0.4.2` was published, discharging the condition M002 recorded. **Publication is no longer outstanding.** For current state read `plans/closure/ecosystem-adoption/003b-status.md` §16, not the M003b statements below.

Source plan: `plans/implementation/planning-hygiene/002-post-m003a-cross-repository-handoff-reconciliation.md`

Roadmap: `plans/subsystems/planning-hygiene-roadmap.md`

Predecessor evidence (unchanged, read-only):

- `plans/closure/contract-conformance/003-status.md`
- `plans/closure/bootstrap-installers/003-status.md`
- `plans/closure/ecosystem-adoption/003a-status.md`
- `plans/closure/planning-hygiene/001-status.md`

## 1. Paired-plan baselines (WP1)

| Role | Repository | Commit |
|---|---|---|
| Eggpack M003b plan registration | `eggstack/eggpack` | `325d44e` (`plans/implementation/ecosystem-adoption/003b-eggsearch-seven-target-eggpack-producer-cutover.md`) |
| Eggpack M002 authoring baseline | `eggstack/eggpack` | `edd461fa887e0fe8e90ea23c285097a34efba551` |
| Eggsearch M003b paired plan registration | `eggstack/eggsearch` | `377f9e8` (`plans/implementation/eggpack-release-adoption/001-seven-target-eggpack-producer-cutover.md`) |
| Eggsearch registry row for that plan | `eggstack/eggsearch` | `6767062` |
| External reviewed baseline at M002 authoring | `eggstack/eggsearch` | `33f508d87b9623b785bd151f46779f01b2409eb3` |

M002 was authored against the state where both paired plans were registered but M003b had not yet been implemented. It is being closed after M003b's implementation, so its reconciliation reflects the **post-implementation** truth rather than a snapshot that would immediately rot. Where the plan's §8 acceptance criterion 9 requires unresolved M003b obligations to remain explicit, those obligations are recorded as M003b's named remaining condition (publication) rather than as open implementation work.

## 2. Files reconciled

| File | Change |
|---|---|
| `AGENTS.md` | Current-handoff paragraph replaced (F4) |
| `plans/registry.md` | Six control-surface rows plus the execution graph and two handoff paragraphs (F1, F2, F5) |
| `plans/subsystems/ecosystem-adoption-roadmap.md` | Contract M003 "research question" retired (F3); M003b state, milestone prose, and status row made exact (F3, F5) |
| `plans/subsystems/planning-hygiene-roadmap.md` | M002 status, resolved-state list, status row, and completion definition |
| `plans/closure/planning-hygiene/002-status.md` | This record (WP6) |

## 3. Stale statements removed

### F1 — registry milestone index marked M003b blocked on already-closed conditions

The active subsystem row, the dependency-ready paragraph, the milestone index row, the planned/blocked row, the immediate execution graph, the downstream disposition table, the next-handoff paragraph, and `AGENTS.md` all now state one thing: M003b is **conditionally closed** with its closure record, its paired-plan paths, its implementation commit, its hosted run, and its named remaining condition.

Removed wording included:

- `M003b ready` / `M003b adoption [READY FOR PLAN AUTHORING; mirrored Eggsearch plan required]`
- `M003b is ready for plan authoring but requires … a mirrored implementation plan registered in eggstack/eggsearch`
- `What M003b may not do yet is edit eggstack/eggsearch: the required mirrored implementation plan must be registered in that repository first`
- `**No Eggpack plan is now ready.** All three registered lines are closed.`

The last of these was also factually wrong in a second way: after M003b there are four registered lines, not three.

### F2 — registry summary described M003a as the remaining plan

The paragraph near the dependency-ready section claimed M003a "leaves Eggsearch M003b ready for plan authoring once a mirrored Eggsearch plan is registered". M003a is closed, the mirrored plan is registered, and the implementation has since landed. Replaced with closure references and the paired-plan handoff.

### F3 — ecosystem roadmap retained a stale Contract M003 "research question"

The roadmap tracked duplicated `{product}`/`{target}` expansion across consumers as "Contract M003's bounded research question". Contract M003 is closed at `plans/closure/contract-conformance/003-status.md` on `43fa2d7`, and `eggpack contract expand` answers exactly that question as a bounded local scalar projection. The roadmap now says the producer-side capability is closed and that any further reduction of the duplicated parsing is **consumer-owned cleanup** belonging to a plan registered in that consumer repository — not an open Eggpack question, and not something to smuggle into an adoption cutover.

### F4 — AGENTS.md current-handoff paragraph duplicated itself and contradicted itself

The paragraph repeated the Bootstrap M003 and Contract M003 closure sentences verbatim and then ended with two mutually exclusive claims: that M003b was "ready for plan authoring" and that it "remains blocked until M003a closes". Replaced with one compact statement sourced from the registry, per the standing rule that duplicated status prose is what goes stale.

### F5 — paired-plan registration became the durable handoff

Every active surface now names both exact plan paths, their exact registration commits (`eggstack/eggpack@325d44e`, `eggstack/eggsearch@377f9e8`, registry `6767062`), the exact Eggsearch implementation commit (`eabbf80`), and the hosted run (`37531104901`). The generic "mirrored plan required" statement is gone from every active surface.

## 4. Empty production / package / workflow diff (required by §7)

```text
git diff --check                                              clean
git status --short                                            only planning/AGENTS.md/docs
git diff <parent>..HEAD -- crates/ Cargo.toml Cargo.lock .github/
                                                               empty
```

M002 changed **no** Rust source, no Cargo metadata, no generated workflow, no package, and no external repository state. It is a planning-surface reconciliation only.

## 5. Stale-state sweep (WP5)

Every pattern required by the plan's §6 WP5 was searched across active surfaces (closure records and `plans/archive/` excluded, since historical text is preserved deliberately):

| Pattern | Active matches after the pass |
|---|---|
| `M003b remains blocked until M003a closes` | 0 |
| `M003b.*blocked.*M003a` | 0 |
| `one plan remains.*M003a` | 0 |
| `Contract M003.*research question` | 0 |
| `research/decision ready` | 0 |
| `unblocked for research/plan` | 0 |
| `mirrored implementation plan.*required` | 0 |

Remaining non-zero matches are confined to `plans/implementation/ecosystem-adoption/003a-…md` (the M003a plan body, which correctly describes the state as of its own authoring) and to the M002 plan body itself, which quotes the patterns it was written to remove. Both are historical/implementation-plan text, not active control surface.

One genuinely load-bearing ambiguity survived the sweep and was left intact deliberately: `plans/subsystems/planning-hygiene-roadmap.md` §3 says Contract M003's consumer-evidence gate "is satisfied, with the duplicated producer-fact parsing precisely identified across 2 repositories and 3 implementations". That is a statement about evidence, not about Contract M003 being open, and F3 now supplies the missing half — that the producer-side capability is closed. The two sentences are complementary rather than contradictory.

## 6. Verification (§7)

| Check | Result |
|---|---|
| `git diff --check` | clean |
| `git status --short` | only planning/AGENTS.md/docs files |
| `git diff <parent>..HEAD -- crates/ Cargo.toml Cargo.lock .github/` | **empty** |
| every current M003b control-surface reference agrees on status | yes — conditionally closed, with the same closure record, commits, run, and condition |
| Contract M003 described as closed wherever used as current evidence | yes |
| Bootstrap M003 described as closed with no remaining producer work | yes |
| M003a described as closed | yes |
| both paired M003b plan paths resolve | yes |
| Eggsearch unmodified by the reconciliation pass itself | yes — M002 made no external edit; the Eggsearch work is M003b's and is attributed to it |

Hosted CI was not required for this docs-only pass, per plan §7.

## 7. Remaining M003b obligations

M003b's implementation obligations are discharged and evidenced in
`plans/closure/ecosystem-adoption/003b-status.md`. Exactly one condition remains, and it is operational rather than technical:

> **Publication of eggsearch `v0.4.2` (draft `405157598`) is a manual maintainer action.** It is owned by `eggstack/eggsearch`, not by Eggpack.

Specifically still outstanding, and why:

- The draft is not published, so the `v0.4.2` assets are not downloadable from GitHub Releases.
- `Release provenance` has not been dispatched against `v0.4.2`, because it requires a staged draft and is the step immediately before publication. Provenance **subject parity against `v0.4.1` was established** (the `v0.4.1` attestation covers 16 subjects; the new workflow attests a strict superset of those 16 plus the two generated exact installers), so M003b §17.13 is satisfied and not deferred to this gap.
- No `v0.4.2` attestation exists yet, and none is claimed.

No Eggpack-side implementation, schema, or capability work is owed on this line.

## 8. Acceptance criteria

| # | Criterion | Status |
|---|---|---|
| 1 | no active Eggpack surface says M003a is still open | met |
| 2 | no active Eggpack surface says M003b is blocked on M003a | met |
| 3 | no active surface describes Contract M003 as still a research question | met |
| 4 | AGENTS.md contains exactly one current-handoff statement | met |
| 5 | registry, ecosystem roadmap, and AGENTS.md agree on paired M003b readiness | met — all three say conditionally closed with the same evidence |
| 6 | both paired plans referenced by exact path | met |
| 7 | no historical closure rewritten | met — only active surfaces touched |
| 8 | no production/package/workflow/external implementation changed | met — empty diff |
| 9 | unresolved M003b obligations remain explicit | met — §7 above, and `plans/closure/ecosystem-adoption/003b-status.md` §16 |
| 10 | `plans/closure/planning-hygiene/002-status.md` records the final state | met — this record |

## 9. Stop conditions

None of the plan's §9 stop conditions fired.

- Both paired plans registered without an unresolved architecture choice — M003a had already settled every choice with evidence.
- Eggsearch release-relevant source/config/workflow did not materially change from the reviewed baseline before registration; registration happened at `377f9e8`, before any implementation commit.
- M003b plan authoring required no Eggpack producer feature.
- The attestation-preservation design kept OIDC out of every generated job.
- No reconciliation required rewriting historical closure evidence.

## 10. Unresolved findings

None. M002 is a docs-only pass with no open finding.

One observation is recorded for the next planning pass rather than acted on here: `plans/registry.md` has grown several near-duplicate long status cells (the ecosystem row, the milestone index row, the planned/blocked row, the next-handoff paragraphs, and the downstream disposition table each restate M003b). They agree today, but that is five places to keep in sync by hand. The registry's own "prefer a pointer to the owning section over restating a rule" convention argues for collapsing them to one authoritative cell plus pointers. Doing so is a registry-structure change beyond M002's §4 scope and is not attempted here.

## 11. Next handoff

- **Eggpack planning hygiene**: no open work; M001 and M002 are closed.
- **Eggpack**: `plans/registry.md` is the control surface. No plan is `ready`; the next action, when one exists, is plan authoring.
- **Eggsearch (owner of the remaining M003b condition)**: publish `v0.4.2`, dispatch `Release provenance` for `v0.4.2`, verify public installers/updater.