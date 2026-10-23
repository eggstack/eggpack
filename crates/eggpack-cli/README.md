# eggpack-cli

`eggpack` is the deterministic wiring layer of the workspace: it parses
`argv`, reads bounded local files, calls into the library crates, and writes
files. It holds no layout, planning, build, qualification, finalization,
rendering, gating, staging, or installer logic of its own.

This file is the full command reference. Every command, flag, output path, exit
code, and diagnostic below is taken from the implementation.

## Top-level help

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

There are three families:

| Family | Commands | Purpose |
|---|---|---|
| Local producer operator | `check`, `plan`, `package`, `qualify`, `collect`, `manifest` | Build, qualify, and finalize a release on a workstation. |
| CI generation | `ci generate`, `ci check`, and the nine `ci _*` runners | Render and drift-check the GitHub Actions workflow, and provide the file-in/file-out steps a rendered workflow invokes. |
| Contract projection | `contract expand` | Print one expanded contract field as a scalar. |

## Exit codes and diagnostics

Every command exits `0` on success and `1` on failure. There are only two
exit codes: a caller distinguishes failure causes by message text, not by
status.

- **Success** prints its result on stdout. Nothing is written to stderr.
- **Failure** prints exactly one line on stderr, prefixed `eggpack: `, and
  prints nothing at all on stdout. The message names a cause class; it never
  echoes captured process output, credentials, or file contents.
- `eggpack --version` / `-V` prints `eggpack <version>` and exits `0`.
- `eggpack --help` / `-h` prints the top-level help above and exits `0`.

## Local producer operator commands

These six commands are the local producer path: `check` → `plan` → `package` →
`qualify` → `collect` → `manifest`. They read local paths, write local paths,
and make no network calls.

Each command declares a **class** — `validation`, `build`, `release
aggregation`, or `final-bytes evidence` — and `eggpack <cmd> --help` prints
the command name, its class, its usage line, `class: <class>`, and the
statement that no command in this set stages or publishes. For example:

```text
$ eggpack collect --help
eggpack collect — release aggregation
usage: eggpack collect --contract <path> --release-plan <path> --build-bindings <path> --qualification-bindings <path> --artifacts-dir <path> --output-root <path>
class: release aggregation
No command in this set stages or publishes.
```

