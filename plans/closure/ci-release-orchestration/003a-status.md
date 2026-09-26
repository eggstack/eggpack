# CI and Release Orchestration Milestone 003a Closure — Local Staging Payload and GitHub Draft Adapter

Historical closure annotation (M003c): post-closure review identified full-file upload buffering, page-1-only asset reconciliation, prefix-based upload-origin validation, and unencoded asset query names. These findings are corrected and closed by `plans/closure/ci-release-orchestration/003c-status.md`; the historical M003a evidence below is retained as originally recorded.

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Hard/interface dependency closures:

- CI M002a: `plans/closure/ci-release-orchestration/002a-status.md`;
- Bootstrap M002a: `plans/closure/bootstrap-installers/002a-status.md`;
- Build/Qualification M004: `plans/closure/build-qualification/004-status.md`;
- ADR-0003: `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`.

Reviewed baseline: `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9` (M003a implementation; no production-code commit supersedes it — later commits, if any, are closure/registry only).

Implementation commits: `36f1cc1` (`feat: implement CI M003a local staging payload and GitHub draft adapter`). One commit carries the M003a provider/payload slice; M003a-owned paths are `crates/eggpack-github/**` (new provider crate plus fixture matrix), `crates/eggpack-cli/src/main.rs` (`_prepare-stage`, `_stage-github-draft`, additive `release-manifest.json` handoff in `_aggregate`), `crates/eggpack-cli/Cargo.toml`, `crates/eggpack-cli/README.md`, `crates/eggpack-github/README.md`, root `README.md`, `scripts/check-local.sh`, `Cargo.toml`, and `Cargo.lock` (new `eggfetch-core 0.2.0` lean profile plus `tokio`).

Implementation SHA: `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9`.

External baselines:

- `eggstack/eggfetch@3a2e23f1953505bf052df6e9c79478bfcad49d78` (reviewed per plan);
- `eggfetch-core 0.2.0` with explicit lean features `standard-http1`, `tls-rustls`, `tls-native-roots`, `json` (no default retry/redirect policy);
- GitHub REST Releases API version `2026-03-10`.

Hosted CI: run `36180399698`, event `push`, `head_sha` exactly `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9`. Conclusion `success` (attempt 1). All four lanes passed: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

## Executive finding

