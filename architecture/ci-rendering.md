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
| `render_github` | `:1727` | `&CIPlan`, `&GitHubPolicy` | `Result<String>` |
| `render_release_github` | `:3132` | `&ReleaseCIPlanV1`, `&GitHubPolicy` | `Result<String>` |
| `render_reusable_release_github` | `:3155` | `&DistributionContract`, `&ReleaseWorkflowShapeV1`, `&GitHubPolicy` | `Result<String>` |

Each takes references only and returns a `String`. None of the three, nor the
three `check_*` functions (`check_github` `:1887`, `check_reusable_release_github`
`:3255`, `check_release_github` `:4294`), performs filesystem I/O, spawns a
process, reads the environment, or consults a clock. Rendering is a pure function
of its arguments: the same plan and policy always produce the same bytes. The
checked-in workflow reaches the comparison as an `existing: &[u8]` supplied by
the caller (`:1890`, `:3259`, `:4297`), so the comparison cannot read the file it
is judging. The `check_*` half returns a `DriftReport` (`:1843`) and never returns
the existing bytes; nothing in this half writes.

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
| `GitHubPolicy` | `:728` | runners, pins, triggers, timeouts, optional staging, provisioning, and release identity mode | timeout 1–360 `:1402-1403`; retention 1–90 `:1404-1405`; sysroot map ≤256 `:1425-1426`; staging identity rejects control chars, `/`, `..` `:1446-1453`; staging tag source requires its matching trigger `:1464-1472`; mapped mode is reusable-staging-only and pairs `RefName` with push or `DispatchInput` with dispatch |
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
target order (`:2402-2417`). `ReleasePlan::to_json` in `eggpack-core` serializes
without re-sorting and relies on construction-time ordering
(`crates/eggpack-core/src/lib.rs:489-497`); `CIPlan` does not.

## The three render entry points

| | `render_github` `:1727` | `render_release_github` `:3132` | `render_reusable_release_github` `:3155` |
|---|---|---|---|
| Plan input | `CIPlan` (build graph only) | `ReleaseCIPlanV1` (qualified graph) | static `ReleaseWorkflowShapeV1` |
| Jobs | `preflight`, one per target | `preflight`, builds, qualifications, consumer validation, gate, aggregate, optional `stage` | same as release, plus `resolve` |
| Extra required policy | none beyond core `GitHubPolicy` | `eggpack_tool` `:3368`, `download_artifact` `:3373`, `release_inputs` `:3378`, per-target sysroot for Emulated `:3387-3411`, validator map `:3415-3425` | all of the above plus `pack_config` `:3170`, `draft_template` `:3174`, a staging intent `:3161-3164` |
| Identity | n/a | tag, release id, and revision checked in and verified per job | none in bytes; resolved at run time by `resolve` |
| Drift check | `check_github` `:1887` | `check_release_github` `:4294` | `check_reusable_release_github` `:3255` |

The three shapes answer different questions. The build-only renderer is the
earliest slice and emits no gate. The exact release renderer serves a workflow
already pinned to one tag and one revision, so every job verifies the
checked-out source against the checked-in plan. The reusable renderer serves a
workflow whose bytes must stay valid for every future tag: it projects the graph
from static shape against dummy identity (`:3186-3194`), rewrites the
identity-carrying input paths to workflow-private storage (`:3206-3227`), and
fails closed if an unresolved placeholder survives into the output (`:3248-3250`).
When `GitHubPolicy.release_identity_mode` is absent, generated bytes keep the
historical exact-tag behavior. The explicit `v_prefixed_stable_semver` mode is
accepted only with staging and one matching tag source; it emits a literal
resolver mode and passes the event tag through an environment variable. Each
release-producing job downloads the plan and draft policy, then verifies the
selected tag, mapped manifest ID, checked-out `HEAD`, and local tag peel before
its release command. See
[release-identity-policy.md](../docs/release-identity-policy.md) for the
checked-in policy spelling and grammar.

Staging renderers resolve the selected tag with `git ls-remote` in preflight,
before the first source checkout. Lightweight tags use their direct object;
annotated tags use the peeled object. The resolver requires one full commit OID
and exports only that OID as a job output. All subsequent checkouts use the
output, then compare `HEAD^{commit}` with it before invoking Eggpack. The stage
job resolves the tag again and rejects movement since preflight. An optional
`GitHubStagingPolicyV1.environment` renders `environment.name` on `stage` only;
GitHub Environment creation and reviewer configuration remain repository-owner
operations and are not inferred by the renderer.

