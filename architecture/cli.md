# `eggpack-cli` — Deep Dive

The deterministic wiring layer of the workspace and the only binary the
generated workflow actually invokes. `eggpack-cli` contributes exactly one
artifact — the `eggpack` binary (`crates/eggpack-cli/Cargo.toml:11-13`) from a
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
`resolve_runtime_release_plan` (`:140`, `:155-157`), gate policy via
`evaluate_gate` (`:855`, `:869`), qualification via `qualify_target` (`:585`),
finalization via `aggregate_finalize[_with_consumer]` (`:948`, `:966`).

What the crate does keep are fail-closed preconditions checked *before*
delegation, plus path adapters. Neither is a release rule:

| Local check | Line | Why it is wiring, not policy |
|---|---|---|
| Argument arity caps | `:120`, `:315`, `:338`, `:369`, `:400`, `:717`, `:1052`, `:1113` | Rejects an invocation the renderer would never emit. No release meaning. |
| Paired-flag requirements | `:436-441`, `:821-823`, `:1057-1062` | Selects between two library entry points. |
| Non-`Passed` evidence refused before a consumer script runs | `:740-745` | A guard on the validator seam; the gate also fails closed on it. |
| Handoff/evidence identity cross-checks | `:729-731`, `:746-751`, `:752-765` | Confirms the two files describe the same target, release, and revision. |
| Stray `consumer-evidence.json` rejected | `:845-854`, `:942-947` | Prevents validation being silently skipped. |
| Manifest round-trip byte comparison | `:986-993` | Proves the sidecar is the same document, not a re-serialization. |
| `absolute_path` / `absolute_under_root` / `absolutize_cli_path` | `:658`, `:634`, `:677` | Adapters between the renderer's portable relative paths and the absolute paths core requires (`:667-676`). |

The rule of thumb: if the answer would change when a library changes, the check
belongs in the library. The adapters are the only code here whose reason for
existing is a mismatch between two other crates, and each says so at its call
site (`:533-534`, `:891-892`).

## Argument parsing, hand-rolled

`dispatch` (`:21`) is the whole parser. It builds one usage string, reused
verbatim in four places (`:23`, `:30`, `:34`, `:37`):

```text
eggpack ci <generate|check|_resolve-release|_verify-source|_capture-build|_qualify-target|_validate-consumer|_evaluate-gate|_aggregate|_prepare-stage|_stage-github-draft> [options]
```

Order of decisions: empty argv is a usage error (`:22-24`); `--version`/`-V`
prints `eggpack <CARGO_PKG_VERSION>` and exits 0 (`:25-28`); `--help`/`-h`
prints the usage string and exits 0 (`:29-32`); both are handled *before* the
`ci` check, so they work only as the first token. Any first token other than
`ci` is a usage error (`:33-35`), a missing subcommand is a usage error
(`:36-38`), and an unrecognised `ci` subcommand yields `unknown ci subcommand`
(`:51`). The eleven subcommands dispatch at `:39-50`.

`get_flag` (`:211`) accepts exactly two forms: `--name value` (`:215-220`) and
`--name=value` (`:221-223`). Its two error messages are `missing value for
--name` (`:219`) and `missing required --name` (`:225`). `get_flag_optional`
(`:228`) is the same scan returning `Option`. Both scan left to right and return
the first match, so a repeated flag silently takes the first occurrence.

Unknown flags are **not** rejected. No function scans for unrecognised
arguments; a typo is caught only if it pushes the argument count past a cap:

| Command | Cap | Line |
|---|---|---|
| `ci _resolve-release` | 28 | `:120` |
| `ci generate` (reusable / exact) | 8 / 6 | `:315` / `:338` |
| `ci check` (reusable / exact) | 8 / 6 | `:369` / `:400` |
| `ci _validate-consumer` | 14 | `:717` |
| `ci _prepare-stage` | 18 | `:1052` |
| `ci _stage-github-draft` | 8 | `:1113` |
| `ci _capture-build`, `ci _evaluate-gate` | none | paired-flag checks instead (`:436-441`, `:821-823`) |

`#![forbid(unsafe_code)]` (`:1`) holds workspace-wide. `#![deny(missing_docs)]`
(`:2`) has no target: the crate exposes no public items, so in a binary it is
inert. The three `///` blocks (`:98-104`, `:667-676`, `:701-707`) document
handlers for internal readers, not for rustdoc.

