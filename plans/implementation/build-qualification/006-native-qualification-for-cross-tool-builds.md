# Build and Qualification Milestone 006 — Native Qualification for Cross-Tool Builds

Status: proposed / not started (blocked on the ADR-0005 decision)

Repository baseline: `28f3630413c1fa6ae35ca1fdfc404a64b30b3b88`

Source roadmaps:

- `plans/subsystems/build-qualification-roadmap.md`
- `plans/subsystems/ci-release-orchestration-roadmap.md`
- `plans/subsystems/ecosystem-adoption-roadmap.md`

Blocking decision:

- `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md` (proposed; Option A recommended)

Blocked milestone:

- `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md` (hit §20 stop condition)

First proving consumer shape:

- `eggstack/eggsact` five-target direct release matrix: two Linux targets built with `CargoZigbuild` for a glibc 2.17 floor and natively qualified on matching native runners, three `NativeCargo` targets natively qualified.

Related decisions:

- ADR-0004 first-party native Cargo/cargo-zigbuild adapter;
- ADR-0003 checked-in generated CI and explicit publication gate.

Primary class: capability / producer-contract correction / reusable capability

## 1. Why this milestone exists

Ecosystem M001 stopped at its §20 condition *"generated workflow cannot
express all five current targets/runners"*.

Eggpack couples two independent producer axes — `BuildStrategy` (how a
candidate is produced) and `Qualification` (how it is proved) — by rejecting
`Qualification::Native` for any `CargoZigbuild` target. Two declaration
validators enforce the coupling:

- `crates/eggpack-core/src/lib.rs` — `validate_policy`, reached via
  `PackConfig::resolve`, rejects `policy.strategy == BuildStrategy::CargoZigbuild`
  when `qualification == Qualification::Native`;
- `crates/eggpack-ci/src/lib.rs` — `CIPlan::validate`, which requires
  `policy.strategy == BuildStrategy::NativeCargo` before checking that the
  qualification host matches the target triple.

The execution path already supports the combination.
`eggpack_core::qualification` selects its method from `Qualification` and the
observed host, never from `policy.strategy`, and its `Qualification::Native`
branch already refuses to execute unless the host matches the target OS and
architecture.

The full diagnosis, options, and decision request are in ADR-0005. This plan
describes the implementation **if and only if** ADR-0005 is accepted with
Option A.

## 2. Preconditions

Do not start until:

1. ADR-0005 is `accepted` with Option A;
2. ADR-0005 Option B or C was not selected — those require no producer code
   change and instead record a declared-intent compromise in the Ecosystem
   M001 closure;
3. the reviewed baseline above is still the parent of the working tree.

## 3. Objective

Allow a producer to declare `Qualification::Native` for a target built with
cross tools, whenever the effective qualification host matches the target's
operating system and architecture.

## 4. Invariants

- qualification must never execute on a host that does not match the target
  OS/arch, at declaration or at run time;
- every executing classification must still require a bounded smoke binding;
- a non-matching host must still produce a failed `HostMismatch` evidence
  record rather than a pass or a skip;
- `NativeCargo` targets must still reject any cross-tool version;
- glibc/macOS floor applicability rules must not change;
- no document that resolves today may change meaning, so no schema version
  bump and no historical closure rewrite;
- the change must stay a general producer capability, not a per-consumer
  special case.

## 5. In scope

- relax the `BuildStrategy` term from both `Qualification::Native`
  declaration validators;
- preserve and strengthen the host-matching and smoke-required guards;
- regression tests covering acceptance and every rejection path in §8;
- a reusable-workflow render/check test using an eggsact-shaped five-target
  configuration with two CargoZigbuild + native targets;
- documentation updates naming the corrected rule;
- closure record with hosted CI evidence.

## 6. Out of scope

- Option D (`NativeCrossBuilt` classification) and any `Qualification` or
  `QualificationMethod` enum growth;
- the live GitHub draft qualification, which belongs to Ecosystem M001;
- any eggsact repository change, including its `eggpack_tool.revision` pin,
  which Ecosystem M001 re-points after this milestone closes;
- adding unsupported host emulation to reduce runner counts;
- generalizing build/CI configuration into a DSL.

## 7. Required production changes

### 7.1 `eggpack-core` producer policy validation

