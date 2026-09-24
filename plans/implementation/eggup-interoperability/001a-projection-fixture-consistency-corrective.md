# Eggup Interoperability Milestone 001a — Projection Fixture Consistency Corrective

Status: closed

Closure: `plans/closure/eggup-interoperability/001a-status.md`

Repository baseline: `dfd6eb3c6eebc97a371bc680ec21366eb04c924e` (Eggup Interop M001 closed; post-closure bundle projection defect confirmed)

Original implementation baseline: `99c9040a5d106cfa46a4ba02f9fa0cee8653166c`

Historical closure: `plans/closure/eggup-interoperability/001-status.md`

External Eggup API baseline: `eggstack/eggup@2cab1f97ef30fa347c2030da321462459672c521`

Source roadmap: `plans/subsystems/eggup-interoperability-roadmap.md`

Applicable process: `plans/003-planning-process.md#13-corrective-work`

Primary class: corrective / interface evidence

## 1. Objective

Correct the post-closure inconsistency between the checked-in CodeGG bundle ReleaseManifest fixture and its expected Eggup consumer projection, then make all direct/bundle/archive projection fixtures mechanically validated against their paired manifests.

The corrective must restore the claim that Eggpack's interoperability fixtures preserve exact artifact/install/member identity and ensure the same class of drift cannot pass CI again.

## 2. Why this corrective is required

Post-closure review found that:

- `bundle-manifest.json` contains three CodeGG bundle entries:
  - `codegg-2.4.0-x86_64-unknown-linux-gnu` -> `codegg`;
  - `codegg-helper-2.4.0-x86_64-unknown-linux-gnu` -> `codegg-helper`;
  - `codegg-manifest-2.4.0.json` -> `codegg-manifest.json`.
- `projection-bundle.json` instead contains only two acquisition units and substitutes an unrelated `eggsact` entry.
- Existing tests parse/round-trip the manifest fixtures and inspect the archive projection, but do not validate the bundle projection against the bundle manifest.

The M001 closure therefore overstates its evidence for bundle projection fidelity. The production ReleaseManifest wire format and helper APIs are not implicated.

## 3. Affected invariants

This corrective must preserve and prove:

- one projection acquisition unit for every direct/bundle release artifact selected for the consumer target;
- exact artifact filename, exact size, exact SHA-256, and exact install/member identity propagation;
- bundle projection cardinality equals manifest bundle cardinality;
- no extra, missing, substituted, or crossed bundle member relationship;
- one archive acquisition unit exactly matches the manifest archive artifact;
- archive member projection exactly preserves source/install/size/digest pairs;
- archive projection remains explicitly `extraction_required = true`;
- target selection is exact canonical matching with no alias/nearest-target fallback;
- projection fixtures remain documentation/test evidence only, not a new production wire format;
- Manifest v1 serialization stays unchanged;
- Eggpack gains no Eggup dependency;
- Eggup adapter implementation remains blocked until this corrective closes.

## 4. Scope

### In scope

- correct `projection-bundle.json`;
- mechanically validate direct, bundle, and archive projection fixtures against paired manifests;
- add negative regression tests for missing, extra, substituted, duplicate, and crossed projection relationships;
- preserve deterministic fixture JSON;
- update the historical M001 closure with a transparent post-closure corrective note;
- create `plans/closure/eggup-interoperability/001a-status.md` at closure;
- update interoperability roadmap/registry state;
- fix the stale Bootstrap M001 roadmap prose while touching planning hygiene.

### Out of scope

- modifying Eggup code;
- creating `eggup-eggpack`;
- changing ReleaseManifest v1;
- changing manifest helper APIs unless a tiny consumer-neutral test helper is strictly required;
- acquisition/network behavior;
- archive extraction;
- installation/ownership/rollback/service behavior;
- release selection/origin policy;
- authenticity/signatures;
- CI Orchestration work.

## 5. Required fixture correction

Replace the current bundle projection with an exact projection of `bundle-manifest.json`.

