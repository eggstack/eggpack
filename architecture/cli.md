# `eggpack-cli` — Deep Dive

The deterministic wiring layer of the workspace and the only binary the
generated workflow actually invokes. `eggpack-cli` contributes exactly one
artifact — the `eggpack` binary (`crates/eggpack-cli/Cargo.toml11-13`) from a
single file, `crates/eggpack-cli/src/main.rs` (2940 lines) — and every release
decision in it is delegated to a library crate.

All citations below are `crates/eggpack-cli/src/main.rs:LINE` unless another
file is named.

## Responsibility and the wiring rule

The crate parses `argv`, reads bounded files, calls into the library crates, and
writes files. It holds no layout authority (that is `eggpack-contract`), no
planning, build, qualification, or finalization logic (`eggpack-core`), no
rendering, gating, or aggregation logic (`eggpack-ci`), no draft-staging logic
(`eggpack-github`), and no installer rendering (`eggpack-bootstrap`).

The boundary is verifiable rather than aspirational. There are zero `pub` items
in the file, so nothing here is a library API another crate can depend on. Every
semantic question is asked of a library call and the answer used verbatim:
`PackConfig::resolve` and `GitHubDraftTemplateV1::resolve` via
`resolve_runtime_release_plan_with_mode` (`441`, `449`), gate policy via
`evaluate_gate` (`1188`, `1227`), qualification via `qualify_target` (`900`),
finalization via `aggregate_finalize[_with_consumer]` (`1320`, `1338`).

What the crate does keep are fail-closed preconditions checked *before*
delegation, plus path adapters. Neither is a release rule:

| Local check | Line | Why it is wiring, not policy |
|---|---|---|
| Argument arity caps | `374`, `569`, `592`, `623`, `654`, `971`, `1306`, `1367` | Rejects an invocation the renderer would never emit. No release meaning. |
| Paired-flag requirements | `690-695`, `1075-1077`, `1311-1316` | Selects between two library entry points. |
| Non-`Passed` evidence refused before a consumer script runs | `994-999` | A guard on the validator seam; the gate also fails closed on it. |
| Handoff/evidence identity cross-checks | `983-985`, `1000-1005`, `1006-1019` | Confirms the two files describe the same target, release, and revision. |
| Stray `consumer-evidence.json` rejected | `1099-1108`, `1196-1201` | Prevents validation being silently skipped. |
| Manifest round-trip byte comparison | `1240-1247` | Proves the sidecar is the same document, not a re-serialization. |
| `absolute_path` / `absolute_under_root` / `absolutize_cli_path` | `912`, `888`, `931` | Adapters between the renderer's portable relative paths and the absolute paths core requires (`921-930`). |
| `contract expand` 1 MiB input bound | `94`, `298` | Rejects an oversized local contract before parsing. No release meaning. |
| `contract expand` scalar output bound | `100`, `281-285` | Refuses to print a value it cannot bound. No release meaning. |
| `contract expand` exact argument set | `181-244` | A consumer script reads this output, so silently keeping the first of a repeated flag is not acceptable here. |

The rule of thumb: if the answer would change when a library changes, the check
belongs in the library. The adapters are the only code here whose reason for
existing is a mismatch between two other crates, and each says so at its call
site (`787-788`, `1145-1146`).

## Argument parsing, hand-rolled

`dispatch` (`:21`) selects a command family and nothing else. Two usage
constants hold the help text — `USAGE` (`:44`) and `CONTRACT_USAGE` (`:47`) —
reused verbatim at `:23`, `:30`, `:31`, `:37`, `:38`, `:53`, and `:77`:

```text
usage: eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]

usage: eggpack contract expand --contract <distribution.toml> --release-id <opaque-release-id> --target <triple-or-alias> --field <canonical-target|asset|sidecar|install>
```

Order of decisions: empty argv is a usage error (`:22-24`); `--version`/`-V`
prints `eggpack <CARGO_PKG_VERSION>` and exits 0 (`:25-28`); `--help`/`-h`
prints both usage lines and exits 0 (`:29-32`); all three are handled *before*
the family match, so they work only as the first token. The family match at
`:34-40` accepts exactly `ci` and `contract`; any other first token is a usage
error naming the family. `ci_dispatch` (`:51`) preserves the historical
behaviour verbatim, including the `unknown ci subcommand` diagnostic (`:68`) and
the eleven subcommand arms (`:57-67`), plus a bare `ci` as a usage error
(`:52-54`). `contract_dispatch` (`:73`) accepts `expand` (`:75`), `help`
(`:76-79`), and rejects anything else (`:80`, `:81`).

