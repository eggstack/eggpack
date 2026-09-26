# CI and Release Orchestration Milestone 003c Closure — Staging Source Identity and Bounded Transfer

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: corrective plan baseline `e77b913a4f66bf8b3c721562236d75d0e7bea879`; implementation parent `2cf166687d5b4c1e62ddf595ab931c5b2fcf5940` was the clean repository `main` state reviewed before changes.

Implementation commits: `fe0a7f72ff6d4ec182c1160abb0b3f3f6df0aa2c` (implementation), `5c2809908dbd6b236b64e41faf425a133d0e52c4` (pagination regression coverage), and `5c28099d4af475c3da1ed542c870a0515e91da86` (upload-origin negative matrix).

## Executive finding

M003c is closed. Staging-capable generated workflows now use one event-derived source ref throughout release-producing jobs and compare checked-out `HEAD^{commit}` with the checked-in `ReleasePlan.source_revision` before release work. `StagingTagSource` controls required triggers, dispatch input rendering, checkout refs, and the stage condition. Production GitHub asset transfer now verifies and streams opened files in 64 KiB chunks, and remote asset snapshots cover every bounded page. Upload origin and query construction use parsed URL semantics.

No real GitHub draft was created or required by M003c. M003b's live draft qualification is now ready to resume; Phase 8 exit and consumer adoption remain blocked on that evidence and the mirrored eggsact adoption plan.

## Root-cause matrix

| Finding | Correction | Evidence |
|---|---|---|
| A — dispatch built from branch A while staging tag B | DispatchInput checkout ref is `inputs.release_tag` in preflight, build, qualification, gate, aggregate, and stage. Every source-dependent job runs `_verify-source` against ReleasePlan. | `m003c_tag_source_controls_checkout_input_and_stage_guard`; CLI `source_verifier_accepts_tag_commit_and_rejects_other_checkout`; direct/bundle/archive staging goldens. |
| B — `StagingTagSource` was documentary | Validation requires Push for RefName and WorkflowDispatch for DispatchInput. RefName emits no staging-only input and gates stage on tag push; DispatchInput requires `release_tag` and gates stage on dispatch only. | `m003c_tag_source_controls_checkout_input_and_stage_guard`; missing-trigger validation; goldens and deterministic drift comparison. |
| C — production uploads materialized full asset copies | Directory staging opens each regular file, checks opened-handle length and SHA-256 in 64 KiB chunks, rewinds that handle, then sends it through a known-length one-shot Eggfetch body stream. No production `std::fs::read` or full-file Vec clone remains in `stage_with_dir`. | `directory_staging_uses_opened_files_and_rejects_digest_mismatch_before_mutation`; `pathname_replacement_after_open_cannot_change_uploaded_inode`; `upload_request_body_streams_multiple_bounded_chunks_with_known_length`; structural source review. |
| D — asset listing only read page 1 | `list_assets` takes a page number; bounded aggregation requests 100 assets per page and fails if the last allowed page is full. Both pre-upload and post-upload snapshots use all pages. | `staging_reconciles_101_assets_across_pages`; `asset_pagination_covers_101_and_fails_when_bound_exhausted`; `later_page_unexpected_mismatch_and_cross_page_duplicate_fail_closed`. |
| E — upload origin used prefix matching | Returned upload templates are normalized only by the exact `{?name,label}` suffix, parsed, and checked for HTTPS, exact `uploads.github.com`, no credentials, no non-default port/fragment/query, and exact owner/repository/release asset path. | `upload_origin_is_exact_and_names_are_query_encoded`, including lookalike host, userinfo, port, fragment, and wrong path rejection. |
| F — asset name was concatenated into query | Upload URL uses `Url::query_pairs_mut().append_pair("name", name)`; the original name remains the expected response name. | URL round-trip coverage for spaces, plus, ampersand, question mark, hash, percent, and Unicode; upload response exact-name check. |

## Requirement-to-evidence matrix

