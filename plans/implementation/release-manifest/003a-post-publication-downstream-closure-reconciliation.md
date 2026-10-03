# Release Manifest Milestone 003a — Post-Publication Downstream Closure Reconciliation

Status: closed

Closure record: `plans/closure/release-manifest/003a-status.md`

Class: planning / closure hygiene / cross-repository evidence reconciliation

Roadmap: `plans/subsystems/release-manifest-roadmap.md`

Predecessor closure: `plans/closure/release-manifest/003-status.md`

Eggpack authoring baseline: `64cc8453a572651f8f935add42a91d426d035f06`

External evidence baseline:

- Eggup M004 publication closure: `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`;
- Eggup formal post-closure status reconciliation: `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`;
- Eggup M004 publication source: `02a1d32931be29cc3d8980833643b2cd822f2d28`;
- Eggup hosted qualification: run `37090397398`, green on Stable, MSRV, macOS, and Windows lanes;
- published downstream set: `eggup-acquisition 0.1.2` -> `eggup-eggfetch 0.1.2` -> `eggup-eggpack 0.1.2`, all recorded closed by Eggup.

This is a documentation/planning corrective only. It does not reopen Release Manifest M003 and it does not authorize any Rust, package, registry, tag, release, workflow, or external-repository mutation.

## 1. Objective

Reconcile Eggpack's active planning and closure surfaces with the downstream state that became true after Release Manifest M003 closed:

1. Eggpack published and qualified `eggpack-manifest 0.1.0`;
2. Eggup consumed that prerequisite;
3. Eggup completed M004 and published its remaining 0.1.2 interoperability package chain;
4. Eggup formally reconciled its own stale blocked-state prose;
5. Eggpack still contains present-tense statements describing Eggup M004 and the 0.1.2 chain as unfinished.

The end state is a single truthful cross-repository narrative: the Eggpack-owned publication prerequisite and Eggup-owned registry promotion are both closed, the producer/consumer package seam is registry-resolvable end to end, and the next downstream migration is Eggsact-owned rather than an Eggpack blocker or handoff.

## 2. Why this is ready

All required evidence already exists and is immutable enough for a docs-only reconciliation.

Release Manifest M003 is closed at `plans/closure/release-manifest/003-status.md`. Its publication identity remains:

- `eggpack-manifest 0.1.0`;
- publication source `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`;
- crates.io checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`;
- tag `eggpack-manifest-v0.1.0`;
- hosted run `37064833069` green;
- external exact-`=0.1.0` registry-only smoke green;
- published `src/lib.rs` byte-identical to the consumer-qualified pin `678bbf04f5a02827003a1d9ab83ba4f0e6360e41`.

Eggup then closed the downstream registry promotion at `ea1f1c5` and performed a dedicated status reconciliation at `3b82d53`. No Eggpack code or package change is required to consume that evidence.

Per `plans/003-planning-process.md` sections 8, 9, 12, and 13, historical closure evidence is preserved while later cross-repository facts are reconciled through a new corrective plan/closure rather than silently rewriting history.

## 3. Current finding

At the authoring baseline, Eggpack's authoritative active surfaces are not fully aligned with the downstream closure.

Known stale present-tense claims include, but are not limited to:

- `plans/registry.md` describing Release Manifest M003 as the “next cross-repo gate” after it is already closed;
- `plans/registry.md` saying Eggup's `eggup-acquisition 0.1.2 -> eggup-eggfetch 0.1.2 -> eggup-eggpack 0.1.2` chain remains unfinished;
- `plans/registry.md` describing Eggup M004 as still blocked on that chain;
- `plans/subsystems/eggup-interoperability-roadmap.md` saying the three Eggup package publications remain unfinished and M004 remains Eggup-owned future work.

These statements were accurate when the M003 closure/handoff was authored. They became stale only after Eggup executed M004. The corrective must preserve that chronology rather than making the earlier closure appear to have known a future result.

## 4. Invariants

The reconciliation MUST preserve all of the following:

- Release Manifest M003 remains closed; M003a is not a runtime or publication requalification.
- Historical M003 evidence is not rewritten to erase the state that existed at closure time.
- The M003 closure's publication identity, checksum, tag, source commit, qualification evidence, and registry-only proof remain unchanged.
- Eggup M004 is described as downstream evidence reviewed from Eggup, not as work performed or closed by Eggpack.
- Eggpack claims no authority over Eggup publication policy or Eggsact migration policy.
- The next Eggsact Git-to-registry migration, if any, remains Eggsact-owned and separately authorized.
- The existing low-severity `eggpack-manifest 0.1.0` README dangling repository-link finding remains deferred to a future crate version; this corrective MUST NOT edit the published 0.1.0 source representation merely for cosmetic alignment.
- No crate version, dependency, lockfile, source file, release artifact, tag, GitHub Release, workflow, crates.io object, or credential is changed.
- No automatic publication mechanism is introduced.
- No external repository is modified by this Eggpack plan.

## 5. In scope

### 5.1 Active registry reconciliation

Update `plans/registry.md` so all active/current-state text consistently records:

- Release Manifest M003 closed and consumed downstream;
- Eggup M004 closed at `ea1f1c5`;
- Eggup's three-package 0.1.2 publication chain completed in dependency order;
- Eggup's formal planning cleanup at `3b82d53`;
- the producer/consumer registry seam is closed end to end;
- no Eggpack implementation work is unlocked by that downstream closure;
- Eggsact's Git-to-registry migration is the next downstream action, owned by Eggsact and not an Eggpack dependency-ready item.

Remove or reframe present-tense claims that M004 is blocked, that those packages are absent, or that Release Manifest M003 is still the next cross-repo gate.

### 5.2 Release Manifest roadmap reconciliation

Update `plans/subsystems/release-manifest-roadmap.md` to record the downstream receipt of M003:

- Eggup M004 consumed `eggpack-manifest 0.1.0`;
- `eggup-eggpack 0.1.2` now resolves the producer schema from crates.io at exact `=0.1.0`;
- adapter-only and Eggsact-shaped registry-only proofs passed downstream;
- no further Release Manifest implementation is implied.

M003 remains closed. M003a exists only to reconcile post-closure planning truth.

### 5.3 Eggup interoperability roadmap reconciliation

Update `plans/subsystems/eggup-interoperability-roadmap.md` so its current state and milestone table no longer describe Eggup M004 as future/blocked work.

Record:

- M004 closed downstream in Eggup;
- publication source `02a1d32`;
- closure `ea1f1c5`;
- formal Eggup status cleanup `3b82d53`;
- hosted run `37090397398` green;
- `eggup-acquisition 0.1.2`, `eggup-eggfetch 0.1.2`, and `eggup-eggpack 0.1.2` published;
- `eggup-eggpack 0.1.2` consumes registry `eggpack-manifest =0.1.0`;
- no Eggpack work remains on this interoperability seam.

Do not repurpose Eggpack interoperability's separately named “M004 bootstrap receipt compatibility” placeholder; it remains planned only if future evidence justifies it.

### 5.4 M003 closure downstream-receipt addendum

Do not rewrite the historical body of `plans/closure/release-manifest/003-status.md`.

Append a clearly dated/identified post-closure addendum that states:

- the closure's statement that Eggup's chain was unfinished was correct at M003 closure time;
- Eggup subsequently completed M004 at `ea1f1c5`;
- Eggup formally reconciled its planning at `3b82d53`;
- this later event confirms that the M003 handoff was successfully consumed;
- no M003 source, publication, checksum, tag, or qualification claim changes;
- no finding in the M003 closure is upgraded or reopened.

### 5.5 Corrective closure

Create `plans/closure/release-manifest/003a-status.md` recording:

- exact Eggpack implementation/reconciliation commit;
- exact external Eggup evidence commits;
- files changed;
- stale claims removed/reframed;
- proof that no production/package/workflow content changed;
- any hosted CI observed, or an explicit statement that no hosted runtime qualification was required because the delta is docs/planning-only;
- unresolved findings and downstream disposition.

Then mark M003a closed in the roadmap and registry.

## 6. Out of scope

This plan does NOT authorize:

- changes under `crates/`;
- `Cargo.toml` or `Cargo.lock` changes;
- package version changes;
- republishing, yanking, or modifying `eggpack-manifest 0.1.0`;
- publishing any other Eggpack workspace crate;
- Git tag or GitHub Release changes;
- CI/workflow changes;
- fixes to the known published README dangling repository-only link;
- changes in `eggstack/eggup` or `eggstack/eggsact`;
- Eggsact Git-to-registry migration;
- a new ReleaseManifest schema version;
- JSON Schema export;
- authenticity/signing work;
- bootstrap receipt work.

If any of those becomes necessary, stop and write the separately scoped plan required by the owning subsystem/repository.

## 7. Required production changes

None.

A successful implementation has zero production, package, dependency, lockfile, workflow, release, registry, or external-repository delta.

## 8. Ordered work packages

### WP1 — Revalidate external evidence and establish the reconciliation baseline

Before editing:

1. confirm Eggpack `main` still contains M003 closure `48ed13c` / handoff record `64cc845`;
2. confirm Eggup M004 closure `ea1f1c5` and formal status reconciliation `3b82d53` remain reachable on Eggup `main`;
3. review Eggup `plans/closure/eggpack-manifest-interoperability/004-status.md`;
4. inventory active Eggpack present-tense statements that still describe M004 or its publication chain as unfinished.

If Eggup has materially revised or reopened M004 after `3b82d53`, stop and re-review this plan.

### WP2 — Reconcile the Eggpack active registry

Update current-state, cross-repo mapping, execution graph, next-handoff, and downstream-disposition text as needed.

Keep historical evidence compact. Do not duplicate the full Eggup closure record into Eggpack.

### WP3 — Reconcile both affected subsystem roadmaps

Bring Release Manifest and Eggup Interoperability current-state/status sections onto the same post-M004 truth.

Do not create a new runtime dependency or imply that Eggsact migration blocks Eggpack.

### WP4 — Append the M003 downstream-receipt addendum

Append only. Preserve all original closure evidence and historical statements.

The addendum must clearly distinguish “state at M003 closure” from “later downstream completion.”

### WP5 — Consistency sweep

Search active planning surfaces for stale present-tense variants, including concepts equivalent to:

- Release Manifest M003 is the next cross-repo gate;
- Eggup M004 remains blocked;
- Eggup's 0.1.2 acquisition/eggfetch/eggpack chain remains unfinished;
- `eggup-acquisition` / `eggup-eggfetch` are still only 0.1.1;
- `eggup-eggpack` is still unpublished;
- Eggup can now begin M004.

Historical closure prose may retain time-correct language if its chronology is explicit; active registry/roadmap prose may not.

### WP6 — Close M003a

Write `plans/closure/release-manifest/003a-status.md`, mark the plan/roadmap/registry closed, and record the final handoff: no Eggpack work remains from the M003 -> Eggup M004 publication seam.

## 9. Compatibility, migration, and failure semantics

There is no API, wire-format, package, or runtime migration.

This pass changes only the description of already-completed cross-repository state.

Failure semantics:

- if evidence cannot be tied to exact Eggup commits, leave the affected claim unresolved rather than infer completion;
- if Eggup M004 has been reopened or corrected materially, stop and update this plan before applying stale assumptions;
- if a required fix touches code/package/workflow state, stop and split it into the owning subsystem's implementation plan;
- if a current Eggpack statement is historically scoped and accurate for its date, preserve it or clarify chronology rather than deleting evidence.

## 10. Verification

Because this is intentionally docs/planning-only, verification focuses on repository truth and scope containment.

Required:

```text
git diff --check
git status --short
git diff <implementation-parent>..HEAD -- crates/ Cargo.toml Cargo.lock .github/
```

The production/package/workflow diff above MUST be empty.

Run targeted stale-state searches against active planning surfaces. At minimum, inspect results for:

```text
rg -n "next cross-repo gate|remains .*blocked|chain remains .*unfinished|still at .*0\.1\.1|eggup-eggpack.*unpublished|can execute its registry-only promotion" \
  plans/registry.md \
  plans/subsystems/release-manifest-roadmap.md \
  plans/subsystems/eggup-interoperability-roadmap.md