One correction worth recording: despite the name, no `workflow_call` trigger is
emitted, and `workflow_call` does not appear anywhere in the workspace sources.
The reusable renderer emits the same `WorkflowTrigger` set as the exact renderer
(`:3461-3474`). "Reusable" here means tag-independent and identity-resolved at
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
`validate_source_revision` `:4577`).

The point of the enum is that the generated workflow cannot express arbitrary
shell. There is no generic command variant, so a plan cannot smuggle a string
into a `run:` block; the shell text is a rendering of a closed set. The same
vocabulary is invoked directly by the local orchestration tests
(`m002a_runner_command_round_trip_covers_all_required_args` `:5852`), so tested
and rendered behaviour cannot diverge.

## Job and step composition

Triggers are emitted first (`:3461-3474`). `push:` and `workflow_dispatch:`
appear in the order the policy lists them. Dispatch gains a `release_tag` string
input only when staging is enabled *and* the tag source is `DispatchInput`
(`:3466-3474`); there is no branch or `latest` default. The top-level
`permissions` block is `contents: read` (`:3476`, and `:1736` for build-only),
and every job restates it. Concurrency keys on workflow and ref, or on the
resolved staging tag for dispatch-driven staging (`:3483-3493`).

Dispatch tags are workflow data throughout the renderer. The reusable
`resolve` and write-scoped `stage` jobs bind `inputs.release_tag` to
`EGGPACK_RELEASE_TAG`, validate a bounded canonical stable `vX.Y.Z` form in
Bash, and do so before checkout. The resolver passes the tag to
`_resolve-release` only through the quoted `"$EGGPACK_RELEASE_TAG"` argument.
The renderer has a source-level guard that rejects either dispatch-tag
expression in any emitted `run:` script; the check permits the same value in
action inputs such as checkout `with.ref`, where it is data.
`m003j_dispatch_tag_validation_treats_values_as_data` proves a command
substitution marker executes in the old script template but not in the new
validation script.

| Order | Job | Emitter | `needs:` |
|---|---|---|---|
| 1 | `preflight` | `emit_preflight_job` `:3501`, job head `:3504` | — |
| 2 | `resolve` (reusable only) | `emit_resolve_job` `:3569`, job head `:3578` | `preflight` |
| 3 | one build job per `CIPlan` target | `emit_build_jobs` `:3622`, job head `:3652` | `resolve` if reusable, else `preflight` (`:3656-3660`) |
| 4 | one qualification job per build job | `emit_qualification_jobs` `:3752`, job head `:3781` | that build job (`:3783-3789`) |
| 5 | one consumer job per validated target | `emit_consumer_validation_jobs` `:3835`, job head `:3872` | that qualification job (`:3874-3880`) |
| 6 | gate, id fixed `required_gate` | `emit_gate_job` `:3932`, job head `:3939` | inline list (`:3945-3965`) |
| 7 | aggregate, id fixed `aggregate` | `emit_aggregate_job` `:4018`, job head `:4028` | the gate (`:4030-4036`) |
| 8 | `stage`, only when the graph has staging | `emit_stage_job` `:4100`, job head `:4139` | the aggregate, plus an `if:` tag guard (`:4141-4153`) |

Job identifiers are not invented by the renderer. They come from the validated
plan, and `ReleaseCIPlanV1::validate` pins them to exact strings: the gate must
be `required_gate` (`:2436`), the aggregate `aggregate` (`:2437`), the stage job
`stage` (`:2504`), and each build job id must equal the derived
`target_job_id(target)` (`:310`). The gate's `needs:` is the one list-valued
dependency: each qualification contributes its consumer job when a validator is
configured for that target and the qualification job otherwise (`:3945-3956`), so
a graph with no validators emits exactly the pre-validator dependency list.

