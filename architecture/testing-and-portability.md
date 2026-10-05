# Test Topology and Portability — Deep Dive

This document answers two questions: whether the workspace's tests prove what
their names claim, and what will fail on a machine that is not a hosted CI
runner. It covers test placement, the seven ignored tests, fixture corpora, the
two hosted CI lanes, and the local gate script. Every claim is cited to source
or config; where a rationale is not written down, that absence is called out.

## Verified baseline

Measured at commit `fc072af` with
`cargo test --workspace --all-targets --all-features --locked`:
**219 passed, 0 failed, 7 ignored** across 9 test targets.

| Test target | Passed | Ignored | Ignored reason present? |
|---|---|---|---|
| `eggpack-bootstrap` unittests | 8 | 0 | — |
| `eggpack-ci` unittests | 59 | 3 | 2 of 3 |
| `eggpack-cli` unittests (`src/main.rs`) | 12 | 0 | — |
| `eggpack-contract` unittests | 24 | 0 | — |
| `eggpack-contract` `tests/conformance.rs` | 11 | 0 | — |
| `eggpack-contract` `tests/fixtures.rs` | 4 | 0 | — |
| `eggpack-core` unittests | 43 | 4 | 0 of 4 |
| `eggpack-github` unittests | 32 | 0 | — |
| `eggpack-manifest` unittests | 26 | 0 | — |

The "ignored reason present?" column is the finding that matters: five of the
seven ignored tests carry no written justification in code or adjacent comment.
Their reason is inferable from surrounding code but is nowhere stated.

## Where tests live

Two topologies coexist.

**Inline `mod tests`.** Ten modules declared inside `src/*.rs`:
`bootstrap/src/lib.rs:1051`, `ci/src/lib.rs:4708`, `cli/src/main.rs:1167`,
`contract/src/lib.rs:1727`, `core/src/lib.rs:586`, `core/src/builder.rs:887`,
`core/src/finalization.rs:508`, `core/src/qualification.rs:1244`,
`manifest/src/lib.rs:350`, and `github/src/lib.rs:3019` (the only one that
declares `mod tests;` out of line).

What this buys: tests can reach `pub(crate)` and private items without widening
visibility, which matters for crates that keep internal invariants private
(`qualification.rs` reaches `qualify_target_for_host` internals directly). The
cost is review burden. `eggpack-ci` carries 59 unit tests starting at
`crates/eggpack-ci/src/lib.rs:4708` inside a 10221-line `lib.rs` — roughly 5500
lines of test code in one file, with golden-rewriting tools and assertions
interleaved. `eggpack-cli` puts 12 tests in a 2940-line `main.rs`, which also
means the binary's tests only run through the binary target, never as a library.

**A test *support* module, not a `#[test]` host.**
`crates/eggpack-github/src/tests.rs` is 1848 lines that host no tests of their
own; it is helpers plus assertions built on `FixtureGithub`. Notably,
`FixtureGithub` itself is declared at `crates/eggpack-github/src/lib.rs:2655`,
which is **not** `#[cfg(test)]`-gated — the only `#[cfg(test)]` in that file is
the `mod tests;` at line 3018. The in-memory GitHub double, including its fault
injectors (`upload_rename_next`, `upload_digest_mismatch_next`,
`upload_size_mismatch_next`, `upload_fail_502_create_starter`,
`upload_fail_502_next`, `upload_fail_422_next`, around lines 2645-2651), is part
of the crate's public API rather than test-only code.

**Separate integration targets.** Only `eggpack-contract` has them:
`tests/conformance.rs` (11 tests) and `tests/fixtures.rs` (4 tests). Both load
fixtures through `env!("CARGO_MANIFEST_DIR")` + `tests/fixtures/`
(`conformance.rs:9-12`, `fixtures.rs:5-9`). Because they link the crate as an
external consumer, they can only reach the public surface — which is precisely
what makes them a useful check on the public API shape, and why the contract
validators are pinned there rather than inline.

## The ignored tests

All seven are `#[ignore]`d inside inline test modules. An ignored test is an
unproven claim: it compiles but never runs in the default gate.

**`eggpack-core/src/qualification.rs` — 4 self-invoking child targets**

| Test | Line | Reason written in code? |
|---|---|---|
| `qualification_child_target` | 2324-2326 | No |
| `qualification_child_sleep` | 2328-2332 | No |
| `qualification_child_output` | 2334-2338 | No |
| `qualification_child_nonzero` | 2340-2344 | No |

