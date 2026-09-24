# Build and Qualification Milestone 001 — PackConfig v1 and Deterministic ReleasePlan

Status: ready for handoff

Repository implementation baseline: `0b1c3b9795280dea610c2fbe9d1533591d610b3f` (Contract M001/M002 and Manifest M001/M001a closed; no producer build-plan implementation exists)

Planning-batch dependency note: `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` establishes the initial `eggpack-core` package in this batch. This milestone is semantically dependency-ready now, but production edits SHOULD follow that core bootstrap (or be applied in the same coordinated branch after its crate skeleton) to avoid two plans independently creating `eggpack-core`.

Source roadmap: `plans/subsystems/build-qualification-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#5.3-eggpack-core`
- `plans/000-long-term-specification.md#8-release-planning`
- `plans/000-long-term-specification.md#9-build-and-target-model`
- `plans/000-long-term-specification.md#10-qualification-model`
- `plans/001-terminology-and-domain-model.md#5-producer-configuration`
- `plans/001-terminology-and-domain-model.md#7-release-plan`

Related ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: invariant / infrastructure

## 1. Objective

Define the first versioned producer-only `PackConfig` and deterministic `ReleasePlan` in `eggpack-core`.

M001 must represent current Eggstack native-binary build and qualification intent without executing a build, selecting a durable external backend, redefining DistributionContract artifact identity, or becoming a generic workflow language. Given the same contract, config, release/source identities, and selected targets, planning must produce the same canonical plan.

## 2. Readiness and dependencies

Hard/interface dependencies are satisfied:

- DistributionContract M001/M002 is closed and is the sole target/artifact naming authority.
- ReleaseManifest M001/M001a is closed and fixes the downstream evidence boundary that build/finalization must eventually satisfy.
- External backend evaluation M001 is closed with disposition C: `dist` is design prior art only and is not an authorized production backend.

This plan does not depend on Manifest M002 semantics. The operational sequencing note above exists only because both milestones use the canonical `eggpack-core` crate.

## 3. Current evidence

The current Eggstack release estate requires a producer plan capable of representing:

- native Cargo builds;
- cargo-zigbuild cross builds and glibc compatibility floors;
- x86_64/aarch64 Linux;
- x86_64/aarch64 macOS;
- x86_64/aarch64 Windows where supported;
- ARMv7 where a consumer supports it;
- native, deferred-native, emulated/QEMU, and structural qualification classifications;
- required versus non-gating/experimental target support;
- current direct, bundle, and archive artifact forms.

No current Eggpack crate owns producer configuration or ReleasePlan types.

## 4. Invariants

- PackConfig is producer policy, not a portable consumer contract.
- DistributionContract remains the only authority for target aliases, artifact filenames, install names, checksum names, and direct/bundle/archive identity.
- PackConfig references target identities; it cannot redefine or override them.
- ReleasePlan is invocation intent, never evidence that a build/artifact/qualification exists.
- ReleaseId and SourceRevision are explicit caller inputs and remain opaque bounded strings.
- Unknown/duplicate targets and config entries fail closed; there is no nearest-target or host-target guessing.
- Target aliases may be accepted as selection input, but the resolved plan stores canonical target triples.
- Planning is deterministic and side-effect free.
- Planning performs no network I/O, subprocess execution, filesystem staging, Git discovery, publication, signing, or installation.
- Build success and qualification remain distinct concepts in every type/API.
- A cross build cannot be represented as native qualification merely because compilation occurs.
- No `dist`/cargo-dist type, GitHub Actions syntax, service-manager type, Eggup type, or application-specific release type enters the canonical plan model.
- Configuration remains intentionally finite/enumerated; arbitrary shell/workflow DSL fields are prohibited.

## 5. Scope

### In scope

- PackConfig schema v1;
- strict parser/validator, initially TOML unless existing crate constraints show a stronger reason otherwise;
- target-scoped producer policy;
- bounded build strategy representation;
- provider-neutral build-host/runner requirements;
- toolchain requirements and compatibility/deployment floors;
- qualification classification;
- support/gating classification;
- deterministic ReleasePlan resolution from contract + config + invocation;
- deterministic structured serialization suitable for inspection/golden tests;
- direct/bundle/archive plan fixtures derived from existing contracts;
- diagnostics for unsupported/missing/mismatched policy.