| Requirement | Evidence and result |
|---|---|
| DispatchInput source identity across jobs | Renderer test proves all release job checkouts resolve dispatch input, source verifier appears before each job's release operation, and stage is dispatch-only. |
| RefName source identity and stage authorization | Renderer test proves all release job checkouts use `github.ref`, stage requires push plus tag ref type, and no staging-only `release_tag` input is emitted. |
| Checked-out source verifier | `_verify-source` parses bounded ReleasePlan JSON, invokes `git rev-parse --verify HEAD^{commit}` without a shell, validates lowercase 40-hex output, compares exact revisions, and emits fixed diagnostics. Disposable Git A/B fixture tags B at `vX`; B passes and A fails. |
| Drift detection | Rendered workflows are compared byte-for-byte; tests mutate a checkout to implicit ref, remove a verifier, broaden stage authorization, and switch source policy. Each differs from expected output. |
| Direct/bundle/archive regressions | M003b staging goldens were regenerated and pass for direct, bundle, and archive plans; staging-disabled M002a golden remains byte-compatible. Existing local orchestration tests remain green for all three forms. |
| Streamed upload body | Eggfetch `RequestBody::from_stream` receives a known length and fixed 64 KiB chunks. Multi-chunk body test covers a body larger than three chunks. The directory staging race test replaces the pathname after open and observes the original inode's bytes uploaded. |
| Pagination and exact set | 101 expected assets reconcile successfully over pages. Page-2 unexpected, digest mismatch, and cross-page duplicate cases fail before upload; a full page at the configured bound fails closed. |
| URL origin and name encoding | Exact origin/path validation rejects lookalikes, userinfo, non-default ports, fragments, and operation identity mismatches. Query-pair round trips preserve all required filename characters. |
| Permission and authority invariants | Parsed-workflow test still proves only stage has `contents: write`; no OIDC, publication, tag mutation, or broad release CLI authority was added. |
| Provider and schema compatibility | No DistributionContract, ReleaseManifest, M001 CIPlan, or StagingPayload schema changed. Existing `max_list_pages` remains optional with a serde default; default is 16 and validation accepts 1–32. |

## Production implementation evidence

- `crates/eggpack-ci/src/lib.rs`: added typed `_verify-source`; staging policy validation enforces its matching trigger; a shared checkout renderer selects GitHub ref or dispatch input; all staging-enabled source jobs verify ReleasePlan identity; stage conditions are source-specific.
- `crates/eggpack-cli/src/main.rs`: added bounded ReleasePlan parsing and direct Git invocation with fixed failure output.
- `crates/eggpack-github/src/lib.rs`: upload seam accepts a one-shot reader and evidence length; directory staging opens and verifies file handles before provider mutation; Eggfetch upload is a known-length 64 KiB stream; asset pages are aggregated within a 32-page ceiling; upload URL and name are parsed/encoded safely.
- `crates/eggpack-ci/tests/fixtures/m003b-{direct,bundle,archive}-staging.yml`: regenerated source-ref and verifier goldens.
- `crates/eggpack-ci/README.md` and `crates/eggpack-github/README.md`: document source policy, stream bounds, pagination, and URL handling.

## Documentation and operations evidence

Operator-facing crate documentation now describes each source mode, pre-work source verification, the bounded streaming contract, pagination ceiling/exhaustion, and URL validation. The historical M003a/M003b closure records carry post-closure annotations without removing their original evidence. No real GitHub release or tag was changed during verification.

## Verification executed

