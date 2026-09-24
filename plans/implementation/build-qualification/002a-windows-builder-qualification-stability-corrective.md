# Build and Qualification Milestone 002a — Windows Builder Qualification Stability Corrective

Status: ready for handoff

Repository baseline: f5bb416481b1aad528c982ce3fe3e757af061ada

Historical M002 implementation plan:

- plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md

Historical M002 closure:

- plans/closure/build-qualification/002-status.md

Source roadmap:

- plans/subsystems/build-qualification-roadmap.md

Applicable ADR:

- plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md

Primary class: corrective / qualification stability / CI observability

## 1. Objective

Stabilize and strengthen Windows qualification evidence for the closed native/cross builder seam before Build/Qualification M003 qualification execution is planned or implemented.

M002a addresses an intermittent hosted-Windows workspace-test failure pattern observed after M002 closure. The same production code has passed Windows qualification and later failed on documentation/closure-only revisions, while an immediate rerun of the failed Windows job passed.

This corrective must determine whether the instability comes from:

- builder/process cleanup behavior;
- MSVC/linker/toolchain discovery;
- temporary-directory/test isolation;
- shared Cargo state;
- test parallelism/timing;
- GitHub-hosted runner environment variance;
- or another reproducible cause.

The corrective must leave M002 architecture intact unless a failing reproduction demonstrates a production defect.

Build M003 is re-blocked until M002a closes.

## 2. Post-closure evidence requiring correction

M002 originally closed using passing Windows evidence:

- run 36031996802 — all Linux stable, Linux Rust 1.89, macOS, and Windows lanes passed;
- run 36031996579 — second passing run for the same revision.

Subsequent revisions that did not materially change the builder exposed instability:

- run 36035978249 at commit 00a3399773045121baabbe86f062d41a4c2ce1fb:
  - Linux stable: pass;
  - Linux Rust 1.89: pass;
  - macOS: pass;
  - Windows workspace tests: fail.

- run 36040032768 at commit 69baab129963892505f1034a605669b83bb59a5a:
  - all required lanes passed.

- run 36040609714 at current baseline f5bb416481b1aad528c982ce3fe3e757af061ada:
  - attempt 1: Linux stable/MSRV/macOS pass, Windows workspace tests fail;
  - attempt 2, rerunning only the failed Windows job: Windows passes and the workflow completes successfully.

This pass/fail/rerun-pass pattern means a single successful Windows run is not sufficient evidence of deterministic qualification stability.

No deterministic production regression is currently established.

## 3. Existing implementation boundary to preserve

M002 currently owns:

- BuildBindingsV1;
- explicit Cargo package/bin source bindings;
- NativeCargo/CargoZigbuild command construction;
- shell-free bounded process execution;
- process-group/job cancellation and timeout cleanup;
- private Cargo target/work directories;
- exact candidate discovery;
- BuildAttempt/CandidateArtifact evidence.

M002a must not absorb:

- qualification execution semantics from M003;
- finalization/aggregation from M004;
- CIPlan/GitHub renderer authority from CI M001;
- release staging/publication;
- arbitrary command execution.

## 4. Corrective invariants

- Windows qualification must not depend on a lucky runner state.
- A failed Windows run must identify the exact failing builder test or environmental prerequisite in retained CI evidence.
- A test must not silently skip required M002 Windows execution evidence and still satisfy closure.
- Real Cargo candidate smoke and process-tree timeout/cancellation remain required Windows evidence.
- The corrective must distinguish product-code failure from hosted-runner/toolchain unavailability.
- No arbitrary shell or generic backend is introduced.
- Test isolation must not depend only on process IDs when multiple executions can reuse/collide with the same state.
- Temporary work must be unique per test invocation and cleaned safely.
- Tests that intentionally mutate or depend on process-global environment must be serialized or eliminated.
- M003 remains blocked until stability is demonstrated by repeated clean hosted evidence.

## 5. Scope

### In scope

- isolate Windows-sensitive builder tests from the broad workspace test command when useful for diagnosis;
- make temporary directories collision-resistant across repeated and parallel test execution;
- audit test use of process-global environment and shared Cargo/rustup directories;
- audit MSVC/linker discovery assumptions;
- add explicit diagnostic context sufficient to identify failed preconditions without leaking secrets;
- make Windows builder tests deterministic under normal test parallelism, or explicitly serialize only the tests that require global/process/toolchain state;
- add a dedicated Windows builder qualification CI step/job if that improves observability;
- repeat hosted Windows qualification enough times to demonstrate stability;
- update historical M002 closure with a post-closure corrective annotation;
- add plans/closure/build-qualification/002a-status.md at closure;
- reconcile roadmap/registry stale M002/M003 text.

### Out of scope

