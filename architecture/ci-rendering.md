# `eggpack-ci` — Workflow Rendering and Drift Checking — Deep Dive

This document covers the rendering half of `eggpack-ci`: how a projected release
plan plus a caller-supplied policy becomes deterministic GitHub Actions YAML, and
how the checked-in workflow is diffed against what the current inputs would
render. It divides from [ci.md](ci.md), which orients the crate and covers
planning and gating, and from [ci-consumer-seam.md](ci-consumer-seam.md), which
covers the external consumer validator, runtime identity, and the artifact
handoff formats.

## Responsibility

Rendering turns intent into text. Three entry points produce workflow bytes:

| Function | Line | Inputs | Output |
|---|---|---|---|
| `render_github` | `:1660` | `&CIPlan`, `&GitHubPolicy` | `Result<String>` |
| `render_release_github` | `:2924` | `&ReleaseCIPlanV1`, `&GitHubPolicy` | `Result<String>` |
| `render_reusable_release_github` | `:2947` | `&DistributionContract`, `&ReleaseWorkflowShapeV1`, `&GitHubPolicy` | `Result<String>` |

Each takes references only and returns a `String`. None of the three, nor the
three `check_*` functions (`check_github` `:1788`, `check_reusable_release_github`
`:3048`, `check_release_github` `:3809`), performs filesystem I/O, spawns a
process, reads the environment, or consults a clock. Rendering is a pure function
of its arguments: the same plan and policy always produce the same bytes. The
checked-in workflow reaches the comparison as an `existing: &[u8]` supplied by
the caller (`:1791`, `:3052`, `:3812`), so the comparison cannot read the file it
is judging. The `check_*` half returns a `DriftReport` and never returns the
existing bytes (`:1774`); nothing in this half writes.

## Policy is caller-supplied, never inferred

No action commit SHA and no runner image is baked into the workspace. The caller
supplies them, which is what makes a generated workflow reviewable: a reviewer
can see the exact `owner/repo@40-hex` the workflow will use and check it against
the upstream repository.

| Type | Line | Caller supplies | Validation enforces |
|---|---|---|---|
| `ActionPin` | `:453` | one `reference` string | `validate_pin` `:1527`: `owner/repo@40-hex`, repo equal to the expected literal |
| `RunnerMapping` | `:468` | `os`, `arch`, `label`, `cargo_zigbuild`, `zig` | label charset `safe_runner_label` `:1540`; ≤128 mappings `:1408`; duplicate `(os, arch)` rejected `:1474-1481` |
| `WorkflowTrigger` | `:718` | an ordered subset of `Push` / `WorkflowDispatch` | 1–2 entries `:1406-1407`; duplicates rejected `:1482-1491` |
| `GitHubPolicy` | `:728` | runners, pins, triggers, timeouts, optional staging and provisioning blocks | timeout 1–360 `:1402-1403`; retention 1–90 `:1404-1405`; sysroot map ≤256 `:1425-1426`; staging identity rejects control chars, `/`, `..` `:1446-1453`; staging tag source requires its matching trigger `:1464-1472` |
| `EggpackToolPolicy` | `:781` | install timeout, and in practice only the revision | `validate` `:794`: repo equals `https://github.com/eggstack/eggpack` `:795`, revision exactly 40 hex `:796-797`, package exactly `eggpack-cli` `:798`, timeout 1–60 `:799-800` |
| `CrossToolProvisioningV1` | `:503` | install and download timeouts, per-arch Zig digests | `validate` `:542` bounds both timeouts and both digests; `ZigOfficialArchiveV1::validate` `:516` requires 64 lowercase hex, rejects the all-zero digest `:522` |

The distinction matters. Action SHAs and runner labels are free-form but bounded;
the Eggpack install origin and package name are exact literals (`:795`, `:798`).
The set of permitted action *repositories* is a fixed four-entry allowlist
(`actions/checkout`, `dtolnay/rust-toolchain`, `actions/upload-artifact`,
`actions/download-artifact`) at `:1412-1416`; only the commit within one of them
is caller-chosen. Tool *versions* split across the two inputs by design: Zig and
cargo-zigbuild versions come from the resolved plan (`cross_tool_versions`
`:1563-1576`), SHA-256 digests from policy (`zig_expected_digest` `:590-599`),
and the download origin is fixed in code (`zig_download_url` `:577-583`). No
caller-supplied URL, mirror, or redirect target is representable.