`contract expand` deliberately does **not** use the shared flag helper.
`parse_contract_expand_args` (`:181`) is command-local and exact: it accepts
`--name value` and `--name=value` (`:194-197`, `:204-212`), rejects positional
arguments (`:188-193`), unknown options (`:198-200`), a repeated flag rather
than keeping the first (`:201-203`), empty values (`:213-215`), and reports
every missing required flag in sorted order (`:220-228`).

`get_flag` (`465`) accepts exactly two forms: `--name value` (`469-474`) and
`--name=value` (`475-477`). Its two error messages are `missing value for
--name` (`473`) and `missing required --name` (`479`). `get_flag_optional`
(`482`) is the same scan returning `Option`. Both scan left to right and return
the first match, so a repeated flag silently takes the first occurrence. That is
why `contract expand` parses its own arguments rather than inheriting the
looseness of the shared helper.

In the `ci` family, unknown flags are **not** rejected. No function scans for
unrecognised arguments; a typo is caught only if it pushes the argument count
past a cap:

| Command | Cap | Line |
|---|---|---|
| `ci _resolve-release` | 30 | `415` |
| `ci generate` (reusable / exact) | 8 / 6 | `691` / `714` |
| `ci check` (reusable / exact) | 8 / 6 | `745` / `776` |
| `ci _validate-consumer` | 14 | `971` |
| `ci _prepare-stage` | 18 | `1306` |
| `ci _stage-github-draft` | 8 | `1367` |
| `ci _capture-build`, `ci _evaluate-gate` | none | paired-flag checks instead (`690-695`, `1075-1077`) |

`#![forbid(unsafe_code)]` (`1`) holds workspace-wide. `#![deny(missing_docs)]`
(`2`) has no target: the crate exposes no public items, so in a binary it is
inert. The three `///` blocks (`352-358`, `921-930`, `955-961`) document
handlers for internal readers, not for rustdoc.

The trade-off is deliberate. No `clap` means no dependency, no derive expansion,
and a parse whose behaviour is fully determined by the code above. The cost is
hand-maintained parsing, flat usage strings as the only help, no completions,
and — in the `ci` family — a typo diagnosed as `too many arguments for ci ...`
rather than by name. `contract expand` pays for a second, stricter parser
instead, because a consumer script reads its output.

## `eggpack contract expand`

Contract M003 added the crate's second command family and its first
caller-facing scalar projection. `contract_dispatch` (`:73`) reaches exactly one
subcommand, and `contract_expand` (`:295`) reads, expands, projects, and prints:

```text
--contract (required, once)  local path, read through read_bounded with a 1 MiB bound
--release-id (required, once) opaque expansion input, not a release selector; [A-Za-z0-9-_.+], since it lands in artifact file names
--target (required, once)    target triple or alias, resolved by the contract
--field (required, once)     canonical-target | asset | sidecar | install
```

