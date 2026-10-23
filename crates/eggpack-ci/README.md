# eggpack-ci

Provider-neutral projection of a resolved `ReleasePlan` plus `BuildBindingsV1` into a bounded `CIPlan`, with deterministic checked-in GitHub Actions rendering and drift checking. Builder arguments are derived through `eggpack_core::cargo_command`; this crate does not infer package/bin identities, execute builds, perform qualification, create final manifests, stage releases, publish, access GitHub, or accept arbitrary YAML/step input.

The renderer requires caller-supplied runner labels and full immutable action commit pins. Runner policy declares whether cross-build images already contain cargo-zigbuild and Zig; M001 does not auto-install either tool. Generated build jobs use read-only permissions and hand off private candidate binaries as internal workflow artifacts. Qualification remains unresolved intent in M001 and no M001 workflow claims release completion.

M002 layers an executable `ReleaseCIPlanV1` on the closed M001 graph: per-target qualification jobs projected from M003 bindings, a required-evidence gate reading structured evidence (never logs), and an aggregate/finalize node invoking M004. Each qualification job runs on the runner mapped to that target's effective qualification host (`qualification_host`, else the build host), which is independent of `BuildStrategy`: a `CargoZigbuild` candidate for a compatibility floor is transferred unchanged to a matching native qualification host and qualified there, and a host that does not match the target's OS/arch is rejected at graph validation. Build handoffs carry identity-bound relative paths without absolute runner paths; qualification evidence is transported as deterministic JSON with candidate bytes revalidated after download. Required targets must pass; missing or corrupt evidence fails closed; non-gating failures suppress the finalized release instead of producing a partial one. Generated release workflows stay read-only, pin the Eggpack runtime tool to the official repository at an exact revision installed with `--locked`, and upload the completed finalized release as an internal workflow artifact only. No GitHub Release, publication, signing, or staging is performed.

M002a makes the generated workflow executable end-to-end. Every generated CLI invocation carries all required explicit file inputs from `GitHubReleaseInputsV1` (contract, release plan, build/qualification bindings, CI plan as repository-relative policy paths; no discovery or globbing). Build jobs install the pinned tool, invoke `_capture-build` with the known Cargo target root, and upload one canonical per-target directory (`build-handoff.json` plus `candidates/<relative-path>`). Qualification jobs download the matching build directory, invoke `_qualify-target` with explicit contract/plan/bindings/target/candidate/handoff/output arguments, and upload the complete directory (`build-handoff.json`, `evidence.json`, `candidates/...`). Gate and aggregate jobs download every qualification directory into canonical `eggpack-inputs/<target>/` trees and pass `--inputs-dir` plus explicit plan/output paths. Emulated qualification requires an explicit per-target provider sysroot policy or generation is rejected. The finite `RunnerCommand` model is shared by GitHub rendering and local executable orchestration tests, so CLI signature drift is caught by round-trip tests rather than YAML inspection alone.

M003b adds an optional provider-neutral staging intent (`StagingJob` with
`StagingProvider::GitHubDraft`, attached via `with_github_draft_staging`) and a
`GitHubStagingPolicyV1` provider policy (staging runner, exact owner/repository,
explicit tag source, repository-relative contract/install/GitHub-policy paths,
receipt retention). When staging is enabled, `render_release_github` emits
exactly one `stage` job (`needs: aggregate`, `contents: write`, tag-or-dispatch
guard, exact-tag checkout, pinned tool install, exact aggregate download,
`_prepare-stage` before `_stage-github-draft` with `GITHUB_TOKEN` in environment
only, staging-receipt upload). Concurrency stays release-scoped and becomes
tag-aware for manual dispatch; M003a remote reconciliation remains
authoritative. Static guards reject `id-token: write`, `gh release`,
`--clobber`, publish commands, tag mutation, and raw `curl`. Graphs without
staging serialize exactly as M002a. `ci check` detects any staging
step/permission drift.

`render_github` and `render_release_github` are pure. `check_github` and `check_release_github` compare deterministic bytes after CRLF-to-LF normalization and report drift without changing files.

`check_github`, `check_release_github`, and `check_reusable_release_github` share one
comparison: each renders its own artifact and then delegates to the same private
comparison, so the size bound, the CRLF-to-LF-only normalization, and the
first-difference offset are defined once and every check function behaves
identically on identical inputs. The release renderer is likewise one
implementation: `render_release_github_inner` validates, then delegates to named
private helpers for policy and shape validation, release-identity resolution,
trigger and input cross-checks, tool-pin and emulated-runtime validation, the
workflow header and permissions, each job, each step, and the static output
guards. None of this changes a rendered byte.

Staging-enabled workflows resolve one release source for every job. `RefName`
uses the GitHub ref and authorizes staging only for tag pushes. `DispatchInput`
requires `workflow_dispatch.inputs.release_tag`, uses it for all release
checkouts, and authorizes staging only for dispatches. Each job verifies
`HEAD^{commit}` against the checked-in `ReleasePlan.source_revision` before
release work; `ci check` compares the complete deterministic workflow.

For staging workflows, the first job validates the selected tag and resolves it
to a full commit OID with `git ls-remote` before checking out source. Annotated
tags are peeled to their commit. Every later checkout uses that frozen OID and
checks `HEAD^{commit}` before invoking Eggpack. Dispatch tags are bounded data
in `EGGPACK_RELEASE_TAG`, never executable workflow text. An optional
`GitHubStagingPolicyV1.environment` names a preconfigured GitHub Environment on
the write-authorized `stage` job only; repository owners configure its required
reviewers separately. Eggpack does not create the environment or grant its
approval.

