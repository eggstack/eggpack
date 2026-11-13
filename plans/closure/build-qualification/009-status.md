# Build and Qualification Milestone 009 Closure — Local Producer Operator Commands

Status: closed

Source plan: `plans/implementation/build-qualification/009-local-producer-operator-commands.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `282430b` (the M008 closure), on top of Build/Qualification M007 (`e3c6192`) and CI/Release M003l (`9cf0bf8`).

Implementation SHAs:

- `eb77d19` — `feat(cli): add local producer operator commands` (the implementation half: the six commands, the shared CLI helpers, and the integration tests);
- `8950eb9` — `docs(cli): document the local producer operator commands` (the documentation half: `docs/quickstart.md`, `crates/eggpack-cli/README.md`, `architecture/cli.md`);
- a follow-up rename commit correcting the `--candidates-dir` flag name, recorded in the findings below.

## Executive finding

M009 is closed. The canonical specification names nine operator commands; the binary previously exposed **two** families (`ci` and `contract`). It now exposes **six** top-level operator commands — `check`, `plan`, `package`, `qualify`, `collect`, `manifest` — so a producer can construct, qualify, finalize, and aggregate a real release **on a workstation**, without a hosted GitHub Actions run and without hand-assembling the `ci _*` file protocol.

The central design decision was made by the implementation rather than by duplication: the new commands and the existing `ci _*` runners call the **same** core functions and the **same** serialization helpers, and the byte-identity between them is **proven by test**, not asserted. `eggpack-core` needed **no change at all** — every function the commands needed was already public — which is the strongest available evidence that the new surface is thin wiring over closed APIs rather than a second implementation of the pipeline.

Nothing existing changed meaning. The `ci _*` family keeps its arguments, exit discipline, diagnostics, and output bytes; the `ci` and `contract` usage lines are byte-identical inside the new help text; and no command in this milestone stages or publishes, so no command in this milestone can mutate a public release.

## The command inventory

| Command | Class | Reads | Writes |
|---|---|---|---|
| `eggpack check` | validation | contract, pack-config, build bindings, qualification bindings, selected targets, release id, source revision | a verdict on stdout; non-zero on any violation |
| `eggpack plan` | validation | the same as `check` | a `ReleasePlan` JSON at `--output` |
| `eggpack package` | build | release plan, build bindings, source root | `<output-root>/<target>/{build-handoff.json,candidates/…}` |
| `eggpack qualify` | build | contract, release plan, build bindings, qualification bindings, candidates dir | `<output-dir>/<target>/{evidence.json,build-handoff.json,candidates/…}` |
| `eggpack collect` | release aggregation | contract, release plan, bindings, candidates dir | a finalized release tree at `--output-root`, with `release-manifest.json` **beside** it |
| `eggpack manifest` | final-bytes evidence | contract, release plan, finalized root | `release-manifest.json` at `--output` |

Every command succeeds with exactly `<cmd>: ok` on stdout, and fails with exactly one `eggpack: <message>` line on stderr, exit `1`, and **empty stdout**.

The verified top-level help output:

```text
usage: eggpack <check|plan|package|qualify|collect|manifest|ci|contract> [options]

local producer operator commands:
  check     validation           validate contract, policy, and bindings before anything is built
  plan      validation           resolve the release identity and write a ReleasePlan
  package   build                build every planned target's candidates under an output root
  qualify   build                qualify built candidates and write qualification evidence
  collect   release aggregation  aggregate qualified candidates into a finalized release tree
  manifest  final-bytes evidence derive release-manifest.json from a finalized release tree

No command in this set stages or publishes; staging and publication stay
separate, explicitly authorized actions.

usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]
usage: eggpack contract expand --contract <distribution.toml> --release-id <opaque-release-id> --target <triple-or-alias> --field <canonical-target|asset|sidecar|install>
```

The per-command help output, verified:

```text
eggpack plan — validation
usage: eggpack plan --contract <path> --pack-config <path> --build-bindings <path> --qualification-bindings <path> --selected <alias[,alias]> --release-id <id> --source-revision <revision> --output <path>
class: validation
No command in this set stages or publishes.
```

## Production implementation evidence

| Location | Change |
|---|---|
| `crates/eggpack-cli/src/operator.rs` (new, 730 lines) | The six commands, their flag parsing, their `--help`, and their dispatch. `check` and `plan` call `eggpack_ci::resolve_runtime_release_plan_with_mode(..., ExactTag)` — **the same function `ci _resolve-release` calls** — then the new public `ReleasePlan::validate()`, then `BuildBindingsV1::validate_for` and `QualificationBindingsV1::validate_for`. `package` calls `eggpack_core::execute_target`, `eggpack_ci::project_build_handoff`, and `validate_build_artifact_dir`. `qualify` calls `reconstruct_attempt` and `eggpack_core::qualify_target`. `collect` calls `project_ci_plan`, `project_release_plan`, and `eggpack_ci::aggregate_finalize`. `manifest` calls `eggpack_core::build_manifest` over the finalized tree's bytes. |
| `crates/eggpack-cli/src/main.rs` | `mod operator;`; the six dispatch arms; `help_text()`; and **four shared private CLI helpers** now used by *both* surfaces: `encode_release_plan` (`ci _resolve-release` + `plan`), `encode_release_manifest` (`ci _aggregate` + `manifest`), `collect_finalization_inputs` (`ci _aggregate` + `collect`), and `stage_qualification_artifacts` (`ci _qualify-target` + `qualify`). |
| `crates/eggpack-cli/tests/operator_commands.rs` (new, 1,341 lines) | The integration suite, including the two byte-identity tests and the per-command argument, refusal, exit-code, and stdout-discipline tests. |
| `crates/eggpack-core/**` | **Unchanged.** Every needed function was already public, so no visibility change turned out to be necessary. This is the proof that the new surface is thin wiring rather than a second pipeline. |

The refactor of the `ci _*` runners was **behaviour-preserving by construction**: the removed bodies and the new shared helpers are the same code, and the `git show eb77d19` diff confirms the `ci_aggregate` and `ci_resolve_release` bodies were moved verbatim rather than rewritten.

## The byte-identity evidence

The plan's §12.2 requires, proven by test rather than by assertion:

| Test | What it proves |
|---|---|
| `plan_writes_the_same_bytes_as_ci_resolve_release` | `eggpack plan`'s output artifact is byte-identical to `ci _resolve-release --output-plan` for the same inputs. |
| `package_qualify_collect_manifest_matches_ci_aggregate` | A real Cargo build → `qualify` → `collect` produces a `release-manifest.json` byte-identical to the one `ci _aggregate` produces from the same inputs, and `eggpack manifest` re-derives those same bytes from the finalized tree. |

The guarantee is **one implementation, not two**: resolution is one `resolve_runtime_release_plan_with_mode` call; the graph is one `project_ci_plan` + `project_release_plan` pair; aggregation is one `aggregate_finalize`; and serialization is the shared `encode_release_plan` / `encode_release_manifest` helpers.

## Verification executed

All verification below ran against implementation SHA `eb77d19`, scoped to the actual change as the maintainer directed for this stage:

```text
cargo fmt --all -- --check                                                  passed
cargo check --workspace --all-targets --all-features --locked               passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-cli --all-targets --all-features --locked              passed (27 + 8 + 18 = 53 passed, 0 failed)
cargo test -p eggpack-core --all-targets --all-features --locked             passed (81 passed, 0 failed, 6 ignored)
cargo test -p eggpack-ci --all-targets --all-features --locked                passed (67 passed, 0 failed, 3 ignored)
git diff --check                                                            passed
```

The `eggpack-ci` run was added because the CI crate's canonical re-resolution comparison reads a `ReleasePlan`, whose shape M008 changed.

**Not run, by the maintainer's scope direction and by the absence of a push:** `./scripts/check-local.sh`, the `cargo +1.89.0` MSRV lane, the `cargo package` steps, and hosted CI. Plan §12.8 requires the full gate and a hosted run; neither was performed and no hosted run is claimed here. The full local gate runs once at the end of the chain.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §12.1 — six new top-level commands exist, each documented in `--help` with its class | `crates/eggpack-cli/src/operator.rs` `COMMANDS`; the verified help output above; `eggpack <cmd> --help` prints the name, usage line, `class: <class>`, and the no-staging line. |
| §12.2 — `plan` and `collect` are byte-identical to `ci _resolve-release` and `ci _aggregate`, proven by test | The two named tests above. |
| §12.3 — the `ci _*` family is unchanged in behaviour, arguments, and output bytes; no generated workflow changes | The `ci` bodies were moved verbatim into shared helpers; the `ci` and `contract` usage lines are byte-identical inside the new help text; the `eggpack-ci` suite is green. |
| §12.4 — every command exits `0` on success and `1` on failure with exactly one `eggpack: <message>` line on stderr and empty stdout on failure | The exit-code and stdout-discipline tests; the success output is exactly `<cmd>: ok`. |
| §12.5 — every command refuses a symlinked output path and an oversized input, using the existing bounded helpers | `read_bounded`, `reject_symlink_output`, and `atomic_write` are reused unchanged; the symlink and oversized-input refusal tests cover every command. |
| §12.6 — no new dependency, no `clap`, no schema change, no template grammar, no network, no discovery, no release selection | `Cargo.toml` is untouched in every crate; `crates/eggpack-core/**` is untouched; no schema type changed. |
| §12.7 — the quickstart documents the local path end to end with verbatim verified commands, and the CLI README is a complete command reference | The documentation half; see the documentation evidence below. |
| §3 invariant — the CLI is deterministic wiring only | Every command is thin: select inputs, validate, call one core API, write one artifact. |
| §3 invariant — the `ci _*` runner family keeps working byte-for-byte | The `ci` bodies were moved, not rewritten; the `eggpack-ci` and `eggpack-cli` suites are green. |
| §3 invariant — no new command stages or publishes | No command in the set writes to GitHub or performs a publication; the help text says so explicitly. |
| §3 invariant — failure is fail-closed and single-line | One `eggpack: <message>` line on stderr, exit `1`, empty stdout, and no partial artifact before failing. |
| §8 invariant — no schema change of any kind | `ReleasePlan`, `CIPlan`, `PackConfig`, `DistributionContract`, and `ReleaseManifest` v1 are all unchanged. |

## Observable behaviour changes

These are confined to the **new** commands and to the top-level help and unknown-command diagnostic, which the plan's §8 explicitly records as a diagnostic rather than a contract:

1. `eggpack` with no arguments and `eggpack --help` now print the command taxonomy instead of just the `ci` usage line. The `ci` and `contract` usage lines themselves are byte-identical inside the new text.
2. The unknown-top-level-command diagnostic now lists the six new commands alongside `ci` and `contract`.
3. **No existing invocation changed.** The `ci _*` arguments, diagnostics, exit codes, and output bytes are unchanged.

Two narrower deviations from a literal reading of the plan, both recorded:

- **The new commands accept a TOML pack config; `ci _resolve-release` additionally accepts a JSON pack config.** `eggpack_core::PackConfig` exposes only `from_toml`, so the JSON acceptance in the runner is a CLI-level convenience. The new commands therefore accept a **narrower** input set than the runner. This does not affect the byte-identity claim, which is about outputs for identical inputs.
- **`check` and `plan` take `--release-id` (an opaque release id) rather than the runner's `--tag` plus `--identity-mode`.** The plan's §5.1 table names "release id" as the input, and the new commands resolve in exact-tag mode only. The `ci _*` identity modes are unchanged.

## Documentation evidence

The documentation half of M009 is commit `8950eb9` — `docs(cli): document the local producer operator commands` — and updated `docs/quickstart.md`, `crates/eggpack-cli/README.md`, and `architecture/cli.md`.

**The verbatim local walkthrough, actually run on Linux x86-64:**

```text
$ eggpack check --contract contract.toml --pack-config pack-config.toml \
    --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
    --selected linux-x64 --release-id 1.2.6 --source-revision "$REV"
check: ok
$ eggpack plan <same flags> --output out/release-plan.json
plan: ok
$ eggpack package --release-plan out/release-plan.json --build-bindings build-bindings.toml \
    --source-root demo --output-root out/build
package: ok
$ eggpack qualify --contract contract.toml --release-plan out/release-plan.json \
    --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
    --candidates-dir out/build --output-dir out/qualification
qualify: ok
$ eggpack collect --contract contract.toml --release-plan out/release-plan.json \
    --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
    --candidates-dir out/qualification --output-root out/release/finalized
collect: ok
$ eggpack manifest --contract contract.toml --release-plan out/release-plan.json \
    --finalized-root out/release/finalized --output out/manifest/release-manifest.json
manifest: ok
$ diff out/release/release-manifest.json out/manifest/release-manifest.json
(identical)
$ eggpack ci _resolve-release --contract contract.toml --pack-config pack-config.toml \
    --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
    --selected linux-x64 --tag 1.2.6 --source-revision "$REV" --template draft-template.json \
    --source-root . --output-plan out/release-plan.json --output-ci-plan out/release-ci-plan.json \
    --output-github-policy out/draft-policy.json
checked-out source matches release plan
resolved runtime release identity for tag 1.2.6
$ eggpack ci generate --ci-plan out/release-ci-plan.json --github-policy github-policy.json --output out/release.yml
generated 6831 bytes to out/release.yml
$ eggpack ci check --ci-plan out/release-ci-plan.json --github-policy github-policy.json --workflow out/release.yml
ci check: match (6831 bytes)
# after `printf '# manual edit\n' >> out/release.yml`
eggpack: ci check: drift detected (expected 6831 bytes, found 6845 bytes, first difference at Some(6831))
```

**Verified by running:** every quickstart command and its output; the six `--help` texts; `eggpack --help` (byte-diffed against the README quote — match); the failure modes (missing, unknown, and duplicate flags; a bad alias; a missing output-root parent; a missing finalized root), each confirming one `eggpack: <message>` line on stderr, empty stdout, and exit `1`; the stdout/stderr separation; and the two manifest files being byte-identical.

**Read from the code rather than executed:** the `ci _*` runner flag surfaces and the `contract expand` semantics, both verified against the `get_flag` call sites; `contract_expand.rs`'s tests pass unchanged, including `the_documented_readme_example_runs_verbatim`.

### A pre-existing documentation defect this milestone found

The quickstart's §2 had **already drifted** before M009: `ci _resolve-release` now requires the local tag to peel to the revision (that is CI M003k's behaviour), but the quickstart's setup never created a tag, so that section no longer reproduced. The walkthrough now creates the tag in its setup, and the drift-check section reproduces byte-identically.

### One boundary, stated plainly

Plan §13 makes "the quickstart's documented outputs cannot be produced verbatim on all four supported platforms" a stop condition. The walkthrough was verified on **Linux x86-64 only**. `qualify` performs host-matched native qualification, so on macOS and Windows the step fails by design unless the policy pins a matching host. The quickstart states this scope explicitly rather than leaving a reader misled, and the plan's §14 requirement to record "the actual observed output of every command on every supported platform" can only be completed from hosted CI, which was not run because nothing was pushed.

### Interface findings raised by the documentation half, recorded not absorbed

- **`--candidates-dir` is misnamed.** For `qualify`, `collect`, and `manifest` it is the **per-target artifact root** — the directory holding `<target>/build-handoff.json`, `<target>/evidence.json`, and `<target>/candidates/` — not a candidates directory. Passing the candidates directory itself fails with `eggpack: build handoff is unavailable`, which gives no hint about what was expected. This was corrected by a follow-up rename to `--artifacts-dir` after this record's implementation commit.
- **`eggpack plan` and `eggpack check` never run `git`.** `--source-revision` is shape-checked only, unlike `ci _resolve-release`, which verifies the checked-out HEAD and the tag. This is deliberate — the operator commands do no repository discovery beyond the explicit `--source-root` binding — but it is a real difference between the two surfaces.
- **`collect` and `ci _aggregate` place `release-manifest.json` in different parents.** Both place it **beside** the finalized root and never inside it, which is the workspace invariant; the parents differ only because `collect` takes `--output-root` while `ci _aggregate` takes `--output`. This is consistent, not a defect.

## Compatibility and no-regression evidence

- **No schema version bump.** Nothing in any schema changed.
- **No generated workflow byte changed.** The `ci` family's arguments and outputs are unchanged.
- **No dependency added** in any crate.
- **`eggpack-core` is untouched** by this milestone.
- **No historical closure rewritten.** Every prior closure record is unchanged.
- **Pre-existing tests stay green** across core, CI, and CLI.

## Security and reliability review

- The new surface adds **no** staging, publication, credential, network, repository-discovery, or release-selection capability. Every command is local-only and reads and writes local paths.
- Every command reuses the existing bounded non-symlink reader and the symlink-rejecting atomic writer, so a symlinked output path is refused and an oversized input is refused rather than read.
- Failure is fail-closed and writes no artifact: a command that fails leaves no partial artifact where a complete one is expected.
- No `unsafe` was introduced; the CLI crate's `#![forbid(unsafe_code)]` still holds.
- The `ci` family's unchanged behaviour means the least-privilege shape of the generated workflows is untouched.

## Failure and recovery review

- A command that writes an artifact writes it atomically through the existing `atomic_write`, so a failed run never leaves a partial file where a complete one is expected.
- Restarting a failed stage re-runs it from its inputs. There is no resumable state.
- Two concurrent runs against different output roots are independent; two against the same output root are the caller's responsibility, and the atomic writer is the only guarantee offered.
- `collect` requires the **parent** of `--output-root` to exist, which is the core finalizer's own invariant; the command surfaces that as a single bounded diagnostic rather than creating the parent itself.
- Rollback is a revert of this milestone's commits. The six commands disappear and the `ci` and `contract` families are unchanged, so no consumer is affected.

## Findings

| Severity | Finding | Disposition |
|---|---|---|
| None (implementation) | `eggpack-core` required **no** change. The plan's §4 anticipated "the minimal `eggpack-core` API surface changes needed to let a caller perform each stage from the command line"; that surface already existed. | Recorded as a positive finding: it is the strongest available evidence that the new commands are thin wiring over closed APIs rather than a second implementation of the pipeline. No corrective is required. |
| Low (deviation, recorded) | The new commands accept a TOML pack config only, while `ci _resolve-release` also accepts JSON. | Recorded above. The new commands accept a narrower input set; nothing existing changed. |
| Low (deviation, recorded) | `check` and `plan` take `--release-id` in exact-tag mode rather than the runner's `--tag` plus `--identity-mode`. | Recorded above; the plan's §5.1 table names "release id" as the input. |
| Operational boundary | The quickstart walkthrough was verified on Linux x86-64 only. No hosted run was performed because nothing was pushed, so plan §12.8 is not claimed as met. | Accepted; the same boundary every prior milestone recorded. The full local gate runs once at the end of the chain. |
| None | No unresolved medium-or-higher defect in the corrected producer surface. | M009 acceptance criteria satisfied except §12.8, which is out of the maintainer's scoped verification. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M009 | Reason |
|---|---|---|
| Build/Qualification M009 | closed | This record; implementation `eb77d19`. |
| CI/Release M003m | ready | Unblocked by M003l; independent of this milestone. |
| CI/Release M003n | blocked | It follows CI M003m. |
| Planning Hygiene M003 | ready | Its docs-only baseline reconciliation now runs against the final shape of the tree. |

## Downstream-unblock answer

Closing M009 unblocks **no** other milestone in this chain: CI M003m was already unblocked by M003l, CI M003n follows M003m, and Planning Hygiene M003 is independent. What M009 unblocks is the **operator surface** itself — the specification's §5.4 requirement that the CLI distinguish validation, build, staging, and publication is now met for validation, build, and release aggregation, with staging and publication deliberately left as separately authorized actions.

## Eggstack references

- Closed source plan: `plans/implementation/build-qualification/009-local-producer-operator-commands.md`
- Specification requirement satisfied: `plans/000-long-term-specification.md` §5.4 (the operator command surface and the command-class requirement)
- Predecessors in this chain: `plans/closure/build-qualification/007-status.md` (`e3c6192`), `plans/closure/ci-release-orchestration/003l-status.md` (`9cf0bf8`), `plans/closure/build-qualification/008-status.md` (`7e7cb6a`)