`CIPlan` is the one plan type that rejects non-canonical input rather than
re-sorting it: targets strictly ascending by triple (`:309`), outputs strictly
ascending by `LogicalOutputSelector` (`:333`), qualification jobs in canonical
target order (`:2400-2415`). `ReleasePlan::to_json` in `eggpack-core` serializes
without re-sorting and relies on construction-time ordering
(`crates/eggpack-core/src/lib.rs:489-497`); `CIPlan` does not.

## The three render entry points

| | `render_github` `:1660` | `render_release_github` `:2924` | `render_reusable_release_github` `:2947` |
|---|---|---|---|
| Plan input | `CIPlan` (build graph only) | `ReleaseCIPlanV1` (qualified graph) | static `ReleaseWorkflowShapeV1` |
| Jobs | `preflight`, one per target | `preflight`, builds, qualifications, consumer validation, gate, aggregate, optional `stage` | same as release, plus `resolve` |
| Extra required policy | none beyond core `GitHubPolicy` | `eggpack_tool` `:3087`, `download_artifact` `:3092`, `release_inputs` `:3097`, per-target sysroot for Emulated `:3111-3120`, validator map `:3125` | all of the above plus `pack_config` `:2961`, `draft_template` `:2965`, a staging intent `:2953` |
| Identity | n/a | tag, release id, and revision checked in and verified per job | none in bytes; resolved at run time by `resolve` |
| Drift check | `check_github` `:1788` | `check_release_github` `:3809` | `check_reusable_release_github` `:3048` |

The three shapes answer different questions. The build-only renderer is the
earliest slice and emits no gate. The exact release renderer serves a workflow
already pinned to one tag and one revision, so every job verifies the
checked-out source against the checked-in plan. The reusable renderer serves a
workflow whose bytes must stay valid for every future tag: it projects the graph
from static shape against dummy identity (`:2977-2985`), rewrites the
identity-carrying input paths to workflow-private storage (`:2998-3019`), and
fails closed if an unresolved placeholder survives into the output (`:3041-3043`).

One correction worth recording: despite the name, no `workflow_call` trigger is
emitted, and `workflow_call` does not appear anywhere in the workspace sources.
The reusable renderer emits the same `WorkflowTrigger` set as the exact renderer
(`:3138-3153`). "Reusable" here means tag-independent and identity-resolved at
run time, not the GitHub reusable-workflow calling convention.

`render_github` and `check_github` have no production caller in `eggpack-cli`;
they are exercised by this crate's own tests.

## The `RunnerCommand` vocabulary

Generated steps are structured values, not shell strings. `RunnerCommand` (`:972`)
has nine variants, each a finite internal CI operation, rendered to
`eggpack ci _<subcommand>` with named flags by `argv()` (`:1120`):
`VerifySource` (`:974`), `CaptureBuild` (`:979`), `QualifyTarget` (`:994`),
`EvaluateGate` (`:1015`), `Aggregate` (`:1024`), `PrepareStage` (`:1039`),
`StageGithubDraft` (`:1060`), `ValidateConsumer` (`:1071`), and
`ResolveRelease` (`:1088`). Optional flags append at fixed positions, not at the
end: `--qemu-sysroot` last (`:1183-1186`), `--installer-presentation` and
`--source-root` in order (`:1260-1267`), and `--consumer-validators` spliced
immediately before `--selected` (`:1359-1370`).

`to_shell()` (`:1382`) is the only path from a command to text. Every token goes
through `shell_quote` (`:1770`), which wraps in single quotes and rewrites an
embedded quote as `'"'"'`, with one exception: the literal token `$head_sha`
renders double-quoted so the runner shell expands it (`:1386-1388`). That token
is the render-only marker for a derived HEAD and can never collide with a real
value, because real revisions must be 40 hex (`:1376-1381`,
`validate_source_revision` `:4575`).

