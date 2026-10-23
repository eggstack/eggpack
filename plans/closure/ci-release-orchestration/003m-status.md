# CI/Release Milestone 003m Closure — Bounded Process Execution Convergence Corrective

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003m-bounded-process-execution-convergence-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `282430b` (the M008 closure), on top of CI/Release M003l (`9cf0bf8`) and Build/Qualification M007 (`e3c6192`).

Implementation SHA: `bd73f5c` — `fix(ci): converge consumer-validator execution onto the bounded runner`, authored on branch `feat/m003m-bounded-process-execution` in an isolated git worktree and cherry-picked into the delivery branch as `6a7cb4c`.

## Executive finding

M003m is closed. The CI crate's consumer-validator execution no longer has its own process runner. It routes through the workspace's **single** bounded runner in `eggpack-core`, and the CI-local runner, its bounded readers, and its unbounded drain helper are **deleted**.

The defect this corrects was a real hang, not a style issue. The CI runner killed only the **direct child** and then joined its reader threads **without a bound**. A descendant that inherited the stdout or stderr pipe and outlived the child kept that pipe open, so the join never returned: the CI job sat there past its own timeout, past its cancellation flag, and past its output bound — defeating all three controls that the same function implemented. The `eggpack-core` runner had already solved this with `command-group` process groups and a bounded `join_drained`.

Nothing was weakened. The consumer validator's **contract** is unchanged — `ConsumerValidationOutcome` and `ConsumerValidationFailure` keep their variants and their meanings, and the mapping is asserted by test. The generated workflow bytes are unchanged. No dependency was added; `eggpack-ci` reaches the shared runner through `eggpack-core`, which it already depended on.

## Production implementation evidence

### `eggpack-core` — the shared runner, extended

| Location | Change |
|---|---|
| `crates/eggpack-core/src/builder.rs` | New public surface: `InterpreterCommandSpec` — executable, `Vec<OsString>` arguments, working directory, environment, timeout, stdout and stderr limits, and `expected_stdout` — and `run_interpreter_bounded(&InterpreterCommandSpec, Option<&AtomicBool>) -> Result<ProcessEvidence, BuildError>`. A new `ExecutableAllowance::Interpreter` variant admits only `python3` / `python`, so the interpreter case does not weaken the build or git allowances. |
| `crates/eggpack-core/src/builder.rs` | A private `Cancellation` enum (`None` / `Build` / `Flag`) so one runner serves both the build path's cancellation source and the CI path's flag. `MAX_PROCESS_OUTPUT` (8 MiB) is the single runner's ceiling while `MAX_OUTPUT` (256 KiB) remains the builder default. |
| `crates/eggpack-core/src/builder.rs` | **Prompt over-limit detection carried in:** a `stream_over_limit` helper plus a new poll-loop branch kills the process group at the **moment of excess** rather than at the deadline. Additionally, a run whose readers were not drained kills the group, so **no descendant can outlive a failed run**. `drain` and `join_drained` now join both readers under one budget. |

### `eggpack-ci` — the local runner, deleted

| Location | Change |
|---|---|
| `crates/eggpack-ci/src/lib.rs` | **Deleted**: `read_limited`, `read_limited_stderr`, `drain_readers` (the unbounded `join()`), `take_finished_over`, and `validator_command`. |
| `crates/eggpack-ci/src/lib.rs` | **Added**: `validator_spec`, which builds an `eggpack_core::InterpreterCommandSpec`. `run_interpreter_preflight` and `run_validator_process` now call `eggpack_core::run_interpreter_bounded`. The CI crate's local `Command` construction, `env_clear`, kill logic, and reader threads are gone entirely. |

### The grep evidence

The CI crate's **production region** — everything before its `#[cfg(test)]` module, which begins at line 4929 — contains **zero** occurrences of `Command::new`, `.spawn()`, `group_spawn`, `.kill()`, `thread::spawn`, `JoinHandle`, `Stdio::`, or `env_clear`. The CI crate's only two process entry points are calls to `eggpack_core::run_interpreter_bounded` (at lines 4649 and 4906). The CI-local symbols (`read_limited`, `drain_readers`, `take_finished_over`, `validator_command`) return no matches at all.

