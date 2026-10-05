# Contract Conformance Milestone 003 Closure — Bounded Direct-Contract Expansion CLI

Status: closed

Source plan: `plans/implementation/contract-conformance/003-bounded-direct-contract-expansion-cli.md`

Roadmap: `plans/subsystems/contract-conformance-roadmap.md`

Applicable ADRs: `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`, `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Reviewed baseline: `911da48c7c1c7967397a2d190730fc643c8c6dc3` (the plan's implementation baseline).

Implementation commits:

- `43fa2d7` — `feat: add eggpack contract expand bounded direct-contract CLI projection`, on branch `m003-contract-expand-cli`, fast-forwarded into `main`.

Reviewed external baselines (read-only, for the §11 parity proof):

- `eggstack/eggsact` at `d4e6e5c5cd7e9665373fbe312f4e1fb122fb5729`;
- `eggstack/stegoeggo` at `5662d56246f3bf71d328f76356cbc58f6819b6a2`, release `v0.5.0` -> `57ca94c910269e080b06aa8cb34c3767b3bf669d`; its `release/eggpack/distribution.toml` was verified byte-identical across `a01a022`, `v0.5.0`, and `origin/main`.

## Executive finding

M003 is closed and it delivered exactly the bounded projection the plan specified: one local command, four fields, one scalar per invocation, and no new schema.

The load-bearing design decision was **deliberate incompleteness**. Every capability a consumer might plausibly ask for next — repository discovery, release listing, conformance validation, bundle/archive list projection, non-direct target selection — was left out, and `architecture/cli.md` now records each omission with its reason, because the failure mode of this kind of milestone is silent scope growth rather than missing features. The one judgement the command does make is that `canonical-target` answers for every asset form while `asset`/`sidecar`/`install` fail closed for bundle and archive targets; a first implementation had this backwards and the test suite caught it before commit.

Two findings worth recording because they were not visible from the plan:

1. **The shared flag helper was the wrong tool for this surface.** `get_flag` returns the first occurrence of a repeated flag and tolerates unknown trailing arguments. That is acceptable for a renderer-driven `ci` family but wrong for a command whose stdout a consumer script will parse, so the command has its own exact parser rather than the historical commands being tightened incidentally. The plan's §7B anticipated this and the choice is now documented in `architecture/cli.md`.
2. **A real bug was caught by the first test run, not by review.** `project_field` initially matched on the asset form first and the field second, which made `canonical-target` fail for bundle and archive targets — exactly the opposite of the intent, and a `debug_assert` is what surfaced it in the test lane. Recorded as finding C-1 below.

Nothing was weakened to reach closure. The full local gate passes end to end, and the consumer-shaped parity proof runs against the real contracts of both adopted consumers rather than against a fixture that only looks like them.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §5.1 `eggpack contract expand` exists with exactly the four required flags | `crates/eggpack-cli/src/main.rs`: `contract_dispatch` (`:73`), `contract_expand` (`:295`), `parse_contract_expand_args` (`:181`). Wired into `dispatch` (`:21`, `:34-40`). |
| §5.2 local file input only | `read_bounded` (`:298`) with the 1 MiB bound (`:94`). No `Path::join`, no cwd read, no env read, no git invocation, no HTTP client anywhere in the new code. |
| §5.3 bounded deterministic diagnostics on stderr; nothing on stdout on failure | `run` (`:11-19`) prints exactly one `eggpack: <message>` line; `project_field` refuses an empty or over-`MAX_SCALAR_BYTES` value (`:278-285`); `bound_echo` (`:105`) caps argv-derived text at `:100` bytes. Proven at the process boundary in `every_failure_mode_exits_nonzero_with_an_empty_stdout`. |
| §5.4 stdout is exactly one scalar plus `\n` on success | `success_prints_exactly_one_scalar_and_nothing_else` asserts byte equality of stdout and an empty stderr, on two fixtures and four fields. |
| §5.5 opaque but filesystem-safe release id; no SemVer ordering or selection | `contract_expand` passes `--release-id` straight to `expand` (`:301`). `m003_release_id_is_opaque_and_accepts_prerelease_punctuation` accepts `1.2.3`, `v1.2.3`, `v1.2.3-rc.1`, `1.2.3-rc.1+build.5`, `nightly-2026-10-05` and rejects `a/b`, `a\b`, and empty. |
| §5.6 no new serialized expansion document | The only new serializable-free types are `ContractField` (`:120`) and `ContractExpandArgs` (`:161`), both private to the binary, neither `serde`. `m003_projects_fields_without_a_serialized_expansion_model` rejects `assets`, `entries`, `members`, `archive`, `ALL`, `canonical_target`. |
| §5.7 no general query language or framework | 4 `match` arms in `ContractField::parse` and one in `project_field`. No `clap`, no dependency added: `crates/eggpack-cli/Cargo.toml` is unchanged. |
| §5.8 unknown field, bundle/archive target, and non-direct selectors rejected | `ContractField::parse` (`:135-142`); `project_field` (`:261-269`); `m003_fails_closed_for_bundle_and_archive_direct_fields`; `canonical_target_is_valid_for_every_asset_form`. |
| §5.9 fails closed on unsupported schema version, invalid contract, unknown target | `parse_toml_str` and `expand` own those rules; `m003_rejects_invalid_schema_unknown_targets_and_missing_input` covers schema 2, unknown key, malformed TOML, four unknown triples/aliases, absent file, and a directory. |
| §5.10 symlinks and oversized inputs rejected | `read_bounded` refuses both; `m003_rejects_oversized_and_symlink_contract_input` and `symlinked_and_oversized_contracts_are_refused`. |
| §5.11 deterministic, no clock, locale, env, or absolute paths | No such input exists in the new code path. `repeated_invocations_are_byte_identical` runs the real binary repeatedly. |
| §5.12 the `ci` family and generated workflows are unaffected | No renderer, workflow, or binding changed. `git diff` touches `eggpack-cli` and `eggpack-contract` tests only. `m003_dispatch_adds_a_family_without_regressing_ci` pins `unknown ci subcommand` and the `ci` exit mapping; all 12 pre-existing `ci` tests still pass unmodified. |
| §5.13 the existing contract/conformance surface is not weakened | `DistributionContract` and every conformance type are untouched; `cargo test -p eggpack-contract` is green with one added test. |
| §5.14 CLI crate README documents the command, its bounds, its scope, and its exclusions | `crates/eggpack-cli/README.md`, including the local-only, opaque-release-id, direct-only-field, exact-argument-surface, read-only, and consumer-policy-stays-consumer-owned statements. |
| §5.15 README/docs and architecture updated with verified commands only | Root `README.md` example verified by running it against the contract `docs/quickstart.md` writes; `architecture/cli.md` gains a `contract expand` section and a full test table; `architecture/contract.md` records the consumer-facing entry point; `architecture/overview.md` line counts and citation state updated. |
| §6 CLI unit/integration tests | 10 unit tests in `main.rs` + 8 process-boundary tests in `crates/eggpack-cli/tests/contract_expand.rs`. |
| §6 direct/simple fixture coverage | `m003_expands_every_field_and_resolves_aliases`. |
| §6 alias resolution | Same test, each of the four fields queried through both a triple and an alias. |
| §6 opaque release id with prerelease punctuation | `m003_release_id_is_opaque_and_accepts_prerelease_punctuation`. |
| §6 all four fields | Same test plus `success_prints_exactly_one_scalar_and_nothing_else`. |
| §6 invalid contract/schema/unknown target | `m003_rejects_invalid_schema_unknown_targets_and_missing_input`. |
| §6 bundle/archive rejection for direct-only fields | `m003_fails_closed_for_bundle_and_archive_direct_fields` and `canonical_target_is_valid_for_every_asset_form`. |
| §6 oversized/symlink input | Two tests, one in-process and one at the process boundary. |
| §6 argument-count/unknown-field rejection | `m003_argument_parser_is_exact_and_rejects_ambiguity` and `argument_errors_exit_nonzero_without_a_scalar`. |
| §6 deterministic repeated output | `m003_never_mutates_the_contract_and_is_deterministic` and `repeated_invocations_are_byte_identical`. |
| §6 consumer-shaped comparison fixtures using the current Eggsact/StegoEggo contract shape | `crates/eggpack-contract/tests/fixtures/consumer-direct-targets.toml` is byte-identical to the adopted eggsact contract; `consumer_direct_targets_fixture_matches_the_live_consumer_shape` pins it; `m003_consumer_shaped_contract_matches_frozen_public_names` compares CLI output to the public names both consumers publish. |
| §6 README example is executable and guarded | `the_documented_readme_example_runs_verbatim` runs the documented command with its documented relative paths and working directory. |
| §7 no dependency, serialized model, or new file-safety helper added | `crates/eggpack-cli/Cargo.toml` unchanged; no new file-safety helper; `read_bounded` reused. |
| §7 local-only | No network-capable code in the new path. |
| §7 direct-only selector semantics | `project_field` (`:252-287`). |
| §7 deterministic error taxonomy | Four distinct diagnostics, each tested; `unknown field` names the supported set. |
| §7 safe release-id validation | Contract-owned `validate_version`; no duplicate validation added. |
| §8 the `ci` family is unchanged | `git diff --stat` shows no renderer/binding/workflow change; all historical tests pass; `unknown ci subcommand` preserved verbatim. |
| §8 direct-only selector semantics | As above. |
| §8 deterministic error taxonomy | As above. |
| §8 safe release-id validation | As above. |
| §9 `cargo fmt`, `check`, `clippy -D warnings`, per-crate and workspace tests, `doc`, `tree`, `package`, MSRV | `scripts/check-local.sh` exits 0. Detail in §Verification executed. |
| §9 fmt on the touched files | Enforced by the gate's `cargo fmt --all -- --check`. |
| §9 MSRV check and test | `cargo +1.89.0 check --workspace --all-targets` and `cargo +1.89.0 test -p eggpack-cli --all-targets`, both in the gate, both green. |
| §9 `cargo package` for the CLI crate | Green in the gate, with the required `--config patch.crates-io.*` flags copied from the script. The new integration test file is packaged and ignored by `exclude = ["tests/"]` in the same way as the contract crate's test files. |
| §10 the two adopted consumers must be able to consume it | Proven in §Consumer parity evidence below, against both real contracts. |
| §11 manual scratch proof against the consumer repos | Executed and recorded below. |
| §12 README claims verified by execution | Both documented examples executed; see §Documentation and operations evidence. |
| §13.1 CLI unit/integration tests | 18 tests added. |
| §13.2 direct contract fixture coverage | Met. |
| §13.3 consumer-shaped comparison fixtures | Met, against the byte-identical live contract shape. |
| §13.4 producer crate targets and aliases expanded deterministically | Proven by `m003_expands_every_field_and_resolves_aliases` and by the parity run against both consumers. |
| §13.5 generated workflow, ci generate/check, conformance, staged payload, release-manifest unchanged | Unchanged by diff; their suites pass unmodified. |
| §13.6 docs contain verified commands only | Both documented commands executed; see §Documentation and operations evidence. |
| §13.7 command output matches the current eggsact and stegoeggo contract shape for every published target | Proven below for both consumers, every target, every alias, all four fields. |
| §13.8 no unresolved medium-or-higher defect in contract expansion or the new CLI surface | See §Unresolved findings. |

## Production implementation evidence

Production code changed in exactly one file: `crates/eggpack-cli/src/main.rs`, +849 lines.

New private items:

- `USAGE` (`:44`) and `CONTRACT_USAGE` (`:47`) — the two family usage strings, lifted out of `dispatch` so both families and both help paths reuse one literal each. `USAGE`'s text is unchanged from the historical string.
- `ci_dispatch` (`:51`) — the historical `ci` subcommand table extracted verbatim, including the `unknown ci subcommand` diagnostic (`:68`). Extraction, not rewrite: the eleven arms and their messages are byte-identical to the previous inline match.
- `contract_dispatch` (`:73`) — `expand`, `help`, and two rejections.
- `MAX_CONTRACT_BYTES` (`:94`) and `MAX_SCALAR_BYTES` (`:100`).
- `bound_echo` (`:105`) — caps argv-derived diagnostic text.
- `ContractField` (`:120`) — the closed four-value field set.
- `ContractExpandArgs` (`:161`) and `parse_contract_expand_args` (`:181`) — the exact argument parser.
- `project_field` (`:252`) — the pure projection over an existing `ExpandedTarget`.
- `contract_expand` (`:295`) — read, expand, project, print.

Changed in `dispatch` (`:21-41`): the family match now accepts `ci` and `contract` and names the family in the error. An unknown top-level token now produces `unknown command "x": expected 'ci' or 'contract'` plus both usage lines, where it previously produced only the `ci` usage string. This is a diagnostic improvement on a path that previously existed only to be wrong; no test or workflow depended on the old text, and `architecture/cli.md` records the new behaviour.

Non-production changes:

- `crates/eggpack-cli/tests/contract_expand.rs` — new, 381 lines, 8 tests. This is the crate's first `tests/` directory.
- `crates/eggpack-contract/tests/fixtures/consumer-direct-targets.toml` — new test fixture, a byte-identical copy of the adopted eggsact contract.
- `crates/eggpack-contract/tests/fixtures.rs` — one added test pinning that fixture.
- `crates/eggpack-cli/README.md`, `README.md`, `architecture/cli.md`, `architecture/contract.md`, `architecture/overview.md` — documentation.

Not changed: `crates/eggpack-cli/Cargo.toml`, `Cargo.lock`, every other crate's source, every workflow, every generated artifact, and every consumer repository.

## Consumer parity evidence (§11 manual scratch proof)

The §11 requirement is a manual scratch comparison against the consumer repositories. It was executed with the built binary against the real contract files, not against a fixture:

```text
eggpack contract expand --contract <repo>/release/eggpack/distribution.toml \
  --release-id v1.2.7 --target <triple and each alias> --field <each field>
