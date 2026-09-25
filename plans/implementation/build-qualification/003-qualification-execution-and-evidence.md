# Build and Qualification Milestone 003 — Qualification Execution and Evidence

Status: closed

Repository baseline: `25a6f185f865510be0ab26a51a819d949701674c`

Source roadmap: `plans/subsystems/build-qualification-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#8-release-planning`
- `plans/000-long-term-specification.md#9-build-and-target-model`
- `plans/000-long-term-specification.md#10-qualification-model`
- `plans/001-terminology-and-domain-model.md#8-build-and-qualification`

Applicable ADRs: `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`, `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`

Primary class: capability / qualification evidence

## 1. Objective

Turn M002 candidate outputs and ReleasePlan qualification intent into bounded, structured execution evidence. Represent native, deferred-native, emulated, and structural outcomes without treating build success or a planned run as qualification proof. Support explicit product-owned smoke hooks with bounded time/output and controlled environment.

This plan was added because the roadmap marked M003 ready to plan but the referenced implementation plan did not exist at repository baseline.

## 2. Readiness and dependencies

Hard dependencies are closed: Contract M001/M002; Manifest M001/M001a/M002; Build M001/M002/M002a; and ADR-0004. M002 candidate paths and process evidence are stable. Manifest M002 is also closed, satisfying one of M004's prerequisites; M004 remains gated on this milestone's closure.

## 3. Current evidence

`ReleasePlan` stores qualification intent and a host requirement. `BuildAttempt` returns validated candidate paths but contains no qualification result. `eggpack-core` already has a bounded process-group runner that permits only Cargo-related executable names; qualification requires a separate explicit runtime boundary.

## 4. Invariants

- A successful build is never qualification evidence.
- Native qualification requires the executing host OS/architecture to match the planned target and the candidate to be the exact validated candidate for that target.
- Deferred-native records pending execution and cannot pass a release gate.
- Emulated qualification records the declared emulator/runtime and does not claim native proof.
- Structural qualification records only checks actually performed and never implies execution.
- Hooks are caller-declared, shell-free, bounded in time and output, and receive a cleared/minimal environment; no arbitrary inherited environment dump or secret logging.
- Qualification failures produce typed evidence and no success result.
- Candidate bytes are read/executed only from explicit paths; symlinks and non-regular files fail closed.
- Qualification evidence is distinct from final artifact hashes and manifest evidence.

## 5. Scope

### In scope

- typed qualification plan, status, execution result, and bounded evidence model;
- native candidate execution checks and deferred-native status;
- explicitly configured emulator execution for supported targets, without emulator installation;
- structural candidate validation;
- product smoke hooks with explicit executable/arguments, environment allowlist/overrides, deadline, and output limits;
- process-group timeout, cancellation, and output-limit handling;
- unit/integration fixtures for each qualification class and negative boundaries;
- documentation, CI, and crate metadata updates.

### Out of scope

- building candidates (M002);
- final filenames, checksum sidecars, archive assembly, final hashing, manifest construction, or aggregation (M004);
- arbitrary shell/script/plugin/workflow DSL;
- downloading or installing emulators/tools;
- product-specific semantic parsing of hook output;
- signing, publication, or authenticity claims.

## 6. Required production changes

Extend `eggpack-core` with a qualification module and explicit APIs that accept a planned target, its build attempt/candidate set, and bounded qualification configuration. Use an enumerated execution kind and typed outcomes (passed, failed, deferred, structural-only, cancelled, timed out, output limit exceeded). A passed result must identify target, source/release identity, candidate selectors, qualification mode, and bounded process evidence. Do not serialize local paths, arbitrary environment values, or output contents.

Reuse the process-group cleanup primitive where practical, while keeping the executable allowlist separate from Cargo build commands. Verify candidate containment/regular-file status immediately before execution to reduce path replacement risks. For emulation and hooks, require explicit executable and argv; reject shell metacharacter interpretation by never invoking a shell. Bound config collections and argument/environment lengths.

## 7. Ordered work packages

1. Define serialized domain types, validation bounds, and identity binding to ReleasePlan/BuildAttempt.
2. Add safe candidate revalidation and native/deferred/structural qualification paths.
3. Add explicit bounded emulator and product-hook process execution with cancellation/timeout/output evidence.
4. Add negative and lifecycle tests, including wrong-host native claims, target mismatch, invalid paths, failed hooks, timeout, cancellation, and output overflow.
5. Document configuration, authority boundary, result interpretation, and failure modes.
6. Run formatting, lint, package/MSRV, workspace, and hosted cross-platform CI; record exact evidence and determine downstream readiness.

## 8. Failure, restart, and contention semantics

Qualification is fail-closed and returns no passing record after command failure, timeout, cancellation, output overflow, candidate mutation/replacement detection, or identity mismatch. Re-execution creates a new result; it does not mutate prior evidence. The caller owns candidate/work roots. Concurrent runs must not share mutable result state; process groups are always waited after termination.

## 9. Compatibility and migration

Additive producer-core API only. PackConfig v1, ReleasePlan intent, DistributionContract, Manifest v1, and existing builder wire/config schemas remain unchanged. Qualification records are producer evidence and do not alter Eggup consumer behavior.

## 10. Required tests

- successful matching-host native candidate and rejected mismatched host;
- build success without execution remains unqualified;
- deferred-native is pending, not passed;
- structural result claims no execution;
- explicit emulator/hook success and nonzero failure;
- timeout/cancellation kill and reap child process groups;
- stdout/stderr bounds and secret/output redaction;
- wrong target/plan/source identity and symlink/non-regular/replaced candidate rejection;
- deterministic bounded result serialization and invalid-config rejection.

## 11. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
git diff --check
```

Hosted CI must pass Linux stable, Linux Rust 1.89, macOS, and Windows, including the unskippable Windows builder qualification checks established by M002a.

## 12. Documentation updates

Update `eggpack-core` README/rustdoc and the Build/Qualification roadmap, registry, and closure record. State precisely which modes executed and which are deferred or structural; do not describe process exit alone as application correctness.

## 13. Acceptance criteria

- Each of the four qualification classes has typed, accurately scoped evidence.
- Native claims cannot be made for mismatched host/target or unrelated candidates.
- Hooks/emulators are explicit, bounded, shell-free, redacted, cancellable, and cleaned up.
- Regression tests cover claim inflation and process failure paths.
- Local and hosted required verification passes.
- Closure explicitly resolves M003 and evaluates M004 dependency readiness.

## 14. Stop conditions

Stop and revise the plan if implementation requires a public contract/manifest schema change, arbitrary executable/plugin language, unbounded environment inheritance, or unresolved medium-or-higher candidate execution/path-safety findings.

## 15. Closure evidence

Record implementation SHA, test commands/results, hosted run links, environmental gaps, unresolved findings by severity, and downstream state transitions in `plans/closure/build-qualification/003-status.md`.

## 16. Handoff

On closure, M004 is eligible to proceed because Manifest M002 is already closed. Reconcile archive encoding and finalization responsibilities within the producer boundary and avoid claiming publication or consumer extraction authority.

Closure: `plans/closure/build-qualification/003-status.md`.
