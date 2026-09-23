# Contract and Conformance Milestone 001 — Workspace Bootstrap and DistributionContract v1 Migration

Status: ready for handoff

Repository baseline: `f3762d81fe86d0a8b53a0cabe37562c56a2b7007`

External source baseline:

- `eggstack/eggup@cf5b3d3819c168eb2dbf841daa8332f3eb28c915`
- predecessor distribution M001 implementation: `889a234cbe7f461d92def3df45c83c06a7d257e5`
- predecessor distribution M002 corrective: `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`

Source roadmap:

- `plans/subsystems/contract-conformance-roadmap.md#M001--bootstrap-workspace-and-faithfully-import-distributioncontract-v1`

Long-term requirements:

- `plans/000-long-term-specification.md#5-layer-model`
- `plans/000-long-term-specification.md#6-portable-distribution-contract`
- `plans/000-long-term-specification.md#7-artifact-forms`
- `plans/001-terminology-and-domain-model.md#4-distribution-contract`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: invariant / infrastructure

## 1. Objective

Bootstrap Eggpack as a Rust workspace and establish `eggpack-contract` by faithfully migrating the corrected, unpublished `eggup-dist` DistributionContract v1 implementation and its evidence into this repository.

This milestone is intentionally fidelity-first. It creates the proven portable release-layout boundary in its correct producer-side home without extending schema semantics, adding build policy, or deleting the predecessor from Eggup.

## 2. Why this milestone is ready

Hard and interface dependencies are satisfied:

- Eggpack's canonical planning and ownership ADRs are accepted.
- The new repository contains no production code that must be migrated around.
- Eggup distribution M001 established schema v1 and closed with implementation `889a234c...`.
- Eggup distribution M002 closed the template-grammar and expanded-name-collision defects with implementation `0a68f29...`.
- `eggup-dist` remains unpublished, so moving the crate boundary does not require preserving a published crate name/API for third parties.
- Eggup runtime crates do not depend on `eggup-dist`, so the migration does not cut a runtime dependency edge.
- The source baseline is frozen above for this handoff. Later Eggup changes do not silently alter the migration target.

No unresolved architecture decision is required for a faithful import.

## 3. Current implementation evidence

At the external source baseline, `eggup-dist` is a small release-time crate with Rust 2021 edition and MSRV 1.89; `serde` plus `toml 0.8` as its direct schema dependencies; explicit `schema_version = 1`; strict unknown-field rejection; opaque product/version identity subject only to bounded filesystem safety; canonical target triples plus globally unambiguous aliases; fixed `direct | bundle | archive` asset forms; strict non-executable placeholder grammar; flat asset/install/sidecar names; traversal-free literal archive member paths; release-file/install-name namespace uniqueness including ASCII-case-only collision rejection; and deterministic parse/expand behavior.

The predecessor fixtures represent simple direct, CodeGG-like bundle, and Egress-like archive layouts. The predecessor M002 closure recorded 24 unit tests plus 4 fixture tests passing on stable and Rust 1.89. Those counts are provenance rather than acceptance by themselves: Eggpack closure must identify which predecessor behaviors were reproduced even if test organization/count changes.

Eggpack currently has planning documents only and no Cargo workspace.

## 4. Invariants that must not regress

- Valid corrected schema-v1 documents retain the same meaning after migration.
- Unsupported schema majors fail with a typed error.
- Unknown fields fail closed.
- Target/alias lookup is deterministic; unknown targets never guess.
- Duplicate triples, duplicate aliases, and alias/triple collisions fail.
- Template grammar remains a small substitution grammar, not shell/general templating.
- Unknown, malformed, nested, unmatched, empty, whitespace-bearing, or context-invalid placeholders fail.
- `{asset}` remains checksum-sidecar-context-only.
- Expanded release-file and install-name namespaces reject exact and ASCII-case-only collisions.
- Assets/install names/sidecars remain flat filenames.
- Archive member sources remain literal, relative, normalized, traversal-free paths.
- Product version remains opaque; generic contract code does not impose SemVer ordering.
- SHA-256 sidecar naming is integrity metadata and is never represented as independent authenticity.
- Contract parsing/validation/expansion performs no network I/O, subprocess execution, archive extraction, installer generation, or live installation.
- `eggpack-contract` does not depend on Eggup.
- Eggup's existing `eggup-dist` copy remains intact until a later Eggup migration/cleanup plan explicitly removes or deprecates it.