- changing ReleasePlan or BuildBindings wire/config semantics;
- adding M003 qualifier logic;
- changing final release artifact identity;
- installing arbitrary toolchains at runtime;
- changing supported Windows target policy without a new decision;
- weakening timeout/cancellation requirements;
- simply marking flaky tests ignored;
- unconditional retries that hide first-attempt failures.

## 6. Required investigation work

### A. Capture exact failing tests

The first implementation step must make the Windows failure attributable.

Prefer a dedicated CI sequence equivalent to:

~~~text
cargo test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
~~~

The exact structure may differ, but closure evidence must identify which test failed in any reproduced failure.

Do not rely solely on the aggregate workspace exit code.

### B. Audit temp/work directory uniqueness

Current builder tests use temporary paths derived partly from std::process::id().

Review every builder test temporary directory for collision behavior across:

- repeated execution in the same process;
- parallel unit tests;
- multiple Cargo test binaries;
- reruns after an interrupted/failed test;
- Windows delayed file/process cleanup.

Use a bounded unique test identifier such as process id plus an atomic counter or other deterministic-in-process uniqueness mechanism. Do not add randomness to production APIs.

All cleanup must remain scoped beneath the test-owned directory.

### C. Audit global environment dependence

Review Windows-sensitive execution for inherited variables including:

- PATH;
- VCToolsInstallDir;
- VCINSTALLDIR;
- LIB/LIBPATH/INCLUDE;
- WindowsSdkDir/WindowsSDKVersion;
- RUSTUP_HOME/CARGO_HOME;
- TEMP/TMP.

Tests must not mutate process-global environment concurrently.

If a test must temporarily alter environment, isolate it behind a test-only serial mechanism or refactor the production seam to accept explicit environment input where architecturally appropriate. Do not make production execution accept arbitrary environment maps beyond the bounded existing CommandSpec policy.

### D. Audit linker/toolchain discovery

The builder currently preserves a bounded Windows developer environment and prefers the Visual Studio linker when VCToolsInstallDir is present.

M002a must prove one of:

1. the hosted runner reliably exposes the required MSVC environment and the builder uses it deterministically; or
2. the test/CI lane explicitly initializes the documented Visual Studio developer environment before required Windows builder qualification.

Do not accept Git for Windows usr/bin/link.exe as an MSVC linker.

Do not silently return early from the required real Cargo smoke when the linker is absent and count that as qualification evidence.

If the hosted image legitimately cannot satisfy the required environment without an explicit setup step, add the narrow setup needed in CI and document it as runner provisioning rather than builder product behavior.

### E. Audit timeout/cancellation cleanup

Exercise timeout and cancellation repeatedly on Windows and verify:

- owned process/job termination completes;
- wait completes;
- reader threads terminate;
- temporary files/directories can be removed immediately after process cleanup;
- no child Cargo/rustc/build-script process remains alive;
- no subsequent test inherits locked files or poisoned environment/state.

If command-group has platform behavior that is sensitive to timing, wrap it with a deterministic post-kill/wait contract rather than adding sleeps until tests happen to pass.

### F. Cargo state isolation

Determine whether tests are contending on shared Cargo/rustup state.

Where practical:

- use test-owned CARGO_TARGET_DIR;
- keep tiny fixtures dependency-free and --offline;
- avoid concurrent writes to a shared fixture target;
- preserve normal read access to installed toolchains;
- do not clone registries or download dependencies merely for the test.

## 7. Required CI hardening

Add a Windows-specific qualification surface that makes M002 evidence obvious.

At minimum it must run the two acceptance-critical cases:

- real_local_cargo_fixture_builds_a_direct_candidate;
- timeout_kills_and_waits_for_the_process_group.

The CI configuration should ensure these cases are actually executed, not filtered/skipped.

Prefer an explicit command/job whose output lists the named tests.

Do not remove the ordinary Windows workspace test lane.

A retry may be used as diagnostic evidence during implementation, but closure cannot be based on "retry until green."

## 8. Required stability matrix

Before closure, record repeated Windows evidence on the final corrective SHA.

Minimum:

- three independent first-attempt successful Windows qualification runs on the identical final SHA or semantically identical no-code SHA;
- each run must execute both acceptance-critical tests;
- no manual rerun required for those qualifying runs;
- ordinary Windows workspace tests also pass.

If GitHub does not allow three runs of the exact same SHA without manual re-run mechanics, use a documented equivalent that does not change production code, such as workflow_dispatch/re-run-all, while counting only first attempt of each explicitly initiated qualification run.

A failure during this stability matrix reopens investigation; do not average it away.

Linux stable, Linux Rust 1.89, and macOS must remain green as regression guards.

## 9. Production-code policy

Begin with test/CI observability and test-isolation changes.

