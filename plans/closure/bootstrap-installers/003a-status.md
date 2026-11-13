# Bootstrap Installers Milestone 003a Closure — ADR-0006 Adversarial Digest Fixture Correction

Status: closed

Source plan: `plans/implementation/bootstrap-installers/003a-adr-0006-digest-fixture-correction.md`

Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md` (registered there)

Triggered by: CI/Release M003l (`9cf0bf8`), which implemented accepted ADR-0006 and tightened the SHA-256 hex acceptance set.

Reviewed baseline: `d8d6a5b` (Planning Hygiene M003), on top of the full review chain.

## Executive finding

Bootstrap Installers 003a is closed. The bootstrap installers' adversarial digest coverage is restored.

The defect was not in the installers. It was in five test fixtures that used the all-zeros digest as the stand-in for "a digest that does not match the bytes being installed". When CI M003l implemented ADR-0006, the all-zeros sentinel became invalid **at manifest validation**, so those fixtures began failing at setup — before the installer's own size-and-digest integrity check could run. The installers' behaviour was correct throughout; the fixtures had been relying on a value that had just become illegal.

The correction is deliberately the narrowest one that preserves the coverage: the fixtures now use a **well-formed but wrong** digest, so the manifest still validates and the installer's own check is what rejects the artifact. The all-zeros sentinel stays refused everywhere. The acceptance rule did not move.

## The defect, as observed

```text
$ cargo test -p eggpack-bootstrap --all-targets --all-features --locked
test result: FAILED. 8 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out

---- tests::m002_archive_posix_runtime_matrix stdout ----
thread 'tests::m002_archive_posix_runtime_matrix' panicked at crates/eggpack-bootstrap/src/lib.rs:2659:88:
called `Result::unwrap()` on an `Err` value: BootstrapError("invalid release manifest")
---- tests::m002_bundle_posix_runtime_matrix stdout ----
thread 'tests::m002_bundle_posix_runtime_matrix' panicked at crates/eggpack-bootstrap/src/lib.rs:2337:88:
called `Result::unwrap()` on an `Err' value: BootstrapError("invalid release manifest")
```

The failing unwraps are on `render_posix_with_policy(...).unwrap()` calls, which validate the manifest before rendering.

## Production implementation evidence

Five fixture sites in `crates/eggpack-bootstrap/src/lib.rs` changed. The all-zeros placeholder `"00".repeat(32)` became `"ff".repeat(32)`.

| Site | Test intent | Why the site exists |
|---|---|---|
| `:1331` | a PowerShell installer refuses a mismatched digest and leaves no files | `bad_digest_ps` |
| `:1367` | a POSIX installer refuses a mismatched digest and leaves no files | `bad_digest` |
| `:2335` | "wrong SHA leaves no files" for a bundle target | `bad_sha` |
| `:2656` | "archive size / SHA mismatch rejects without placement" for an archive target | `bad` archive |
| `:3330` | "archive integrity negatives fail before member listing/extraction" for an archive target | `bad` archive |

**Why `"ff"` is a correct choice.** It is exactly 64 lowercase hex characters, so the manifest's acceptance rule admits it. It is not the all-zeros sentinel, so ADR-0006's rule does not refuse it. It is not the digest of any bytes any fixture serves: the bootstrap crate's real fixture digests are `"ab".repeat(32)`, `"11".repeat(32)`, and the SHA-256 of `b"abc"` (`ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`) — `"ff"` collides with none of them, and a `grep` confirmed `"ff".repeat(32)` did not previously appear in the crate at all.

**Why the assertions are unchanged.** Each site still asserts that the installer **refuses** the artifact and **leaves nothing behind**, for example:

```rust
let (ok, dest, _) = run_ps("bad-digest-powershell", &bad_digest_ps, b"abc", "200 OK");
assert!(!ok);
assert_eq!(fs::read_dir(dest).unwrap().count(), 0);
```

The coverage these tests were written to provide — that the installer detects a digest that does not match the bytes it fetched — is intact. A well-formed wrong digest is also the *realistic* case: the all-zeros case is now caught earlier, by the manifest validator, which is defence in depth rather than a replacement.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §3 — the installer-side integrity check stays under test | Each of the five sites still asserts `!ok` and an empty destination; the manifest now validates, so the rejection comes from the installer's own check. |
| §3 — the fixtures' digests stay well-formed | `"ff".repeat(32)` is 64 lowercase hex characters. |
| §3 — the fixtures' digests stay wrong | `"ff"*32` differs from every digest the crate's fixtures use, and the tests still assert refusal. |
| §3 — no collision with an existing fixture digest | A `grep` confirmed `"ff".repeat(32)` did not appear in the crate before the change; the crate's other digests are `"ab"*32`, `"11"*32`, and the SHA-256 of `b"abc"`. |
| §3 — no production behaviour change | Only test fixtures changed; `git diff --stat` shows one file, and it is a test module. |
| §4 — the acceptance rule is not weakened | `eggpack-manifest` and ADR-0006 are untouched. The all-zeros digest remains refused. |
| §4 — out of scope: every other crate | `git diff --stat` shows no change outside `crates/eggpack-bootstrap/`. |
| §9 — no all-zeros placeholder remains in the bootstrap crate | `grep -c '"00".repeat(32)'` returns **0**; `grep '"ff".repeat(32)'` returns exactly the five intended sites. |
| §12.5 — the full workspace test suite is green | **337 passed, 0 failed**; and `./scripts/check-local.sh` exits **0**. |