`render_release_github_inner` (`:4258`) is the orchestrator, not the
implementation. It validates, then emits, and each concern has one named private
helper: `validate_release_render_policy` (`:3307`) for the plan and policy
shape, `validate_release_identity_mode` (`:3320`) for the identity mode,
`validate_staging_trigger_match` (`:3341`) for the trigger cross-check,
`validate_release_tool_pins` (`:3364`) and `validate_emulated_runtime_policy`
(`:3387`) for the pin and emulated-runtime rules,
`resolve_consumer_validator_map` (`:3415`) for the validator map,
`emit_release_workflow_header` (`:3457`) for the header and permissions, one
`emit_*_job` helper per job, `emit_source_checkout_step` (`:3428`),
`emit_runtime_identity_download_step` (`:3437`), and
`emit_tool_install_and_verify_steps` (`:3446`) for step emission, and
`validate_rendered_release_workflow` (`:4204`) for the static guards. The
helpers share one validated context, `ReleaseRenderContext` (`:3269`), so no
emitter re-derives a policy decision.

Step order inside a build job is fixed: checkout (`:3667`), runtime-identity
download when reusable (`:3668`), toolchain setup (`:3669`), toolchain
verification or cross-tool provisioning (`:3676-3685`), pinned tool install
(`:3689`), source verification when staging is enabled (`:3446-3449`), one build
step per output (`:3692-3717`), `_capture-build` (`:3731`), artifact upload
(`:3735`). The install precedes the first `eggpack` invocation; source
verification follows the toolchain the install depends on. Least privilege is
structural: every job carries `permissions: contents: read` except `stage`,
which carries `contents: write` (`:4160`); no job receives `id-token: write`,
and the renderer fails closed if that string ever appears (`:4208-4210`). The
staging token travels through `env` only (`:4182`). Artifact handoff names are
derived, not literal: `handoff_name` (`:422`), `BUILD_HANDOFF_FILE` (`:965`),
`QUALIFICATION_EVIDENCE_FILE` (`:967`), `CANDIDATES_DIR` (`:969`),
`GATE_OUTCOME_FILE` (`:971`), `RUNTIME_IDENTITY_ARTIFACT` (`:3118`).
`mkdir_step` (`:2921`) asserts its directory is a fixed safe relative path before
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
selector (`:333`); qualification order is forced canonical (`:2402-2417`). So
identical `(plan, policy)` gives byte-identical output. The trigger and runner
arrays are emitted in the caller's listed order (`:3461`, `:3626-3632`), so a
policy listing the same set in a different order renders differently: policy
array order is part of the determinism input, not something the renderer
normalizes. Output is size-bounded at 8 MiB (`MAX_WORKFLOW_BYTES` `:24`),
checked after rendering (`:1828`, `:4205`) and again against existing bytes by
each `check_*` (`:1861-1863`). See [determinism.md](determinism.md).

## Drift checking

`DriftReport` (`:1843`) carries `matches`, `expected_bytes`, `actual_bytes`, and
`first_difference` — an offset into the normalized bytes, not a diff.

There is exactly one drift comparison. `compare_workflow_bytes` (`:1860-1884`)
takes the **already-rendered** expected bytes and the existing bytes, and does
the five things, in this order: reject existing input over the size bound
(`:1861-1863`), normalize both sides, compare, locate the first differing
offset, and build the report. The three public check functions are one-liners
that render and then delegate: `check_github` (`:1887-1894`),
`check_reusable_release_github` (`:3255-3262`), `check_release_github`
(`:4294-4301`).

Because the bound and the normalization are applied once, all three check
functions are guaranteed to behave identically on identical inputs. The first
difference falls back to `expected.len().min(existing.len())` when one side is a
prefix of the other (`:1875`).

Normalization is exactly one transformation, `normalize_newlines` (`:1895-1908`):
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
re-checks the bound (`crates/eggpack-cli/src/main.rs:395-398`), calls the checker
(`:403`), and prints the verdict (`:405-413`).

## Tool provisioning in the render

Two independent provisioning concerns appear in the rendered text.

**The Eggpack tool itself.** `tool_install_snippet` (`:2939`) emits
`cargo install --git <repo> --rev <rev> --locked <package>` followed by
`eggpack --version`, under the policy's install timeout. The three interpolated
values are not quoted, which is safe only because `EggpackToolPolicy::validate`
(`:794-805`) reduces them to a literal URL, a 40-hex revision, and the literal
package `eggpack-cli`. The install appears in every release job that can invoke
`eggpack`: `preflight` when staging is enabled and the render is not reusable
(`:3550-3554`), `resolve` (`:3588`), and every release-producing job through the
one shared `emit_tool_install_and_verify_steps` (`:3446-3449`) — build
(`:3689`), qualification (`:3800`), consumer validation (`:3891`), gate
(`:3974`), aggregate (`:4045`), and `stage` (`:4173`).