For target `x86_64-unknown-linux-gnu`, the projection must contain exactly these acquisition/member relationships:

```text
codegg-2.4.0-x86_64-unknown-linux-gnu
  -> member_id / relative_destination: codegg

codegg-helper-2.4.0-x86_64-unknown-linux-gnu
  -> member_id / relative_destination: codegg-helper

codegg-manifest-2.4.0.json
  -> member_id / relative_destination: codegg-manifest.json
```

Each unit must carry the exact size and SHA-256 from its paired manifest entry. The transaction group remains one coherent artifact set.

Do not "fix" the manifest to match the bad projection; the manifest is the producer evidence authority and is already valid.

## 6. Required projection validation harness

Add a test-side validation layer that loads the checked-in projection JSON and proves it is an exact projection of the paired ReleaseManifest fixture.

The projection schema may remain test/documentation-only, but tests must parse it into typed test structs or equivalently strict serde values with explicit assertions.

### A. Direct validation

For `direct-manifest.json` + `projection-direct.json`:

- selected target exists exactly;
- release identity equals manifest product/release;
- exactly one acquisition unit exists for the selected direct artifact;
- name, exact_size, sha256 equal manifest values;
- member_id and relative_destination equal manifest install identity;
- transaction group is one artifact set.

### B. Bundle validation

For `bundle-manifest.json` + `projection-bundle.json`:

- selected target exists exactly;
- acquisition unit count equals bundle entry count;
- every manifest entry maps to exactly one projection unit;
- no projection unit lacks a manifest entry;
- name/size/digest/install relationship matches entry-by-entry;
- duplicates and crossed relationships fail;
- transaction group denotes one coherent artifact set.

Comparison should be relationship-aware, not independent sets.

### C. Archive validation

For `archive-manifest.json` + `projection-archive.json`:

- exactly one acquisition unit equals the archive artifact name/size/digest;
- projected members exactly equal manifest member source/install/size/digest relationships;
- no extra/missing/crossed member;
- `extraction_required` is true;
- transaction grouping text/state remains consistent with consumer-owned extraction before Eggup ArtifactSet construction.

## 7. Detection-gap regressions

Add regressions that would fail under the original M001 fixture set:

1. substitute `eggsact` for `codegg-helper` in a bundle projection -> fail;
2. drop `codegg-manifest.json` -> fail;
3. add unrelated fourth bundle acquisition unit -> fail;
4. swap digests or destinations between two bundle units -> fail;
5. duplicate one bundle unit -> fail;
6. wrong selected target -> fail;
7. archive member source/install relationship crossing -> fail;
8. archive projection with `extraction_required = false` -> fail;
9. direct projection with wrong exact_size/digest -> fail.

Tests should exercise the actual checked-in projection fixtures, not only generated in-memory equivalents.

## 8. Production-code boundary

Prefer keeping the validation harness in tests because the projection files are explicitly not a production schema.

Do not add a general "Eggup projection" API to `eggpack-manifest` merely to validate documentation fixtures.

A small generic helper may be added only if it is independently useful to all manifest consumers and does not name or encode Eggup semantics. Any such addition must be additive, non-wire, and justified in closure evidence.

## 9. Closure and planning cleanup

Preserve `plans/closure/eggup-interoperability/001-status.md` as historical evidence. Add a short post-closure note that:

- identifies the incorrect bundle projection;
- states that existing CI did not compare projection fixtures pairwise;
- points to M001a;
- withdraws the prior "M002 ready" transition until M001a closes.

At M001a closure, create `plans/closure/eggup-interoperability/001a-status.md` with:

- corrected fixture blob identities/hashes;
- exact pairwise projection matrix;
- regression evidence;
- unchanged Manifest v1 evidence;
- no Eggup dependency evidence;
- hosted CI;
- explicit decision on whether Eggup adapter plan authoring is re-unblocked.

Also correct the stale Bootstrap roadmap sentence that still calls its already-closed M001 "ready".

## 10. Eggup cross-repository gate

