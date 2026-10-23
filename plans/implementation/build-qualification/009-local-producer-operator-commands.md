# Build/Qualification M009 — Local Producer Operator Commands

Status: **ready for implementation**.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/build-qualification-roadmap.md`.
Primary class: capability / specification-gap corrective.
Historical upstream authority: `plans/000-long-term-specification.md` §5.4
`eggpack-cli` (`:171-185`), which names the intended operator surface and
requires that *"CLI commands MUST distinguish validation, build, staging, and
publication. A command that mutates a public release MUST be explicit."*
Also `plans/subsystems/build-qualification-roadmap.md` §11 completion
definition: *"Eggpack can construct and qualify real direct/bundle/archive
native releases **locally** or via accepted backend with accurate evidence
classification."*
Dependency: none that blocks execution. Contract M001/M002/M003, Release
Manifest M001/M001a/M002/M003a, and Build/Qualification M001–M006 are all
closed, so the underlying APIs are stable. This milestone does not touch
`crates/eggpack-ci/**`, so it does not contend with CI M003k (in flight).

## 1. Objective

Restore the operator surface the canonical specification names, so a producer
can construct, qualify, finalize, and aggregate a real release **on a
workstation**, without a GitHub Actions run and without the `ci _*` internal
runner family.

Specifically, add six top-level local commands:

| Command | Class | Produces |
|---|---|---|
| `eggpack check` | validation | a validation verdict over contract, policy, and bindings |
| `eggpack plan` | validation | a `ReleasePlan` (intent) |
| `eggpack package` | build | build candidates |
| `eggpack qualify` | build | qualification evidence |
| `eggpack collect` | release aggregation | a finalized release tree |
| `eggpack manifest` | final-bytes evidence | a `ReleaseManifest` v1 |

Each command is deterministic wiring over an already-closed `eggpack-core` API.
None adds a schema, a template grammar, a discovery mechanism, a network call,
or a release-selection policy.

## 2. Current evidence and the gap

### 2.1 The specification names nine commands; seven do not exist

`plans/000-long-term-specification.md:171-185` gives the operator surface as:

```text
check
plan
package
qualify
collect / release aggregation
manifest
installer generate/check
ci generate/check
staging/draft-release commands when later enabled
```

The CLI's actual top-level dispatch (`crates/eggpack-cli/src/main.rs:35-41`)
accepts exactly two families — `ci` and `contract`:

```rust
match args[0].as_str() {
    "ci" => ci_dispatch(&args[1..]),
    "contract" => contract_dispatch(&args[1..]),
    other => Err(format!("unknown command {other:?}: expected 'ci' or 'contract'\n{USAGE}\n{CTRACT_USAGE}")),
}
```

`ci generate` and `ci check` exist. `ci` also carries the nine internal
`ci _*` runners (`crates/eggpack-cli/src/main.rs:52-71`). `contract expand`
exists (Contract M003). **`check`, `plan`, `package`, `qualify`, `collect`, and
`manifest` do not exist at all**, and `installer generate/check` does not exist.

### 2.2 The pipeline stages are reachable only through internal runners

`crates/eggpack-cli/src/main.rs` exposes the build, qualification, and
aggregation stages solely as `ci _capture-build`, `ci _qualify-target`,
`ci _evaluate-gate`, `ci _aggregate`, and their siblings. Those are, by the
repo's own definition, *"file-in/file-out wrappers, not a scripting interface"*
(`AGENTS.md`, and `crates/eggpack-cli/README.md:8`).

That design is correct for what it is: the generated workflow needs a
deterministic, file-in/file-out step protocol that a YAML runner can invoke.
It is the wrong surface for a human operator, because:

- the caller must hand-assemble a specific directory layout and a specific set
  of intermediate files for each stage, with no command that validates the
  layout or reports what is missing;
- the stages are named after their *workflow step*, not after what they do, and
  the naming leaks the internal protocol into the operator's vocabulary;
- the workflow steps and the operator actions cannot share an implementation,
  so the operator path would be a second, divergent implementation of the same
  pipeline — precisely the class of duplication CI M003l/M003n are removing
  elsewhere.

### 2.3 The documented local path stops before the pipeline

`docs/quickstart.md` walks resolve → generate → check and then states at
`:216-217`: *"After planning come build → qualify → finalize → stage; the
generated workflow replays exactly these steps in CI."* There is no documented
way to run those steps locally. A producer cannot rehearse a release on a
workstation, cannot reproduce a CI failure locally, and cannot inspect a
finalized tree without a hosted run.

### 2.4 The specification's command-class requirement is not yet met

`plans/000-long-term-specification.md:185` requires that CLI commands
distinguish **validation, build, staging, and publication**, and that a command
mutating a public release be explicit. Today the `ci _*` family collapses
build, qualification, aggregation, and staging into one flat namespace of
underscore-prefixed names. The taxonomy the specification asks for does not
exist in the command surface.

## 3. Invariants

- The CLI remains **deterministic wiring only**. No new dependency, no `clap`,
  no template grammar, no serialized model, no repository discovery, no
  network, no release selection, no consumer policy.
- Each new command is **deterministic**: identical inputs produce byte-identical
  outputs, and the same bounded-reader/bounded-writer helpers the existing
  commands use are reused unchanged.
- The command taxonomy maps onto the specification's four classes —
  validation, build, staging, publication — and each command declares its
  class in `--help`. **No new command in this milestone performs staging or
  publication**, so no command in this milestone can mutate a public release.
- The `ci _*` runner family keeps working **byte-for-byte**. Generated
  workflows depend on its exact argument surface and exit discipline, and this
  milestone must not perturb a single generated byte.
- The new commands are **thin**: they select and validate inputs, call one
  `eggpack-core` API, and write one artifact. They contain no pipeline logic of
  their own. The pipeline logic lives in `eggpack-core`, which owns it.
- Failure is fail-closed and single-line: exit non-zero with one
  `eggpack: <message>` diagnostic on stderr, and nothing on stdout.
- Every new command is **local-only**: it reads and writes local paths, spawns
  no network activity, and performs no repository discovery beyond the
  explicit `--source-root`/`--source-revision` binding that already exists.
- Producer/consumer ownership is preserved: this milestone adds producer
  operations. It does not add installation, update, rollback, receipt, or
  latest/exact selection semantics, which remain Eggup's and the product's.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-cli/**` — the six new top-level commands, their
argument parsing, their `--help` text, their tests, and the CLI README. Plus
the minimal `eggpack-core` API surface changes needed to let a caller perform
each stage from the command line (for example, exposing a stage that is today
only reachable through a CI runner's file protocol).

**Out of scope:**

- `installer generate/check`. The Bootstrap subsystem owns installer
  rendering; a separate Bootstrap milestone must own that command. This
  milestone must not render installers.
- `eggpack stage` and any staging/draft-release command. Staging writes to
  GitHub, and publication authority is gated by ADR-0003. The staging
  capability exists today as `ci _prepare-stage` and `ci _stage-github-draft`;
  exposing a user-facing staging command is a separate decision with its own
  evidence bar, because it is the first operator command with a credential and
  a network effect.
- Any change to the `ci _*` runners' behaviour, arguments, or output bytes.
- Any change to `ReleaseManifest` v1, `PackConfig` v1, `CIPlan` v1, or any other
  schema.
- Any change to `eggpack-core`'s pipeline semantics, evidence semantics, or
  validation rules. This milestone adds an *entry point*, not behaviour.
- Release selection, "latest" resolution, version comparison, Cargo fallback,
  or any other consumer policy.
- A general configuration or workflow language.

## 5. Required production changes

### 5.1 The command set and its classes

Each command is specified by behaviour, not by an exact flag list. The
following names and classes are required; the flag surface follows the
existing convention of `--flag value` and `--flag=value`, each flag required
exactly once, with unknown flags, duplicate flags, empty values, and
positional arguments rejected.

| Command | Class | Primary inputs | Primary output |
|---|---|---|---|
| `eggpack check` | validation | contract, pack-config, build bindings, qualification bindings | a verdict; non-zero on any violation |
| `eggpack plan` | validation | contract, pack-config, bindings, selected targets, release id, source revision | `ReleasePlan` JSON |
| `eggpack package` | build | plan, bindings, source root | build candidates under a caller-named output root |
| `eggpack qualify` | build | plan, bindings, candidates, host | qualification evidence JSON |
| `eggpack collect` | release aggregation | plan, qualified candidates | a finalized release tree |
| `eggpack manifest` | final-bytes evidence | contract, finalized tree | `release-manifest.json` |

`eggpack plan` and `eggpack collect` are the two commands that must reflect
the existing `_resolve-release` and `_aggregate` behaviour closely enough that
the same inputs yield the same artifacts. Where a `ci _*` runner already
performs a stage, the new command and the runner MUST call the **same**
`eggpack-core` function. Two implementations of a stage is exactly the
duplication this milestone exists to prevent.

### 5.2 Reuse, not reimplementation

- The bounded non-symlink reader (`crates/eggpack-cli/src/main.rs:553`
  `read_bounded`), the symlink-rejecting output writer (`:581`
  `reject_symlink_output`, `:617` `atomic_write`), and the single-line failure
  discipline are reused unchanged.
- The `eggpack ci generate` / `ci check` / `contract expand` paths are
  untouched.
- Where `eggpack-core` currently exposes a stage only as an internal or
  CI-shaped function, the minimal public surface is added so both the `ci _*`
  runner and the new operator command call it. This is a visibility change, not
  a behaviour change.

### 5.3 Command-class visibility

`--help` states each command's class (validation / build / release
aggregation / final-bytes evidence) and states plainly that no command in this
set stages or publishes. This is the specification's `:185` requirement
expressed at the surface where an operator reads it.

### 5.4 Exit discipline

Every command exits `0` on success and `1` on failure with exactly one
`eggpack: <message>` line on stderr and an empty stdout. No command may print
a partial artifact before failing.

## 6. Ordered work packages

### WP1 — Failing discriminating tests first

Integration tests that fail against the current tree:

- `eggpack check` on a valid contract/policy/binding set exits `0`; on a
  contract with an unknown field, a wrong `schema_version`, an uncovered
  target, or an extra binding, it exits non-zero with one bounded diagnostic
  and an empty stdout;
- `eggpack plan` on the same inputs produces a `ReleasePlan` that is
  **byte-identical** to the plan the existing `ci _resolve-release` produces
  from the same inputs;
- `eggpack package` → `eggpack qualify` → `eggpack collect` → `eggpack
  manifest` produces a `release-manifest.json` that is byte-identical to the
  one the existing `ci _aggregate` produces from the same inputs;
- every command rejects an unknown flag, a duplicate flag, an empty value, a
  positional argument, a symlinked output path, and an oversized input;
- every command writes nothing to stdout on failure;
- `--help` names each command and states its class.

### WP2 — Minimal `eggpack-core` surface

Expose, or add thin wrappers for, the core functions the six commands need so
that the `ci _*` runners and the new commands call the same code. No pipeline
logic is written in the CLI.

### WP3 — Command implementations

Implement the six commands as thin, deterministic wiring. Each parses and
validates its arguments, calls one core API, and writes one artifact through
the existing bounded writer.

### WP4 — Command taxonomy in `--help`

Restructure `USAGE` (`crates/eggpack-cli/src/main.rs:45`) to show the command
families and their classes, while keeping the existing `ci` and `contract`
usage text intact so no documented invocation changes meaning.

### WP5 — Documentation

Rewrite `docs/quickstart.md` to walk the **local** path end to end —
`check` → `plan` → `package` → `qualify` → `collect` → `manifest` — with every
command and its expected output run verbatim, per the quickstart's existing
contract. Keep the existing CI rendering and drift-check sections. Update
`crates/eggpack-cli/README.md` to become the actual full command reference
that `docs/quickstart.md:212` already promises, including the `ci _*` runner
flag surface. Update `architecture/cli.md`.

## 7. Failure, restart and contention semantics

- Every command is fail-closed: on any validation failure it writes no
  artifact and exits non-zero.
- A command that writes an artifact writes it atomically through the existing
  `atomic_write`, so a failed run never leaves a partial file where a
  complete one is expected.
- A command never overwrites an existing artifact silently. It either refuses
  or replaces atomically, matching the existing `ci generate` behaviour
  explicitly, and the choice is documented per command.
- Local outputs are written under a caller-named root. The caller owns that
  root; Eggpack does not create or delete it outside the explicit output
  paths.
- Restarting a failed stage re-runs it from its inputs. There is no resumable
  state.
- Two concurrent runs against different output roots are independent. Two
  concurrent runs against the same output root are the caller's responsibility,
  and the atomic writer is the only guarantee offered.

## 8. Compatibility and migration

- **No generated workflow byte changes.** The `ci _*` runner arguments, exit
  discipline, and diagnostics are unchanged, so `eggpack ci check` drift status
  is unchanged for every existing consumer.
- No schema change of any kind: `ReleasePlan`, `CIPlan`, `PackConfig`,
  `DistributionContract`, and `ReleaseManifest` v1 are unchanged.
- No new dependency. `eggpack-cli` already depends on `eggpack-core`,
  `eggpack-contract`, `eggpack-manifest`, and `serde_json`.
- The `eggpack` binary's existing invocations keep working unchanged. The
  error text for an unknown top-level command changes, because the set of
  known commands changes; that message is a diagnostic, not a contract, and the
  change is recorded here.
- `eggup-eggpack` and every consumer are unaffected: no manifest changes.

## 9. Required tests

- per-command argument validation tests: missing flag, duplicate flag, unknown
  flag, empty value, positional argument, `--flag=value` form;
- per-command symlink and oversized-input refusal tests;
- the byte-identity tests from WP1 against the corresponding `ci _*` runner
  outputs;
- a process-boundary test that each command's stdout on success is exactly
  what is documented, and is empty on failure;
- a test that `eggpack ci generate`, `eggpack ci check`, and `eggpack contract
  expand` still behave exactly as before, including their documented outputs;
- an exit-code test asserting `0` on success and `1` on failure for every
  command.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
```

Add a direct narrowed run for the new tests, e.g.
`cargo test -p eggpack-cli --all-targets --all-features --locked <new_test_name> -- --nocapture`.

The closure record must additionally record the **verbatim** quickstart
walkthrough, with the actual observed output of every command, since
`docs/quickstart.md` is contractually a document of verified commands.

## 11. Documentation updates

- `docs/quickstart.md` — the local end-to-end path, with every command run
  verbatim and its real output quoted.
- `crates/eggpack-cli/README.md` — the actual full command reference,
  including the `ci _*` runner flag surface it currently only summarizes in
  prose.
- `architecture/cli.md` — the new operator surface, the command taxonomy, and
  the explicit statement that this milestone adds no staging or publication
  command.
- `architecture/overview.md` — line counts and baseline SHA are already stale
  and are **not** re-based here; see
  `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`.
- `plans/subsystems/build-qualification-roadmap.md` — the M009 row.

## 12. Acceptance criteria

1. Six new top-level commands exist: `check`, `plan`, `package`, `qualify`,
   `collect`, `manifest`. Each is documented in `--help` with its class.
2. `eggpack plan` and `eggpack collect` produce byte-identical artifacts to
   the existing `ci _resolve-release` and `ci _aggregate` for identical inputs,
   and this is proven by test, not by assertion.
3. The `ci _*` runner family is unchanged in behaviour, arguments, and output
   bytes; no generated workflow changes; `eggpack ci check` drift status is
   unchanged.
4. Every command exits `0` on success and `1` on failure with exactly one
   `eggpack: <message>` line on stderr and an empty stdout on failure.
5. Every command refuses a symlinked output path and an oversized input, using
   the existing bounded helpers.
6. No new dependency, no `clap`, no schema change, no template grammar, no
   network, no discovery, no release selection.
7. `docs/quickstart.md` documents the local path end to end with verbatim
   verified commands and outputs, and `crates/eggpack-cli/README.md` is a
   complete command reference.
8. The full workspace gate and `./scripts/check-local.sh` are green, and hosted
   CI is green on all lanes.
9. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- any existing `ci _*` runner's behaviour, arguments, diagnostics, or output
  bytes would change;
- any generated workflow byte would change;
- a new command would require a schema change, a new dependency, a network
  call, repository discovery, or release-selection policy;
- a new command would stage or publish anything;
- the six commands cannot be implemented as thin wrappers over already-closed
  `eggpack-core` APIs, because that would mean duplicating pipeline logic in
  the CLI;
- the quickstart's documented outputs cannot be produced verbatim on all four
  supported platforms, because the quickstart is a verified document and a
  platform-specific variant would require a separate decision.

## 14. Closure evidence

Write `plans/closure/build-qualification/009-status.md` with: the reviewed
baseline and implementation SHA; the command inventory with each command's
class; the byte-identity evidence between the new commands and the `ci _*`
runners, with exact test names; the verbatim quickstart walkthrough with the
actual observed output of every command on every supported platform; the
verification commands executed and their actual results; hosted CI run IDs and
the lane matrix; the explicit statement that no generated workflow byte and no
schema changed; an invariant review against §3; unresolved findings by
severity; and the named follow-ons — Bootstrap installer commands, the
user-facing staging command, and the ADR-0003 publication gate.

## 15. Handoff notes

- Start from `fb81aa1400e1f333f6500075f78d944a9242b673`.
- This milestone is independent of CI M003k and of the CI M003l/M003m/M003n
  chain. It does not touch `crates/eggpack-ci/**`.
- If CI M003l lands first and moves pipeline stages into `eggpack-core`
  behind shared functions, this milestone should consume those shared
  functions rather than re-exposing them. Re-baseline before implementing if
  the core surface moved.
- The `installer generate/check` and staging commands named by
  `plans/000-long-term-specification.md:175-183` remain **unimplemented**
  after this milestone. Record that explicitly in the closure record's
  unresolved findings rather than leaving it implicit — the specification
  names nine commands and this milestone delivers six of the nine.