The point of the enum is that the generated workflow cannot express arbitrary
shell. There is no generic command variant, so a plan cannot smuggle a string
into a `run:` block; the shell text is a rendering of a closed set. The same
vocabulary is invoked directly by the local orchestration tests
(`m002a_runner_command_round_trip_covers_all_required_args` `:5823`), so tested
and rendered behaviour cannot diverge.

## Job and step composition

Triggers are emitted first (`:3138-3153`). `push:` and `workflow_dispatch:`
appear in the order the policy lists them. Dispatch gains a `release_tag` string
input only when staging is enabled *and* the tag source is `DispatchInput`
(`:3144-3150`); there is no branch or `latest` default. The top-level
`permissions` block is `contents: read` (`:3154`, and `:1669` for build-only),
and every job restates it. Concurrency keys on workflow and ref, or on the
resolved staging tag for dispatch-driven staging (`:3165-3168`).

| Order | Job | Line | `needs:` |
|---|---|---|---|
| 1 | `preflight` | `:3175` | — |
| 2 | `resolve` (reusable only) | `:3207` | `preflight` |
| 3 | one build job per `CIPlan` target | `:3244` | `resolve` if reusable, else `preflight` (`:3276-3280`) |
| 4 | one qualification job per build job | `:3369` | that build job (`:3395-3396`) |
| 5 | one consumer job per validated target | `:3439` | that qualification job (`:3473-3474`) |
| 6 | gate, id fixed `required_gate` | `:3524` | inline list (`:3532-3547`) |
| 7 | aggregate, id fixed `aggregate` | `:3597` | the gate (`:3608-3609`) |
| 8 | `stage`, only when the graph has staging | `:3671` | the aggregate, plus an `if:` tag guard (`:3708-3721`) |

Job identifiers are not invented by the renderer. They come from the validated
plan, and `ReleaseCIPlanV1::validate` pins them to exact strings: the gate must
be `required_gate` (`:2357`), the aggregate `aggregate` (`:2358`), the stage job
`stage` (`:2425`), and each build job id must equal the derived
`target_job_id(target)` (`:310`). The gate's `needs:` is the one list-valued
dependency: each qualification contributes its consumer job when a validator is
configured for that target and the qualification job otherwise (`:3536-3546`), so
a graph with no validators emits exactly the pre-validator dependency list.

Step order inside a build job is fixed: checkout (`:3287`), runtime-identity
download when reusable (`:3288`), toolchain setup (`:3289`), toolchain
verification or cross-tool provisioning (`:3296-3305`), pinned tool install
(`:3309`), source verification when staging is enabled (`:3310`), one build step
per output (`:3313`), `_capture-build` (`:3350`), artifact upload (`:3355`). The
install precedes the first `eggpack` invocation; source verification follows the
toolchain the install depends on. Least privilege is structural: every job
carries `permissions: contents: read` except `stage`, which carries
`contents: write` (`:3724`); no job receives `id-token: write`, and the renderer
fails closed if that string ever appears (`:3770-3772`). The staging token
travels through `env` only (`:3749`). Artifact handoff names are derived, not
literal: `handoff_name` (`:432`), `BUILD_HANDOFF_FILE` (`:941`),
`QUALIFICATION_EVIDENCE_FILE` (`:943`), `CANDIDATES_DIR` (`:945`),
`GATE_OUTCOME_FILE` (`:947`), `RUNTIME_IDENTITY_ARTIFACT` (`:4438`).
`mkdir_step` (`:2842`) asserts its directory is a fixed safe relative path before
emitting `mkdir -p '<dir>'`.

## Determinism of the render

The YAML is hand-concatenated into a `String`. There is no YAML emitter among the
crate's dependencies — `eggpack-ci` depends on `eggpack-contract`,
`eggpack-core`, `serde`, `serde_json`, and `sha2` only
(`crates/eggpack-ci/Cargo.toml`); `serde_yaml` is a dev-dependency used by tests
to re-parse the output. Concatenation means key order is fixed by the order of
`push_str` calls and cannot drift against a serializer's own ordering, and
nothing in the output depends on hash map iteration order.