The trade-off is deliberate. No `clap` means no dependency, no derive expansion,
and a parse whose behaviour is fully determined by the code above. The cost is
hand-maintained parsing, one flat usage string as the only help, no completions,
and a typo diagnosed as `too many arguments for ci ...` rather than by name.

## `ci generate` and `ci check`

Both require `--github-policy` and take one of two mutually exclusive input
modes; supplying both is an error (`:309-311`, `:363-365`).

| | Reusable mode | Exact mode |
|---|---|---|
| `ci generate` (`:301`) | `--workflow-shape` + `--contract` → `render_reusable_release_github` (`:327`) | `--ci-plan` → `render_release_github` (`:347`) |
| `ci check` (`:358`) | same inputs + `--workflow` → `check_reusable_release_github` (`:387`) | same + `--workflow` → `check_release_github` (`:414`) |

`ci generate` writes exactly one file: `--output`, through `atomic_write`
(`:329`, `:349`). No directory creation, no second write, so a generate run
cannot touch anything the caller did not name. The output path is checked by
`reject_symlink_output` and its parent must already be a real directory
(`:276-280`). Inputs are read at 1 MiB each (`:318-320`, `:341-342`); success
prints `generated <n> bytes to <path>` (`:330-334`).

`ci check` never writes. There is no `atomic_write`, `fs::write`, or
`create_dir_all` anywhere in `:351-419`; the only effect on success is one
`println!`. It reads the existing workflow through `read_bounded` at 8 MiB
(`:368`, `:398`), so the bound is checked against file metadata *before* any
allocation and the input is also refused if it is a symlink. This is the same
gate every other input in the crate uses; the workflow is not a special case.

Drift comparison is delegated. The CRLF→LF-only normalization lives in
`eggpack-ci` (`normalize_newlines`, `crates/eggpack-ci/src/lib.rs:1818`, applied
at `lib.rs:3818-3819`), not here. The CLI branches on `report.matches` and, on
drift, returns `ci check: drift detected (expected N bytes, found N bytes, first
difference at ...)` (`:393-397`, `:420-424`).

Read-only is what makes `ci check` safe in a pre-merge or scheduled job: a
failing check cannot have mutated the tree it inspects, so a red run's only side
effect is a nonzero exit. The unit test pins both halves — a CRLF-converted
workflow still matches, one appended byte fails (`:1424-1445`).

## The nine internal `ci _*` runner commands

These exist so a generated workflow step is a file-in/file-out call with a fixed
shape. Each row can be traced end to end from a rendered step.

| Command | Handler | Reads | Writes |
|---|---|---|---|
| `_verify-source` | `:55` | `--release-plan` (`:57`) | nothing; prints the pass line (`:94`) |
| `_resolve-release` | `:106` | `--contract`, `--pack-config`, `--build-bindings`, `--qualification-bindings`, optional `--consumer-validators`, `--template` (`:127-180`) | `--output-plan`, `--output-ci-plan`, `--output-github-policy` (`:204-206`) |
| `_capture-build` | `:428` | `--release-plan`, `--build-bindings`, plus `--cargo-target-dir` **or** `--candidate-dir` (`:442-475`) | `--output` (`:522`) and/or `--output-dir`: `build-handoff.json` (`:514`) + `candidates/<relative_path>` (`:505-510`) |
| `_qualify-target` | `:528` | `--release-plan`, `--build-bindings`, `--qualification-bindings`, `--build-handoff`, `--contract`, optional `--qemu-sysroot` (`:542-571`) | `--output-dir`: `evidence.json` (`:599`), `build-handoff.json` (`:604-607`), `candidates/` (`:618-627`) |
| `_validate-consumer` | `:709` | `--consumer-validators`, `--build-handoff`, `--evidence`, `--source-root` (`:720-775`) | `--output` (consumer evidence, `:799`) |
| `_evaluate-gate` | `:816` | `--ci-plan`; per target `<inputs-dir>/<target>/evidence.json` (`:831`) or `<evidence-dir>/<handoff name>.json` (`:835`), plus `consumer-evidence.json` when validators exist (`:863`) | `--output` (gate outcome JSON, `:874`) |
| `_aggregate` | `:886` | `--contract`, `--release-plan`, `--ci-plan`, per target `<inputs-dir>/<target>/{build-handoff.json,evidence.json,candidates/}` (`:912-937`), plus consumer evidence (`:960`) | `--output-root` finalized tree (via the library), `--output` summary (`:1007`), `release-manifest.json` beside it (`:996-997`) |
| `_prepare-stage` | `:1042` | `--contract`, `--release-manifest`, `--finalized-root`, `--github-policy`, `--install-policy`, optional `--installer-presentation` + `--source-root` (`:1063-1076`) | staging output dir via the library; `--output-payload` (`:1102`) |
| `_stage-github-draft` | `:1107` | `--payload`, `--github-policy`, optional `--staging-dir`; token from the environment (`:1116-1124`) | `--output-receipt` (`:1158`); the draft itself, in `eggpack-github` |

