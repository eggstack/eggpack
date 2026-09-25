# CI and Release Orchestration Milestone 003b Closure — Generated Draft Staging Job and Operational Qualification

Status: conditionally closed

Source plan: `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Hard/interface dependency closures:

- CI M003a: `plans/closure/ci-release-orchestration/003a-status.md`;
- CI M002a: `plans/closure/ci-release-orchestration/002a-status.md`;
- ADR-0003: `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`.

Reviewed baseline: `84e4e4f362a43e2ab6f984905fca631b3ac27367` (M003b implementation).

Implementation commits: `84e4e4f` (`feat: implement CI M003b generated draft staging job`). M003b-owned paths are `crates/eggpack-ci/src/lib.rs` (staging intent, provider policy, renderer stage job, guards, tests), `crates/eggpack-ci/tests/fixtures/m003b-*-staging.yml` (new goldens), `crates/eggpack-ci/Cargo.toml` + `Cargo.lock` (dev-dep test harness wiring), `crates/eggpack-cli/src/main.rs` (RunnerCommand round-trip harness), `scripts/check-local.sh` (packaging harness fix), root `README.md`, `crates/eggpack-ci/README.md`, `crates/eggpack-cli/README.md`, `crates/eggpack-github/README.md`.

Final M003a interface re-review (plan readiness rule): the M003a CLI/provider contracts at closure `36f1cc1` were re-reviewed against this plan before handoff. `_prepare-stage` flags (`--contract`, `--release-manifest`, `--finalized-root`, `--github-policy`, `--install-policy`, `--output-dir`, `--output-payload`) and `_stage-github-draft` flags (`--payload`, `--github-policy`, `--staging-dir`, `--output-receipt`) match the rendered `RunnerCommand::PrepareStage` / `RunnerCommand::StageGithubDraft` argv exactly; `GitHubDraftPolicyV1`, `StagingPayloadV1`, and `GitHubDraftReceiptV1` semantics are unchanged. No M003a interface change was required.

Hosted CI: run `36193654633`, event `push`, `head_sha` exactly `84e4e4f362a43e2ab6f984905fca631b3ac27367`. Conclusion `success` (attempt 1). All four lanes passed: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

## Executive finding

M003b is conditionally closed. The deterministic checked-in workflow now stages a fully qualified M002a aggregate as a GitHub draft release through the M003a provider adapter, with write authority isolated to exactly one staging job. Direct, bundle, and archive staging renders are golden-pinned; the provider-neutral graph, renderer policy, typed runner commands, permission matrix, drift detection, and local fake-GitHub orchestration (direct/bundle/archive, rerun, tag-mismatch, published/mismatch refusal) are qualified. No real GitHub draft was assembled: no maintainer-authorized repository/tag with a mirrored eggsact adoption plan was available, and no throwaway public release was created. Phase 8 exit is therefore not satisfied; eggsact adoption remains blocked.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Provider-neutral staging node (`StagingProvider::GitHubDraft`, `StagingJob`, aggregate dependency, finalized/receipt handoff names) | `m003b_staging_intent_is_provider_neutral_and_optional`; `with_github_draft_staging` additive; M001 `CIPlan` untouched (`staged.ci_plan == graph.ci_plan`); disabled graphs serialize with no `staging` field. |
| GitHub renderer policy (runner, owner/repo, tag source, paths, receipt retention; validation) | `GitHubStagingPolicyV1` + `GitHubStagingInputsV1` + `StagingTagSource`; `GitHubPolicy::validate` rejects bad runner/identity/paths; `m003b_staging_requires_policy_and_rejects_forbidden_commands`. |
| Generated stage job (`needs: aggregate`, `contents: write` only there, checkout/tag/install/download/prepare/stage/receipt steps, no gh/curl, no publish/tag mutation) | Goldens `m003b-direct/bundle/archive-staging.yml`; `m003b_rendered_staging_is_least_privilege_and_exact` (writers == `["stage"]`, needs aggregate, tag/dispatch guard, exact handoff download, prepare-before-stage, env-only token, receipt upload, deterministic rerender, drift on permission + step edits). |
| Exact aggregate handoff (`eggpack-finalized/root`, `summary.json`, `release-manifest.json`) | Stage downloads `eggpack-finalized-release` to `./eggpack-finalized`; prepare consumes `./eggpack-finalized/release-manifest.json` + `./eggpack-finalized/root`; no rebuilt binaries. |
| Trigger/tag semantics (exact-tag only; tag-push via `ref_name` + ref-type tag; dispatch via explicit `release_tag`; exact checkout; no branch/latest) | `workflow_dispatch.inputs.release_tag` required when staging enabled; stage `if: github.ref_type == 'tag' \|\| workflow_dispatch`; checkout `ref: ${{ dispatch && inputs.release_tag \|\| github.ref }}`; `Validate exact-tag source` step; M003a verifies server-side tag→commit before/after. |
| Permissions (top `read`; preflight/build/qualify/gate/aggregate `read`; stage `write`; no `id-token: write`) | Parsed-YAML matrix test proves exactly one writer (`stage`); top-level `contents: read`; no `id-token: write` anywhere; static renderer guards. |
| Concurrency (release-scoped, tag-aware dispatch; M003a authoritative) | Staging-enabled group `eggpack-${{ github.workflow }}-${{ dispatch && inputs.release_tag \|\| github.ref }}`; disabled retains M002a key; test asserts tag-aware group. |
| Staging receipt/status (internal artifact, no secrets) | Receipt upload `eggpack-staging-receipt` → `./eggpack-staging-receipt.json`; receipt is M003a `GitHubDraftReceiptV1` (ids, tag/source, draft/immutable, asset digests, created/reused counts; no token/API bodies). |
| Golden fixtures (direct/bundle/archive + staging; disabled byte-compatible) | `m003b_golden_direct_bundle_archive_with_staging`; `m003b_staging_disabled_renders_byte_compatible` (M002a `m002-direct.yml` unchanged). |
| Local executable orchestration (aggregate → prepare → fake adapter → receipt; same typed argv; direct/bundle/archive; rerun/mismatch/published/refusal) | `m003b_local_orchestration_stages_through_fake_adapter` (direct, create+reuse, tag-mismatch, published, mismatched asset); `m003b_local_orchestration_covers_bundle_and_archive` (bundle + archive create+reuse); CLI `prepare_stage_materializes_exact_payload` via `RunnerCommand::PrepareStage.argv()`; existing capture→aggregate harness unchanged. |
| Live operational qualification (real draft, remains draft, rerun without clobber, no publication) | NOT performed — outstanding condition (see below). Fake-server + loopback evidence only. |
| `ci check` detects staging drift | Permission edit + step edit both fail `check_release_github`; CLI `generate_equals_library_and_check_detects_drift` unchanged. |
| No unresolved medium-or-higher staging finding | None; see findings table. |

## Production implementation evidence

- `crates/eggpack-ci/src/lib.rs`: `StagingProvider`, `StagingJob`, `ReleaseCIPlanV1.staging` (optional, additive), `with_github_draft_staging`, `GitHubStagingPolicyV1`/`GitHubStagingInputsV1`/`StagingTagSource`, `RunnerCommand::PrepareStage`/`StageGithubDraft` (+ `argv`/`to_shell`), staging validation in `GitHubPolicy::validate` and `ReleaseCIPlanV1::validate`, `render_release_github` stage job (tag-aware dispatch input + concurrency, exact-tag checkout/validation, pinned tool, exact aggregate download, prepare-then-stage, env-only `GITHUB_TOKEN`, receipt upload), static forbidden-command guards (`gh release`, `--clobber`, `release --publish`, `git tag `, `git push --tags`, `curl `, `id-token: write`).
- `crates/eggpack-ci/tests/fixtures/m003b-{direct,bundle,archive}-staging.yml`: new goldens, all with single `stage` writer and tag-aware concurrency.
- `crates/eggpack-cli/src/main.rs`: `prepare_stage_materializes_exact_payload` now drives `ci_prepare_stage` through `RunnerCommand::PrepareStage.argv()`, proving renderer/CLI argument identity.
- `scripts/check-local.sh`: `cargo package -p eggpack-ci` gains `eggpack-bootstrap` + `eggpack-github` path patches for the new dev-dep harness.
- No M001 `CIPlan` schema, M003 qualification, M004 finalization, contract/manifest schemas, or pre-existing job permissions changed.

## Verification executed

Local verification at implementation commit `84e4e4f362a43e2ab6f984905fca631b3ac27367`:

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-github --all-targets --all-features --locked     21 passed
cargo test -p eggpack-ci --all-targets --all-features --locked         29 passed, 2 ignored
cargo test -p eggpack-cli --all-targets --all-features --locked        3 passed
cargo test --workspace --all-targets --all-features --locked           166 passed, 6 ignored
cargo doc --workspace --no-deps --locked                               passed
cargo tree -p eggpack-ci --locked                                      passed
cargo package -p eggpack-ci --locked --allow-dirty (+ path patches)    passed (via check-local)
cargo package -p eggpack-cli --locked --allow-dirty (+ path patches)   passed (via check-local)
cargo +1.89.0 check --workspace --all-targets --locked                 passed (via check-local)
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                passed (via check-local)
./scripts/check-local.sh                                               passed (exit 0)
git diff --check                                                       passed
```