## 5. Scope

### In scope

- Create root Rust workspace metadata.
- Set workspace baseline to Rust 1.89 and edition 2021 for migration fidelity.
- Establish workspace lints that deny/forbid unsafe code consistently with the predecessor's security posture.
- Add `crates/eggpack-contract`.
- Migrate/adapt the corrected `eggup-dist` implementation into the new crate.
- Rename crate/package-level identifiers from Eggup-specific distribution tooling to Eggpack ownership where needed without changing schema-v1 meaning.
- Preserve public semantic domain names such as `DistributionContract` where they remain correct.
- Import the corrected predecessor unit tests and representative fixtures.
- Add explicit migration/provenance documentation identifying the frozen source baseline and predecessor closure commits.
- Add project README/package documentation and a change record.
- Add local verification helper(s) only if they remain thin wrappers around documented Cargo commands.
- Add ordinary hosted CI sufficient to prove Linux stable/MSRV and practical macOS/Windows portability of this pure crate.
- Verify `cargo package` and dependency-tree boundaries for `eggpack-contract`.

### Explicitly out of scope

- ReleaseInventory/ArchiveMemberInventory/conformance-report implementation from the predecessor M003 plan.
- Concrete ReleaseManifest.
- PackConfig or ReleasePlan.
- build runners, Cargo/cargo-zigbuild invocation, QEMU, or qualification hooks.
- network access, GitHub APIs, archive extraction, installer generation, CI generation.
- `eggpack` CLI beyond any strictly necessary test/dev harness.
- signatures, attestations, SBOMs.
- publishing a crate.
- removing/deprecating `eggup-dist` in Eggup.
- changing schema-v1 semantics merely to make future Eggpack work easier.

## 6. Required production changes

### Workspace and package topology

Create a minimal workspace containing root Cargo metadata, Cargo.lock, LICENSE, README/CHANGELOG, architecture documentation, `crates/eggpack-contract`, and the existing planning tree.

Do not create placeholder production crates for manifest/core/CLI yet.

Workspace package metadata should identify MIT licensing and the Eggpack repository.

### Contract implementation

Port the corrected source at the frozen Eggup baseline. Preserve raw/validated shape separation, schema-v1 version checks, unknown-field rejection, target/alias validation, fixed asset forms, strict template parser, archive-source validation, bounded inputs, deterministic expansion, and `NameCollision`/namespace semantics.

Use Eggpack crate/module naming where appropriate, but do not refactor semantic behavior during this milestone.

### Fixtures and golden evidence

Import equivalent fixtures for simple direct, CodeGG-like multi-entry bundle, and Egress-like archive layouts. Preserve enough fixture content to make differential review against the predecessor straightforward.

Where practical, record canonical expanded outputs in tests so migration is checked semantically rather than only by successful parsing.

### Migration provenance

Add `architecture/contract-v1-migration.md` or equivalent documenting the frozen Eggup source SHA, predecessor M001/M002 implementation SHAs, old/new crate paths, intentionally preserved semantics, intentionally deferred M003 validators, and the fact that Eggup cleanup is a later cross-repository operation.

### CI

Introduce ordinary project CI, not release CI. At minimum cover stable Linux fmt/check/clippy/test/doc, Rust 1.89 check/test, and practical macOS/Windows checks/tests for the portable crate. CI must not publish.

## 7. Ordered work packages

### Work package A — Workspace bootstrap

Create the minimal workspace, lockfile, licensing/project metadata, baseline lints, CI, and change log. Prove dependency resolution on stable and Rust 1.89 without unrelated heavy dependencies.

### Work package B — Faithful contract import

Move corrected schema-v1 behavior into `eggpack-contract`. Preserve all public semantics/errors unless a name is Eggup-specific only. Closure must contain a requirement-to-source mapping and identify any deliberate API-name-only changes.

### Work package C — Predecessor test/fixture parity

Import/recreate positive and negative tests, representative fixtures, and deterministic expansion/golden checks. Closure must map every M001/M002 predecessor invariant to Eggpack evidence.

### Work package D — Package/docs/platform qualification

Complete package metadata/readme/rustdoc, migration provenance, hosted CI, and dependency/package qualification.

