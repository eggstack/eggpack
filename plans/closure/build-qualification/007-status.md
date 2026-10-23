# Build and Qualification Milestone 007 Closure — Evidence Digest Unification Corrective

Status: closed

Source plan: `plans/implementation/build-qualification/007-evidence-digest-unification-corrective.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Related decision: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` (**accepted**, Option A). M007 implements no acceptance set and changes no ADR decision; it delivers the canonical file-digest *implementation* that ADR-0006 §5.1 item 5 names as a prerequisite for CI M003l.

Reviewed baseline: `a0fec98` (`origin/main` — the merge of `eggstack/eggpack#1`, CI M003k, and `eggstack/eggpack#2`).

Implementation SHA: `e3c6192b3b0e405a0cfe46e5025f31a185881f7e` — `fix(core): unify producer file-digest authority`.

## Executive finding

M007 is closed. `eggpack-core` now has exactly **one** file-digest authority: `file_digest` in the new `crates/eggpack-core/src/digest.rs`, carrying the stricter of the two divergent semantics it replaces. The three in-crate implementations that existed before this milestone are gone, and the two out-of-crate implementations in `eggpack-ci` and `eggpack-github` are now unblocked from being converged onto it by CI M003l.

Nothing was weakened to achieve this. The hashed bytes are unchanged — same hash function, same encoding, same lowercase hex — and the in-tree golden digests are byte-identical. What changed is *which* helper is used and *what it refuses*: a path that changes length, type, or symlink status while it is being read is now refused on the manifest-building path as well as on the finalization path, where previously only the finalization path checked. That is the intended correction, and it fails closed.

## Production implementation evidence

| Location | Change |
|---|---|
| `crates/eggpack-core/src/digest.rs` (new) | `pub fn file_digest(path: &Path) -> Result<(u64, String), DigestError>` (`:102`) — the single producer-side file-digest authority. Pre-open `symlink_metadata`; refuses anything that is not a regular non-symlink file; refuses an empty file **before** the read; streams through a fixed 64 KiB buffer (`hash_stream` `:117`); accumulates size with `checked_add` (`add_size` `:139`); re-checks `symlink_metadata` after the read (`post_read_check` `:144`, `post_read_stable` `:153`) and refuses a path that became a symlink, stopped being a regular file, or changed length. Carries the TOCTOU rationale that the retired `crates/eggpack-core/src/lib.rs:292-296` comment carried. |
| `crates/eggpack-core/src/digest.rs` (new) | `pub enum DigestError` (`:31`) with the typed refusal reasons `Unavailable`, `NotRegularFile`, `Empty`, `OpenFailed`, `ReadFailed`, `SizeOverflow`, `ChangedDuringRead`. Replaces the previous opaque `String` error, so no diagnostic became vaguer than it was. |
| `crates/eggpack-core/src/lib.rs` | `fn digest_file` **retired**. The two `build_manifest` call sites (`:209`, `:256`) now call `file_digest` with subject `"finalized file"`. |
| `crates/eggpack-core/src/finalization.rs` | `fn digest_regular` **retired**. Four call sites routed through `digest_candidate_bytes` (`:316`) and `digest_finalized_bytes` (`:321`). |
| `crates/eggpack-core/src/qualification.rs` | The inline SHA-256 streaming loop inside `inspect_candidate` is replaced by `file_digest` (`:1138-1141`). `DigestError::ChangedDuringRead` maps to `QualificationFailure::CandidateChanged`; every other variant maps to `QualificationFailure::InvalidRuntimeEnvironment`, so all original failure classifications are preserved. |
| `crates/eggpack-core/src/lib.rs` | `pub mod digest;` and the crate-root re-exports of `file_digest` and `DigestError`, so `eggpack-ci` and `eggpack-github` can consume the same authority. |

Retained, deliberately unchanged: the hash function, the digest encoding, the lowercase hex representation, the finalization order, archive assembly, sidecar content, the `ReleaseManifest v1` field set and `schema_version`, and the published `eggpack-manifest` crate.

Documentation: `crates/eggpack-core/README.md`, `architecture/core-finalization.md` (a new "File-digest authority" section states the TOCTOU rule **once**, replacing the two separate descriptions), and `architecture/core.md` (file map and a single pointer to that rule). All `file:line` citations in the two architecture documents were re-based **by symbol name** and read back to confirm the cited line holds what the sentence claims.

## Plan inventory correction — a third in-crate implementation

The plan's §2 evidence inventory named **two** in-crate file-digest implementations. It was incomplete: `crates/eggpack-core/src/qualification.rs` (`inspect_candidate`) held a **third** SHA-256 streaming loop, which the plan's §5 requirement "every core call site routes to the canonical helper" and its §12 acceptance criterion "exactly one file-digest implementation" both require to be routed.