They are wrappers, not a scripting interface. The input grammar is: paths, a
target name that must already exist in the plan (`:561-565`, `:925-929`), and
nothing else. There is no shell invocation, no caller-supplied program name, no
expression evaluation, and no flag whose value is interpreted as a command. The
single external program in the crate is the literal `"git"` with fixed arguments
(`:72-73`). The one validator process is spawned inside `eggpack-ci` under a
finite interpreter enum, not from a caller string.

Invocations are rendered, not typed: `RunnerCommand::argv()` in
`eggpack-ci` (`crates/eggpack-ci/src/lib.rs:1118`) is what the tests feed
straight into these handlers (argv[0] assertions at `lib.rs:5846`, `6722`, `6741`,
`8299`).

## The file-safety helpers

`read_bounded` (`:242`) guards every input: `symlink_metadata`, refuse a symlink
(`:245-247`), refuse anything not a regular file (`:248-250`), refuse a length
above the caller's bound (`:251-253`), then read (`:254`). Failures are reported
with the caller's `label`, so diagnostics name the input class rather than echo a
path. Production bounds:

| Bound | Inputs | Lines |
|---|---|---|
| 1 MiB | release plan, contract, pack config, build/qualification bindings, ci plan, github policy (generate/check), workflow shape, build handoff, evidence | `:57`, `:127-130`, `:161-167`, `:318-320`, `:341-342`, `:372-374`, `:403-404`, `:442-443`, `:542-545`, `:726-732`, `:824`, `:898-900`, `:914-919` |
| 64 KiB | draft template, consumer validators, consumer evidence, installer presentation | `:148`, `:180`, `:720`, `:864`, `:961`, `:1030` |
| 256 KiB | install policy, github draft policy | `:1018`, `:1065`, `:1117` |
| 1 MiB | release manifest, staging payload | `:1064`, `:1116` |
| 8 MiB | existing workflow, via `read_bounded` (checked *before* the read, symlink refused) | `:368`, `:398` |

`reject_symlink_output` (`:257`) refuses an existing symlink or directory at the
output path. It is reached only from `atomic_write` (`:270`), so callers cannot
forget it; a non-existent path passes, which is the create case.

`atomic_write` (`:269`) writes a temporary sibling `.eggpack-tmp-<pid>` (`:281`),
removes a stale one from a prior aborted run (`:283`), writes (`:284`), and
renames over the destination (`:285`). Where rename does not replace, it removes
and retries once (`:287-297`).

What "atomic" buys: a concurrent reader of `--output` sees either the previous
file or the complete new one, never a truncated document. All 15 production
`atomic_write` call sites carry JSON control documents (`:204-206`, `:329`,
`:349`, `:514`, `:522`, `:599`, `:604`, `:799`, `:874`, `:997`, `:1007`,
`:1102`, `:1158`).

What it does **not** protect against, stated plainly:

- No `fsync` of the temp file or directory (`:284-285`), so a crash or power loss
  can leave a truncated or missing destination.
- The Windows fallback is explicitly not atomic: between the failed rename
  (`:285`) and the `remove_file` (`:291`) the path holds no file.
- No `O_NOFOLLOW` / `linkat` re-check, and `#![forbid(unsafe_code)]` (`:1`) rules
  out adding one, so the check at `:257` and the rename at `:285` are separated
  by a TOCTOU window.
- It protects exactly the path given. The parent must already be a real directory
  (`:276-280`), so commands staging a tree create it themselves: `create_dir_all`
  at `:498`, `:596`, `:619`.
- Candidate binaries bypass it entirely: `_capture-build` and `_qualify-target`
  hard-link with a `copy` fallback (`:507-509`, `:624-626`). Only the JSON
  descriptors are atomic; artifacts are staged before the descriptor naming them,
  and the directory validators (`:515-516`, `:628-629`) run afterwards.

## Path absolutization

Three helpers, three different contracts.