**Argument surface.** These six commands use a strict parser, not the loose
helper the `ci` family uses. Every flag is required exactly once (except
`manifest`'s optional `--artifacts-dir`), may be written `--flag value` or
`--flag=value`, and must have a non-empty value. A repeated flag, an unknown
option, a positional argument, or a missing required flag is an error, and the
missing flags are listed in sorted order:

```text
eggpack: missing required --build-bindings, --contract, --pack-config, --qualification-bindings, --release-id, --selected, --source-revision
```

**Input bounds.** Every input document is read through the shared bounded
reader at 1 MiB. A symlink, a non-regular file, or an oversized file is
refused before it is parsed.

### `eggpack check`

```text
usage: eggpack check --contract <path> --pack-config <path> --build-bindings <path> --qualification-bindings <path> --selected <alias[,alias]> --release-id <id> --source-revision <revision>
```

| Flag | Meaning |
|---|---|
| `--contract` | Distribution contract (schema-v1 TOML). |
| `--pack-config` | Producer policy: strategy, host, toolchain, floor, qualification, support. |
| `--build-bindings` | Logical output → package/binary mapping. |
| `--qualification-bindings` | Per-target qualification bindings (smoke checks). |
| `--selected` | Comma-separated contract target aliases or triples to plan. |
| `--release-id` | The release identity. Must pass the exact-tag safety rules. |
| `--source-revision` | The release's source revision: exactly 40 lowercase hex characters. |

**Reads** the five documents above. **Writes** nothing. The command resolves
the release plan, validates it against its own declared floor, and checks that
the build bindings match the plan and that the qualification bindings match
the plan and the build bindings.

`check` never runs `git`: `--source-revision` is an explicit value, checked for
shape only. Use `ci _resolve-release` when the checkout itself must be
verified.

Success prints `check: ok`.

### `eggpack plan`

```text
usage: eggpack plan --contract <path> --pack-config <path> --build-bindings <path> --qualification-bindings <path> --selected <alias[,alias]> --release-id <id> --source-revision <revision> --output <path>
```

Takes every `eggpack check` flag plus:

| Flag | Meaning |
|---|---|
| `--output` | Where the canonical `ReleasePlan` JSON is written. |

**Reads** the five documents above. **Writes** one file, `--output`, through
the atomic writer; its parent directory must already exist.

The plan is the release's *intent* — release identity, source revision, and the
planned targets with their policies. It is not evidence that anything was
built. The encoder is shared with `ci _resolve-release`, so the two surfaces
write byte-identical plans for one resolved plan.

Success prints `plan: ok`.

### `eggpack package`

```text
usage: eggpack package --release-plan <path> --build-bindings <path> --source-root <path> --output-root <path>
```

| Flag | Meaning |
|---|---|
| `--release-plan` | The `ReleasePlan` JSON written by `eggpack plan`. |
| `--build-bindings` | Build bindings document. |
| `--source-root` | The Cargo project root to build in. Absolute, or relative to the working directory; `..` components are rejected. |
| `--output-root` | The directory the build artifacts are written under. |

**Reads** the plan and the bindings, and **builds** in `--source-root` with
`cargo +<toolchain>` (the toolchain pinned in the plan's policy) and
`--release --locked`. The build runs with a cleared environment, a process
group, a bounded deadline, and a bounded output cap. It is a real Cargo build:
the project must compile.

**Writes**, under `--output-root`:

- `<target>/build-handoff.json` — the canonical build descriptor: release
  identity, target, strategy, and one entry per logical output with its size.
- `<target>/candidates/<relative_path>` — the candidate bytes.
- `.eggpack-build/` — a private Cargo scratch directory, recreated on every
  run so a restarted stage starts from a clean target directory.

`--output-root` is created if it does not exist. Every target is built before
any artifact is written, so a failed build leaves no half-written release
behind.

Success prints `package: ok`.

### `eggpack qualify`

```text
usage: eggpack qualify --contract <path> --release-plan <path> --build-bindings <path> --qualification-bindings <path> --artifacts-dir <path> --output-dir <path>
```

| Flag | Meaning |
|---|---|
| `--contract` | Distribution contract (schema-v1 TOML). |
| `--release-plan` | The `ReleasePlan` JSON. |
| `--build-bindings` | Build bindings document. |
| `--qualification-bindings` | Per-target qualification bindings. |
| `--artifacts-dir` | The **per-target artifact root** to qualify: the directory holding `<target>/build-handoff.json` and `<target>/candidates/`. |
| `--output-dir` | Where the qualification artifact directories are written. |

> `--artifacts-dir` is the **per-target artifact root**: the directory that
> holds one subdirectory per target, each with that target's
> `build-handoff.json` and `candidates/`. It is the root `eggpack package`
> writes — not the `candidates/` directory inside a target's directory.

**Reads** the four documents plus the handoff and candidate bytes under
`--artifacts-dir`. **Writes**, under `--output-dir`, one directory per
target holding `evidence.json`, `build-handoff.json`, and `candidates/`.

The evidence records the planned and actual classification, the actual host
the qualification ran on, the candidate digests and formats, and the bounded
process outcomes. A `native` qualification is host-matched proof; it does not
prove a declared glibc or macOS floor, and a host mismatch is recorded as a
failure, never as a pass or a skip.

Every target is qualified before any artifact is written.

Success prints `qualify: ok`.

### `eggpack collect`

```text
usage: eggpack collect --contract <path> --release-plan <path> --build-bindings <path> --qualification-bindings <path> --artifacts-dir <path> --output-root <path>
```

| Flag | Meaning |
|---|---|
| `--contract` | Distribution contract (schema-v1 TOML). |
| `--release-plan` | The `ReleasePlan` JSON. |
| `--build-bindings` | Build bindings document. |
| `--qualification-bindings` | Per-target qualification bindings. |
| `--artifacts-dir` | The **per-target artifact root** to aggregate: the directory written by `eggpack qualify`. |
| `--output-root` | The new release root to finalize into. |

**Reads** the four documents plus the handoff, evidence, and candidate bytes
under `--artifacts-dir`. **Writes** the finalized release tree under
`--output-root`, and `release-manifest.json` **beside** that root — never
inside it.

The parent of `--output-root` must already exist: the finalizer writes one
*new* release root under a caller-secured parent and never overwrites an
existing one. The manifest therefore lands in `--output-root`'s parent.

`collect` projects the same release graph the CI path projects, so the local
path aggregates the same targets with the same finalization settings the
generated workflow uses.

Success prints `collect: ok`.

### `eggpack manifest`

```text
usage: eggpack manifest --contract <path> --release-plan <path> --finalized-root <path> --output <path> [--artifacts-dir <path>]
```

| Flag | Meaning |
|---|---|
| `--contract` | Distribution contract (schema-v1 TOML). |
| `--release-plan` | The `ReleasePlan` JSON. |
| `--finalized-root` | The finalized release tree to read the final bytes from. |
| `--output` | Where the derived `release-manifest.json` is written. |
| `--artifacts-dir` | Optional. The per-target artifact root. Required only for archive targets. |

**Reads** the contract, the plan, and the finalized tree's bytes. **Writes**
one file, `--output`; its parent directory must already exist.

The manifest is the *final-bytes evidence*: for each target it records the
asset form, the artifact name, its size, and its SHA-256. Those are integrity
facts about the bytes — not a signature and not a provenance claim.

`--artifacts-dir` is needed only for archive targets. An archive's member
digests are facts about the member bytes, which the finalized tree does not
retain, so they are read from the qualified candidate bytes instead. A direct
target needs no candidate bytes.

Success prints `manifest: ok`.

## `eggpack ci`

The `ci` family renders the GitHub Actions workflow and provides the internal
runners a rendered workflow invokes.

The `ci` family uses the **loose** flag helper: flags are read left to right
and the first occurrence wins, so a repeated flag is silently ignored. Unknown
flags are not rejected; a typo is caught only if it pushes the argument count
past the command's cap. Use the local operator commands when strict argument
handling matters.

### `eggpack ci generate`

Renders a workflow and writes exactly one file, `--output`, through the atomic
writer. The output path is refused if it is a symlink or a directory, and its
parent must already be a real directory. Nothing else is created, so a
generate run cannot touch anything the caller did not name.

| Flag | Required | Meaning |
|---|---|---|
| `--github-policy` | yes | Caller-supplied runner labels, action pins, and release input paths. |
| `--output` | yes | Where the rendered workflow is written. |
| `--ci-plan` | one of two modes | A checked-in `ReleaseCIPlanV1` (exact mode). |
| `--workflow-shape` | one of two modes | A `ReleaseWorkflowShapeV1` (reusable mode). |
| `--contract` | with `--workflow-shape` | The distribution contract, in reusable mode. |

The two modes are mutually exclusive: supplying both `--ci-plan` and
`--workflow-shape` is an error.

Success prints `generated <n> bytes to <path>`. The rendering is deterministic:
the same inputs produce byte-identical output.

### `eggpack ci check`

Renders the workflow in memory and compares it with an existing file. It
**never writes**. The existing workflow is read through the bounded reader at
8 MiB.

| Flag | Required | Meaning |
|---|---|---|
| `--github-policy` | yes | Caller-supplied policy, as for `ci generate`. |
| `--workflow` | yes | The existing workflow file to compare against. |
| `--ci-plan` | one of two modes | A checked-in `ReleaseCIPlanV1` (exact mode). |
| `--workflow-shape` | one of two modes | A `ReleaseWorkflowShapeV1` (reusable mode). |
| `--contract` | with `--workflow-shape` | The distribution contract, in reusable mode. |

Comparison is CRLF→LF only. There is no fuzzy matching, so a
reformatted-but-equivalent file is drift too.

Success prints `ci check: match (<n> bytes)` and exits `0`. Drift prints
`eggpack: ci check: drift detected (expected <n> bytes, found <n> bytes,
first difference at Some(<n>))` and exits `1`.

### The nine internal `ci _*` runners

These exist so a generated workflow step is a file-in/file-out call with a
fixed shape. They are **wrappers, not a scripting interface**: the input
grammar is paths, a target name that must already exist in the plan, and
nothing else. There is no shell invocation, no caller-supplied program name,
and no flag whose value is interpreted as a command.

Invocations are rendered, not typed — `RunnerCommand::argv()` in
`eggpack-ci` is what produces them.

#### `ci _verify-source`

| Flag | Required | Meaning |
|---|---|---|
| `--release-plan` | yes | The plan whose `source_revision` is checked. |
| `--github-policy` | all three or none | The resolved draft policy. |
| `--expected-tag` | all three or none | The tag the policy must carry. |
| `--source-root` | all three or none | The checkout to verify. |

With only `--release-plan`, the command checks that the plan's
`source_revision` is 40 lowercase hex characters and that the inherited
working directory is at that revision. With all three of the optional flags,
it additionally checks the tag, the mapped identity, and that the tag peels to
the plan's revision.

Writes nothing. Prints `checked-out source matches release plan`.

#### `ci _resolve-release`

| Flag | Required | Meaning |
|---|---|---|
| `--contract` | yes | Distribution contract. |
| `--pack-config` | yes | Producer policy. |
| `--build-bindings` | yes | Build bindings. |
| `--qualification-bindings` | yes | Qualification bindings. |
| `--selected` | yes | Comma-separated target aliases to plan. |
| `--tag` | yes | The exact existing tag. |
| `--source-revision` | yes | 40 lowercase hex characters. |
| `--template` | yes | The static draft template. |
| `--source-root` | yes | The checkout the revision is verified against. |
| `--output-plan` | yes | Where the `ReleasePlan` is written. |
| `--output-ci-plan` | yes | Where the `ReleaseCIPlanV1` is written. |
| `--output-github-policy` | yes | Where the resolved draft policy is written. |
| `--consumer-validators` | no | Consumer validator configuration. |
| `--identity-mode` | no | `exact_tag` (default) or `v_prefixed_stable_semver`. |

Verifies that the checked-out `HEAD` under `--source-root` equals
`--source-revision` **and** that the tag peels to the same revision; either
check failing fails the command closed. The three output documents are written
only after both checks pass.

Prints two lines:

```text
checked-out source matches release plan
resolved runtime release identity for tag <tag>
```

#### `ci _capture-build`

| Flag | Required | Meaning |
|---|---|---|
| `--release-plan` | yes | The `ReleasePlan` JSON. |
| `--build-bindings` | yes | Build bindings. |
| `--target` | yes | The target to capture. |
| `--cargo-target-dir` | one of two | A Cargo target root; candidate paths are derived from it. |
| `--candidate-dir` | one of two | A directory of already-staged candidate bytes. |
| `--output` | one of two | Where the handoff JSON is written. |
| `--output-dir` | one of two | Where the canonical build artifact directory is written. |

At least one of `--cargo-target-dir`/`--candidate-dir` and at least one of
`--output`/`--output-dir` are required. `--output-dir` receives
`build-handoff.json` plus `candidates/<relative_path>` bytes.

Prints `captured build handoff for <target>`.

#### `ci _qualify-target`

| Flag | Required | Meaning |
|---|---|---|
| `--release-plan` | yes | The `ReleasePlan` JSON. |
| `--build-bindings` | yes | Build bindings. |
| `--qualification-bindings` | yes | Qualification bindings. |
| `--contract` | yes | Distribution contract. |
| `--target` | yes | The target to qualify. |
| `--candidate-dir` | yes | The candidate bytes to qualify. |
| `--build-handoff` | yes | The build handoff describing those bytes. |
| `--output-dir` | yes | Where the qualification artifact directory is written. |
| `--qemu-sysroot` | no | The sysroot for an `emulated` target. |

Writes `evidence.json`, `build-handoff.json`, and `candidates/` into
`--output-dir`. Prints `qualified <target>: <status>`.

#### `ci _validate-consumer`

| Flag | Required | Meaning |
|---|---|---|
| `--consumer-validators` | yes | The consumer validator configuration. |
| `--target` | yes | The target to validate. |
| `--candidate-dir` | yes | The candidate bytes to validate against. |
| `--build-handoff` | yes | The build handoff. |
| `--evidence` | yes | The qualification evidence. |
| `--source-root` | yes | The source tree the validator runs in. |
| `--output` | yes | Where the consumer evidence is written. |

Refuses fast unless the qualification evidence is `Passed` and its identity
matches the handoff, so a failed candidate is never executed against.

Prints `consumer validation passed for <target>`.

#### `ci _evaluate-gate`

| Flag | Required | Meaning |
|---|---|---|
| `--ci-plan` | yes | The `ReleaseCIPlanV1`. |
| `--output` | yes | Where the gate outcome JSON is written. |
| `--evidence-dir` | one of two | A flat directory of `<evidence-handoff-name>.json` files. |
| `--inputs-dir` | one of two | The canonical per-target layout `<inputs-dir>/<target>/evidence.json`. |

At least one of `--evidence-dir`/`--inputs-dir` is required. Stray
`consumer-evidence.json` in the inputs directory, with no configured
validator, fails closed so validation can never be silently bypassed.

Prints `gate outcome: <outcome>`. An outcome that is not complete or
suppressed is an error.

#### `ci _aggregate`

| Flag | Required | Meaning |
|---|---|---|
| `--contract` | yes | Distribution contract. |
| `--release-plan` | yes | The `ReleasePlan` JSON. |
| `--ci-plan` | yes | The `ReleaseCIPlanV1`. |
| `--inputs-dir` | yes | The per-target layout `<inputs-dir>/<target>/{build-handoff.json,evidence.json,candidates/}`. |
| `--output-root` | yes | The new release root to finalize into. |
| `--output` | yes | Where the aggregate summary JSON is written. |

Writes the finalized release tree under `--output-root`, the summary at
`--output`, and `release-manifest.json` beside `--output`'s parent — never
inside the finalized root.

Prints `aggregate: Complete`.

#### `ci _prepare-stage`

| Flag | Required | Meaning |
|---|---|---|
| `--contract` | yes | Distribution contract. |
| `--release-manifest` | yes | The `release-manifest.json`. |
| `--finalized-root` | yes | The finalized release tree. |
| `--github-policy` | yes | The GitHub draft policy. |
| `--install-policy` | yes | The installer policy. |
| `--output-dir` | yes | Where the staging payload directory is written. |
| `--output-payload` | yes | Where the payload document is written. |
| `--installer-presentation` | paired | Presentation for product installer wrappers. |
| `--source-root` | paired | The source tree the wrappers are rendered from. |

`--installer-presentation` and `--source-root` are paired: one without the
other is an error. The pair selects the product-wrapper rendering; neither
selects the generated default.

Prints `prepare-stage: <n> assets`.

#### `ci _stage-github-draft`

| Flag | Required | Meaning |
|---|---|---|
| `--payload` | yes | The staging payload document. |
| `--github-policy` | yes | The GitHub draft policy. |
| `--output-receipt` | yes | Where the staging receipt is written. |
| `--staging-dir` | no | The directory the payload's relative paths resolve against. Defaults to the payload file's parent. |

The GitHub token is read from the environment only, under the name the policy
declares. Its absence fails before any network I/O, and the token text is
never serialized, logged, or included in an error.

Prints `stage-github-draft: release <id> draft staged`.

## `eggpack contract expand` (Contract M003)

A bounded, local-only projection over existing `eggpack-contract` schema-v1
semantics. It lets a consumer repository read a producer fact from the contract
instead of re-implementing TOML parsing and template expansion.

```sh
eggpack contract expand \
  --contract distribution.toml \
  --release-id v1.2.7 \
  --target linux-x64 \
  --field asset
# eggsact-v1.2.7-x86_64-unknown-linux-gnu
```

- **Local file input only.** One contract path, read through the CLI's bounded
  non-symlink reader with a 1 MiB limit. No git discovery, current-directory
  search, environment fallback, network, GitHub API, or release lookup.
- **Exactly one scalar.** `--field` selects one of `canonical-target`,
  `asset`, `sidecar`, or `install`. On success stdout is that value plus a
  single `\n` and stderr is empty; every failure exits nonzero, prints nothing
  to stdout, and writes a bounded diagnostic to stderr. Repeated runs over
  equal bytes and arguments are byte-identical.
- **`--release-id` is opaque.** It is an expansion input, not a release
  selector: Eggpack applies no SemVer ordering and chooses no release.
  Ordinary prerelease and build punctuation (`v1.2.3-rc.1+build.5`) is
  accepted; the existing contract path-safety rules still reject path
  separators and empty values.
- **Direct-only fields.** `asset`, `sidecar`, and `install` are valid only when
  the resolved target expands to a direct artifact. Bundle and archive targets
  fail closed with `selected --field <name> requires a direct artifact target`
  rather than choosing a primary entry or archive member.
  `canonical-target` is valid for every asset form.
- **Closed argument surface.** `--contract`, `--release-id`, `--target`, and
  `--field` are each required exactly once and accept `--flag value` or
  `--flag=value`. Duplicate flags, unknown options, empty values, positional
  arguments, and unsupported field names are rejected.
- **Read-only.** The contract and its directory are never modified.
- **Consumer policy stays consumer-owned.** Frozen public-name tables,
  latest/exact selection, Cargo fallback, install destinations, updater
  policy, and compatibility-floor checks are not provided here and must not be
  inferred from this output.

`eggpack-contract` remains the sole semantic authority: the command calls
`DistributionContract::parse_toml_str` and `DistributionContract::expand` and
adds no template grammar, target/alias resolution, or serialized expansion
document of its own.

The example above is executable and guarded by
`crates/eggpack-cli/tests/contract_expand.rs::the_documented_readme_example_runs_verbatim`,
which runs it with these exact relative paths from a temporary working
directory.

## Staging and publication

**No command in this file stages or publishes.** The six local producer
operator commands are local-only: they read local paths, write local paths,
and reach no network. The `ci` family can render a workflow and can prepare
and reconcile a GitHub **draft**, but staging is draft-only and publication is
never performed by this binary.

Staging and publication remain separate, explicitly authorized actions:

- Staging writes into a staging payload and reconciles it into a draft on the
  exact existing tag. There is no clobber, no tag mutation, and no overwrite
  of an existing draft.
- Publication is a human action performed outside this binary. An internal
  artifact is not a public release.

A failed or optional stage suppresses release output rather than failing the
run.

## See also

- [../../docs/quickstart.md](../../docs/quickstart.md) — a verified
  end-to-end walkthrough of the local producer path and the CI rendering path.
- [../../architecture/cli.md](../../architecture/cli.md) — the command
  taxonomy and the wiring boundary.
- [../../architecture/overview.md](../../architecture/overview.md) — the
  workspace-level module map.