### Out of scope

- invoking Cargo/cargo-zigbuild;
- selecting or implementing a `dist` backend;
- product hook execution;
- QEMU/container execution;
- CIPlan/GitHub Actions rendering;
- filesystem staging/finalization;
- digest/manifest construction;
- Git/source-revision discovery;
- release selection/version ordering;
- publication;
- arbitrary environment or command DSL.

## 6. Required production changes

### A. PackConfig v1

Add producer configuration types under `eggpack-core`, with strict unknown-field handling and explicit schema version.

A minimal target-policy model MUST be able to express, without artifact-name duplication:

- canonical target reference;
- build strategy;
- build host/runner requirement;
- toolchain requirement;
- compatibility/deployment floor when relevant;
- qualification mode;
- support/gating tier.

The exact Rust names may differ, but the semantic model should remain small and enumerated.

Initial build strategy enum SHOULD be limited to strategies already evidenced and not requiring a backend ADR, for example:

- native Cargo;
- cargo-zigbuild.

Do not add `dist`, generic shell command, or arbitrary adapter configuration to M001.

### B. Provider-neutral host/runner model

Represent host requirements without GitHub runner labels.

The model must distinguish OS and architecture at minimum. It may use bounded enums for known initial families plus a carefully bounded extension strategy only if required by existing target evidence.

A ReleasePlan may state that build host and qualification host differ.

### C. Compatibility/deployment floors

Represent the compatibility constraints needed by current native releases, including at least:

- no explicit floor;
- glibc major/minor floor;
- macOS deployment major/minor floor where declared.

Do not encode package-manager policy or infer floors from the current host.

### D. Qualification classification

Represent:

- native;
- deferred native;
- emulated;
- structural.

This milestone stores classification/requirements only. It does not execute checks or product hooks.

The plan must make it impossible to serialize a cross-build as native qualification merely by setting an unrelated boolean. Use typed variants/validated combinations.

### E. Support/gating policy

Represent whether a selected target is required to complete the release aggregation gate versus explicitly non-gating/experimental/deferred.

Keep this small. Avoid a general policy expression language.

### F. Deterministic ReleasePlan resolution

Provide a pure resolver equivalent in purpose to:

```text
DistributionContract
      +
PackConfig
      +
ReleaseId
      +
SourceRevision
      +
selected target triples/aliases
      |
      v
ReleasePlan
```

Resolution MUST:

1. validate PackConfig v1;
2. resolve selected triples/aliases through DistributionContract;
3. reject duplicate canonical selections;
4. require exactly one applicable producer policy for each selected canonical target;
5. reject PackConfig target entries that do not correspond to contract targets;
6. derive artifact form/expected release identity from contract expansion rather than config;
7. normalize targets into deterministic canonical order;
8. retain builder/runner/toolchain/floor/qualification/support intent;
9. produce a plan that contains no claim that artifacts or qualification evidence already exist.

### G. Structured plan serialization

Add deterministic human/machine-inspectable serialization, preferably compact/pretty JSON using a documented stable field order.

This output is an inspection/debugging contract for M001, not yet a stabilized public 1.0 wire format and not a signing object.

## 7. Ordered work packages

1. Extend the established `eggpack-core` crate with planning modules/types.
2. Freeze PackConfig v1 schema and strict validation.
3. Implement build-strategy, host requirement, floor, qualification, and support enums.
4. Implement pure target-policy resolution against DistributionContract.
5. Implement deterministic ReleasePlan and serialization.
6. Add direct/bundle/archive representative config+plan goldens.
7. Add negative validation/config/selection matrix.
8. Document explicit non-goals and external-backend disposition.
9. Qualify package/MSRV/docs/dependencies/hosted CI.
10. Write closure and update Build/Qualification + CI roadmaps/registry.

