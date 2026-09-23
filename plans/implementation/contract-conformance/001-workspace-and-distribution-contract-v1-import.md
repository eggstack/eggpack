# Contract and Conformance Milestone 001 — Workspace Bootstrap and DistributionContract v1 Import

Status: closed

Repository baseline: `23a7f312589327088482a0158be611b472c80dc9`

External predecessor baseline:

- repository: `eggstack/eggup`
- reviewed main: `cf5b3d3819c168eb2dbf841daa8332f3eb28c915`
- predecessor M001 implementation: `889a234cbe7f461d92def3df45c83c06a7d257e5`
- predecessor M002 corrective: `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`
- `eggup-dist/src/lib.rs` blob: `5a8d7bc7c3bd134b691f8e1c7b2f6de640a55477`
- `eggup-dist/Cargo.toml` blob: `41d153536e724b379f4b78a5072ffe412fe240b8`
- fixture harness blob: `4ef2e0704603ba00128f2a715e8b35d5422a41fa`
- fixture blobs:
  - simple direct: `499c2192bd2eebdb543df38021b3586ec69f6fe4`
  - CodeGG bundle: `6f54543e49488d5b6363f28b52f13dc819f3d52c`
  - Egress archive: `e1106397459bfe8ade40922f6d8f74287e372a7e`

Source roadmap:

- `plans/subsystems/contract-conformance-roadmap.md`

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

Bootstrap Eggpack as a Rust workspace and establish `eggpack-contract` by faithfully importing the corrected, unpublished `eggup-dist` DistributionContract schema-v1 implementation and its qualification evidence.

This milestone is a migration and proof-continuity milestone, not an opportunity to redesign the schema or begin the build engine.

At closure, Eggpack must independently provide the same valid schema-v1 interpretation, failure behavior, fixture expansion, template grammar, collision guarantees, archive-member safety, and runtime-independence properties proven by Eggup distribution M001/M002.

## 2. Why this milestone is ready

Hard dependencies are satisfied:

- Eggpack canonical specification, terminology, roadmap, and planning process exist.
- ADR-0001 fixes producer/consumer ownership.
- ADR-0002 fixes the contract/plan/manifest separation.
- Eggup predecessor M001 and M002 are closed with exact implementation SHAs and test evidence.
- `eggup-dist` is unpublished, so moving ownership does not require a crates.io compatibility migration.
- The Eggpack repository contains no production code that could conflict with the imported contract.

Interface dependency:

- Eggup retains its copy until this milestone closes; therefore migration can be staged without leaving either repository without the qualified contract.

## 3. Current implementation evidence

At the predecessor baseline, `eggup-dist` is a tooling-only crate with:

- explicit `schema_version = 1`;
- strict unknown-field rejection;
- bounded opaque ProductId/version inputs;
- canonical target triples plus globally unambiguous aliases;
- three fixed asset forms: direct, bundle, archive;
- small template vocabulary `{product}`, `{version}`, `{target}`, `{alias}`, and checksum-context `{asset}`;
- parse-time strict brace/placeholder grammar;
- release/install namespace collision detection including ASCII-case-only collisions;
- literal, traversal-free archive member sources;
- deterministic parse/expand behavior;
- representative direct/bundle/archive fixtures;
- only `serde` and narrowly configured `toml` as dependencies;
- no network, process execution, extraction, installer generation, release selection, or service behavior.

Predecessor closure reported stable and Rust 1.89 success, package qualification, documentation success, and no runtime Eggup crate dependency on `eggup-dist`.

Eggpack currently has planning documents only.

## 4. Invariants that must not regress

- Valid predecessor schema-v1 documents retain the same meaning.
- Unknown schema majors fail typed.
- Unknown fields fail.
- Unsupported targets fail; no nearest-target or host guessing.
- Target aliases remain globally unambiguous.
- Templates are data substitution only; no shell, expressions, environment, conditionals, or generic formatting language.
- Malformed braces/placeholders fail before a partially valid expansion escapes.
- Expanded release filenames are flat and unique under exact and ASCII-case comparison.
- Expanded install names are flat and unique under exact and ASCII-case comparison.
- Asset/checksum cross-collisions fail.
- Archive member sources remain literal, relative, normalized, forward-slash paths with no absolute path, drive prefix, backslash, empty component, `.`, or `..`.
- Direct/bundle/archive remain the only schema-v1 artifact forms.
- Product versions remain opaque except for existing filesystem-safety validation.
- Checksum sidecars are integrity metadata, not authenticity.
- The contract crate performs no network I/O, subprocess execution, archive extraction, release publication, live install/update, or service management.
- Migration does not remove or mutate Eggup's predecessor implementation.
- `eggpack-contract` remains usable without `eggpack-core`, CLI, GitHub, archive, or build dependencies.

## 5. Scope

### In scope

