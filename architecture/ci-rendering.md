# `eggpack-ci` — Workflow Rendering and Drift Checking — Deep Dive

This document covers the rendering half of `eggpack-ci`: how a projected release
plan plus a caller-supplied policy becomes deterministic GitHub Actions YAML, and
how the checked-in workflow is diffed against what the current inputs would
render. It divides from [ci.md](ci.md), which orients the crate and covers
planning and gating, and from [ci-consumer-seam.md](ci-consumer-seam.md), which
covers the external consumer validator, runtime identity, and the artifact
handoff formats.

Line references of the form `module.rs:line` are relative to
`crates/eggpack-ci/src/`.

## Responsibility

Rendering turns intent into text. Three entry points produce workflow bytes:

| Function | Location | Inputs | Output |
|---|---|---|---|
| `render_github` | `render_github.rs:143` | `&CIPlan`, `&GitHubPolicy` | `Result<String>` |
| `render_release_github` | `render_release.rs:253` | `&ReleaseCIPlanV1`, `&GitHubPolicy` | `Result<String>` |
| `render_reusable_release_github` | `render_release.rs:276` | `&DistributionContract`, `&ReleaseWorkflowShapeV1`, `&GitHubPolicy` | `Result<String>` |

Each takes references only and returns a `String`. None of the three, nor the
three `check_*` functions (`check_github` `drift.rs:63`, `check_reusable_release_github`
`drift.rs:88`, `check_release_github` `drift.rs:99`), performs filesystem I/O, spawns a
process, reads the environment, or consults a clock. Rendering is a pure function
of its arguments: the same plan and policy always produce the same bytes. The
checked-in workflow reaches the comparison as an `existing: &[u8]` supplied by
the caller (`drift.rs:66`, `drift.rs:92`, `drift.rs:102`), so the comparison cannot read the file it
is judging. The `check_*` half returns a `DriftReport` (`drift.rs:16`) and never returns
the existing bytes; nothing in this half writes.

## Policy is caller-supplied, never inferred

No action commit SHA and no runner image is baked into the workspace. The caller
supplies them, which is what makes a generated workflow reviewable: a reviewer
can see the exact `owner/repo@40-hex` the workflow will use and check it against
the upstream repository.

| Type | Location | Caller supplies | Validation enforces |
|---|---|---|---|
| `ActionPin` | `policy.rs:15` | one `reference` string | `validate_pin` `policy.rs:703`: `owner/repo@40-hex`, repo equal to the expected literal |
| `RunnerMapping` | `policy.rs:30` | `os`, `arch`, `label`, `cargo_zigbuild`, `zig` | label charset `safe_runner_label` `policy.rs:717`; ≤128 mappings `policy.rs:580`; duplicate `(os, arch)` rejected `policy.rs:650-657` |
| `WorkflowTrigger` | `policy.rs:274` | an ordered subset of `Push` / `WorkflowDispatch` | 1–2 entries `policy.rs:578-579`; duplicates rejected `policy.rs:658-667` |
| `GitHubPolicy` | `policy.rs:320` | runners, pins, triggers, timeouts, optional staging, provisioning, and release identity mode | timeout 1–360 `policy.rs:574-575`; retention 1–90 `policy.rs:576-577`; sysroot map ≤256 `policy.rs:597-598`; staging identity rejects control chars, `/`, `..` `policy.rs:622-629`; staging tag source requires its matching trigger `policy.rs:640-648`; mapped mode is reusable-staging-only and pairs `RefName` with push or `DispatchInput` with dispatch |
| `EggpackToolPolicy` | `policy.rs:377` | install timeout, and in practice only the revision | `validate` `policy.rs:385`: repo equals `https://github.com/eggstack/eggpack` `policy.rs:391`, revision exactly 40 hex `policy.rs:392-393`, package exactly `eggpack-cli` `policy.rs:394`, timeout 1–60 `policy.rs:395-396` |
| `CrossToolProvisioningV1` | `policy.rs:65` | install and download timeouts, per-arch Zig digests | `validate` `policy.rs:98` bounds both timeouts and both digests; `ZigOfficialArchiveV1::validate` `policy.rs:78` requires 64 lowercase hex, rejects the all-zero digest (`policy.rs:161` delegates to the canonical rule) |

