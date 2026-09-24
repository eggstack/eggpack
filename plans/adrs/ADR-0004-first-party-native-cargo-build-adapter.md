# ADR-0004: First-Party Native Cargo Build Adapter Boundary

Status: accepted

Date: 2026-09-24

Decision owners: project maintainers

Related specification sections:

- plans/000-long-term-specification.md#5.3-eggpack-core
- plans/000-long-term-specification.md#5.5-adapters
- plans/000-long-term-specification.md#8-release-planning
- plans/000-long-term-specification.md#9-build-and-target-model
- plans/000-long-term-specification.md#17-external-backend-policy

Affected subsystem roadmaps:

- build and qualification;
- CI/release orchestration;
- ecosystem adoption.

## Context

Build/Qualification M001 closed the provider-neutral PackConfig/ReleasePlan model. The bounded external-backend spike also closed with disposition C: dist/cargo-dist is useful design prior art and selected output normalization evidence, but it does not match Eggpack's canonical artifact identity, manifest, qualification, publication, or consumer-ownership boundaries closely enough to become the production authority.

The next build milestone needs a concrete execution seam. Eggpack currently represents two intentionally finite build strategies:

- NativeCargo;
- CargoZigbuild.

A durable implementation decision is therefore required before process execution is added. The decision must not turn Cargo command syntax, GitHub Actions, or any external backend's data model into canonical Eggpack identity.

## Decision drivers

- preserve DistributionContract as the sole release-artifact identity authority;
- preserve ReleasePlan as provider-neutral intent;
- support existing Eggstack direct Rust binaries without waiting for a general backend;
- avoid a general shell/workflow DSL;
- keep build execution replaceable behind a narrow adapter interface;
- make process security, timeouts, output bounds, and tool-version verification explicit;
- avoid importing dist/cargo-dist release/publication semantics.

## Considered options

### Option A — Adopt dist/cargo-dist as the production builder

Rejected for the initial native slice.

The completed capability spike found meaningful semantic mismatches around raw-direct versus archive outputs, bundle representation, final evidence, and publication/workflow authority. Adopting it here would either create a second identity authority or require Eggpack to reverse-engineer canonical state from a backend that already makes broader release decisions.

### Option B — First-party adapters for Cargo and cargo-zigbuild

Selected.

Eggpack implements a narrow execution adapter for the already-modeled NativeCargo and CargoZigbuild strategies. The adapter receives canonical ReleasePlan intent plus explicit producer build-source bindings and returns candidate-build evidence. It does not name final release files, qualify runtime behavior, assemble archives, emit manifests, or publish.

### Option C — Generic command/plugin backend first

Rejected.

An arbitrary command model would become a workflow/build DSL before real consumers justify it and would weaken validation, timeout, environment, and provenance guarantees.

## Decision

For the initial native-binary producer slice, Eggpack will use first-party process adapters for:

1. Cargo native target builds;
2. cargo-zigbuild cross-target builds.

The adapter boundary is an Eggpack-owned implementation seam, not a public release identity.

The initial execution path MUST:

- derive target strategy/toolchain/floor intent from ReleasePlan;
- receive explicit logical-output-to-Cargo package/bin bindings from producer-side adapter configuration;
- invoke binaries directly without a shell;
- require Cargo lockfile-respecting builds;
- verify configured toolchain/tool versions before claiming successful execution;
- use an owner-private target/work directory rather than relying on repository target state;
- produce CandidateArtifact/BuildAttempt evidence only;
- keep final release filenames, install names, checksums, archive assembly, manifests, qualification results, and publication outside the builder adapter.

For cargo-zigbuild GNU targets with a configured glibc floor, the adapter may use cargo-zigbuild's documented target-plus-glibc-version form. The exact command rendering is implementation detail and must be regression-tested; it is not stored as canonical release identity.

No production dist/cargo-dist dependency is authorized by this ADR.

No generic shell-command backend is authorized by this ADR.

A later backend may be added if it consumes the same Eggpack domain contracts and a new ADR justifies durable adoption.

## Consequences

### Positive

- unblocks a concrete native builder without surrendering Eggpack identity/qualification authority;
- directly serves simple Rust-native consumers;
- keeps external backend replacement possible;
- makes process execution auditable and bounded;
- avoids premature workflow DSL design.

### Negative

- Eggpack must own subprocess lifecycle, output bounds, tool preflight, and candidate-artifact discovery;
- cargo/cargo-zigbuild behavior needs cross-platform qualification;
- non-Cargo build systems remain unsupported until separately justified.

### Neutral or deferred

- qualification execution remains Build/Qualification M003;
- finalization/aggregation remains M004;
- generated CI may render these strategies but does not execute them inside core planning;
- source-binding configuration remains producer-only and may evolve before public API stabilization.

## Compatibility and migration

PackConfig/ReleasePlan remain provider-neutral and continue to derive release artifact form from DistributionContract.

Build-source bindings introduced by the builder milestone MUST reference logical contract slots rather than duplicate final artifact/install filenames.

Existing hand-written release workflows remain authoritative until Eggpack consumer-adoption milestones close.

## Security and reliability implications

The implementation must explicitly cover:

- no shell interpolation;
- bounded stdout/stderr capture;
- hard process deadlines and child cleanup;
- no implicit tool installation;
- exact toolchain/tool-version preflight;
- environment filtering/redaction;
- owner-private build directories;
- regular-file/symlink checks on discovered candidates;
- no trust claim from build success alone;
- no credentials in diagnostics.

If cross-platform descendant-process termination cannot be made reliable enough for closure, the builder milestone must stop and re-plan rather than weakening the timeout invariant.

## Verification

The builder milestone must prove:

- deterministic command-spec rendering from identical plan/binding inputs;
- NativeCargo and CargoZigbuild positive fixtures;
- glibc-floor rendering for GNU zigbuild targets;
- missing/mismatched tools fail before accepted candidate evidence;
- timeout/non-zero/oversized-output failures are bounded;
- exact package/bin discovery with no filename guessing;
- no final-release/manifests/publication authority in the adapter;
- stable/Rust 1.89/macOS/Windows qualification.

## Supersession

None.
