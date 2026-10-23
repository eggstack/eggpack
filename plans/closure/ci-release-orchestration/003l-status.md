# CI/Release Milestone 003l Closure — Producer Rule Authority Convergence Corrective

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003l-producer-rule-authority-convergence-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Accepted decision: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` (**accepted**, Option A, 2026-10-23)

Reviewed baseline: `a0fec98` (`origin/main` — the merge of `eggstack/eggpack#1` (CI M003k) and `eggstack/eggpack#2`).

Implementation SHA: `9cf0bf8` — `fix(ci): converge producer rule authority onto core and manifest`.

Predecessor in this chain: Build/Qualification M007 (`e3c6192`), which delivered the canonical file-digest authority this milestone consumes.

## Executive finding

M003l is closed. The workspace now carries **one** SHA-256 hex acceptance rule, **one** host/target match rule, **one** floor-applicability rule, **one** exact-tag safety rule, and **one** file-digest authority. Five divergent copies across `eggpack-core`, `eggpack-ci`, and `eggpack-github` were deleted and replaced by delegation to their owners.

The acceptance set that ADR-0006 decided is now real: the all-zeros sentinel is refused by every producer path, including the published `eggpack-manifest` crate, which carries the rule in a `0.1.1` patch release. `ReleaseManifest v1` did not change — no field, no shape, no `schema_version` — so a consumer that reads real producer output is unaffected. A consumer that *emits or accepts* an all-zeros digest will now fail validation, and `eggup-eggpack` is named as the affected downstream consumer.

Nothing was weakened. Every rule kept its refusal set, and where a rule moved between crates the diagnostic wording stayed byte-identical so that a consumer grepping a CI log line does not see it change.

## Production implementation evidence

### `eggpack-manifest` — the SHA-256 acceptance set (ADR-0006 §5.1 item 1)

| Location | Change |
|---|---|
| `crates/eggpack-manifest/src/lib.rs` | `pub enum Sha256Error` (`:56`) with `NotLowercaseHex` (`:60`) and `AllZeros` (`:62`); `pub fn validate_sha256_hex(digest: &str) -> Result<(), Sha256Error>` (`:98`) — the single SHA-256 hex acceptance rule: exactly 64 lowercase hex characters, and never the all-zeros digest. |
| `crates/eggpack-manifest/src/lib.rs` | `ArtifactRecord::validate`, `ByteEvidence::validate`, and `decode_sha256` now route through `validate_sha256_hex` instead of restating the rule. |
| `crates/eggpack-manifest/Cargo.toml` | Explicit `version = "0.1.1"`, with a comment recording why the crate carries its own patch version while `[workspace.package] version` stays `0.1.0` for the other six crates. The dependents' `version = "0.1.0"` requirement is unchanged: `^0.1.0` permits `0.1.1`. |
| `Cargo.lock` | Refreshed for the version change. `cargo package -p eggpack-manifest --locked --allow-dirty` verifies `eggpack-manifest v0.1.1` successfully. |

### `eggpack-core` — the canonical producer rule surface

| Location | Change |
|---|---|
| `crates/eggpack-core/src/lib.rs` | `pub fn host_matches_target(host: HostRequirement, triple: &str) -> bool` (`:90`) — was private. The single host/target match rule. |
| `crates/eggpack-core/src/lib.rs` | `pub enum FloorApplicabilityError` (`:126`) with `NotGlibcTarget`, `NotMacosTarget`, `MacosFloorUnderZigbuild`; `pub fn validate_floor_applicability(floor, strategy, triple)` (`:186`) — the single floor-applicability rule, composed of a triple half and a strategy half. |
| `crates/eggpack-core/src/lib.rs` | `pub fn validate_policy(policy, triple) -> Result<(), CoreError>` (`:206`) — was private. The single policy validation rule. |
| `crates/eggpack-core/src/lib.rs` | `pub enum ExactTagSafetyError` (`:274`) with `OutOfBounds`, `ControlCharacters`, `QueryOrInjectionCharacters`, `UnsafePathShape`, `UnsafeGitRefPath`; `pub fn validate_exact_tag_safety(tag) -> Result<(), ExactTagSafetyError>` (`:313`) — the single exact-tag **safety** rule, carrying the union of the two former concern sets. |
| `crates/eggpack-core/src/lib.rs` | `pub use eggpack_manifest::{validate_sha256_hex, Sha256Error};` (`:18`). |
| `crates/eggpack-core/src/builder.rs` | `run_git_tag_bounded` now calls `crate::validate_exact_tag_safety(tag)` instead of its own inline condition list. `cargo_command` now calls `crate::validate_floor_applicability(...)` for the macOS-floor-under-zigbuild refusal. **Both were in scope by substance even though they live in `builder.rs` rather than `lib.rs`.** |