The distinction matters. Action SHAs and runner labels are free-form but bounded;
the Eggpack install origin and package name are exact literals (`policy.rs:391`, `policy.rs:394`).
The set of permitted action *repositories* is a fixed four-entry allowlist
(`actions/checkout`, `dtolnay/rust-toolchain`, `actions/upload-artifact`,
`actions/download-artifact`) at `policy.rs:584-589`; only the commit within one of them
is caller-chosen. Tool *versions* split across the two inputs by design: Zig and
cargo-zigbuild versions come from the resolved plan (`cross_tool_versions`
`render_github.rs:43-57`), SHA-256 digests from policy (`zig_expected_digest` `policy.rs:146`),
and the download origin is fixed in code (`zig_download_url` `policy.rs:133`). No
caller-supplied URL, mirror, or redirect target is representable.

`CIPlan` is the one plan type that rejects non-canonical input rather than
re-sorting it: targets strictly ascending by triple (`planning.rs:264`), outputs strictly
ascending by `LogicalOutputSelector` (`planning.rs:288`), qualification jobs in canonical
target order (`release_plan.rs:312-324`). `ReleasePlan::to_json` in `eggpack-core` serializes
without re-sorting and relies on construction-time ordering
(`crates/eggpack-core/src/lib.rs:489-497`); `CIPlan` does not.

## The three render entry points

| | `render_github` `render_github.rs:143` | `render_release_github` `render_release.rs:253` | `render_reusable_release_github` `render_release.rs:276` |
|---|---|---|---|
| Plan input | `CIPlan` (build graph only) | `ReleaseCIPlanV1` (qualified graph) | static `ReleaseWorkflowShapeV1` |
| Jobs | `preflight`, one per target | `preflight`, builds, qualifications, consumer validation, gate, aggregate, optional `stage` | same as release, plus `resolve` |
| Extra required policy | none beyond core `GitHubPolicy` | `eggpack_tool` `render_release.rs:478`, `download_artifact` `render_release.rs:483`, `release_inputs` `render_release.rs:488`, per-target sysroot for Emulated `render_release.rs:508-518`, validator map `render_release.rs:525-535` | all of the above plus `pack_config` `render_release.rs:290-293`, `draft_template` `render_release.rs:294-297`, a staging intent `render_release.rs:282-285` |
| Identity | n/a | tag, release id, and revision checked in and verified per job | none in bytes; resolved at run time by `resolve` |
| Drift check | `check_github` `drift.rs:63` | `check_release_github` `drift.rs:99` | `check_reusable_release_github` `drift.rs:88` |

The three shapes answer different questions. The build-only renderer is the
earliest slice and emits no gate. The exact release renderer serves a workflow
already pinned to one tag and one revision, so every job verifies the
checked-out source against the checked-in plan. The reusable renderer serves a
workflow whose bytes must stay valid for every future tag: it projects the graph
from static shape against dummy identity (`render_release.rs:306-322`), rewrites the
identity-carrying input paths to workflow-private storage (`render_release.rs:327-348`), and
fails closed if an unresolved placeholder survives into the output (`render_release.rs:369-371`).
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
(`render_release.rs:571-585`). "Reusable" here means tag-independent and identity-resolved at
run time, not the GitHub reusable-workflow calling convention.

`render_github` and `check_github` have no production caller in `eggpack-cli`;
they are exercised by this crate's own tests.

## The `RunnerCommand` vocabulary

Generated steps are structured values, not shell strings. `RunnerCommand` (`runner.rs:13`)
has nine variants, each a finite internal CI operation, rendered to
`eggpack ci _<subcommand>` with named flags by `argv()` (`runner.rs:169`):
`VerifySource` (`runner.rs:15`), `CaptureBuild` (`runner.rs:26`), `QualifyTarget` (`runner.rs:41`),
`EvaluateGate` (`runner.rs:62`), `Aggregate` (`runner.rs:71`), `PrepareStage` (`runner.rs:86`),
`StageGithubDraft` (`runner.rs:107`), `ValidateConsumer` (`runner.rs:118`), and
`ResolveRelease` (`runner.rs:135`). Optional flags append at fixed positions, not at the
end: `--qemu-sysroot` last (`runner.rs:251-254`), `--installer-presentation` and
`--source-root` in order (`runner.rs:328-335`), and `--consumer-validators` spliced
immediately before `--selected` (`runner.rs:368-369`).