## The outcome mapping, asserted

Every row of the plan's §5.4 table is asserted by `m003m_runner_outcome_maps_onto_consumer_validation_outcome`:

| Runner outcome | Consumer validation outcome | Asserted by |
|---|---|---|
| success | `Passed` | the `pass.py` fixture |
| non-zero exit | `Failed(NonZeroExit)` | `sys.exit(3)` |
| deadline exceeded | `Failed(Timeout)` | a sleeping script with a 1 s timeout |
| output bound exceeded | `Failed(OutputLimit)` | a flooding script with a 1 KiB stdout limit |
| cancellation observed | `Failed(Cancelled)` | the cancellation flag set mid-run |
| interpreter not launchable | `Failed(InterpreterUnavailable)` | an emptied `PATH` — **`#[cfg(not(windows))]`**, because Windows `CreateProcess` search cannot simulate a missing interpreter. The existing suite uses the same guard. |

Supporting tests: `m003m_process_group_termination_leaves_no_live_descendant`, `m003m_bounded_join_returns_instead_of_hanging`, `m003m_parked_reader_fails_closed_when_the_descendant_escapes_the_group` (unix), `m003m_output_limit_terminates_the_run_before_its_deadline`, and `m003m_cancellation_terminates_the_group_and_reports_cancelled`.

## Discriminating-power proof

Each new test was shown to fail against the **pre-change** behaviour, for its own reason. This was demonstrated by temporarily reproducing the old runner behaviour in the shared runner and then **reverting** the experimental patches (`grep TEMPORARY` → no matches):

- with an unbounded join and a direct-child-only kill, `m003m_bounded_join_returns_instead_of_hanging` failed with *"the consumer validation never returned within 60s: the reader-thread join is not bounded…"*;
- with a direct-child kill and no group reap, `m003m_process_group_termination_leaves_no_live_descendant` failed with *"the descendant outlived the run: the process group was not terminated"*.

The bounded-join test uses an explicit outer timeout and a diagnostic that distinguishes "hung" from "failed closed", exactly as the plan requires.

## The one observable behaviour change for the build and git paths

> An over-limit process is now killed **at the moment of excess** rather than **at its deadline**. The observable outcome (`OutputLimitExceeded`) is unchanged; only the moment of termination moves earlier.

This is a strict improvement — the timeout, the cancellation flag, and the output bound are now all genuinely effective — and it is the plan's §8 recorded change. The evidence that no build or git outcome changed is that the `eggpack-core` suite is **byte-for-byte the same result set** as the pre-change baseline (81 passed, 0 failed, 6 ignored), including `stdout_overflow_is_bounded_and_not_success`, `timeout_kills_and_waits_for_the_process_group`, and `parked_drain_reader_cannot_outlive_the_deadline`.

## Verification executed

All verification below ran against implementation SHA `bd73f5c`, scoped to the actual change as the maintainer directed for this stage:

```text
cargo fmt --all -- --check                                                  passed
cargo check --workspace --all-targets --all-features --locked               passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked               passed (73 passed, 0 failed, 3 ignored; 67/3 before M003m)
cargo test -p eggpack-core --all-targets --all-features --locked             passed (81 passed, 0 failed, 6 ignored — identical to the pre-change baseline)
git diff --check                                                            passed
```