If repeated evidence shows only test harness/runner nondeterminism:

- fix tests/CI only;
- state explicitly that production builder code is unchanged.

If a builder defect is reproduced:

- add a focused regression;
- make the smallest production fix;
- preserve ADR-0004 boundaries;
- explain the exact failure mechanism in closure.

Do not refactor the entire process runner under a qualification corrective without demonstrated need.

## 10. Planning/status reconciliation

As part of registration and closure:

- retain historical M002 status as closed, but annotate it with the post-closure M002a stability finding;
- remove stale prose saying M002 is "Closing";
- remove stale M002 closure prose saying Windows verification remains unresolved while the same record claims closure;
- mark M002a as the active corrective;
- re-block M003 qualification execution until M002a closes;
- keep M004 blocked on M003;
- keep CI M001 closed;
- CI M002 remains blocked on Build M003/M004, so no additional CI milestone transition is needed.

## 11. Ordered work packages

1. Add named Windows builder qualification CI visibility.
2. Reproduce or stress the current pass/fail behavior without changing production semantics.
3. Make all builder test temp/work paths uniquely isolated.
4. Remove or serialize any process-global test environment mutation.
5. Harden/explicitly provision Windows MSVC environment detection.
6. Stress timeout/cancellation cleanup and immediate filesystem cleanup.
7. Add focused regressions for the identified cause.
8. Run the three-run Windows stability matrix.
9. Run full stable/MSRV/macOS/Windows checks.
10. Annotate M002 historical closure and write M002a closure.
11. Re-open M003 only if the stability matrix is clean.

## 12. Failure/restart semantics

A failing qualification run is evidence, not noise.

Do not auto-dismiss or automatically rerun it as success.

Implementation may rerun failures to characterize intermittency, but the first failed attempt must be retained in closure evidence with the identified cause or unresolved disposition.

Test-owned temporary state must be removable after success, timeout, cancellation, and failure.

## 13. Required tests

At minimum:

- unique temp-root generation for repeated same-process calls;
- parallel test temp roots cannot collide;
- stale previous temp directory cannot be mistaken for the current invocation;
- real Windows Cargo smoke executes rather than silently skips under the qualified CI environment;
- Git usr/bin/link.exe is not treated as the MSVC linker;
- explicit valid MSVC environment is preserved;
- timeout returns TimedOut only after process-group cleanup;
- cancellation returns Cancelled only after process-group cleanup;
- immediate post-timeout temp-tree removal succeeds;
- immediate post-cancellation temp-tree removal succeeds;
- repeated timeout/cancellation loops do not leak child processes/files;
- existing Linux/macOS process tests continue to pass;
- CI crate remains unaffected except ordinary workspace regression coverage.

## 14. Verification commands

Run and record at minimum:

~~~bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
git diff --check
~~~

On Windows hosted CI, record explicit named execution of the two acceptance-critical tests plus ordinary workspace tests.

## 15. Documentation updates

Update:

- plans/closure/build-qualification/002-status.md with a post-closure corrective annotation;
- plans/subsystems/build-qualification-roadmap.md;
- plans/registry.md;
- new plans/closure/build-qualification/002a-status.md at closure;
- builder README if test/runner provisioning requirements become operationally relevant.

Do not rewrite historical successful or failed runs.

## 16. Acceptance criteria

M002a closes only when:

- the source of the intermittent Windows failure is identified or convincingly eliminated through deterministic isolation/provisioning;
- required Windows builder tests execute explicitly;
- no required case silently skips;
- temp/work state is collision-resistant and cleanup-safe;
- process timeout/cancellation leaves no observable child/file residue;
- three independent first-attempt Windows qualification runs pass on the final corrective state;
- Linux stable, Rust 1.89, and macOS remain green;
- historical M002 closure language is reconciled truthfully;
- no unresolved medium-or-higher builder qualification/stability finding remains.

Only then may Build M003 return to ready-to-plan.

## 17. Stop conditions

Stop and prepare a new plan/ADR if:

- reliable Windows process-tree cleanup cannot be achieved with the selected command-group boundary;
- the supported Windows host policy requires a durable change;
- CI must install or bootstrap a materially different toolchain/backend;
- tests reveal a deeper CommandSpec/environment authority redesign is needed;
- M003 qualification semantics are required to solve M002 stability.

## 18. Closure evidence required

Record:

- exact corrective implementation SHA;
- exact root cause;
- production-code delta or explicit none;
- CI/test-isolation changes;
- named Windows builder test evidence;
- three qualifying Windows runs and run attempts;
- timeout/cancellation cleanup evidence;
- MSVC/linker environment evidence;
- full stable/MSRV/macOS/Windows results;
- historical failed runs retained as evidence;
- unresolved findings;
- explicit M003 readiness disposition.