`to_shell()` (`runner.rs:460`) is the only path from a command to text. Every token goes
through `shell_quote` (`render_github.rs:254`), which wraps in single quotes and rewrites an
embedded quote as `'"'"'`, with one exception: the literal token `$head_sha`
renders double-quoted so the runner shell expands it (`runner.rs:464-465`). That token
is the render-only marker for a derived HEAD and can never collide with a real
value, because real revisions must be 40 hex (`runner.rs:150-151`,
`validate_source_revision` `validate.rs:51`).

The point of the enum is that the generated workflow cannot express arbitrary
shell. There is no generic command variant, so a plan cannot smuggle a string
into a `run:` block; the shell text is a rendering of a closed set. The same
vocabulary is invoked directly by the local orchestration tests
(`m002a_runner_command_round_trip_covers_all_required_args` `tests.rs:1159`), so tested
and rendered behaviour cannot diverge.

## Job and step composition

Triggers are emitted first (`render_release.rs:571-585`). `push:` and `workflow_dispatch:`
appear in the order the policy lists them. Dispatch gains a `release_tag` string
input only when staging is enabled *and* the tag source is `DispatchInput`
(`render_release.rs:574-585`); there is no branch or `latest` default. The top-level
`permissions` block is `contents: read` (`render_release.rs:586`, and `render_github.rs:152` for build-only),
and every job restates it. Concurrency keys on workflow and ref, or on the
resolved staging tag for dispatch-driven staging (`render_release.rs:601-607`).

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
| 1 | `preflight` | `emit_preflight_job` `render_release.rs:611`, job head `render_release.rs:614` | — |
| 2 | `resolve` (reusable only) | `emit_resolve_job` `render_release.rs:679`, job head `render_release.rs:688` | `preflight` |
| 3 | one build job per `CIPlan` target | `emit_build_jobs` `render_release.rs:732`, job head `render_release.rs:752` | `resolve` if reusable, else `preflight` (`render_release.rs:764-770`) |
| 4 | one qualification job per build job | `emit_qualification_jobs` `render_release.rs:862`, job head `render_release.rs:882` | that build job (`render_release.rs:892-899`) |
| 5 | one consumer job per validated target | `emit_consumer_validation_jobs` `render_release.rs:945`, job head `render_release.rs:982` | that qualification job (`render_release.rs:983-990`) |
| 6 | gate, id fixed `required_gate` | `emit_gate_job` `render_release.rs:1042`, job head `render_release.rs:1040` | inline list (`render_release.rs:1052-1072`) |
| 7 | aggregate, id fixed `aggregate` | `emit_aggregate_job` `render_release.rs:1130`, job head `render_release.rs:1131` | the gate (`render_release.rs:1141-1148`) |
| 8 | `stage`, only when the graph has staging | `emit_stage_job` `render_release.rs:1212`, job head `render_release.rs:1242` | the aggregate, plus an `if:` tag guard (`render_release.rs:1252-1265`) |

Job identifiers are not invented by the renderer. They come from the validated
plan, and `ReleaseCIPlanV1::validate` pins them to exact strings: the gate must
be `required_gate` (`release_plan.rs:177`), the aggregate `aggregate` (`release_plan.rs:178`), the stage job
`stage` (`release_plan.rs:245`), and each build job id must equal the derived
`target_job_id(target)` (`planning.rs:342`). The gate's `needs:` is the one list-valued
dependency: each qualification contributes its consumer job when a validator is
configured for that target and the qualification job otherwise (`render_release.rs:1052-1066`), so
a graph with no validators emits exactly the pre-validator dependency list.