## 8. Failure, cancellation, restart, and contention semantics

The contract crate is deterministic in-memory schema tooling.

On invalid input it returns a bounded typed error, returns no partially valid expanded target, does not rewrite source files, does not infer alternate targets/versions/names, and performs no network/process/filesystem mutation.

Cancellation, restart, and contention are not runtime concerns for this milestone.

Parser/validator inputs must remain bounded so malformed configuration cannot create unbounded diagnostics or allocation growth through declared fields.

## 9. Compatibility and migration

This is a repository/crate ownership migration, not a schema migration.

Valid corrected DistributionContract v1 input must remain valid with equivalent expansion. Input rejected at the corrected predecessor baseline must not become accepted accidentally.

The Rust crate name becomes `eggpack-contract`; no compatibility alias crate is required because `eggup-dist` is unpublished.

Do not edit Eggup from this plan. After Eggpack M001 closure, a separate Eggup plan may stop future producer-side work in `eggup-dist`, remove/deprecate the duplicate when downstream plans are reconciled, and redirect the old Eggup M003 validator work to Eggpack.

## 10. Required tests

Focused coverage must include schema version success/failure; unknown fields; opaque/filesystem-safe versions; target/alias collision classes; unknown target; missing alias input; direct/bundle/archive expansion; sidecar naming; malformed/unknown/context-invalid templates; repeated valid placeholders; release-file and install-name exact/ASCII-case collisions; asset/sidecar cross-collisions; archive absolute/traversal/backslash/drive/empty-path negatives; and deterministic serialization/round-trip where supported.

Fixture/integration coverage must include simple direct, CodeGG bundle, Egress archive, and deterministic fixture output.

Boundary evidence must show no HTTP/TLS/process/archive stack and no Eggup dependency in `eggpack-contract`.

## 11. Required verification commands

Run and record exact results:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked

cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --all-features --locked

cargo package -p eggpack-contract --locked --allow-dirty
cargo tree -p eggpack-contract --locked

git diff --check
```

If a local verification script is added, run it too and show that it wraps documented checks. Hosted CI results must be recorded separately rather than inferred from local commands.

## 12. Documentation updates

Required: root README as needed, CHANGELOG, `crates/eggpack-contract/README.md`, public rustdoc/examples, `architecture/contract-v1-migration.md`, contract subsystem roadmap, registry, and closure record.

## 13. Acceptance criteria

M001 closes only when Eggpack is a valid Rust 1.89+ workspace; `eggpack-contract` independently implements corrected DistributionContract v1; every predecessor M001/M002 invariant has explicit Eggpack evidence; representative direct/bundle/archive fixtures pass; schema-v1 meaning did not change; package/MSRV/dependency qualification passes; hosted cross-platform CI is green or a precisely scoped operational condition is recorded; no network/process/build/release dependency leaked into the contract crate; no medium-or-higher migration ambiguity remains; and Eggup's existing copy remains untouched.

## 14. Stop conditions

Stop and report rather than improvise if the frozen Eggup source cannot be recovered or differs materially from recorded closure evidence; preserving corrected v1 behavior requires a schema change; a desired Eggpack feature would require adding producer build/runner fields to DistributionContract v1; license/attribution provenance is ambiguous; the import unexpectedly pulls a network/process/native dependency; or a predecessor security/correctness defect is discovered that makes faithful import unsafe.

If a predecessor defect is discovered, write a corrective/schema plan rather than silently changing the migration.

## 15. Closure evidence required

Record the Eggpack implementation SHA; frozen Eggup source SHA and predecessor implementation SHAs; source-to-destination module mapping; public API/type inventory; predecessor-invariant-to-test matrix; direct/bundle/archive fixture outputs; strict template/path/collision negative-test matrix; dependency tree; package contents/result; stable and Rust 1.89 command results; hosted Linux/macOS/Windows evidence; proof no Eggup dependency exists; proof no Eggup file was removed/modified by this plan; unresolved findings/severity; and whether M002 validators are unblocked.

## 16. Handoff notes

Do not implement the predecessor M003 validator plan in the same pass. M001 is a migration/evidence milestone so failures can be attributed cleanly.

Do not delete `eggup-dist`; cross-repository cleanup is deliberately deferred until this milestone has accepted closure evidence.

Preserve user changes if the repository acquires unrelated work after this planning baseline.