```

| Repository | Targets | Selector forms queried | Fields queried | Frozen public table match |
|---|---|---|---|---|
| `eggstack/eggsact` @ `d4e6e5c` | 5 | triple + 1 alias each | 4 | pass |
| `eggstack/stegoeggo` @ `v0.5.0` | 5 | triple + 1 alias each | 4 | pass |

Per repository: 5 targets x (triple + alias) x 4 fields = 40 queries, 80 total. Each `--field asset` result was compared against the consumer's own frozen public name for that target, each `--field sidecar` against `<asset>.sha256`, each `--field install` against the contract's `install`, and each `--field canonical-target` against the resolved triple. No discrepancy.

The specific names this exercise was meant to protect, now sourced from the producer rather than reconstructed by the consumer:

```text
eggsact-x86_64-unknown-linux-gnu            stegoeggo-x86_64-unknown-linux-gnu
eggsact-aarch64-unknown-linux-gnu          stegoeggo-aarch64-unknown-linux-gnu
eggsact-x86_64-apple-darwin                 stegoeggo-x86_64-apple-darwin
eggsact-aarch64-apple-darwin                stegoeggo-aarch64-apple-darwin
eggsact-x86_64-pc-windows-msvc.exe          stegoeggo-x86_64-pc-windows-msvc.exe
```

Note the `.exe` rule: it is a property of the contract, and both consumers' wrapper scripts reconstruct it today with a hand-written `*-pc-windows-msvc` substring test (recorded as low-severity F-1 in `plans/closure/bootstrap-installers/003-status.md`). `contract expand --field asset` removes the need to reconstruct it. That cleanup is consumer-side work and is **not** authorized by this closure.

The version-free public naming of both contracts means `--release-id` does not appear in the expanded asset name for these two consumers; the fixture-based tests cover the version-bearing form as well, since `eggsact-direct-targets.toml` and `simple-direct.toml` use `{version}` templates.

## Verification executed

Full local gate, `bash scripts/check-local.sh`, **exit 0**:

```text
cargo fmt --all -- --check                                                             passed
cargo check --workspace --all-targets --locked                                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings          passed
cargo test -p eggpack-contract --all-targets --all-features --locked                   passed (5 fixture + 11 conformance + 24 unit)
cargo test -p eggpack-github --all-targets --all-features --locked                      passed (32)
cargo test -p eggpack-ci --all-targets --all-features --locked                         passed
cargo test -p eggpack-cli --all-targets --all-features --locked                         passed (22 bin + 8 integration)
cargo test --workspace --all-targets --all-features --locked                           passed
cargo doc --workspace --no-deps --locked                                               passed
cargo tree -p <each of the seven crates> --locked                                      passed
cargo package -p <each of the seven crates> --locked --allow-dirty (+ patch flags)     passed (7 of 7)
cargo +1.89.0 check --workspace --all-targets --locked                                 passed
cargo +1.89.0 test -p <each crate> --all-targets --locked                              passed (7 of 7)
git diff --check                                                                        passed
```

Gate output contains 5 warnings, all pre-existing and none from this change: two are `eggpack-contract`'s own test files being excluded from the published package by `exclude = ["tests/"]`, and three are `yoke-derive v0.8.3` being yanked in the registry for a transitive dependency recorded in `Cargo.lock`.

Manual consumer parity proof: as recorded in §Consumer parity evidence above.

Documentation examples executed verbatim:

```text
$ eggpack contract expand --contract contract.toml --release-id 1.2.6 --target linux-x64 --field asset
eggsact-1.2.6-x86_64-unknown-linux-gnu
```

run against the contract block `docs/quickstart.md` instructs the reader to write, extracted and executed unmodified.

**Not performed, and therefore not claimed:** no hosted CI run was triggered by this milestone; no real consumer script or CI workflow was migrated to consume the command; no Windows or macOS lane was exercised locally — the portability lane for the new code is the same parser and reader the historical `ci` commands already use, plus one `#[cfg(unix)]` symlink test whose Windows counterpart (`reject_symlink_output` on a directory) remains a documented gap in `architecture/cli.md`; and no consumer repository was modified.