## 8. Failure, restart, and contention semantics

Planning is pure and synchronous. Invalid config or invocation returns a typed error before any external effect.

There are no retries, locks, staging directories, subprocesses, network requests, or public mutations.

Equal validated inputs MUST produce equal ReleasePlan values and deterministic serialized output.

## 9. Compatibility and migration

This creates PackConfig v1 and the first ReleasePlan model. It does not change DistributionContract or ReleaseManifest v1.

Because PackConfig is producer-only, future schema evolution may be handled independently from consumer manifest compatibility, but schema versioning starts at M001 and unknown major versions fail closed.

Do not encode a durable external build backend until a separate ADR authorizes one.

## 10. Required tests

At minimum:

- parse/round-trip strict PackConfig v1;
- unknown schema version and unknown field rejection;
- duplicate target policy rejection;
- unknown config target rejection;
- selected alias resolves to canonical target;
- duplicate alias+triple selection collapses to duplicate canonical target and fails;
- missing target policy fails;
- config cannot define asset/install/checksum names;
- deterministic plan independent of selected-target input order;
- direct Eggsact-like plan;
- CodeGG bundle plan;
- Egress archive plan;
- native Cargo + native qualification;
- cargo-zigbuild + deferred-native qualification;
- emulated and structural classification representation;
- glibc floor representation;
- build host and qualification host may differ;
- required/non-gating support tier behavior represented without execution;
- no `dist` backend accepted;
- ReleasePlan contains no artifact-exists/qualification-passed claims;
- unchanged contract/manifest/core manifest-builder suites;
- dependency/source scan for network/process/GitHub/Eggup coupling.

## 11. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Record Linux stable, Linux Rust 1.89, macOS stable, and Windows stable hosted CI.

## 12. Documentation updates

Update:

- `crates/eggpack-core/README.md` planning/configuration section;
- root README;
- Build/Qualification roadmap;
- CI/release-orchestration roadmap dependency state;
- registry;
- closure record `plans/closure/build-qualification/001-status.md`.

Include a small PackConfig + resolved ReleasePlan example demonstrating that filenames remain contract-derived.

## 13. Acceptance criteria

M001 closes when:

- PackConfig v1 is strict, bounded, versioned, and cannot redefine portable artifact identity;
- ReleasePlan resolution is deterministic and canonical-target based;
- current native Cargo/cargo-zigbuild, host/floor, and qualification classifications are representable;
- direct/bundle/archive identities are derived from contract expansion;
- no build/process/network/backend/publication behavior is implemented;
- no general workflow/command DSL is introduced;
- package/MSRV/docs/dependency and hosted CI qualification pass;
- no unresolved medium-or-higher planning/model defect remains.

Closure should explicitly state whether CI Orchestration M001 is now unblocked to plan against the ReleasePlan interface.

## 14. Stop conditions

Stop for ADR/replanning if:

- a durable external backend must be selected to define PackConfig;
- existing consumers require arbitrary command/workflow language rather than bounded strategies;
- artifact identity cannot remain exclusively contract-derived;
- ReleasePlan needs to claim actual build/qualification evidence;
- provider-specific CI syntax is required in the canonical model;
- a new ownership boundary with Eggup/application policy is required.

## 15. Closure evidence required

Record:

- exact implementation/review baselines;
- PackConfig v1 schema/API inventory;
- ReleasePlan field/authority matrix;
- direct/bundle/archive representative plan goldens;
- determinism evidence;
- negative config/selection cases;
- dependency tree and source scan;
- MSRV/package/docs/hosted CI;
- unresolved findings;
- whether CI Orchestration M001 and Build M002 become ready.

## 16. Handoff notes

This plan intentionally defines intent only. Build/Qualification M002 will later implement the native/cross builder seam; M003 will execute qualification; M004 will finalize/aggregate bytes.

Within the current planning batch, do not independently bootstrap a second `eggpack-core` package. Apply this plan after the Manifest M002 core skeleton exists, or coordinate both implementations in one branch with Manifest M002's crate bootstrap first.
