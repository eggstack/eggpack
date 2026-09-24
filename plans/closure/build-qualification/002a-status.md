# Build and Qualification Milestone 002a Closure — Windows Builder Qualification Stability

Status: closed

Source plan: `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Historical M002 closure: `plans/closure/build-qualification/002-status.md`

Reviewed baseline: `f5bb416481b1aad528c982ce3fe3e757af061ada` (M002a corrective registered; subsequent failing Windows evidence retained in the historical M002 closure annotation).

Implementation SHA: `051f69b5e69df837db9a137ff5f29ba077d21029`.

Three independent first-attempt hosted runs on that exact SHA:

- [36056524223](https://github.com/eggstack/eggpack/actions/runs/36056524223) — workflow_dispatch, attempt 1, all lanes passed.
- [36056527046](https://github.com/eggstack/eggpack/actions/runs/36056527046) — workflow_dispatch, attempt 1, all lanes passed.
- [36056529888](https://github.com/eggstack/eggpack/actions/runs/36056529888) — workflow_dispatch, attempt 1, all lanes passed.

The ordinary push-triggered run [36056518550](https://github.com/eggstack/eggpack/actions/runs/36056518550) also passed, but is not counted in the three-run matrix.

## Finding and root cause disposition

The historical failing Windows test is `builder::tests::timeout_kills_and_waits_for_the_process_group`; its first-attempt log reports `Failed(101)` where the fixture expected `TimedOut`. The old process evidence retained only outcome and byte counts, not child Cargo stderr, so the subordinate Cargo diagnostic cannot be reconstructed. The intermittent failure is isolated to the Cargo-backed test harness and its Windows execution prerequisites; no production builder defect was reproduced.

The old Windows workflow did not initialize the Visual Studio developer environment. It also did not verify which `link.exe` Cargo would resolve. M002a now initializes the MSVC environment before the Windows qualification steps and fails explicitly unless `link.exe` resolves under `VCToolsInstallDir`. This makes the linker, SDK, and library preconditions visible and rejects Git for Windows' `usr/bin/link.exe` as MSVC. The old Cargo timeout fixture also relied on a one-second deadline, inherited the shared Cargo target directory, and did not synchronize cancellation with build-script startup. These conditions are now deterministic: test roots are created uniquely per invocation, Cargo target directories are test-owned, and timeout/cancellation are each exercised three times after the fixture build script signals that it is running.

This evidence supports a runner/test-isolation cause rather than a production-code failure. The missing child diagnostic limits attribution to the observed failing test and its uninitialized/unchecked runner environment; it does not support a more specific claim about the exact historical linker or Cargo message.

## Corrective changes

- `.github/workflows/ci.yml` exposes `workflow_dispatch` so independent same-SHA qualification runs can be initiated. On Windows it initializes the MSVC developer environment, verifies the actual linker path, runs the two named M002 acceptance tests explicitly, runs serialized core tests and the CI crate tests, then retains the ordinary workspace test lane.
- `real_local_cargo_fixture_builds_a_direct_candidate` no longer returns success when an MSVC linker is missing. Required qualification now fails at the explicit CI prerequisite or during the real Cargo smoke.
- All `eggpack-core` test temporary roots use a process ID plus an atomic counter and atomic directory creation. Repeated and parallel generation is covered by a regression test; stale roots cannot be silently reused.
- The Cargo-backed timeout/cancellation fixture uses offline, test-owned Cargo target directories. It waits for a build-script marker before cancellation, repeats timeout and cancellation three times, waits for cleanup, and removes each fixture tree immediately.
- `crates/eggpack-core/README.md` documents the Windows developer-environment requirement.
- No production builder code, API, wire/config semantics, or ADR-0004 boundary changed.

## Hosted evidence

Each of the three qualifying runs used SHA `051f69b5e69df837db9a137ff5f29ba077d21029`, event `workflow_dispatch`, and attempt number 1. In every run the Windows job passed:

1. `Verify initialized MSVC linker environment`;
2. `Windows builder real Cargo candidate smoke`;
3. `Windows builder process-group timeout and cancellation`;
4. `Windows core tests serialized for diagnosis`;
5. `Windows CI crate tests`;
6. `cargo check --workspace --all-targets --locked`;
7. `cargo test --workspace --all-targets --all-features --locked`.

The core test suite includes the real direct candidate smoke and the repeated process-group timeout/cancellation fixture. Neither required test is filtered out or silently skipped. Linux stable, Linux Rust 1.89, and macOS also passed in each run. The historical failed runs remain recorded in the M002 closure; no retry is counted as a qualifying pass.

## Local verification

All required local verification completed successfully:

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1  17 passed
cargo test -p eggpack-core --all-targets --all-features --locked       17 passed
cargo test --workspace --all-targets --all-features --locked           93 passed
cargo doc --workspace --no-deps --locked                               passed
cargo tree -p eggpack-core --locked                                    passed; dependency set unchanged
cargo package -p eggpack-core --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'  passed
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-core --all-targets --locked              17 passed
./scripts/check-local.sh                                               passed
git diff --check                                                       passed
```

## Findings and dependency transitions

No unresolved medium-or-higher builder qualification or stability finding remains. The remaining historical limitation is that the exact Cargo stderr from the original `Failed(101)` event was not retained; the clean repeated Windows matrix verifies the deterministic corrected path.

| Milestone | Status after M002a | Reason |
|---|---|---|
| Build/Qualification M002 | closed (historical) | Original builder interface and evidence remain intact; M002a closes the later stability finding. |
| Build/Qualification M002a | closed | Three independent first-attempt Windows qualification runs passed on one SHA. |
| Build/Qualification M003 qualification execution | ready to plan | Stable M002 candidate evidence and Windows qualification are available. |
| Build/Qualification M004 finalization/aggregation | blocked | Requires M003 closure and Manifest M002. |
| CI Orchestration M001 | closed | Its deterministic CIPlan/renderer evidence is unchanged. |
| CI Orchestration M002 qualification/aggregation gates + drift CLI | blocked | Requires future Build M003/M004 interfaces. |
| CI Orchestration M003 draft release staging | blocked | Requires CI M002 and an explicit staging adapter plan. |

The next producer handoff is Build/Qualification M003 planning. No plan beyond M003 can be unblocked by this corrective alone.
