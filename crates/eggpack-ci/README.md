# eggpack-ci

Provider-neutral projection of a resolved `ReleasePlan` plus `BuildBindingsV1` into a bounded `CIPlan`, with deterministic checked-in GitHub Actions rendering and drift checking. Builder arguments are derived through `eggpack_core::cargo_command`; this crate does not infer package/bin identities, execute builds, perform qualification, create final manifests, stage releases, publish, access GitHub, or accept arbitrary YAML/step input.

The renderer requires caller-supplied runner labels and full immutable action commit pins. Runner policy declares whether cross-build images already contain cargo-zigbuild and Zig; M001 does not auto-install either tool. Generated build jobs use read-only permissions and hand off private candidate binaries as internal workflow artifacts. Qualification remains unresolved intent in M001 and no M001 workflow claims release completion.

M002 layers an executable `ReleaseCIPlanV1` on the closed M001 graph: per-target qualification jobs projected from M003 bindings, a required-evidence gate reading structured evidence (never logs), and an aggregate/finalize node invoking M004. Build handoffs carry identity-bound relative paths without absolute runner paths; qualification evidence is transported as deterministic JSON with candidate bytes revalidated after download. Required targets must pass; missing or corrupt evidence fails closed; non-gating failures suppress the finalized release instead of producing a partial one. Generated release workflows stay read-only, pin the Eggpack runtime tool to the official repository at an exact revision installed with `--locked`, and upload the completed finalized release as an internal workflow artifact only. No GitHub Release, publication, signing, or staging is performed.

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

Staging-enabled workflows resolve one release source for every job. `RefName`
uses the GitHub ref and authorizes staging only for tag pushes. `DispatchInput`
requires `workflow_dispatch.inputs.release_tag`, uses it for all release
checkouts, and authorizes staging only for dispatches. Each job verifies
`HEAD^{commit}` against the checked-in `ReleasePlan.source_revision` before
release work; `ci check` compares the complete deterministic workflow.

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

Runner labels and action commit pins are policy inputs rather than CIPlan data.
Cross-build runners must declare preinstalled cargo-zigbuild and Zig; generated
build jobs check the configured cargo-zigbuild version and never install tools.
Qualification and finalization jobs install the pinned `eggpack-cli` tool and verify it before use; no unpinned binary, arbitrary repository, or curl-pipe-shell is allowed.
