# `eggpack-cli` — Deep Dive

The deterministic wiring layer of the workspace and the only binary the
generated workflow actually invokes. `eggpack-cli` contributes exactly one
artifact — the `eggpack` binary (`crates/eggpack-cli/Cargo.toml11-13`) — built
from two source files: `crates/eggpack-cli/src/main.rs` (4617 lines) and
`crates/eggpack-cli/src/operator.rs` (1094 lines). Every release decision in
both is delegated to a library crate.

Citations below are `crates/eggpack-cli/src/main.rs:LINE` unless another file
is named; `operator.rs` citations are `crates/eggpack-cli/src/operator.rs:LINE`.

## Responsibility and the wiring rule

The crate parses `argv`, reads bounded files, calls into the library crates, and
writes files. It holds no layout authority (that is `eggpack-contract`), no
planning, build, qualification, or finalization logic (`eggpack-core`), no
rendering, gating, or aggregation logic (`eggpack-ci`), no draft-staging logic
(`eggpack-github`), and no installer rendering (`eggpack-bootstrap`).

The boundary is verifiable rather than aspirational. There are zero `pub`
items in `main.rs`, and the only visibility beyond the crate is three
`pub(crate)` items in `operator.rs` — a command descriptor and its table
(`operator.rs:42-52`, `operator.rs:55-92`) and the family dispatcher (`operator.rs:241`).
So nothing here is a library API another crate can depend on. Every semantic
question is asked of a library call and the answer used verbatim:
`PackConfig::resolve` and `GitHubDraftTemplateV1::resolve` via
`resolve_runtime_release_plan_with_mode` (`478`, `operator.rs:302`), gate policy
via `evaluate_gate` (`1432`, `1446`), qualification via `qualify_target`
(`1179`, `operator.rs:726`), finalization via `aggregate_finalize[_with_consumer]`
(`1503`, `1521`, `operator.rs:834`).

What the crate does keep are fail-closed preconditions checked *before*
delegation, plus path adapters. Neither is a release rule:

| Local check | Line | Why it is wiring, not policy |
|---|---|---|
| Argument arity caps | `452`, `915`, `969`, `1000`, `1294`, `1596`, `1657` | Rejects an invocation the renderer would never emit. No release meaning. |
| Paired-flag requirements | `1032-1037`, `1398-1400`, `1601-1606` | Selects between two library entry points. |
| Non-`Passed` evidence refused before a consumer script runs | `1317-1321` | A guard on the validator seam; the gate also fails closed on it. |
| Handoff/evidence identity cross-checks | `1306`, `1323-1328` | Confirms the two files describe the same target, release, and revision. |
| Stray `consumer-evidence.json` rejected | `1422-1431`, `1497-1501`, `operator.rs:823-833` | Prevents validation being silently bypassed. |
| Manifest round-trip byte comparison | `735-737` | Proves the sidecar is the same document, not a re-serialization. |
| `absolute_path` / `absolute_under_root` / `absolutize_cli_path` | `1229`, `1205`, `1248` | Adapters between the renderer's portable relative paths and the absolute paths core requires (`1351-1353`, `1166`). |
| Operator flag strictness | `operator.rs:174-227` | A consumer-facing surface: a repeated flag must not silently take the first value. |
| `contract expand` 1 MiB input bound | `133`, `346` | Rejects an oversized local contract before parsing. No release meaning. |
| `contract expand` scalar output bound | `140`, `330-334` | Refuses to print a value it cannot bound. No release meaning. |
| `contract expand` exact argument set | `230-293` | A consumer script reads this output, so silently keeping the first of a repeated flag is not acceptable here. |

The rule of thumb: if the answer would change when a library changes, the check
belongs in the library. The adapters are the only code here whose reason for
existing is a mismatch between two other crates, and each says so at its call
site (`1244-1247`, `1127-1128`, `1468-1469`).

## Argument parsing, hand-rolled

`dispatch` (`24`) selects a command family and nothing else. Three usage
constants hold the help text — `OPERATOR_USAGE` (`50`), `USAGE` (`54`), and
`CONTRACT_USAGE` (`57`) — reused verbatim at `67`, `82`, `83`, `115`, and in
the `help_text` body (`65-85`):

```text
usage: eggpack <check|plan|package|qualify|collect|manifest|ci|contract> [options]

usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]

usage: eggpack contract expand --contract <distribution.toml> --release-id <opaque-release-id> --target <triple-or-alias> --field <canonical-target|asset|sidecar|install>
```

Order of decisions: empty argv is a usage error (`25-27`); `--version`/`-V`
prints `eggpack <CARGO_PKG_VERSION>` and exits 0 (`28-31`); `--help`/`-h`
prints the full help and exits 0 (`32-35`); all three are handled *before* the
family match, so they work only as the first token. The family match at
`36-46` accepts exactly `ci`, `contract`, and the six operator commands
`check`, `plan`, `package`, `qualify`, `collect`, and `manifest` (`39-41`);
any other first token is a usage error naming the family. `ci_dispatch` (`89`)
preserves the historical behaviour verbatim, including the `unknown ci
subcommand` diagnostic (`106`) and the eleven subcommand arms (`95-105`), plus a
bare `ci` as a usage error (`90-92`). `contract_dispatch` (`111`) accepts
`expand` (`113`), `help` (`114-117`), and rejects anything else (`118`, `119`).

There are now **two** parsers, and the difference is deliberate.

`contract expand` does **not** use the shared flag helper.
`parse_contract_expand_args` (`230`) is command-local and exact: it accepts
`--name value` and `--name=value` (`243-246`, `253-260`), rejects positional
arguments (`237-242`), unknown options (`247-249`), a repeated flag rather
than keeping the first (`250-252`), empty values (`262-264`), and reports
every missing required flag in sorted order (`269-277`).

The six operator commands use a second strict parser, `OperatorArgs::parse`
(`operator.rs:174`), with the same rules applied to their whole family: an
unknown option is rejected (`operator.rs:195-197`), a repeated flag is rejected
rather than silently taking the first (`operator.rs:198-200`), an empty value
is rejected (`operator.rs:210-212`), and missing required flags are listed in
sorted order (`operator.rs:217-224`).