The semantic work is not here. `contract_expand` calls
`DistributionContract::parse_toml_str` (`:299`) and `DistributionContract::expand`
(`:301`) and uses their answers verbatim, which is what keeps this a projection
rather than a second implementation of schema-v1 semantics. `project_field`
(`:252`) is the only new semantic judgement: `canonical-target` answers for every
asset form (`:258-260`), and the three direct-only fields fail closed for
bundle/archive (`:261-269`) rather than picking a primary entry or archive
member.

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
on stderr, and a nonzero exit with an empty stdout on every failure (`:278-285`,
with `run`'s mapping at `:11-19`). Diagnostics are bounded twice over — the
contract library truncates its own error detail, and `bound_echo` (`:105`) caps
anything the CLI itself quotes from argv at `:100` bytes.

Consumer policy is not inferred from any of this. A caller reading `--field
asset` still owns its own frozen public-name table, latest/exact selection,
Cargo fallback, install destination, updater, and compatibility-floor checks.
This milestone gives a consumer a way to stop re-implementing TOML parsing and
template expansion; it gives no consumer a reason to move policy into Eggpack.


## `ci generate` and `ci check`

Both require `--github-policy` and take one of two mutually exclusive input
modes; supplying both is an error (`563-565`, `617-619`).

| | Reusable mode | Exact mode |
|---|---|---|
| `ci generate` (`555`) | `--workflow-shape` + `--contract` → `render_reusable_release_github` (`581`) | `--ci-plan` → `render_release_github` (`601`) |
| `ci check` (`612`) | same inputs + `--workflow` → `check_reusable_release_github` (`641`) | same + `--workflow` → `check_release_github` (`668`) |

`ci generate` writes exactly one file: `--output`, through `atomic_write`
(`583`, `603`). No directory creation, no second write, so a generate run
cannot touch anything the caller did not name. The output path is checked by
`reject_symlink_output` and its parent must already be a real directory
(`530-534`). Inputs are read at 1 MiB each (`572-574`, `595-596`); success
prints `generated <n> bytes to <path>` (`584-588`).

`ci check` never writes. There is no `atomic_write`, `fs::write`, or
`create_dir_all` anywhere in `605-673`; the only effect on success is one
`println!`. It reads the existing workflow through `read_bounded` at 8 MiB
(`622`, `652`), so the bound is checked against file metadata *before* any
allocation and the input is also refused if it is a symlink. This is the same
gate every other input in the crate uses; the workflow is not a special case.

Drift comparison is delegated. The CRLF→LF-only normalization lives in
`eggpack-ci` (`normalize_newlines`, `crates/eggpack-ci/src/lib.rs2072`, applied
at `lib.rs:3818-3819`), not here. The CLI branches on `report.matches` and, on
drift, returns `ci check: drift detected (expected N bytes, found N bytes, first
difference at ...)` (`647-651`, `674-678`).

Read-only is what makes `ci check` safe in a pre-merge or scheduled job: a
failing check cannot have mutated the tree it inspects, so a red run's only side
effect is a nonzero exit. The unit test pins both halves — a CRLF-converted
workflow still matches, one appended byte fails (`1678-1699`).

## The nine internal `ci _*` runner commands

These exist so a generated workflow step is a file-in/file-out call with a fixed
shape. Each row can be traced end to end from a rendered step.

| Command | Handler | Reads | Writes |
|---|---|---|---|
| `_verify-source` | `309` | `--release-plan` (`311`) | nothing; prints the pass line (`348`) |
| `_resolve-release` | `401` | `--contract`, `--pack-config`, `--build-bindings`, `--qualification-bindings`, optional `--consumer-validators`, `--template`; optional fixed `--identity-mode` from checked-in renderer policy (`402-415`) | `--output-plan`, `--output-ci-plan`, `--output-github-policy` (`412-414`) |
| `_capture-build` | `682` | `--release-plan`, `--build-bindings`, plus `--cargo-target-dir` **or** `--candidate-dir` (`696-729`) | `--output` (`776`) and/or `--output-dir`: `build-handoff.json` (`768`) + `candidates/<relative_path>` (`759-764`) |
| `_qualify-target` | `782` | `--release-plan`, `--build-bindings`, `--qualification-bindings`, `--build-handoff`, `--contract`, optional `--qemu-sysroot` (`796-825`) | `--output-dir`: `evidence.json` (`853`), `build-handoff.json` (`858-861`), `candidates/` (`872-881`) |
| `_validate-consumer` | `963` | `--consumer-validators`, `--build-handoff`, `--evidence`, `--source-root` (`974-1029`) | `--output` (consumer evidence, `1053`) |
| `_evaluate-gate` | `1070` | `--ci-plan`; per target `<inputs-dir>/<target>/evidence.json` (`1085`) or `<evidence-dir>/<handoff name>.json` (`1089`), plus `consumer-evidence.json` when validators exist (`1117`) | `--output` (gate outcome JSON, `1128`) |
| `_aggregate` | `1140` | `--contract`, `--release-plan`, `--ci-plan`, per target `<inputs-dir>/<target>/{build-handoff.json,evidence.json,candidates/}` (`1166-1191`), plus consumer evidence (`1214`) | `--output-root` finalized tree (via the library), `--output` summary (`1261`), `release-manifest.json` beside it (`1250-1251`) |
| `_prepare-stage` | `1296` | `--contract`, `--release-manifest`, `--finalized-root`, `--github-policy`, `--install-policy`, optional `--installer-presentation` + `--source-root` (`1317-1330`) | staging output dir via the library; `--output-payload` (`1356`) |
| `_stage-github-draft` | `1361` | `--payload`, `--github-policy`, optional `--staging-dir`; token from the environment (`1370-1378`) | `--output-receipt` (`1412`); the draft itself, in `eggpack-github` |

They are wrappers, not a scripting interface. The input grammar is: paths, a
target name that must already exist in the plan (`815-819`, `1179-1183`), and
nothing else. There is no shell invocation, no caller-supplied program name, no
expression evaluation, and no flag whose value is interpreted as a command. The
single external program in the crate is the literal `"git"` with fixed arguments
(`326-327`). The one validator process is spawned inside `eggpack-ci` under a
finite interpreter enum, not from a caller string.

Invocations are rendered, not typed: `RunnerCommand::argv()` in
`eggpack-ci` (`crates/eggpack-ci/src/lib.rs1372`) is what the tests feed
straight into these handlers (argv[0] assertions at `lib.rs:5846`, `6722`, `6741`,
`8299`).

## The file-safety helpers

`read_bounded` (`496`) guards every input: `symlink_metadata`, refuse a symlink
(`499-501`), refuse anything not a regular file (`502-504`), refuse a length
above the caller's bound (`505-507`), then read through a `take(max + 1)` reader
and reject any surplus (`510-522`). The read is bounded as well as the check: a
file that grows, or is replaced, between the two cannot be read past the bound.
Failures are reported with the caller's `label`, so diagnostics name the input
class rather than echo a path. Production bounds:

| Bound | Inputs | Lines |
|---|---|---|
| 1 MiB | release plan, contract, pack config, build/qualification bindings, ci plan, github policy (generate/check), workflow shape, build handoff, evidence | `311`, `381-384`, `415-421`, `572-574`, `595-596`, `626-628`, `657-658`, `696-697`, `796-799`, `980-986`, `1078`, `1152-1154`, `1168-1173` |
| 64 KiB | draft template, consumer validators, consumer evidence, installer presentation | `402`, `434`, `974`, `1118`, `1215`, `1284` |
| 256 KiB | install policy, github draft policy | `1272`, `1319`, `1371` |
| 1 MiB | release manifest, staging payload | `1318`, `1370` |
| 8 MiB | existing workflow, via `read_bounded` (checked before *and* during the read, symlink refused) | `622`, `652` |

`reject_symlink_output` (`504`) refuses an existing symlink or directory at the
output path. It is reached only from `atomic_write` (`516`), so callers cannot
forget it; a non-existent path passes, which is the create case.

`atomic_write` claims a fresh temporary sibling through `claim_temp`:
`.eggpack-tmp-<pid>-<counter>`, opened with `create_new` so two calls in one
process cannot collide and a pre-planted symlink is never followed. It writes
that file and renames it over the destination. Where rename does not replace
(Windows), it renames the destination *aside* to `.eggpack-prev-<pid>-<counter>`
first and retries; if that retry fails it moves the destination back, so a
failed replace never leaves the caller's file deleted.

What "atomic" buys: a concurrent reader of `--output` sees either the previous
file or the complete new one, never a truncated document. All 15 production
`atomic_write` call sites carry JSON control documents (`458-460`, `583`,
`603`, `768`, `776`, `853`, `858`, `1053`, `1128`, `1251`, `1261`,
`1356`, `1412`).

What it does **not** protect against, stated plainly:

- No `fsync` of the temp file or directory (`538-539`), so a crash or power loss
  can leave a truncated or missing destination.
- The Windows fallback is explicitly not atomic: between the failed rename
  (`539`) and the `remove_file` (`545`) the path holds no file.
- No `O_NOFOLLOW` / `linkat` re-check, and `#![forbid(unsafe_code)]` (`1`) rules
  out adding one, so the check at `511` and the rename at `539` are separated
  by a TOCTOU window.
- It protects exactly the path given. The parent must already be a real directory
  (`530-534`), so commands staging a tree create it themselves: `create_dir_all`
  at `752`, `850`, `873`.
- Candidate binaries bypass it entirely: `_capture-build` and `_qualify-target`
  hard-link with a `copy` fallback (`761-763`, `878-880`). Only the JSON
  descriptors are atomic; artifacts are staged before the descriptor naming them,
  and the directory validators (`769-770`, `882-883`) run afterwards.

## Path absolutization

Three helpers, three different contracts.

`absolute_path` (`912`) returns an absolute input unchanged and otherwise joins
it to the current directory. No validation of any kind. Used for the consumer
candidate path (`1028`) and the validator work directory (`1030`).

`absolute_under_root` (`888`) requires the root to be a real directory
(`889-893`), then splits the relative validator script on `/` and rejects empty,
`.`, and `..` segments (`903-908`). This *is* a traversal rejection, scoped to
the consumer validator script path. The comment at `894-895` states the path was
already validated upstream; the segment loop is the second line of defence.

`absolutize_cli_path` (`931`) rejects an empty path (`932-934`), passes
absolute inputs through (`935-937`), requires an absolute base (`938-940`),
drops `.` components (`944`), rejects `..` with `path must not contain
parent-directory components` (`945-947`), and rejects anything else with `path
has an unsupported component` (`949`).

It enforces **component-level escape rejection, not containment.** No
canonicalisation, no symlink resolution, no check that the result resolves under
the base — a symlinked component inside the base still points outside it, and
existence is not tested. It is used for `--candidate-dir` in `_qualify-target`
(`789-792`) and `--output-root` in `_aggregate` (`1147-1150`); commands needing a
real directory check it separately (`714-727`, `828-833`).

These helpers exist because core's `inspect_candidate` and `finalize_release`
require absolute paths while the renderer deliberately emits portable relative
ones, so a checked-in workflow is location-independent (doc comment `921-930`,
call sites `787-788`, `1145-1146`). The explicit base parameter keeps unit tests
off the process working directory. `absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative`
(`1440`) asserts absolute passthrough, `./`-relative joining, and rejection of
`../escape`, `a/../../escape`, the empty path, and a relative base.

## Where `release-manifest.json` is written

The aggregate path writes the sidecar at `1248-1253`, inside
`if let Some(finalized)` (`1235`):

- It is derived from `--output`'s **parent**, not from `--output-root`
  (`1248-1250`). The manifest therefore follows the summary file; if a caller
  places `--output` outside the finalized root's parent, the sidecar lands there.
- It is written only when finalization returned a finalized root. An incomplete
  aggregate produces no manifest.
- It is skipped entirely when `--output` has no parent component (`1249`), so a
  bare filename writes the summary and no sidecar.
- Before writing, the manifest is encoded, decoded through
  `eggpack_manifest::ReleaseManifest::from_json`, and re-encoded; a mismatch is
  `staging manifest does not decode to exact manifest` (`1240-1247`).

The rule, precisely: **the manifest is written beside the finalized output root,
never inside it.** The test pins both halves — `root/release-manifest.json`
decodes to the exact manifest, and `!output_root.join("release-manifest.json")
.exists()` guards M004 root semantics (`1931-1942`).

`eggpack-github` owns the same filename independently:
`FIXED_MANIFEST_NAME = "release-manifest.json"`
(`crates/eggpack-github/src/lib.rs:31`), written into its own staging output
directory with the same round-trip check (`lib.rs1031-1044`, `lib.rs1700-1713`).
So there are two writers of the same name, into two different directories: beside
the finalized root (this crate) and inside the staging payload directory
(`eggpack-github`). Neither writes it into the finalized root. The finalizer's
own containment rule is in [core-finalization.md](core-finalization.md).

## Policy and presentation loading

`read_install_policy` (`1271`) reads at most 256 KiB with the label `install
policy` and tries `BootstrapInstallPolicyV1::from_toml` first, then
`from_json` (`1274-1278`).

`read_installer_presentation` (`1281`) reads at most 64 KiB and tries
`toml::from_str` first, calling `.validate()` explicitly on that branch
(`1286-1291`), then `InstallerPresentationV1::from_json` (`1292-1293`). The
explicit `validate()` call exists only on the TOML path; I did not verify that
`from_json` performs the same checks internally.

Both are consumed only by `_prepare-stage` (`1326`, `1329`).
`--installer-presentation` and `--source-root` are paired: one without the other
is an error (`1311-1316`). The pair selects
`prepare_staging_payload_with_presentation` (`1330`) for product wrappers;
`GeneratedDefault` passes neither and calls `prepare_staging_payload` (`1342`).
Other bounds on that path: release manifest 1 MiB (`1318`), github draft policy
256 KiB (`1319`).

They are inputs, not derivations. Installer policy and presentation are decisions
owned by the product repository and validated by `eggpack-bootstrap` and
`eggpack-github`; re-deriving either here would create a second source of truth
for installer content, so the CLI only reads and bounds them.

## Process execution: the `git` site

`verify_source_revision` (`373`) is the production HEAD verification spawn in
this crate — and it no longer constructs a `Command` itself. It delegates to
`eggpack_core::run_git_bounded` (`384-385`), while mapped identity also calls
`run_git_tag_bounded` (`364-369`) for the exact local tag peel, so the bounded, process-grouped
spawn lives in `builder.rs`, not here. `grep -n 'Command::new'` in this file now
returns **only** test sites (`1482`, `1500`, `1507`, `2102`, `2117`); there
is no production `Command::new` in `eggpack-cli` at all. The file likewise reads
no named environment variable: its only `std::env` uses are `args()` (`8`) and
`current_dir()` (`780`, `888`, `905`, `1138`), plus a test-only `temp_dir()`
(`1418`).

What it does: it validates that the expected revision is 40 lowercase hex
characters (`326-332`), then delegates to `eggpack_core::run_git_bounded`
(`336-337`), which owns the spawn. That helper builds a `CommandSpec` for
`git rev-parse --verify HEAD^{commit}` (`builder.rs409-420`) and, for mapped
mode, `refs/tags/<tag>^{commit}` (`builder.rs422-479`); both use the same bounded
runner with a 30 s deadline (`builder.rs20`), a 4 KiB retained-output cap
(`builder.rs22`), a `command-group` process group, and a cleared environment
(`builder.rs484-528`).

Why it is load-bearing. A generated workflow renders concrete plans, paths, and
policies. If the checkout has moved — a rebase, a tag force-update, a detached job
at the wrong commit — the workflow would build and aggregate bytes for a commit
other than the one the plan was derived from, so every downstream artifact would
describe a release the plan does not describe. Two call sites pin the identity:
`ci _verify-source` (`321-361`) requires the plan's `source_revision` to be 40
lowercase hex then verifies it against the inherited working directory (`373-389`);
`ci _resolve-release` (`401`) verifies the caller-supplied `--source-revision`
against the explicit `--source-root` before resolving anything.

Mapped reusable mode also peels the exact local tag to that revision and
resolves the manifest ID under the fixed renderer-supplied identity mode.
`_verify-source` receives the runtime draft policy and event-selected tag, then
checks the tag, mapped ID, plan revision, HEAD, and tag peel as one tuple. These
extra arguments are absent in legacy exact-tag mode.

It fails closed. Anything other than `CommandOutcome::Success` is an error
(`384-388`): spawn failure, deadline, output-cap breach, non-zero status, and
mismatch all collapse to one message that carries no git output. Only exact
equality passes, and the pass path prints a fixed sentence carrying no revision
text (`389`).

The expected object name is matched *inside* the runner
(`builder.rs850-856`) and never returned to this process, so the CLI cannot
leak stdout even by accident. The comparison is exact rather than containment:
`expected_stdout_exact` (`builder.rs388`) requires the trimmed stdout to equal
the expected value, which a substring check would not. This keeps the site
inside the workspace contract in
[process-execution.md](process-execution.md) rather than diverging from it.

## Error and exit discipline

`main` (`7-9`) exits with the `i32` from `run` (`11-19`): `0` on `Ok(())`,
otherwise one `eprintln!("eggpack: {message}")` (`15`) and `1`. There is exactly
one `eprintln!` in the file, so every failure surfaces as one prefixed line on
stderr.

Handlers return `Result<(), String>`, and library errors are almost always
collapsed to a fixed static message by `.map_err(|_| "…")` — `invalid release
plan`, `handoff projection failed`, `gate evaluation failed`. A diagnostic
therefore names a cause class rather than reproducing a library message. The one
deliberate exception is `_stage-github-draft` (`1408`), which interpolates the
inner `GithubError`; the comment at `1405-1407` records that the type carries
only static bounded text — no token, body, or URL — and that live diagnosis
requires naming it.

Diagnostics never echo captured process output. `git` stdout is compared, not
printed (`337-348`). Consumer validator output is excluded from durable evidence by
type, and the CLI additionally refuses to write evidence whose JSON contains
`stdout` or `stderr` (`1049-1052`); a failed validation reports the reason variant
by `Debug` (`1063-1065`), naming the cause (interpreter, candidate identity,
script, timeout, exit) and not the output.

The GitHub token is read by name from the environment
(`eggpack_github::read_token(&policy.token_env)`, `1378`); absence fails before
any network I/O, and the token text is never serialized, logged, or included in an
error (`1376-1377`).

Success output is a fixed set of bounded lines on stdout (`348`, `461`,
`584-588`, `604-608`, `644`, `671`, `778`, `884`, `1056`, `1132`,
`1264`, `1357`, `1413-1416`).

Limitation worth stating: there are only two exit codes. A caller cannot
distinguish drift from a malformed plan by status alone, only by message text.

## Dependencies / dependents

Dependencies (`crates/eggpack-cli/Cargo.toml:19-27`): `eggpack-contract`,
`eggpack-core`, `eggpack-ci`, `eggpack-manifest`, `eggpack-bootstrap`,
`eggpack-github`, plus `serde_json`, `toml` 0.8, and `tokio` with the `rt`
feature only. Dev-dependency: `sha2` 0.10 (`:30`). No `clap`, no HTTP client, no
`command-group`, no YAML emitter.

The crate has one integration-test file,
`crates/eggpack-cli/tests/contract_expand.rs`, added by Contract M003. It exists
because the `contract expand` stdout/stderr/exit-code contract is only observable
from outside the process: the unit tests inside `main.rs` can see
`contract_expand`'s `Result` but not what the binary actually wrote.

This is the only crate that depends on all six workspace crates, so it is the
only place where the whole pipeline can be driven in one process. That is what
`generated_orchestration_cli_executes_capture_to_aggregate` (`1765`) does: it
builds argv from `RunnerCommand::argv()` and calls the handlers directly —
capture (`1857`), qualify (`1880`), gate (`1907`), aggregate (`1924`) — then
asserts the negative cases (tampered candidate bytes at `1950`, missing
required flags at `1955` and `1971`).

To be precise about a common confusion: no workspace crate depends on
`eggpack-cli` — the CLI is a leaf, and `eggpack-ci` does *not* depend on it. What
[ci.md](ci.md) records is that `eggpack-ci`'s *own* dev-dependencies let *its*
tests drive downstream crates. The CLI's only non-Cargo coupling to `eggpack-ci`
is a fixture path read at `1531` (`../eggpack-ci/tests/fixtures/m002-direct.yml`).

Dependents: end users, and the generated workflow, whose steps are invocations of
this binary rendered by `eggpack-ci`'s `RunnerCommand`
(`crates/eggpack-ci/src/lib.rs1372`).

## Tests

Twenty-two `#[test]` functions in `mod tests` (`1410`). The twelve `ci` tests
call handler functions and `verify_source_revision` in-process rather than
spawning the binary; the ten `m003_*` tests also drive `run()` and `dispatch`
(`3741`), so the exit-code mapping is exercised for the `contract` family. The
eight tests in `crates/eggpack-cli/tests/contract_expand.rs` spawn the real
binary, which is the only way to observe its stdout contract.

| Test | Line | Pins |
|---|---|---|
| `absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative` | `1429` | Path adapter: passthrough, `.` joining, `..`/empty/relative-base rejection |
| `stage_github_draft_accepts_rendered_four_flag_call` | `1453` | Arity bound admits the renderer's eight-argument call; ten fails |
| `source_verifier_accepts_tag_commit_and_rejects_other_checkout` | `1479` | `git` identity: matching HEAD passes, other revision and non-hex fail |
| `generate_equals_library_and_check_detects_drift` | `1633` | `generate` == library renderer; CRLF matches; one byte drifts; bad input errors |
| `generated_orchestration_cli_executes_capture_to_aggregate` | `1754` | Full capture → qualify → gate → aggregate path plus negative cases and manifest placement |
| `prepare_stage_materializes_exact_payload` | `1991` | Staging payload contents from a real finalized root |
| `resolve_release_emits_distinct_runtime_identity_per_tag` | `2232` | Per-tag resolution identity; wrong revision and malformed tag fail |
| `validate_consumer_executes_typed_command_end_to_end` | `2263` | Consumer validator against real candidate bytes |
| `validate_consumer_refuses_failed_qualification_evidence` | `2427` | Non-`Passed` evidence refused before execution (`994-999`) |
| `prepare_stage_with_product_wrappers_materializes_four_installers` | `2578` | Presentation + source-root pair produces the wrapper set |
| `shape_generate_check_round_trip_and_drift` | `2712` | Reusable shape mode round trip and drift |
| `shape_generate_and_check_accept_native_qualification_for_cross_tool_targets` | `2913` | Reusable mode with native qualification across tool targets |
| `m003_argument_parser_is_exact_and_rejects_ambiguity` | `3277` | Every accepted form; duplicate flag, unknown option, positional argument, missing value, empty value, missing-required set, unsupported field |
| `m003_expands_every_field_and_resolves_aliases` | `3350` | CLI equals `DistributionContract::expand` for all four fields, through both a triple and an alias |
| `m003_release_id_is_opaque_and_accepts_prerelease_punctuation` | `3408` | Opaque expansion input including `v1.2.3-rc.1+build.5`; path separators and empty rejected |
| `m003_fails_closed_for_bundle_and_archive_direct_fields` | `3462` | `canonical-target` answers for both non-direct forms; the three direct-only fields do not |
| `m003_rejects_invalid_schema_unknown_targets_and_missing_input` | `3494` | schema_version 2, unknown key, malformed TOML, unknown triples and aliases, absent file, directory input |
| `m003_rejects_oversized_and_symlink_contract_input` | `3556` | The 1 MiB bound and the symlink refusal |
| `m003_never_mutates_the_contract_and_is_deterministic` | `3597` | Five equal runs, identical bytes, contract untouched on success and on failure |
| `m003_consumer_shaped_contract_matches_frozen_public_names` | `3630` | The live consumer contract shape against the public names both consumers publish, including the `.exe` rule; ARMv7 absent |
| `m003_dispatch_adds_a_family_without_regressing_ci` | `3741` | `run()` exit mapping for both families; `unknown ci subcommand` unchanged |
| `m003_projects_fields_without_a_serialized_expansion_model` | `3772` | No record, list, or member selector exists |
| `contract_expand::success_prints_exactly_one_scalar_and_nothing_else` | `tests/contract_expand.rs:113` | stdout is the scalar plus one newline and stderr is empty, on both direct fixtures |
| `contract_expand::repeated_invocations_are_byte_identical` | `tests/contract_expand.rs:163` | Determinism across processes |
| `contract_expand::the_command_never_writes_to_the_contract_or_its_directory` | `tests/contract_expand.rs:176` | No file created or modified, on success and on every failure shape |
| `contract_expand::every_failure_mode_exits_nonzero_with_an_empty_stdout` | `tests/contract_expand.rs:197` | Nonzero exit, empty stdout, bounded stderr across all rejection paths |
| `contract_expand::canonical_target_is_valid_for_every_asset_form` | `tests/contract_expand.rs:230` | Bundle and archive both resolve a canonical target |
| `contract_expand::argument_errors_exit_nonzero_without_a_scalar` | `tests/contract_expand.rs:250` | One defect per invocation; truncated invocations report `missing required`; `--flag=value` accepted |
| `contract_expand::symlinked_and_oversized_contracts_are_refused` | `tests/contract_expand.rs:312` | Symlink and size-bound refusals at the process boundary |
| `contract_expand::the_documented_readme_example_runs_verbatim` | `tests/contract_expand.rs:350` | The CLI README example, with its documented relative paths and working directory |

The coverage shape follows from the crate's role. Most tests assert argument
handling, arity, path helpers, and renderer parity, because that is where the
crate's own logic lives. The one substantial end-to-end path (`1754`) proves
that the handlers compose; the real proof that generated workflows run against
real binaries lives in `eggpack-ci`'s own tests and in the hosted CI lanes.
Contract M003 gave the crate an integration-test surface for the first time,
which matters more than the row count: before it, "stdout is exactly one scalar
and every failure is silent on stdout" was a README claim with nothing behind
it, the same shape as the earlier unguarded-example drift. The
`the_documented_readme_example_runs_verbatim` guard exists so that cannot recur.

Not covered here, and therefore guaranteed only by reading the code: the Windows
rename fallback in `atomic_write` (`532-545`), and `reject_symlink_output`
against a directory or symlink (`504-511`). `run()`'s exit-code mapping and
`dispatch`'s unknown-top-level-token rejection are now covered for the
`contract` family (`3741`, `tests/contract_expand.rs:250`) but still only
in-process for the `ci` family. See
[testing-and-portability.md](testing-and-portability.md) for the workspace view.

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