Ordering here was a real defect class, not a design preference. Source
verification originally preceded the install in exact-staging preflight, and
output directories were not created before the commands that write into them;
both were corrected so the pinned tool is installed before any `eggpack`
invocation and required directories are created first. `plans/registry.md:115`
records this as the closed M003e corrective ("renderer installs tool before use
and creates output dirs") and notes that post-closure live dispatch then exposed
a rejected `-p` install flag, corrected by M003f. The current ordering is
asserted structurally by `assert_tool_before_use` (`:9317`) and
`assert_mkdir_before_output` (`:9360`).

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
(`:1646-1648`, applied at `:3322-3324`) so the build binds to the verified Zig
rather than to ambient `PATH`. Provisioning is restricted to Linux x86-64 and
AArch64, enforced twice — in `GitHubPolicy::validate` (`:1510-1516`) and again in
the release renderer (`:3633-3651`).

The render finishes with static guards over its own output
(`validate_rendered_release_workflow` `:4204-4249`): no
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
  (`run_consumer_validator` `:4616`).
- **Publish.** The only write-capable step emitted is
  `_stage-github-draft` (`:4133-4138`), guarded by a tag-shape `if:`
  (`:4148-4153`), and the publish tokens are rejected at `:4211-4223`.
- **Talk to GitHub.** There is no HTTP client dependency. The only network
  acquisition is *emitted as text* for a runner to perform (`:1631-1634`); the
  renderer never opens a connection.
- **Decide release content.** Job set, ordering, identifiers, artifact names,
  and tool versions are all read from the validated plan (`:3652-3653`,
  `:3781-3782`, `:3939-3940`, `:4028-4029`, `:4139-4140`). Layout and naming
  authority is `eggpack-contract`; build,
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
`assert_golden` (`:5210`) is `assert_eq!(actual, fixture.replace("\r\n", "\n"))`
against fixtures pulled in with `include_str!`, so the comparison is against the
tracked bytes as of compile time. There are thirteen `assert_golden` call sites
(`:5235`, `:5469`, `:6504`, `:6529`, `:6546`, `:6563`, `:7177`, `:7249`, `:7266`,
`:7283`, `:12026`, `:12043`, `:12060`) and nine YAML goldens in
`crates/eggpack-ci/tests/fixtures/`
(`m002-*`, `m003b-*-staging`, `native-direct`, `native-direct-multitarget`),
alongside one contract input, `mixed-direct-targets.toml`.

Three tests are `#[ignore]`d and all three mutate tracked fixtures:
`m002a_regenerate_goldens` (`:6573`), `m003b_regenerate_goldens` (`:7184`), and
`m005_regenerate_m001_multitarget_golden` (`:11398`). Each now carries a comment
above its `#[ignore]` recording that it mutates tracked fixtures, and each calls
`write_golden`
(`:5217`), which is `#[allow(dead_code)]` and writes directly under
`CARGO_MANIFEST_DIR` (`:5217-5219`) with no atomic replace, no symlink check,
and no diff preview. Running `cargo test -p eggpack-ci --lib -- --ignored`
therefore rewrites the golden corpus in place. The goldens are then reviewed as
an ordinary diff, which is the intended mechanism, but the write path bypasses
the atomic-replacement discipline the CLI applies to `ci generate` and offers no
confirmation step. A reviewer should know that an ignored test in this crate is
not read-only. Non-golden behavioural coverage worth naming:
`rendering_is_deterministic_parseable_read_only_and_hands_off_candidate` (`:5223`),
`drift_check_normalizes_only_crlf_and_detects_manual_edits` (`:5284`),
`m003b_staging_disabled_renders_byte_compatible` (`:7161`),
`m003d_reusable_check_detects_shape_drift` (`:9830`),
`m005_release_renderer_provisions_before_build_with_private_cache` (`:11243`).

The review burden is the honest cost of the split. The file is 12219 lines and
carries 78 `#[test]` functions in-module; the rendering tests share that module
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