`render_release_github_inner` (`render_release.rs:1370`) is the orchestrator, not the
implementation. It validates, then emits, and each concern has one named private
helper: `validate_release_render_policy` (`render_release.rs:417`) for the plan and policy
shape, `validate_release_identity_mode` (`render_release.rs:430`) for the identity mode,
`validate_staging_trigger_match` (`render_release.rs:451`) for the trigger cross-check,
`validate_release_tool_pins` (`render_release.rs:474`) and `validate_emulated_runtime_policy`
(`render_release.rs:497`) for the pin and emulated-runtime rules,
`resolve_consumer_validator_map` (`render_release.rs:525`) for the validator map,
`emit_release_workflow_header` (`render_release.rs:567`) for the header and permissions, one
`emit_*_job` helper per job, `emit_source_checkout_step` (`render_release.rs:538`),
`emit_runtime_identity_download_step` (`render_release.rs:547`), and
`emit_tool_install_and_verify_steps` (`render_release.rs:556`) for step emission, and
`validate_rendered_release_workflow` (`render_release.rs:1316`) for the static guards. The
helpers share one validated context, `ReleaseRenderContext` (`render_release.rs:379`), so no
emitter re-derives a policy decision.

Step order inside a build job is fixed: checkout (`render_release.rs:768`), runtime-identity
download when reusable (`render_release.rs:769`), toolchain setup (`render_release.rs:779`), toolchain
verification or cross-tool provisioning (`render_release.rs:786-795`), pinned tool install
(`render_release.rs:790`), source verification when staging is enabled (`render_release.rs:556-559`), one build
step per output (`render_release.rs:802-824`), `_capture-build` (`render_release.rs:843`), artifact upload
(`render_release.rs:845`). The install precedes the first `eggpack` invocation; source
verification follows the toolchain the install depends on. Least privilege is
structural: every job carries `permissions: contents: read` except `stage`,
which carries `contents: write` (`render_release.rs:1272`); no job receives `id-token: write`,
and the renderer fails closed if that string ever appears (`render_release.rs:1320-1322`). The
staging token travels through `env` only (`render_release.rs:1294`). Artifact handoff names are
derived, not literal: `handoff_name` (`planning.rs:354`), `BUILD_HANDOFF_FILE` (`policy.rs:542`),
`QUALIFICATION_EVIDENCE_FILE` (`policy.rs:545`), `CANDIDATES_DIR` (`policy.rs:548`),
`GATE_OUTCOME_FILE` (`policy.rs:551`), `RUNTIME_IDENTITY_ARTIFACT` (`identity.rs:29`).
`mkdir_step` (`render_release.rs:39`) **refuses** — it returns `Result` — when its
directory is not a fixed safe relative path, instead of emitting `mkdir -p
'<dir>'`. The refusal is deliberate: a contract may legally declare a triple at
the contract's own name bound, which yields a directory name longer than the
renderer's bound, and the renderer must reject that document rather than panic
on caller-supplied input.

## Determinism of the render

The YAML is hand-concatenated into a `String`. There is no YAML emitter among the
crate's dependencies — `eggpack-ci` depends on `eggpack-contract`,
`eggpack-core`, `serde`, `serde_json`, and `sha2` only
(`crates/eggpack-ci/Cargo.toml`); `serde_yaml` is a dev-dependency used by tests
to re-parse the output. Concatenation means key order is fixed by the order of
`push_str` calls and cannot drift against a serializer's own ordering, and
nothing in the output depends on hash map iteration order.

Escaping is nonetheless structural rather than hand-rolled: every scalar passes
through `yaml_scalar` (`render_github.rs:250`), which is `serde_json::to_string`. A JSON string
literal is valid YAML double-quoted style, so quoting, backslashes, and control
characters are handled by `serde_json` rather than by this crate. The cost of
hand-concatenation is that the emitted YAML is never schema-validated by the
renderer; the tests re-parse it to cover that.

Order is inherited from validated input, not chosen during rendering. Job order
is `plan.targets` order, which `CIPlan::validate` forces strictly ascending by
target triple (`planning.rs:264`); output order within a job is forced ascending by logical
selector (`planning.rs:288`); qualification order is forced canonical (`release_plan.rs:312-324`). So
identical `(plan, policy)` gives byte-identical output. The trigger and runner
arrays are emitted in the caller's listed order (`render_release.rs:571`, `render_release.rs:735-742`), so a
policy listing the same set in a different order renders differently: policy
array order is part of the determinism input, not something the renderer
normalizes. Output is size-bounded at 8 MiB (`MAX_WORKFLOW_BYTES` `drift.rs:12`),
checked after rendering (`render_github.rs:244`, `render_release.rs:1316-1318`) and again against existing bytes by
each `check_*` (`drift.rs:37-39`). See [determinism.md](determinism.md).

