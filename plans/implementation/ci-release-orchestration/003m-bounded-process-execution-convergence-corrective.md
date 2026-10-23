# CI/Release M003m — Bounded Process Execution Convergence Corrective

Status: **blocked** on CI M003l.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`.
Primary class: invariant / reliability and isolation corrective.
Historical upstream authority: `architecture/process-execution.md` and
`architecture/overview.md:326-328`, which record the workspace rule that process
execution is *"bounded time/output, `command-group` process groups/jobs,
cleared env except small toolchain/path allowlist"* — and which already name
the defect this milestone corrects: *"`ci`'s independent reimplementation is the
only site without process-group kill."*
Affected closed milestones: CI M003d (implementation `946dd7a827fcbf7552721ed162972ae5ae78b5f2`,
introduced the CI-side consumer-validator runner and the CI-side bounded
readers). The closure remains historical and is not rewritten.
Dependency: **CI M003l** must be closed first, so that the rule surface is
settled before the execution surface is changed.

## 1. Objective

Make `eggpack-ci`'s consumer-validator execution use the workspace's single
bounded process runner, so that it inherits the same guarantees the build and
git paths already have:

- the supervised process is terminated as a **process group**, so no descendant
  outlives the run;
- the reader-thread joins are **bounded**, so a descendant that inherited a
  pipe cannot park the run past its deadline;
- the environment allowlist and the output bounds follow one definition.

The milestone removes the CI crate's independent process-execution
reimplementation rather than adjusting it in place.

## 2. Current evidence and the defects

`eggpack-core` and `eggpack-cli` already share one bounded runner:
`crates/eggpack-cli/src/main.rs:365` and `:384` call
`eggpack_core::run_git_tag_bounded` and `eggpack_core::run_git_bounded`, which
route into the shared runner at `crates/eggpack-core/src/builder.rs:596-692`.

`eggpack-ci` does not use it. The CI crate has its own implementation, with two
spawn sites:

| Site | Location | Purpose |
|---|---|---|
| `run_interpreter_preflight` | `crates/eggpack-ci/src/lib.rs:4733-4774` | probe an interpreter for `Python 3` |
| `run_validator_process` | `crates/eggpack-ci/src/lib.rs:5030-5104` | run the consumer validator script against a candidate |

Both use `crates/eggpack-ci/src/lib.rs:4696-4731` `validator_command` to build a
`std::process::Command`, and both read through the CI-local bounded readers
`read_limited` (`:4457-4481`) and `read_limited_stderr` (`:4483`).

### 2.1 Defect A — the reader-thread join is unbounded, and the run can hang

`crates/eggpack-ci/src/lib.rs:4783-4795`:

```rust
fn drain_readers(
    stdout: &mut Option<std::thread::JoinHandle<(Vec<u8>, bool)>>,
    stderr: &mut Option<std::thread::JoinHandle<(Vec<u8>, bool)>>,
) -> bool {
    let mut over = false;
    if let Some(handle) = stdout.take() {
        over |= handle.join().unwrap_or_default().1;
    }
    if let Some(handle) = stderr.take() {
        over |= handle.join().unwrap_or_default().1;
    }
    over
}
```

Both `join()` calls are **unbounded**. The function's own doc comment at
`:4779-4782` states the assumption it rests on:

> *"The child is killed first, so its pipes close and these joins return;
> leaving the handles to drop detaches threads that still hold a pipe no one
> drains, and their completion is part of the run's time bound."*

That assumption holds only when the killed child was the **only** writer to the
pipe. `run_interpreter_preflight` has the same unbounded join at
`:4763-4764`.

The core runner already solved exactly this problem, and its comment names the
hazard verbatim (`crates/eggpack-core/src/builder.rs:642-651`):

> *"a drain thread returns only at pipe EOF, which needs every write end
> closed, so a descendant that inherited stdout and outlived the child would
> park the join for that descendant's entire lifetime. The joins therefore get a
> bounded budget taken from what is left of the caller's own deadline, and a
> reader still parked at it is detached and failed closed below."*

Core implements this with `join_drained` (`crates/eggpack-core/src/builder.rs:700-…`)
and the `MIN_DRAIN_GRACE` / `MAX_DRAIN_GRACE` budget at `:649-651`.

**Consequence.** A consumer validator script that spawns a descendant which
inherits stdout or stderr leaves that pipe's write end open. When the CI runner
kills the direct child on timeout, cancellation, or output limit, the reader
thread never observes EOF, and the unbounded `join()` blocks forever. The
timeout, the cancellation flag, and the output bound — all three of which this
function implements — are defeated, and the CI job hangs indefinitely instead of
failing closed. A product-supplied validator script therefore has a path to
wedge the pipeline with no bound.

### 2.2 Defect B — only the direct child is killed

Both CI spawn sites call `child.kill()` (`:4754`, `:5068`, `:5074`, `:5083`,
`:5091`), which terminates the **direct child only**. The core runner uses
`command_group::CommandGroup` (`crates/eggpack-core/src/builder.rs:4`, `:596`),
whose `kill()` terminates the whole process group or job object.

A validator that spawns a long-lived descendant therefore leaves that
descendant running after the run has been declared failed, cancelled, or
timed out. On a CI runner that may be a process that survives the job step.

### 2.3 Defect C — the environment allowlist and output bounds are defined twice

The CI runner clears the environment and restores only `PATH`, plus
`SYSTEMROOT` on Windows (`crates/eggpack-ci/src/lib.rs:4708-4715`). The core
runner defines a per-allowance allowlist (`crates/eggpack-core/src/builder.rs:533-572`)
that additionally preserves `HOME`/`USERPROFILE` for git, the MSVC variables
for the builder, and `CARGO_HOME`/`RUSTUP_HOME`. The CI allowlist is not
wrong — it is stricter — but it is a second, separately-maintained definition
of the workspace's environment rule, and the two are not obviously related to a
reader.

The output bound is likewise defined twice: core caps `stdout_limit` and
`stderr_limit` at `MAX_OUTPUT` and refuses a zero limit
(`crates/eggpack-core/src/builder.rs:510-512`); the CI runner accepts the
request's limits and bounds them elsewhere.

### 2.4 The asymmetry that must not be lost

The CI runner has one behaviour core does **not** have: it detects an
over-limit stream **promptly** and kills the child immediately
(`crates/eggpack-ci/src/lib.rs:5073-5078`). The core runner detects over-limit
only after the process ends (`:659-661`, checked at `:678`), so a chatty
process runs until its deadline.

Converging CI onto core's runner without porting this behaviour would be a
**regression** in output-limit responsiveness. The prompt detection is
therefore carried into the shared runner as part of this milestone, not
discarded.

## 3. Invariants

- Every spawned process is terminated as a **process group**, so that no
  descendant outlives the run. There is no path in which a supervised run ends
  with a live descendant.
- Every reader-thread join is **bounded** by a budget derived from the run's own
  remaining deadline. A reader still parked at its budget is detached and the
  run is **failed closed**. The run's wall-clock time is bounded by its
  deadline regardless of what any descendant does.
- The timeout, the cancellation flag, and the output bound are **effective**:
  each must be able to end a run that ignores it, and each must end the run
  promptly rather than at some later point.
- The environment is **cleared** except for a documented, minimal allowlist.
  The allowlist is defined **once**, in one place, and the CI crate does not
  restate it.
- Output limits are enforced on both streams, and exceeding either is a
  distinguishable failure from a non-zero exit.
- Process execution never returns captured output **contents** to the caller;
  it returns bounded sizes, statuses, and the specific failure reason.
- The build and git paths keep their current behaviour exactly. This milestone
  changes the CI crate's execution path; it does not change what a build does.
- No new dependency in `eggpack-ci`. `command-group` is already a dependency of
  `eggpack-core`; if the shared runner is used, `eggpack-ci` needs none.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-ci/src/lib.rs`, and the minimal
