# Bootstrap Installers Milestone 003 — Two-Consumer Adoption and Receipt-Boundary Decision

Status: closed

Closure record: `plans/closure/bootstrap-installers/003-status.md`

Closure disposition (2026-10-05): M003 is closed as the evidence/ownership decision pass it was registered as, with **zero production, configuration, or workflow changes in Eggpack and zero changes in either consumer repository**. Reviewed against Eggpack `013e091`, eggsact `d4e6e5c` (release `v1.2.7` -> `d8014cfe`), stegoeggo `v0.5.0` -> `57ca94c9`. All five candidate-review questions are answered with evidence: the existing exact-installer generator generalizes across both live consumers with every embedded name/size/SHA-256 equal to the consumer's own published `release-manifest.json`; remaining wrapper logic is product policy under ADR-0001 apart from one consumer-side asset-name reconstruction recorded as low-severity F-1; wrapper delegation is **declined** because neither installer form exposes a failure/outcome channel that could preserve a 404-only Cargo fallback while hard-failing 5xx/TLS/digest errors; and an **Eggup receipt handoff is rejected** because bootstrap installs exact bytes while Eggup owns transaction and rollback state. No common producer gap was found, so no M003a corrective was registered, and the roadmap §11 completion definition is satisfied. The optional consumer-side cleanup of F-1 becomes eligible once Contract M003 provides `eggpack contract expand --field asset`, under a plan registered in the consumer's own repository.

Repository implementation baseline: `911da48c7c1c7967397a2d190730fc643c8c6dc3`

Source roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#13-bootstrap-installers`
- `plans/001-terminology-and-domain-model.md#13-bootstrap-installer`
- `plans/002-long-term-roadmap.md#phase-6--bootstrap-installer-generationconformance`
- `plans/003-planning-process.md`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: capability evidence / architecture-boundary decision / polish

## 1. Objective

Close the question Bootstrap M003 was created to answer now that two independent public consumers exist:

1. do the existing Eggpack-generated exact installers actually generalize across two real products without another producer primitive;
2. which remaining installer behavior is legitimate product-owned wrapper policy rather than duplicated producer authority;
3. should bootstrap installation emit or hand off an Eggup receipt, or should that remain exclusively in Eggup's deployment transaction boundary;
4. does the subsystem completion definition already hold, or is one narrowly justified follow-up bootstrap capability required.

This milestone is a candidate-review and closure decision, not a presumption that more code is needed.

Expected outcome from current evidence: the existing exact-installer generator is reusable; public wrappers retain legitimate product selection/fallback policy; Eggup receipt semantics remain out of bootstrap scope. If the evidence confirms that, M003 closes with documentation/evidence only.

If a common producer-side defect or missing primitive is found, stop and author a separately bounded M003a corrective/feature plan rather than implementing it opportunistically here.

## 2. Why this is ready

Planning Hygiene M001 discharged the only hard blocker.

Two independent consumers now publicly ship Eggpack-generated exact installers:

### Eggsact

- Ecosystem M001 closed on public `v1.2.7`;
- live Eggpack run `36652731202`;
- published release inventory includes product wrappers plus `install-exact.sh` / `install-exact.ps1`;
- exact/latest wrapper smoke and update checks passed;
- generated exact installers derive their target/artifact/checksum facts from Eggpack.

### StegoEggo

- producer cutover implementation `3b96fae`;
- consumer closure `c75132a`;
- public ordinary stable `v0.5.0`;
- Eggpack run `37181914252`;
- exact 15-asset inventory with generated exact installers plus product wrappers;
- public installer smoke passed;
- real public `0.4.2 -> 0.5.0` updater transition passed.

The original "two-consumer adoption evidence" dependency is therefore satisfied.

## 3. Current architecture to adjudicate

### 3.1 Eggpack-generated exact installer authority

The closed Bootstrap M001/M002/M002a line already owns:

- deterministic POSIX/PowerShell exact-release installer rendering;
- target -> artifact/install-name projection from `DistributionContract`;
- exact finalized size/SHA-256 from `ReleaseManifest`;
- bounded acquisition;
- private staging/cleanup;
- fail-closed integrity checks;
- direct/bundle/archive installation safety as qualified by the historical milestones.