`absolute_path` (`:658`) returns an absolute input unchanged and otherwise joins
it to the current directory. No validation of any kind. Used for the consumer
candidate path (`:774`) and the validator work directory (`:776`).

`absolute_under_root` (`:634`) requires the root to be a real directory
(`:635-639`), then splits the relative validator script on `/` and rejects empty,
`.`, and `..` segments (`:649-654`). This *is* a traversal rejection, scoped to
the consumer validator script path. The comment at `:640-641` states the path was
already validated upstream; the segment loop is the second line of defence.

`absolutize_cli_path` (`:677`) rejects an empty path (`:678-680`), passes
absolute inputs through (`:681-683`), requires an absolute base (`:684-686`),
drops `.` components (`:690`), rejects `..` with `path must not contain
parent-directory components` (`:691-693`), and rejects anything else with `path
has an unsupported component` (`:695`).

It enforces **component-level escape rejection, not containment.** No
canonicalisation, no symlink resolution, no check that the result resolves under
the base — a symlinked component inside the base still points outside it, and
existence is not tested. It is used for `--candidate-dir` in `_qualify-target`
(`:535-538`) and `--output-root` in `_aggregate` (`:893-896`); commands needing a
real directory check it separately (`:460-473`, `:574-579`).

These helpers exist because core's `inspect_candidate` and `finalize_release`
require absolute paths while the renderer deliberately emits portable relative
ones, so a checked-in workflow is location-independent (doc comment `:667-676`,
call sites `:533-534`, `:891-892`). The explicit base parameter keeps unit tests
off the process working directory. `absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative`
(`:1186`) asserts absolute passthrough, `./`-relative joining, and rejection of
`../escape`, `a/../../escape`, the empty path, and a relative base.

## Where `release-manifest.json` is written

The aggregate path writes the sidecar at `:994-999`, inside
`if let Some(finalized)` (`:981`):

- It is derived from `--output`'s **parent**, not from `--output-root`
  (`:994-996`). The manifest therefore follows the summary file; if a caller
  places `--output` outside the finalized root's parent, the sidecar lands there.
- It is written only when finalization returned a finalized root. An incomplete
  aggregate produces no manifest.
- It is skipped entirely when `--output` has no parent component (`:995`), so a
  bare filename writes the summary and no sidecar.
- Before writing, the manifest is encoded, decoded through
  `eggpack_manifest::ReleaseManifest::from_json`, and re-encoded; a mismatch is
  `staging manifest does not decode to exact manifest` (`:986-993`).

The rule, precisely: **the manifest is written beside the finalized output root,
never inside it.** The test pins both halves — `root/release-manifest.json`
decodes to the exact manifest, and `!output_root.join("release-manifest.json")
.exists()` guards M004 root semantics (`:1677-1688`).

`eggpack-github` owns the same filename independently:
`FIXED_MANIFEST_NAME = "release-manifest.json"`
(`crates/eggpack-github/src/lib.rs:31`), written into its own staging output
directory with the same round-trip check (`lib.rs:777-790`, `lib.rs:1446-1459`).
So there are two writers of the same name, into two different directories: beside
the finalized root (this crate) and inside the staging payload directory
(`eggpack-github`). Neither writes it into the finalized root. The finalizer's
own containment rule is in [core-finalization.md](core-finalization.md).

## Policy and presentation loading

`read_install_policy` (`:1017`) reads at most 256 KiB with the label `install
policy` and tries `BootstrapInstallPolicyV1::from_toml` first, then
`from_json` (`:1020-1024`).

`read_installer_presentation` (`:1027`) reads at most 64 KiB and tries
`toml::from_str` first, calling `.validate()` explicitly on that branch
(`:1032-1037`), then `InstallerPresentationV1::from_json` (`:1038-1039`). The
explicit `validate()` call exists only on the TOML path; I did not verify that
`from_json` performs the same checks internally.

Both are consumed only by `_prepare-stage` (`:1072`, `:1075`).
`--installer-presentation` and `--source-root` are paired: one without the other
is an error (`:1057-1062`). The pair selects
`prepare_staging_payload_with_presentation` (`:1076`) for product wrappers;
`GeneratedDefault` passes neither and calls `prepare_staging_payload` (`:1088`).
Other bounds on that path: release manifest 1 MiB (`:1064`), github draft policy
256 KiB (`:1065`).

They are inputs, not derivations. Installer policy and presentation are decisions
owned by the product repository and validated by `eggpack-bootstrap` and
`eggpack-github`; re-deriving either here would create a second source of truth
for installer content, so the CLI only reads and bounds them.

