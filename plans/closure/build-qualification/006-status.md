# Build and Qualification Milestone 006 Closure — Native Qualification for Cross-Tool Builds

Status: closed

Source plan: `plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Accepted decision: `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md` (accepted, Option A)

Reviewed baseline: `28f3630413c1fa6ae35ca1fdfc404a64b30b3b88` (the Ecosystem M001 §20 stop-condition baseline; production code had not advanced beyond it before this implementation — only planning-only descendants existed).

Implementation SHAs:

- `472d825d0bcec1034cf796ada135a800474f8f09` — `feat: implement Build M006 native qualification for cross-tool builds` (production changes, tests, and documentation);
- `398cd43bf1597ba49bfc35b5334611aa04b16600` — `test: cover the public qualify_target path for cross-tool-built candidates` (adds the public-entry-point assertion to the M006 qualification test; no production change).

Hosted CI on the final implementation SHA `398cd43`: [36484758546](https://github.com/eggstack/eggpack/actions/runs/36484758546), completed successfully (push event): `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, and `portability (windows-latest)` all success on the first attempt. The intermediate implementation SHA `472d825` was also qualified green on all four lanes by [36484521327](https://github.com/eggstack/eggpack/actions/runs/36484521327). Closure edits touch plans only; no production code changed after either run.

## Executive finding

M006 is closed. `BuildStrategy` (how candidate bytes are produced) and `Qualification` (how those exact bytes are proved) are now genuinely independent producer axes, as accepted by ADR-0005 Option A. A `CargoZigbuild` target for an explicit glibc floor may declare `Qualification::Native` and is admissible exactly when its effective qualification host — `qualification_host`, else the build host — matches the target's OS and architecture.

Nothing was weakened to achieve this. The host match is still required at declaration (`validate_policy`, `CIPlan::validate`) and at execution (`qualify_target`); a non-matching host still produces a failed `HostMismatch` evidence record with no executed process rather than a pass or a skip; every executing classification still requires a bounded smoke binding; `NativeCargo` still rejects any `cargo_zigbuild`/`zig` version; glibc/macOS floor applicability and the `CargoZigbuild` + macOS-floor rejection are unchanged. No `Qualification` or `QualificationMethod` variant was added (Option D was not needed), no schema version was bumped, and no checked-in golden changed.

The first proving consumer shape is proven inside this repository: an eggsact-shaped five-target reusable release workflow renders and passes `eggpack ci check` with zero drift, and a Linux AArch64 candidate cross-built on Linux x86-64 is handed off unchanged and qualified natively on a separate Linux AArch64 runner.

## Production implementation evidence

Exactly three production edits, all removals of the strategy term:

| Location | Change |
|---|---|
| `crates/eggpack-core/src/lib.rs` (`validate_policy`) | Dropped `policy.strategy == BuildStrategy::CargoZigbuild ||` from the `Qualification::Native` guard; kept `host_matches_target` and the effective-host resolution unchanged. Error text updated from "native qualification requires a matching native build/qualification host" to "native qualification requires a qualification host matching the target OS/arch", because the strategy term was the inaccurate part. |
| `crates/eggpack-ci/src/lib.rs` (`CIPlan::validate`) | `native_qualification_valid` is now computed from `host_matches_target(qualification_host.unwrap_or({host_os, host_arch}), target)` alone. Canonical ordering, per-strategy toolchain-version grammar, floor applicability, the `CargoZigbuild` + macOS-floor rejection, and every handoff/command/aggregation equality check are untouched. |
| `crates/eggpack-core/src/qualification.rs` (`qualify_target_for_host_with_runner`) | Removed the pre-execution guard `if target.policy.qualification == Qualification::Native && target.policy.strategy == BuildStrategy::CargoZigbuild { return Err("Native qualification cannot use CargoZigbuild") }`. See finding F-1. |

Retained, deliberately unchanged: `QualificationBindingsV1::validate_for`'s smoke-required rule for `Native | DeferredNative | Emulated`; the `Qualification::Native` execution branch's `host != required || !target_matches_host(...)` → failed `HostMismatch` guard; the `NativeCargo` cross-tool rejection in both declaration validators; `CompatibilityFloor` applicability in both validators; the QEMU-sysroot-requires-`Emulated` rule; the native direct-exec command construction (candidate path + fixed argv, no shell, cleared environment).

Documentation: `crates/eggpack-core/README.md` (independent axes, host-matched native qualification, and the compatibility-floor evidence boundary), `crates/eggpack-ci/README.md` (qualification jobs run on the runner mapped to the effective qualification host, independent of strategy), and `architecture/core.md` (`TargetPolicy` axis note plus corrected `BuildStrategy`/qualification producer rule and the explicit statement that native execution evidence is not independent proof of a declared floor). No canonical planning document, ADR, or historical closure was rewritten.

## Admissibility change, before and after

| Strategy | Qualification | Before (`28f3630`) | After (`398cd43`) |
|---|---|---|---|
| `NativeCargo` | `Native`, matching host | admitted | admitted (unchanged) |
| `NativeCargo` | `Native`, non-matching host | rejected | rejected (unchanged) |
| `CargoZigbuild` | `Native`, matching host | **rejected** ("native qualification requires a matching native build/qualification host") | **admitted**; the resolved `ReleasePlan` preserves the strategy, `Qualification::Native`, the glibc floor, and the exact `cargo_zigbuild`/`zig` versions |
| `CargoZigbuild` | `Native`, non-matching host | rejected | rejected (unchanged) |

Non-`Native` classifications (`DeferredNative`, `Emulated`, `Structural`) are unchanged for both strategies, and `NativeCargo` with any cross-tool version remains rejected for every classification.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §7.1 core policy validation: keep host check, drop strategy term, update error text | `crates/eggpack-core/src/lib.rs`; the term is gone, the message names the real requirement, and the `NativeCargo` cross-tool rejection and floor applicability are byte-identical. Proven by `native_qualification_is_admitted_for_cross_tool_builds_on_a_matching_host` plus the still-green `zig_version_is_required_for_zigbuild_and_forbidden_for_native`. |
| §7.2 CI graph validation: `native_qualification_valid` from host match alone, all surrounding guards unchanged | `crates/eggpack-ci/src/lib.rs` `CIPlan::validate`; proven by `m006_eggsact_five_target_matrix_renders_and_checks_without_drift` (all five graph jobs validate) and `m006_native_qualification_still_fails_closed` (every surrounding guard still rejects). |
| §7.3 documentation: corrected rule + compatibility-floor evidence boundary | `crates/eggpack-core/README.md`, `crates/eggpack-ci/README.md`, `architecture/core.md`; the floor boundary is stated in the crate README, the architecture boundaries section, and the `TargetPolicy::floor` rustdoc. |
| §4 invariant: qualification never executes on a non-matching host, at declaration or at run time | Declaration: `host_matches_target` in both validators. Run time: the retained execution branch. Proven by the `mismatched` assertion (failed `HostMismatch`, `processes.is_empty()`) in `cross_tool_built_candidate_qualifies_natively_on_the_matching_host`. |
| §4 invariant: every executing classification still requires a bounded smoke binding | `QualificationBindingsV1::validate_for` untouched; proven by the `without_smoke` assertions in the same test and by `m006_native_qualification_still_fails_closed` (render rejected when a `Native` target has no smoke). |
| §4 invariant: `NativeCargo` targets must still reject any cross-tool version | Unchanged code; the three-combination loop in the new core test and the pre-existing `zig_version_is_required_for_zigbuild_and_forbidden_for_native` both assert rejection. |
| §4 invariant: glibc/macOS floor applicability unchanged | Unchanged code; new coverage for glibc-on-Darwin, macOS-floor-on-Linux, and `CargoZigbuild` + macOS floor in the core and CI tests. |
| §4 invariant: general producer capability, not a per-consumer special case | The change is two boolean-term removals plus one guard removal. No eggsact name, target, or version appears in any production code path; the new fixture contract is a test input. |
| §6 out of scope: no `Qualification`/`QualificationMethod` growth | Neither enum gained a variant. |
| §6 out of scope: no independent ELF GLIBC-symbol-floor verification | Not attempted; recorded as residual risk below and in the M006 documentation. |

## §8 acceptance test matrix

| # | Acceptance case | Where | Result |
|---|---|---|---|
| A1 | `CargoZigbuild` + `Qualification::Native` PackConfig with a matching host resolves, and the resulting `ReleasePlan` keeps the glibc floor and exact cross-tool versions | `crates/eggpack-core/src/lib.rs::native_qualification_is_admitted_for_cross_tool_builds_on_a_matching_host` | pass — resolved policy re-asserts `CargoZigbuild`, `Native`, `Glibc{2,17}`, `cargo_zigbuild = 0.23.3`, `zig = 0.14.1`; an explicit matching `qualification_host` resolves to the same effective host |
| A2 | A five-target eggsact-shaped `ReleaseWorkflowShapeV1` (two CargoZigbuild + native, three NativeCargo + native) renders a reusable release workflow and passes `ci check` with zero drift | `crates/eggpack-ci/src/lib.rs::m006_eggsact_five_target_matrix_renders_and_checks_without_drift` (library render + `check_reusable_release_github`) and `crates/eggpack-cli/src/main.rs::shape_generate_and_check_accept_native_qualification_for_cross_tool_targets` (real `ci generate` + `ci check` entry points) | pass — deterministic re-render is byte-identical; `DriftReport { matches: true, expected_bytes == actual_bytes, first_difference: None }`; CLI `ci generate` output equals the library renderer and `ci check` returns success |
| A3 | `qualify_target` on a CargoZigbuild-built candidate at a matching native host yields `QualificationMethod::Native`, `QualificationStatus::Passed`, and executed process outcomes | `crates/eggpack-core/src/qualification.rs::cross_tool_built_candidate_qualifies_natively_on_the_matching_host` | pass — both the injected-host seam and the public `qualify_target(QualificationRequest { .. })` entry point return `Passed` + `QualificationMethod::Native` with exactly one `candidate_smoke` process whose outcome is `CommandOutcome::Success`; evidence passes `QualificationEvidence::validate_for`; serialized evidence contains no local path |
| A4 | Split-host topology: an `aarch64-unknown-linux-gnu` target built with `CargoZigbuild` on Linux x86-64 resolves to a separate Linux AArch64 qualification job, transfers the canonical candidate handoff, and executes the exact candidate natively there | `m006_eggsact_five_target_matrix_renders_and_checks_without_drift` (graph projection + rendered topology) | pass — `build_aarch64_unknown_linux_gnu` runs on `ubuntu-latest` and runs `cargo +1.89.0 zigbuild --release --locked --target aarch64-unknown-linux-gnu.2.17`; `qualify_build_aarch64_unknown_linux_gnu` `needs` that build job, runs on `ubuntu-24.04-arm`, downloads `eggpack-build-handoff-aarch64-unknown-linux-gnu`, and invokes `_qualify-target --target aarch64-unknown-linux-gnu`; the qualification job contains no cross-tool provisioning |

## §8 rejection test matrix (each still fails closed)

| # | Rejection case | Where | Result |
|---|---|---|---|
| R1 | `CargoZigbuild` + `Native` with a mismatched qualification host architecture | core test (`Aarch64` for x86-64 target, and `Aarch64` build host with no override) + `m006_native_qualification_still_fails_closed` | pass — resolve/render rejected |
| R2 | `CargoZigbuild` + `Native` with a mismatched host OS | core test (`macos`/`x86_64` for a GNU/Linux target) + `m006_native_qualification_still_fails_closed` | pass — resolve/render rejected |
| R3 | The split-host AArch64 case with Linux x86-64 as the qualification host | `m006_native_qualification_still_fails_closed` (Linux x86-64, Linux armv7, Windows aarch64) | pass — `PackConfig::resolve` errors and `render_reusable_release_github` errors in all three cases |
| R4 | `Qualification::Native` without a smoke binding | `cross_tool_built_candidate_qualifies_natively_on_the_matching_host` (`without_smoke`) + `m006_native_qualification_still_fails_closed` | pass — `QualificationBindingsV1::validate_for` errors and `qualify_target` errors; reusable render errors |
| R5 | `NativeCargo` with `cargo_zigbuild` and/or `zig` set | core test (all three combinations) + pre-existing `zig_version_is_required_for_zigbuild_and_forbidden_for_native` + `m006_native_qualification_still_fails_closed` | pass — rejected in every case, for `Native` and non-`Native` classifications alike |
| R6 | glibc floor on a non-GNU/Linux target; macOS floor on a non-Darwin target | core test (glibc on `aarch64-apple-darwin`, macOS floor on the GNU/Linux target) + `m006_native_qualification_still_fails_closed` | pass — rejected |
| R7 | `CargoZigbuild` combined with a macOS floor | `m006_native_qualification_still_fails_closed` | pass — `CIPlan::validate` rejects |
| R8 | `Structural` with a smoke binding | `m006_native_qualification_still_fails_closed` (render rejected); pre-existing `classification_requires_the_exact_smoke_and_host_configuration` and `m002_graph_projection_preserves_m001_and_validates_bindings` unchanged | pass — rejected; with the smoke removed the same target renders, keeps `cargo zigbuild` in its build job, and its qualification job carries no cross tools |
| R9 | `QualificationState` forging / host mismatch at run time | `m006_native_qualification_still_fails_closed` execution branch; pre-existing `native_host_mismatch_is_a_failure_and_never_runs_smoke` | pass — a mismatching architecture produces `Failed(HostMismatch)` with no executed process, and that failure record is itself valid evidence |

## Eggsact-shaped render evidence

Test input: new contract fixture `crates/eggpack-contract/tests/fixtures/eggsact-direct-targets.toml` (five direct targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`). The shape uses eggsact's exact cross-tool pair and digests (Zig 0.14.1, cargo-zigbuild 0.23.3, the M005 official archive digests) in provisioned mode, `ubuntu-latest` / `ubuntu-24.04-arm` / `macos-14` / `macos-13` / `windows-latest` runners, one bounded `--version` smoke per target, one `Python3` consumer validator per target, and a GitHub-draft staging intent.

Observed rendered topology (abridged from the test's parsed job assertions):

```text
build_aarch64_unknown_linux_gnu        needs: resolve                 runs-on: ubuntu-latest
  cargo +1.89.0 zigbuild --release --locked --target aarch64-unknown-linux-gnu.2.17 --package eggsact --bin eggsact
  install exact cargo-zigbuild 0.23.3 + SHA-256-verified Zig 0.14.1 (M005 provisioning unchanged)
  upload eggpack-build-handoff-aarch64-unknown-linux-gnu
qualify_build_aarch64_unknown_linux_gnu  needs: build_aarch64_unknown_linux_gnu  runs-on: ubuntu-24.04-arm
  download eggpack-build-handoff-aarch64-unknown-linux-gnu -> ./eggpack-handoff/build_aarch64_unknown_linux_gnu
  eggpack ci _qualify-target ... --target aarch64-unknown-linux-gnu ...   (no cross tools in this job)
  upload eggpack-evidence-aarch64-unknown-linux-gnu
build_x86_64_unknown_linux_gnu          needs: resolve                 runs-on: ubuntu-latest
  cargo +1.89.0 zigbuild --release --locked --target x86_64-unknown-linux-gnu.2.17 --package eggsact --bin eggsact
qualify_build_x86_64_unknown_linux_gnu  needs: build_x86_64_unknown_linux_gnu  runs-on: ubuntu-latest
qualify_build_aarch64_apple_darwin      runs-on: macos-14
qualify_build_x86_64_apple_darwin       runs-on: macos-13
qualify_build_x86_64_pc_windows_msvc    runs-on: windows-latest
required_gate / aggregate / stage       (stage is the only `contents: write` job)
```

Every `CIPlan` job in the projected graph carries `qualification.classification = native` with `qualification.state = unresolved`, and every `QualificationJob.host` equals the target's effective qualification host and matches the target triple. `ci check` on the generated bytes reports `matches: true` with equal byte counts and no first difference. Least privilege is unchanged: only `stage` has `contents: write`, and there is no `gh release`, `publish`, or `id-token: write`.

## Verification executed

All verification below ran against implementation SHA `398cd43`:

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test --workspace --all-targets --all-features --locked           passed (211 passed, 7 ignored; 206/7 before M006)
cargo doc --workspace --no-deps --locked                               passed
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-contract --all-targets --locked         passed
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked         passed
cargo +1.89.0 test -p eggpack-core --all-targets --locked              passed
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked         passed
cargo +1.89.0 test -p eggpack-github --all-targets --locked           passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                passed
cargo +1.89.0 test -p eggpack-cli --all-targets --locked               passed
./scripts/check-local.sh                                               passed (exit 0, includes cargo package for every crate)
git diff --check                                                       passed
```

The five new tests are `native_qualification_is_admitted_for_cross_tool_builds_on_a_matching_host` and `cross_tool_built_candidate_qualifies_natively_on_the_matching_host` (eggpack-core), `m006_eggsact_five_target_matrix_renders_and_checks_without_drift` and `m006_native_qualification_still_fails_closed` (eggpack-ci), and `shape_generate_and_check_accept_native_qualification_for_cross_tool_targets` (eggpack-cli).

Hosted CI run 36484758546 on the exact implementation SHA passed all four lanes on the first attempt: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

## Compatibility and no-regression evidence

- **No schema version bump.** `PackConfig`, `ReleasePlan`, `CIPlan`, `ReleaseWorkflowShapeV1`, `QualificationBindingsV1`, and `QualificationEvidence` all remain schema version 1; no new serialized field was added anywhere.
- **No golden changed.** `native-direct.yml`, `native-direct-multitarget.yml`, `m002-direct.yml`, `m002-bundle.yml`, `m002-archive.yml`, `m002-mixed.yml`, and the three `m003b-*-staging.yml` files are byte-identical; none of the `#[ignore]`d regenerate helpers was run. This is expected: no in-tree document declares `Native` together with `CargoZigbuild`, so no existing document changes meaning.
- **No historical closure rewritten.** `plans/closure/build-qualification/001`–`005-status.md` and every other historical closure record are untouched.
- **Pre-existing tests stay green**, including the cross-tool rejection test the plan names explicitly (`zig_version_is_required_for_zigbuild_and_forbidden_for_native`), all M001/M002/M002a/M003/M003a/M003b/M003c/M003d suites, both M005 cross-tool suites, and the `NativeCargo` no-provisioning render test.

## Security and reliability review

The change relaxes a *declaration* predicate; it does not change what is executed, where, or with which credentials. No new network, archive, credential, privilege, or publication surface is introduced, and no shell, argv, or environment DSL is added.

- The risk introduced is declaring an inappropriate qualification host, not executing on one. Both declaration validators and the execution branch still require the host to match the target OS/architecture, and a non-matching host yields a failed `HostMismatch` record rather than a pass or a skip.
- The candidate that executes is still the exact validated candidate: identity, format, architecture, size, and SHA-256 are checked before execution and re-hashed after (`CandidateChanged` on any difference), so a cross-built candidate cannot be substituted between build and qualification.
- Execution remains bounded and shell-free: direct exec of the candidate path with fixed argv, cleared environment, timeout and stdout/stderr limits, process-group cleanup, typed `SmokeFailed`/`SmokeTimedOut`/`OutputLimitExceeded`/cancellation outcomes, and no output contents in evidence.
- Generated workflows are unchanged in privilege shape: qualification and consumer jobs keep `contents: read`, `stage` remains the only writer, `concurrency` is unchanged, and there is no `gh release`, `publish`, or `id-token: write`. The five-target render asserts this directly.
- Evidence remains free of local paths, output contents, and environment values; the new test asserts the serialized evidence contains no candidate path.

## Failure and recovery review

- A producer that misconfigures a split-host topology fails closed at declaration (no plan, no workflow) rather than executing on the wrong host: `m006_native_qualification_still_fails_closed` proves three distinct host mismatches are rejected by both `PackConfig::resolve` and the renderer.
- A misconfigured smoke binding fails closed at binding validation and at execution (`qualify_target` errors rather than executing without a smoke).
- A cross-tool build that produces a structurally wrong or wrong-architecture candidate still fails through `inspect_candidate` with `StructuralMismatch` before any execution; M006 changed nothing on that path.
- A candidate that mutates during execution still fails with `CandidateChanged`, and a non-zero/timed-out/cancelled smoke still fails with the typed outcomes asserted by `native_smoke_failure_timeout_cancellation_and_output_limit_are_typed`.
- Recovery is a producer-configuration edit only: no migration, no data conversion, no state to reconcile, and no partially-applied document, because no checked-in document changed meaning. Rollback is a revert of the three production edits; the previously stricter behavior returns with no consumer impact (no in-tree document relied on the relaxed rule).
- There is no contention or restart hazard: the milestone adds no new external service, cache, or shared state.



| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, plan accuracy) | The plan §1 and ADR §2 diagnosis stated that the execution path "already supports the combination" because `eggpack_core::qualification` "consults `policy.strategy` nowhere in the `Qualification::Native` branch". That is true of the branch, but `qualify_target` had a separate pre-execution guard — `if target.policy.qualification == Qualification::Native && target.policy.strategy == BuildStrategy::CargoZigbuild { return Err("Native qualification cannot use CargoZigbuild") }` — which would have rejected every case the plan's own §8/A3 acceptance criterion requires. | Removed as part of this milestone, because A3 could not otherwise be satisfied. The blast radius stayed inside the accepted rule: the removed guard duplicated the declaration validator's term and nothing else. No new ADR or corrective is required, but the ADR/plan diagnosis was one guard short of the full picture and any future change in this area should audit execution-path guards, not just the two declaration validators. Recorded here so the record is not silently inaccurate. |
| None | No unresolved medium-or-higher defect in the corrected producer contract. | M006 acceptance criteria satisfied. |
| Operational boundary | Eggpack hosted CI still has no Linux AArch64 runner, so the split-host AArch64 topology is proven by unit/fixture/render evidence (graph projection, runner selection, handoff transfer, `runs-on` mapping) and by an executed native smoke on the lane-native host — not by an actual hosted AArch64 run from this repository. | Recorded per the same boundary M005 recorded. The first real Linux x86-64/AArch64 runs belong to eggsact M001 and are not claimed here. |
| Operational condition | No real cross-tool build, Zig download, or `ubuntu-24.04-arm` execution occurred during Eggpack verification (unit tests are network-free and host-local by design). | SHA/archive/provisioning assumptions remain covered by M005 fixtures; the native smoke is executed for real on the local host with the exact candidate bytes. |
| Environment | Verification ran on Linux x86-64 only; the macOS and Windows lanes are covered by hosted CI only. | Accepted; the same boundary every prior milestone recorded. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M006 | Reason |
|---|---|---|
| Build/Qualification M006 | closed | This record; implementation `398cd43`; hosted run 36484758546 green. The build/qualification milestone chain M001-M006 is now closed. |
| Ecosystem adoption M001 (eggsact) | ready (unblocked) | Its §20 stop condition is resolved by this closure. Remaining gates are its own §23 consumer-baseline re-review, the `eggpack_tool.revision` re-pin to `398cd43`, re-running its §7 configuration through the real renderer, and one maintainer-authorized real release tag. No eggsact repository change is claimed by this closure. |
| CI M003b live draft qualification | conditionally closed, no longer producer-blocked | Its sole remaining condition is the real draft/rerun proof inside M001. Build M006 was the producer blocker and is closed. |
| Bootstrap M003 | blocked (unchanged) | Independent adoption evidence/candidate review remains outstanding. |
| Eggup Interoperability M003 | ready to plan (unchanged) | Independent of this seam. |
| Ecosystem M002 stegoeggo and later | blocked (unchanged) | Still require M001 closure evidence; the shared native-qualification gap they expected M006 to resolve is now resolved. |

## Downstream-unblock answer

Closing M006 unblocks **Ecosystem M001** and nothing else. Because M001 has not itself closed, no later ecosystem milestone, Phase 8 exit, or Bootstrap M003 is unblocked by this pass. `plans/registry.md`, the build-qualification, CI/release-orchestration, and ecosystem-adoption roadmaps, and the M001 plan status have been updated to match; no historical closure was rewritten.

## Residual risk: compatibility-floor evidence boundary

M006 deliberately does **not** treat native execution on a modern matching host as proof of a declared compatibility floor. A glibc 2.17 floor is a build-policy input carried into `cargo zigbuild --target <triple>.2.17`; the native smoke on `ubuntu-latest` or `ubuntu-24.04-arm` proves that the exact candidate runs on that host, which may be much newer than the declared minimum runtime. This is stated in the crate README, `architecture/core.md`, the `TargetPolicy::floor` rustdoc, and the test assertions (the smoke argv is a bounded `--version`; nothing inspects GLIBC symbol versions).

Independent verification that the final ELF requires no GLIBC symbol version newer than the declared floor remains unplanned and unclaimed. It is a separate qualification-hardening milestone that should be planned before Eggpack advertises direct evidence for a floor; per planning process §8 it requires its own plan and closure record, and it is not a precondition for M001.

## Eggstack references

- Accepted decision: `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md`
- Closed source plan: `plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md`
- Unblocked consumer plan: `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md` (§24)
- Related closed work: `plans/closure/build-qualification/005-status.md` (cross-tool provisioning this milestone composes with), `plans/closure/ci-release-orchestration/003d-status.md` (consumer composition seam)
- First proving consumer baseline (not migrated by this closure): `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`, since advanced to `34aed3ab36da2637c22412f7ca65d35f1ca5021d` and flagged for re-review by M001 §23.