None carries a comment. The reason is structural, not written down: they are not
tests, they are *child programs*. The harness `run_native_child`
(`qualification.rs:2346`) copies the test binary to a candidate path, then
re-executes it with `--exact qualification::tests::<name> --ignored --nocapture`
(`qualification.rs:2369-2373`). Running them directly would either sleep 30
seconds (`qualification_child_sleep`, line 2331), print 16 KiB
(`qualification_child_output`, line 2337), or `std::process::exit(7)`
(`qualification_child_nonzero`, line 2343). They are driven by
`native_smoke_failure_timeout_cancellation_and_output_limit_are_typed`
(lines 2404-2426), and `qualification_child_target` is additionally used as a
trivially-succeeding argv target at lines 1701, 1792, and 2001.

**`eggpack-ci/src/lib.rs` — 3 golden regenerators**

| Test | Line | Reason written in code? |
|---|---|---|
| `m002a_regenerate_goldens` | 6167-6169 | Yes: "Regenerate checked-in M002a goldens … Run explicitly with `cargo test -p eggpack-ci --lib -- --ignored`" (6170-6171) |
| `m003b_regenerate_goldens` | 6775-6777 | **No** |
| `m005_regenerate_m001_multitarget_golden` | 10142-10144 | Yes: "Regenerate the M001 multitarget golden after the M005 exact-Zig check" (10145-10146) |

All three call the `write_golden` helper (`crates/eggpack-ci/src/lib.rs:4818`,
annotated `#[allow(dead_code)]`), which overwrites checked-in `.yml` files
under `tests/fixtures/`. They are ignored because they *write* into the source
tree — a default `cargo test` run must never mutate tracked files. Only
`m003b_regenerate_goldens` lacks a comment stating this, even though its
behavior is identical to its two siblings.

## Fixture corpora

**Declared contracts** — `crates/eggpack-contract/tests/fixtures/*.toml`, 7 files.
Three shape the layouts exercised by tests: `simple-direct.toml` (Direct),
`codegg-bundle.toml` (Bundle, 3 entries), `egress-archive.toml` (Archive, 2
members, `.tar.gz`). `eggsact-direct-targets.toml` is an additional input used
by the CI crate. All are **positive** evidence: they parse and must be accepted.

**Observed-vs-declared pairs** — `observed-simple.toml`, `observed-codegg.toml`,
`observed-egress.toml`, deserialized as `ObservedTargetMapping` by a separate
loader (`conformance.rs:14-19`). They are the declared half of the conformance
comparison: `golden_observation_fixtures_conform_for_all_layouts`
(`conformance.rs:44-50`) walks all three pairs. Conformance *negative* evidence
is **not** stored as fixture files; it is constructed programmatically in
`conformance.rs` (for example pushing an unexpected `source.tar.gz` onto an
inventory to prove `ExtrasPolicy::AllowExtras` behaviour, `conformance.rs:36-45`).

**Rendered workflow goldens** — `crates/eggpack-ci/tests/fixtures/*.yml`, 9
files: `m002-{direct,mixed,bundle,archive}.yml`,
`m003b-{direct,bundle,archive}-staging.yml`, `native-direct.yml`,
`native-direct-multitarget.yml`, plus one input `mixed-direct-targets.toml`.
These are golden files: `assert_golden` (`crates/eggpack-ci/src/lib.rs:4811-4813`)
compares rendered output with exact string equality after normalising CRLF to
LF. CI workflow rendering is therefore pinned against expected output, and any
renderer change surfaces as a diff against these nine files rather than as a
semantic assertion failure.

**Eggup interoperability fixtures** — `plans/closure/eggup-interoperability/fixtures/`,
10 JSON files, consumed by `eggpack_interoperability_manifest_fixtures_are_valid_v1`
(`crates/eggpack-manifest/src/lib.rs:639-700`) via `include_str!` with relative
paths crossing out of the crate. This is the clearest positive/negative split in
the repo:

- Positive, round-tripped byte-for-byte: `direct-manifest.json`,
  `bundle-manifest.json`, `archive-manifest.json` (lines 640-653 assert
  `to_json()` output equals the trimmed input).
- Negative, asserted `is_err()`: `unknown-schema.json` (654-657),
  `corrupt-digest.json` (658-661), `corrupt-size.json` (662-665).
- Projection: `projection-{direct,bundle,archive}.json`; only
  `projection-archive.json` is referenced (693-699), checking
  `extraction_required == true`, one acquisition unit, two members.
- `wrong-target.json` exists on disk but is **not** referenced by any test. The
  wrong-target case is instead asserted inline at line 670
  (`direct.target("x86_64-pc-windows-gnu").is_err()`). The file is a spare.