Hosted verification: run `36193654633` (push, `head_sha` = `84e4e4f`, conclusion `success`, attempt 1) passed all four lanes: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

No medium-or-higher finding emerged. No real GitHub draft was created; network semantics rest on the deterministic fixture plus M003a loopback transport proof, as authorized for conditional close.

## Invariant review

Generated workflow stays checked in and deterministic; all pre-stage jobs retain `contents: read`; exactly one staging job has `contents: write`; no job has `id-token: write`; token travels via environment only; staging calls only M003a draft-only commands; workflow never creates/moves/deletes tags; never publishes; staging runs only after aggregate; consumes the exact aggregate artifact; payload includes manifest + both installers; no `gh release --clobber`; reruns reconcile via M003a; published/immutable state fails closed; `ci check` detects staging permission/step drift. All hold.

## Failure/recovery review

Aggregate failure blocks stage (`needs: aggregate`). Prepare failure performs no GitHub mutation (no token in prepare step). Partial draft creation leaves the draft for inspection; explicit rerun reconciles exact state without clobber (tested). Staging failure reds the workflow with no publication or broad cleanup. Receipt-upload failure after remote success reds the workflow while the remote draft remains; rerun re-verifies/reuses exact state. All paths fail closed.

## Compatibility/migration review

M001 `CIPlan` v1 semantics unchanged. `ReleaseCIPlanV1` staging is optional and additive; disabled graphs serialize exactly as M002a (byte-compatible golden). New provider types (`StagingProvider`, `StagingJob`, `GitHubStagingPolicyV1`) are staging-scoped and introduce no public schema beyond CI policy. No consumer migration required. Existing hand-written workflows remain until a mirrored adoption plan proves parity; Eggpack does not overwrite workflows without `generate`/`check`.

