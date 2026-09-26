# `eggpack-cli` — Deep Dive

Minimal deterministic `eggpack` binary for `ci generate/check` + internal
runner + draft-staging wrappers. Single binary (`[[bin]] name = "eggpack"`),
single module `crates/eggpack-cli/src/main.rs` (`#![forbid(unsafe_code)]`,
no `clap` — manual `--flag value` / `--flag=value` parsing with strict
arity guards). No generic scripting interface.

## Command tree (`src/main.rs:21-52`)

```text
eggpack [--version|-V|--help]
eggpack ci generate | check
  | _verify-source | _resolve-release | _capture-build | _qualify-target
  | _validate-consumer | _evaluate-gate | _aggregate
  | _prepare-stage | _stage-github-draft
```

| Command | Handler | Flags |
|---|---|---|
| `ci generate` | `:301-356` | Always `--github-policy --output`. Exact: `--ci-plan`. Reusable: `--workflow-shape --contract` (mutually exclusive). |
| `ci check` | `:358-426` | Same pairing; `--workflow` existing file; drift → nonzero with first-difference report. |
| `ci _verify-source` | `:55-96` | `--release-plan`; validates 40-hex `source_revision`, runs `git rev-parse --verify HEAD^{commit}`. |
| `ci _resolve-release` | `:106-209` | `--contract --pack-config --build-bindings --qualification-bindings [--consumer-validators] --selected --tag --source-revision --template --source-root --output-plan --output-ci-plan --output-github-policy`. Verifies `HEAD == source_revision`, resolves `PackConfig::resolve(tag)`, `GitHubDraftTemplateV1::resolve(tag)`, projects CI plan(s). Workflow-private outputs only. |
| `ci _capture-build` | `:428-526` | `--release-plan --build-bindings --target` + (`--candidate-dir` \| `--cargo-target-dir`) + (`--output` \| `--output-dir`). Stages `build-handoff.json + candidates/` for `--output-dir`. |
| `ci _qualify-target` | `:528-627` | `--release-plan --build-bindings --qualification-bindings --target --candidate-dir --build-handoff --output-dir --contract [--qemu-sysroot]`. Copies validated handoff + candidates beside `evidence.json`. |
| `ci _validate-consumer` | `:670-759` | `--consumer-validators --target --candidate-dir --build-handoff --evidence --source-root --output`. Checks handoff/evidence identity + selector/package/binary match; `absolute_under_root` script guard; rejects evidence containing `stdout/stderr`. |
| `ci _evaluate-gate` | `:761-828` | `--ci-plan --output` + (`--evidence-dir` \| `--inputs-dir`). Canonical `inputs-dir/<target>/evidence.json`; fails closed on stray `consumer-evidence.json`. |
| `ci _aggregate` | `:831-955` | `--contract --release-plan --ci-plan --inputs-dir --output-root --output`. Layout `inputs-dir/<target>/{build-handoff.json, evidence.json, candidates/}` → `aggregate_finalize[_with_consumer]`. Writes `release-manifest.json` beside `summary.json` (never inside the M004 root), round-trip verified. |
| `ci _prepare-stage` | `:982-1045` | `--contract --release-manifest --finalized-root --github-policy --install-policy --output-dir --output-payload` (+ paired `--installer-presentation/--source-root`). |
| `ci _stage-github-draft` | `:1047-1099` | `--payload --github-policy --output-receipt` (+ `--staging-dir` default = payload parent). `tokio` current-thread runtime + `block_on(stage_with_dir)`; token env-only. |

`RunnerCommand::argv` parity covered by round-trip tests
(`:1472-1552`, `:1713-1726`, `:1840-1876`, `:1968-2043`).

## Wiring between crates (`Cargo.toml:19-27`)

`eggpack-contract` (parse TOML in generate/check/resolve/qualify/aggregate/
prepare), `eggpack-core` (`ReleasePlan/PackConfig/BuildBindingsV1/
QualificationBindingsV1/QualificationRuntime/qualify_target`; bindings accept
TOML-then-JSON), `eggpack-ci` (handoff, cargo path, artifact-dir validators,
`reconstruct_attempt`, project/render/check, gate/aggregate, resolve,
consumer validator, `RunnerCommand`), `eggpack-manifest` (`from_json/to_json`
round-trip in `_aggregate`), `eggpack-bootstrap` (`BootstrapInstallPolicyV1`
via `read_install_policy`), `eggpack-github` (`GitHubDraftTemplateV1/
GitHubDraftPolicyV1/StagingPayloadV1/InstallerPresentationV1`,
`prepare_staging_payload[_with_presentation]`, `read_token`,
`EggfetchTransport::new`, `stage_with_dir`). Plus `serde_json`, `toml 0.8`,
`tokio[rt]`. No GitHub client, no `clap`.

## Determinism properties

- Rendering delegates to `eggpack-ci` deterministic renderers
  (`generate == library renderer` invariant); `check` renders in memory with
  CRLF→LF normalization only.
- Bounded I/O: `read_bounded` (symlink reject, regular-file only; 1MiB default,
  64KiB validators/templates/evidence, 256KiB policies, 8MiB existing workflow).
- Safe writes: `reject_symlink_output` + `atomic_write` (parent must be a real
  dir; temp `.eggpack-tmp-<pid>` + rename; Windows remove+retry once).
- No network/discovery except: `_verify-source/_resolve-release` run `git
  rev-parse` in an explicit `source-root`; `_stage-github-draft` networks via
  `EggfetchTransport`. Everything else is file-in/file-out.
- Bounded diagnostics (`generated {} bytes`, `captured build handoff for
  {target}`, `gate outcome`, `aggregate: Complete`, …); fail-closed `String`
  errors via `run()` → `eprintln` + exit 1.

## Boundaries

Not a generic scripting/YAML DSL; `_ *` commands are internal deterministic
wrappers. No publication path: prepare/stage are draft-only with exact
draft/asset reuse; publication stays manual (ADR-0003). M004 root untouched
(`release-manifest.json` beside, not inside). Consumer evidence never carries
logs; `_validate-consumer` never rebuilds binaries. Toolchain pinned: official
Eggpack repo at exact revision via `cargo install --git --rev --locked`
(`EggpackToolPolicy`).