`get_flag` (`558`) accepts exactly two forms: `--name value` (`562-567`) and
`--name=value` (`568-570`). Its two error messages are `missing value for
--name` (`566`) and `missing required --name` (`572`). `get_flag_optional`
(`575`) is the same scan returning `Option`. Both scan left to right and return
the first match, so a repeated flag silently takes the first occurrence. That is
why `contract expand` and the operator family parse their own arguments rather
than inheriting the looseness of the shared helper.

In the `ci` family, unknown flags are **not** rejected. No function scans for
unrecognised arguments; a typo is caught only if it pushes the argument count
past a cap:

| Command | Cap | Line |
|---|---|---|
| `ci _resolve-release` | 30 | `452` |
| `ci generate` (reusable / exact) | 8 / 6 | `915` / `938` |
| `ci check` (reusable / exact) | 8 / 6 | `969` / `1000` |
| `ci _validate-consumer` | 14 | `1294` |
| `ci _prepare-stage` | 18 | `1596` |
| `ci _stage-github-draft` | 8 | `1657` |
| `ci _capture-build`, `ci _evaluate-gate` | none | paired-flag checks instead (`1032-1037`, `1398-1400`) |

`#![forbid(unsafe_code)]` (`1`) holds workspace-wide. `#![deny(missing_docs)]`
(`2`) has no target: the crate exposes no public items, so in a binary it is
inert. The `///` blocks document handlers for internal readers, not for
rustdoc.

The trade-off is deliberate. No `clap` means no dependency, no derive expansion,
and a parse whose behaviour is fully determined by the code above. The cost is
hand-maintained parsing, flat usage strings as the only help, no completions,
and — in the `ci` family — a typo diagnosed as `too many arguments for ci ...`
rather than by name. `contract expand` and the operator family pay for a
stricter parser instead, because a consumer script and an operator read their
output.

## The local producer operator commands

Build/Qualification M009 added six top-level commands — `check`, `plan`,
`package`, `qualify`, `collect`, and `manifest` — so a producer can construct,
qualify, and finalize a release on a workstation without a GitHub Actions run
and without the `ci _*` internal runner family. They live in
`crates/eggpack-cli/src/operator.rs`.

**This milestone adds no staging or publication command.** The six commands
are local-only: they read local paths, write local paths, and reach no
network. `operator.rs:1-12` states it in the module header, the per-command
help states it at `operator.rs:263`, and the top-level help states it at
`77-80`: *"No command in this set stages or publishes; staging and publication
stay separate, explicitly authorized actions."* Staging and publication remain
reachable only through the `ci _prepare-stage` / `ci _stage-github-draft`
runners, and publication is never performed by this binary at all.

### The taxonomy

Each command declares a **class**, and the class is part of the command's
identity rather than a comment: it appears in the command table
(`operator.rs:55-92`), in `eggpack <cmd> --help` (`operator.rs:258-265`), and
in the top-level help listing (`main.rs:69-75`).

| Command | Class | Effect |
|---|---|---|
| `check` | validation | validate contract, policy, and bindings before anything is built |
| `plan` | validation | resolve the release identity and write a ReleasePlan |
| `package` | build | build every planned target's candidates under an output root |
| `qualify` | build | qualify built candidates and write qualification evidence |
| `collect` | release aggregation | aggregate qualified candidates into a finalized release tree |
| `manifest` | final-bytes evidence | derive release-manifest.json from a finalized release tree |

The three classes are the milestone's own vocabulary, and they line up with the
authority separation the workspace already keeps: `validation` produces intent
and refuses to write anything unvalidated; `build` produces candidates and
their qualification evidence; `release aggregation` produces a finalized tree;
`final-bytes evidence` produces the manifest over those final bytes. Nothing
in the set crosses from evidence back into intent, and nothing in the set
reaches the publication boundary.

### The shared shape

All six share one resolution path and one writer.

`resolve_planning_inputs` (`operator.rs:288`) is the single place the planning
input set is read and validated: the contract (`operator.rs:289`), the policy
(`operator.rs:290`), the build bindings (`operator.rs:291`), the qualification
bindings (`operator.rs:292-293`), the selected targets (`operator.rs:294`), and the
release identity (`operator.rs:295-296`). It then
resolves through `resolve_runtime_release_plan_with_mode`
(`operator.rs:302-311`) — the same function `ci _resolve-release` calls
(`478`) — validates the plan against its own
declared floor (`operator.rs:313-314`), and checks that the build bindings
match the plan (`operator.rs:315-320`) and the qualification bindings match the
plan and the build bindings (`operator.rs:321-328`). `check` and `plan` differ only in what they do
with the result: `check` discards it (`operator.rs:500`), `plan` writes it
(`operator.rs:520-521`).

Because the encoder is shared (`encode_release_plan`, `717-719`), `eggpack
plan` and `ci _resolve-release` write byte-identical plans for one resolved
plan. That is a property, not a coincidence: one rule set, one document.

The identity mode is a caller choice, not a hardcoded constant: `--identity-mode`
selects `exact_tag` (the default) or `v_prefixed_stable_semver`, the two modes
`ci _resolve-release` offers (`operator.rs:376-385`). `--release-id` carries
the exact tag in both modes; the mode decides how that tag maps to the release
identity. An unknown mode is refused.

`--consumer-validators` carries the same JSON validator map
`ci _resolve-release` takes. When it is present, the consumer-aware graph is
projected through `project_release_plan_with_consumer`
(`operator.rs:344-369`), so a consumer-validated release plans, aggregates, and
derives its manifest locally exactly as it would in CI. A validator that does
not describe the release is refused before anything is built.

The producer policy and both bindings documents are read as **TOML or JSON**,
the same two encodings the CI path accepts (`operator.rs:407-470`). The JSON
document is decoded into the canonical document shape and re-read by the same
strict core parser, so one validator stays authoritative for both encodings
and the operator's bounds, unknown-field rejection, and strictness do not
diverge from the CI path's.

Every input document is read through the shared bounded reader at 1 MiB
(`operator.rs:27`, `operator.rs:407-470`), and every document the operator family writes
goes through the same atomic writer the `ci` family uses.

### The per-command surface