M003a is closed. The provider boundary needed to assemble an exact, reviewable GitHub draft release from an Eggpack-qualified finalized release exists without introducing publication authority. Complete direct, bundle, and archive staging payloads are deterministic; the standalone `release-manifest.json` plus both bootstrap installers are included; M004 root semantics are unchanged; the GitHub adapter verifies the exact existing tag and source revision (including bounded annotated peeling), creates or reuses an exact draft idempotently, reconciles assets exactly without clobbering, applies only the narrow 502 `starter` recovery, keeps credentials environment-only with redacted diagnostics, exposes no publication or tag-write endpoint, and is safely rerunnable. No real GitHub draft was required; network semantics are qualified against a deterministic fixture plus a loopback HTTP transport check. Live draft evidence is reserved for M003b.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Complete direct/bundle/archive staging payloads deterministic | `direct_payload_is_exact_and_deterministic`, `bundle_and_archive_payloads_are_exact`, `installers_deterministic_and_exact_tag_origin`, plus CLI `prepare_stage_materializes_exact_payload`; second materialization byte-identical; payload `to_json` round-trips. |
| Standalone manifest exactly matches M004 manifest | Staging writes `release-manifest.json` via deterministic `ReleaseManifest::to_json()` and verifies `from_json` canonical bytes equal the M004 manifest; CLI `_aggregate` writes the sibling `release-manifest.json` and verifies the same; `generated_orchestration_cli_executes_capture_to_aggregate` extended to assert the handoff file exists, is deterministic, and is absent from the M004 root. |
| Missing/extra finalized file rejects | `missing_extra_and_tampered_finalized_files_reject` (missing, extra). |
| Tampered finalized file rejects | Same test (artifact bytes and sidecar digest paths) plus sidecar-content validation in `prepare_staging_payload`. |
| Symlink rejects | `symlink_and_auxiliary_collision_reject` (symlink extra file); output dirs require real directories with `0700` on Unix; staged reads reject symlinks. |
| Auxiliary asset collision rejects | Same test with `install.sh` colliding contract asset. |
| Generated installers deterministic; origin exact-tag only | `installers_deterministic_and_exact_tag_origin`; origin `https://github.com/<owner>/<repo>/releases/download/<tag>` derived from policy; `latest/download` rejected by construction and asserted absent. |
| Staging payload deterministic | Payload assets sorted by name; `to_json`/`from_json` round-trip asserted. |
| Lightweight exact tag accepted | `lightweight_exact_tag_accepted`. |
| Annotated tag peeling accepted | `annotated_tag_peeling_accepted` (single tag-object hop to exact commit). |
| Tag mismatch rejects before release mutation | `tag_mismatch_rejects_before_mutation` (zero create/upload calls). |
| Excessive tag-object depth rejects | `excessive_tag_depth_rejects` (9-hop chain exceeds bound 8). |
| No release -> draft created | `lightweight_exact_tag_accepted` (`created == true`); `existing_exact_draft_reused` covers reuse. |
| Existing exact draft reused | `existing_exact_draft_reused` (`created == false`, same numeric id). |
| Published release rejects | `published_and_immutable_reject` (draft false). |
| Immutable release rejects | Same test (immutable true). |
| Wrong draft tag/title/prerelease policy rejects | `wrong_draft_policy_rejects` (title mismatch, zero uploads). |
| Existing exact assets reused | `existing_exact_assets_reused_and_missing_uploaded` (1 reuse + 1 upload, then full reuse on rerun). |
| Missing asset uploaded | Same test. |
| Mismatched same-name asset rejects | `mismatched_unexpected_and_duplicate_assets_reject` (size mismatch). |
| Unexpected asset rejects | Same test (`evil.bin`). |
| Renamed upload response rejects | `renamed_and_422_responses_fail_closed` (rename case). |
| 422 duplicate-name response fails closed | Same test (422 case). |
| 502 + exact zero-byte starter -> starter deleted, invocation fails, next invocation can resume | `starter_502_cleanup_is_narrow_and_resumable` (one delete, then success). |
| 502 without exact starter does not delete anything | `starter_502_without_exact_starter_deletes_nothing` (zero deletes). |
| Upload digest/size mismatch rejects | Same rename/422 test file (digest and size cases). |
| Post-stage tag movement rejects | `post_stage_tag_movement_rejects` (tag flipped after uploads). |
| Token absent rejects before I/O | `token_absent_rejects_before_io_and_redacts_diagnostics` (zero create/upload). |
| Diagnostics redact token | Same test (distinctive secret absent from error and `EggfetchTransport` debug). |
| Policy rejects injection and publish flags | `policy_rejects_injection_and_publish_flags` (owner slash, tag query char, non-allowlisted env, `draft:false` unknown field). |
| Lean transport performs bounded loopback HTTP | `lean_eggfetch_client_talks_to_loopback_without_retry_redirect` (real `127.0.0.1` server, bounded body, no retry/redirect features). |
| No publication/tag-write endpoint exists | By construction: `eggpack-github` implements only ref/tag lookup, release list/create (draft true), asset list/upload, and narrow asset delete; no publish, tag create/move/delete, or `gh release --clobber` path; CLI exposes only `_prepare-stage` and `_stage-github-draft`. |

## Production implementation evidence

