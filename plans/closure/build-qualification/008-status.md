# Build and Qualification Milestone 008 Closure — Compatibility Floor Verification Corrective

Status: closed

Source plan: `plans/implementation/build-qualification/008-compatibility-floor-verification-corrective.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Related decisions: `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md` (§9 requires the independent floor verification this milestone implements); `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` (**accepted**, Option A — this milestone discharges the residual acceptance set ADR-0006 required `eggpack-core` to consume).

Reviewed baseline: `9cf0bf8` (CI/Release M003l), on top of Build/Qualification M007 (`e3c6192`).

Implementation SHA: `7e7cb6a` — `feat(core): record compatibility floor verification status`.

## Executive finding

M008 is closed. A declared compatibility floor is no longer silently dropped. Every target that declares a floor now carries a finite, closed **floor verification status** on both the producer's intent artifact (`ReleasePlan`) and the evidence for the exact candidate bytes (`QualificationEvidence`), and the status is computed by **independent bounded inspection of the produced ELF** rather than implied by the build strategy.

The defect this corrects was specific and real: under `BuildStrategy::NativeCargo` a declared glibc floor was accepted at declaration and then dropped, and nothing downstream recorded that it went unverified. A native build on a modern host produces floor-compatible bytes by accident, and the producer's artifacts did not say so.

Nothing was weakened. The `cargo-zigbuild` triple rewrite is byte-identical, the macOS-under-zigbuild refusal is byte-identical and still fires at build-command time rather than moving to declaration time, and `ReleaseManifest` v1 and the published `eggpack-manifest` crate are untouched.

## The status model

A finite, closed enum — not a free-form string, not an `Option<bool>`. Every variant maps to one concrete observation:

| Status | Concrete observation |
|---|---|
| `NotDeclared` | the target declares `CompatibilityFloor::None` |
| `ToolchainBound` | the floor was bound into the build toolchain by the cargo-zigbuild triple rewrite, and the toolchain is trusted to honour it |
| `Verified` | the produced bytes were independently inspected and their highest required `GLIBC_x.y` version is not newer than the declared floor |
| `Unverified(reason)` | a floor was declared and the produced bytes did not confirm it, for a bounded reason |
| `NotSatisfied` | the produced bytes were independently inspected and require a newer runtime than the declared floor |

`FloorUnverifiedReason` is likewise closed: `PendingInspection`, `UnreadableCandidate`, `MalformedCandidate`, `NoVersionRequirement`, `UnrecognizedVersionRequirement`, `FloorKindNotVerifiable`.

The plan's §5.1 required **at least** `NotDeclared`, `ToolchainBound`, `Verified`, and `Unverified`. The fifth variant, `NotSatisfied`, exists because the plan's §7 requires a contradicted floor to **fail closed and MUST NOT be reported as `Unverified`**. Reporting a false declared floor as merely "unverified" would have been a weakening, so the contradiction is modelled as its own status and as its own hard failure.

## Production implementation evidence

| Location | Change |
|---|---|
| `crates/eggpack-core/src/floor.rs` (new, 1,260 lines) | The bounded ELF floor inspector and the status model: `FloorVerificationStatus` (`:72`), `FloorUnverifiedReason` (`:30`), `planned_floor_status` (`:105`) — the plan-time status, a pure function of the policy so a caller cannot choose it; `floor_status_admissible` (`:133`) — the plan/evidence cross-check; `inspect_candidate_floor` (`:591`); `aggregate_floor_status` (`:666`). |
| `crates/eggpack-core/src/floor.rs` | Bounded ELF reading: `MAX_ELF_HEADER` 64, `MAX_ELF_PROGRAM_HEADERS` 256, `MAX_ELF_DYNAMIC_ENTRIES` 1024, `MAX_ELF_VERSION_REQUIREMENTS` 256, `MAX_ELF_VERSION_AUXILIARIES` 256, `MAX_ELF_VERSION_NAME` 64. Every offset is checked against the file length and every read is exact-length, matching the existing `MAX_BINARY_HEADER` discipline. The reader locates `PT_DYNAMIC`, reads the dynamic table, and collects the `GLIBC_` symbol version names. |
| `crates/eggpack-core/src/lib.rs` | `PlannedTarget::floor_status` (`#[serde(default)]`) and `ReleasePlan::validate()` — public, and called by the plan's JSON serialization path. `resolve` derives the status from the policy. |
| `crates/eggpack-core/src/qualification.rs` | `QualificationEvidence::floor` records the status of the inspected bytes. `QualificationFailure::FloorNotSatisfied` is the hard failure for a contradicted floor. The inspection runs inside `inspect_candidate`, on the same canonical path the digest authority hashes, so the status is bound to the exact bytes that were digested. A contradicted floor short-circuits to `Failed(FloorNotSatisfied)` **before** any process executes. `validate_for` cross-checks the evidence's status against the plan's declared floor. |
| `crates/eggpack-core/src/qualification.rs:383` | The candidate digest acceptance check now calls `crate::validate_sha256_hex(&record.sha256).is_err()` instead of the inline 64-character lowercase-hex test. This discharges the residual ADR-0006 acceptance set. |
| `crates/eggpack-core/src/builder.rs`, `finalization.rs` | Fixture field additions only; the production `cargo_command` is untouched. |
| `crates/eggpack-ci/src/lib.rs` | The two named test fixtures only, one line each, adding `floor: eggpack_core::FloorVerificationStatus::NotDeclared`. No CI behaviour, rule, or message changed. |