## Invariant review

| Invariant | Result |
|---|---|
| Contract remains the sole authority for layout and names | Held and reinforced. The command calls `DistributionContract` and adds no template grammar, resolution, or document. |
| The new command is a bounded local projection over existing schema-v1 semantics | Held. Four fields, one scalar, one local file, no I/O beyond a bounded read. |
| No new serialized schema or serialized expansion document | Held. The two new private types are not `serde`; no file is written at all. |
| No general query language or CLI framework | Held. No dependency added; `crates/eggpack-cli/Cargo.toml` is unchanged. |
| No network, repository discovery, or release selection | Held by construction: the new code contains no HTTP client, no `git` invocation, no cwd or env read, and no path joining. |
| Fail-closed validation model preserved | Held and extended. Every rejection path is a nonzero exit with an empty stdout. |
| Determinism and stable serialization preserved | Held. No clock, locale, env, absolute path, or iteration-order input. Repeated cross-process runs are byte-identical. |
| The `ci` family and generated workflows are unaffected | Held. Zero renderer, binding, workflow, or staging change; all historical tests pass unmodified; the `unknown ci subcommand` diagnostic is preserved byte-for-byte. |
| Consumer policy stays consumer-owned | Held. Nothing about latest/exact selection, Cargo fallback, install destination, updater behaviour, or compatibility floors moved, and the README states each exclusion explicitly. |
| Bounded file reading reused rather than duplicated | Held. `read_bounded` reused for the symlink refusal, regular-file check, and size bound. |
| Historical closure records are not rewritten | Held. `plans/closure/contract-conformance/001` and `002` untouched. |
| Line citations are re-based by symbol name, never arithmetic | Held, and proved rather than assumed — see §Documentation and operations evidence. |