- `crates/eggpack-github/src/lib.rs` (new, ~2100 lines): `GitHubDraftPolicyV1` (strict, allowlisted `GITHUB_TOKEN`, bounded timeout/metadata/pages, injection guards, `download_origin`); `StagingPayloadV1`/`StagingAsset`/`StagingAssetKind` (deterministic, flat paths, unique names, media types); `GitHubDraftReceiptV1` (draft-only receipt, no secrets); `prepare_staging_payload` (contract/manifest revalidation, exact finalized inventory, sidecar content checks, private `0700` staging dir, standalone manifest with canonical round-trip, deterministic bootstrap installers with exact-tag origin, unique-name and digest computation); `GithubApi` transport seam (async, fixed-host production plus deterministic fixture); `verify_tag_source` (lightweight plus bounded 8-deep annotated peeling); `stage_with_bytes`/`stage_with_dir` (policy/payload agreement, pre/post tag verification, bounded paginated draft lookup, duplicate rejection, exact create/reuse validation, exact asset reconciliation, 502 narrow starter cleanup without auto-retry, final exact-set verification, redacted errors); `EggfetchTransport` (fixed `https://api.github.com` plus `https://uploads.github.com` constraint, `Accept`/`X-GitHub-Api-Version`/`User-Agent` headers, Bearer auth with redacted `Debug`, lean `eggfetch-core` profile with no retries/redirects, 3xx fail-closed, bounded timeouts/body sizes, 502/422 mapping); `FixtureGithub` (deterministic in-memory endpoint semantics with call counts and behavior flags).
- `crates/eggpack-github/src/tests.rs` (new, 21 tests): full local-payload plus fixture-adapter matrix above, including loopback HTTP transport proof.
- `crates/eggpack-cli/src/main.rs`: `_aggregate` additively writes sibling `release-manifest.json` with canonical decode verification (M004 root untouched); new `_prepare-stage` (contract, release-manifest, finalized root, GitHub policy, install policy in TOML/JSON, output dir, output payload) and `_stage-github-draft` (payload, policy, receipt, environment-only token, current-thread Tokio runtime, staging dir resolution); usage strings updated; tests extended (`release-manifest.json` handoff assertions plus `prepare_stage_materializes_exact_payload`).
- `Cargo.toml` workspace plus `crates/eggpack-cli/Cargo.toml` (new `eggpack-github`, `eggpack-manifest`, `eggpack-bootstrap`, `tokio` deps; `sha2` dev-dep); `Cargo.lock` (new `eggfetch-core 0.2.0` lean tree plus `tokio`).
- No M001 `CIPlan`, M003 qualification, M004 finalization, contract/manifest schemas, or permission policy changed.

## Verification executed

Local verification at implementation commit `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9`:

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-github --all-targets --all-features --locked     21 passed
cargo test -p eggpack-ci --all-targets --all-features --locked         21 passed, 1 ignored
cargo test -p eggpack-cli --all-targets --all-features --locked        3 passed
cargo test -p eggpack-bootstrap --all-targets --all-features --locked  8 passed
cargo test --workspace --all-targets --all-features --locked           158 passed, 5 ignored
cargo doc --workspace --no-deps --locked                               passed
cargo tree -p eggpack-github --locked                                  passed (lean eggfetch-core tree)
cargo package -p eggpack-github --locked --allow-dirty (+ path patches) passed
cargo package -p eggpack-cli --locked --allow-dirty (+ path patches)   passed (via check-local)
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-github --all-targets --locked            21 passed
./scripts/check-local.sh                                               passed (exit 0)
git diff --check                                                       passed
```

Hosted verification: run `36180399698` (push, `head_sha` = `36f1cc1`, conclusion `success`, attempt 1) passed all four lanes: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

No medium-or-higher finding emerged. No real GitHub draft was created; network semantics rest on the deterministic fixture plus loopback transport check, as authorized for M003a.

## Invariant review

Only an already-existing exact tag may be staged; tag resolves to the exact `ReleaseManifest.source_revision` before and after staging; annotated tags peel bounded to an exact commit; Eggpack never creates, moves, force-updates, or deletes tags; draft is always `true` for created releases; no API/CLI path publishes; published/immutable releases never mutate; unexpected assets fail closed; same-name reuse requires exact size plus `sha256:` digest; mismatches never clobber or silently delete; only an exact zero-byte `starter` after a 502 may be deleted, and that invocation still fails; credentials come only from `GITHUB_TOKEN` and never serialize, log, or appear in `Debug`; HTTP retries/redirects are disabled; bounded timeouts/body sizes apply; staging paths reject symlinks/traversal; M004 root semantics unchanged. All hold.

## Failure/recovery review

Reruns re-verify tag/source, reuse exact drafts/assets, upload only missing assets, fail on mismatched/unexpected assets, never clobber or publish. Failure after draft creation leaves the incomplete draft for maintainer inspection; no automatic release deletion. 502 starter cleanup is narrow (exactly one zero-byte `starter` with the expected name), reports failure, performs no same-invocation retry; the next explicit invocation may resume. Receipt-upload failure is CLI-local (no remote mutation beyond what staging already reported). All paths fail closed with bounded redacted diagnostics.

## Compatibility/migration review

M001 `CIPlan` v1, M003 qualification, M004 finalization, contract/manifest schemas, and generated-workflow permissions are unchanged. `GitHubDraftPolicyV1`, `StagingPayloadV1`, and `GitHubDraftReceiptV1` are new additive provider-facing types, not stable public contracts beyond staging. `_aggregate` output is additive (sibling `release-manifest.json`); generated workflows already upload `./eggpack-finalized`, so no renderer change was required. No consumer migration is required or claimed.

## Security review

Token redaction (environment-only, absent-before-I/O, `BearerAuth(<redacted>)`, custom redacted `Debug`, error strings proven secret-free); fixed production hosts (`https://api.github.com`, uploads constrained to `https://uploads.github.com` with `upload_url` prefix check); HTTPS/TLS via `tls-rustls` plus native roots; no retries/redirects (lean profile, 3xx fail-closed); metadata body bounds; annotated peel bound (8) plus pre/post tag verification (TOCTOU); asset-name injection guards (flat safe names, exact `?name=` construction); local symlink/traversal handling; draft/public/immutable state transitions (create `draft:true`, `make_latest:false`, no `target_commitish`, no publish endpoint); 502 starter authorization (exact single zero-byte record only); malicious remote behavior (duplicate/unexpected/renamed/digest-mismatched assets fail closed). No new privilege scope; the future M003b renderer must isolate write permission to the stage job only.