In `validate_policy` (`crates/eggpack-core/src/lib.rs`), keep the host check
and drop only the strategy term, so a `Qualification::Native` policy is
admissible exactly when its effective qualification host matches the target
triple:

- retain `host_matches_target(host, triple)`;
- retain the `CompatibilityFloor` applicability rules unchanged;
- retain the `NativeCargo` rejection of any `cargo_zigbuild`/`zig` toolchain
  version, which is enforced separately and must stay.

The error text must be updated to describe the actual requirement, since it
currently attributes the failure to a "native build/qualification host".

### 7.2 `eggpack-ci` graph validation

In `CIPlan::validate` (`crates/eggpack-ci/src/lib.rs`), compute
`native_qualification_valid` from the host match alone, keeping the explicit
`qualification_host` override and the `host_os`/`host_arch` default. All
surrounding guards — canonical ordering, toolchain-version grammar per
strategy, floor applicability, and the `CargoZigbuild` + macOS-floor
rejection — remain unchanged.

### 7.3 Documentation

- `crates/eggpack-core/README.md` — state that native qualification requires a
  matching native host and does not depend on build strategy;
- `architecture/core.md` — record the corrected producer rule alongside
  `BuildStrategy`.

## 8. Tests

Acceptance:

- a `CargoZigbuild` + `Qualification::Native` PackConfig with a matching host
  resolves, and the resulting `ReleasePlan` keeps the glibc floor and exact
  cross-tool versions;
- a five-target eggsact-shaped `ReleaseWorkflowShapeV1` with two
  CargoZigbuild + native targets and three `NativeCargo` + native targets
  renders a reusable release workflow and passes `eggpack ci check` with zero
  drift;
- `qualify_target` on a CargoZigbuild-built candidate at a matching native host
  yields `QualificationMethod::Native`, `QualificationStatus::Passed`, and
  executed process outcomes.

Rejection (each must still fail):

- `CargoZigbuild` + `Qualification::Native` with a mismatched qualification
  host architecture;
- `CargoZigbuild` + `Qualification::Native` with a mismatched host OS;
- `Qualification::Native` without a smoke binding;
- `NativeCargo` with `cargo_zigbuild` and/or `zig` set;
- glibc floor on a non-GNU/Linux target and macOS floor on a non-Darwin
  target;
- `CargoZigbuild` combined with a macOS floor;
- `Structural` with a smoke binding.

Invariants:

- every pre-existing `NativeCargo` and cross-tool test stays green, including
  the `crates/eggpack-core/src/lib.rs` test that asserts cross-tool versions
  are rejected on `NativeCargo`;
- golden workflows for M001/M002/M002a/M003/M003a/M003b/M003c/M003d render
  byte-identically, since no existing document changes meaning.

## 9. Verification commands

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
```

Plus hosted CI green on every lane, with the run id recorded in the closure.

## 10. Acceptance criteria

- ADR-0005 accepted with Option A;
- both declaration validators admit `Native` + `CargoZigbuild` only with a
  matching qualification host;
- every §8 rejection path still fails closed;
- an eggsact-shaped five-target shape renders and passes `ci check` with zero
  drift;
- `qualify_target` executes and records `QualificationMethod::Native` for a
  cross-tool-built candidate on a matching native host;
- existing goldens and cross-tool rules unchanged;
- no schema version bump and no historical closure rewritten;
- full local verification and hosted CI green;
- closure record written to `plans/closure/build-qualification/006-status.md`.

## 11. Stop conditions

Stop and re-plan if:

- relaxing the validators permits a plan whose qualification host does not
  match its target, or a passing evidence record without an executed smoke;
- any historical golden or closed-milestone test changes meaning;
- hosted CI is not green on all lanes;
- the change appears to require a `Qualification` enum addition, which is
  Option D and belongs in its own ADR.

## 12. Closure evidence

Create `plans/closure/build-qualification/006-status.md` recording:

- ADR-0005 acceptance reference;
- reviewed baseline and implementation commit;
- before/after admissibility of the four qualification/strategy combinations;
- the full §8 acceptance and rejection test matrix with results;
- the eggsact-shaped render and zero-drift `ci check` evidence;
- `qualify_target` evidence record for a cross-tool-built candidate;
- confirmation that no golden, schema version, or historical closure changed;
- exact verification run, including hosted run id and per-lane results;
- unresolved findings by severity;
- disposition for Ecosystem M001 and the eggsact tool pin re-point.
