# CI and Release Orchestration Milestone 003c — Staging Source Identity and Bounded Transfer Corrective

Status: closed

Repository baseline: `e77b913a4f66bf8b3c721562236d75d0e7bea879`

Historical staging implementations:

- M003a implementation: `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9`
- M003a closure: `plans/closure/ci-release-orchestration/003a-status.md`
- M003b implementation: `84e4e4f362a43e2ab6f984905fca631b3ac27367`
- M003b conditional closure: `plans/closure/ci-release-orchestration/003b-status.md`

Source roadmap:

- `plans/subsystems/ci-release-orchestration-roadmap.md`

External transport baseline reviewed for this corrective:

- `eggstack/eggfetch@6616d061aba523182ad00a37a64a4919b44bd73d`
- `eggfetch-core 0.2.0`
- current RequestBody supports one-shot streamed bodies with known length through `RequestBody::from_stream`
- current native transport sends stream chunks incrementally and applies write deadlines without eager buffering

Primary class: corrective / source-integrity / provider safety / bounded I/O / generated CI

## 1. Objective

Correct the post-M003a/M003b staging findings before any real GitHub draft qualification or consumer adoption.

M003c must establish one exact source identity across every job in a staging-capable generated workflow, make the existing `StagingTagSource` policy semantically authoritative, eliminate full-file buffering from production release-asset uploads, reconcile all remote assets across bounded pagination, and harden upload URL/query handling to the fixed-host policy already claimed by M003a.

This corrective is a prerequisite for:

- resuming M003b live-draft qualification
- Phase 8 completion
- eggsact M001 adoption
- any downstream consumer workflow that relies on generated draft staging

No live draft should be attempted until M003c closes.

## 2. Post-closure findings

### Finding A — high: workflow_dispatch source mismatch

For a staging-enabled workflow_dispatch run, the generated workflow accepts an explicit `release_tag`, but only the final stage job checks out that tag.

Preflight/build/qualification/gate/aggregate jobs continue to check out the workflow-dispatch ref, typically the selected branch.

A dispatch can therefore build candidate bytes from branch commit A while staging against tag commit B. The remote tag-to-ReleaseManifest verification can still pass because it occurs only after the wrong-source bytes have already been built.

Current build/qualification evidence carries declared source identity but does not independently prove the Git checkout that produced candidate bytes.

This violates the exact-source invariant and blocks live qualification.

### Finding B — medium: StagingTagSource is not enforced

`GitHubStagingPolicyV1.tag_source` declares `RefName` or `DispatchInput`, but current rendering does not branch on that value. The stage job accepts either a tag ref or workflow_dispatch regardless of the configured variant.

The type is documentary rather than authoritative.

### Finding C — medium: production asset uploads are fully buffered

Current production staging uses `std::fs::read` in `stage_with_dir`, then clones the complete asset again through `.bytes(bytes.to_vec())` in the Eggfetch upload request.

Large binaries/archives can therefore require multiple complete in-memory copies.

This is unnecessary because the reviewed Eggfetch baseline supports one-shot streamed request bodies with known length.

### Finding D — medium: remote asset reconciliation only reads page 1

Current production `list_assets` requests only `per_page=100&page=1`, while `StagingPayloadV1` permits up to 1,024 assets.

Unexpected, duplicate, mismatched, or stale assets on later pages can be invisible to pre-upload and post-upload exact-set checks.

### Finding E — medium: upload-host validation is prefix-based

Current `check_upload_url` accepts URLs beginning with `https://uploads.github.com`. Prefix matching is weaker than the stated fixed-origin invariant and can accept lookalike host text.

### Finding F — medium: upload asset names are not URL-query encoded

`validate_asset_name` rejects path separators/control characters but permits query-significant characters such as spaces, ampersand, question mark, hash, plus, and percent.

The upload URL currently interpolates `?name=<asset-name>` directly rather than using encoded query pairs.

## 3. Corrective boundaries

Preserve:

- M001/M002/M002a build/qualification/finalization semantics
- M003a draft-only GitHub authority
- M003b one-writer permission model
- no publication endpoint
- no tag creation/move/delete
- no automatic registry publication
- no Eggup deployment authority
- no generic workflow DSL
- no second HTTP client