**Not run, by the maintainer's scope direction and by the absence of a push:** `./scripts/check-local.sh`, the `cargo +1.89.0` MSRV lane, the `cargo package` steps, and hosted CI. Plan §12 requires the full gate and a hosted run; neither was performed and no hosted run is claimed here. The full local gate runs once at the end of the chain.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §3 — every spawned process is terminated as a process group; no path kills only the direct child | `run_interpreter_bounded` uses `group_spawn()` with group `kill()` and `wait()`; `m003m_process_group_termination_leaves_no_live_descendant` observes the descendant's **absence**, not merely the child's exit; the `!drained` branch kills the group so a descendant cannot outlive a failed run. |
| §3 — every reader-thread join is bounded by a budget from the run's remaining deadline; a parked reader is detached and the run fails closed | `join_drained`; `m003m_bounded_join_returns_instead_of_hanging` and `m003m_parked_reader_fails_closed_when_the_descendant_escapes_the_group`. |
| §3 — the timeout, the cancellation flag, and the output bound each end a run promptly | `m003m_output_limit_terminates_the_run_before_its_deadline` and `m003m_cancellation_terminates_the_group_and_reports_cancelled`. |
| §3 — the environment is cleared except for a documented, minimal allowlist, defined once | The CI crate no longer constructs an environment at all; it passes the spec to the shared runner. The allowlist now lives in one place. |
| §3 — output limits are enforced on both streams and are distinguishable from a non-zero exit | The `stream_over_limit` helper covers both streams; the outcome mapping asserts `Failed(OutputLimit)` distinctly. |
| §3 — process execution never returns captured output contents | The runner returns bounded sizes, statuses, and the failure reason. `expected_stdout` matches a bounded banner; the contents do not cross the boundary. |
| §3 — the build and git paths keep their current behaviour exactly | The `eggpack-core` suite is byte-for-byte the same result set as the pre-change baseline. |
| §3 — no new dependency in `eggpack-ci` | `Cargo.toml` and `Cargo.lock` are untouched; `command-group` was already a dependency of `eggpack-core`. |
| §5.2 — the CI-local runner is removed | The grep evidence above: zero process-spawning code in the CI production region, and the five CI-local symbols return no matches. |
| §5.3 — prompt over-limit detection in the shared runner | The `stream_over_limit` helper and the poll-loop branch; `m003m_output_limit_terminates_the_run_before_its_deadline`. |
| §5.4 — the outcome mapping is preserved | `m003m_runner_outcome_maps_onto_consumer_validation_outcome` asserts every row. |
| §5.5 — diagnostic compatibility | The `ConsumerValidationFailure` rendered diagnostics are unchanged; the reasons are unchanged. Two narrowings are recorded below. |
| §7 — a failed run leaves no process group behind; reaping is attempted on every path | The `!drained` group kill plus the existing group `wait()` on every path. |
| §8 — no generated workflow byte changes | The consumer-validator step is invoked the same way; only its internals changed. |

## Observable behaviour changes

1. **Output-limit termination moves earlier** for the build and git paths: an over-limit process is killed at the moment of excess rather than at its deadline. The outcome (`OutputLimitExceeded`) is unchanged.
2. **The CI consumer validator's environment is now the shared narrow allowlist** — `PATH`, `SystemRoot`, `WINDIR`, `TEMP`, `TMP` — instead of CI's own `PATH` + `SYSTEMROOT`. The caller's explicit `PATH` override is preserved. This is exactly what the plan's §3 requires ("the allowlist is defined once, in one place, and the CI crate does not restate it"), and it is a narrowing.
3. **The interpreter version banner is matched on stdout only** (`expected_stdout`), because output contents never cross the runner boundary. The old check also accepted stderr. CPython 3.4 and later write `--version` to stdout, so no realistic interpreter changes outcome. The failure reason (`InterpreterUnavailable`) and the rendered diagnostics are unchanged.

Items 2 and 3 are narrowings in the same direction and are recorded here rather than absorbed.

## Compatibility and no-regression evidence

- **No generated workflow byte changed.**
- **No schema change.** `ConsumerValidationOutcome` and `ConsumerValidationFailure` keep their variants and their meanings.
- **No new dependency.** `Cargo.toml` and `Cargo.lock` are untouched in every crate.
- **No consumer import changed.** `eggpack-cli` and `eggpack-github` import `eggpack_ci` exactly as before.
- **No historical closure rewritten.**
- **Pre-existing tests stay green** in both crates.