| Command | Reads | Writes | Success line |
|---|---|---|---|
| `check` | contract, pack config, build bindings, qualification bindings; optionally the identity mode and the consumer validator map | nothing | `check: ok` (`operator.rs:506`) |
| `plan` | the same, plus the release identity | `--output` (plan JSON) | `plan: ok` (`operator.rs:522`) |
| `package` | release plan, build bindings; builds in `--source-root` | `<output-root>/<target>/{build-handoff.json,candidates/}` | `package: ok` (`operator.rs:571`) |
| `qualify` | contract, release plan, both bindings, the per-target artifact root; optionally the QEMU sysroot | `<output-dir>/<target>/{evidence.json,build-handoff.json,candidates/}` | `qualify: ok` (`operator.rs:760`) |
| `collect` | contract, release plan, both bindings, the per-target artifact root; optionally the consumer validator map | the finalized tree under `--output-root`, plus `release-manifest.json` beside it | `collect: ok` (`operator.rs:875`) |
| `manifest` | contract, release plan, both bindings, the finalized tree's bytes; optionally the consumer validator map and the artifact root | `--output` (manifest JSON) | `manifest: ok` (`operator.rs:1021`) |

Three naming and placement details are worth stating plainly, because they are
easy to get wrong:

- **`--artifacts-dir` is the per-target artifact root**, not the
  `candidates/` directory inside it. `qualify` reads
  `<artifacts-dir>/<target>/build-handoff.json` (`operator.rs:716-722`) and
  `collect` reads the same layout through `collect_finalization_inputs`
  (`operator.rs:821`, `main.rs:748-782`). The flag is named after the
  directory it points at, not after the `candidates/` subdirectory it holds.
- **`collect` writes the manifest beside the finalized root**, in
  `--output-root`'s parent (`operator.rs:782-786`, `operator.rs:871-874`), while
  `ci _aggregate` writes it beside `--output` (`1538-1541`). The two are
  different placements, described under *Where `release-manifest.json` is
  written*.
- **`package` recreates its private build scratch on every run**
  (`operator.rs:535-536`, `operator.rs:627-639`), so a restarted stage starts from a clean
  Cargo target directory.

### Staging never truncates a candidate

`qualify`, `ci _qualify-target`, and `ci _capture-build` stage candidates
through the same helper (`main.rs:833-887`): a hard link is preferred, and the
fallback copies the bytes into a fresh temporary sibling and renames that file
over the destination. The destination is therefore never opened for writing in
place, so a destination that resolves to the source itself cannot lose the
source's bytes to a copy. The same-file case is refused outright
(`main.rs:834-836`).

`qualify` additionally refuses an `--output-dir` that names the same location
as `--artifacts-dir` (`operator.rs:690-695`): the qualification is about those
candidate bytes, so staging into the directory they were read from is refused
before anything is written. The refusal is fail-closed and the candidate bytes
are left exactly as they were.

`package` is the only command in the set that spawns a build. It delegates to
`eggpack_core::execute_target` (`operator.rs:546`) with the core builder's own
bounded, process-grouped execution, and the private Cargo target directory
lives under the caller's output root (`operator.rs:36`, `operator.rs:535`).

### The local path and the CI path are the same rules

`collect` projects the same graph the CI path projects: `project_ci_plan`
(`operator.rs:793`) and `project_release_plan` (`operator.rs:797`), the same two
calls `ci _resolve-release` makes (`522`, `525`). With
`--consumer-validators`, the consumer-aware projection
(`project_release_plan_with_consumer`, `operator.rs:802`) and the
consumer-aware aggregation (`aggregate_finalize_with_consumer`,
`operator.rs:854`) are the ones `ci _resolve-release` and `ci _aggregate` use.
`qualify` calls
`qualify_target` (`operator.rs:726`), the same function `ci _qualify-target`
calls (`1179`), with the same `--qemu-sysroot` the runner takes
(`operator.rs:696-707`). `package` calls `execute_target` (`operator.rs:546`), the same
builder the `ci _capture-build` runner's output is derived from.

The one thing the local path does *not* do is verify a checkout.
`ci _resolve-release` verifies that `HEAD` under `--source-root` equals
`--source-revision` (`458`) and that the tag peels to the same revision
(`459`). The operator commands take `--source-revision` as an explicit value
and check its shape only (`operator.rs:296`, through the library's
`validate_source_revision`), and never run `git`. That is a deliberate
boundary: a local operator path that resolved a repository implicitly would
mean different things in different directories. A caller who needs the checkout
verified uses `ci _resolve-release`.

### The derived manifest is self-checking

`eggpack manifest` derives the manifest from the finalized tree's bytes, and
the derivation checks itself against the finalized tree rather than trusting
the caller's inputs:

- **Archive members are read back out of the finalized archive.** An archive's
  member digests are facts about the member bytes, and the finalized tree
  keeps those bytes only inside the `.tar.gz`. Every member is therefore
  extracted from the finalized archive and digested
  (`operator.rs:1056-1094`), and the digest is compared with the candidate
  bytes the manifest records (`operator.rs:972-993`). A member that is absent
  from the finalized archive, or that differs from it, fails the command. A
  stale or tampered artifact root therefore never yields a manifest that
  describes bytes the release does not contain.
- **The collected manifest is compared when it is present.** `collect` writes
  `release-manifest.json` beside the finalized root; when that file is there,
  the derived document must be that document (`operator.rs:1030-1049`). A
  difference fails the command instead of writing a second, different
  manifest.

Because the derived manifest is compared against the finalized tree, the
bindings and the optional consumer validator map are required inputs: the
manifest projects the same release graph `collect` projects, including its
evidence references (`operator.rs:1012`), so the derived and the collected
document cannot drift apart.

## `eggpack contract expand`

Contract M003 added the crate's second command family and its first
caller-facing scalar projection. `contract_dispatch` (`111`) reaches exactly one
subcommand, and `contract_expand` (`344`) reads, expands, projects, and prints:

```text
--contract (required, once)  local path, read through read_bounded with a 1 MiB bound
--release-id (required, once) opaque expansion input, not a release selector; [A-Za-z0-9-_.+], since it lands in artifact file names
--target (required, once)    target triple or alias, resolved by the contract
--field (required, once)     canonical-target | asset | sidecar | install
```

The semantic work is not here. `contract_expand` calls
`DistributionContract::parse_toml_str` (`348`) and `DistributionContract::expand`
(`351`) and uses their answers verbatim, which is what keeps this a projection
rather than a second implementation of schema-v1 semantics. `project_field`
(`301`) is the only new semantic judgement: `canonical-target` answers for every
asset form, and the three direct-only fields fail closed for bundle/archive
rather than picking a primary entry or archive member.