M003 must not reopen that implementation merely because public wrappers still exist.

### 3.2 Product wrapper authority

Both adopted consumers intentionally retain public wrapper behavior such as:

- latest vs explicit version selection;
- crates.io/version authority;
- Cargo fallback policy;
- install destination/PATH policy;
- product-specific candidate/version checks;
- user-facing fallback/error wording.

Those are application release/install policy under ADR-0001.

A wrapper containing some host classification does not automatically make it a second producer authority. The review must distinguish "policy needed to choose a path" from "duplicated artifact/checksum truth that should have been delegated."

### 3.3 Eggup receipt boundary

Bootstrap is first-install producer output.

Eggup owns:

- local deployment transaction state;
- update/rollback/recovery;
- destination replacement;
- install receipts;
- optional service lifecycle.

M003 SHOULD conclude that generated bootstrap scripts do not mint an Eggup receipt unless concrete cross-repository evidence proves a consumer transaction can safely accept a producer bootstrap handoff without shifting authority.

A design convenience is insufficient evidence for changing this boundary.

## 4. Invariants

- No runtime/product policy moves from consumer repositories into Eggpack.
- No Eggup transaction/receipt semantics move into generated bootstrap installers without a new architecture decision.
- Generated exact installers remain release-specific and do not select latest versions.
- Product wrappers may continue selecting latest/exact/Cargo fallback paths.
- Integrity remains distinct from authenticity.
- Bootstrap does not become self-update.
- Existing exact installer safety semantics are not weakened.
- Historical M001/M002/M002a closures remain historical evidence.
- M003 does not modify external repositories.
- A discovered feature gap gets a new plan; it is not silently implemented inside this decision pass.

## 5. In scope

Read-only/adjudication work:

- compare Eggsact and StegoEggo public wrapper semantics;
- compare their generated exact installer presentation and release inventory;
- inspect current `eggpack-bootstrap` and staging integration;
- classify duplicated vs product-owned wrapper logic;
- inspect public-release smoke/closure evidence;
- evaluate whether any common two-consumer requirement is absent from the current generator;
- explicitly decide Eggup receipt handoff disposition;
- update bootstrap roadmap/registry;
- create `plans/closure/bootstrap-installers/003-status.md`.

Targeted local verification of the current bootstrap crate is in scope.

## 6. Out of scope

- changing generated script bytes;
- adding wrapper delegation;
- adding new exit-code protocols;
- adding release selection/latest lookup;
- adding Cargo/package-manager fallback to generated exact installers;
- changing install destinations;
- Eggup code/receipt changes;
- consumer-repository changes;
- service lifecycle;
- signatures/attestations;
- package publication.

If review shows one of these is necessary for subsystem completion, stop and write M003a with that exact scope.

## 7. Candidate-review questions

The closure MUST answer each question explicitly.

### Q1 — Are target/artifact/checksum producer facts singular?

For each consumer identify:

- canonical Eggpack contract/config authority;
- generated exact installer source;
- any remaining hand-written producer fact;
- whether that fact is actually product policy.

A frozen public-name compatibility table may remain consumer-owned if it is a guard against accidental public contract change; it must not be treated as the canonical generator input.

### Q2 — Do exact installers cover the required public target form?

Compare both consumers' public exact installers with the target sets actually published.

Any unsupported target that deliberately takes a product-owned Cargo fallback path belongs to the wrapper unless the exact release contract declares an artifact for it.

### Q3 — Is wrapper-to-exact-installer delegation now warranted?

Do not assume yes.

Evaluate whether delegation would preserve, without ambiguous exit parsing:

- latest/exact selection;
- 404-only fallback policy;
- unsupported-host fallback;
- hard failure on checksum/version/TLS/5xx errors;
- install-destination behavior.

If delegation requires a new stable exit-status/protocol contract, that is a separate capability and MUST be planned separately.

### Q4 — Is an Eggup receipt handoff warranted?