It was routed through `file_digest` in this milestone. The change is behaviour-preserving: the same bytes are hashed, the same `canonical` path is inspected, and every original failure classification is retained. Had it been left in place, "exactly one implementation" and "every core call site" would both have been false at closure.

This is recorded here rather than silently absorbed, because a plan's evidence inventory being short by one implementation is a planning defect worth knowing about: any future change in this area should inventory the whole workspace, not only the two call sites a defect report happened to name.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §5.1 one canonical helper carrying the stricter semantics | `crates/eggpack-core/src/digest.rs`; `file_digest` performs the pre-open metadata check, the pre-read empty refusal, the bounded streaming read, the `checked_add` accumulation, and the post-read re-check. |
| §5.1 helper is `pub`, returns a typed error | `pub fn file_digest`, `pub enum DigestError`, re-exported at the crate root. |
| §5.2 both duplicates retired, every core call site routed | `grep` for `digest_file` and `digest_regular` returns **zero** occurrences in `eggpack-core`; the only file-digest streaming loop left in the crate is `digest.rs:118`. |
| §5.3 refusal reasons distinguished | The seven `DigestError` variants map one-to-one onto the refusals the two retired helpers performed. |
| §5.4 TOCTOU rationale documented | Carried into the new module's doc comment on `file_digest`. |
| §3 invariant: identical bytes ⇒ identical digest and size | `regular_file_digest_is_the_lowercase_hex_sha256_of_its_bytes` and `identical_bytes_digest_identically_and_different_bytes_differ`; the in-tree golden digests are unchanged and `manifest_records_agree_with_the_finalized_bytes_for_every_artifact` passes on both the pre- and post-change tree. |
| §3 invariant: a symlink is never followed | `symlink_is_refused`, `finalization_refuses_a_symlinked_candidate`, `manifest_builder_refuses_a_symlinked_artifact`. |
| §3 invariant: an empty file is refused before it is read | `empty_file_is_refused_before_it_is_read`. |
| §3 invariant: size overflow is an error, never a wrap | `add_size` uses `checked_add` and returns `DigestError::SizeOverflow`. |
| §3 invariant: streaming through a fixed buffer, never the whole artifact | `hash_stream` reads through a fixed 64 KiB buffer. |
| §8 invariant: no schema change, published crate untouched | `ReleaseManifest v1` fields, `schema_version`, and `crates/eggpack-manifest/**` are untouched by this commit. |
| §8 invariant: no new dependency | `sha2` was already an `eggpack-core` dependency; `Cargo.toml` is unchanged. |
| §4 out of scope: no acceptance set | No SHA-256 hex acceptance rule was added, changed, or removed. ADR-0006 is untouched by this commit. |
| §4 out of scope: `eggpack-ci` and `eggpack-github` digest implementations | Both are untouched by this commit; CI M003l converges them onto `file_digest`. |

## Verification executed

All verification below ran against implementation SHA `e3c6192`, scoped to the actual change as the maintainer directed for this stage:

```text
cargo fmt --all -- --check                                                  passed
cargo check -p eggpack-core --all-targets --all-features --locked           passed
cargo clippy -p eggpack-core --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-core --all-targets --all-features --locked             passed (61 passed, 0 failed, 6 ignored; 57/6 before M007)
cargo check --workspace --all-targets --all-features --locked                passed
git diff --check                                                            passed
```

The workspace-wide `cargo check` was run specifically to prove that retiring the two private helpers broke no other crate; `eggpack-ci` and `eggpack-github` each carry their own digest implementation and do not call core's.

The four new tests are `regular_file_digest_is_the_lowercase_hex_sha256_of_its_bytes`, `identical_bytes_digest_identically_and_different_bytes_differ`, `empty_file_is_refused_before_it_is_read`, `symlink_is_refused`, and `post_read_symlink_change_is_refused` (`crates/eggpack-core/src/digest.rs`), plus `finalization_refuses_a_symlinked_candidate` (`crates/eggpack-core/src/finalization.rs`) and `manifest_builder_refuses_a_symlinked_artifact` / `manifest_records_agree_with_the_finalized_bytes_for_every_artifact` (`crates/eggpack-core/src/lib.rs`).

**Pre-change discrimination was verified empirically.** The pre-change tree was extracted to a scratch directory and the new tests were run against it: `finalization_refuses_a_symlinked_candidate` fails against the pre-change tree and passes after the change; `manifest_records_agree_with_the_finalized_bytes_for_every_artifact` passes on **both** trees, which is what proves the digests and sizes are unchanged.

## Deviations and observable behaviour changes