### `eggpack-ci` — five copies deleted

Deleted: the CI crate's private `host_matches_target`, its floor/strategy validity helpers (`floor_valid`, `toolchain_valid`, `native_qualification_valid`, the CargoZigbuild/macOS check), its independent SHA-256 validator body, its independent exact-tag safety body, and its own file-digest implementation.

What remains in `eggpack-ci` are **thin delegations that keep the crate's own diagnostic wording**:

| Location | Change |
|---|---|
| `crates/eggpack-ci/src/lib.rs:589` | `pub fn validate_sha256_hex` now applies `eggpack_core::validate_sha256_hex` and maps the refusal to the CI crate's own `"invalid SHA-256 digest"`. |
| `crates/eggpack-ci/src/lib.rs:4639` | `fn hash_candidate_file` now returns `Result<(u64, String), eggpack_core::DigestError>` and delegates to `eggpack_core::file_digest`. **There is no longer a `(0, String::new())` sentinel success value and no empty digest on any path.** |
| `crates/eggpack-ci/src/lib.rs:4886` | `fn validate_exact_tag` delegates to `eggpack_core::validate_exact_tag_safety` and maps each refusal class to the CI crate's existing message wording. |
| `crates/eggpack-ci/src/lib.rs` | `CIPlan::validate` applies the core policy validation instead of re-deriving the host match and the floor rules. |

### `eggpack-github` — two copies deleted

| Location | Change |
|---|---|
| `crates/eggpack-github/src/lib.rs:571` | `fn validate_sha256` delegates to `eggpack_manifest::validate_sha256_hex` and keeps the crate's own wording, adding `"sha256 must not be the all-zeros digest"` for the new refusal class. |
| `crates/eggpack-github/src/lib.rs:604` | `fn digest_file` delegates to `eggpack_core::file_digest` and maps each canonical refusal to the crate's existing staging diagnostics (`staging_digest_message`). |
| `crates/eggpack-github/src/lib.rs` | `validate_tag` delegates to the canonical exact-tag safety rule. |

## The exact-tag safety rule — the union, proven

Neither former validator was a superset of the other, so the canonical rule is the **union**, not "the same rule twice":

- empty; length bound; control characters — from both;
- `?`, space, `\` — from both;
- `..`; leading and trailing `/` — from both;
- `#`, `@`, `'`, `"`, `` ` ``, `$` — from the CI concern set;
- trailing `.`; `@{`; `^`, `:`, `*`, `[`, `~` — from the core concern set;
- path components that are empty, begin with `.`, or end with `.lock` — from the core concern set.

The canonical validator is **safety only**. The M003j canonical-**format** check (`vX.Y.Z`) remains a separate function with a separate diagnostic; the two were not merged.

## The one behaviour change a caller can observe