Escaping is nonetheless structural rather than hand-rolled: every scalar passes
through `yaml_scalar` (`:1767`), which is `serde_json::to_string`. A JSON string
literal is valid YAML double-quoted style, so quoting, backslashes, and control
characters are handled by `serde_json` rather than by this crate. The cost of
hand-concatenation is that the emitted YAML is never schema-validated by the
renderer; the tests re-parse it to cover that.

Order is inherited from validated input, not chosen during rendering. Job order
is `plan.targets` order, which `CIPlan::validate` forces strictly ascending by
target triple (`:309`); output order within a job is forced ascending by logical
selector (`:333`); qualification order is forced canonical (`:2400-2415`). So
identical `(plan, policy)` gives byte-identical output. The trigger and runner
arrays are emitted in the caller's listed order (`:3139`, `:3245-3252`), so a
policy listing the same set in a different order renders differently: policy
array order is part of the determinism input, not something the renderer
normalizes. Output is size-bounded at 8 MiB (`MAX_WORKFLOW_BYTES` `:24`),
checked after rendering (`:1761`, `:3760`) and again against existing bytes by
each `check_*` (`:1794`, `:3055`, `:3815`). See [determinism.md](determinism.md).

## Drift checking

`DriftReport` (`:1776`) carries `matches`, `expected_bytes`, `actual_bytes`, and
`first_difference` — an offset into the normalized bytes, not a diff.

Each `check_*` does the same five things: render the expected bytes, reject
existing input over the size bound, normalize both sides, compare, and locate the
first differing offset. `check_github` `:1788-1817`,
`check_reusable_release_github` `:3048-3078`, `check_release_github` `:3809-3838`.
The three bodies are near-identical copies; the first difference falls back to
`expected.len().min(existing.len())` when one side is a prefix of the other
(`:1808`, `:3069`, `:3829`).

Normalization is exactly one transformation, `normalize_newlines` (`:1818-1831`):
`\r\n` becomes `\n`, and nothing else. That is the entire tolerance. A lone
`\r`, trailing whitespace, a reordered step, a reordered trigger, or a reordered
job is drift. The strictness is the contract: a checked-in workflow that matches
is provably what the current inputs render, and anything else is a review
question rather than a silent pass. What the report gives a reviewer is a verdict
and a location, not a diagnosis — "expected N bytes, found M, first difference at
offset K" — so the reviewer must regenerate the expected render to learn what
changed.

Drift checking is what keeps a checked-in generated workflow honest. The workflow
is generated, committed, and reviewed as a diff; `eggpack ci check` re-derives it
from the same inputs and refuses a mismatch. Publication stays an explicit,
separate human action — the renderer emits no publish path and rejects publish
tokens outright. This is the recorded decision in
[principles-roadmap.md](principles-roadmap.md) ADR-0003 (`:90-93`):
checked-in deterministic workflows by default, `generate`/`check`,
drift-detected, least-privilege, no silent immutable overwrite. The read-only
boundary is enforced by ownership: `check_*` accept bytes and return a report, so
they cannot write, and `eggpack-cli` owns file access — it reads the workflow and
re-checks the bound (`crates/eggpack-cli/src/main.rs:406-409`), calls the checker
(`:414`), and prints the verdict (`:416-424`).

## Tool provisioning in the render

Two independent provisioning concerns appear in the rendered text.

**The Eggpack tool itself.** `tool_install_snippet` (`:2860`) emits
`cargo install --git <repo> --rev <rev> --locked <package>` followed by
`eggpack --version`, under the policy's install timeout. The three interpolated
values are not quoted, which is safe only because `EggpackToolPolicy::validate`
(`:794-805`) reduces them to a literal URL, a 40-hex revision, and the literal
package `eggpack-cli`. The install appears in every release job that can invoke
`eggpack`: `preflight` when staging is enabled and the render is not reusable
(`:3187-3191`), `resolve` (`:3219`), build (`:3309`), qualification (`:3407`),
consumer validation (`:3485`), gate (`:3556`), aggregate (`:3618`), and `stage`
(`:3739`).

