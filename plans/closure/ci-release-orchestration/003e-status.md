# CI and Release Orchestration Milestone 003e Closure — Generated Release Execution Wiring Corrective

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003e-generated-release-execution-wiring-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: implementation parent `3f95af43f99224755c97161e0c1102a84e713e35` was the clean repository `main` state reviewed before changes.

Implementation commit: `b9062d4` on branch `m003e-execution-wiring` (supersedes `8d9e92a`, amended only to make the new CLI unit test portable after the first hosted run exposed a Unix-specific path assumption on Windows).

## Executive finding

M003e is closed. Every generated release job now installs the pinned Eggpack tool before any Eggpack invocation and verifies the checked-out source immediately after the install; the resolve, validate, gate, and aggregate jobs create their output directories before the steps that write them; and the CLI adapter absolutizes generated relative candidate/output-root paths against an explicit base while rejecting escapes. No library validation was relaxed, and no new action, trigger, or privilege was added.

The disposable end-to-end chain (resolve, build, capture, qualify, validate, gate, aggregate, prepare-stage) completes against the corrected tool with no manual mkdir/absolute-path workarounds, and the regenerated five-target eggsact candidate proves install-before-use plus mkdir-before-write in all 20 executable jobs.

No real GitHub draft was created or required by M003e. Ecosystem M001 (eggsact) is now ready to resume: re-review its consumer baseline, re-pin to the M003e implementation, regenerate the candidate, and run the maintainer-authorized live draft qualification.

## Root-cause matrix

| Finding | Correction | Evidence |
|---|---|---|
| F1 — exact preflight invoked `_verify-source` before the tool install | Preflight installs the pinned tool first (exact staging) and drops the dead `_verify-source` call when staging is disabled; every per-target job moved the install before verify | `m003e_tool_before_use_in_every_job`; regenerated goldens |
| F2 — `resolve` never created the runtime identity directory | Resolve job emits `mkdir -p './eggpack-runtime'` before `_resolve-release`; `resolve --output-plan/--output-ci-plan/--output-github-policy` parents are accepted relatively | `m003e_output_dirs_exist_before_writes`; e2e resolve with relative outputs |
| F3 — validate jobs never created their evidence directory | Each validate job emits `Create consumer evidence directory` before `_validate-consumer` | `m003e_output_dirs_exist_before_writes`; e2e validate passed |
| F4 — gate never created its outcome directory | Gate job emits `Create gate outcome directory` before `_evaluate-gate` | `m003e_output_dirs_exist_before_writes`; e2e gate outcome Complete |
| F5 — aggregate never created its output directory | Aggregate job emits `Create finalized release directory` before `_aggregate` | `m003e_output_dirs_exist_before_writes`; e2e aggregate Complete |
| F6 — aggregate output root was relative but must be absolute | CLI `absolutize_cli_path` joins the generated relative `--output-root` onto the explicit `--source-root` base; `..` escapes and non-absolute bases rejected | `absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative`; e2e aggregate with relative `--output-root` |
| F7 — qualify candidate directory was relative but must be absolute | Same adapter fix applied to `_qualify-target --candidate-dir` | CLI unit test; e2e qualify Passed with relative `--candidate-dir` and no pre-created evidence dir |

## Requirement-to-evidence matrix

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked         passed (57 passed, 3 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked        passed (10 tests)
cargo test --workspace --all-targets --all-features --locked           passed (215 passed, 7 ignored)
cargo doc --workspace --no-deps --locked                               passed
cargo tree (all seven crates)                                          passed
cargo package (all seven crates)                                       passed with workspace path patches
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test (all seven crates)                                  passed
./scripts/check-local.sh                                               passed (exit 0 on final SHA)
git diff --check                                                       passed
```

The path patches for package verification are the repository's established `scripts/check-local.sh` mechanism for unpublished sibling crates.

Hosted CI run `36572608484` validates final implementation commit `b9062d4`; Linux stable, Linux Rust 1.89, macOS, and Windows lanes passed. The workflow run is linked at [GitHub Actions run 36572608484](https://github.com/eggstack/eggpack/actions/runs/36572608484). A first attempt (run `36572094210`) failed only the Windows lane on a Unix-specific `/abs/root` assumption in the new CLI unit test; the production code was unchanged and the portable rewrite re-ran green on all four lanes.

Golden regeneration inventory (all seven regenerated by the ignored helpers and green):

- `crates/eggpack-ci/tests/fixtures/m002-direct.yml`
- `crates/eggpack-ci/tests/fixtures/m002-bundle.yml`
- `crates/eggpack-ci/tests/fixtures/m002-archive.yml`
- `crates/eggpack-ci/tests/fixtures/m002-mixed.yml`
- `crates/eggpack-ci/tests/fixtures/m003b-direct-staging.yml`
- `crates/eggpack-ci/tests/fixtures/m003b-bundle-staging.yml`
- `crates/eggpack-ci/tests/fixtures/m003b-archive-staging.yml`

Disposable end-to-end chain (fresh clone of eggsact at tag `v9.9.9`, commit `8f0a84de`, corrected release tool, zero manual workarounds):

- five-target resolve against the real `release/eggpack` config: release plan, CI plan (5 qualifications), and draft policy all resolve;
- single-target projection of the real contract shape (linux-x64 block copied verbatim) through build, capture, qualify (Passed), consumer validate (passed), gate (Complete), aggregate (Complete, exact public asset `eggsact-x86_64-unknown-linux-gnu` plus `.sha256` sidecar), and prepare-stage (7 assets including `install.sh`/`install.ps1`);
- regenerated five-target candidate (55952 bytes, 21 jobs): per-step static proof shows install-before-use and mkdir-before-write in all 20 executable jobs.

## Invariant, recovery, compatibility, and security review

- Library validation semantics are unchanged: `atomic_write` still requires an existing parent (the renderer now guarantees it), `inspect_candidate`/`finalize_release` still require absolute paths (the CLI adapter now guarantees them).
- Exact plan/contract/identity checks are untouched; the adapter joins paths lexically with no symlink resolution and rejects any `..` component, so a crafted relative path cannot escape the explicit base.
- No new third-party action, trigger, event, permission, or network egress was added. Tool install still precedes the toolchain-dependent `cargo install`, and the build job still compiles the product before capture.
- Source and staging-disabled workflow compatibility is preserved: the exact non-staging preflight emits no `_verify-source` call, exactly as before.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher execution-wiring finding. | M003e acceptance criteria satisfied. |
| Outstanding operational condition | No real GitHub draft qualification was performed. | M003e does not require it. Ecosystem M001 resumes with a maintainer-authorized repository/tag; the live draft remains unpublished. |
| Evidence boundary | Hosted CI exercises Linux stable/MSRV/macOS/Windows repository checks, not a live five-target release against GitHub. | Recorded; live draft qualification remains the separate Ecosystem M001 operational gate. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003e | Reason |
|---|---|---|
| CI M003e | closed | This record and hosted run `36572608484`. |
| Ecosystem adoption M001 (eggsact) | ready to resume | Corrective dependency is closed; it must re-review its consumer baseline, re-pin to `b9062d4`, regenerate the candidate, and run the maintainer-authorized live draft. |
| CI M003b live draft qualification | still blocked | Runs inside Ecosystem M001; unchanged. |
| Phase 8 exit | blocked | Requires full M003b live draft qualification; unchanged. |
| Bootstrap M003 | blocked | Independent adoption evidence/candidate review remains outstanding. |

Registry and the CI roadmap now identify M003e as closed and Ecosystem M001 as ready to resume.