## Failure/recovery review

- **A real defect was caught in the test lane, not in review.** The first `project_field` implementation matched on asset form before field, so `canonical-target` failed for bundle and archive targets; a `debug_assert` surfaced it immediately. The fix inverts the check: form-independent fields resolve first, and only the direct-only fields inspect the asset form (`:256-276`). Recorded as C-1.
- **The documented examples are guarded rather than trusted.** `the_documented_readme_example_runs_verbatim` executes the CLI README command with its documented relative paths and working directory, and the root README example was executed against the contract `docs/quickstart.md` writes. The plan's §7B stopping condition — an unguarded example — is therefore not merely avoided but actively regression-tested.
- **A re-basing defect was caught by verification, not by inspection.** The first re-basing pass shifted range end-numbers below 55 that should not have moved, turning `main (7-9)` into `main (7-263)`. The validation compared each cited line's *content* against the pre-change file before shifting, which is what made the error visible. The corrected pass verifies 397 anchors byte-for-byte and refuses to write on any mismatch. Recorded as D-1.
- **Rollback.** The change is one feature commit on one branch, fast-forwarded into `main`, plus the closure commit. Reverting `43fa2d7` restores the previous `ci`-only surface completely; no migration, no state, no consumer pin changes. The new command is additive, so nothing depends on it yet.
- **No contention or shared state.** The gate is hermetic; the integration tests use per-test temp directories under the OS temp root and set the child's working directory rather than the process's, so the parallel test lanes cannot race.
- **Consumer migration is reversible.** Nothing in either consumer repository was changed. A consumer that adopts the command changes one script's name-construction call and can revert it independently.