Default answer is no unless evidence demonstrates a real consumer benefit and a safe ownership boundary.

The closure must distinguish:

- "bootstrap installed exact bytes";
- "Eggup transaction committed/owns rollback state".

They are not equivalent.

### Q5 — Does Bootstrap subsystem completion hold now?

The roadmap completion definition is at least two consumers removing duplicated mapping/checksum authority from bootstrap installers.

Determine this based on effective authority, not the mere existence of wrapper code.

## 8. Ordered work packages

1. Freeze exact Eggpack/Eggsact/StegoEggo evidence baselines.
2. Build before/after authority maps for both consumers.
3. Compare wrapper vs exact-installer responsibilities.
4. Audit two public release inventories and installer smokes.
5. Run current targeted bootstrap tests and docs checks.
6. Decide Q1-Q5 with evidence.
7. If no producer gap exists, write M003 closure and mark the bootstrap milestone line complete.
8. If a producer gap exists, leave M003 open/blocked on a newly-authored M003a plan; do not code the gap in this pass.
9. Reconcile registry, roadmap, and AGENTS handoff.

## 9. Failure/restart semantics

This pass is non-mutating outside planning/docs.

If evidence is incomplete or contradictory:

- do not infer success;
- record the exact missing receipt/run/file;
- leave M003 open with that evidence blocker.

If a common medium-or-higher safety defect is found in generated installers:

- stop closure;
- create a corrective implementation plan;
- do not classify it as "consumer-specific" merely to finish M003.

## 10. Compatibility and migration

No consumer migration is authorized.

Any later wrapper delegation must be separately planned in each consumer repository because wrapper policy is product-owned.

No Eggup migration is authorized.

## 11. Verification

Required Eggpack checks:

```bash
cargo fmt --all -- --check
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked
cargo doc -p eggpack-bootstrap --no-deps --locked
git diff --check
```

Read-only external evidence review:

- Eggsact `v1.2.7` closure/release inventory/installer smoke;
- StegoEggo `v0.5.0` closure/release inventory/installer smoke;
- exact generated installer names and public wrapper names;
- relevant release-contract scripts.

Do not modify either external repository.

## 12. Documentation requirements

If M003 closes without code, update:

- bootstrap roadmap;
- registry;
- `AGENTS.md` next handoff if necessary;
- closure record.

The closure must explicitly state:

- whether the subsystem completion definition is satisfied;
- why remaining wrapper policy is or is not duplicate producer authority;
- Eggup receipt decision;
- whether wrapper delegation deserves any separate future milestone.

## 13. Acceptance criteria

M003 closes only if:

1. both independent public-consumer release/installer evidence sets are reviewed;
2. current exact installers are shown to derive producer facts from Eggpack;
3. any remaining wrapper duplication is classified by ownership with concrete examples;
4. no unresolved common producer-side bootstrap capability is needed for either live consumer;
5. generated exact installers retain fail-closed size/SHA and first-install boundaries;
6. the Eggup receipt decision is explicit and ownership-correct;
7. targeted Eggpack bootstrap tests are green;
8. no unresolved medium-or-higher bootstrap defect remains;
9. no external repository or production source changed;
10. `plans/closure/bootstrap-installers/003-status.md` records the evidence and disposition.

## 14. Stop conditions

Stop and author a separate plan if:

- both consumers need the same missing bootstrap primitive;
- wrapper delegation requires a new stable machine-readable/exit-code protocol;
- exact installers cannot express a published target safely;
- consumer policy must move into Eggpack to eliminate duplication;
- an Eggup receipt becomes necessary to preserve correctness;
- any medium-or-higher installer safety issue is found.

## 15. Closure evidence

Create `plans/closure/bootstrap-installers/003-status.md` with:

- reviewed repository/release baselines;
- two-consumer authority matrix;
- generated/public installer inventory;
- wrapper policy comparison;
- targeted test results;
- receipt-handoff decision;
- subsystem completion decision;
- unresolved findings by severity;
- any M003a follow-up required;
- downstream handoff.
