# CI and Release Orchestration Milestone 003g Closure — Live Qualification Failures

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003g-live-qualification-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `8d1701a` on branch `m003f-tool-install-command` (closed M003f plus the M001 pin-pointing docs).

Implementation commits: `5acda73` (F9, F10 diagnosis), `4b28820` (F10a, F10c, zigbuild presence check), `5ac5b83` (F11), `e5c81f2` (F12, F13) on branch `m003g-live-qualification`.

## Executive finding

M003g is closed. Five defects were found by the real five-target pipeline
across seven live dispatches and all are corrected. The final live run staged a
complete 15-asset draft for a real eggsact release.

| Finding | Correction | Live evidence |
|---|---|---|
| F9 — `cargo zigbuild --version` ran before its PATH export took effect; `GITHUB_PATH` only affects later steps, so both Linux cross builds died with `no such command: zigbuild` | Same-step `export PATH="$install_root/bin:$PATH"`; the later build step is the functional proof | run 36634754073 |
| F9b — the install check itself was invalid: `cargo zigbuild --version` is rejected by cargo-zigbuild's CLI (`unexpected argument '--version'`) | Replaced with `test -x "$install_root/bin/cargo-zigbuild"`; exactness stays in the pinned `--version '0.23.3'` install | run 36639021458 |
| F10 — macOS consumer validation failed with no named cause | Failure variant printed (`consumer validation failed for {target}: {reason:?}`); evidence bytes unchanged | run 36639021458 |
| F10a — root cause: artifact transfer strips POSIX exec bits (upload-artifact documents 644), so the transferred macOS candidate could not spawn | Exec bits restored immediately before spawning, after byte-identity verification, in both `run_bounded_inner` (`Candidate` allowance) and `run_validator_process`; recorded size/digest cover bytes only | run 36642916807 |
| F10c — `_validate-consumer` spawned the consumer script against a candidate whose qualification had already failed | Non-`Passed` evidence refused fast with its status; qualify still exits 0 on Failed so the gate keeps visibility and fails closed | run 36642916807 |
| F11 — `_stage-github-draft`'s arity guard (`> 6`) rejected the renderer's own four-flag call (8 args) | Bound raised to `> 8`; an audit of every other `ci_*` guard against the rendered invocations found this the only mismatch | run 36642916807 |
| F12 — the inner staging error was discarded, making the failure unnameable | Inner `GithubError` surfaced; audited to contain only static bounded messages, never token, body, or URL content | run 36647375561 |
| F13 — `create_release` sent `"make_latest": false` (boolean) where the API specifies a string enum; a latent 422 on the create path | Sent as `"false"`; the create path had never been reached before the successful live run | latent, found by wire review |

## Requirement-to-evidence matrix

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked         passed (59 passed, 3 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked        passed (12 tests)
cargo test --workspace --all-targets --all-features --locked           passed (218 passed, 7 ignored)
cargo doc --workspace --no-deps --locked                               passed
cargo tree (all seven crates)                                          passed
cargo package (all seven crates)                                       passed with workspace path patches
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test (all seven crates)                                  passed
./scripts/check-local.sh                                               passed (exit 0 on final SHA)
git diff --check                                                       passed
```

Hosted CI runs on the implementation commits: 36638321253 (`5acda73`), 36642184204 (`4b28820`), 36646814734 (`5ac5b83`), 36652203168 (`e5c81f2`) — all four lanes (Linux stable, Linux 1.89.0, macOS, Windows) green.

Golden regeneration inventory: no fixture uses the provisioned cross-tools path, so the seven goldens were unchanged by M003g; this was verified by regenerating and confirming a zero diff.

## Live qualification evidence (eggsact `v1.2.7`)

Dispatch sequence on tag `v1.2.7` (annotated, at the published commit `d8014cf`):

1. `36630837644` — F8 (`-p` install flag), failed at provisioning in every job. No mutation.
2. `36634754073` — F9, failed in both Linux cross builds; macOS consumer validation also failed. No mutation.
3. `36639021458` — F9b; macOS consumer validation again failed (F10). No mutation.
4. `36642916807` — F11: all five targets built, qualified, validated, gated, and aggregated; only `stage` failed. No mutation. One `curl` exit-28 against ziglang.org on the AArch64 Linux job was resolved by `gh run rerun --failed` with no code change; it is an external-network condition, not a configuration defect.
5. `36647375561` — 19/20 jobs; `stage` failed on the create path. No mutation.
6. `36652731202` attempt 1 — all 20 jobs green; draft `eggsact v1.2.7` created with 15 assets, receipt id `RE_kwDOTGg0Mc4X0gk6`.
7. `36652731202` attempt 2 (rerun) — 19/20 jobs; `stage` refused to clobber the differing Windows asset with `same-name remote asset digest mismatch`, leaving the draft intact. Correct fail-closed behavior; root cause is product-side MSVC non-determinism, recorded by the consumer as M005a.

## Invariant, recovery, compatibility, and security review

- No library validation was relaxed anywhere. The exec-bit restore happens only after byte identity (size and SHA-256) has already been verified against evidence, and it cannot alter recorded size or digest.
- Qualify still exits 0 on `Failed` evidence so the gate observes it and fails closed; the new refusal is in the consumer validator, which previously executed against a candidate already proven non-satisfying.
- F12 error surfacing was audited against the `GithubError` construction sites: messages are static and bounded, and no site interpolates a token, response body, or URL.
- No new action, trigger, privilege, or network egress. The staging job remains the single `contents: write` job and the only remote writer.
- Diagnostics stay console-only; durable evidence schemas and bytes are unchanged, and script output still never enters evidence.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None in Eggpack | No unresolved medium-or-higher execution or staging defect. | M003g acceptance criteria satisfied. |
| External | ziglang.org download can exceed the bounded 600s curl timeout. | Bounded and fail-closed; resolved by job retry. Owned by provisioning policy, not by this corrective. |
| Cosmetic | `actions/upload-artifact` / `download-artifact` warn about the unsupported `if-no-files-found` input the renderer emits. | Third-party annotation only; uploads and downloads behave correctly. A future renderer fix may drop the input, but it changes no evidence. |
| Consumer-side | The Windows candidate is not byte-reproducible, so rerun reuse fails closed on that one asset. | Correct Eggpack behavior; owned by eggsact M005a (`plans/closure/distribution-update-release/005-status.md`). |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003g | Reason |
|---|---|---|
| CI M003g | closed | This record and hosted runs through 36652203168. |
| CI M003b live draft qualification | satisfied except rerun-reuse | Real draft, exact assets, draft-only, and no-clobber are recorded; byte-identical rerun reuse is blocked on the consumer's Windows determinism (eggsact M005a). |
| Ecosystem adoption M001 (eggsact) | closed | Consumer adoption completed on release `v1.2.7`; see `plans/closure/ecosystem-adoption/001-status.md`. |
| Ecosystem adoption M002 (stegoeggo) | ready to plan | First-consumer evidence exists; second-consumer evidence avoids one-repo schema overfitting. |

Registry and the CI roadmap now identify M003g as closed.
