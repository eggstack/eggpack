# CI/Release M003pa Closure — Process-Group Signal Recheck

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003pa-process-group-signal-recheck-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `1cd6d0bf43c93dc9fd572c2d69a3a675a2958b09` — the committed parent immediately before the M003pa implementation.

Implementation commit: `329a3dfdb166580de4770b2c6b2f046358983003` (`fix: recheck process-group signals after leader exit`), pushed to `feat/auto-20261117-f257539b`.

Hosted CI for the exact implementation SHA: [run 44118309795](https://github.com/eggstack/eggpack/actions/runs/44118309795), push event, attempt 1, completed successfully. All four required jobs reported success:

| Lane | Result | Job |
|---|---|---|
| `linux (stable)` | success | [134162378950](https://github.com/eggstack/eggpack/actions/runs/44118309795/job/134162378950) |
| `linux (1.89.0)` | success | [134162379725](https://github.com/eggstack/eggpack/actions/runs/44118309795/job/134162379725) |
| `portability (macos-latest)` | success | [134162379635](https://github.com/eggstack/eggpack/actions/runs/44118309795/job/134162379635) |
| `portability (windows-latest)` | success | [134162379578](https://github.com/eggstack/eggpack/actions/runs/44118309795/job/134162379578) |

The closure/status update is documentation-only; no production code changed after the implementation run.

## Executive finding

M003pa is closed. A failed Unix process-group signal is considered resolved only when the initial error is `ESRCH`, or when an exited/reaped leader is followed by a successful group-signal retry or retry `ESRCH`. A live or unverified leader and every other retry error remain hard failures. This does not infer that the group is empty from leader exit or pipe EOF.

The implementation and regressions are in `crates/eggpack-core/src/builder.rs`. The retry helper is covered for initial `ESRCH`, a live/unverified leader, retry success, retry `ESRCH`, and a hard retry error. The bounded live-descendant test uses a release-file handshake to prove that a surviving descendant would write a marker; group termination prevents that marker. The real empty-group and output-overflow runner regressions remain covered.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Initial `ESRCH` remains benign and does not retry | `a_failed_group_signal_is_benign_only_after_recheck_proves_termination` | pass |
| A live or unverified leader cannot authorize a retry or benign result | Same helper test; wait errors/non-exited status remain hard failures | pass |
| After leader reaping, only retry success or retry `ESRCH` is benign | Same helper test covers success, `ESRCH`, and retry `EPERM` | pass |
| A live descendant is still targeted after the leader exits | `terminating_an_exited_leader_still_signals_a_live_group_descendant`, using the release-file handshake | pass |
| A real already-exited/empty process group remains benign | `terminating_an_already_exited_process_group_is_not_a_failure` | pass |
| Actual output-overflow runner path remains bounded and fails | `stdout_overflow_is_bounded_and_not_success` | pass |
| Windows job-object behavior is unchanged | Unix-only retry handling; hosted Windows lane, including process-group timeout/cancellation and serialized core tests | pass |
| No public API, manifest/schema, dependency, or generated-workflow change | Reviewed implementation diff; only the private core runner/tests and documentation/planning files changed | pass |
| Normative documentation and affected citations match the implementation | `architecture/process-execution.md` and affected architecture citations rechecked by symbol; `eggpack-core` count is 9,109 lines (`builder.rs` 2,078; `digest.rs` 325) | pass |
| Local canonical gate is green | `scripts/check-local.sh`, exit 0; details below | pass |
| Exact candidate is cross-platform qualified | Hosted run `44118309795` on SHA `329a3dfdb166580de4770b2c6b2f046358983003`; all four jobs above | pass |

## Verification executed

Focused process-group checks passed:

```text
cargo test -p eggpack-core --all-targets --all-features --locked group_signal -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked terminating_an_already_exited_process_group_is_not_a_failure -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked terminating_an_exited_leader_still_signals_a_live_group_descendant -- --nocapture
cargo test -p eggpack-core --all-targets --all-features --locked stdout_overflow_is_bounded_and_not_success -- --nocapture
```

The canonical `scripts/check-local.sh` gate passed with exit code 0. It includes formatting, workspace check/lint/tests, documentation, dependency-tree/package checks, and the Rust 1.89 MSRV lane. It ran in the worktree before the implementation commit; the eight unrelated pre-existing edits were present but excluded from commit `329a3df`. Hosted CI then tested the exact committed SHA on all four required lanes.

## Production implementation evidence

`group_signal_failure_is_benign` in `crates/eggpack-core/src/builder.rs` now accepts a lazy retry closure. It returns benign immediately only for initial `ESRCH`; otherwise the Unix termination path first calls `try_wait`, and invokes the retry only if the leader is observed exited and reaped. Retry success or `ESRCH` is accepted; any other retry error preserves the hard failure. The non-Unix path retains the existing job-object behavior.

## Invariant review

- Process-group termination remains the authority; leader status and pipe state do not inventory descendants.
- A failed signal is never downgraded based solely on leader exit.
- All non-`ESRCH` retry failures remain fail-closed.
- Existing timeout, cancellation, output-limit classification, bounded execution, and output redaction are unchanged.
- M003p's plan and closure are unchanged; M003pa is recorded as a separate corrective.

## Failure and recovery review

The original intermittent output-limit failure was not reproduced during the corrective work; the exact original errno/interleaving remains unobserved. A diagnostic had observed `EPERM` while the direct child had exited, but did not establish whether the process group was empty. The corrective addresses that uncertainty conservatively by re-signalling the group after reaping the leader and accepting only success or `ESRCH`.

A triplicated full-core stress attempt under artificial concurrent load produced native qualification fixture `SmokeTimedOut` failures. The normal canonical gate and the exact-candidate hosted workspace tests passed; this stress-only timeout is not attributed to the M003pa process-group change. No descendant leak or hosted failure remains unresolved.

## Compatibility and migration review

No public API, wire format, manifest schema, dependency, workflow, or migration changed. The change is private to Unix process-group termination and its tests; the Windows path is unchanged and passed its hosted lane. Existing external consumers require no action.

## Security review

No new privilege, network, credential, publication, or shell surface was introduced. The runner continues to signal the supervised process group, preserve bounded execution/error handling, and avoid returning captured output contents. Retry failure remains fail-closed.

## Documentation and operations evidence

The normative process-execution deep dive and affected citations were updated. The overview, CI consumer seam, CLI, validation, and testing/portability references were checked; M003pa status was reconciled in the registry, roadmap, and `AGENTS.md`. The prior M003p history remains intact.

## Unresolved findings

- **Informational:** the precise original intermittent errno/interleaving could not be reproduced. No behavior remains unverified for the acceptance criteria: helper outcomes, descendant termination, the real output-overflow path, local gate, and all four hosted lanes passed.
- No blocker or high/medium-severity finding remains.

## Roadmap disposition and registry updates

M003pa is closed in the CI/release roadmap and registry, with this record linked from the plan. No implementation milestone is active or registered ready; the next action is plan authoring, not implementation. No dependent or cross-repository work is unlocked by this process-runner correction. M003p's historical plan and closure are unchanged.