## Documentation/operations evidence

Root `README.md` (M003a payload/adapter scope, M003b remainder), `crates/eggpack-github/README.md` (draft-only, exact tag, no publication, env-only, rerun semantics), `crates/eggpack-cli/README.md` (`_aggregate` handoff plus `_prepare-stage`/`_stage-github-draft`), roadmap and registry updated herewith. Operators must note: staging creates or reuses a draft only; public publication is a separate human action; incomplete drafts may remain after failure; reruns reconcile exact state rather than clobbering.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher staging safety issue. | M003a acceptance criteria pass. |
| Informational | Runner images remain policy-declared and are exercised only when generated release workflows run against real repositories. | Unchanged; staging jobs do not yet exist (M003b). |
| Environmental | ShellCheck/PSScriptAnalyzer availability is supplementary; loopback HTTP proof covers the lean transport profile locally, while Windows/macOS lanes are evidenced by hosted CI above. | Recorded; not a product finding. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003a | Reason |
|---|---|---|
| CI M001 | closed | Unchanged. |
| CI M002 | closed historically; corrective satisfied | Unchanged. |
| CI M002a | closed | Unchanged. |
| CI M003a | closed | Implementation `36f1cc1`; hosted run 36180399698 green (attempt 1, all lanes). |
| CI M003b | ready | Hard M003a dependency now satisfied; final CLI/provider contracts must be re-reviewed against the M003b plan before handoff, and full closure additionally requires a maintainer-authorized live draft fixture. |
| Bootstrap M002a | closed | Unchanged. |
| Ecosystem adoption (eggsact M001) | blocked | Still requires M003b closure plus a mirrored eggsact plan; unchanged. |

M003b is therefore **dependency-ready**: the provider/payload hard dependency is closed, and the M003b plan may proceed to implementation once its final-interface re-review is recorded.

## Registry updates

`plans/registry.md` reconciled: CI M003a moved to closed with this closure path and implementation/run evidence; CI M003b moved to ready (blocked only on live draft fixture for full closure plus final-interface re-review); narrative, execution graph, and next-handoff paragraphs updated to the post-close state. CI roadmap `plans/subsystems/ci-release-orchestration-roadmap.md` updated to match.


## Post-closure corrective registration — M003c

A later staging audit found medium-severity provider correctness gaps not captured by this closure:

- production release-asset upload fully buffers files and duplicates full asset bytes in memory;
- remote asset reconciliation reads only the first 100 assets despite a 1,024-asset payload bound;
- upload host validation uses prefix matching rather than exact parsed host/origin validation;
- asset names are interpolated into the upload query without URL encoding.

The historical implementation and hosted run remain valid evidence for the behavior they exercised, but the statement that no unresolved medium-or-higher staging safety finding remained is superseded.

Corrective plan: `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md`.

M003a is retained as historical closure evidence with M003c active. No live-draft qualification or consumer adoption should rely on the provider path until M003c closes.