M003d adds the consumer release composition seam without absorbing
product-owned release semantics. Static checked-in workflow shape
(`ReleaseWorkflowShapeV1`: canonical targets, build/core-qualification
bindings, consumer validator configuration, staging intent) carries no
release id, source revision, future tag, or digest. The `resolve` job checks
out the event-selected exact tag, derives HEAD, runs `_resolve-release`
(contract, PackConfig, selected aliases, exact tag, HEAD, draft template),
and uploads the invocation-local ReleasePlan/ReleaseCIPlan/GitHubDraftPolicy
as an internal preflight artifact that every later job downloads before its
`_verify-source` step. `eggpack ci generate/check` operate on static shape in
`--workflow-shape` mode; the same checked-in bytes serve distinct future tags
while exact-source verification stays enforced.

M003d also adds one narrow post-core-qualification verifier per target
(`ConsumerValidatorV1`, finite `Python3` interpreter mapped to `python3` on
Linux/macOS and `python` with a `--version` preflight on Windows, fixed
`interpreter script candidate` argv, no shell/env/argv DSL). Validation runs
after core qualification and before the gate against exact handoff bytes;
required failure blocks aggregation and non-gating failure suppresses the
release. `ConsumerValidationEvidenceV1` carries only identity plus a bounded
outcome; no script output is recorded. Product-wrapper staging is explicit in
the stage job (`--installer-presentation` plus `--source-root` from staging
inputs; absent for M003c-compatible staging). Graphs without validators or
wrappers render byte-identical to M003c.

The consumer validator's execution is not this crate's own: it runs through
`eggpack_core::run_interpreter_bounded`, the shared bounded runner. The
interpreter is restricted to the finite `python3`/`python` names, the argument
vector is OS-native so a validated absolute path reaches the interpreter
byte-exact, the environment is cleared except for the shared allowlist plus the
caller's explicit `PATH` override, the whole process group (or job object) is
terminated on every failure path so no descendant outlives the run, and the
reader-thread joins are bounded by the run's own deadline so a descendant that
inherited a pipe cannot wedge the job. The runner outcome maps onto the
consumer-validation outcome exactly: success is `Passed`, a non-zero exit is
`Failed(NonZeroExit)`, an exceeded deadline is `Failed(Timeout)`, an exceeded
output bound is `Failed(OutputLimit)`, an observed cancellation is
`Failed(Cancelled)`, and an interpreter that cannot be launched is
`Failed(InterpreterUnavailable)`. No script output is recorded, and the
diagnostics are unchanged.

This crate owns no producer rule of its own. The host/target match, the
compatibility floor, the exact-tag safety rule, and the file digest belong to
`eggpack-core`; the SHA-256 hex acceptance set belongs to `eggpack-manifest`.
`CIPlan::validate` applies the core policy validation, the exact-tag check
applies the core safety rule, candidate hashing applies the core file digest
(a refusal is a typed error, not a zero-size placeholder), and
`validate_sha256_hex` applies the manifest rule through core's re-export. Each
call site keeps its own diagnostic wording, so this crate's messages and the
rendered workflow bytes are unchanged.

Runner labels and action commit pins are policy inputs rather than CIPlan data.
Cross-build runners must declare preinstalled cargo-zigbuild and Zig; generated
build jobs check the configured cargo-zigbuild version and never install tools.
Qualification and finalization jobs install the pinned `eggpack-cli` tool and verify it before use; no unpinned binary, arbitrary repository, or curl-pipe-shell is allowed.

## Module map

The crate is a module tree under `src/`. `src/lib.rs` holds the crate
documentation, the module declarations, and the `pub use` re-exports; every
public item is reachable through the crate root exactly as it was when the
crate was a single file, so no consumer changes an import.

| Module | Responsibility |
|---|---|
| `src/lib.rs` | crate documentation, module declarations, public re-exports |
| `src/error.rs` | the bounded `CiError` type and its single constructor |
| `src/validate.rs` | bounded identifier, metadata, and release-identity string validators |
| `src/planning.rs` | the provider-neutral `CIPlan`: schema-v1 graph types, projection, validation |
| `src/policy.rs` | provider policy: action pins, runner mappings, tool provisioning, workflow policy documents |
| `src/runner.rs` | the provider-neutral `RunnerCommand` and its deterministic argument rendering |
| `src/render_github.rs` | the build workflow renderer and the rendering helpers it shares with the release renderers |
| `src/drift.rs` | the one workflow drift comparison and the three public check functions that use it |
| `src/handoff.rs` | the build handoff document: projection, artifact validation, staging |
| `src/release_plan.rs` | the release CI graph: schema-v1 projection, the qualification gate, aggregation, staging intent |
| `src/render_release.rs` | the release workflow renderers and the emission helpers that build their bytes |
| `src/consumer_validator.rs` | the consumer-owned exact-candidate validator: configuration, evidence, bounded interpreter run |
| `src/identity.rs` | the static workflow shape and the runtime release identity it resolves to |
| `src/tests.rs` | the crate's test module |

Items that cross a module boundary are `pub(crate)`; nothing that was private
became publicly reachable. The crate's public API surface is pinned by
`m003n_public_api_surface_is_unchanged` in `src/tests.rs`.