## The comparison direction, and why it is asserted

The check compares `required_glibc <= declared_floor`. A binary requiring a **newer** glibc than declared does **not** satisfy the declared floor and fails it. The direction is the whole point of the rule, so it is asserted by test rather than left implicit.

The three comparison cases the plan names:

- required `2.34` against a declared `2.17` → `NotSatisfied` (fails the floor), **not** `Verified`;
- required `2.17` against a declared `2.17` → `Verified`;
- required older than declared → passes.

A candidate with **no** `GLIBC_` version requirement is compatible with any declared floor, but it does not *confirm* one, so it is recorded as `Unverified(NoVersionRequirement)` — a distinct result, not a generic `Verified` and not an error. The plan required exactly this distinction.

## ADR-0006 conformance — the residual acceptance set

M003l's implementation audit found a **fourth** SHA-256 acceptance set that the original review inventory missed: `crates/eggpack-core/src/qualification.rs` validated a candidate's recorded digest inline with a 64-character lowercase-hex check and therefore **accepted the all-zeros digest**.

ADR-0006 §5.1 item 1 requires the canonical SHA-256 hex validator — owned by `eggpack-manifest` — to be consumed by `eggpack-core`, `eggpack-ci`, and `eggpack-github`. This milestone owns `qualification.rs`, so it is here that the last in-core restatement of the rule was routed through `eggpack_core::validate_sha256_hex`.

The acceptance set before and after:

| | Before | After |
|---|---|---|
| 64 lowercase hex characters | accepted | accepted |
| all-zeros digest | **accepted** | **refused** |
| anything else | refused | refused |

The diagnostic text was not changed: the refusal still reads `"qualification evidence candidate identity is invalid"`.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §12.1 — every target declaring a floor carries a finite, machine-readable status on the plan and the evidence; a missing or contradictory status is a validation error | `PlannedTarget::floor_status`, `QualificationEvidence::floor`, `ReleasePlan::validate()`, `floor_status_admissible`. A plan that declares a floor and omits the status deserializes as `NotDeclared` and is **rejected**. |
| §12.2 — the floor is no longer silently dropped under `NativeCargo`; the declared floor and its status are both present | `planned_floor_status` derives the plan-time status from the policy, including under `NativeCargo`; the regression test asserts a `NativeCargo` target with a declared glibc floor still resolves and still produces a valid plan. |
| §12.3 — the maximum required `GLIBC_x.y` is computed by bounded inspection and compared in the correct direction; a binary requiring a newer glibc fails closed | The bounded ELF reader and the `required <= declared` comparison; `NotSatisfied` plus `QualificationFailure::FloorNotSatisfied`. |
| §12.4 — a malformed or unreadable binary yields `Unverified` with a reason, never `Verified` | `UnreadableCandidate` and `MalformedCandidate`; a non-ELF input, a truncated dynamic table, and a truncated program header each produce a reason. |
| §12.5 — `ReleaseManifest` v1 and the published `eggpack-manifest` crate are unchanged | `git show --stat 7e7cb6a` lists no `eggpack-manifest` path; no `ReleaseManifest` field, shape, or `schema_version` changed anywhere. |
| §12.6 — the `CargoZigbuild` triple rewrite and the `Macos`-under-zigbuild refusal are byte-unchanged | The production `cargo_command` is untouched; the regression test `the_cargo_zigbuild_floor_rewrite_and_macos_refusal_are_unchanged` pins the rewritten triple byte-for-byte and asserts the message `"macOS floor cannot use cargo-zigbuild"`. `validate_policy` still applies only the triple half of the floor rule. |
| §7 — a contradicted floor is a hard failure and MUST NOT be reported as `Unverified` | `NotSatisfied` is a distinct status and `FloorNotSatisfied` is a distinct failure, asserted by test. |
| §7 — an unverifiable floor is a recorded status, not a hard failure of the release | `Unverified(reason)` is recorded on the evidence and does not fail the release. |
| §8 — no new dependency | `Cargo.toml` is untouched in every crate; the ELF reading uses the standard library and the existing `sha2` surface. |
| §8 — `PackConfig` v1 accepts the same documents | A target that declares a glibc floor under `NativeCargo` still resolves; it now carries a status. |
| §3 invariant — all parsing is bounded | Every table and offset has a fixed bound, and every read is exact-length. |
| ADR-0006 §5.1 — `eggpack-core` consumes the canonical SHA-256 validator | `qualification.rs:383`. |

