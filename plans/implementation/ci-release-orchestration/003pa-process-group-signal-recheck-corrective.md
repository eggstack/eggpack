# CI/Release Orchestration Milestone 003pa — Process-Group Signal Recheck Corrective

Status: closed

Corrects: CI/Release M003p, closed at `27cb16c` with closure `plans/closure/ci-release-orchestration/003p-status.md`. The historical plan and closure remain unchanged.

Repository authoring baseline: `1cd6d0bf43c93dc9fd572c2d69a3a675a2958b09`

Primary roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Normative deep dive: `architecture/process-execution.md`

Primary class: fail-closed process-group termination

## 1. Why this milestone exists

At the reviewed baseline, `terminate_process_group` treats a failed Unix group signal as benign whenever `GroupChild::try_wait` reports that the supervised leader exited (`crates/eggpack-core/src/builder.rs`, `group_signal_failure_is_benign` and `terminate_process_group`). Reaping the leader proves only that the leader exited; it does not prove the process group is empty. A surviving descendant may have closed stdout and stderr, so the drain budget is not proof that no group member remains.

Apple documents `EPERM` for `killpg` when a group member is not signalable, and no group signal is sent. Therefore, accepting `EPERM` based only on the leader's exit can report termination as complete while a live descendant remains. `ESRCH` remains the kernel's direct indication that the group does not exist.

The local output-bound regression did not reproduce in 30 repeated runs during research. A prior diagnostic sampled `EPERM` while the direct child had exited; it did not establish that the entire group was empty. The corrective must preserve the macOS zombie-only behavior without broadening the exception to live or unverified group members.

## 2. Why ready

- M003p is closed and its behavior is identified precisely; this plan corrects that closed milestone without editing its historical record.
- The defect is localized to the shared `eggpack-core` termination helper and its tests. The shared runner is the authority for builds, qualification, git, and CI consumer validation.
- No schema, public API, dependency, or external-repository change is required.
- The user has requested implementation and verification. The exact acceptance and local verification steps are listed below; hosted cross-platform qualification remains a closure requirement and is not implied by local success.

## 3. Objective

1. Keep an initial `ESRCH` benign.
2. For another failed Unix group signal, observe/reap the supervised child. If it is still live or its status is unavailable, keep the failure hard.
3. If the leader has exited, retry the group signal after reaping it. Treat only retry success or retry `ESRCH` as completed termination; any other retry error remains a hard failure.
4. Preserve the Windows job-object behavior and all public outcomes outside this termination distinction.
5. Add deterministic coverage for the retry decision and coverage that a surviving group descendant is still targeted; correct the normative process-execution documentation.

## 4. Authority and invariants

The termination rule belongs to `eggpack-core`'s shared bounded runner, not to the CI outcome mapper. The runner must not declare process-group termination complete based solely on the direct child's state.

Affected invariants:

- every failure path targets the whole process group/job;
- no live, unverified, or unsignalable group is treated as terminated;
- only `ESRCH`, successful retry, or retry `ESRCH` establish that no further signal failure remains;
- Windows job-object semantics and error classification remain unchanged;
- no public API, serialized format, generated workflow, or dependency changes;
- deterministic behavior and fail-closed error mapping are preserved.

## 5. Scope

### In scope

- Tighten the private Unix group-signal failure/retry decision in `crates/eggpack-core/src/builder.rs`.
- Update the focused helper tests and add a process-group descendant regression if it can be made bounded and leak-safe.
- Update `architecture/process-execution.md` so it no longer claims that an exited leader proves the group is empty or that pipe-drain completion detects every descendant.
- Re-verify architecture citations into `builder.rs` by symbol/content, not by arithmetic; update affected citations.
- Update this plan's active status surfaces in the CI/release roadmap, registry, and `AGENTS.md`. Do not modify M003p's plan or closure.

### Out of scope

- Changing CI validator error-to-outcome mapping or generated workflow bytes.
- Changing timeout, cancellation, output-limit policy, process-group creation, or public APIs.
- Suppressing `EPERM` unconditionally, adding privileged test setup, or claiming that a direct-child status proves the whole group is empty.
- Remote workflow dispatch, publication, or changes to unrelated pre-existing worktree files. A commit/push requires explicit user authorization; the later authorization to push the implementation candidate solely for hosted CI is recorded in the closure record.

## 6. Required work packages

### WP1 — Fail-closed signal retry

When the initial group signal fails with `ESRCH`, return success. For any other error, call `try_wait`. If the child is live or the wait status is unavailable, return the existing termination error. If the child exited, retry `GroupChild::kill`; accept retry success or retry `ESRCH`, and preserve the termination error for every other retry failure.