## Security review

Permission matrix parsed from rendered YAML (exactly one writer); top-level read-only; OIDC never minted. Token env-only with `secrets.GITHUB_TOKEN` reference; absence fails before I/O (M003a); redacted diagnostics proven in M003a. No `pull_request` trigger reaches the write job; stage gated on tag/dispatch. Tag/ref injection guarded by policy validation + M003a exact-tag pre/post verification + bounded annotated peeling. Shell quoting via single-quote escaping + JSON YAML scalars. Immutable action pins + pinned Eggpack tool revision required by renderer. Artifact trust boundary: canonical internal handoff names only; no rebuilt binaries. Receipt carries no secrets. Dispatch tag validated (required input + shell guard + M003a preflight). Rerun/concurrency: tag-aware group plus authoritative M003a reconciliation. Published/immutable refusal tested. No publish endpoint/command exists (type-level + static guards + tests).

## Documentation/operations evidence

Root `README.md` (stage job, least privilege, rerun, live outstanding), `crates/eggpack-ci/README.md` (staging intent/policy/renderer/guards), `crates/eggpack-cli/README.md` (staging implemented), `crates/eggpack-github/README.md` (stage-job consumption pointer), roadmap and registry updated herewith. Operators must note: stage creates/reuses a draft only; public publish is a separate human action; write permission is isolated to stage; reruns reconcile exact state rather than clobbering; live draft qualification is outstanding.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher staging safety issue. | Conditional-close acceptance criteria pass on fake-adapter evidence. |
| Outstanding condition | Live GitHub draft assembly (plan §7: real repo/tag/run, draft remains unpublished, exact rerun without clobber, no publication/tag mutation) has not been performed. No maintainer-authorized repository/tag with a mirrored eggsact adoption plan was available; no throwaway release was created. | M003b remains `conditionally closed`; Phase 8 exit not satisfied. Requires a maintainer-authorized fixture (expected first candidate: eggsact after its mirrored adoption/release-workflow handoff is registered, preserving crates.io-first/tag-after-publish ordering and product-owned fallback/update semantics). |
| Informational | Runner images remain policy-declared and are exercised only when generated release workflows run against real repositories. | Unchanged; staging jobs run only on tags/dispatch in consumers. |
| Environmental | ShellCheck/PSScriptAnalyzer availability is supplementary; Windows/macOS lanes are evidenced by hosted CI above. | Recorded; not a product finding. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003b | Reason |
|---|---|---|
| CI M001 | closed | Unchanged. |
| CI M002 | closed historically; corrective satisfied | Unchanged. |
| CI M002a | closed | Unchanged. |
| CI M003a | closed | Unchanged. |
| CI M003b | conditionally closed | Implementation `84e4e4f`; hosted run 36193654633 green (attempt 1, all lanes); live draft fixture outstanding. |
| Bootstrap M002a | closed | Unchanged. |
| Ecosystem adoption (eggsact M001) | blocked | Still requires full M003b closure (live draft) plus a mirrored eggsact plan; unchanged. No unblocking performed by this conditional close. |

M003b implementation is complete, but Phase 8 is NOT satisfied and no dependent adoption work is unblocked until the live-draft condition clears via a future full-closure pass.

## Registry updates

`plans/registry.md` reconciled: CI M003b moved to conditionally closed with this closure path and implementation/run evidence; narrative, execution graph, and next-handoff paragraphs updated to the post-conditional-close state. CI roadmap `plans/subsystems/ci-release-orchestration-roadmap.md` updated to match.