- root Rust workspace bootstrap;
- Rust 1.89 MSRV declaration;
- workspace lint policy with unsafe code denied;
- `crates/eggpack-contract`;
- rename/update crate metadata from `eggup-dist` to Eggpack ownership;
- faithful source import;
- faithful unit/integration fixture import;
- predecessor provenance documentation;
- local verification script sufficient for the initial workspace;
- ordinary CI for contract qualification;
- package boundary qualification;
- documentation/rustdoc;
- comparison evidence proving semantic fidelity.

### Explicitly out of scope

- conformance validator M002;
- ReleaseManifest;
- PackConfig/ReleasePlan;
- build/cargo-zigbuild execution;
- archive extraction/creation;
- installer generation;
- generated release CI;
- GitHub Release APIs;
- release publication;
- Eggup-side removal/deprecation;
- Eggup adapter;
- signing/provenance;
- schema-v2 additions;
- "cleanup" that changes predecessor public semantics.

## 6. Required production changes

### A. Root workspace

Create the minimum repository foundation needed to build and qualify the contract crate.

Expected root policy:

- Cargo workspace resolver 2;
- edition 2021 initially for predecessor fidelity;
- `rust-version = "1.89"`;
- MIT package metadata inherited from the predecessor unless repository maintainers have already established a different license before implementation;
- repository metadata points to `https://github.com/eggstack/eggpack`;
- workspace unsafe-code deny;
- proportionate Clippy defaults.

Do not introduce future planned crates as empty placeholders merely to imply capability.

### B. `eggpack-contract` crate

Move/copy predecessor functionality into:

```text
crates/eggpack-contract/
    Cargo.toml
    README.md
    src/lib.rs
    tests/fixtures.rs
    tests/fixtures/*.toml
```

The public Rust crate/package name becomes `eggpack-contract`.

Type names such as `DistributionContract` SHOULD remain stable unless a rename is purely namespace/documentation-level and can be proven not to alter serialized schema-v1.

Do not embed Eggup names in generic error/API semantics except historical provenance comments/docs.

### C. Dependency boundary

Keep the predecessor dependency philosophy:

- `serde` derive;
- `toml` parse/display only;
- no HTTP client;
- no async runtime;
- no archive library;
- no process runner;
- no Git/GitHub dependency;
- no Eggup runtime dependency.

If a dependency beyond the predecessor graph is proposed, stop and justify it in closure rather than adding it casually.

### D. Fixture fidelity

Import all three representative fixtures byte-for-byte initially if possible:

- `simple-direct.toml`;
- `codegg-bundle.toml`;
- `egress-archive.toml`.

If repository/name comments require editing, preserve all parsed semantics and record the exact diff.

Add a migration/provenance test or golden evidence showing the Eggpack version expands these fixtures to the same canonical structures/filenames as the predecessor.

### E. Regression corpus from predecessor M002

Preserve tests for:

- malformed unmatched/stray/empty/nested/double braces;
- unknown/context-invalid placeholders;
- repeated valid placeholders;
- duplicate bundle/archive declarations;
- post-expansion collisions;
- sidecar/asset cross-collisions;
- ASCII case-only collisions;
- archive traversal/absolute/backslash/drive/empty-component paths;
- target/alias collisions;
- missing alias context;
- deterministic serialization/round-trip.

The implementation agent may reorganize the 55 KiB predecessor `lib.rs` into coherent modules if and only if fidelity remains mechanically demonstrated. A straight import followed by later refactor is preferred over simultaneous architecture churn.

### F. Repository verification surface

Add an initial `scripts/check-local.sh` or equivalent that runs the narrow contract checks and workspace formatting/lint/docs/package checks.

Add ordinary GitHub Actions CI sufficient to exercise:

- stable Rust on Linux;
- MSRV 1.89 on Linux;
- at least check/test on macOS and Windows because path/case assumptions are security-relevant even though the crate is pure.

Do not add release/publish workflows in this milestone.

### G. Documentation

The crate README must clearly state:

- role as portable distribution contract;
- relation to Eggpack and Eggup;
- schema-v1 example;
- asset forms;
- template grammar;
- collision rules;
- archive path rules;
- integrity-vs-authenticity distinction;
- explicit non-goals;
- migration provenance from `eggup-dist`.

Root README may be updated from planning-only status to note the first implemented crate once closure succeeds.

## 7. Ordered work packages

### Work package A — Workspace bootstrap

Intent: establish a minimal, correctly constrained Rust project.

Required changes:

- root Cargo workspace;
- license/metadata as appropriate;
- lint policy;
- `eggpack-contract` member;
- local verification script;
- basic CI.

Acceptance evidence:

- Cargo metadata resolves;
- no unexpected workspace members;
- Rust 1.89 can parse/build workspace.

### Work package B — Faithful contract import

Intent: establish Eggpack as the owner of the proven v1 implementation.

Required changes:

- import predecessor source;
- rename crate/package/docs;
- preserve schema names/serialization;
- preserve `#![forbid(unsafe_code)]` or equally strict workspace policy.

Acceptance evidence:

- predecessor positive fixtures parse/expand identically;
- public contract semantics documented.

### Work package C — Regression and fixture import

Intent: preserve M001/M002 proof continuity.

Required changes:

- migrate fixture harness and all important negative tests;
- add explicit migration provenance/golden comparison where practical.

Acceptance evidence:

- every predecessor invariant listed in sections 3-4 has a named test/evidence row.

### Work package D — Package/MSRV/dependency qualification

Intent: prove the leaf crate is suitable for later independent reuse.

Acceptance evidence:

- stable suite green;
- Rust 1.89 suite green;
- `cargo package` green;
- rustdoc green;
- dependency tree remains proportionate;
- no network/process/archive/build dependencies.

### Work package E — Closure and planning transition

Write:

- `plans/closure/contract-conformance/001-status.md`;
- roadmap status M001 -> closed if evidence supports it;
- registry update;
- M002 validator plan only after closure review.

Do not remove `eggup-dist` from Eggup in this work package.

## 8. Failure, cancellation, restart, and contention semantics

The contract library is pure and synchronous.

Invalid input returns typed errors and performs no mutation.

No cancellation/restart semantics apply to parsing/expansion.

CI/package scripts must fail on the first correctness/lint/package failure rather than silently continue.

Large/untrusted inputs must remain bounded according to predecessor limits.

Concurrent callers must not require global mutable state.

## 9. Compatibility and migration

This milestone creates a new unpublished Eggpack crate from an unpublished predecessor.

Valid schema-v1 compatibility is required; package name compatibility is not.

Eggup retains its predecessor copy during this milestone. After Eggpack M001 closes, a separate Eggup plan may:

1. freeze/remove the local `eggup-dist` workspace member;
2. depend on `eggpack-contract` only where tooling needs it; or
3. eliminate the dependency entirely in favor of manifest consumption later.

That choice is explicitly outside this plan.

## 10. Required tests

### Focused unit tests

All parser/template/name/path/target tests from predecessor M001/M002.

### Integration tests

- simple direct fixture;
- CodeGG bundle fixture;
- Egress archive fixture;
- deterministic round-trip/golden expansion.

### Compatibility tests

- explicit schema-v1 parse/serialize;
- predecessor fixture parity;
- unknown future schema major rejection.

### Security and negative tests

- all unsafe archive paths;
- control/separator inputs;
- overlong inputs;
- exact and ASCII-case collision matrices;
- no unexpected placeholder context.

### Dependency/static guards

- no `std::process::Command` in contract crate;
- no HTTP/async/archive dependency;
- no Eggup dependency;
- no unsafe code.

## 11. Required verification commands

At minimum:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-contract --locked
cargo package -p eggpack-contract --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-contract --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted CI results must be recorded separately from local commands.

## 12. Documentation updates

- root `README.md`;
- `crates/eggpack-contract/README.md`;
- crate rustdoc;
- `CHANGELOG.md` if introduced by workspace bootstrap;
- source roadmap;
- registry;
- closure record.

## 13. Acceptance criteria

M001 closes only when:

- Eggpack builds as a Rust 1.89-compatible workspace;
- `eggpack-contract` exists as the sole implemented portable contract in this repo;
- predecessor schema-v1 positive behavior is preserved;
- predecessor M002 strict grammar/collision behavior is preserved;
- all three representative layout fixtures pass;
- no build/network/extraction/runtime updater responsibilities entered the crate;
- package and docs qualification pass;
- no medium-or-higher migration ambiguity remains;
- Eggup's predecessor remains intact pending a separate cleanup plan.

## 14. Stop conditions

Stop and write a corrective/ADR rather than improvise if:

- a valid predecessor fixture cannot be represented without changing v1 semantics;
- imported tests reveal predecessor behavior inconsistent with its closure records;
- preserving behavior requires a new heavy dependency;
- repository licensing conflicts with predecessor MIT metadata;
- an implementation change would merge PackConfig/build policy into DistributionContract;
- migration would require changing Eggup in the same commit;
- schema-v1 must change to support future Eggpack features.

## 15. Closure evidence required

Record:

- Eggpack implementation SHA;
- exact Eggup predecessor SHAs/blobs used;
- source/fixture provenance;
- public API inventory;
- predecessor invariant-to-test matrix;
- direct/bundle/archive expansion results;
- template/path/collision negative-test matrix;
- dependency tree;
- package contents/result;
- stable and MSRV commands/results;
- hosted Linux/macOS/Windows CI;
- documentation/rustdoc result;
- unresolved findings;
- confirmation that Eggup was not modified/removed.

## 16. Handoff notes

Fidelity beats cleanup in this milestone.

The predecessor source is concentrated in one large `lib.rs`; an implementation agent may split modules only after first establishing a mechanically equivalent baseline or while preserving a clear diff/evidence path.

Do not implement the currently-ready Eggup distribution M003 validator plan inside Eggup. After this milestone closes, author its Eggpack successor as Contract/Conformance M002.