**External backend evaluation fixtures** —
`plans/closure/external-backend-evaluation/fixtures/` holds three real Cargo
projects (`direct`, `sibling-bundle`, `archive-pair`, each with `Cargo.toml`,
`Cargo.lock`, and sources). A workspace-wide search finds **no** reference to
this path from any `.rs`, `.sh`, or workflow file. These are closure evidence
from a completed evaluation, not live test inputs; nothing regresses if they
drift.

## What is not unit tested

Coverage is deliberately heavy on pure logic and error paths — contract parsing
and expansion, manifest canonicalisation, workflow rendering, planning,
fail-closed validation — and lighter exactly where the portability lanes take
over:

- **No real network.** The `GithubApi` trait boundary
  (`crates/eggpack-github/src/lib.rs:1674`) is implemented by `FixtureGithub`
  (2818) only. `api.github.com` is never contacted, so HTTP semantics, real
  pagination, and real rate limiting are unproven. What *is* proven is the
  draft-only tag discipline and upload/digest/size mismatch handling, because
  the fixture can inject each of those faults.
- **No published crates.** Path dependencies are not on crates.io yet, so every
  cross-crate test links workspace sources. `cargo package` (local gate only)
  is the sole check that the crates are actually packageable.
- **No real zigbuild / cross-compile.** Real Cargo invocation appears only in
  `eggpack-core/src/builder.rs`, serialized by a process-wide
  `CARGO_FIXTURE_LOCK` mutex (line 892) that both real-Cargo tests take
  (1127, 1247). `real_local_cargo_fixture_builds_a_direct_candidate` (1246)
  builds a trivial `smoke-fixture` crate on the host: it proves the bounded
  execution path works, not that a declared glibc or macOS floor holds.
- **A pass that may be a no-op.** `m002a_powershell_archive_runtime`
  (`crates/eggpack-bootstrap/src/lib.rs:3084`) asserts pwsh 7 and `tar.exe` on
  Windows (3089-3094) but on any other platform *returns early* when either is
  missing (3095-3097, comment: "regression guard, not qualification evidence").
  It is counted as one of the 8 passing bootstrap tests even when it did nothing.
  A silent success, not an `#[ignore]`, so no test listing reveals it.

## The hosted CI lanes

`.github/workflows/ci.yml` is 86 lines, `permissions: contents: read`
(lines 8-9), triggers on push, pull_request, and workflow_dispatch. Both jobs
set `fail-fast: false` (lines 15, 35), so a 1.89.0 or Windows failure does not
mask a stable result.

### `linux` (lines 12-31)

`ubuntu-latest`, matrix `toolchain: [stable, "1.89.0"]`. Three steps are
`matrix.toolchain == 'stable'` only: `cargo fmt --all -- --check` (24-25),
`cargo clippy … -- -D warnings` (28-29), `cargo doc --workspace --no-deps` (30-31).
Both toolchains run `cargo check --workspace --all-targets --locked` (26) and
`cargo test --workspace --all-targets --all-features --locked` (27). The gating
is a latency choice: formatting, lints, and docs are toolchain-independent
outputs, so running them twice buys nothing.

### `portability` (lines 33-86)

`macos-latest` and `windows-latest`, `dtolnay/rust-toolchain@stable` only —
**no MSRV lane on macOS or Windows**. Every step from line 43 to line 84 is
`matrix.os == 'windows-latest'`-gated; both OSes then run plain
`cargo check` (85) and `cargo test --workspace --all-targets --all-features --locked`
(86). The Windows-only steps, in order:

| Step | Lines | Why it exists |
|---|---|---|
| `ilammy/msvc-dev-cmd@v1` (arch x64) | 43-46 | Rust's MSVC toolchain needs an initialized developer environment before any Cargo build; without it `link.exe` is absent and the real-Cargo tests fail with a linker error, not a code failure |
| Verify initialized MSVC linker environment | 47-56 | Fails fast and legibly: throws if `VCToolsInstallDir` is unset, and throws if `link.exe` resolves *outside* that directory. Guards against silently picking up a different `link.exe` on `PATH` |
| Windows builder real Cargo candidate smoke | 57-59 | `real_local_cargo_fixture_builds_a_direct_candidate`; the only lane that proves a real Cargo build links on Windows |
| Process-group timeout and cancellation | 60-62 | `timeout_kills_and_waits_for_the_process_group` (`builder.rs:1126`); process-group/job semantics differ most on Windows |
| Core tests serialized for diagnosis | 63-65 | `-- --test-threads=1`. A serialization workaround: the suite exhibits nondeterministic failures under parallel execution on Windows, so the Windows lane trades coverage speed for signal |
| Windows CI crate tests | 66-68 | `-p eggpack-ci` on Windows — the renderer has `cfg(windows)` branches at 4033, 4245, 4349, 7729, 7887, 7917 that are dead code on Linux |
| Verify pwsh 7 + `tar.exe` | 69-75 | `tar.exe` ships in the Windows image but is not guaranteed; the archive qualification lane requires both, so prerequisite failure is separated from test failure |
| Focused PowerShell archive runtime | 76-78 | `m002a_powershell_archive_runtime`; on Windows it must assert, never skip |
| Generated-orchestration end-to-end | 79-81 | `m002a_generated_orchestration_executes_end_to_end`; shell-out paths that Linux does not exercise |
| Orchestration CLI harness | 82-84 | `generated_orchestration_cli_executes_capture_to_aggregate`; CLI-level wiring on Windows |