## Verification executed

```text
cargo fmt --all -- --check                                                  passed
cargo check --workspace --all-targets --all-features --locked               passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-bootstrap --all-targets --all-features --locked        passed (11 passed, 0 failed; 8 passed / 3 failed before)
cargo test --workspace --all-targets --all-features --locked                 passed (337 passed, 0 failed)
./scripts/check-local.sh                                                     exit 0
git diff --check                                                            passed
```

`./scripts/check-local.sh` runs under `set -euo pipefail` and covers: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; the per-crate test suites for `eggpack-contract`, `eggpack-github`, `eggpack-ci`, and `eggpack-cli`; the full workspace test suite; `cargo doc --workspace --no-deps --locked`; `cargo tree`; `cargo package` for all seven crates with the `--config 'patch.crates-io.<dep>.path="crates/<dep>"'` flags; and the `cargo +1.89.0` MSRV lane. An exit code of **0** means every stage passed.

## Compatibility and migration

None. Test fixtures are not a published surface. No crate's behaviour changed, no schema changed, no manifest changed, and no dependency changed.

## Security and reliability review

- The change **restores** coverage rather than removing it. Before the correction, the three failing tests were asserting nothing — they aborted at setup. After it, each asserts that the installer refuses a mismatched artifact and leaves no files behind.
- The all-zeros sentinel remains refused by the manifest acceptance rule, so a producer still cannot record a placeholder digest as if it were evidence.
- No production code path changed, so the installers' security posture is exactly what it was.

## Failure and recovery review

- A fixture change cannot fail closed against production. If a test had still failed after the correction, the cause would have been something other than the digest placeholder — which the plan's §13 names as a stop condition, and which did not arise.
- Rollback is a revert of this single commit. Reverting restores the failing state, not a working one.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| Resolved in this milestone (medium, correctness of coverage) | CI M003l's ADR-0006 rule left five bootstrap fixtures using an illegal value as a placeholder, so three tests aborted at setup and the installers' digest-mismatch coverage was silently not running. | Corrected in this milestone. The coverage is restored and the acceptance rule is untouched. |
| Process finding (medium) | CI M003l's implementation and its own scoped verification were both green, and the regression was still live in another crate. M003l's scoped gate ran `cargo test -p eggpack-manifest`, `-p eggpack-ci`, `-p eggpack-github`, and `cargo check --workspace` — but **not** the workspace test suite, so the bootstrap fixture breakage was not detected at that milestone. | Recorded here rather than left implicit. The maintainer's "minimal testing scoped to the actual changes" direction produced exactly this gap, and the **integration** gate is what caught it. The standing lesson for this chain: a change to a shared acceptance rule cannot be verified by the crates it touches alone, and the workspace suite must run before the chain closes. |
| Operational boundary | Verification ran on Linux x86-64 only. The macOS and Windows lanes are covered by hosted CI only, and no hosted run was performed because nothing was pushed. | Accepted; the same boundary every milestone in this chain recorded. The two Windows-lane bootstrap tests are `#[cfg]`-gated to Windows. |
| None | No unresolved medium-or-higher defect in the bootstrap installers. | 003a acceptance criteria satisfied. |

## Roadmap disposition and dependency transitions

| Milestone | Status after 003a | Reason |
|---|---|---|
| Bootstrap Installers 003a | closed | This record. |
| Build/Qualification M007, M008, M009 | closed | Unaffected. |
| CI/Release M003l, M003m, M003n | closed | Unaffected; M003l's rule is correct and unchanged. |
| Planning Hygiene M003 | closed | Unaffected. |

## Downstream-unblock answer

Closing 003a unblocks **nothing** in this chain — the whole review chain was already closed. What it discharges is the last open defect the chain's integration gate found, so the branch is green end to end.

## Eggstack references

- Closed source plan: `plans/implementation/bootstrap-installers/003a-adr-0006-digest-fixture-correction.md`
- The decision that caused the defect: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` (**accepted**, Option A)
- The milestone that implemented it: `plans/closure/ci-release-orchestration/003l-status.md` (`9cf0bf8`)
