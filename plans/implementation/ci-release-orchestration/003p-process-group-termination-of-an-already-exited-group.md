# CI/Release Orchestration Milestone 003p — Process-Group Termination of an Already-Exited Group

Status: active

Closure record: `plans/closure/ci-release-orchestration/003p-status.md`

Repository authoring baseline: `d4d5e4399255b8b5a2998e41f084896c45f5dd44`

Corrects: CI/Release M003m (bounded process execution convergence), which closed
on `bd73f5c` (cherry-picked `6a7cb4c`)

Primary roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Normative deep dive: `architecture/process-execution.md`

Primary class: platform-dependent failure in the shared bounded runner

## 1. Why this milestone exists

The first hosted CI run of the 2026-10-23 review chain — run `40207520563` on
`d4d5e43` — failed. The Linux lanes (stable and 1.89.0) and the Windows lane
passed; the macOS lane failed on two tests in the CI crate:

```text
m003d_consumer_execution_matrix
  left: Failed(InterpreterUnavailable)
 right: Failed(OutputLimit)

m003m_runner_outcome_maps_onto_consumer_validation_outcome
  left: Failed(InterpreterUnavailable)
 right: Failed(OutputLimit)
```

Both are the consumer-validator output-bound case: a validator that floods
stdout past its bound. The runner is expected to terminate the process group
and report the output-bound violation. On macOS it reported a *launch*
failure instead.

The chain had never run on hosted CI, so this was invisible locally. It is
also invisible on Linux: the local gate, which runs the same tests, is green.

### Root cause

The shared bounded runner terminates a supervised process group on three
paths — cancellation, deadline, and output bound. All three did:

```rust
child.kill().map_err(|_| build_err("... process-group termination failed"))?;
```

`command-group`'s `kill()` is `killpg(pgid, SIGKILL)` on Unix, and it
documents that a group which has already exited produces an error.

The group's only member is the supervised child. When that child has already
exited it stays in the group as a zombie until it is reaped, and the group
holds no process at all once it is reaped. The two supported platforms
disagree about the zombie case, and **not in the way the errno suggests**:

- **Linux** — `killpg` signals a zombie successfully, so the group signal
  succeeds.
- **macOS** — `killpg` refuses to signal a group that holds an unsignalable
  zombie and reports the *whole* group signal as **`EPERM`**, not `ESRCH`.

The errno was measured, not assumed. A temporary diagnostic on the
consumer-validator path named the runner error
(`output-limit process-group termination failed`); a second one on the group
signal named the errno (`raw_os_error=Some(1)`, "Operation not permitted").
Both diagnostics were removed once the cause was identified; the commits
that carried them are `b03e5c4` and `16fee25`.

A validator that writes past its bound and exits immediately therefore
produces a different result per platform:

- **Linux** — `killpg` succeeds, the group is reaped, the run breaks out with
  the output-limit flag set, and the outcome is `OutputLimitExceeded`.
- **macOS** — `killpg` reports `EPERM`, the runner treats that as a hard
  failure, and the caller maps every runner error onto
  `InterpreterUnavailable`.

Two defects, one cause.

1. **A platform-dependent failure.** The same input produced a different
   outcome per platform. The macOS lane could not go green.
2. **A misdiagnosis.** The CI consumer-validator collapses every runner
   error into `InterpreterUnavailable` — "the interpreter could not be
   launched". The interpreter *was* launched; it was terminated for exceeding
   its output bound. The caller's contract is "the validator did not pass",
   and both spellings fail closed, so this was a diagnostic-accuracy drift
   rather than a safety regression. But a run that was killed for exceeding
   its bound is not a run that could not start, and an operator reading that
   outcome would be sent to the wrong cause.

The same race exists on the deadline and cancellation paths: a child that
exits just as the deadline passes can be reaped before the kill, producing
the identical `ESRCH`. The Windows lane is unaffected — a job object is
terminated through `TerminateJobObject`, which succeeds for an empty job.

## 2. Why ready

- The failure is reproduced and explained, down to the errno. It is not a
  flake: the platform difference is deterministic in the kernel, the macOS
  lane failed on both affected tests in every run, and a diagnostic run
  reported the errno directly.
- The fix is local to one helper and one decision function, and does not
  change any outcome the runner reports for a group that holds a live
  process.
- The rule is testable on the review machine without macOS. The empty-group
  case is reproduced on Linux too — reaping a child and then signalling its
  group reports the empty group as an error there as well — and the decision
  itself is a pure function of the errno and the child's state, so it is
  pinned directly.

## 3. Objective

1. Make the shared bounded runner treat "there is nothing left to terminate"
   as a completed termination rather than a failed one, on all three kill
   paths — decided by observing whether the supervised child has exited,
   not by reading the errno.
2. Keep every other termination failure a hard failure, so a group that
   holds a live process and cannot be signalled still fails the run closed.
3. Prove the behaviour with a regression test that fails without the fix.
4. Re-base the citations in `architecture/process-execution.md` and the other
   architecture deep dives onto the corrected tree.
5. Record the platform difference in the normative process-execution deep
   dive, so the next reader does not have to rediscover it.

## 4. Authoritative ownership

The rule belongs to the shared runner in `eggpack-core`, not to the CI crate.
`run_bounded` is the single owner of process-group termination for every
spawn in the workspace — builds, toolchain preflights, git verification,
qualification smokes, and the CI consumer-validator alike — so the rule is
enforced once, in one place, for all of them.

