# CI M003i — Explicit Git Source Tag and Manifest Release Identity Corrective

Status: **ready** — Bootstrap M002b strict closure is recorded at `plans/closure/bootstrap-installers/002b-status.md` (implementation `88ddf2e796ac3674b306b51726c65ed7cc9bd29a`, hosted run `37806244313`, all four lanes green).
Repository baseline: `eggstack/eggpack@3ef806fedc9d7683948e5678ec0d3c7b78c06f0e`.
External consumer baseline: `dbowm91/wg-basic@125a6a7975a36c65f9b380d8a00cda3604a9241e`.
External blockage: `dbowm91/wg-basic/plans/closure/distribution/003-status.md` and `plans/implementation/distribution/003-eggpack-identity-seam-corrective.md`.
Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`.
Predecessors: CI M003c/M003d/M003g/M003h strict closures; ADR-0001/0002/0003.
Class: **producer capability and security invariant** (not signing or consumer deployment).

## 1. Why this exists

At this baseline `eggpack_ci::resolve_runtime_release_plan` feeds the exact tag to `PackConfig::resolve` as `release_id` and rejects `plan.release_id != tag`. `eggpack-cli::ci_resolve_release` then rejects `draft_policy.tag != plan.release_id`. CLI `resolve_release_emits_distinct_runtime_identity_per_tag` pins this legacy coupling. Therefore an Eggpack release from Git tag `v1.2.3` has manifest `release_id = v1.2.3`.

wg-basic Phase 10 M001 already requires signed manifest `release_id = 1.2.3` with Cargo `1.2.3`, while the immutable source and GitHub release tag remains `v1.2.3`. Its M003 stopped on this mismatch before a qualified pipeline or draft was produced. Changing the wg-basic workflow to strip the prefix after Eggpack resolution would invalidate producer identity continuity.

The foundation already supports separation: `eggpack_core::PackConfig::resolve` takes `release_id` independently of `source_revision`; `StagingPayloadV1` and `GitHubDraftReceiptV1` contain separate `tag` and `release_id`; `GitHubDraftTemplateV1` and the exact GitHub bootstrap origin are tag-based. The missing feature is an explicit static identity policy and validation across runtime stages.

**Objective:** safely bind the identity triple (`source_tag`, `source_revision`, `manifest release_id`) through producer resolution, build, qualification, finalization, staging and signed-consumer handoff. Preserve exact-tag semantics by default for all existing consumers.

## 2. Dependencies and ADR gate

Hard: strict Bootstrap M002b closure with green Windows, Linux and macOS hosted CI; the independent 2026-10-07 Windows compilation defect must not hide M003i regressions.

Interface (closed): CI M003c/M003d/M003g/M003h, current `eggpack-manifest 0.1.0`, and wg-basic's M001 consumer contract.

Operational: use real local Git fixtures and the existing fake GitHub provider for release staging. No live draft, production credential, signing key or public release needed for this Eggpack milestone.

ADR stop condition: if the implementation changes public `ReleaseManifest v1` wire format, Eggup's existing manifest projection, signing authority, or publication authority, stop and write a separately accepted ADR. This plan presumes existing public schemas remain untouched. Internal runtime handoff evolution is allowed when versioned, finite, atomic and qualified.

## 3. Explicit producer policy and identity invariants

Add a finite, checked-in opt-in release identity policy to the existing generated-workflow producer input, preferably the `GitHubPolicy` staging configuration (or a better provider-neutral checked-in equivalent with an explicit ownership rationale). Absence must mean the existing `exact_tag` behavior.

Policy alternatives:

| Mode | Git tag | Manifest ID | Compatibility |
|---|---|---|---|
| `exact_tag` (default) | opaque validated exact tag (e.g. `v1.2.7`) | identical to exact tag | identical historical output for eggsact/stegoeggo/eggsearch |
| `v_prefixed_stable_semver` (explicit opt-in) | exactly `vMAJOR.MINOR.PATCH` | exactly `MAJOR.MINOR.PATCH` | new wg-basic behavior only |

The opt-in grammar requires three nonnegative ASCII decimal components with no leading zeros (except single zero), and no pre-release/build metadata, suffixes, slash, whitespace, control characters or Unicode lookalikes. Reject missing `v`, double `v`, `v01.2.3`, `v1.2.3-rc1`, `v1.2.3+meta` and overflow. Validate both existing exact-tag safety and the narrow SemVer rule. Unknown fields/values fail closed. The release identity mode must come solely from source-controlled producer policy, **not** a new arbitrary runtime `workflow_dispatch` input, heuristic v stripping, regex/template mapping, or version discovery from GitHub.

Keep three separately typed/validated facts:
- exact preexisting `source_tag`: checkout, remote Git tag, draft, installer download origin;
- exact 40-hex `source_revision`: verified local HEAD and remote tag's peeled commit;
- `release_id`: product manifest ID, contract expansion, final artifacts, downstream version semantics.

The policy binds the three; matching two independently does not prove the third. Do not implicitly reconstruct the source tag downstream by prefixing `release_id`.

## 4. Implementation work packages

### WP1 — Pure bounded policy resolver

Add one typed validation/resolution helper that takes checked-in mode, exact tag and source revision and yields the triplet or one fail-closed error. Preserve legacy identity with absent policy; new mode uses stable SemVer mapping only. Add malformed-tag, overflow and injected-string negatives. Fail before writing any runtime plan/policy or creating candidate files.

### WP2 — Runtime CLI and generated workflow

Refactor `resolve_runtime_release_plan` to pass *mapped* manifest ID to `PackConfig::resolve` while retaining exact source tag independently. In `ci_resolve_release` resolve GitHub draft policy against the **tag**, not the manifest ID; replace `draft_policy.tag == plan.release_id` with explicit triplet/policy verification. Extend typed `RunnerCommand::ResolveRelease` and the `eggpack ci _resolve-release` grammar only as required. Emit literal policy from checked-in `ci generate`/`ci check` configuration into reusable workflow and runtime CLI. Preserve both tag-push/ref-name and explicit dispatch-input checkout/source-verification behavior; reject an unqualified combination at render time. Keep deterministic generation and one-writer draft policy.

### WP3 — Internal evidence continuity and compatibility

Prove the exact same identity triplet at every separately executed job: resolve, preflight, build handoff, qualification evidence, consumer validation, required gate, aggregation/finalization and staging. Select one reviewed mechanism: typed `source_tag` additions to existing internal versioned handoffs, or an immutable typed release-identity envelope/digest carried and checked by every job. The binding must be independently verifiable after serialization/deserialization across runner processes, not a local variable. Changing a tag or policy between jobs must fail before a stage upload.

Strict `serde(deny_unknown_fields)` means writers/readers must evolve atomically. Preserve legacy schema-v1 reads and legacy unchanged serialization/goldens in default mode. Where opt-in requires new internal representation, version it explicitly; reject missing/unknown identity fields rather than silently fall back to exact-tag mode. No unauthenticated handoff may overwrite an already verified tag or SHA.

### WP4 — Final bytes, installer and GitHub draft

For the opt-in case generate `ReleasePlan.release_id = 1.2.3`, all contract-expanded artifact filenames and final `release-manifest.json` using `1.2.3`. Manifest `source_revision` must match checked-out/remote verified `v1.2.3`. Generated exact installer URL origin, draft policy and remote release identifier remain `v1.2.3`. The staged payload/receipt must contain `tag: v1.2.3`, `release_id: 1.2.3`, the same revision, and byte-exact inventory. Never create/move a tag, publish, clobber mismatched assets, use latest, or dilute size/hash checks.

No Minisign private material, product trust-root management, signing or provenance claim belongs to Eggpack; wg-basic will sign independently and verify before using Eggup. Producer output can supply byte-exact unsigned signing-request evidence but cannot claim authenticity from SHA-256 alone.

### WP5 — Documentation and upstream/downstream boundary

Update only applicable `architecture/core-planning.md`, `ci.md`, `ci-consumer-seam.md`, `cli.md`, `github.md` and user/release-policy docs. Reconcile cited symbols/line numbers. Add a minimal checked-in opt-in configuration example. Do not modify wg-basic, Eggup or existing published crates/releases under this plan. Produce an immutable Eggpack revision and explicit config spelling for wg-basic's *separate* M003 plan to consume.

## 5. Acceptance tests and fixture matrix

**Legacy preservation:** exact-tag default with an omitted policy parses and renders exactly as before, including `v1.2.7` eggsact, `v0.5.0` stegoeggo and `v0.4.2` eggsearch producer shapes. Assert byte-for-byte golden workflow and handoff compatibility where historical exact outputs are guaranteed. The signed manifest v1 schema and `eggpack-manifest 0.1.0` remain unchanged.

**Successful mapped mode:** actual local Git fixture with commit A, tag `v1.2.3` at A, source checkout at A, runtime resolver, two Linux direct-target builds/qualifications, aggregate/finalization, manifest, installers and staging fake. Assert the tag, commit and `1.2.3` ID after every process boundary; checksum and artifacts must match final bytes. Independently validate the resulting manifest with wg-basic M001 fixture verifier/Eggup projection in scratch; any edits to wg-basic require prior mirrored plan registration. Prove a dry-run draft receipt records both identifiers, and an exact-byte rerun reuses draft assets without clobber.

**Fail-closed matrix:** invalid tag grammar, unknown mode, duplicate/missing fields, tag->commit mismatch, policy tampering after preflight, swapped/stale handoff, source commit changed after resolution, manifest ID changed after build, artifact-name or digest mismatch, wrong Cargo/product version (product validator), wrong/stale GitHub draft tag, existing published/immutable release, same-name/different-digest draft, partial upload. A different tag pointing at the same commit must not pass an identity check for the selected exact tag. No failed case creates a published release or modifies an immutable asset.

**Workflow tests:** generated reusable dispatch and tag-push flows retain exact checkout/source-verification on every stage; `ci check` detects source/tag/policy drift; no new write permission, OIDC claim, tag mutation or public publication path. Verify 1.89 MSRV and old consumers. Record actually executed pipeline tests, not just string match.

## 6. Order and exact commands

1. WP1 resolver + CLI parser tests; verify no output on invalid identity.
2. WP2 runtime and renderer tests in both modes, including policy injection negatives.
3. WP3 cross-stage versioning/continuity and tampering fixtures.
4. WP4 full fake-provider release and installer identity proof, and wg-basic consumer-shaped scratch conformance.
5. Regenerate/check workflow goldens and complete WP5 docs/architecture citation refresh.
6. Run local gates plus hosted four-platform CI on the *implementation commit*; only then close.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted CI: Linux stable, Linux Rust 1.89, macOS, Windows (the focused Bootstrap M002b Windows runtime included). Exercise *real* typed runner commands and local Git refs, not only YAML text. Source and target native qualification remain wg-basic M003's downstream obligation; Eggpack proves the producer identity capability via representative fixture.

## 7. Failure/retry, stop conditions and strict closure

Resolver errors happen before all writes, with bounded/redacted diagnostics. Existing private runtime storage, atomic writes, no-clobber draft reconciliation and release-scoped concurrency remain authoritative. Identical tag/source/mode rerun is idempotent; changed tag or mode never reuses an incompatible draft.

**Stop and re-plan** if implementing the mode requires a public `ReleaseManifest` wire-format change, invalidates previously shipped opaque-tag consumers, demands changes to wg-basic's signed verifier, invents tag identity from manifest ID, permits cross-source candidate mixing, requires product signing credentials or publication, or cannot pass M002b-qualified full CI.

Write `plans/closure/ci-release-orchestration/003i-status.md` only after full acceptance: immutable implementation SHA; exact policy/serialization diff; requirement-to-evidence matrix; legacy byte parity; positive and negative triple-validation runs; final manifest/artifact/installer/staging receipts; actual local/MSRV/Windows/Linux/macOS results and run IDs; severity/open findings; docs and downstream handoff. Update the CI roadmap and registry. Strict M003i closure **unblocks only the Eggpack prerequisite** for wg-basic M003; wg-basic M003, M004/M005 and maintainer production Minisign signing remain separately blocked/unproven until their own evidence.