## Verification executed

All verification below ran against implementation SHA `7e7cb6a`, scoped to the actual change as the maintainer directed for this stage:

```text
cargo fmt --all -- --check                                                  passed
cargo check --workspace --all-targets --all-features --locked               passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-core --all-targets --all-features --locked             passed (81 passed, 0 failed, 6 ignored; 61/6 before M008)
cargo test -p eggpack-ci --all-targets --all-features --locked               passed (67 passed, 0 failed, 3 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked              passed (27 passed + 8 passed, 0 failed)
git diff --check                                                            passed
```

The `eggpack-cli` test run was added because M008 changed the `ReleasePlan` and `QualificationEvidence` shapes that the CLI serializes; it proves the CLI still round-trips them.

**Not run, by the maintainer's scope direction and by the absence of a push:** `./scripts/check-local.sh`, the `cargo +1.89.0` MSRV lane, the `cargo package` steps, and hosted CI. Plan §12.7 requires the full gate and a hosted run; neither was performed, and no hosted run is claimed here. The full local gate runs once at the end of the chain.

## Observable behaviour changes

1. **A glibc floor under `NativeCargo` is now independently inspected.** An ELF requiring a newer glibc than declared fails that target's qualification with `FloorNotSatisfied` instead of passing.
2. **`ReleasePlan` and `QualificationEvidence` JSON gain a `floor_status` / `floor` field.** A document that omits it deserializes as `NotDeclared` and is **rejected** when a floor is declared. A stale plan or evidence artifact produced before this commit now **fails closed**. Consumers that adopt the new shape must re-render and re-pin — this is recorded, as the plan's §8 requires.
3. **A `GLIBC_`-prefixed non-numeric version name** (for example `GLIBC_ABI_DT_RELR`) yields `Unverified(UnrecognizedVersionRequirement)`, never `Verified`.
4. **No refusal moved.** The macOS-under-zigbuild refusal still fires at *build-command* time with the byte-identical message `"macOS floor cannot use cargo-zigbuild"`. `validate_policy` still applies only the triple half of the floor rule, so `PackConfig::resolve` outcomes are unchanged.
5. `ReleaseManifest` v1 and `eggpack-manifest` are untouched.

## Compatibility and no-regression evidence

- **No `ReleaseManifest` change.** No field, no shape, no `schema_version` bump. The published `eggpack-manifest` crate needs no new release from this milestone.
- **No generated workflow byte changed.** No checked-in workflow declares a floor that changes meaning, so `eggpack ci check` drift status is unchanged for existing consumers.
- **No dependency added** in any crate.
- **No historical closure rewritten.** `plans/closure/build-qualification/001`–`007-status.md` and every other historical closure record are untouched.
- **Pre-existing tests stay green** across core, CI, and CLI. The CI crate's two test fixtures were the only CI-side change, one line each.

## Security and reliability review