## Security and reliability review

- The change removes a **denial-of-service shape**: a consumer validator's descendant could previously hold a CI job open indefinitely. The CI job's timeout, cancellation flag, and output bound were all ineffective against it.
- The CI crate no longer constructs processes at all. Its entire spawn surface is two calls into the shared runner, so the CI crate cannot drift from the workspace's process-safety policy again.
- The environment is cleared with one allowlist, defined once.
- No new network, credential, privilege, archive, or publication surface is touched.
- The failure reason for an unlaunchable interpreter is unchanged, so a CI log consumer sees the same line.

## Failure and recovery review

- Every termination path terminates the **whole group**. There is no path that kills only the direct child.
- Every reader join is bounded. A parked reader is detached and the run is reported as failed closed.
- A run's wall-clock time is bounded by its deadline, independent of any descendant's behaviour.
- The runner is stateless between runs: no shared mutable state and no lock, so concurrent runs are independent.
- A failed run leaves no process group behind; reaping is attempted on every path, including the error paths.
- Restarting a failed run re-runs it from its inputs; there is no resumable state.
- Rollback is a revert of this single commit. The unbounded CI-local runner returns; no consumer is affected, because the hang it caused was a failure mode rather than a contract.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, reliability) | The CI consumer-validator runner killed only the direct child and joined its reader threads without a bound, so a descendant holding an inherited pipe could wedge a CI job past its timeout, cancellation, and output bound. | The CI crate now routes through the shared process-group runner; the local runner, its readers, and its unbounded drain are deleted. The discriminating-power proof is recorded above. |
| Low (behaviour narrowing) | The CI consumer validator's environment is now the shared narrow five-variable allowlist instead of CI's own `PATH` + `SYSTEMROOT`. | Required by the plan's §3 ("the allowlist is defined once"). `SystemRoot` remains in the shared list, so Windows interpreters still launch. Recorded here. |
| Low (behaviour narrowing) | The interpreter version banner is matched on stdout only, because output contents never cross the runner boundary. | The failure reason and rendered diagnostics are unchanged; CPython 3.4+ writes `--version` to stdout. Recorded here. |
| Low (test coverage) | The "interpreter not launchable" row of the outcome mapping is asserted on Linux and macOS only. | `#[cfg(not(windows))]`, because Windows `CreateProcess` search cannot simulate a missing interpreter. The existing suite uses the same guard, so this is a pre-existing platform boundary rather than a new gap. |
| Operational boundary | Verification ran on Linux x86-64 only. The macOS and Windows lanes are covered by hosted CI only, and no hosted run was performed because nothing was pushed. | Accepted; the same boundary every prior milestone recorded. The plan's Windows-lane group/job-object test is written but is exercised only by hosted CI. |
| None | No unresolved medium-or-higher defect in the corrected producer contract. | M003m acceptance criteria satisfied. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003m | Reason |
|---|---|---|
| CI/Release M003m | closed | This record; implementation `bd73f5c` (cherry-picked as `6a7cb4c`). |
| CI/Release M003n | unblocked | Its only blocker was M003m. The CI crate's production region now contains no process-spawning code, so the module split it performs is a pure code move. |

## Downstream-unblock answer

Closing M003m unblocks **CI/Release M003n** and nothing else. Because M003n has not itself closed, no later milestone in the chain is unblocked by this pass.

## Eggstack references

- Closed source plan: `plans/implementation/ci-release-orchestration/003m-bounded-process-execution-convergence-corrective.md`
- Predecessor in this chain: `plans/closure/ci-release-orchestration/003l-status.md` (`9cf0bf8`)
- Successor: `plans/implementation/ci-release-orchestration/003n-producer-implementation-consolidation-corrective.md`
- Related: `architecture/process-execution.md`, `architecture/core-build.md`, `architecture/overview.md` (the safety sentence corrected in the same change)
