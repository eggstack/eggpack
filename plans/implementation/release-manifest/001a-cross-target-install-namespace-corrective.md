# Release Manifest Milestone 001a — Cross-Target Install Namespace Corrective and Closure Cleanup

Status: closed

Closure record: `plans/closure/release-manifest/001a-status.md`

Repository baseline: `b71f8b8bc9edca9da58cb0432b4839edc9fa5bd4` (Release Manifest M001 implementation and hosted closure evidence landed)

Affected implementation: `b5df057a0ab8d30664aeccc26aa7944678626930`

Historical closure record: `plans/closure/release-manifest/001-status.md`

Source roadmap: `plans/subsystems/release-manifest-roadmap.md`

Applicable architecture:

- `plans/000-long-term-specification.md#12-concrete-release-manifest`
- `plans/001-terminology-and-domain-model.md#11-release-manifest`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`
- `plans/003-planning-process.md#13-corrective-work`

Primary class: corrective / invariant / closure hygiene

## 1. Objective

Correct ReleaseManifest v1 validation so installed-name uniqueness is enforced within each canonical target's installed namespace rather than across mutually exclusive targets in the whole release. Preserve the global flat release-artifact filename collision guard. Add regression evidence using representative multi-target direct, bundle, and archive manifests, then reconcile the registry/roadmaps and re-close the manifest schema before downstream producer milestones proceed.

## 2. Why this corrective is required

Post-closure review found that `ReleaseManifest::validate()` creates one `installs` set before iterating all target records. As a result, the same install name appearing on two different canonical targets is rejected as a duplicate.

That contradicts the existing DistributionContract model and checked-in evidence. For example, `crates/eggpack-contract/tests/fixtures/simple-direct.toml` validly expands both Linux x86_64 and macOS arm64 to install name `eggsact`. `egress-archive.toml` likewise installs `egress` and `egress-helper` for more than one target. Those target-local namespaces are mutually exclusive at installation time.

The defect was not caught by M001 because its manifest tests exercised only one target per manifest.

## 3. Affected invariants

This corrective must restore and lock down all of the following:

- canonical target records are independent installation namespaces;
- install-name exact/ASCII-case collisions fail within one target;
- identical install names across different canonical targets are valid;
- release artifact filenames remain globally unique under exact/ASCII-case comparison because one concrete release inventory is flat;
- archive member source uniqueness remains scoped to one archive target;
- bundle artifact/install relationships and archive source/install/member-byte relationships remain intact;
- deterministic serialization and schema-v1 wire shape do not change;
- ProductId, ReleaseId, SourceRevision, sizes, digests, bounds, unknown-field/version behavior, and evidence-reference semantics remain unchanged;
- no build/network/process/archive-extraction/install/trust authority is introduced.

## 4. Scope

### In scope

- fix the install-name collision scope in `eggpack-manifest`;
- add multi-target regression tests;
- explicitly test the distinction between global release-file namespace and per-target install namespace;
- retain current schema-v1 JSON representation;
- reconcile stale planning/closure state created by the newly discovered defect;
- create a new corrective closure record rather than rewriting M001 history.

### Out of scope

- changing DistributionContract v1;
- changing manifest JSON fields or schema version;
- adding the Manifest M002 final-artifact builder;
- build/qualification implementation;
- bootstrap installer implementation;
- Eggup adapter implementation;
- signatures, authenticity, canonical signing JSON, publication, or acquisition;
- broad refactors unrelated to collision scope.

## 5. Required production change

In `ReleaseManifest::validate()`, keep release artifact filename tracking at manifest scope, but instantiate install-name collision tracking separately for each `TargetRecord`.

Conceptually:

```text
one ReleaseManifest
  |
  +-- global flat release artifact namespace
  |
  +-- target A
  |     `-- target-local install namespace
  |
  +-- target B
        `-- target-local install namespace
```

Do not weaken within-target collision checks. Direct, bundle, and archive forms must continue to reject duplicate or ASCII-case-colliding install identities inside the same target.

Do not move release artifact filename tracking into target scope. Two targets emitting the same release filename would collide in the concrete release inventory and must still fail.

## 6. Required regression matrix

Add tests covering at least:

1. Two direct targets with distinct release artifact names and the same install name: accepted.
2. Two archive targets with distinct archive artifact names and the same member install names: accepted.
3. Two bundle targets with distinct release artifact names and the same per-target install names: accepted.
4. One target containing duplicate exact install names: rejected.
5. One target containing ASCII-case-colliding install names: rejected.
6. Two targets emitting the same release artifact filename: rejected globally.
7. Two targets emitting ASCII-case-colliding release artifact filenames: rejected globally.
8. Existing direct/bundle/archive round-trip and deterministic JSON tests remain unchanged and green.
9. A regression fixture mirrors the checked-in Eggsact direct contract behavior closely enough that `eggsact` may install under both Linux x86_64 and macOS arm64.
10. A regression fixture mirrors the checked-in Egress archive behavior closely enough that `egress` / `egress-helper` may repeat across target-local install namespaces.