`crates/eggpack-core/src/builder.rs` changes needed to (a) offer the shared
runner to the CI crate and (b) carry CI's prompt over-limit detection into it.

**Out of scope:**

- CI M003l's rule convergence and CI M003n's duplication removal.
- Any change to the **build** or **git** execution paths' behaviour. The core
  runner's existing semantics are preserved.
- Any change to the consumer validator's contract: the request shape, the
  outcome enum, the failure reasons, and the evidence recorded all stay as they
  are.
- Any change to the generated workflow bytes.
- Any change to the product-side smoke-hook contract.
- Any new dependency in any crate.
- Any change to `ReleaseManifest` v1 or any other schema.

## 5. Required production changes

### 5.1 One shared runner

The CI crate's consumer-validator execution routes through the shared bounded
runner in `eggpack-core`, with an executable allowance that fits a
consumer-supplied interpreter and script. The shared runner must be usable for
this case without weakening the build or git allowances.

The runner's existing guarantees carry over unchanged: cleared environment with
the documented allowlist, null stdin, piped stdout and stderr, bounded output,
bounded deadline, bounded drain join, group termination.

### 5.2 Prompt over-limit detection in the shared runner

The shared runner gains the behaviour CI has and core lacks: when either
stream's bound is exceeded, the process group is terminated promptly and the
run is reported as an output-limit failure rather than waiting for the
deadline.