## Compatibility and migration review

- **Backward compatible for every existing caller.** The `ci` family is behaviourally and textually unchanged. A generated workflow rendered by any prior Eggpack revision invokes `eggpack ci ...` exactly as before.
- **Nothing depends on the new command.** No workflow, generated file, staged payload, manifest, installer, or consumer script references it. That is deliberate: M003 closed as an available capability, not a mandated migration.
- **MSRV-safe.** The new code uses only long-stable Rust (no let-chains beyond what the crate already uses, no `impl Trait` in argument position in new signatures, no 2024-edition features). Confirmed by the `+1.89.0` lane.
- **Packaging-safe.** `cargo package` succeeds for the CLI crate with the new `tests/` directory present; the crate's `exclude` handling behaves as it already does for the contract crate's tests.
- **Migration guidance for consumers** (owned by the consumer, not authorized here): replace a hand-written asset-name reconstruction such as `asset="stegoeggo-$target"` or `binary_name="eggsact-${target}"` with `eggpack contract expand --field asset`, keep the frozen `PUBLIC_ASSETS` compatibility table as the CI guard that compares against it, and retain every other behaviour — latest/exact selection, Cargo fallback, install destination, identity checks. The compatibility table is what makes that migration safe: a contract rename now fails the consumer's own check instead of silently shipping.