**The core git-resolution path now refuses tags containing `#`, `@`, `'`, `"`, `` ` ``, or `$` that it previously accepted.** This is fail-closed and correct — those characters are special to git and to shells — but it is a change on a path exercised by live consumer releases.

Grep evidence that no in-tree tag, workflow input, or consumer configuration is affected:

- `git tag -l` in this repository lists exactly one tag: `eggpack-manifest-v0.1.0`.
- The in-tree tag literals are `v1.2.3`, `v1.2.4`, `v1.2.5`, `v1.2.6`, `v2.4.0`, `v3.1.0`, the negative test `v1?x`, and the malformed `vX`.
- `release_tag` is a required input with no default, so no workflow supplies an implicit tag.
- No checked-in fixture carries a tag value.

The other observable changes are narrowings in the same direction: the CI crate's consumer-evidence digest check and expected-digest check now refuse an all-zeros digest; `eggpack-manifest`'s `sha256_bytes` refuses one; and `CIPlan::validate` now enforces core's stricter complete-version rule for the `cargo_zigbuild` and `zig` toolchain versions instead of the CI crate's own looser reading.

**Not changed:** `eggpack-core`'s `PackConfig::resolve` outcome. The macOS-floor-under-zigbuild refusal stays at build-command derivation, and `validate_policy` continues to apply only the triple half of the floor rule, so a producer that declares a macOS floor under `CargoZigbuild` still resolves and is still refused at the same point with the same message.

## Diagnostic compatibility

No message text was removed. Messages that moved between crates kept their wording:

- `"glibc floor applies only to GNU/Linux targets"`, `"macOS floor applies only to macOS targets"`, and `"macOS floor cannot use cargo-zigbuild"` are unchanged in core.
- `"invalid SHA-256 digest"` in the CI crate and `"sha256 must be 64 lowercase hex characters"` in the GitHub and manifest crates are unchanged.
- The CI crate's exact-tag messages (`"exact tag out of bounds"`, `"exact tag contains control characters"`, `"exact tag contains query/injection characters"`, `"exact tag has unsafe path shape"`, `"exact tag has unsafe git ref path shape"`) are unchanged.
- The GitHub crate's staging digest messages (`"staging file is unavailable"`, `"staging file must be a non-empty regular file"`, `"staging file cannot be read"`, `"staging file size exceeds the platform bound"`, `"staging file changed during read"`) are unchanged.

New message classes: `"sha256 must not be the all-zeros digest"` in the manifest and GitHub crates, and the GitHub crate's mapping of the two canonical tag refusal classes onto its own wording.

## Verification executed

All verification below ran against implementation SHA `9cf0bf8`, scoped to the actual change as the maintainer directed for this stage:

```text
cargo fmt --all -- --check                                                  passed
cargo check --workspace --all-targets --all-features --locked               passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-manifest --all-targets --all-features --locked        passed (28 passed, 0 failed)
cargo test -p eggpack-ci --all-targets --all-features --locked               passed (67 passed, 0 failed, 3 ignored)
cargo test -p eggpack-github --all-targets --all-features --locked          passed (39 passed, 0 failed)
cargo test -p eggpack-core --all-targets --all-features --locked             passed (61 passed, 0 failed, 6 ignored)
cargo package -p eggpack-manifest --locked --allow-dirty                     passed (eggpack-manifest v0.1.1, 6 files)
```

The `eggpack-core` test run was added because this milestone edited core; the other three crates are the ones the milestone changed.

**The workspace-wide `cargo check` and `cargo clippy -D warnings` are the proof that no second implementation of any of the five rules survived.** The greps behind that claim:

- `digest_file` / `hash_candidate_file`: the CI and GitHub survivors are now one-line delegations to `eggpack_core::file_digest`, with the sentinel `(0, String::new())` return gone.
- `validate_sha256` / `validate_sha256_hex`: the CI and GitHub survivors are delegations to `eggpack_manifest::validate_sha256_hex`.
- `host_matches_target`: one definition, in `eggpack-core`.
- `validate_exact_tag` / `validate_exact_tag_safety`: one rule, in `eggpack-core`; the CI survivor maps refusal classes onto its own wording.
- `validate_policy` / `validate_floor_applicability`: one rule, in `eggpack-core`.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §5.1 item 1 — host/target match callable from `eggpack-ci` | `eggpack_core::host_matches_target`; `CIPlan::validate` calls it. The CI copy is deleted. |
| §5.1 item 2 — policy validation callable from `eggpack-ci` | `eggpack_core::validate_policy` and `eggpack_core::validate_floor_applicability`; the CI floor and strategy helpers are deleted. |
| §5.1 item 3 — the canonical exact-tag safety validator | `eggpack_core::validate_exact_tag_safety`, carrying the union of both concern sets. |
| §5.1 item 4 — the canonical SHA-256 hex validator, per ADR-0006 | `eggpack_manifest::validate_sha256_hex`, re-exported by core and applied by core, CI, and GitHub. |
| §5.1 item 5 — the canonical file-digest helper, from M007 | `eggpack_core::file_digest`, consumed by the CI and GitHub crates. |
| §5.2 — every deletion verified by grep | The greps above; no second implementation of any of the five rules survives in the workspace. |
| §5.3 — the union, and the M003j format check kept separate | The concern-set table above; the format check remains a separate function with a separate diagnostic. |
| §5.4 — diagnostic compatibility | The "Diagnostic compatibility" section above; no message text removed, moved messages byte-identical. |
| §3 invariant: no second implementation of any producer rule | Confirmed by grep and by the clean workspace `clippy -D warnings`. |
| §3 invariant: `ReleaseManifest v1` unchanged | No field, no shape, no `schema_version` change anywhere in this milestone. |
| §3 invariant: integrity ≠ authenticity | No signature, provenance, or publisher-identity claim is introduced. The rule is about digest *format*, not about who produced the bytes. |
| §8 — the published crate's acceptance set narrows, and the narrowing is recorded | `eggpack-manifest` `0.1.1`; the compatibility event is this record and `architecture/manifest.md`'s subject area, and `eggup-eggpack` is named above. |

## Compatibility and no-regression evidence

- **No schema version bump.** `ReleaseManifest v1`, `PackConfig v1`, `CIPlan v1`, and `QualificationEvidence` are unchanged in shape.
- **No generated workflow byte changed.** The `ci _*` runner arguments, exit discipline, and diagnostics are unchanged, so `eggpack ci check` drift status is unchanged for every existing consumer.
- **No dependency added** in any crate.
- **No historical closure rewritten.** `plans/closure/ci-release-orchestration/001`–`003j-status.md` and every other historical closure record are untouched.
- **Pre-existing tests stay green** across all four affected crates.
- **`eggpack-core` is not published**, so its new public API surface carries no external compatibility cost.

## Security and reliability review

- The all-zeros digest is a natural "digest not computed" sentinel. Refusing it everywhere means a producer cannot record a placeholder as if it were evidence. The published crate is the boundary that matters most, because its documents are consumed by other software.
- The exact-tag union **narrows** what the core git path accepts. Git itself treats `#`, `@{`, `^`, `:`, `*`, `[`, and `~` as special in ref names, and shells treat `#`, `'`, `"`, `` ` ``, and `$` as metacharacters. Refusing them at the producer's own boundary is fail-closed.
- The CI candidate-hashing path no longer has a `(0, String::new())` sentinel: a missing, symlinked, empty, or unreadable candidate now produces a **typed error** and an absent candidate record, never an empty digest that could be mistaken for a computed one.
- No signature, provenance, or authenticity claim is introduced. SHA-256 and size remain **integrity facts only**.
- No new network, credential, privilege, archive, or publication surface is touched.

