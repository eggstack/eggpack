# CI/Release M003pb Closure — Live-Descendant Fixture Startup Handshake

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003pb-process-group-descendant-fixture-startup-handshake-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `eef4f9d0be4014dc0e76489de3f8fe9aedacadb9` — the committed parent named by the corrective plan.

Implementation commit: `edce774bf3597a37789ba59587cc2a5be56e3cee` (`chore: refresh dependencies and fix process-group fixture`), pushed on `feat/auto-20261117-f257539b` and merged by PR [#6](https://github.com/eggstack/eggpack/pull/6). Merge commit: `23ebb10941bb2b38b808f61edddff3ed3d1f2961`.

The implementation commit and merge commit have the same source tree `880352cfea924f0434e4b176cbcf8a81b7fdbd3c`. Thus the exact-source push and pull-request CI below exercise the merged tree as well.

## Hosted CI

Both workflows on the exact implementation SHA completed successfully on all required lanes:

- [Push run 44935151048](https://github.com/eggstack/eggpack/actions/runs/44935151048) — Linux stable, Linux 1.89.0, macOS, and Windows passed.
- [Pull-request run 44935155340](https://github.com/eggstack/eggpack/actions/runs/44935155340) — Linux stable, Linux 1.89.0, macOS, and Windows passed.

The merged commit was then tested by [main push run 44941146518](https://github.com/eggstack/eggpack/actions/runs/44941146518), conclusion `success`, on exact SHA `23ebb10941bb2b38b808f61edddff3ed3d1f2961`; all four lanes passed.

## Executive finding

M003pb is closed. The live-descendant test fixture now makes its process-group leader wait for the descendant-ready marker before exiting. This removes the startup race that previously allowed the test to check for a child before the child had started. The descendant’s release-file handshake, bounded waits, group-termination assertion, and cleanup remain in place.

The correction is test-only. Production process-group signaling, retry behavior, and failure classification are unchanged. The original failure was a fixture-startup assertion, not evidence of a production runner defect.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Leader does not exit before the descendant is ready | Updated fixture in `crates/eggpack-core/src/builder.rs`; the leader waits for the ready marker before exiting | pass |
| The descendant remains live until group termination and cannot write its survivor marker afterward | `terminating_an_exited_leader_still_signals_a_live_group_descendant`, with the existing release-file handshake and bounded cleanup | pass |
| No process leak or unbounded wait | Five consecutive focused runs; the existing bounded deadline and cleanup paths remain unchanged | pass |
| Production process-runner semantics remain unchanged | Reviewed diff: the change is confined to the Unix test fixture in `builder.rs`; no production signal/retry code changed | pass |
| Required broader process-group regression filter passes | `cargo test -p eggpack-core --all-targets --all-features --locked group_signal -- --nocapture` — 1 passed, 0 failed | pass |
| Formatting and whitespace checks pass | `cargo fmt --all -- --check`; `git diff --check` | pass |
| Canonical local gate passes | `scripts/check-local.sh`, exit code 0 | pass |
| Exact candidate passes hosted CI | Runs 44935151048 and 44935155340 on `edce774bf3597a37789ba59587cc2a5be56e3cee`; all four lanes green | pass |
| Merged main tree remains green | Run 44941146518 on `23ebb10941bb2b38b808f61edddff3ed3d1f2961`; all four lanes green and tree identical to the implementation commit | pass |

## Verification executed

The focused regression passed five consecutive times:

```sh
for attempt in 1 2 3 4 5; do cargo test -p eggpack-core --all-targets --all-features --locked terminating_an_exited_leader_still_signals_a_live_group_descendant -- --nocapture || exit $?; done
```

The broader process-group filter passed:

```sh
cargo test -p eggpack-core --all-targets --all-features --locked group_signal -- --nocapture
```

Also passed: `cargo fmt --all -- --check`, `git diff --check`, and the canonical `scripts/check-local.sh` gate with exit code 0. The gate includes workspace checks, lint, tests, docs, dependency trees, local package checks, and the Rust 1.89 MSRV lane. Its `cargo package --allow-dirty` commands were local verification only; they are not release publication checks.

## Production implementation evidence

None. The only process-group change is in the test fixture in `crates/eggpack-core/src/builder.rs`; no production code was changed. The same commit separately refreshes compatible lockfile resolutions and corrects stale GitHub architecture citations, as authorized in the preceding task. Those edits do not broaden M003pb’s runtime scope.

## Invariant review

- Process-group signaling and M003pa retry/error behavior are unchanged.
- The test still proves descendant readiness, live-descendant termination, and survivor-marker prevention.
- Waits and cleanup remain bounded.
- No API, schema, package manifest, workflow, or publication behavior changed as part of M003pb.
- The compatible lockfile refresh was separate from the test-fixture corrective; no dependency declaration was added for M003pb.

## Failure and recovery review

The original local-gate failure occurred because the test leader exited immediately after starting a background shell, before the child-ready marker was guaranteed. Five repeated focused runs, the broader `group_signal` filter, the full local gate, exact-candidate push/PR CI, and the merged-main CI all pass after the handshake. No process leak or remaining test failure was observed. GitHub emitted a non-blocking Node.js 20 deprecation annotation for `actions/checkout@v4`; it did not fail a lane.

## Compatibility, security, and operations review

No public API, wire format, manifest schema, dependency declaration, network behavior, credential handling, workflow permission, tag, or release changed. No credentials or captured process output were exposed. This corrective does not itself authorize publication; the separately user-authorized workspace release is tracked under `plans/implementation/workspace-release/001-eggpack-0.1.2-crates-and-github-source-release.md`.

## Unresolved findings

No blocker or medium/high-severity finding remains. The CI action deprecation annotation is informational and unrelated to the test fixture.

## Roadmap disposition and registry updates

M003pb is closed in the CI/release roadmap and registry. Its closure records the exact implementation SHA, local gate, exact-SHA hosted runs, and merged-main hosted run. M003pa and earlier closure records remain unchanged. The next registered work is the separately authorized Workspace Release M001; no package, tag, or GitHub release has been created by this corrective.