## Drift checking

`DriftReport` (`drift.rs:16`) carries `matches`, `expected_bytes`, `actual_bytes`, and
`first_difference` — an offset into the normalized bytes, not a diff.

There is exactly one drift comparison. `compare_workflow_bytes` (`drift.rs:33-60`)
takes the **already-rendered** expected bytes and the existing bytes, and does
the five things, in this order: reject existing input over the size bound
(`drift.rs:37-39`), normalize both sides, compare, locate the first differing
offset, and build the report. The three public check functions are one-liners
that render and then delegate: `check_github` (`drift.rs:63-70`),
`check_reusable_release_github` (`drift.rs:88-96`), `check_release_github`
(`drift.rs:99-106`).

Because the bound and the normalization are applied once, all three check
functions are guaranteed to behave identically on identical inputs. The first
difference falls back to `expected.len().min(existing.len())` when one side is a
prefix of the other (`drift.rs:51`).

Normalization is exactly one transformation, `normalize_newlines` (`drift.rs:72-85`):
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

**The Eggpack tool itself.** `tool_install_snippet` (`render_release.rs:60`) emits
`cargo install --git <repo> --rev <rev> --locked <package>` followed by
`eggpack --version`, under the policy's install timeout. The three interpolated
values are not quoted, which is safe only because `EggpackToolPolicy::validate`
(`policy.rs:390-401`) reduces them to a literal URL, a 40-hex revision, and the literal
package `eggpack-cli`. The install appears in every release job that can invoke
`eggpack`: `preflight` when staging is enabled and the render is not reusable
(`render_release.rs:662`), `resolve` (`render_release.rs:698`), and every release-producing job through the
one shared `emit_tool_install_and_verify_steps` (`render_release.rs:556-559`) — build
(`render_release.rs:799`), qualification (`render_release.rs:910`), consumer validation (`render_release.rs:1001`), gate
(`render_release.rs:1084`), aggregate (`render_release.rs:1157`), and `stage` (`render_release.rs:1285`).