The CI crate's outcome mapping is **not** changed. Its contract is "the
validator did not pass", and it reports that faithfully. Widening the CI
crate's mapping to distinguish launch failures from termination failures
would change a published-ish surface to paper over a defect in its callee.

## 5. Invariants

- The runner terminates the whole process group on every failure path. No
  descendant may outlive the run.
- A group that holds no process is already terminated. `ESRCH` is the
  kernel's own statement of that fact, and it is treated as success.
- A group whose only member is the unreaped zombie of a child that has
  already exited holds nothing that can outlive the run. macOS reports that
  signal as `EPERM`, and it is treated as success.
- A group that holds a **live** process and cannot be signalled remains a
  hard failure. The run fails closed.
- The check is Unix-only. A Windows job object uses a different error space,
  where the value `3` is `ERROR_PATH_NOT_FOUND` and not `ESRCH`.
- No public API changes. `eggpack-core`'s public surface, `eggpack-ci`'s
  85-item public API, and the generated workflow bytes are unchanged.
- The CI crate's outcome mapping is unchanged.
- The timeout, cancellation, and output-bound outcomes are unchanged for
  every input that is not the already-exited group.

## 6. In scope

### 6.1 The termination helper

A single private helper in `crates/eggpack-core/src/builder.rs` owns the
group signal, and a second private function owns the decision about whether
a failed signal may be ignored. All three kill paths call the helper.

### 6.2 The regression tests

Two tests. The first reaps a child, asserts that the dependency reports the
empty group as an error, and asserts that the runner does not. The second
pins the decision itself: `ESRCH` is benign, `EPERM` is benign only when the
child has exited, and `EPERM` with a live child is not benign.

### 6.3 The citation re-base

`architecture/process-execution.md` carried 85 `builder.rs` citations and the
other deep dives carried 38 more. The fix shifted the source, so every
citation is re-based **by content anchor** — each cited range is matched
against the new source as a contiguous text sequence — and each landing is
verified to hold what the sentence claims. The `eggpack-core/Cargo.toml`
citations are re-based the same way, because the new platform-specific
`libc` dependency moved that file's lines.

### 6.4 The deep dive

`architecture/process-execution.md` gains a subsection stating the platform
difference, the measured errno, the rule, why it is exact rather than
permissive, and why the check is Unix-only.

## 7. Out of scope

- Changing the CI crate's error-to-outcome mapping.
- Changing the runner's outcomes for any input other than the already-exited
  group.
- Widening the process-group guarantee, or the timeout, cancellation, or
  output bounds.
- The `InterpreterUnavailable` mapping's other callers.
- Any change to the generated workflow bytes.
- The Release Manifest M004 publication, which is a separate milestone.

## 8. Required work packages

### WP1 — The termination helper

Add `terminate_process_group`, which signals the group and treats `ESRCH` as
"nothing left to terminate" on Unix. Route all three kill paths through it.

Exit: the helper exists and all three paths call it.

### WP2 — The regression test

A test that reaps a child and then signals its group.

Exit: the test fails without the fix and passes with it.

### WP3 — Citation re-base

Re-base every `builder.rs` and `eggpack-core/Cargo.toml` citation in
`architecture/**` by content anchor, and verify each landing.

Exit: every citation is in range and lands on the construct it claims.

### WP4 — Closure

Record the hosted run that found the defect, the fix, the verification, the
re-base, and the residual findings.

Exit: the closure record is accepted.

## 9. Acceptance criteria

| Criterion | Evidence |
|---|---|
| Empty group is not a failed termination | the regression test passes |
| The test catches the defect | the test fails when the `ESRCH` arm is removed |
| Other errnos still fail closed | the helper's fallthrough arm is unchanged and is the only other behaviour |
| All three kill paths use the helper | three call sites, no raw `child.kill()` remains on those paths |
| Unix-only check | the `ESRCH` arm is `#[cfg(unix)]`; the Windows error space is untouched |
| No public API change | `eggpack-core` and `eggpack-ci` public surfaces unchanged; the CI crate's 85 items unchanged |
| Generated bytes unchanged | no workflow or renderer change |
| Citations re-based | every `architecture/**` citation in range and landing on its construct |
| Hosted CI green | the macOS lane green on the candidate commit |

## 10. Stop conditions

Stop, and do not publish, if any of these hold:

- the local gate is not green;
- hosted CI is not green on the exact publication candidate commit;
- the regression tests do not fail without the fix;
- the CI crate's public API or the generated bytes changed;
- a citation lands on a construct other than the one its sentence claims.

## 11. Security and maintenance review

- The change makes a termination *less* likely to be reported as a failure.
  It cannot make a run succeed that previously failed for a real reason: the
  tolerated errno is specifically the one that means "nothing to terminate".
- The group guarantee is unchanged. `ESRCH` means the group is empty, so
  there is provably no descendant to outlive the run.
- No new capability, credential, network, or archive surface.
- The dependency added is `libc`, a Unix-only, no-code-generation crate that
  was already in the dependency tree transitively through `command-group`.

## 12. Documentation / registry updates required at closure

- `plans/closure/ci-release-orchestration/003p-status.md`.
- `plans/registry.md` — milestone row, execution-graph node, handoff.
- `plans/subsystems/ci-release-orchestration-roadmap.md` — M003p section and
  table row.
- `architecture/process-execution.md` — the new subsection, plus the
  citation re-base.
- `AGENTS.md` — the current handoff.

## 13. Handoff after closure

The shared bounded runner has one documented termination rule across all four
supported platforms. The 2026-10-23 review chain has now run on hosted CI,
and the release-manifest publication it was blocking can proceed under
Release Manifest M004.