- The change closes a **verification gap**, not a hole in a control. A declared floor previously produced no evidence at all under `NativeCargo`; it now produces evidence that says exactly what was and was not confirmed.
- The ELF reader is **bounded on every axis**: header size, program-header count, dynamic-entry count, version-requirement count, version-auxiliary count, and version-name length, with every offset checked against the file length and every read exact-length. A malformed or hostile binary cannot make the reader allocate or read without limit.
- The reader operates on the **same canonical path** the digest authority hashes, so the floor status and the recorded digest describe the same bytes.
- The comparison direction is asserted by test, so a future edit that inverts it fails the suite rather than silently passing a binary that requires a newer runtime than declared.
- No signature, provenance, or authenticity claim is introduced. The status is an **integrity fact about bytes against a declared policy**, not a claim about who produced them.
- No network, credential, privilege, archive, or publication surface is touched.

## Failure and recovery review

- A **contradicted** floor fails closed: the target's qualification fails with `FloorNotSatisfied` and no process executes against it.
- A merely **unverifiable** floor does not fail the release; it is recorded with a bounded reason, so a producer is not blocked by a binary it cannot inspect.
- A **missing** status is a validation error, not a default claim.
- A malformed, truncated, or non-ELF candidate yields `Unverified` with a reason, never `Verified`.
- The inspection is read-only and holds no lock; two concurrent inspections of the same candidate observe the same bytes or a changed-file error, each self-consistent. Restarting re-inspects from the beginning; no resumable state exists.
- Rollback is a revert of this single commit. The previously unrecorded floor returns, with no consumer impact beyond re-admitting an unverified floor silently.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, plan accuracy) | The plan's §2 evidence inventory did not name the fourth SHA-256 acceptance set in `crates/eggpack-core/src/qualification.rs`, which accepted the all-zeros digest and therefore contradicted the ADR-0006 decision this chain implements. | Routed through `eggpack_core::validate_sha256_hex` in this milestone, which owns that file. Recorded in the M003l closure record as well, so neither record is silently inaccurate about it. |
| Medium (pre-existing gap, now named) | ADR-0005 §9 required independent floor verification to be planned. It now is, for GNU/Linux ELF. **Mach-O deployment-target verification is not implemented**: a macOS floor yields `Unverified(FloorKindNotVerifiable)`. | Recorded as a **named follow-on**, not an implicit omission, as the plan's §15 requires. |
| Low (pre-existing gap, now named) | Publishing floor evidence to consumers is not implemented. No `ReleaseManifest` field was added, because doing so is an ADR-triggering compatibility decision and the plan's §13 names it as a stop condition. | Recorded as a **named follow-on** requiring its own ADR. |
| Operational boundary | Verification ran on Linux x86-64 only. The macOS and Windows lanes are covered by hosted CI only, and no hosted run was performed because nothing was pushed. | Accepted; the same boundary every prior milestone recorded. Plan §12.7 is therefore **not** claimed as met. |
| None | No unresolved medium-or-higher defect in the corrected producer contract. | M008 acceptance criteria satisfied except §12.7, which is out of the maintainer's scoped verification. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M008 | Reason |
|---|---|---|
| Build/Qualification M008 | closed | This record; implementation `7e7cb6a`. |
| Build/Qualification M009 | unblocked | Its plan-time surfaces (`ReleasePlan`, `QualificationEvidence`) are now stable, and its `plan` and `qualify` commands must call the new public `ReleasePlan::validate()`. |
| CI/Release M003m | ready | Unblocked by M003l; independent of this milestone. |
| CI/Release M003n | blocked | It follows CI M003m. |

## Downstream-unblock answer

Closing M008 unblocks **Build/Qualification M009**, which consumes the new `ReleasePlan::validate()` and the floor status shapes. It unblocks no CI milestone.

## Eggstack references

- Closed source plan: `plans/implementation/build-qualification/008-compatibility-floor-verification-corrective.md`
- Related accepted decisions: `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md` (§9), `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` (Option A)
- Predecessor in this chain: `plans/closure/build-qualification/007-status.md` (`e3c6192`), `plans/closure/ci-release-orchestration/003l-status.md` (`9cf0bf8`)
- Successor: `plans/implementation/build-qualification/009-local-producer-operator-commands.md`
- Named follow-ons: Mach-O deployment-target verification; publishing floor evidence to consumers (requires its own ADR).