M003c may change generated checkout/ref behavior, finite runner commands, staging policy validation, eggpack-github provider internals/test seams, and staging closure/registry state.

## 4. One exact release source per staging-capable run

Introduce one deterministic resolved release-ref model used by every repository checkout in a staging-enabled graph.

For tag-push mode, the source is the exact GitHub tag ref.

For dispatch mode, the source is the explicit `inputs.release_tag`.

When staging is enabled, every source-dependent job for the release event must use the same resolved ref:

- preflight
- every build job
- every qualification job
- required gate
- aggregate
- stage

Ordinary staging-disabled candidate workflows retain existing checkout behavior.

### RefName mode

- only a tag push may run stage
- stage condition requires push + ref_type tag
- release source is the exact GitHub ref
- branch pushes may run candidate work if policy intentionally allows them, but must never stage
- workflow_dispatch must not reach stage under this mode

### DispatchInput mode

- only workflow_dispatch may run stage
- `release_tag` is required
- every source checkout in that dispatch uses `inputs.release_tag`
- tag pushes must not reach stage under this mode

Do not implicitly support both modes from one enum variant. If true dual-mode staging is later required, add an explicit reviewed variant.

## 5. Verify checked-out source against ReleasePlan

Add a finite internal source verifier, preferably a new `RunnerCommand` variant and CLI command such as:

```text
eggpack ci _verify-source --release-plan <path>
```

The command must:

1. parse the bounded ReleasePlan
2. invoke Git directly without a shell, e.g. `git rev-parse --verify HEAD^{commit}`
3. require lowercase 40-hex output
4. require exact equality with `ReleasePlan.source_revision`
5. emit no arbitrary Git output on failure
6. fail before build/qualification/finalization work proceeds

Run source verification after checkout in preflight and every build job. Prefer also running it in qualification/gate/aggregate/stage jobs because those jobs consume checked-in release policy/configuration.

A workflow-dispatch tag whose checked-in ReleasePlan declares a different source revision must fail closed. Do not rewrite ReleasePlan dynamically in CI.

## 6. Make StagingTagSource authoritative

`GitHubPolicy::validate` and release rendering must enforce the configured tag source.

For `RefName`:

- Push trigger must exist
- stage condition is tag-push only
- no release_tag input is required solely for staging
- staging-event checkout source is GitHub ref

For `DispatchInput`:

- WorkflowDispatch trigger must exist
- required release_tag input is rendered
- stage condition is dispatch-only
- all source checkouts in the dispatch use release_tag

Reject policies whose trigger set cannot satisfy the declared tag source.

Tests must prove changing tag_source changes the generated workflow.

## 7. Stream production asset uploads

Do not add reqwest, raw hyper, or curl.

Use the existing Eggfetch streaming primitive `RequestBody::from_stream(stream, Some(length))` with logical retry/redirect support disabled.

The production directory-based path must not call `std::fs::read` for release assets and must not clone a complete asset into a second Vec.

For each asset:

1. reject symlink/non-regular path
2. open the file once for the upload transaction
3. read metadata/length from that opened handle
4. require exact length equals StagingPayload evidence
5. hash the opened file in bounded chunks
6. require exact SHA-256 equals StagingPayload evidence
7. seek the same opened handle to offset zero
8. stream it in bounded chunks with known Content-Length
9. retain existing response size/digest verification

Using the same opened handle for verify + stream prevents pathname replacement from changing the uploaded inode between verification and transfer.

If same-handle seek/stream cannot be implemented safely on a supported platform, use an invocation-private verified copy and stream that copy. Do not fall back to full buffering.

Use a fixed documented chunk bound such as 64 KiB.

The fake provider may retain small byte-backed fixture helpers, but the production `stage_with_dir` path must exercise the streamed source.

Refactor the provider upload seam as needed so production is not forced through `&[u8]`. Do not expose Eggfetch-specific body types in the public provider-domain API unless necessary.

## 8. Complete remote asset pagination

Make asset enumeration page-aware.

The provider seam may become:

```text
list_assets(owner, repo, release_id, page)
```

