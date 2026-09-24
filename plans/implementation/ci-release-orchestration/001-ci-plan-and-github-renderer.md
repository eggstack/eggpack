# CI and Release Orchestration Milestone 001 — Provider-Neutral CIPlan and Deterministic GitHub Renderer

Status: ready for implementation — Build/Qualification M002 is closed

Repository baseline: 1666df9ac68062f7a1be4ed757a4f1ccc21aad5b

Source roadmap: plans/subsystems/ci-release-orchestration-roadmap.md

Related implementation interface:

- plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md

Required ADRs:

- plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md
- plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md

Long-term references:

- plans/000-long-term-specification.md#14-ci-and-workflow-generation
- plans/002-long-term-roadmap.md#phase-7--checked-in-ci-generation
- plans/001-terminology-and-domain-model.md#14-ci-plan-and-workflow

Primary class: infrastructure / deterministic generation / security-sensitive CI

## 1. Objective

Implement the first provider-neutral CIPlan plus a deterministic GitHub Actions renderer for checked-in release workflows.

M001 consumes ReleasePlan plus the build-source/command interface established by Build M002 and produces a reviewable workflow graph. It owns CI projection and rendering only.

It does not stage or publish releases, execute qualification itself, create final manifests, or become a generic GitHub Actions DSL.

## 2. Readiness and sequencing

The ReleasePlan interface is closed and stable enough for CI planning.

Build M002 is separately registered and defines the exact producer build-source binding and command-spec seam needed to avoid duplicating Cargo package/bin or cargo-zigbuild rendering inside the CI subsystem.

Therefore:

- CIPlan graph/model work may begin immediately against ReleasePlan;
- GitHub build-step rendering MUST consume/reuse M002 build command/binding representation after those types land;
- do not create a parallel CI-only source-binding schema;
- do not duplicate native/cross command construction.

This is an interface dependency, not a reason to re-open ReleasePlan.

## 3. Current evidence

Eggstack release workflows repeatedly encode target matrices, runner labels, Rust toolchain setup, cargo/cargo-zigbuild intent, qualification gates, artifact handoff, aggregation, permissions, and release-event conditions.

ADR-0003 already selects checked-in generated workflows with explicit publication gates.

## 4. Invariants

- CIPlan is provider-neutral.
- GitHub runner labels/action syntax live only in GitHub renderer policy.
- ReleasePlan remains the source of target/build/qualification/support intent.
- Build M002 remains the source of logical Cargo binding/command intent.
- DistributionContract remains the source of artifact identity.
- Generated output is deterministic for identical inputs/policy.
- Generated workflow is checked-in and reviewable.
- No remote reusable workflow is hidden policy authority.
- Build/preflight jobs default to read-only permissions.
- No write permission appears in M001.
- Public publication does not exist in M001.
- Action references must be full immutable commit SHAs in production policy.
- Generated workflow does not install arbitrary unpinned tools via curl/sh or equivalent.
- Required versus non-gating targets are explicit.
- No arbitrary user-provided workflow YAML or step DSL.
- M001 should require no secrets.

## 5. Scope

### In scope

- new isolated crates/eggpack-ci package;
- provider-neutral CIPlan domain;
- deterministic ReleasePlan -> CIPlan projection;
- explicit runner capability model;
- GitHub rendering policy;
- deterministic GitHub Actions YAML;
- preflight/build graph for native/cross strategies;
- support-tier gating representation;
- qualification intent represented as graph metadata/gates, never false proof;
- artifact handoff slots sufficient for later aggregation;
- immutable action-pin policy input;
- golden workflow fixtures;
- syntax/static safety validation;
- render/check library API suitable for later CLI;
- docs/MSRV/package/hosted CI.

### Out of scope

- CLI commands if no CLI crate yet exists;
- actual qualification execution logic;
- aggregate ManifestBuilder job implementation;
- release upload/staging;
- write permissions;
- GitHub Release API;
- registry publication;
- arbitrary Actions expressions supplied by users;
- remote reusable workflow authority;
- non-GitHub provider renderers.

## 6. Required production model

### A. eggpack-ci crate

Add crates/eggpack-ci as an isolated generator package depending on eggpack-core and only minimal serialization/rendering support.

No GitHub HTTP client, async runtime, process runner, release API client, or Eggup dependency.

Forbid unsafe code and deny missing docs.

### B. CIPlan

Define a versioned provider-neutral graph equivalent in purpose to:

~~~text
CIPlan
  schema_version
  release_id/source_revision
  preflight
  target jobs[]
  required aggregation dependencies
~~~

Each target job contains:

- canonical target;
- build strategy/reference to M002 command intent;
- build host requirement;
- qualification classification and optional distinct qualification host;
- support tier;
- artifact form;
- required/non-gating state;
- logical output slots;
- artifact handoff identity needed by later aggregation.

CIPlan MUST NOT contain GitHub runner labels, action names/SHAs, YAML fragments, secrets, or publication operations.

Targets/jobs are canonically ordered.

### C. Graph semantics

Initial graph:

~~~text
preflight
   |
   +--> build[target A] ---+
   +--> build[target B] ---+--> future aggregate boundary
   +--> build[target C] ---+
~~~

Qualification classification is represented without claiming execution.

M001 must not emit a no-op qualification step that appears to qualify a target. If qualification execution is not yet available, the graph must retain it as unresolved intent and must not claim a release-complete workflow.

### D. GitHub renderer policy

Define a bounded GitHub policy object containing:

- HostRequirement -> runner label mapping;
- immutable action pins;
- finite workflow trigger policy;
- default timeouts;
- concurrency policy;
- artifact retention bound if internal artifact upload is rendered.