## Process execution: the `git` site

`verify_source_revision` (`:71`) is the only production process spawn in this
crate — and it no longer constructs a `Command` itself. It delegates to
`eggpack_core::run_git_bounded` (`:82-83`), so the bounded, process-grouped
spawn lives in `builder.rs`, not here. `grep -n 'Command::new'` in this file now
returns **only** test sites (`:1228`, `:1246`, `:1253`, `:1848`, `:1863`); there
is no production `Command::new` in `eggpack-cli` at all. The file likewise reads
no named environment variable: its only `std::env` uses are `args()` (`:8`) and
`current_dir()` (`:526`, `:634`, `:651`, `:884`), plus a test-only `temp_dir()`
(`:1164`).

What it does: it validates that the expected revision is 40 lowercase hex
characters (`:72-78`), then delegates to `eggpack_core::run_git_bounded`
(`:82-83`), which owns the spawn. That helper builds a `CommandSpec` for
`git rev-parse --verify HEAD^{commit}` (`crates/eggpack-core/src/builder.rs:396`,
`400-402`) and runs it through the same `run_bounded_inner` used for builds: a
30 s deadline (`builder.rs:20`), a 4 KiB retained-output cap (`builder.rs:22`),
a `command-group` process group killed and waited on, and `env_clear()` with a
7-var allowlist (`builder.rs:473-485`).

Why it is load-bearing. A generated workflow renders concrete plans, paths, and
policies. If the checkout has moved — a rebase, a tag force-update, a detached job
at the wrong commit — the workflow would build and aggregate bytes for a commit
other than the one the plan was derived from, so every downstream artifact would
describe a release the plan does not describe. Two call sites pin the identity:
`ci _verify-source` (`:55-69`) requires the plan's `source_revision` to be 40
lowercase hex then verifies it against the inherited working directory (`:68`);
`ci _resolve-release` (`:99`) verifies the caller-supplied `--source-revision`
against the explicit `--source-root` before resolving anything.

It fails closed. Anything other than `CommandOutcome::Success` is an error
(`:84-86`): spawn failure, deadline, output-cap breach, non-zero status, and
mismatch all collapse to one message that carries no git output. Only exact
equality passes, and the pass path prints a fixed sentence carrying no revision
text (`:87`).

The expected object name is matched *inside* the runner
(`builder.rs:596-602`) and never returned to this process, so the CLI cannot
leak stdout even by accident. The comparison is exact rather than containment:
`expected_stdout_exact` (`builder.rs:134`) requires the trimmed stdout to equal
the expected value, which a substring check would not. This keeps the site
inside the workspace contract in
[process-execution.md](process-execution.md) rather than diverging from it.

## Error and exit discipline

`main` (`:7-9`) exits with the `i32` from `run` (`:11-19`): `0` on `Ok(())`,
otherwise one `eprintln!("eggpack: {message}")` (`:15`) and `1`. There is exactly
one `eprintln!` in the file, so every failure surfaces as one prefixed line on
stderr.

Handlers return `Result<(), String>`, and library errors are almost always
collapsed to a fixed static message by `.map_err(|_| "…")` — `invalid release
plan`, `handoff projection failed`, `gate evaluation failed`. A diagnostic
therefore names a cause class rather than reproducing a library message. The one
deliberate exception is `_stage-github-draft` (`:1154`), which interpolates the
inner `GithubError`; the comment at `:1151-1153` records that the type carries
only static bounded text — no token, body, or URL — and that live diagnosis
requires naming it.

Diagnostics never echo captured process output. `git` stdout is compared, not
printed (`:83-94`). Consumer validator output is excluded from durable evidence by
type, and the CLI additionally refuses to write evidence whose JSON contains
`stdout` or `stderr` (`:795-798`); a failed validation reports the reason variant
by `Debug` (`:809-811`), naming the cause (interpreter, candidate identity,
script, timeout, exit) and not the output.

The GitHub token is read by name from the environment
(`eggpack_github::read_token(&policy.token_env)`, `:1124`); absence fails before
any network I/O, and the token text is never serialized, logged, or included in an
error (`:1122-1123`).

Success output is a fixed set of bounded lines on stdout (`:94`, `:207`,
`:330-334`, `:350-354`, `:390`, `:417`, `:524`, `:630`, `:802`, `:878`,
`:1010`, `:1103`, `:1159-1162`).