```

No match may survive if it asserts a false present-tense state. Matches inside explicitly historical passages must be reviewed manually and must be time-qualified.

Also verify that the following exact downstream evidence appears in the reconciled current-state surfaces without being misattributed to Eggpack implementation:

- `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`;
- `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`;
- hosted run `37090397398`;
- publication source `02a1d32931be29cc3d8980833643b2cd822f2d28`.

Rust tests are not required solely for this pass because no Rust, Cargo, fixture, script, workflow, or package content may change. If normal hosted CI runs on the final docs commit, record the observed result in the corrective closure without implying it was necessary to validate runtime behavior.

## 11. Documentation requirements

The final state must make these distinctions obvious:

- Eggpack M003 publication: closed by Eggpack.
- Eggup M004 registry promotion: closed by Eggup after consuming the M003 handoff.
- M003a: docs/planning reconciliation only.
- Eggsact registry migration: downstream, separately authorized, not Eggpack-owned.
- Future `eggpack-manifest` package polish: separate future version/plan, not part of this corrective.

## 12. Acceptance criteria

M003a may close only when all are true:

1. `plans/registry.md` contains no false present-tense M004-blocked/publication-unfinished claim.
2. Release Manifest roadmap records downstream consumption without reopening M003.
3. Eggup Interoperability roadmap records M004 as closed downstream and no longer treats its package chain as a blocker.
4. M003 historical closure remains intact and gains only a clearly separated downstream-receipt addendum.
5. Eggup closure evidence is attributed to Eggup with exact commits/run, not claimed as Eggpack execution.
6. No production/package/workflow file changed.
7. No external repository changed.
8. No crate was published, yanked, retagged, or otherwise mutated.
9. The low-severity published README link finding remains explicitly deferred.
10. `plans/closure/release-manifest/003a-status.md` records the final evidence and closes the corrective.

## 13. Stop conditions

Stop and re-plan if:

- Eggup M004 or its formal closure is no longer the current accepted downstream state;
- any of the three published Eggup 0.1.2 versions is yanked or superseded in a way that changes the meaning of this handoff;
- the reconciliation requires changing Rust source, Cargo metadata, workflows, release artifacts, registry state, tags, or external repositories;
- evidence suggests the published `eggpack-manifest 0.1.0` bytes no longer correspond to the M003 closure identity;
- the work reveals a medium-or-higher runtime/security/compatibility defect rather than documentation drift.

## 14. Closure evidence

The M003a closure must include:

- Eggpack authoring baseline and final reconciliation commit;
- Eggup M004 closure `ea1f1c5`;
- Eggup formal status reconciliation `3b82d53`;
- Eggup publication source `02a1d32`;
- hosted run `37090397398`;
- exact list of Eggpack planning/closure files changed;
- stale-state inventory and disposition;
- empty production/package/workflow diff proof;
- `git diff --check` result;
- hosted CI result if one ran;
- unresolved findings by severity;
- explicit statement that M003 remains closed and no further Eggpack work is owed by the M003 -> Eggup M004 registry seam.