### WP2 — Regression coverage

- Deterministically cover initial `ESRCH`, a live/unverified leader, and an exited leader whose retry returns success, `ESRCH`, or `EPERM`. Confirm the retry closure is not called for the initial `ESRCH` or a live/unverified leader.
- Retain the real empty-group test.
- Add or strengthen a bounded process-group test demonstrating that a live descendant is still targeted after the leader exits. Do not rely on changing UID or other privileged setup; model an unsignalable retry with deterministic injected outcomes.
- Keep the output-overflow test as end-to-end coverage of the actual runner path.

### WP3 — Normative documentation and citations

Correct the process-execution deep dive's rule and descendant/drain claims, record the retry semantics and Windows boundary, and re-verify affected `builder.rs` citations by their named constructs. Preserve the M003p plan and closure as historical evidence.

### WP4 — Verification and closure evidence

Run the exact targeted commands and the canonical local gate below. Record results, including any platform or hosted-CI gaps, in a new `plans/closure/ci-release-orchestration/003pa-status.md` only when closure evidence is available. Do not mark this milestone closed based solely on compilation or local tests; hosted macOS, Linux stable/MSRV, and Windows lanes are required before closure.

## 7. Failure, restart, and compatibility

A retry error other than `ESRCH` is never downgraded because the supervised leader exited. The original termination error remains the public failure classification; captured process output remains unavailable to callers. If a test fails or a group cannot be proven terminated, stop and retain fail-closed behavior. The patch adds no migration requirement or compatibility change.

The working tree contains unrelated pre-existing edits in `Cargo.lock`, `architecture/ci.md`, `architecture/github.md`, `architecture/overview.md`, `crates/eggpack-ci/Cargo.toml`, `crates/eggpack-ci/src/tests.rs`, `crates/eggpack-github/Cargo.toml`, and `crates/eggpack-github/src/lib.rs`; leave them intact and out of scope.

## 8. Tests and exact verification commands

Focused tests:

```bash
cargo test -p eggpack-core --all-targets --all-features --locked group_signal -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked terminating_an_already_exited_process_group_is_not_a_failure -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked terminating_an_exited_leader_still_signals_a_live_group_descendant -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked stdout_overflow_is_bounded_and_not_success -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked parked_drain_reader_cannot_outlive_the_deadline -- --nocapture
```

Local checks:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
scripts/check-local.sh
```

`scripts/check-local.sh` is the canonical gate and includes packaging and the Rust 1.89 MSRV lane. Hosted CI on the exact candidate remains required for closure. A later explicit user instruction authorized committing and pushing the implementation candidate solely to run CI; no remote workflow dispatch or publication occurred.

## 9. Acceptance criteria

| Criterion | Evidence |
|---|---|
| Zombie-only macOS failure remains tolerated without assuming the group is empty | deterministic initial-`EPERM`, exited-leader, retry-`ESRCH` coverage and end-to-end output-bound test |
| Live/unverified group cannot be accepted from leader exit alone | deterministic retry-`EPERM` failure and live/unverified-leader failure coverage |
| A live descendant is still targeted after leader exit | bounded process-group regression, with no leaked descendant |
| Empty group remains benign | real empty-group regression passes |
| Windows behavior is unchanged | cfg review and hosted Windows lane |
| API, manifest, and generated workflow bytes are unchanged | diff review and existing workspace tests |
| Normative text and citations match the implementation | source-anchor review of affected `architecture/` citations |
| Local gate is green | exact `scripts/check-local.sh` result |
| Candidate is cross-platform qualified before closure | hosted run on the exact candidate SHA, all four lanes green |

## 10. Stop conditions

Stop and keep the milestone active if the retry returns any non-`ESRCH` error, if a live or unverified group is accepted, if any descendant test leaks a process, if the Windows path changes, if the local gate fails, if architecture citations are not verified, or if hosted CI for the exact candidate is unavailable or red. Do not dispatch or publish. Commit/push requires explicit user authorization; the authorized follow-up push used for CI is documented in the closure record.

## 11. Closure evidence

Closure record: `plans/closure/ci-release-orchestration/003pa-status.md`. M003pa is closed on implementation commit `329a3dfdb166580de4770b2c6b2f046358983003` after `scripts/check-local.sh` returned 0 and hosted run `44118309795` passed all four lanes on that exact SHA. The record distinguishes implementation evidence, local verification, hosted results, and the remaining informational observation. The prior M003p plan and closure remain unchanged.