Ordering here was a real defect class, not a design preference. Source
verification originally preceded the install in exact-staging preflight, and
output directories were not created before the commands that write into them;
both were corrected so the pinned tool is installed before any `eggpack`
invocation and required directories are created first. `plans/registry.md:159`
records this as the closed M003e corrective ("renderer installs tool before use
and creates output dirs") and notes that post-closure live dispatch then exposed
a rejected `-p` install flag, corrected by M003f. The current ordering is
asserted structurally by `assert_tool_before_use` (`tests.rs:5538`) and
`assert_mkdir_before_output` (`tests.rs:5581`).

**Cross-build tools.** `cross_tools_section` (`render_github.rs:132`) selects between two modes.
With no `cross_tools` policy the render is `PreinstalledVerified`:
`verified_cross_tools_step` (`render_github.rs:61`) checks both exact configured versions are
present and installs nothing. With a `cross_tools` policy it is `Provisioned`:
`provision_cross_tools_steps` (`render_github.rs:85`) emits two steps — install exact
cargo-zigbuild into an invocation-private `CARGO_INSTALL_ROOT` under
`${{ runner.temp }}` keyed by run id and attempt (`render_github.rs:96-100`), then download
the official Zig archive over HTTPS-pinned `curl` with bounded connect and total
timeouts (`render_github.rs:111-114`), verify the policy digest with `sha256sum --check`
(`render_github.rs:115`), validate the single-top-level layout, reject absolute and traversal
entries, extract to a private directory, require a regular non-symlink executable
with the exec bit, require exact version equality, and export
`CARGO_ZIGBUILD_ZIG_PATH` plus an invocation-private `CARGO_ZIGBUILD_CACHE_DIR`
(`render_github.rs:117`). Product build steps in this mode receive `provisioned_build_env()`
(`render_github.rs:126-128`, applied at `render_release.rs:809-811`) so the build binds to the verified Zig
rather than to ambient `PATH`. Provisioning is restricted to Linux x86-64 and
AArch64, enforced twice — in `GitHubPolicy::validate` (`policy.rs:686-692`) and again in
the release renderer (`render_release.rs:743-761`).

The render finishes with static guards over its own output
(`validate_rendered_release_workflow` `render_release.rs:1316-1361`): no
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
  (`run_consumer_validator` `consumer_validator.rs:323`).
- **Publish.** The only write-capable step emitted is
  `_stage-github-draft` (`render_release.rs:1245-1250`), guarded by a tag-shape `if:`
  (`render_release.rs:1260-1265`), and the publish tokens are rejected at `render_release.rs:1323-1335`.
- **Talk to GitHub.** There is no HTTP client dependency. The only network
  acquisition is *emitted as text* for a runner to perform (`render_github.rs:111-114`); the
  renderer never opens a connection.
- **Decide release content.** Job set, ordering, identifiers, artifact names,
  and tool versions are all read from the validated plan (`render_release.rs:761-763`,
  `render_release.rs:891-892`, `render_release.rs:1049-1050`, `render_release.rs:1140-1141`, `render_release.rs:1251-1252`). Layout and naming
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
`assert_golden` (`tests.rs:151`) is `assert_eq!(actual, fixture.replace("\r\n", "\n"))`
against fixtures pulled in with `include_str!`, so the comparison is against the
tracked bytes as of compile time. There are thirteen `assert_golden` call sites
(`tests.rs:176`, `tests.rs:410`, `tests.rs:1439`, `tests.rs:1464`, `tests.rs:1481`,
`tests.rs:1498`, `tests.rs:2106`, `tests.rs:2178`, `tests.rs:2195`, `tests.rs:2212`,
`tests.rs:7017`, `tests.rs:7034`, `tests.rs:7051`) and nine YAML goldens in
`crates/eggpack-ci/tests/fixtures/`
(`m002-*`, `m003b-*-staging`, `native-direct`, `native-direct-multitarget`),
alongside one contract input, `mixed-direct-targets.toml`.

Three tests are `#[ignore]`d and all three mutate tracked fixtures:
`m002a_regenerate_goldens` (`tests.rs:1508`), `m003b_regenerate_goldens` (`tests.rs:2113`), and
`m005_regenerate_m001_multitarget_golden` (`tests.rs:6389`). Each now carries a comment
above its `#[ignore]` recording that it mutates tracked fixtures, and each calls
`write_golden`
(`tests.rs:158`), which is `#[allow(dead_code)]` and writes directly under
`CARGO_MANIFEST_DIR` (`tests.rs:158-162`) with no atomic replace, no symlink check,
and no diff preview. Running `cargo test -p eggpack-ci --lib -- --ignored`
therefore rewrites the golden corpus in place. The goldens are then reviewed as
an ordinary diff, which is the intended mechanism, but the write path bypasses
the atomic-replacement discipline the CLI applies to `ci generate` and offers no
confirmation step. A reviewer should know that an ignored test in this crate is
not read-only. Non-golden behavioural coverage worth naming:
`rendering_is_deterministic_parseable_read_only_and_hands_off_candidate` (`tests.rs:164`),
`drift_check_normalizes_only_crlf_and_detects_manual_edits` (`tests.rs:225`),
`m003b_staging_disabled_renders_byte_compatible` (`tests.rs:2090`),
`m003d_reusable_check_detects_shape_drift` (`tests.rs:4833`),
`m005_release_renderer_provisions_before_build_with_private_cache` (`tests.rs:6239`).

The review burden is the honest cost of the split. The crate is a 14-file
module tree and carries 82 test attributes (80 `#[test]`, 2 `#[tokio::test]`) in
`src/tests.rs`; the rendering tests share that module with the planning,
handoff, and consumer-validator tests, so a change to step emission is reviewed
alongside unrelated harness code. See
[testing-and-portability.md](testing-and-portability.md).

## Dependencies / dependents

Dependencies (`crates/eggpack-ci/Cargo.toml`): `eggpack-contract`,
`eggpack-core`, `serde` (derive), `serde_json`, `sha2`. Dev-dependencies:
`serde_yaml` `0.9`, `eggpack-manifest`, `eggpack-bootstrap`, `eggpack-github`,
`tokio` (rt, macros). `sha2` backs artifact and archive digests (`handoff_name`
`planning.rs:354`, Zig provisioning `policy.rs:146`); the dev crates are used by tests that exercise
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