Deliberately absent, each of which would be authority this crate has no business
taking:

| Absent | Why |
|---|---|
| git discovery, repository-root search, `.` fallback, environment fallback | The input is an explicit local path. A silent search would make one command mean different things in different directories. |
| network, GitHub API, release listing, "latest" resolution | A command that can reach the network cannot be reasoned about from its arguments alone. |
| `expected-release-files`, conformance validation, a serialized expansion document | The plan rules out duplicating the conformance surface and out of adding a document whose only reader would be a human. |
| non-direct target selection | `contract expand` cannot resolve a `linux-gnu` to a musl/gnu/system-libc variant; that is `qualify_target`'s input, and guessing would be worse than failing. |
| updater, signature, provenance, install side effects | Outside the producer's first-install scope. The command writes neither the contract, nor its directory, nor anything else. |

The stdout contract is the product: exactly one scalar plus one newline, nothing
on stderr, and a nonzero exit with an empty stdout on every failure (`352-355`,
with `run`'s mapping at `14-22`). Diagnostics are bounded twice over — the
contract library truncates its own error detail, and `bound_echo` (`148`) caps
anything the CLI itself quotes from argv at `140` bytes.

Consumer policy is not inferred from any of this. A caller reading `--field
asset` still owns its own frozen public-name table, latest/exact selection,
Cargo fallback, install destination, updater, and compatibility-floor checks.
This milestone gives a consumer a way to stop re-implementing TOML parsing and
template expansion; it gives no consumer a reason to move policy into Eggpack.

## `ci generate` and `ci check`

Both require `--github-policy` and take one of two mutually exclusive input
modes; supplying both is an error (`907-912`, `961-966`).

| | Reusable mode | Exact mode |
|---|---|---|
| `ci generate` (`901`) | `--workflow-shape` + `--contract` → `render_reusable_release_github` (`927`) | `--ci-plan` → `render_release_github` (`947`) |
| `ci check` (`958`) | same inputs + `--workflow` → `check_reusable_release_github` (`982`) | same + `--workflow` → `check_release_github` (`1010`) |

`ci generate` writes exactly one file: `--output`, through `atomic_write`
(`929`, `949`). No directory creation, no second write, so a generate run
cannot touch anything the caller did not name. The output path is checked by
`reject_symlink_output` and its parent must already be a real directory
(`654`, `660-664`). Inputs are read at 1 MiB each (`918-920`, `941-942`);
success prints `generated <n> bytes to <path>` (`930-934`, `950-954`).

`ci check` never writes. There is no `atomic_write`, `fs::write`, or
`create_dir_all` anywhere in `958-1022`; the only effect on success is one
`println!`. It reads the existing workflow through `read_bounded` at 8 MiB
(`975`, `1005`), so the bound is checked against file metadata *before* any
allocation and the input is also refused if it is a symlink. This is the same
gate every other input in the crate uses; the workflow is not a special case.

Drift comparison is delegated. The CRLF→LF-only normalization lives in
`eggpack-ci` (`normalize_newlines`, `crates/eggpack-ci/src/drift.rs:72`), not
here. The CLI branches on `report.matches` and, on drift, returns `ci check:
drift detected (expected N bytes, found N bytes, first difference at ...)`
(`993-996`, `1016-1019`).

Read-only is what makes `ci check` safe in a pre-merge or scheduled job: a
failing check cannot have mutated the tree it inspects, so a red run's only side
effect is a nonzero exit.

## The nine internal `ci _*` runner commands

These exist so a generated workflow step is a file-in/file-out call with a fixed
shape. Each row can be traced end to end from a rendered step.

| Command | Handler | Reads | Writes |
|---|---|---|---|
| `_verify-source` | `358` | `--release-plan` (`359`); optionally `--github-policy`, `--expected-tag`, `--source-root`, which are all-or-none (`363-370`) | nothing; prints the pass line (`426`) |
| `_resolve-release` | `438` | `--contract`, `--pack-config`, `--build-bindings`, `--qualification-bindings`, optional `--consumer-validators`, `--template`; optional fixed `--identity-mode` from checked-in renderer policy (`439-451`, `473-477`) | `--output-plan`, `--output-ci-plan`, `--output-github-policy` (`551-553`) |
| `_capture-build` | `1024` | `--release-plan`, `--build-bindings`, `--target`, plus `--cargo-target-dir` **or** `--candidate-dir` (`1025-1037`) | `--output` (`1112-1117`) and/or `--output-dir`: `build-handoff.json` (`1108`) + `candidates/<relative_path>` (`1098-1104`) |
| `_qualify-target` | `1122` | `--release-plan`, `--build-bindings`, `--qualification-bindings`, `--contract`, `--target`, `--candidate-dir`, `--build-handoff`, optional `--qemu-sysroot` (`1123-1135`, `1161`) | `--output-dir`: `evidence.json` (`1193`), `build-handoff.json` (`1198`), `candidates/` (`1198`) |
| `_validate-consumer` | `1286` | `--consumer-validators`, `--target`, `--candidate-dir`, `--build-handoff`, `--evidence`, `--source-root` (`1287-1293`) | `--output` (consumer evidence, `1376`) |
| `_evaluate-gate` | `1393` | `--ci-plan`; per target `<inputs-dir>/<target>/evidence.json` (`1408`) or `<evidence-dir>/<handoff name>.json` (`1412`), plus `consumer-evidence.json` when validators exist (`1440`) | `--output` (gate outcome JSON, `1451`) |
| `_aggregate` | `1463` | `--contract`, `--release-plan`, `--ci-plan`, per target `<inputs-dir>/<target>/{build-handoff.json,evidence.json,candidates/}` (`1493`, `748-782`), plus consumer evidence (`1514-1520`) | `--output-root` finalized tree (via the library), `--output` summary (`1551`), `release-manifest.json` beside it (`1536-1544`) |
| `_prepare-stage` | `1586` | `--contract`, `--release-manifest`, `--finalized-root`, `--github-policy`, `--install-policy`, optional `--installer-presentation` + `--source-root` (`1587-1606`) | staging output dir via the library; `--output-payload` (`1646`) |
| `_stage-github-draft` | `1651` | `--payload`, `--github-policy`, optional `--staging-dir`; token from the environment (`1652-1654`, `1668`, `1679-1682`) | `--output-receipt` (`1702`); the draft itself, in `eggpack-github` |

They are wrappers, not a scripting interface. The input grammar is: paths, a
target name that must already exist in the plan (`1155-1159`, `767-771`), and
nothing else. There is no shell invocation, no caller-supplied program name, no
expression evaluation, and no flag whose value is interpreted as a command. The
single external program in the crate is the literal `"git"` with fixed arguments
(`402`, `421`). The one validator process is spawned inside `eggpack-ci` under a
finite interpreter enum, not from a caller string.

Invocations are rendered, not typed: `RunnerCommand::argv()` in
`eggpack-ci` (`crates/eggpack-ci/src/runner.rs:169`) is what the tests feed
straight into these handlers.

## The file-safety helpers

`read_bounded` (`589`) guards every input: `symlink_metadata`, refuse a symlink
(`592-594`), refuse anything not a regular file (`595-597`), refuse a length
above the caller's bound (`598-600`), then read through a `take(max + 1)` reader
and reject any surplus (`605-613`). The read is bounded as well as the check: a
file that grows, or is replaced, between the two cannot be read past the bound.
Failures are reported with the caller's `label`, so diagnostics name the input
class rather than echo a path. Production bounds:

| Bound | Inputs | Lines |
|---|---|---|
| 1 MiB | release plan, contract, pack config, build/qualification bindings, ci plan, github policy (generate/check), workflow shape, build handoff, evidence | `360`, `460-463`, `509`, `515`, `756`, `761`, `918-920`, `941-942`, `972-974`, `1003-1004`, `1038-1039`, `1136-1139`, `1162`, `1197`, `1303`, `1309`, `1401`, `1409`, `1413`, `1475-1477`, `1607` |
| 64 KiB | draft template, consumer validators, consumer evidence, installer presentation, github draft policy | `372`, `487`, `528`, `1297`, `1441`, `1516`, `1574` |
| 256 KiB | install policy, github draft policy | `1562`, `1609`, `1661` |
| 1 MiB | release manifest, staging payload | `1608`, `1660` |
| 8 MiB | existing workflow, via `read_bounded` (checked before *and* during the read, symlink refused) | `975`, `1005` |

`reject_symlink_output` (`617`) refuses an existing symlink or directory at the
output path. It is reached only from `atomic_write` (`654`), so callers cannot
forget it; a non-existent path passes, which is the create case.

`atomic_write` claims a fresh temporary sibling through `claim_temp`:
`.eggpack-tmp-<pid>-<counter>`, opened with `create_new` so two calls in one
process cannot collide and a pre-planted symlink is never followed (`634-651`).
It writes that file and renames it over the destination (`665-675`). Where
rename does not replace (Windows), it renames the destination *aside* to
`.eggpack-prev-<pid>-<counter>` first and retries; if that retry fails it moves
the destination back, so a failed replace never leaves the caller's file
deleted (`677-708`).

What "atomic" buys: a concurrent reader of `--output` sees either the previous
file or the complete new one, never a truncated document. All production
`atomic_write` call sites carry JSON control documents or build descriptors.

What it does **not** protect against, stated plainly:

- No `fsync` of the temp file or directory, so a crash or power loss can leave
  a truncated or missing destination.
- The Windows fallback is explicitly not atomic: between the failed rename and
  the `remove_file` the path holds no file.
- No `O_NOFOLLOW` / `linkat` re-check, and `#![forbid(unsafe_code)]` (`1`) rules
  out adding one, so the symlink check and the rename are separated by a TOCTOU
  window.
- It protects exactly the path given. The parent must already be a real
  directory (`660-664`), so commands staging a tree create it themselves:
  `create_dir_all` at `1094`, `1190`, `operator.rs:650`, and `operator.rs:755`.
- Candidate binaries bypass it entirely: `_capture-build`, `_qualify-target`,
  and the operator family's `write_build_artifacts` stage their bytes through
  the shared staging helper (`1101-1104`, `operator.rs:663-666`). Only the JSON
  descriptors are atomic; artifacts are staged before the descriptor naming
  them. The staging helper never truncates its source: the bytes are copied
  into a fresh temporary sibling and renamed over the destination, and a
  destination that resolves to the source itself is refused
  (`main.rs:833-887`).

## Path absolutization

Three helpers, three different contracts.

`absolute_path` (`1229`) returns an absolute input unchanged and otherwise joins
it to the current directory. No validation of any kind. Used for the consumer
candidate path (`1351`) and the validator work directory (`1353`).

`absolute_under_root` (`1205`) requires the root to be a real directory
(`1206-1210`), then splits the relative validator script on `/` and rejects
empty, `.`, and `..` segments (`1220-1225`). This *is* a traversal rejection,
scoped to the consumer validator script path. The comment at `1211-1212` states
the path was already validated upstream; the segment loop is the second line of
defence.

`absolutize_cli_path` (`1248`) rejects an empty path (`1249-1251`), rejects
`..` with `path must not contain parent-directory components` (`1252-1260`),
passes absolute inputs through (`1261-1263`), requires an absolute base
(`1264-1266`), drops `.` components (`1270`), and rejects anything else
with `path has an unsupported component` (`1272`).

The `..` rule applies to absolute and relative inputs alike: one rule for both
forms, so an absolute path cannot name a location the caller did not name, and
`package --output-root /abs/../x` is refused exactly like `package --output-root
../x`.

It enforces **component-level escape rejection, not containment.** No
canonicalisation, no symlink resolution, no check that the result resolves under
the base — a symlinked component inside the base still points outside it, and
existence is not tested. It is used for `--candidate-dir` in `_qualify-target`
(`1129-1132`) and `--output-root` in `_aggregate` (`1470-1473`), and by the
operator family for `--source-root`, `--output-root`, and `--output-dir`
(`operator.rs:532-533`, `operator.rs:723`, `operator.rs:591`); commands needing a real directory check
it separately.

These helpers exist because core's `inspect_candidate` and `finalize_release`
require absolute paths while the renderer deliberately emits portable relative
ones, so a checked-in workflow is location-independent. The explicit base
parameter keeps unit tests off the process working directory.
`absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative`
(`1730`) asserts absolute passthrough, `./`-relative joining, and rejection of
`../escape`, `a/../../escape`, an absolute path with a `..` component, the
empty path, and a relative base.

## Where `release-manifest.json` is written

There are now three writers of that filename, and they place it differently.

The `ci _aggregate` path writes the sidecar at `1536-1544`, inside
`if let Some(finalized)` (`1536`):

- It is derived from `--output`'s **parent**, not from `--output-root`
  (`1538-1540`). The manifest therefore follows the summary file; if a caller
  places `--output` outside the finalized root's parent, the sidecar lands
  there.
- It is written only when finalization returned a finalized root. An incomplete
  aggregate produces no manifest.
- It is skipped entirely when `--output` has no parent component (`1539`), so a
  bare filename writes the summary and no sidecar.
- Before writing, the manifest is encoded, decoded through
  `eggpack_manifest::ReleaseManifest::from_json`, and re-encoded; a mismatch is
  `staging manifest does not decode to exact manifest` (`726-739`).

The operator `collect` command writes it from the **finalized root's parent**
(`operator.rs:782-786`, `operator.rs:871-874`), not from a summary file's parent. The
finalizer writes one new release root under a caller-secured parent, so the
manifest beside that root is the one an operator actually finds.

The `eggpack manifest` command writes it where the caller says: `--output`
(`operator.rs:1020`), and only there.

The rule, precisely: **the manifest is written beside the finalized output root,
never inside it.** `eggpack-github` owns the same filename independently:
`FIXED_MANIFEST_NAME = "release-manifest.json"`
(`crates/eggpack-github/src/lib.rs:32`), written into its own staging output
directory with the same round-trip check. So the writers are: beside the
finalized root (`ci _aggregate`, `eggpack collect`), inside the staging payload
directory (`eggpack-github`), and at the caller's explicit `--output`
(`eggpack manifest`). None writes it into the finalized root. The finalizer's
own containment rule is in [core-finalization.md](core-finalization.md).

## Policy and presentation loading

`read_install_policy` (`1561`) reads at most 256 KiB with the label `install
policy` and tries `BootstrapInstallPolicyV1::from_toml` first, then
`from_json` (`1562-1568`).

`read_installer_presentation` (`1571`) reads at most 64 KiB and tries
`toml::from_str` first, calling `.validate()` explicitly on that branch
(`1576-1581`), then `InstallerPresentationV1::from_json` (`1582-1583`). The
explicit `validate()` call exists only on the TOML path; I did not verify that
`from_json` performs the same checks internally.

Both are consumed only by `_prepare-stage` (`1616`, `1619`).
`--installer-presentation` and `--source-root` are paired: one without the other
is an error (`1601-1606`). The pair selects
`prepare_staging_payload_with_presentation` (`1620`) for product wrappers;
`GeneratedDefault` passes neither and calls `prepare_staging_payload` (`1632`).
Other bounds on that path: release manifest 1 MiB (`1608`), github draft policy
256 KiB (`1609`).

They are inputs, not derivations. Installer policy and presentation are decisions
owned by the product repository and validated by `eggpack-bootstrap` and
`eggpack-github`; re-deriving either here would create a second source of truth
for installer content, so the CLI only reads and bounds them.

## Process execution: the `git` site

`verify_source_revision` (`410`) is the production HEAD verification spawn in
this crate — and it no longer constructs a `Command` itself. It delegates to
`eggpack_core::run_git_bounded` (`421`), while `verify_source_tag` (`401`)
calls `run_git_tag_bounded` (`402`) for the exact local tag peel, so the
bounded, process-grouped spawn lives in `builder.rs`, not here. `grep -n
'Command::new'` in this file now returns **only** test sites; there is no
production `Command::new` in `eggpack-cli` at all. The file likewise reads no
named environment variable: its only `std::env` uses are `args()` (`11`),
`current_dir()`, and a test-only `temp_dir()`.

What it does: it validates that the expected revision is 40 lowercase hex
characters (`411-416`), then delegates to `eggpack_core::run_git_bounded`
(`421`), which owns the spawn. That helper builds a `CommandSpec` for
`git rev-parse --verify HEAD^{commit}` (`builder.rs:513-518`) and, for the tag
peel, `git rev-parse --verify refs/tags/<tag>^{commit}`
(`builder.rs:535-541`); both use the same bounded runner with a 30 s deadline
(`builder.rs:29`), a 4 KiB retained-output cap (`builder.rs:31`), a
`command-group` process group, and a cleared environment (`builder.rs:735`).
See
[process-execution.md](process-execution.md) for the workspace spawn contract.

Why it is load-bearing. A generated workflow renders concrete plans, paths, and
policies. If the checkout has moved — a rebase, a tag force-update, a detached
job at the wrong commit — the workflow would build and aggregate bytes for a
commit other than the one the plan was derived from, so every downstream artifact
would describe a release the plan does not describe. Two call sites pin the
identity: `ci _verify-source` (`358`) requires the plan's `source_revision` to
be 40 lowercase hex then verifies it against the inherited working directory
(`383`, `398`); `ci _resolve-release` (`438`) verifies the caller-supplied
`--source-revision` against the explicit `--source-root` (`458`) and the tag
against the same revision (`459`) before resolving anything.

The tag peel is not mapped-only: `ci _resolve-release` peels the exact local
tag in every identity mode (`459`). Mapped reusable mode additionally resolves
the manifest ID under the fixed renderer-supplied identity mode.
`_verify-source` receives the runtime draft policy and event-selected tag, then
checks the tag, mapped ID, plan revision, HEAD, and tag peel as one tuple
(`371-385`); that branch requires the policy's mode to be
`VPrefixedStableSemver` (`376-377`). These extra arguments are absent in legacy
exact-tag mode.

It fails closed. Anything other than `CommandOutcome::Success` is an error
(`404-406`, `423-425`): spawn failure, deadline, output-cap breach, non-zero
status, and mismatch all collapse to one message that carries no git output.
Only exact equality passes, and the pass path prints a fixed sentence carrying
no revision text (`426`).

The expected object name is matched *inside* the runner and never returned to
this process, so the CLI cannot leak stdout even by accident. The comparison is
exact rather than containment: the `expected_stdout_exact` field of the
`CommandSpec` (`builder.rs:177-181`) requires stdout to equal the expected value,
which a substring check would not.

## Error and exit discipline

`main` (`10-12`) exits with the `i32` from `run` (`14-22`): zero on `Ok(())`,
otherwise one `eprintln!("eggpack: {message}")` (`18`) and `1`. There is exactly
one `eprintln!` in the file, so every failure surfaces as one prefixed line on
stderr.

Handlers return `Result<(), String>`, and library errors are almost always
collapsed to a fixed static message by `.map_err(|_| "…")` — `invalid release
plan`, `handoff projection failed`, `gate evaluation failed`. A diagnostic
therefore names a cause class rather than reproducing a library message. The one
deliberate exception is `_stage-github-draft` (`1698`), which interpolates the
inner `GithubError`; the comment at `1695-1697` records that the type carries
only static bounded text — no token, body, or URL — and that live diagnosis
requires naming it.

The operator family collapses library errors the same way, but keeps the cause
class visible: `diagnostic` (`operator.rs:579-588`) reduces a multi-line parser
diagnostic to its last non-empty line and bounds it, so the exit discipline
stays one line.

Diagnostics never echo captured process output. `git` stdout is compared, not
printed. Consumer validator output is excluded from durable evidence by type,
and the CLI additionally refuses to write evidence whose JSON contains `stdout`
or `stderr` (`1373-1375`); a failed validation reports the reason variant by
`Debug` (`1382-1389`), naming the cause (interpreter, candidate identity,
script, timeout, exit) and not the output.

The GitHub token is read by name from the environment
(`eggpack_github::read_token(&policy.token_env)`, `1668`); absence fails before
any network I/O, and the token text is never serialized, logged, or included in
an error.

Success output is a fixed set of bounded lines on stdout (`354`, `426`, `554`,
`930`, `950`, `990`, `1013`, `1118`, `1201`, `1379`, `1455`, `1554`, `1647`,
`1703`, and `operator.rs:506`, `355`, `404`, `579`, `648`, `728`). The operator
family's six lines are exactly `<command>: ok`.

Limitation worth stating: there are only two exit codes. A caller cannot
distinguish drift from a malformed plan by status alone, only by message text.

## Dependencies / dependents

Dependencies (`crates/eggpack-cli/Cargo.toml:19-27`): `eggpack-contract`,
`eggpack-core`, `eggpack-ci`, `eggpack-manifest`, `eggpack-bootstrap`,
`eggpack-github`, plus `serde_json`, `toml` 0.8, and `tokio` with the `rt`
feature only. Dev-dependency: `sha2` 0.10 (`:30`). No `clap`, no HTTP client, no
`command-group`, no YAML emitter.

The crate has two integration-test files, `crates/eggpack-cli/tests/contract_expand.rs`
(Contract M003) and `crates/eggpack-cli/tests/operator_commands.rs` (M009).
They exist because the `contract expand` stdout/stderr/exit-code contract and
the operator commands' exit-code and argument contract are only observable from
outside the process: the unit tests inside `main.rs` can see a handler's
`Result` but not what the binary actually wrote.

This is the only crate that depends on all six workspace crates, so it is the
only place where the whole pipeline can be driven in one process. That is what
`generated_orchestration_cli_executes_capture_to_aggregate` (`2064`) does: it
builds argv from `RunnerCommand::argv()` and calls the handlers directly —
capture, qualify, gate, aggregate — then asserts the negative cases. The
operator family has the same property in the other direction: `eggpack plan`
and `ci _resolve-release` share one encoder (`717-719`), and
`package_qualify_collect_manifest_matches_ci_aggregate`
(`tests/operator_commands.rs:650`) asserts that the local path produces the
same bytes the CI path produces.

To be precise about a common confusion: no workspace crate depends on
`eggpack-cli` — the CLI is a leaf, and `eggpack-ci` does *not* depend on it.
What [ci.md](ci.md) records is that `eggpack-ci`'s *own* dev-dependencies let
*its* tests drive downstream crates.

Dependents: end users, and the generated workflow, whose steps are invocations of
this binary rendered by `eggpack-ci`'s `RunnerCommand`
(`crates/eggpack-ci/src/runner.rs:169`).

## Tests

Twenty-seven `#[test]` functions in `mod tests` (`1711`). The `ci` tests call
handler functions and `verify_source_revision` in-process rather than spawning
the binary; the `m003_*` tests also drive `run()` and `dispatch` (`4555`), so
the exit-code mapping is exercised for the `contract` family. The eight tests
in `crates/eggpack-cli/tests/contract_expand.rs` and the eighteen in
`crates/eggpack-cli/tests/operator_commands.rs` spawn the real binary, which is
the only way to observe its stdout contract.

| Test | Line | Pins |
|---|---|---|
| `absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative` | `1730` | Path adapter: passthrough, `.` joining, `..`/empty/relative-base rejection |
| `stage_github_draft_accepts_rendered_four_flag_call` | `1758` | Arity bound admits the renderer's eight-argument call; ten fails |
| `source_verifier_accepts_tag_commit_and_rejects_other_checkout` | `1784` | `git` identity: matching HEAD passes, other revision and non-hex fail |
| `generate_equals_library_and_check_detects_drift` | `1939` | `generate` == library renderer; CRLF matches; one byte drifts; bad input errors |
| `generated_orchestration_cli_executes_capture_to_aggregate` | `2064` | Full capture → qualify → gate → aggregate path plus negative cases and manifest placement |
| `prepare_stage_materializes_exact_payload` | `2301` | Staging payload contents from a real finalized root |
| `resolve_release_emits_distinct_runtime_identity_per_tag` | `2547` | Per-tag resolution identity; wrong revision and malformed tag fail |
| `m003i_mapped_identity_survives_two_target_capture_qualification_finalize_and_stage` | `2637` | Mapped identity across a two-target capture → qualify → finalize → stage path |
| `validate_consumer_executes_typed_command_end_to_end` | `2959` | Consumer validator against real candidate bytes |
| `validate_consumer_refuses_failed_qualification_evidence` | `3123` | Non-`Passed` evidence refused before execution (`1317-1321`) |
| `prepare_stage_with_product_wrappers_materializes_four_installers` | `3274` | Presentation + source-root pair produces the wrapper set |
| `shape_generate_check_round_trip_and_drift` | `3412` | Reusable shape mode round trip and drift |
| `shape_generate_and_check_accept_native_qualification_for_cross_tool_targets` | `3615` | Reusable mode with native qualification across tool targets |
| `m003_argument_parser_is_exact_and_rejects_ambiguity` | `3981` | Every accepted form; duplicate flag, unknown option, positional argument, missing value, empty value, missing-required set, unsupported field |
| `echoed_argv_diagnostics_are_bounded_without_panicking` | `4054` | Diagnostics quoting argv stay bounded |
| `m003_expands_every_field_and_resolves_aliases` | `4094` | CLI equals `DistributionContract::expand` for all four fields, through both a triple and an alias |
| `m003_release_id_is_opaque_and_accepts_prerelease_punctuation` | `4152` | Opaque expansion input including `v1.2.3-rc.1+build.5`; path separators and empty rejected |
| `m003_fails_closed_for_bundle_and_archive_direct_fields` | `4206` | `canonical-target` answers for both non-direct forms; the three direct-only fields do not |
| `m003_rejects_invalid_schema_unknown_targets_and_missing_input` | `4238` | schema_version 2, unknown key, malformed TOML, unknown triples and aliases, absent file, directory input |
| `m003_rejects_oversized_and_symlink_contract_input` | `4300` | The 1 MiB bound and the symlink refusal |
| `atomic_write_replaces_in_place_and_leaves_no_temp_or_backup` | `4341` | The atomic writer replaces in place and leaves no temp or backup file |
| `atomic_write_refuses_a_symlinked_output` | `4361` | A symlinked output path is refused |
| `read_bounded_rejects_a_file_that_grows_past_the_bound` | `4379` | A file that grows past the bound mid-read is refused |
| `m003_never_mutates_the_contract_and_is_deterministic` | `4411` | Five equal runs, identical bytes, contract untouched on success and on failure |
| `m003_consumer_shaped_contract_matches_frozen_public_names` | `4444` | The live consumer contract shape against the public names both consumers publish, including the `.exe` rule; ARMv7 absent |
| `m003_dispatch_adds_a_family_without_regressing_ci` | `4555` | `run()` exit mapping for both families; `unknown ci subcommand` unchanged |
| `m003_projects_fields_without_a_serialized_expansion_model` | `4586` | No record, list, or member selector exists |
| `contract_expand::success_prints_exactly_one_scalar_and_nothing_else` | `tests/contract_expand.rs:113` | stdout is the scalar plus one newline and stderr is empty, on both direct fixtures |
| `contract_expand::repeated_invocations_are_byte_identical` | `tests/contract_expand.rs:163` | Determinism across processes |
| `contract_expand::the_command_never_writes_to_the_contract_or_its_directory` | `tests/contract_expand.rs:176` | No file created or modified, on success and on every failure shape |
| `contract_expand::every_failure_mode_exits_nonzero_with_an_empty_stdout` | `tests/contract_expand.rs:197` | Nonzero exit, empty stdout, bounded stderr across all rejection paths |
| `contract_expand::canonical_target_is_valid_for_every_asset_form` | `tests/contract_expand.rs:230` | Bundle and archive both resolve a canonical target |
| `contract_expand::argument_errors_exit_nonzero_without_a_scalar` | `tests/contract_expand.rs:250` | One defect per invocation; truncated invocations report `missing required`; `--flag=value` accepted |
| `contract_expand::symlinked_and_oversized_contracts_are_refused` | `tests/contract_expand.rs:312` | Symlink and size-bound refusals at the process boundary |
| `contract_expand::the_documented_readme_example_runs_verbatim` | `tests/contract_expand.rs:350` | The CLI README example, with its documented relative paths and working directory |
| `operator_commands::every_command_exits_zero_on_success_and_one_on_failure` | `tests/operator_commands.rs:395` | Exit-code mapping for all six commands |
| `operator_commands::check_accepts_a_valid_input_set` | `tests/operator_commands.rs:434` | The validation class accepts a resolvable input set |
| `operator_commands::plan_writes_the_same_bytes_as_ci_resolve_release` | `tests/operator_commands.rs:581` | One plan encoder, two surfaces |
| `operator_commands::package_qualify_collect_manifest_matches_ci_aggregate` | `tests/operator_commands.rs:650` | The local path's final bytes equal the CI path's |
| `operator_commands::help_names_every_command_and_its_class` | `tests/operator_commands.rs:1197` | The taxonomy is part of the help |
| `operator_commands::existing_command_families_still_behave` | `tests/operator_commands.rs:1244` | The `ci` and `contract` families are unregressed |

The coverage shape follows from the crate's role. Most tests assert argument
handling, arity, path helpers, and renderer parity, because that is where the
crate's own logic lives. The one substantial end-to-end path (`2064`) proves
that the handlers compose; the real proof that generated workflows run against
real binaries lives in `eggpack-ci`'s own tests and in the hosted CI lanes.
Contract M003 gave the crate an integration-test surface for the first time,
which matters more than the row count: before it, "stdout is exactly one scalar
and every failure is silent on stdout" was a README claim with nothing behind
it, the same shape as the earlier unguarded-example drift. The
`the_documented_readme_example_runs_verbatim` guard exists so that cannot recur.

Not covered here, and therefore guaranteed only by reading the code: the Windows
rename fallback in `atomic_write` (`677-708`), and `reject_symlink_output`
against a directory or symlink (`617-627`). `run()`'s exit-code mapping and
`dispatch`'s unknown-top-level-token rejection are covered for the `contract`
family (`4555`, `tests/contract_expand.rs:250`) and for the operator family
(`tests/operator_commands.rs:395`) but still only in-process for the `ci`
family. See [testing-and-portability.md](testing-and-portability.md) for the
workspace view.

## Related deep dives

- [overview.md](overview.md) — workspace-level view and module map
- [ci.md](ci.md) — the CI crate that renders invocations of this binary
- [ci-rendering.md](ci-rendering.md) — renderers, policy types, drift checking
- [ci-consumer-seam.md](ci-consumer-seam.md) — consumer validator and runtime identity
- [core.md](core.md) — planning, build, qualification, finalization
- [core-finalization.md](core-finalization.md) — output root containment and manifest
- [core-qualification.md](core-qualification.md) — host-matched qualification proof
- [core-build.md](core-build.md) — bounded builder execution
- [github.md](github.md) — draft staging, policy, `FIXED_MANIFEST_NAME`
- [bootstrap.md](bootstrap.md) — install policy and installer rendering
- [process-execution.md](process-execution.md) — the workspace spawn contract this `git` site does not follow
- [validation-model.md](validation-model.md) — fail-closed validation rules
- [testing-and-portability.md](testing-and-portability.md) — test strategy and hosted lanes