## Failure and recovery review

- A rule that moved between crates fails **closed** in its new home: a refusal that was previously possible in one crate is still possible, now via the canonical rule.
- The narrowed tag acceptance set fails closed: an unsafe tag is refused earlier, not later.
- The narrowed SHA-256 acceptance set fails closed: a placeholder digest is refused rather than recorded.
- Rollback is a revert of this single commit. The previously permissive acceptance set returns, with no consumer impact beyond re-admitting the all-zeros sentinel.
- There is no contention or restart hazard: the milestone adds no new external service, cache, or shared state.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, plan accuracy) | The plan's §2 inventory named **three** SHA-256 acceptance rules. There was a **fourth**: `crates/eggpack-core/src/qualification.rs` validated a candidate's recorded digest inline with a 64-character lowercase-hex check and accepted the all-zeros digest. ADR-0006 §5.1 requires `eggpack-core` to consume the canonical validator, so the inline check is a restatement of the rule. | Routed through `eggpack_core::validate_sha256_hex` in **Build/Qualification M008**, which owns `qualification.rs`. M003l did not own that file, so the correction is recorded here and discharged by the successor milestone rather than silently left. |
| Resolved in this milestone (medium, plan scope accuracy) | The plan's §4 scope named `crates/eggpack-core/**` visibility changes, but the two core-side rule *copies* — the exact-tag condition list in `run_git_tag_bounded` and the macOS-floor refusal in `cargo_command` — live in `crates/eggpack-core/src/builder.rs`. Leaving them would have left two live implementations of two of the five rules, failing the plan's own §12 requirement. | Both are routed to the canonical rules in this milestone. The plan's scope description was short by one file; recorded here so the record is not silently inaccurate. |
| Low (residual) | `eggpack-github`'s `verify_file` hashes a generic `ReadSeek` rather than a path, so the path-based `file_digest` cannot serve it. It is a different operation (streaming a caller-supplied reader), not a duplicate of the file-digest rule. | Left as-is and recorded. It is not a second *file-digest* implementation: it takes no path and makes no path-metadata decision. |
| Operational boundary | Verification ran on Linux x86-64 only. The macOS and Windows lanes are covered by hosted CI only. | Accepted; the same boundary every prior milestone recorded. No hosted run was claimed because nothing was pushed. |
| None | No unresolved medium-or-higher defect in the corrected producer contract. | M003l acceptance criteria satisfied. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003l | Reason |
|---|---|---|
| CI/Release M003l | closed | This record; implementation `9cf0bf8`. |
| CI/Release M003m | unblocked | Its only blocker was M003l. |
| CI/Release M003n | still blocked | It follows M003m. |
| Build/Qualification M008 | in progress | It consumes this milestone's canonical rule surface and discharges the residual `qualification.rs` SHA-256 acceptance set recorded above. |

## Downstream-unblock answer

Closing M003l unblocks **CI/Release M003m** and nothing else. Because M003m has not itself closed, CI M003n and every later milestone in the chain remain blocked behind it.

## Eggstack references

- Accepted decision: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md`
- Closed source plan: `plans/implementation/ci-release-orchestration/003l-producer-rule-authority-convergence-corrective.md`
- Predecessor that delivered the digest authority: `plans/closure/build-qualification/007-status.md` (`e3c6192`)
- Successor: `plans/implementation/ci-release-orchestration/003m-bounded-process-execution-convergence-corrective.md`
- Downstream consumer to be notified: `eggup-eggpack` (the only known consumer of `eggpack-manifest 0.1.0`), which must be given the `0.1.1` patch release and the narrowed acceptance set.