or expose an equivalent bounded all-pages helper.

Production requests remain `per_page=100&page=N`.

Add an explicit bounded asset-list page policy or a fixed internal bound large enough to cover the 1,024-asset payload model. Recommended default: 16 pages, accepted range no greater than 32.

If the final permitted page is full, fail with pagination-bound-exhausted because completeness cannot be proven.

Both pre-upload and post-upload snapshots must:

- aggregate all bounded pages
- detect duplicates across page boundaries
- detect unexpected assets on later pages
- detect mismatched expected assets on later pages
- verify the exact final set across all pages

Required tests include 101+ assets, unexpected page-2 asset, cross-page duplicate, page-2 mismatch, full-page continuation, and pagination-bound exhaustion.

## 9. Exact upload origin validation

Replace prefix matching with parsed URL validation.

For a GitHub release upload URL require:

- scheme exactly https
- host exactly uploads.github.com
- no username/password
- no non-default port
- expected release upload path shape
- owner/repository/release id match the current operation
- no unexpected fragment

Normalize the GitHub upload template suffix only through an explicit parser/helper.

The production transport may continue constructing a fixed upload endpoint after validating the returned template.

## 10. Correct query encoding

Construct upload URLs through a URL API and encoded query pairs.

Do not concatenate `?name={name}`.

Required filename coverage:

- spaces
- plus
- ampersand
- question mark
- hash
- percent
- Unicode if DistributionContract currently permits it

The name returned by GitHub after upload must equal the original asset name exactly.

If the project wants a narrower asset-name grammar instead, that requires a separately reviewed contract change; do not silently narrow names inside the GitHub adapter.

## 11. Generated workflow regression matrix

Add goldens/static assertions for both source modes.

### DispatchInput

Prove:

- required release_tag input exists
- preflight/build/qualification/gate/aggregate/stage check out release_tag for dispatch
- source verifier runs before builds
- stage is dispatch-only
- tag push cannot reach stage

### RefName

Prove:

- stage requires push + ref_type tag
- all source-dependent jobs participating in the tagged release use the same GitHub ref
- source verifier runs
- workflow_dispatch cannot reach stage

### Drift

`eggpack ci check` must detect:

- a build job reverting to implicit checkout
- one job checking out a different ref
- source verifier removal
- stage condition broadening
- tag_source-derived input/condition drift

## 12. End-to-end source-integrity tests

Use a disposable Git fixture or equivalent executable harness.

Required positive case:

1. create commit A on main
2. create commit B and tag vX at B
3. dispatch-mode staging requests vX
4. prove every release-producing job resolves B
5. candidate/manifest source identity is B
6. stage adapter verifies remote tag B
7. orchestration succeeds

Required negatives:

- checkout A while ReleasePlan claims B -> source verifier fails before build
- tag moved between pre-stage/post-stage checks -> staging fails
- release_tag points at B but checked-in plan at B claims C -> fails
- RefName mode invoked by dispatch -> stage unauthorized
- DispatchInput mode invoked by tag push -> stage unauthorized

## 13. Provider transport tests

Add focused tests for:

- multi-chunk streamed upload
- known Content-Length equals payload evidence
- file digest mismatch rejects before HTTP mutation
- pathname replacement after open cannot substitute bytes
- streamed write timeout fails safely
- redirects/retries remain disabled
- exact uploads.github.com host accepted
- uploads.github.com.example.invalid rejected
- userinfo/non-default port/fragment rejected
- query-significant asset names round-trip exactly
- 101+ remote assets reconcile across pages
- pagination exhaustion fails closed

Do not claim exact RSS from unit tests; prove structurally that production upload no longer materializes a full-file Vec.

## 14. Closure and registry reconciliation

On registration of M003c:

### M003a

Preserve `plans/closure/ci-release-orchestration/003a-status.md` as historical evidence, but annotate it with the post-closure findings covering:

- full-file buffering
- incomplete asset pagination
- weak upload-origin validation
- missing query encoding

Roadmap state:

```text
M003a closed historically / M003c corrective active
```

### M003b

Preserve `plans/closure/ci-release-orchestration/003b-status.md` as historical conditional-close evidence, but annotate it with:

- workflow_dispatch source mismatch
- non-authoritative StagingTagSource

Withdraw the statement that live draft evidence is the only remaining blocker.

Roadmap state:

```text
M003b conditionally closed historically / M003c corrective active
```

### M003c

Register:

```text
M003c staging source identity + bounded transfer corrective — READY
```

Keep blocked:

- live GitHub draft qualification
- Phase 8 exit
- eggsact M001 adoption
- any consumer migration using generated draft staging

After M003c closes, M003b may return to conditionally closed with only the live-draft evidence outstanding.

## 15. Verification

Run at minimum:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-github --locked
cargo package -p eggpack-github --locked --allow-dirty
cargo package -p eggpack-ci --locked --allow-dirty
cargo package -p eggpack-cli --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-github --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted Linux stable, Linux Rust 1.89, macOS, and Windows must pass.

No real GitHub draft is required to close M003c. Live qualification resumes only after corrective closure.

## 16. Compatibility review

Expected compatibility:

- DistributionContract unchanged
- ReleaseManifest unchanged
- M001 CIPlan unchanged
- staging-disabled M002a rendering remains byte-compatible
- M003a StagingPayload v1 remains compatible unless a provider-policy field is needed
- M003b staging graph remains finite

If `GitHubDraftPolicyV1` gains an asset-page bound, either provide a documented serde default for backward-compatible schema-v1 JSON or explicitly bump the provider-policy schema. Do not silently reinterpret serialized policy.

## 17. Security review

Explicitly verify:

- checked-out source equals ReleasePlan source revision
- all release-producing jobs use one resolved release ref
- tag_source cannot be ignored
- no branch build can be staged as another tag
- upload host uses parsed exact-host equality
- query names are encoded
- streamed body cannot redirect/retry to leak token/body
- token remains redacted
- pagination cannot hide unexpected remote state
- starter deletion remains narrow
- no publication/tag mutation is added
- only stage has contents: write

## 18. Acceptance criteria

M003c closes only when:

- dispatch builds/qualifies/finalizes from the exact requested tag source
- tag-push mode uses one exact tag source across all jobs
- checked-out HEAD is compared to ReleasePlan.source_revision before build
- StagingTagSource controls trigger/input/condition/checkout semantics
- no staging mode implicitly accepts the other mode
- production release assets are streamed with bounded memory
- production upload path contains no full-file std::fs::read / duplicate full Vec
- pre/post asset reconciliation covers all bounded pages
- pagination exhaustion fails closed
- upload host validation uses parsed exact-origin semantics
- asset query names are correctly encoded
- direct/bundle/archive staging regressions remain green
- all hosted lanes pass
- no unresolved medium-or-higher staging correctness/safety finding remains

Only after closure may M003b resume live-draft qualification.

## 19. Stop conditions

Stop and re-plan if:

- exact source verification requires moving build authority outside CI orchestration
- actions/checkout cannot provide one consistent ref without redesigning trigger semantics
- Eggfetch streaming cannot support authenticated known-length upload without redirects/retries
- safe streaming needs an upstream Eggfetch primitive not present in the reviewed baseline
- complete asset pagination cannot be proven within a bounded policy
- query/origin correctness requires narrowing DistributionContract filename semantics
- any fix adds publication/tag-mutation authority

If an Eggfetch upstream gap is discovered despite the reviewed streaming primitive, write a separate Eggfetch plan and block only the streaming portion rather than introducing a second HTTP stack.

## 20. Closure evidence

Create:

`plans/closure/ci-release-orchestration/003c-status.md`

Record:

- implementation SHA
- root-cause matrix for all findings
- RefName and DispatchInput rendered source-ref behavior
- source verifier results
- disposable Git A/B/tag test
- direct/bundle/archive goldens
- multi-chunk streaming evidence
- proof no production full-file buffering remains
- pagination tests beyond 100 assets
- pagination bound-exhaustion result
- exact upload-host negative matrix
- encoded asset-name matrix
- M003a/M003b historical closure annotations
- hosted matrix
- unresolved findings
- explicit M003b live-draft readiness disposition
