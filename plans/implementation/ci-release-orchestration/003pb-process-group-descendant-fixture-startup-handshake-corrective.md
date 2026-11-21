# CI/Release M003pb — Process-Group Descendant Fixture Startup Handshake Corrective

Status: active

Corrects: CI/Release M003pa, closed at implementation 329a3dfdb166580de4770b2c6b2f046358983003 and recorded in plans/closure/ci-release-orchestration/003pa-status.md. That closure remains historical and unchanged.

Repository baseline (committed HEAD): eef4f9d0be4014dc0e76489de3f8fe9aedacadb9, branch feat/auto-20261117-f257539b. The current worktree also contains the separately requested lockfile update and architecture/github.md citation corrections; they are outside this corrective.

Roadmap: plans/subsystems/ci-release-orchestration-roadmap.md

Classification: infrastructure / test reliability. No production capability or process-runner behavior changes.

## 1. Objective

Make the live-descendant process-group regression deterministic by ensuring the group leader exits only after its background descendant has written the descendant-started marker. The descendant must then remain alive awaiting the existing release-file handshake until the test terminates the group. Preserve bounded waits and cleanup so a failed fixture cannot leak a process.

## 2. Why ready and current evidence

The user's request authorizes fixing the local validation failure and revalidating locally and through hosted CI. No schema, API, dependency, or cross-repository prerequisite is involved. Commit/push remains separately gated on explicit user authorization.

The saved scripts/check-local.sh output exited 101 in the core unit tests at terminating_an_exited_leader_still_signals_a_live_group_descendant, failing the assertion that the descendant fixture had started (crates/eggpack-core/src/builder.rs:1776). The same focused test passed in isolation. The fixture starts a background shell and immediately exits its leader; the Rust test only then waits a bounded interval for the child's start marker. Scheduling delay can therefore yield a false test failure before the termination behavior is meaningfully exercised. There is no evidence that the production retry logic failed. After the handshake edit, the focused test passed five consecutive runs, formatting and git diff --check passed, and scripts/check-local.sh exited 0. Hosted CI remains pending explicit commit/push authorization.

## 3. Invariants

- The production process-group signal/retry implementation and its fail-closed behavior remain unchanged.
- The test proves the descendant is alive after leader exit and before group termination; a release marker still detects an incorrectly surviving descendant.
- Every wait remains bounded; cleanup runs on both success and failure paths; no process is intentionally orphaned.
- No public API, schema, dependency, workflow, release artifact, or publication behavior changes.

## 4. Scope

### In scope

- Strengthen only the Unix test fixture in crates/eggpack-core/src/builder.rs so the leader waits for the descendant-start marker before exiting.
- Keep the existing bounded deadline, release-file handshake, termination assertions, and cleanup; adjust the timeout only if focused evidence demonstrates it is too tight.
- Recheck architecture citations into builder.rs by named construct after the edit; preserve line count if practical. The one-line replacement preserves the file line count, and no architecture citation targets the changed line or a range containing it; the nearby test anchor at builder.rs:1835 remains unchanged.
- Record this corrective in the CI/release roadmap, registry, and AGENTS.md without rewriting M003pa's plan or closure.

### Out of scope

- Any changes to production runner or signal semantics, the generated workflow, dependency versions, schemas, or user-facing release behavior.
- Package publication, tags, releases, or remote workflow dispatch.
- Commit or push until explicit user authorization is received.

## 5. Ordered work packages

1. Update the fixture command so the background descendant writes its ready marker and waits on the release file, while the group leader waits for the ready marker before exiting.
2. Run the focused regression repeatedly; confirm it observes a successful leader exit, a started live descendant, successful group termination, and no survivor marker. Confirm failure paths still terminate/release the fixture.
3. Reverify source citations, run formatting and the canonical local gate, then request authorization before pushing the exact candidate for hosted CI.

## 6. Failure, restart, and compatibility

If startup does not complete before the existing leader deadline, the test must terminate the group, release the fixture if needed, clean temporary files, and fail diagnostically rather than hang or leak. If the focused test exposes a production process-runner failure, stop this test-only corrective and register a separately scoped implementation plan; do not silently change runtime semantics here. There is no migration or compatibility impact.

## 7. Exact verification

Run the focused regression five consecutive times with this Bash loop:

for attempt in 1 2 3 4 5; do cargo test -p eggpack-core --all-targets --all-features --locked terminating_an_exited_leader_still_signals_a_live_group_descendant -- --nocapture || exit; done

Then run:

cargo test -p eggpack-core --all-targets --all-features --locked group_signal -- --nocapture
cargo fmt --all -- --check
scripts/check-local.sh

The canonical gate includes workspace checks, clippy, tests, docs, packaging, and the Rust 1.89 MSRV lane. Hosted CI on the exact implementation SHA (Linux stable, Linux 1.89.0, macOS, Windows) is required for closure, but may be triggered only after explicit authorization to commit and push. Passing older hosted runs on M003pa is not evidence for this candidate.

## 8. Documentation, acceptance, and stop conditions

The normative process-execution contract does not change; update the corrective plan, roadmap, registry, and current handoff only. Acceptance requires: the fixture cannot exit its leader before the ready marker exists; five consecutive focused runs pass without a survivor; all architecture citations into the touched source remain accurate; scripts/check-local.sh exits 0; and exact-SHA hosted CI passes all four lanes before closure. Stop if there is any process leak, focused failure, production behavior change, local-gate failure, inaccurate citation, red hosted lane, or missing authorization. Do not close the milestone until all required evidence is recorded.

Closure evidence will be recorded at plans/closure/ci-release-orchestration/003pb-status.md, preserving this plan and all earlier closure records.