Limitation worth stating: there are only two exit codes. A caller cannot
distinguish drift from a malformed plan by status alone, only by message text.

## Dependencies / dependents

Dependencies (`crates/eggpack-cli/Cargo.toml:19-27`): `eggpack-contract`,
`eggpack-core`, `eggpack-ci`, `eggpack-manifest`, `eggpack-bootstrap`,
`eggpack-github`, plus `serde_json`, `toml` 0.8, and `tokio` with the `rt`
feature only. Dev-dependency: `sha2` 0.10 (`:30`). No `clap`, no HTTP client, no
`command-group`, no YAML emitter.

This is the only crate that depends on all six workspace crates, so it is the
only place where the whole pipeline can be driven in one process. That is what
`generated_orchestration_cli_executes_capture_to_aggregate` (`:1511`) does: it
builds argv from `RunnerCommand::argv()` and calls the handlers directly —
capture (`:1603`), qualify (`:1626`), gate (`:1653`), aggregate (`:1670`) — then
asserts the negative cases (tampered candidate bytes at `:1696`, missing
required flags at `:1701` and `:1717`).

To be precise about a common confusion: no workspace crate depends on
`eggpack-cli` — the CLI is a leaf, and `eggpack-ci` does *not* depend on it. What
[ci.md](ci.md) records is that `eggpack-ci`'s *own* dev-dependencies let *its*
tests drive downstream crates. The CLI's only non-Cargo coupling to `eggpack-ci`
is a fixture path read at `:1277` (`../eggpack-ci/tests/fixtures/m002-direct.yml`).

Dependents: end users, and the generated workflow, whose steps are invocations of
this binary rendered by `eggpack-ci`'s `RunnerCommand`
(`crates/eggpack-ci/src/lib.rs:1118`).

## Tests

Twelve `#[test]` functions in `mod tests` (`:1167`). They call handler
functions and `verify_source_revision` in-process rather than spawning the
binary, so `run()` and its exit-code mapping are not exercised anywhere.

| Test | Line | Pins |
|---|---|---|
| `absolutize_cli_path_keeps_absolute_rejects_escape_and_joins_relative` | `:1186` | Path adapter: passthrough, `.` joining, `..`/empty/relative-base rejection |
| `stage_github_draft_accepts_rendered_four_flag_call` | `:1210` | Arity bound admits the renderer's eight-argument call; ten fails |
| `source_verifier_accepts_tag_commit_and_rejects_other_checkout` | `:1236` | `git` identity: matching HEAD passes, other revision and non-hex fail |
| `generate_equals_library_and_check_detects_drift` | `:1390` | `generate` == library renderer; CRLF matches; one byte drifts; bad input errors |
| `generated_orchestration_cli_executes_capture_to_aggregate` | `:1511` | Full capture → qualify → gate → aggregate path plus negative cases and manifest placement |
| `prepare_stage_materializes_exact_payload` | `:1748` | Staging payload contents from a real finalized root |
| `resolve_release_emits_distinct_runtime_identity_per_tag` | `:1989` | Per-tag resolution identity; wrong revision and malformed tag fail |
| `validate_consumer_executes_typed_command_end_to_end` | `:2020` | Consumer validator against real candidate bytes |
| `validate_consumer_refuses_failed_qualification_evidence` | `:2184` | Non-`Passed` evidence refused before execution (`:740-745`) |
| `prepare_stage_with_product_wrappers_materializes_four_installers` | `:2335` | Presentation + source-root pair produces the wrapper set |
| `shape_generate_check_round_trip_and_drift` | `:2469` | Reusable shape mode round trip and drift |
| `shape_generate_and_check_accept_native_qualification_for_cross_tool_targets` | `:2670` | Reusable mode with native qualification across tool targets |

The coverage shape follows from the crate's role. Most tests assert argument
handling, arity, path helpers, and renderer parity, because that is where the
crate's own logic lives. The one substantial end-to-end path (`:1511`) proves
that the handlers compose; the real proof that generated workflows run against
real binaries lives in `eggpack-ci`'s own tests and in the hosted CI lanes.

Not covered here, and therefore guaranteed only by reading the code: `run()`'s
exit-code and single-line-stderr mapping, `dispatch` rejecting an unknown
top-level token, the Windows rename fallback in `atomic_write` (`:287-297`),
`reject_symlink_output` against a directory or symlink (`:262-264`), and
`read_bounded`'s size bound. See
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