No arbitrary YAML strings.

Production action references MUST be full commit SHAs. Human-readable comments may include friendly action versions.

Runner labels are provider policy, never canonical target identity.

### E. Permissions

M001 generated workflows must set explicit permissions.

Default is contents: read with no id-token/write or contents/write.

If M001 requires write permission, stop and re-plan because staging/publication belongs later.

### F. Build steps

The renderer must reuse Build M002 source-binding and command-spec semantics.

It MUST NOT infer package/bin names from release filenames, independently implement cargo-zigbuild floor syntax, accept raw command strings from PackConfig, or hide tool installation behind unpinned actions.

### G. Artifact handoff

M001 may render per-target upload of internal candidate/build outputs for later jobs.

These are not final public release assets.

Handoff names must be deterministic, collision-safe, and based on canonical target/logical identity rather than pretending to be finalized DistributionContract asset names.

### H. Deterministic renderer

For identical CIPlan + GithubPolicy:

- bytes are identical;
- job/step order is stable;
- quoting/newline behavior is stable;
- no timestamps/random IDs appear.

Expose library operations equivalent to render_github(plan, policy) -> String and check_github(plan, policy, existing_bytes) -> DriftReport.

Check mode is library-only in M001; CLI wiring may land later.

## 7. Ordered work packages

1. Add eggpack-ci crate and dependency boundaries.
2. Define CIPlan graph/domain and validation.
3. Implement deterministic ReleasePlan -> CIPlan projection.
4. Define bounded GithubPolicy and validation.
5. Reuse/bridge Build M002 binding/command-spec types after available.
6. Implement deterministic GitHub renderer for preflight/build graph.
7. Implement read-only permissions, timeouts, concurrency, runner mapping, and immutable action pins.
8. Implement internal artifact handoff representation.
9. Add deterministic render/check library APIs.
10. Add golden direct multi-target workflows and negative policy/graph tests.
11. Add YAML parse/static safety validation.
12. Update docs/roadmap/registry and write closure.

## 8. Failure/restart/contention semantics

Generation is pure and performs no network/process/public mutation.

Invalid CIPlan or GithubPolicy returns typed failure and emits no successful workflow.

Check mode compares deterministic bytes after documented newline normalization only and does not rewrite files.

## 9. Compatibility

This creates an internal/provider-neutral CIPlan v1 and GitHub renderer contract. Do not claim it as a stable public 1.0 format.

Existing consumer workflows remain authoritative until adoption closures prove parity.

No DistributionContract or ReleaseManifest change. No publication authority.

## 10. Required tests

At minimum:

- deterministic target/job ordering;
- equal inputs -> byte-identical YAML;
- aliases never reintroduced after ReleasePlan canonicalization;
- NativeCargo target projection;
- CargoZigbuild target projection;
- required/non-gating/experimental semantics;
- build/qualification host distinction retained;
- direct/bundle/archive form retained as metadata;
- unresolved qualification cannot be represented as passed;
- unknown/duplicate runner mapping rejects;
- mutable action refs such as @v4, @master, branches, tags rejected by production policy;
- malformed/non-SHA action pin rejected;
- explicit read-only permissions present;
- no write/id-token permissions;
- deterministic timeout/concurrency settings;
- no raw secret values;
- no arbitrary YAML/command field;
- internal handoff names collision-safe;
- drift helper detects a one-byte/manual edit;
- independent YAML parser accepts rendered fixtures;
- no release publication operation;
- Eggsact-like direct target fixture;
- mixed native/cross plan fixture;
- unchanged core/contract/manifest/bootstrap suites.

## 11. Verification commands

~~~bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-ci --locked
cargo package -p eggpack-ci --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
./scripts/check-local.sh
git diff --check
~~~

Also parse generated workflow fixtures with an independent YAML parser/tool in the test harness. Network-based GitHub validation is not required for closure.

Record Linux stable, Linux Rust 1.89, macOS, and Windows hosted CI.

## 12. Documentation updates

Update root README, crates/eggpack-ci/README.md, CI roadmap, registry, and plans/closure/ci-release-orchestration/001-status.md.

Document the distinction between CIPlan graph intent, generated GitHub representation, candidate handoff, later qualification/aggregation, and later staging/publication.

## 13. Acceptance criteria

M001 closes when:

- CIPlan is provider-neutral and deterministic;
- GitHub renderer is isolated/deterministic;
- runner/action policy is explicit and bounded;
- action references are immutable SHAs;
- workflow permissions are read-only;
- build/source semantics reuse M002 instead of duplicating them;
- generated output is parseable/reviewable;
- drift helper detects edits;
- no workflow claims unimplemented qualification/publication;
- stable/MSRV/package/docs/hosted CI pass;
- no unresolved medium-or-higher CI generation/security finding remains.

Closure must state whether CI M002 qualification/aggregation/drift-CLI work is ready to plan.

## 14. Stop conditions

Stop/re-plan if:

- GitHub renderer needs a generic user-authored step/YAML language;
- build command/source identity must be duplicated rather than reused from M002;
- generated M001 workflow needs write permissions;
- qualification must be faked/no-op to render a graph;
- a remote mutable reusable workflow must become policy authority;
- deterministic rendering cannot be guaranteed;
- staging/publication semantics become necessary.

## 15. Closure evidence required

Record implementation SHA, M002 interface baseline consumed, CIPlan type/schema inventory, GithubPolicy pin/runner inventory, golden workflow hashes, deterministic regeneration evidence, permissions/action-pin audit, drift detection, YAML parse evidence, dependency tree, package/MSRV/docs/hosted CI, unresolved findings, and CI M002 readiness disposition.
