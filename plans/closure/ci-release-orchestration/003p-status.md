# CI/Release Orchestration Milestone 003p — Process-Group Termination of an Already-Exited Group

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003p-process-group-termination-of-an-already-exited-group.md`

Primary roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Normative deep dive: `architecture/process-execution.md`

Corrects: CI/Release M003m (bounded process execution convergence), closed on
`bd73f5c`

Reviewed baseline: `d4d5e4399255b8b5a2998e41f084896c45f5dd44`

Implementation commits:

- `182a0a1` — the first attempt: tolerate `ESRCH` from the group signal
- `b03e5c4` — temporary diagnostic on the consumer-validator path (removed)
- `16fee25` — temporary diagnostic on the group signal (removed)
- `27cb16c` — the corrected rule, which decides by observing the child

The two diagnostic commits are kept rather than rewritten out of history. They
are the evidence for the measured errno and they show the order in which the
cause was established.

## Executive finding

The 2026-10-23 post-implementation review chain had never run on hosted CI.
Its first run failed, and the failure was a real defect in the shared bounded
runner rather than an environment problem.

The runner terminates a supervised process group on its cancellation,
deadline, and output-bound paths, and treated a failed group signal as a hard
failure. A group's only member is the supervised child; when that child has
already exited it stays in the group as a zombie until it is reaped.

**Linux signals a zombie process group successfully. macOS refuses to signal
a group that holds one and reports the whole group signal as `EPERM`.** The
errno was measured on the failing lane, not assumed: a temporary diagnostic
reported `raw_os_error=Some(1)`, "Operation not permitted".

A validator that writes past its output bound and exits immediately therefore
produced a different result per platform. On Linux the group signal succeeded
and the outcome was `OutputLimitExceeded`. On macOS the signal failed, the
runner returned an error, and the CI consumer-validator — which maps every
runner error onto `InterpreterUnavailable` — reported an output-bound
violation as "the interpreter could not be launched".

Two defects shared one cause: a platform-dependent failure, and a
misdiagnosis. Both were fail-closed, so neither let an invalid release
through; the second cost an operator the wrong first place to look.

The fix makes the rule "a failed group signal may be ignored when there is
nothing left to terminate", and decides it by observing whether the
supervised child has exited rather than by reading the errno. `ESRCH` — the
kernel's own statement that the group holds no process — is benign. Any
other errno is benign only when the child has already exited, because a child
that has exited is not a process that can outlive the run. A group that holds
a live process and still cannot be signalled remains a hard failure, so the
run fails closed.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| The macOS lane passes | hosted run `40263176191`, `portability (macos-latest)` = success |
| Every lane passes | the same run: `linux (stable)` success, `linux (1.89.0)` success, `portability (windows-latest)` success |
| The runner tolerates a group with nothing left to terminate | `terminate_process_group` and `group_signal_failure_is_benign`, `crates/eggpack-core/src/builder.rs:644-694` |
| A live group that cannot be signalled still fails closed | `group_signal_failure_is_benign` returns false for `EPERM` with a live child |
| The decision is pinned | `a_failed_group_signal_is_benign_only_when_nothing_is_left_to_terminate` |
| The empty-group case is pinned | `terminating_an_already_exited_process_group_is_not_a_failure` |
| All three kill paths use the helper | three call sites; no raw `child.kill()` remains on the cancellation, deadline, or output-bound paths |
| The CI crate is unchanged | the CI crate's 85 public items and every rendered byte are unchanged |
| The public API is unchanged | `eggpack-core` and `eggpack-ci` public surfaces unchanged |
| The local gate is green | `scripts/check-local.sh` exit 0, 942 passed, 0 failed |
| Citations are re-based | 136 `builder.rs` citations across `architecture/**`, every one in range and landing on its construct |

## Production implementation evidence

The change is one helper and one decision function in
`crates/eggpack-core/src/builder.rs`, plus the `libc` constant they compare
against. `libc` is a Unix-only dependency of `eggpack-core`; it was already
in the dependency tree transitively through `command-group`.

```rust
fn terminate_process_group(
    child: &mut command_group::GroupChild,
    message: &str,
) -> Result<(), BuildError> {
    match child.kill() {
        Ok(()) => Ok(()),
        Err(error) => {
            // `try_wait` reaps a child that has already exited, so this both
            // observes and clears the zombie that made the signal fail.
            let child_exited = matches!(child.try_wait(), Ok(Some(_)));
            #[cfg(unix)]
            if group_signal_failure_is_benign(&error, child_exited) {
                return Ok(());
            }
            #[cfg(not(unix))]
            let _ = child_exited;
            Err(build_err(message))
        }
    }
}
```

The three kill paths — cancellation, deadline, and output bound — call this
helper. The output-bound path is the one the macOS lane exercised.

`group_signal_failure_is_benign` is the decision, extracted so it can be
pinned directly rather than only through the platform:

```rust
#[cfg(unix)]
fn group_signal_failure_is_benign(error: &std::io::Error, child_exited: bool) -> bool {
    if error.raw_os_error() == Some(libc::ESRCH) {
        return true;
    }
    child_exited
}
```

`Cargo.lock` changed by exactly one line: `libc` added to `eggpack-core`'s
dependency list. No version moved.

## Exact verification executed and results

### Hosted CI

| Run | Commit | Result |
|---|---|---|
| `40207520563` | `d4d5e43` | **failure** — macOS lane; `m003d_consumer_execution_matrix` and `m003m_runner_outcome_maps_onto_consumer_validation_outcome` reported `Failed(InterpreterUnavailable)` where they expected `Failed(OutputLimit)` |
| `40260732267` | `182a0a1` | **failure** — macOS lane; same two tests, same symptom. The `ESRCH`-only rule did not cover macOS's `EPERM` |
| `40261344903` | `b03e5c4` | **failure** — macOS lane; the diagnostic named the runner error: `output-limit process-group termination failed` |
| `40261826255` | `16fee25` | **failure** — macOS lane; the diagnostic named the errno: `raw_os_error=Some(1)`, `kind=PermissionDenied`, "Operation not permitted" |
| `40263176191` | `27cb16c` | **success** — all four lanes green, macOS included |

The first run is the one that matters: it is the first hosted run of the
2026-10-23 review chain, and it failed on a platform the local gate and the
Linux lanes never exercised.

### Local gate

`./scripts/check-local.sh` at `27cb16c` — exit 0: `fmt --check`, `check`,
`clippy -D warnings`, the per-crate test lanes, the workspace test lane,
`doc`, `tree`, seven `cargo package` invocations, and the MSRV check and test
lanes. **942 passed, 0 failed** — the 938 of the pre-correction baseline plus
the two new regression tests in the two lanes that run them.

### Targeted verification

- `cargo test -p eggpack-core --all-targets --all-features --locked -- builder::tests`
  — 18 passed, 0 failed, including both new regression tests.
- The empty-group regression test was confirmed to **fail** when the fix's
  arm is removed, so it is a real regression test and not a tautology.
- The rule was checked directly against the dependency's behaviour on the
  review machine: reaping a child and then signalling its group reports the
  empty group as an error (`raw_os_error` 3) on Linux too; a `try_wait`
  followed by a `wait` is safe.

## Invariant review

| Invariant | Result |
|---|---|
| The runner terminates the whole process group on every failure path | held — all three paths call the helper |
| No descendant may outlive the run | held — a descendant holding the write end of a pipe keeps the drain reader parked, which the drain budget detects, and the run fails closed with the output-limit outcome |
| A group that holds a live process and cannot be signalled fails closed | held — `group_signal_failure_is_benign` returns false for a live child |
| The check is Unix-only | held — the `EPERM`/`ESRCH` comparison is `#[cfg(unix)]`; the Windows error space, where 3 is `ERROR_PATH_NOT_FOUND`, is untouched |
| No public API change | held — `eggpack-core` and `eggpack-ci` public surfaces unchanged; the CI crate's 85 items unchanged |
| The generated workflow bytes are unchanged | held — no renderer change |
| The CI crate's outcome mapping is unchanged | held — the mapping still reports "the validator did not pass" |
| Determinism and stable serialization | held — no rendered output is affected |

## Failure and recovery review

- A failed group signal on a live group is still a hard error, and the run
  fails closed. The relaxation applies only when the supervised child has
  already exited.
- `try_wait` inside the helper reaps a child that has already exited. The
  runner's subsequent `wait()` after that is safe, and this was verified
  against the dependency directly rather than assumed.
- The change does not mask a partial termination: a descendant that keeps a
  pipe open is still caught by the drain budget, which fails the run closed.
- If the fix had been wrong, the macOS lane would have kept failing. It
  failed four times and then passed, so the signal is unambiguous.

## Compatibility and migration review

No consumer-visible change. The affected crate is `eggpack-core`, which is
not published. The CI crate's public API and every rendered workflow are
byte-identical. The only observable difference is that a run which previously
reported a runner error now reports the outcome it actually had.

## Security review

- The relaxation applies only to a group with nothing left to terminate. It
  cannot turn a run that should fail into one that passes: the decision
  requires the supervised child to have exited, which means there is no
  child left to leak.
- No new capability, credential, network, archive, or privilege surface.
- The added dependency is `libc`, a Unix-only constant crate already present
  transitively.
- The temporary diagnostics that printed the runner error to stderr were
  removed before the closing commit. The CI crate's contract of never
  returning captured output contents is unchanged.

## Documentation and operations evidence

- `architecture/process-execution.md` gains the subsection "Signalling a
  group that has already exited", stating the measured platform difference,
  the rule, why it is exact rather than permissive, and why the check is
  Unix-only.
- Every `builder.rs` citation in `architecture/**` — 136 across nine
  documents — is re-based onto the corrected tree **by content anchor**: each
  cited range is matched against the new source as a contiguous text
  sequence, and each landing was verified to hold what the sentence claims.
  133 citations fall inside the changed region and were rewritten; the rest
  are in the unchanged region. Every one is in range.
- The `eggpack-core/Cargo.toml` citations are re-based the same way, because
  the new platform-specific dependency moved that file's lines.
- The M003p plan and this closure record are the planning surface.

## Unresolved findings

- **None blocking.** The macOS lane is green and the local gate is green.
- **Informational — the emulated qualification path is still not covered end
  to end.** No emulated host or QEMU is available on the review machine, so
  that path remains unexercised by an actual emulated run. This is the same
  environmental gap M009a recorded, unchanged by this milestone.
- **Informational — two diagnostic commits remain in history**
  (`b03e5c4`, `16fee25`). They are the evidence for the measured errno and
  were kept rather than rewritten out, per the repository's rule that history
  is never rewritten.

## Roadmap disposition

CI/Release M003p is **closed**. The 2026-10-23 review chain has now run on
hosted CI and is green on all four lanes. Release Manifest M004 — the
`eggpack-manifest 0.1.1` publication and the ADR-0006 compatibility record —
is no longer blocked by it, and remains the active milestone for this
subsystem's release-manifest seam.

## Registry updates

- `plans/registry.md` — M003p row in the milestone index, an execution-graph
  node, and the handoff paragraph.
- `plans/subsystems/ci-release-orchestration-roadmap.md` — M003p narrative
  and table row.
- `AGENTS.md` — the current handoff.