Ordering here was a real defect class, not a design preference. Source
verification originally preceded the install in exact-staging preflight, and
output directories were not created before the commands that write into them;
both were corrected so the pinned tool is installed before any `eggpack`
invocation and required directories are created first. `plans/registry.md:115`
records this as the closed M003e corrective ("renderer installs tool before use
and creates output dirs") and notes that post-closure live dispatch then exposed
a rejected `-p` install flag, corrected by M003f. The current ordering is
asserted structurally by `assert_tool_before_use` (`:9288`) and
`assert_mkdir_before_output` (`:9331`).

**Cross-build tools.** `cross_tools_section` (`:1652`) selects between two modes.
With no `cross_tools` policy the render is `PreinstalledVerified`:
`verified_cross_tools_step` (`:1581`) checks both exact configured versions are
present and installs nothing. With a `cross_tools` policy it is `Provisioned`:
`provision_cross_tools_steps` (`:1605`) emits two steps — install exact
cargo-zigbuild into an invocation-private `CARGO_INSTALL_ROOT` under
`${{ runner.temp }}` keyed by run id and attempt (`:1616-1620`), then download
the official Zig archive over HTTPS-pinned `curl` with bounded connect and total
timeouts (`:1631-1634`), verify the policy digest with `sha256sum --check`
(`:1635`), validate the single-top-level layout, reject absolute and traversal
entries, extract to a private directory, require a regular non-symlink executable
with the exec bit, require exact version equality, and export
`CARGO_ZIGBUILD_ZIG_PATH` plus an invocation-private `CARGO_ZIGBUILD_CACHE_DIR`
(`:1635-1637`). Product build steps in this mode receive `provisioned_build_env()`
(`:1646-1648`, applied at `:3320-3322`) so the build binds to the verified Zig
rather than to ambient `PATH`. Provisioning is restricted to Linux x86-64 and
AArch64, enforced twice — in `GitHubPolicy::validate` (`:1510-1516`) and again in
the release renderer (`:3256-3265`).

The render finishes with static guards over its own output (`:3763-3804`): no
`id-token: write`; none of `gh release`, `--clobber`, `release --publish`,
`git tag `, `git push --tags`; and no `curl ` line lacking the full HTTPS-only
plus bounded-timeout plus `"$zig_url"` shape, with a closing check that the fixed
`ziglang.org/download/` origin is present whenever that curl appears. These
guards are textual rather than semantic. They cannot be defeated by a hostile
policy value without also failing the render closed, which is the safe
direction; the primary control remains that the only caller-derived strings
reaching a `run:` block are policy literals already validated or `shell_quote`d
argv.

## Boundaries / non-goals

Rendering is text production. It does not:

- **Execute the workflow.** No `std::process` appears in the render or check
  region; process execution in this crate belongs to the consumer validator
  (`run_consumer_validator` `:4151`).
- **Publish.** The only write-capable step emitted is
  `_stage-github-draft` (`:3700`), guarded by a tag-shape `if:` (`:3710-3721`),
  and the publish tokens are rejected at `:3773-3785`.
- **Talk to GitHub.** There is no HTTP client dependency. The only network
  acquisition is *emitted as text* for a runner to perform (`:1631-1634`); the
  renderer never opens a connection.
- **Decide release content.** Job set, ordering, identifiers, artifact names,
  and tool versions are all read from the validated plan (`:3085-3086`, `:3244`,
  `:3369`). Layout and naming authority is `eggpack-contract`; build,
  qualification, and finalization authority is `eggpack-core`. See
  [core.md](core.md).
- **Model installed state.** No type in the render region represents an
  installed artifact, a receipt, or an install path. This crate produces only
  producer evidence. See
  [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md).
- **Write files.** The checkers take `&[u8]`; the CLI owns all file access, and
  `eggpack ci check` is read-only by construction (see [cli.md](cli.md)).

## Tests and the golden corpus

Rendering is pinned by checked-in goldens, not by property assertions.
`assert_golden` (`:4811`) is `assert_eq!(actual, fixture.replace("\r\n", "\n"))`
against fixtures pulled in with `include_str!`, so the comparison is against the
tracked bytes as of compile time. There are ten `assert_golden` call sites
(`:4836`, `:5070`, `:6102`, `:6127`, `:6144`, `:6161`, `:6772`, `:6842`,
`:6859`, `:6876`) and nine YAML goldens in `crates/eggpack-ci/tests/fixtures/`
(`m002-*`, `m003b-*-staging`, `native-direct`, `native-direct-multitarget`),
alongside one contract input, `mixed-direct-targets.toml`.

Three tests are `#[ignore]`d and all three mutate tracked fixtures:
`m002a_regenerate_goldens` (`:6200`), `m003b_regenerate_goldens` (`:6810`), and
`m005_regenerate_m001_multitarget_golden` (`:10179`). Each now carries a comment
above its `#[ignore]` recording that it mutates tracked fixtures, and each calls
`write_golden`
(`:4818`), which is `#[allow(dead_code)]` and writes directly under
`CARGO_MANIFEST_DIR` (`:4819-4821`) with no atomic replace, no symlink check,
and no diff preview. Running `cargo test -p eggpack-ci --lib -- --ignored`
therefore rewrites the golden corpus in place. The goldens are then reviewed as
an ordinary diff, which is the intended mechanism, but the write path bypasses
the atomic-replacement discipline the CLI applies to `ci generate` and offers no
confirmation step. A reviewer should know that an ignored test in this crate is
not read-only. Non-golden behavioural coverage worth naming:
`rendering_is_deterministic_parseable_read_only_and_hands_off_candidate` (`:4824`),
`drift_check_normalizes_only_crlf_and_detects_manual_edits` (`:4885`),
`m003b_staging_disabled_renders_byte_compatible` (`:6756`),
`m003d_reusable_check_detects_shape_drift` (`:8585`),
`m005_release_renderer_provisions_before_build_with_private_cache` (`:9991`).

The review burden is the honest cost of the split. The file is 10221 lines and
carries 60 `#[test]` functions in-module; the rendering tests share that module
with the planning, handoff, and consumer-validator tests, so a change to step
emission is reviewed alongside unrelated harness code. See
[testing-and-portability.md](testing-and-portability.md).

## Dependencies / dependents

Dependencies (`crates/eggpack-ci/Cargo.toml`): `eggpack-contract`,
`eggpack-core`, `serde` (derive), `serde_json`, `sha2`. Dev-dependencies:
`serde_yaml` `0.9`, `eggpack-manifest`, `eggpack-bootstrap`, `eggpack-github`,
`tokio` (rt, macros). `sha2` backs artifact and archive digests (`handoff_name`
`:432`, Zig provisioning `:602`); the dev crates are used by tests that exercise
the crate against real serializer and staging behaviour.

Dependents: `eggpack-cli` only. `crates/eggpack-cli/src/main.rs` calls
`render_reusable_release_github` (`:327`), `render_release_github` (`:347`),
`check_reusable_release_github` (`:387`), and `check_release_github` (`:414`),
among roughly 130 `eggpack_ci::` references. `render_github` and `check_github`
are library surface without a CLI caller.

## Related deep dives

- [ci.md](ci.md) — crate orientation, projection model, job vocabulary, gates,
  aggregation. Read first.
- [ci-consumer-seam.md](ci-consumer-seam.md) — consumer validator, runtime
  identity resolution, artifact handoff formats.
- [overview.md](overview.md) — module map and producer pipeline.
- [determinism.md](determinism.md) — the workspace-wide determinism contract.
- [validation-model.md](validation-model.md) — fail-closed patterns reused by
  the policy types.
- [core.md](core.md) — build, qualification, and finalization authority.
- [cli.md](cli.md) — `ci generate` and `ci check` wiring and file ownership.
- [testing-and-portability.md](testing-and-portability.md) — testing and
  cross-platform lanes.
- [github.md](github.md) — the draft staging adapter the stage step invokes.
- [principles-roadmap.md](principles-roadmap.md) — ADRs, including ADR-0003.