A production dependency on `eggpack-contract` is not justified by this corrective. If cross-crate fixture reuse would add package/dependency complexity, encode the representative manifest values directly and cite the source fixtures in test comments/docs.

## 7. Closure and planning cleanup

Preserve `plans/closure/release-manifest/001-status.md` as historical closure evidence. Do not rewrite it to imply the defect was known at original closure.

Create `plans/closure/release-manifest/001a-status.md` when this corrective closes. It must record:

- defect reproduction before the fix;
- exact implementation SHA;
- corrected namespace semantics;
- full regression matrix;
- stable/MSRV/package/docs/hosted-CI evidence;
- confirmation that the schema-v1 wire representation did not change;
- confirmation that Manifest M002, Build/Qualification M001, Bootstrap Installers M001, and Eggup interoperability M001 may be unblocked again only after this corrective closes.

At implementation closure, update the original M001 closure only if needed with a short historical addendum pointing to M001a; do not alter its original evidence or conclusions retroactively.

Planning cleanup in this corrective must also ensure:

- registry execution graph no longer labels Manifest M001 merely `READY`;
- release-manifest roadmap current-state text reflects that `eggpack-manifest` exists;
- Eggup interoperability roadmap current-state text reflects that ReleaseManifest v1 exists;
- dependent milestones are marked blocked while M001a is open and re-evaluated at M001a closure.

## 8. Ordered work packages

1. Add a failing multi-target direct regression proving the current global install namespace defect.
2. Refactor install collision tracking to target scope while keeping release artifact tracking manifest-global.
3. Add archive and bundle multi-target acceptance tests.
4. Add within-target and global-release collision negative tests.
5. Run the complete existing manifest/contract/workspace test suites and deterministic JSON goldens.
6. Review public schema/wire output for zero changes.
7. Run stable/MSRV/package/docs/dependency qualification.
8. Observe hosted Linux/macOS/Windows CI.
9. Write `001a-status.md`, reconcile planning state, and only then re-unblock downstream milestones.

## 9. Compatibility and migration

This is a validation-correctness change within schema v1, not a schema evolution. Manifests that were semantically valid under the DistributionContract model but falsely rejected by the M001 validator become accepted. No previously valid within-target collision becomes valid.

Serialized field names, enum tags, ordering rules, digest encoding, size semantics, and version number must remain byte-compatible for equivalent values.

No ADR is required unless implementation evidence shows that collision scope cannot be corrected without changing the public manifest schema or the DistributionContract/ReleaseManifest authority boundary.

## 10. Required verification commands

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
cargo +1.89.0 test -p eggpack-contract --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted CI must pass the repository's Linux stable, Linux Rust 1.89, macOS stable, and Windows stable lanes.

## 11. Acceptance criteria

M001a closes only when:

- the pre-fix regression demonstrably fails for a valid multi-target repeated install identity;
- the same case passes after the fix;
- install collision checks remain strict within each target;
- release artifact collisions remain strict across the whole manifest;
- direct, bundle, and archive multi-target cases are covered;
- schema-v1 serialized representation is unchanged;
- all prior M001 and Contract M001/M002 tests remain green;
- stable/MSRV/package/docs/dependency checks pass;
- hosted CI passes;
- planning and closure state is reconciled;
- no unresolved medium-or-higher finding remains in the manifest domain.

## 12. Stop conditions

Stop and prepare an ADR/corrective redesign if:

- fixing namespace scope requires a schema-v1 wire-format change;
- a concrete release inventory cannot retain one global release-file namespace;
- target-local installation identity cannot be represented without introducing consumer install-policy authority into the manifest;
- the fix requires a build, network, Eggup, or installation dependency in `eggpack-manifest`;
- additional post-closure defects materially change manifest identity or trust semantics.

## 13. Downstream gate

Until M001a closes, keep these blocked in Eggpack:

- Release Manifest M002 final-artifact builder;
- Build/Qualification M001 PackConfig/ReleasePlan;
- Bootstrap Installers M001;
- Eggup interoperability M001 implementation planning beyond interface notes.

Eggup M004 retirement remains authorized by Contract M002 and is independent of this manifest corrective.

Gate disposition: this condition was satisfied when M001a closed. The newly ready milestones and remaining blockers are recorded in `plans/closure/release-manifest/001a-status.md` and the updated registry.

## 14. Handoff

At registration, this was the sole dependency-ready Eggpack implementation handoff. The corrective is now closed; follow-on milestone plans must be registered before their implementation begins.
