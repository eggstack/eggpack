# Build and Qualification Milestone 002 Closure Review — Native/Cross Builder Execution Seam

Status: blocked; closure criteria not met

Source plan: `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `e8bc338ab193fa8ed5fc7debb0cd68b54ebf586b` (Build/Qualification M001 closure baseline)

Implementation commits: `98b60e460e8424ab2b806ef969eb822769f5d8e6`, `156f746bae276429c616e58d6068fb95bd052dce`, `ff7cc5b1c4bd1bcae264e2f11a1670519508f588`, `f01322875334b16d0ca0f17ce74ca4f9e9a61acb`, `30a2cca3992579e5811a423ad007f883284b4ee5`, `cee288487cce0d45919d47f34f961c4cdf91ad03`, `f978aea6d5d8cd4d8a087aa1beea3f777629eefa`, `0abdc93b0997175225638289d4ca52cdca5cd24a`.

Hosted CI: [run 36030970954](https://github.com/eggstack/eggpack/actions/runs/36030970954) failed on Windows; Linux stable, Linux Rust 1.89, and macOS passed. Earlier Windows diagnostic run [36030511727](https://github.com/eggstack/eggpack/actions/runs/36030511727) confirmed no `VCToolsInstallDir`, no linker override, no `RUSTFLAGS`, and only `C:\Program Files\Git\usr\bin\link.exe` in the discovered linker directories.

## Executive finding

The first-party Cargo builder seam is implemented, including strict producer bindings, native/cargo-zigbuild command specifications, preflight, bounded process execution, private target storage, exact candidate lookup, and structured build evidence. Linux stable, Linux Rust 1.89, and macOS hosted lanes pass. M002 cannot close because the required Windows real-Cargo and Cargo-backed process-tree timeout/cancellation evidence cannot run on the current hosted runner: Rust resolves Git's unrelated `link.exe`, and no Visual Studio linker environment is exposed. The implementation does not skip or weaken those required tests. This environment issue blocks the milestone's explicit cross-platform acceptance criterion.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Strict versioned bindings map contract logical slots to explicit package/bin sources | `BuildBindingsV1`, typed selectors, unknown-field rejection, and contract/ReleasePlan validation in `crates/eggpack-core/src/builder.rs`; unit regressions pass. |
| Deterministic native and cargo-zigbuild command intent, with tool preflight | Command builders use explicit toolchain, target, package/bin, locked release mode and glibc floor syntax; mismatch/missing-tool paths have regression coverage. |
| Shell-free bounded execution with timeout/cancellation cleanup | `command-group` process groups/Windows job support, bounded output/runtime, and typed outcomes are implemented. Linux/macOS/local tests exercise the runner. Windows Cargo-backed timeout/cancellation remains unqualified because the fixture cannot link. |
| Private build target directories and exact candidate discovery | Invocation-owned target path/metadata and expected package/bin output lookup reject wrong, absent, empty, symlink, or non-regular candidates; tests pass on available lanes. |
| No final identity, qualification, finalization, or publication authority | `BuildAttempt`/`CandidateArtifact` describe candidate bytes only; no `Qualification::Native`, final filename, checksum, manifest, or publication result is emitted. |
| Rust stable, MSRV, macOS, Windows hosted qualification | Linux stable, Linux 1.89, and macOS pass in run 36030970954. Windows fails due to missing MSVC linker on the runner before real Cargo builder and timeout fixture execution. Criterion unmet. |
| Cargo-zigbuild operational evidence | No cargo-zigbuild/Zig installation is assumed. Command/preflight semantics are tested; execution is recorded as unavailable as allowed by plan section 11. |

## Production implementation evidence

The public builder API is exported from `eggpack-core` (`BuildBindingsV1`, selectors, command/build result and cancellation types). Bindings remain producer-side and do not duplicate release filenames or install identities. NativeCargo and CargoZigbuild commands are explicit and shell-free. Preflight reports missing or mismatched tools before an accepted build attempt. Cargo output is directed to private per-invocation work paths. Candidate discovery derives one exact target path and does not scan or guess. `command-group = 5.0.1` supplies process-group/Windows job termination support. No contract/manifest wire changes were made.

## Verification executed

Local verification recorded during implementation: core stable tests (16 tests at the latest implementation checkpoint), core Rust 1.89 tests, core Clippy, prior full `scripts/check-local.sh`, formatting, package/docs/dependency-tree checks, and `git diff --check` passed. Workspace stable and Rust 1.89 hosted checks/tests passed on Linux; macOS hosted workspace checks/tests passed in run 36030970954.

Hosted Windows initially failed resolving Git's `link.exe` as a linker. Subsequent diagnostic and linker-discovery changes confirmed the hosted image does not expose the Visual Studio developer environment. The latest run 36030970954 still fails on the Windows Cargo-backed timeout fixture after the direct smoke test was guarded. The fixture was not skipped because doing so would hide the required timeout/cancellation evidence. No green Windows result is claimed.

## Invariant, failure/recovery, and security review

The implemented path launches fixed executables without a shell, bounds output and runtime, uses process-group/job cleanup, returns no candidate after unsuccessful outcomes, and does not expose raw environment dumps. Attempts own private target subdirectories; no automatic cleanup of arbitrary caller paths or retries occurs. No qualification, final artifact naming, release publication, or secret-bearing output is added. The unresolved concern is verification coverage on Windows, not a known code defect. Recovery requires provisioning a Windows runner with a valid MSVC toolchain exposed to the workflow (or changing the runner/toolchain policy through an explicit decision), then rerunning the full matrix and confirming both native smoke and timeout/cancellation tests execute and pass.

## Compatibility/migration review

The seam is additive to `eggpack-core`; DistributionContract and ReleaseManifest formats are unchanged. Existing repository release workflows remain untouched. No migration is needed. Build docs describe candidate bytes as unqualified intermediate output.

## Unresolved findings

| Severity | Finding | Required disposition |
|---|---|---|
| Blocking | Current Windows hosted workflow does not provide a usable MSVC linker to Cargo; required native Cargo and Cargo-backed timeout/cancellation evidence cannot pass. | Provision/configure a valid Visual Studio C++ build environment for the Windows lane, or explicitly revise supported-host/qualification policy; rerun Windows CI before closure. |
| Informational | cargo-zigbuild and Zig are not present in the standard image. | Command/preflight regressions provide the plan-authorized evidence; operational execution remains future environment qualification. |

## Roadmap disposition and dependency transitions

M002 remains blocked and has no successful closure. Build M003 qualification execution remains blocked on M002 candidate evidence plus completion of platform qualification. CI M001 is blocked because its GitHub build rendering must consume this shared builder interface and the Windows builder lane cannot currently execute; do not start its implementation until the platform execution boundary is resolved. Build M004 and CI M002/M003 remain blocked on their existing dependencies. This is the unforeseen issue specified by the user's stop condition; implementation stops here for reassessment.

| Milestone | Status | Implementation plan | Closure record | Blocker |
|---|---|---|---|---|
| M002 native/cross builder seam | blocked | `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md` | this record (blocked review, not closure) | Windows runner lacks usable MSVC linker; required Cargo execution and timeout/cancellation evidence fail. |
| M003 qualification execution | blocked | — | — | M002 closure and qualified candidate evidence interface |
| CI M001 CIPlan + GitHub renderer | blocked | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | — | M002 Windows builder execution qualification |
