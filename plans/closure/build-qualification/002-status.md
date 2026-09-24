# Build and Qualification Milestone 002 Closure — Native/Cross Builder Execution Seam

Status: closed

Source plan: `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `e8bc338ab193fa8ed5fc7debb0cd68b54ebf586b` (Build/Qualification M001 closure baseline)

Implementation commits: `98b60e460e8424ab2b806ef969eb822769f5d8e6`, `156f746bae276429c616e58d6068fb95bd052dce`, `ff7cc5b1c4bd1bcae264e2f11a1670519508f588`, `f01322875334b16d0ca0f17ce74ca4f9e9a61acb`, `30a2cca3992579e5811a423ad007f883284b4ee5`, `cee288487cce0d45919d47f34f961c4cdf91ad03`, `f978aea6d5d8cd4d8a087aa1beea3f777629eefa`, `0abdc93b0997175225638289d4ca52cdca5cd24a`.

Hosted CI: [run 36031996802](https://github.com/eggstack/eggpack/actions/runs/36031996802) passed on Linux stable, Linux Rust 1.89, macOS, and Windows. Windows executed and passed both `real_local_cargo_fixture_builds_a_direct_candidate` and `timeout_kills_and_waits_for_the_process_group`; the `eggpack-core` suite reports 16 passed. A second run for the same pushed revision, [36031996579](https://github.com/eggstack/eggpack/actions/runs/36031996579), also passed all four lanes. Earlier linker failures and diagnostics are historical and resolved by the successful hosted execution evidence.

## Executive finding

The first-party Cargo builder seam is implemented, including strict producer bindings, native/cargo-zigbuild command specifications, preflight, bounded process execution, private target storage, exact candidate lookup, and structured build evidence. Stable, MSRV, macOS, and Windows hosted checks/tests pass. The successful Windows run executes both the real Cargo candidate smoke and Cargo-backed timeout/cancellation fixture, closing the previously observed toolchain qualification gap. M002 acceptance criteria are met.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Strict versioned bindings map contract logical slots to explicit package/bin sources | `BuildBindingsV1`, typed selectors, unknown-field rejection, and contract/ReleasePlan validation in `crates/eggpack-core/src/builder.rs`; unit regressions pass. |
| Deterministic native and cargo-zigbuild command intent, with tool preflight | Command builders use explicit toolchain, target, package/bin, locked release mode and glibc floor syntax; mismatch/missing-tool paths have regression coverage. |
| Shell-free bounded execution with timeout/cancellation cleanup | `command-group` process groups/Windows job support, bounded output/runtime, and typed outcomes are implemented. The timeout/cancellation fixture passes on Linux/macOS and Windows hosted lanes. |
| Private build target directories and exact candidate discovery | Invocation-owned target path/metadata and expected package/bin output lookup reject wrong, absent, empty, symlink, or non-regular candidates; tests pass on available lanes. |
| No final identity, qualification, finalization, or publication authority | `BuildAttempt`/`CandidateArtifact` describe candidate bytes only; no `Qualification::Native`, final filename, checksum, manifest, or publication result is emitted. |
| Rust stable, MSRV, macOS, Windows hosted qualification | Linux stable, Linux 1.89, macOS, and Windows pass in run 36031996802. Windows log confirms the direct Cargo smoke and timeout/cancellation fixture each ran and passed. |
| Cargo-zigbuild operational evidence | No cargo-zigbuild/Zig installation is assumed. Command/preflight semantics are tested; execution is recorded as unavailable as allowed by plan section 11. |

## Production implementation evidence

The public builder API is exported from `eggpack-core` (`BuildBindingsV1`, selectors, command/build result and cancellation types). Bindings remain producer-side and do not duplicate release filenames or install identities. NativeCargo and CargoZigbuild commands are explicit and shell-free. Preflight reports missing or mismatched tools before an accepted build attempt. Cargo output is directed to private per-invocation work paths. Candidate discovery derives one exact target path and does not scan or guess. `command-group = 5.0.1` supplies process-group/Windows job termination support. No contract/manifest wire changes were made.

## Verification executed

Local verification recorded during implementation: core stable tests (16 tests at the latest implementation checkpoint), core Rust 1.89 tests, core Clippy, prior full `scripts/check-local.sh`, formatting, package/docs/dependency-tree checks, and `git diff --check` passed. Hosted run 36031996802 passed Linux stable (including formatting, Clippy, and docs), Linux Rust 1.89, macOS, and Windows workspace checks/tests. Windows log confirms the real candidate smoke and process-tree timeout/cancellation test passed (16 core tests, 0 failures). Run 36031996579 also passed all four hosted jobs.

Earlier hosted Windows attempts failed while resolving `link.exe`; the successful runs above supersede that incomplete evidence. The final Windows tests were not skipped: both Cargo-backed smoke and timeout/cancellation tests are visible as passed in the hosted log.

## Invariant, failure/recovery, and security review

The implemented path launches fixed executables without a shell, bounds output and runtime, uses process-group/job cleanup, returns no candidate after unsuccessful outcomes, and does not expose raw environment dumps. Attempts own private target subdirectories; no automatic cleanup of arbitrary caller paths or retries occurs. No qualification, final artifact naming, release publication, or secret-bearing output is added. The original closure evidence included successful Windows execution; subsequent post-closure runs revealed intermittent Windows qualification instability, now tracked separately by M002a.

## Compatibility/migration review

The seam is additive to `eggpack-core`; DistributionContract and ReleaseManifest formats are unchanged. Existing repository release workflows remain untouched. No migration is needed. Build docs describe candidate bytes as unqualified intermediate output.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher builder safety/correctness finding. | M002 acceptance criteria pass. |
| Informational | cargo-zigbuild and Zig are not present in the standard image. | Command/preflight regressions provide the plan-authorized evidence; operational execution remains future environment qualification, as permitted by plan section 11. |

## Roadmap disposition and dependency transitions

M002 is closed. Build M003 qualification execution is unblocked and ready for planning because the builder emits bounded candidate evidence and all required host lanes pass. CI M001 is unblocked and ready for implementation; its renderer must reuse M002 bindings/command semantics. Build M004 remains blocked on M003 plus Manifest M002. CI M002/M003 remain blocked on their existing dependencies.

| Milestone | Status | Implementation plan | Closure record | Blocker |
|---|---|---|---|---|
| M002 native/cross builder seam | closed | `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md` | this record | Stable/MSRV/macOS/Windows hosted checks pass; Windows Cargo smoke and timeout/cancellation tests executed successfully. |
| M003 qualification execution | ready to plan | — | — | M002 candidate evidence interface closed |
| CI M001 CIPlan + GitHub renderer | ready | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | — | Consume M002 bindings and command intent; no duplicate builder semantics. |


## Post-closure corrective registration

Subsequent hosted evidence revealed that Windows qualification is not yet stable enough to treat a single green run as sufficient proof.

Observed after historical M002 closure:

- run 36035978249 at `00a3399773045121baabbe86f062d41a4c2ce1fb`: Windows workspace tests failed while Linux stable, Linux Rust 1.89, and macOS passed;
- run 36040032768 at `69baab129963892505f1034a605669b83bb59a5a`: all lanes passed;
- run 36040609714 at `f5bb416481b1aad528c982ce3fe3e757af061ada`: attempt 1 failed the Windows workspace tests, while rerunning only the failed Windows job as attempt 2 passed.

No deterministic production regression is established by this pattern, but the pass/fail/rerun-pass behavior is a qualification-stability finding.

Corrective plan: `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md`.

Historical M002 implementation and successful closure evidence above remain valid records of what passed at the time. M002a supplements them with stability/observability qualification. Build M003 readiness is withdrawn until M002a closes.

## M002a corrective closure annotation

M002a closed at implementation SHA `051f69b5e69df837db9a137ff5f29ba077d21029`; see `plans/closure/build-qualification/002a-status.md`. Historical failing Windows logs identify `builder::tests::timeout_kills_and_waits_for_the_process_group`, which returned `Failed(101)` rather than `TimedOut`. The old runner did not retain the child Cargo diagnostic text, so the exact underlying Cargo message cannot be reconstructed from those runs. The corrective addresses the observable runner/test gaps: CI initializes and verifies MSVC, the Windows smoke cannot silently skip, Cargo-backed timeout and cancellation use isolated target trees and wait until the build script starts, and all test temporary roots are unique across parallel and repeated invocations. Three first-attempt hosted runs on the corrective SHA pass. Build M003 is now ready to plan; M004 remains blocked on M003.