## The local gate

`scripts/check-local.sh` is 53 lines, `set -euo pipefail`, and runs strictly in
order:

1. `cargo fmt --all -- --check` (line 4)
2. `cargo check --workspace --all-targets --locked` (5)
3. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` (6)
4. Per-crate tests: `contract`, `github`, `ci`, `cli` (7-10)
5. `cargo test --workspace --all-targets --all-features --locked` (11)
6. `cargo doc --workspace --no-deps --locked` (12)
7. `cargo tree` per crate, all seven (13-19)
8. `cargo package` per crate (20-45)
9. MSRV lane: `cargo +1.89.0 check --workspace --all-targets --locked` (46) then
   `+1.89.0 test` per crate (47-53)

Steps 4 and 5 are redundant by design: the per-crate runs give a fast, precisely
located failure, and the workspace run catches cross-crate interaction.

**Why `cargo package` needs path patches.** Every non-leaf crate depends on
siblings by path, and no `eggpack-*` crate is published to crates.io yet.
`cargo package` resolves dependencies from the registry by default, so it cannot
resolve a sibling. The script therefore injects
`--config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"'` and
one override per unpublished path dependency, escalating as the dependency graph
deepens: `core` and `bootstrap` patch two (22-27), `github` four (34-38), `ci`
five (28-33), `cli` six (39-45). Only `contract` and `manifest`, the true leaves,
package bare (20-21). **Never run bare `cargo publish`.**

**Why `--allow-dirty`.** `cargo package` refuses to package from a working tree
with uncommitted changes unless overridden. A local developer run is almost
always mid-change, so the flag is present on every invocation (20-45). It is a
convenience flag, not a safety relaxation for CI — CI does not package at all.

## MSRV and toolchain

- `rust-version = "1.89"`, `edition = "2021"`, `resolver = "2"` (`Cargo.toml`,
  `[workspace]` / `[workspace.package]`).
- Workspace lints: `unsafe_code = "deny"`, and
  `clippy::all = { level = "warn", priority = -1 }` (`[workspace.lints.*]`).
- All seven crates additionally harden the crate root with
  `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` — for example
  `crates/eggpack-core/src/lib.rs:1-2`, `crates/eggpack-ci/src/lib.rs:1-2`,
  `crates/eggpack-github/src/lib.rs:9-10`. `deny(missing_docs)` is why the
  public `FixtureGithub` carries rustdoc while most inline test helpers do not.
- There is **no** `rust-toolchain.toml`. The MSRV is enforced only by the
  explicit `cargo +1.89.0` invocations, so a developer on stable gets no
  automatic warning that they have moved past 1.89.

## How to reproduce

Fast loop:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test -p <crate> --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Single test:

```bash
cargo test -p eggpack-core --all-targets --all-features --locked \
  real_local_cargo_fixture_builds_a_direct_candidate -- --nocapture
```

Full local gate (all 53 lines, including `cargo package` and the MSRV lane):

```bash
scripts/check-local.sh
```

Windows serialization, for diagnosing a test that passes locally and fails on a
parallel Windows run — this is the same workaround the Windows lane uses:

```bash
cargo test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1
```

Regenerating a golden (these are the ignored tests, run deliberately):

```bash
cargo test -p eggpack-ci --lib -- --ignored
```

## Related deep dives

- [overview.md](overview.md) — module map and producer pipeline
- [core.md](core.md) — planning, build, qualify, finalize
- [core-build.md](core-build.md) — builder and candidate construction
- [process-execution.md](process-execution.md) — bounded execution and process groups
- [validation-model.md](validation-model.md) — fail-closed validation
- [determinism.md](determinism.md) — canonical ordering and byte-identical renders
- [ci.md](ci.md) — workflow rendering internals
- [cli.md](cli.md) — command wiring
- [contract.md](contract.md) — layout authority and conformance
- [manifest.md](manifest.md) — release manifest evidence and Eggup mapping