1. **The post-read race cannot be staged deterministically.** The plan's §6 WP1 explicitly permits asserting the *presence* of the re-check by unit-testing the post-read comparison directly rather than by racing a thread. `post_read_check` is therefore unit-tested against real files for a length change, a type change, and a symlink change. The tests prove the **comparison refuses**; they do not prove that `file_digest` still *calls* that comparison. This limitation is stated in the test comments, as the plan requires.
2. **Diagnostics were split where the old code merged them.** The retired `digest_regular` used a single message for symlink, non-regular file, and empty file. The new helper distinguishes them ("candidate file is not a regular non-symlink file", "… must be non-empty"), which is the plan's §5.3 requirement that no diagnostic become vaguer.
3. **Two finalization sites changed their subject noun.** The finalized-copy and sidecar digests now report "finalized file …" rather than "candidate file …", because they are no longer digesting a candidate. No message text was otherwise changed.
4. **Behavioural narrowing (intended).** `build_manifest` now refuses a file that changes during the read. Previously only the finalization path refused such a file. The plan's §8 records this as the intended correction.

## Compatibility and no-regression evidence

- **No schema version bump.** `ReleaseManifest v1`, `PackConfig v1`, `CIPlan v1`, and `QualificationEvidence` are all unchanged; no serialized field was added, removed, or renamed anywhere.
- **No golden changed.** The in-tree golden digests are byte-identical; `manifest_records_agree_with_the_finalized_bytes_for_every_artifact` passes on both the pre- and post-change tree.
- **No public API break beyond two private removals.** `digest_file` and `digest_regular` were both private `fn`s. `eggpack-core` is not published.
- **No historical closure rewritten.** `plans/closure/build-qualification/001`–`006-status.md` and every other historical closure record are untouched.
- **Pre-existing tests stay green**, including the finalization suite with its golden digests and the qualification suite.

## Security and reliability review

- The change removes a **TOCTOU window**, not a check. The manifest-building path previously digested a file without re-checking that the file it read was still the file it opened; it now applies the same post-read re-check the finalization path already applied. The window was exploitable by a process that replaced a finalized artifact between the metadata read and the read itself.
- Nothing is weakened: a symlink is still never followed, an empty file is still refused, and a path that changes during the read is refused rather than digested.
- No signature, provenance, or authenticity claim is introduced. SHA-256 and size remain **integrity facts only**, consistent with the workspace's standing rule that integrity ≠ authenticity.
- No network, credential, privilege, publication, or archive surface is touched. The helper performs no writes and holds no locks, so concurrent callers are independent; each observes a self-consistent `(size, digest)` pair or an error.
- No error message embeds artifact content, so no diagnostic can leak bytes.

## Failure and recovery review

- A refused digest is a hard failure of the enclosing operation. It is never downgraded to a warning, never retried against the same path, and never recorded as an empty digest. No partial or placeholder digest is emitted; there is no sentinel success value.
- Restarting a failed finalization re-runs the digest from the beginning; there is no resumable state to corrupt.
- Rollback is a revert of this single commit. The previously laxer `build_manifest` behaviour returns, with no consumer impact, because no producer emits a file that changes during its own read.
- There is no contention or restart hazard: the milestone adds no new external service, cache, or shared state.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, plan accuracy) | The plan's §2 inventory named two in-crate file-digest implementations; `crates/eggpack-core/src/qualification.rs` (`inspect_candidate`) held a third SHA-256 streaming loop. | Routed through the canonical helper in this milestone, because the plan's own §5 and §12 require "every core call site" and "exactly one implementation" to hold. The plan's inventory was short by one implementation; recorded here so the record is not silently inaccurate. |
| Low (test coverage) | The post-read re-check cannot be proven to be *invoked* by a deterministic test on this platform. | Tested at the comparison level, as the plan's §6 WP1 explicitly permits, with the limitation stated in the test comments. A hostile-reader simulation is not implemented and is not claimed. |
| Operational boundary | Verification ran on Linux x86-64 only. The macOS and Windows lanes are covered by hosted CI only. | Accepted; the same boundary every prior milestone recorded. |
| None | No unresolved medium-or-higher defect in the corrected producer contract. | M007 acceptance criteria satisfied. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M007 | Reason |
|---|---|---|
| Build/Qualification M007 | closed | This record; implementation `e3c6192`. |
| Build/Qualification M008 | unblocked | Its only blocker was M007's canonical digest authority. |
| CI/Release M003l | unblocked on its remaining blocker | ADR-0006 is accepted (Option A), and M003l's digest work package now has the canonical helper to consume. Its other gate — the M003k merge — is satisfied by `a0fec98`. |

## Downstream-unblock answer

Closing M007 unblocks **Build/Qualification M008** and removes one of **CI/Release M003l**'s three gates. It unblocks no other milestone.

## Eggstack references

- Closed source plan: `plans/implementation/build-qualification/007-evidence-digest-unification-corrective.md`
- Related accepted decision: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md`
- Successor: `plans/implementation/build-qualification/008-compatibility-floor-verification-corrective.md`
- Consumer of the delivered API: `plans/implementation/ci-release-orchestration/003l-producer-rule-authority-convergence-corrective.md`