## Security review

No new attack surface of consequence, and the properties the plan asked for are enforced and tested:

- **Read-only and non-mutating.** Proven at the process boundary: `the_command_never_writes_to_the_contract_or_its_directory` compares the contract bytes and the directory entry count before and after both a successful query and four failing queries.
- **Bounded input.** 1 MiB, checked against file metadata before any allocation, via the shared reader.
- **No symlink traversal.** The symlink refusal is the reader's, tested in-process and at the process boundary.
- **No injection surface.** The expanded value is printed as a single `println!("{value}")`; it is a contract-validated name that the contract layer already restricts to flat, separator-free, control-character-free text, so no shell quoting is involved and the command never evaluates its output.
- **Bounded output.** A value that cannot be bounded is refused rather than printed (`:281-285`).
- **No secrets.** The command reads no environment variable, prints no path other than what the caller passed, and writes nothing. Its diagnostics quote only the caller's own argv, capped by `bound_echo`.
- **No privilege or lifecycle effect.** It runs with the caller's privileges and changes nothing outside the process.
- **Fail-closed on ambiguity.** A duplicate flag, an unknown option, an unknown field, an unknown target, or a non-direct field selection all exit nonzero with no partial value. This matters more than usual here because the output is intended for shell capture: a partially-correct name would be silently installed.

## Documentation and operations evidence

Updated and verified:

- `crates/eggpack-cli/README.md` — new `eggpack contract expand` section stating the local-only input, the exact-one-scalar stdout contract, the opaque release id, the direct-only field rule, the closed argument surface, read-only behaviour, and the explicit list of consumer policies this output must not be read as. The example is guarded by `the_documented_readme_example_runs_verbatim`.
- `README.md` — one verified example plus a pointer to the CLI README, placed next to the note that `generate` and `check` are the only user-facing `ci` commands. Output verified by execution against the `docs/quickstart.md` contract.
- `architecture/cli.md` — new `eggpack contract expand` section (argument contract, the delegation boundary, a table of five deliberately absent capabilities with reasons, the stdout contract, and the "consumer policy is not inferred" statement); a rewired wiring-rule table; a full rewritten Tests section including all 18 new tests; a Dependencies note explaining why the crate now has an integration-test surface.
- `architecture/contract.md` — the fixture count and the new fixture test; two fixture-table rows (`eggsact-direct-targets.toml`'s narrowed use, and the new `consumer-direct-targets.toml`); and, in Tools / capabilities, the consumer-facing entry point with its delegation boundary and the explicit statement that consumer policy may not be read out of its output.
- `architecture/overview.md` — `eggpack-cli` line count corrected to `3803 src + 381 integration tests`; the CLI card extended with the new family; the Citation-verification state extended.

### Citation re-basing, and how it was verified

`architecture/cli.md` is line-cited and this change moved `cli/src/main.rs` by `+254` for everything below the rewritten dispatch block. AGENTS.md requires re-basing by symbol name and warns that a blanket re-base corrupts as much as it fixes. So the shift was **proved before it was applied**:

1. Every line from the pre-change line 55 onward was compared against the post-change file at `+254`; 2901 of 2902 lines matched byte-for-byte, with the single difference being the file's final brace, replaced by the new test block. That is what makes the arithmetic safe for that region *and* what bounds it.
2. The 16 anchors inside the rewritten dispatch block were excluded from the shift and hand-written instead, then verified individually against the current source.
3. Each of the 397 shifted anchors was content-verified during substitution — the script asserts that the pre-change line it is moving is byte-identical to the post-change line it is moving it to, and refuses to write on any mismatch.
4. A symbol audit then compared every remaining citation against the declaration line of the symbol its prose names; 4 reported mismatches were inspected and all 4 were false positives of the heuristic, not defects.

That pass also found four citations in `cli.md` that were **already wrong before M003** and are now fixed, verified by content: `reject_symlink_output` and `atomic_write` each pointed 7-10 lines past its real declaration.

Residual drift was left in place and recorded rather than renumbered: the aggregate-sidecar, `release-manifest.json`-placement, path-absolutization call-site, and end-to-end-orchestration sub-anchors in `architecture/cli.md` all resolve to unrelated lines in the pre-M003 file as well, so M003 carried them forward faithfully instead of inventing numbers whose intent it could not confirm. `architecture/overview.md` §Citation-verification state now names them explicitly.

**Not changed:** canonical documents `plans/000`-`plans/002`, `plans/003-planning-process.md`, every ADR, and both historical contract closures. No ADR was required: this milestone moved no ownership boundary, no public schema, no publication authority, no trust model, and no release finalization semantics. It exposed an existing one through a CLI.

Operations impact: one new optional command. No workflow change, no configuration, no migration, no release-operator step.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| Low (C-1, fixed pre-commit) | The first `project_field` implementation matched asset form before field, so `canonical-target` failed for bundle and archive targets. | Fixed before commit by inverting the check and adding `canonical_target_is_valid_for_every_asset_form` in both the unit and integration suites. No residual. |
| Low (D-1, fixed pre-commit) | The first `architecture/cli.md` re-basing pass shifted range end-numbers below the rewrite point, corrupting `main (7-9)` into `main (7-263)`. | Fixed by restoring from `HEAD` and re-running with per-anchor content verification and an explicit end-number guard. Recorded because the failure mode — a plausible-looking wrong number — is exactly what AGENTS.md warns about. |
| Low (F-1, external) | Both consumers' wrapper scripts reconstruct the asset name (`binary_name="eggsact-${target}"`, `asset="stegoeggo-$target"` plus a `.exe` substring append) instead of deriving it. This command removes the need to, but only consumer-side work can delete it. | Not authorized by this closure. Eligible now for a plan in each consumer repository, which keeps its frozen `PUBLIC_ASSETS` table as the guard. Same finding as F-1/F-2 in `plans/closure/bootstrap-installers/003-status.md`. |
| Low (G-1, documentation) | `architecture/cli.md` retains pre-existing drift in sections M003 did not touch: the aggregate-sidecar and `release-manifest.json`-placement sub-anchors, the path-absolutization call sites, and the `1754` end-to-end sub-anchors. | Left unrenumbered on purpose and named in `architecture/overview.md` §Citation-verification state. Fixing them requires reading each cited line and confirming the intended anchor, which is a separate documentation pass. |
| Informational | `m003_consumer_shaped_contract_matches_frozen_public_names` covers the eggsact contract shape. StegoEggo's contract is structurally identical (same `{product}-{target}` template, same `.exe` rule) but is not duplicated as a fixture. | Accepted. Duplicating a second nearly-identical consumer fixture would add maintenance cost without adding a distinct case; the stegoeggo contract is exercised by the direct §11 parity run against the real file. |
| Informational | `git diff --check` and the gate's `fmt --check` are the whitespace gate; no line-ending or CRLF policy applies to the new Rust or Markdown files. | No action. CRLF-to-LF drift comparison remains a `ci check` concern for generated workflows only, and no generated file changed. |
| None | No unresolved medium-or-higher defect in contract expansion or the new CLI surface. | M003 acceptance criteria satisfied. |

## Roadmap disposition

| Milestone | Status after M003 | Reason |
|---|---|---|
| Contract M003 | closed | This record. Bounded projection implemented, tested, documented, and verified end to end; consumer parity proven against both adopted consumers. |
| Contract subsystem (roadmap §11) | complete | The authority-transfer portion was already complete, and M003 satisfied the remaining condition: the first-consumer evidence now has a bounded, closed implementation rather than a registered plan. Both authority transfer and the consumer projection are done. |
| Contract M001 / M002 | unchanged, closed | Untouched; no closure rewritten. |
| Bootstrap M003 | unaffected | Its closure explicitly declined any bootstrap dependency on this command; the F-1 cleanup it identified becomes *eligible* now, which is a consequence, not a blocker. |
| Ecosystem M003a | unaffected | Read-only scratch preflight over the Eggsearch target set; it does not consume `contract expand` and cannot be blocked by it. |

No milestone was unblocked by this closure and none was newly blocked. The single downstream effect is that each consumer's optional F-1/F-2 asset-name cleanup is now actionable, under a plan registered in that consumer's own repository.

## Downstream handoff

Contract conformance has no remaining registered work. Two things become possible that were not before:

1. **Consumer-side asset-name cleanup is now unblocked.** A consumer can replace its hand-written asset-name reconstruction with `eggpack contract expand --field asset`, keeping its frozen public-name table as the CI guard. This is consumer-owned work requiring a plan in each consumer repository; this closure does not authorize it and does not claim it.
2. **The mirrored Eggsearch adoption path has one fewer piece to invent.** Any future consumer that needs contract facts has a bounded, tested, read-only producer projection available, and a fixture shape to copy. Ecosystem M003a's design should assume this command exists.

## Registry updates

- Contract M003 row: `ready` -> `closed`, closure pointer `plans/closure/contract-conformance/003-status.md`.
- Contract subsystem roadmap: §11 completion recorded, M003 row closed, §7 M003 disposition updated.
- `plans/registry.md`: subsystem table row, the M003 row in the milestone index, the planned/blocked paragraph, the immediate execution graph, and the "Two Eggpack plans are ready" handoff section.
- `AGENTS.md`: current-handoff paragraph.

## Eggstack references

- Closed source plan: `plans/implementation/contract-conformance/003-bounded-direct-contract-expansion-cli.md`
- Roadmap: `plans/subsystems/contract-conformance-roadmap.md`
- Predecessor closures (historical, untouched): `plans/closure/contract-conformance/001-status.md`, `plans/closure/contract-conformance/002-status.md`
- Sibling subsystem closure that produced the F-1 finding this command makes actionable: `plans/closure/bootstrap-installers/003-status.md`
- Implementation: `crates/eggpack-cli/src/main.rs` (`contract_dispatch`, `contract_expand`, `parse_contract_expand_args`, `project_field`, `ContractField`), `crates/eggpack-cli/tests/contract_expand.rs`, `crates/eggpack-contract/tests/fixtures/consumer-direct-targets.toml`
- Documentation: `crates/eggpack-cli/README.md`, `README.md`, `architecture/cli.md`, `architecture/contract.md`, `architecture/overview.md`
- Consumer contracts used read-only for the parity proof: `eggstack/eggsact@d4e6e5c` `release/eggpack/distribution.toml`; `eggstack/stegoeggo@v0.5.0` `release/eggpack/distribution.toml`. No external commit, release, tag, or package state was created or changed.