This is a **strict improvement** for the build and git paths too, and it is
recorded as such. It does not change any outcome for a process that stays
within its bounds.

### 5.3 The CI-local runner is removed

`crates/eggpack-ci/src/lib.rs`:

- `validator_command` (`:4696-4731`) is removed or reduced to argument
  construction that the shared runner accepts;
- `run_interpreter_preflight` (`:4733-4774`) routes through the shared runner;
- `run_validator_process` (`:5030-5104`) routes through the shared runner;
- `read_limited` (`:4457-4481`), `read_limited_stderr` (`:4483`…), and
  `drain_readers` (`:4783-4795`) are removed;
- `take_finished_over` (`:4797-4804`) is removed or subsumed.

Every removal is verified by grep: no second process runner and no second
bounded reader remain in the CI crate.

### 5.4 Outcome mapping is preserved

`ConsumerValidationOutcome` and `ConsumerValidationFailure` keep their current
variants and their current meanings. The mapping from runner outcomes to
consumer-validation outcomes is:

| Runner outcome | Consumer validation outcome |
|---|---|
| success | `Passed` |
| non-zero exit | `Failed(NonZeroExit)` |
| deadline exceeded | `Failed(Timeout)` |
| output bound exceeded | `Failed(OutputLimit)` |
| cancellation observed | `Failed(Cancelled)` |
| interpreter not launchable | `Failed(InterpreterUnavailable)` |

The mapping is asserted by test, so the shared runner's richer outcome set does
not silently change what a consumer validator reports.

### 5.5 Diagnostic compatibility

`ConsumerValidationFailure`'s rendered diagnostics stay as they are. A consumer
that matches on a CI log line must not see it change. The closure record lists
any diagnostic change explicitly.

## 6. Ordered work packages

### WP1 — Failing discriminating tests first

- **Group termination:** a supervised script that spawns a descendant which
  outlives it is terminated such that the descendant does not survive the run.
  The test must observe the descendant's absence, not merely the direct child's
  exit.
- **Bounded join:** a supervised script that spawns a descendant which holds
  stdout open past the deadline causes the run to **return** — with a
  fail-closed outcome — within a bounded time, rather than blocking. The test
  asserts the run completes and that its outcome is a failure, not a hang.
  **This test must fail against the current tree by hanging**, so it is written
  with an explicit outer timeout and a diagnostic that distinguishes "hung" from
  "failed closed".
- **Prompt output limit:** a script that writes past its output bound is
  terminated promptly — measurably before its deadline — and reports the
  output-limit failure.
- **Cancellation:** a cancelled run terminates the group and reports
  `Cancelled`.
- **Outcome mapping:** every row of the §5.4 table is asserted.

### WP2 — Shared runner extension

Extend the shared runner with the CI use case and with prompt over-limit
detection. Prove the build and git paths' existing tests still pass unchanged.

### WP3 — CI migration

Route both CI spawn sites through the shared runner and delete the CI-local
runner, readers, and drain helpers.

### WP4 — Verification and documentation

Run the full gate. Update `architecture/process-execution.md`,
`architecture/ci.md`, `architecture/core-build.md`,
`crates/eggpack-ci/README.md`, `crates/eggpack-core/README.md`, the CI
roadmap, and the registry. `architecture/overview.md:326-328` carries the
sentence naming CI as the only site without process-group kill — that sentence
must be corrected in the same change.

## 7. Failure, restart and contention semantics

- Every termination path terminates the **whole group**. There is no path that
  kills only the direct child.
- Every reader join is bounded. A parked reader is detached, and the run is
  reported as failed closed.
- A run's wall-clock time is bounded by its deadline, independent of any
  descendant's behaviour. The deadline is the only bound that matters and it
  is always in force.
- The runner is stateless between runs. There is no shared mutable state and no
  lock, so concurrent runs are independent.
- A failed run leaves no process group behind. Reaping is attempted on every
  path, including the error paths.
- Restarting a failed run re-runs it from its inputs; there is no resumable
  state.

## 8. Compatibility and migration

- **No generated workflow byte changes.** The consumer-validator step is
  invoked the same way; only its internals change.
- No schema change. `ConsumerValidationOutcome` and
  `ConsumerValidationFailure` keep their variants and meanings.
- No new dependency. `eggpack-ci` reaches the shared runner through
  `eggpack-core`, which it already depends on, and `command-group` is already
  a dependency of `eggpack-core`.
