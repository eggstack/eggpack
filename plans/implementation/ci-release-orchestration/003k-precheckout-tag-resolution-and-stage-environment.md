# CI Release Orchestration M003k — Pre-checkout Tag Resolution and Staging Approval Gate

Status: in progress. Producer baseline: `eggstack/eggpack@559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce` (M003j implementation). Downstream trigger/evidence: `dbowm91/wg-basic@bd7c085d191ccf147e0f4dab72fde7cc8818f3fd`; R001 requires canonical tag validation and immutable commit resolution before any source checkout, plus reviewer-gated staging.

Subsystem roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`. Canonical authority: ADR-0003; `plans/003-planning-process.md`; CI M003i/M003j closure records.

Primary class: invariant / security corrective.

## Objective

For generated release workflows, validate the selected tag and resolve it to one immutable commit before any release-source checkout. Bind checkouts and source-verification steps to that commit, reject moved or malformed refs, and allow product policy to attach a named GitHub Environment only to the write-authorized draft-staging job.

## Evidence and gap

M003j safely handles dispatch input and its tag validator, but the generated `preflight` checks out the dispatch ref before that validator runs in `resolve`. That first job currently only runs `cargo --version`, but it violates R001's required ordering. `GitHubStagingPolicyV1` has no environment field, so consumers cannot make the generated `stage` job wait for a configured reviewer. The downstream wg-basic workflow cannot close R001 until these producer contracts are qualified and pinned.

## Invariants and scope

- Every tag is validated as bounded data in an environment binding, never interpolated into executable script text.
- Resolve exactly one existing tag to a full commit OID without checking out source. Resolve annotated tags to their peeled commit; reject missing, ambiguous, malformed, or non-commit results.
- All subsequent source checkouts use the resolved OID. Recheck checked-out HEAD against that OID before any Eggpack/product command; moved refs and stale handoffs fail closed.
- The write-authorized `stage` job alone may reference a configured named GitHub Environment. Eggpack renders the reference but does not create/configure the environment or select reviewers.
- Preserve draft-only staging, same-run artifacts, read-only candidate jobs, source/manifest identity, no tag mutation, and no publication path.
- Keep the optional environment field backward compatible; add no ReleaseManifest or runtime schema change.

## Ordered work packages

1. Update exact and reusable renderers so their first job validates the tag and resolves its OID without checkout, exposes the bounded OID as an output, then checks out that OID. Propagate the OID to all source-consuming jobs and reject any checked-out HEAD mismatch before Eggpack runs.
2. Add an optional, validated staging environment name to provider policy and emit `environment.name` only on `stage`. Update fixtures and provide a downstream wg-basic config example.
3. Add generator tests for malformed/injection input, lightweight/annotated ref-resolution handling, pre-checkout order, SHA checkout/recheck, environment scoping, and compatibility when no environment is configured.
4. Update CI renderer architecture, roadmap and registry. Prove output with a downstream fixture and `ci check`.

## Failure and compatibility

Invalid tags, missing refs, resolution ambiguity/failure, invalid OIDs, or checkout mismatch stop before release commands or draft mutation. Failed approval leaves the stage job pending/cancelled without staging. Absence of the environment preserves existing provider policy compatibility; rendered workflow changes are limited to safe resolution/checkouts and optional environment assignment.

## Verification

Run focused `eggpack-ci` tests; `cargo fmt --all -- --check`; `cargo test --workspace --all-targets --all-features --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `scripts/check-local.sh`; and hosted Linux stable/MSRV/macOS/Windows CI. Regenerate wg-basic workflow at the new immutable producer revision and run its release-contract, native candidate, and systemd lifecycle lanes. Do not stage or publish any release. Local `scripts/check-local.sh` passed on 2026-10-10; hosted qualification and downstream evidence are pending.

## Acceptance and closure

No release job checks out a dispatch-selected ref before validating and resolving it; all source-consuming jobs use the frozen commit and reject a mismatch; adversarial input never reaches executable workflow text; only `stage` carries an environment; the no-environment form remains valid; and existing least-privilege/draft-only checks pass. Closure records exact SHAs, hosted runs and downstream drift evidence, with environment reviewer identity/configuration explicitly listed as consumer-owned operations.

## Stop conditions

Stop if the tag cannot be resolved without checking out source, safe output propagation requires interpolation, workflow write scope broadens, or environment assignment cannot be limited to `stage`. Do not hand-edit consumer YAML or claim downstream closure without exact-head hosted checks and settings review.