No Eggup implementation plan is authorized by this corrective.

Until M001a closes:

- Eggpack registry/interop roadmap mark Eggup adapter M002 blocked;
- Eggup registry/roadmap must state that optional manifest-adapter plan authoring is blocked on Eggpack M001a closure;
- do not create or implement `eggup-eggpack`;
- do not copy the flawed M001 fixture baseline into Eggup.

After M001a closes, Eggup may author a new adapter implementation plan against the corrected fixture baseline and current Eggup API baseline.

## 11. Ordered work packages

1. Add a failing regression against the current checked-in `projection-bundle.json`.
2. Correct the bundle projection to match all three manifest entries.
3. Implement strict pairwise direct/bundle/archive projection-fixture validation.
4. Add the detection-gap negative matrix.
5. Re-run Manifest v1 round-trip/helper tests and workspace tests.
6. Confirm no Manifest JSON wire change and no Eggup dependency.
7. Update historical closure annotation, interop roadmap, registry, and Bootstrap stale prose.
8. Run package/MSRV/docs/hosted CI.
9. Write M001a closure with corrected fixture identities and re-evaluate Eggup adapter readiness.

## 12. Failure/restart/contention semantics

This is deterministic fixture/test/planning work with no network or mutation outside the repository test process.

A projection mismatch fails tests/validation; there is no partial runtime consumer projection or installation behavior.

## 13. Compatibility and migration

ReleaseManifest v1 remains unchanged.

The corrected bundle projection is a test/documentation fixture compatibility correction, not a production schema migration.

The old projection blob remains traceable through Git history and the original M001 closure's historical hash inventory; do not rewrite history to imply the bad blob never existed.

## 14. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-manifest --locked
cargo package -p eggpack-manifest --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Record Linux stable, Linux Rust 1.89, macOS, and Windows hosted CI.

## 15. Documentation updates

Update:

- `plans/closure/eggup-interoperability/001-status.md` with historical corrective annotation;
- `plans/subsystems/eggup-interoperability-roadmap.md`;
- `plans/registry.md`;
- `plans/subsystems/bootstrap-installers-roadmap.md` stale M001 prose;
- new `plans/closure/eggup-interoperability/001a-status.md` at closure.

The architecture mapping document only needs modification if the corrected fixture exposes a contradiction in the written contract. The current defect appears to be fixture/test drift, not an architecture-contract defect.

## 16. Acceptance criteria

M001a closes only when:

- `projection-bundle.json` exactly represents all three CodeGG manifest bundle entries;
- direct, bundle, and archive projection fixtures are mechanically checked against paired manifests;
- missing/extra/substituted/crossed projection relationships fail;
- the actual checked-in projection files are exercised by tests;
- Manifest v1 JSON wire output is unchanged;
- no Eggup dependency is introduced;
- original closure history remains transparent;
- stable/MSRV/package/docs/hosted CI pass;
- no unresolved medium-or-higher interop-evidence defect remains.

## 17. Stop conditions

Stop and re-plan if:

- pairwise projection cannot be defined without changing Manifest v1;
- Eggup requires producer/build types to preserve direct/bundle mapping;
- the projection model reveals a real ambiguity in bundle or archive identity;
- fixing the fixture requires adding release selection/origin/install ownership to the manifest;
- archive extraction semantics must be defined to validate archive producer evidence.

## 18. Closure evidence required

Record:

- exact implementation/review SHA;
- original bad projection identity and corrected projection identity;
- corrected direct/bundle/archive projection matrix;
- negative regression matrix;
- exact test commands/results;
- Manifest wire no-change evidence;
- dependency-tree/no-Eggup evidence;
- hosted CI run;
- unresolved findings;
- explicit transition: Eggup adapter plan remains blocked or becomes ready to author.

## 19. Handoff notes

This is the sole Eggpack interoperability corrective. Keep it narrow and evidence-driven.

CI Orchestration M001 is unrelated and may be planned independently. Eggup runtime/service work is unrelated. Do not modify Eggup code from this plan.