The full repository check passed at `5c28099`; the final `5c28099` change only expands upload-origin test inputs, and its focused test passed. Hosted CI run `36213316240` validates the final commit across all lanes.

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-github --all-targets --all-features --locked     passed (27 tests)
cargo test -p eggpack-ci --all-targets --all-features --locked         passed (30 passed, 2 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked        passed (4 tests)
cargo test --workspace --all-targets --all-features --locked           passed (174 passed, 6 ignored)
cargo doc --workspace --no-deps --locked                               passed
cargo tree -p eggpack-github --locked                                  passed
cargo package -p eggpack-github --locked --allow-dirty                 passed with workspace path patches
cargo package -p eggpack-ci --locked --allow-dirty                     passed with workspace path patches
cargo package -p eggpack-cli --locked --allow-dirty                    passed with workspace path patches
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-github --all-targets --locked             passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                 passed
./scripts/check-local.sh                                               passed
git diff --check                                                       passed
```

The path patches for package verification are the repository's established `scripts/check-local.sh` mechanism for unpublished sibling crates.

Hosted CI run `36213316240` validates final implementation commit `5c28099`; Linux stable, Linux Rust 1.89, macOS, and Windows lanes passed (attempt 1). The workflow run is linked at [GitHub Actions run 36213316240](https://github.com/eggstack/eggpack/actions/runs/36213316240).

## Invariant, recovery, compatibility, and security review

- Exact checked-out source is verified before builds and before qualification, gate, aggregate, or staging consumes checked-in release policy. ReleasePlan is never rewritten dynamically.
- RefName and DispatchInput cannot stage under each other's event. The stage remains the only writer; remote GitHub tag verification still runs before and after staging.
- Upload retries and redirects remain disabled by the lean Eggfetch profile. Known-length stream errors fail the request; token redaction and bounded metadata response handling are unchanged.
- Asset enumeration cannot silently stop at a full final permitted page. Duplicate, unexpected, mismatched, and incomplete remote state fails closed. Narrow starter cleanup remains exact-name/state/size scoped.
- Source and staging-disabled workflow compatibility is preserved. The provider policy schema remains v1 and existing explicit page limits preserve their value.
- No publication endpoint, tag mutation, registry publication, or Eggup deployment authority was added.

### Failure and recovery review

Source mismatch fails in preflight or the affected job before its build, qualification, gate, aggregate, or stage work. Ref/event mismatch makes the stage condition false. Upload file size/digest errors fail before remote mutation; an upload-stream or timeout failure remains a failed staging attempt with retries and redirects disabled. A partial GitHub draft remains available for inspection and a later explicit rerun re-verifies remote state. Pagination exhaustion, duplicates, unexpected assets, and mismatches fail closed; only the pre-existing exact zero-byte `starter` 502 cleanup remains authorized.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher staging correctness or safety finding. | M003c acceptance criteria satisfied. |
| Outstanding operational condition | No real GitHub draft qualification was performed. | M003c does not require it. M003b live qualification is unblocked to resume with a maintainer-authorized repository/tag; Phase 8 and consumer adoption remain blocked until that evidence and the mirrored eggsact plan exist. |
| Evidence boundary | Hosted CI exercises Linux stable/MSRV/macOS/Windows repository checks, not a live production upload against GitHub. | Recorded; live draft qualification remains the separate M003b operational gate. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003c | Reason |
|---|---|---|
| CI M003a | closed historically; corrective closed | M003c resolves its transfer, pagination, origin, and query findings while preserving the original record. |
| CI M003b | conditionally closed | Exact-source and tag-source findings are corrected. Live GitHub draft evidence is the only remaining M003b condition. |
| CI M003c | closed | This record and hosted run `36213316240`. |
| M003b live draft qualification | ready to resume | Corrective dependency is closed; it still requires an authorized fixture and must remain unpublished. |
| Phase 8 exit | blocked | Requires full M003b live draft qualification. |
| Ecosystem adoption M001 (eggsact) | blocked | Requires M003b live draft qualification plus a mirrored eggsact release-adoption plan. |
| Bootstrap M003 | blocked | Independent adoption evidence/candidate review remains outstanding. |
| Eggup Interoperability M003 | ready to plan | Independent of the staging corrective. |

Historical M003a/M003b closure records remain intact with corrective annotations. Registry and both affected roadmaps now identify M003c as closed, M003b live qualification as ready to resume, and downstream consumer adoption as still blocked on its remaining gates.