- The prompt output-limit change **does** alter behaviour for the build and git
  paths: a process that exceeds its output bound is now killed at the moment of
  excess rather than at its deadline. The observable outcome
  (`OutputLimitExceeded`) is unchanged; only the moment of termination moves
  earlier, and no test asserted the old timing. The closure record states this
  explicitly.
- `eggup-eggpack` and every consumer are unaffected.
- No consumer configuration change is required.

## 9. Required tests

- the group-termination test from WP1;
- the bounded-join test from WP1, with an explicit outer timeout so a
  regression is reported as a hang rather than a silent timeout;
- the prompt output-limit test from WP1, asserting the run ends materially
  before its deadline;
- the cancellation test from WP1;
- the outcome-mapping tests from WP1;
- the full existing `eggpack-ci` and `eggpack-core` suites, unchanged;
- the existing build and git process tests, proving the shared runner's new
  prompt-detection behaviour did not change their outcomes;
- a Windows-lane test for group/job-object termination, in the same spirit as
  the existing Windows process-group test.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
```

Add a direct narrowed run for the new tests, e.g.
`cargo test -p eggpack-ci --all-targets --all-features --locked <new_test_name> -- --nocapture`.
Append `-- --test-threads=1` when diagnosing Windows serialization.

## 11. Documentation updates

- `architecture/process-execution.md` — one runner, one environment allowlist,
  one bound set; the CI crate is no longer an exception.
- `architecture/ci.md` — the consumer-validator execution path.
- `architecture/core-build.md` — the shared runner's prompt output-limit
  detection.
- `architecture/overview.md:326-328` — **remove the sentence naming CI as the
  only site without process-group kill**, because it becomes false. This
  correction is in scope for this milestone and is not deferred to the
  Planning Hygiene reconciliation.
- `crates/eggpack-ci/README.md`, `crates/eggpack-core/README.md`.
- `plans/subsystems/ci-release-orchestration-roadmap.md` — the M003m row.

## 12. Acceptance criteria

1. `eggpack-ci` contains no independent process runner, no independent bounded
   reader, and no unbounded reader-thread join. Grep evidence is in the closure
   record.
2. A supervised consumer validator that spawns a descendant leaves no live
   descendant after the run, proven by test.
3. A supervised consumer validator whose descendant holds a pipe open past the
   deadline **returns** within a bounded time with a fail-closed outcome. This
   is proven by a test that fails against the pre-change tree.
4. The timeout, cancellation, and output-limit bounds are each effective and
   prompt.
5. The environment allowlist and the output bounds are defined once.
6. `ConsumerValidationOutcome` and `ConsumerValidationFailure` keep their
   variants and meanings; the mapping is asserted by test.
7. No generated workflow byte changes; no schema changes; no new dependency.
8. The full workspace gate and `./scripts/check-local.sh` are green, and hosted
   CI is green on all lanes, including the Windows portability lane.
9. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- the shared runner cannot express the consumer-validator case without
  weakening the build or git allowances;
- prompt over-limit detection cannot be added without changing an existing
  build or git outcome;
- the bounded-join fix would require detaching a reader in a way that could
  report a run as passed when it was not;
- any `ConsumerValidationOutcome` or `ConsumerValidationFailure` variant would
  need to change meaning;
- the change would require a new dependency, a new schema, or a new command;
- the CI crate's consumer-validator step would require a changed invocation
  from the generated workflow.

## 14. Closure evidence

Write `plans/closure/ci-release-orchestration/003m-status.md` with: the
reviewed baseline and implementation SHA; the before/after runner inventory;
the group-termination evidence; the bounded-join evidence, explicitly stating
that the test hangs against the pre-change tree and returns against the
post-change tree; the prompt output-limit evidence with the measured
termination time against the deadline; the outcome-mapping evidence; the grep
output proving the CI-local runner, readers, and drain helpers are gone; the
verification commands executed and their actual results; hosted CI run IDs and
the lane matrix including the Windows portability lane; an invariant review
against §3; the explicit statement of the build/git prompt-detection behaviour
change; unresolved findings by severity; and the CI M003n unblock.

The CI M003d closure record remains historical and unchanged.

## 15. Handoff notes

- Start from the CI M003l closure SHA.
- Write WP1's bounded-join test **first** and confirm it hangs against the
  current tree. That hang is the evidence that the defect is real; a test that
  merely passes before the change proves nothing.
- The prompt output-limit behaviour is CI's existing strength. Port it into the
  shared runner; do not let the convergence lose it.
- `architecture/overview.md:326-328` must be corrected in this change, not
  left to a later reconciliation. Leaving a false statement in the architecture
  index after fixing the code it describes would create a new drift.
