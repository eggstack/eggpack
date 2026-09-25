# CI and Release Orchestration Milestone 003b — Generated Draft Staging Job and Operational Qualification

Status: conditionally closed

Closure record: `plans/closure/ci-release-orchestration/003b-status.md`

Closure disposition (2026-09-25): M003b implementation is complete and locally qualified on fake-adapter evidence (conditional close; live draft outstanding). The final M003a CLI/provider contracts were re-reviewed against this plan with no interface change required (`_prepare-stage` / `_stage-github-draft` flag shapes match the rendered `RunnerCommand` argv). Generated `stage` job, permission matrix, drift detection, and local fake-GitHub orchestration (direct/bundle/archive, rerun, tag-mismatch, published/mismatch refusal) all pass. Full closure additionally requires a maintainer-authorized live draft fixture; Phase 8 exit is not satisfied and eggsact adoption remains blocked until then.

Unblocked by M003a closure (2026-09-25): `plans/closure/ci-release-orchestration/003a-status.md` (implementation `36f1cc1`, hosted run 36180399698 green). Final M003a CLI/provider contracts must be re-reviewed against this plan before handoff; full closure additionally requires a maintainer-authorized live draft fixture.

Repository baseline: `97db44513ab7e0083fa71c5ab03aa57d63fca1be`

Source roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Hard dependencies:

- M003a closure: `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md`;
- CI M002a closure: `plans/closure/ci-release-orchestration/002a-status.md`;
- ADR-0003: `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`.

Operational closure dependency:

- one maintainer-authorized repository/tag where a real GitHub draft can be assembled and inspected without publishing it;
- expected first candidate: eggsact, after its mirrored adoption/release-workflow handoff is registered.

Primary class: capability / generated CI / least-privilege staging / operational qualification

## 1. Objective

Extend the deterministic checked-in GitHub workflow so a fully qualified M002a aggregate can be staged as a GitHub **draft release** using the provider adapter delivered by M003a.

M003b is the Phase 8 orchestration slice:

```text
build -> qualify -> gate -> aggregate
                              |
                              v
                    prepare staging payload
                              |
                              v
                    stage GitHub draft release
                              |
                              X human publish only
```

Only the staging job receives GitHub write authority. Every prior job remains read-only.

## 2. Readiness rule

This plan is intentionally registered before it is dependency-ready so the full Phase 8 boundary is explicit.

Do not implement M003b until M003a closes and the final M003a CLI/provider contracts are re-reviewed against this plan.

If M003a materially changes its staging payload or CLI contract, update/re-review M003b before handoff.

## 3. Invariants

- generated workflow remains checked in and deterministic;
- all build/qualification/gate/aggregate jobs retain `contents: read`;
- exactly one staging job may have `contents: write`;
- no job receives `id-token: write`;
- token is passed through environment only;
- staging job can only call M003a draft-only commands;
- workflow never creates/moves/deletes tags;
- workflow never publishes a release;
- staging runs only after aggregate completion;
- staging consumes the exact aggregate artifact, not rebuilt/re-fetched release binaries;
- staging payload must include manifest and both generated installers;
- generated workflow cannot use `gh release ... --clobber`;
- reruns reconcile the existing draft through M003a semantics;
- published/immutable release state fails closed;
- CI check must detect any manual staging-permission or command drift.

## 4. Production changes

### A. Provider-neutral staging node

Extend the executable release graph with a bounded staging intent rather than embedding GitHub details directly in core build nodes.

Suggested additive types:

- `StagingIntent`;
- `StagingProvider::GitHubDraft`;
- `StagingJob`.

The provider-neutral graph records:

- aggregate dependency;
- staging provider;
- deterministic internal finalized handoff name;
- deterministic staging receipt artifact name;
- whether staging is required.

Do not mutate M001 `CIPlan` schema-v1 semantics.

### B. GitHub renderer policy

Extend GitHub provider policy with explicit staging settings only when staging is enabled.

At minimum:

- staging runner label;
- pinned download-artifact action;
- exact repository owner/name;
- exact tag input/ref mapping;
- paths for:
  - contract;
  - bootstrap spec/install policy;
  - GitHub draft policy;
- staging receipt retention.

The renderer validates that:

- GitHub staging policy exists when graph requests staging;
- staging runner is finite/safe;
- tag source is explicit;
- no publish flag exists.

### C. Generated stage job

Add one final job:

```yaml
stage:
  needs: aggregate
  permissions:
    contents: write
```

Steps:

1. checkout exact source/tag for local policy/config files;
2. install the immutable-pinned Eggpack CLI;
3. download the aggregate internal artifact;
4. run the M003a local stage-preparation command;
5. run the M003a GitHub draft staging command with `GITHUB_TOKEN` in environment;
6. upload the bounded staging receipt as an internal Actions artifact.

Do not use GitHub CLI for release mutation.

Do not invoke raw `curl` for release mutation.

### D. Exact aggregate handoff

The staging job must consume exactly:

```text
eggpack-finalized/
  root/
  summary.json
  release-manifest.json
```

The M003a preparation command revalidates this handoff before constructing the upload payload.

No build candidate, checksum, or installer is independently fetched from GitHub.

### E. Trigger/tag semantics

Support only explicit exact-tag staging.

For tag-push workflows:

- derive the candidate tag from `github.ref_name`;
- require ref type = tag;
- M003a still independently verifies the server-side tag -> source commit relationship.

For workflow_dispatch:

- require an explicit existing tag input;
- do not default to latest branch/head;
- checkout the exact tag before staging.

Do not support arbitrary branch staging as a release.

### F. Permissions

Static rule:

- workflow top-level: `contents: read`;
- preflight/build/qualify/gate/aggregate: `contents: read`;
- stage job only: `contents: write`;
- no other write scopes.

Tests must parse rendered YAML and prove this exact permission matrix.

### G. Concurrency

Retain release-scoped concurrency so two runs for the same ref/tag do not stage concurrently.

If current M002 concurrency key is not tag-specific enough for manual dispatch, extend it deterministically using the resolved staging tag.

Do not rely on concurrency as correctness; M003a remote reconciliation remains authoritative.

### H. Staging receipt/status

After success, the workflow publishes an internal Actions artifact containing only the M003a staging receipt.

The receipt allows maintainers to inspect:

- release ID;
- tag/source identity;
- draft/immutable state;
- uploaded/reused asset counts;
- exact asset digests.

Do not include token or raw API bodies.

## 5. Generated workflow tests

Add golden fixtures for:

- direct + staging;
- bundle + staging;
- archive + staging;
- staging disabled (M002a output remains byte-compatible except intentional schema/policy version changes).

Required static assertions:

- only stage has `contents: write`;
- no `id-token: write`;
- stage depends on aggregate;
- stage downloads exact finalized handoff;
- prepare command occurs before network staging command;
- token appears only as environment reference, never rendered literal;
- no `gh release`;
- no `--clobber`;
- no publish command;
- no tag mutation command;
- staging receipt uploaded;
- deterministic rerender;
- `ci check` catches staging step/permission edits.

## 6. Local executable orchestration

Extend the M002a local command harness so it can exercise:

```text
aggregate
 -> prepare-stage
 -> fake GitHub draft adapter
 -> staging receipt
```

The local harness uses the M003a loopback GitHub fixture and the same typed runner-command arguments rendered into YAML.

Test:

- direct;
- bundle;
- archive;
- rerun exact draft;
- tag mismatch;
- published release refusal;
- mismatched asset refusal.

## 7. Live operational qualification

Phase 8 cannot close on fake-server evidence alone.

Before final M003b closure, stage one real GitHub draft in a maintainer-authorized repository using a real existing tag whose commit exactly matches the staged ReleaseManifest source revision.

Preferred first candidate after mirrored planning: eggsact.

Operational qualification must prove:

1. generated workflow/check-in is current;
2. build/qualification/aggregate complete;
3. stage job alone has `contents: write`;
4. exact existing tag/source preflight passes;
5. complete artifact/checksum/manifest/installer set appears in draft;
6. release remains draft after job success;
7. human can inspect it before publication;
8. rerun reuses exact draft/assets without clobber;
9. public publication is not performed by Eggpack.

If a suitable authorized live fixture is not available, M003b may implement but must remain `conditionally closed` or `closing`; Phase 8 exit is not satisfied.

Do not create a throwaway public release without explicit maintainer authorization.

## 8. Eggsact operational fixture boundary

Current reviewed eggsact baseline:

`eggstack/eggsact@43971e7c1af7f936acfd876f9bff246e72866f2d`.

Its existing release workflow already:

- requires an existing exact version tag;
- builds five native/cross targets;
- verifies/smokes/checksums binaries;
- creates/reuses a draft GitHub Release;
- uploads installers;
- leaves publication manual.

This makes eggsact a strong Phase 8/9 operational fixture, but M003b must not silently migrate it.

Before using eggsact for the live qualification:

- register the corresponding eggsact distribution/release corrective/adoption plan in eggsact;
- preserve crates.io-first/tag-after-publish ordering;
- preserve product-owned Cargo fallback/update semantics;
- do not remove predecessor release workflow until generated parity is proven.

## 9. Security review

Explicitly audit:

- YAML permission matrix;
- token environment-only handling;
- fork/PR trigger exclusion for write job;
- tag/ref injection;
- shell quoting;
- immutable action/Eggpack pins;
- Actions artifact trust boundary;
- staging receipt secret leakage;
- workflow dispatch tag validation;
- rerun/concurrency behavior;
- published/immutable refusal;
- no release-publish endpoint/command.

The staging job must never execute on untrusted pull-request code with write authority.

## 10. Failure/restart semantics

If aggregate fails: stage never runs.

If prepare-stage fails: no GitHub mutation.

If draft creation/upload partially succeeds: M003a leaves draft for inspection and a later explicit rerun reconciles exact state.

If staging fails:

- workflow is red;
- already-created draft remains draft;
- no automatic publication;
- no broad cleanup/deletion.

If receipt upload fails after remote staging success:

- workflow fails;
- remote draft remains;
- rerun re-verifies/reuses exact draft.

## 11. Verification commands

After M003a closes and M003b is implemented:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted Linux stable, Rust 1.89, macOS, and Windows must pass.

A live staging workflow run is required for full closure.

## 12. Documentation

Update:

- root README;
- `eggpack-ci` README;
- `eggpack-cli` README;
- `eggpack-github` README;
- CI/release orchestration roadmap;
- registry;
- closure `plans/closure/ci-release-orchestration/003b-status.md`.

Document operationally:

- stage creates/reuses draft only;
- public publish is a separate human action;
- write permission is isolated to stage;
- reruns are exact-state reconciliation, not clobber.

## 13. Acceptance criteria

M003b closes only when:

- generated stage job is deterministic;
- only stage has `contents: write`;
- no untrusted PR trigger can reach stage;
- stage consumes M002a aggregate output;
- payload includes all finalized assets, checksum sidecars, manifest, and both installers;
- exact tag/source verification occurs through M003a;
- real GitHub draft assembly succeeds;
- real draft remains unpublished after workflow;
- exact rerun succeeds without clobber;
- `ci check` detects permission/staging drift;
- no unresolved medium-or-higher staging finding remains.

On full closure, Phase 8 is satisfied and eggsact Phase 9 adoption may move from interface-blocked to a mirrored implementation handoff.

## 14. Stop conditions

Stop and re-plan if:

- stage job needs broader GitHub permissions;
- workflow must create/move tags;
- real draft staging requires publication;
- write job must run on PR/untrusted code;
- M003a reconciliation semantics prove insufficient;
- eggsact or another fixture requires product-specific release policy inside Eggpack.

## 15. Closure evidence

Record:

- implementation SHA;
- final M003a interface baseline;
- rendered staging goldens;
- parsed permission matrix;
- local fake-GitHub end-to-end staging results;
- live staging repository/tag/run;
- exact staged asset inventory/digests;
- rerun evidence;
- confirmation release remained draft;
- no publication/tag mutation;
- hosted matrix;
- unresolved findings;
- Phase 8 completion disposition;
- explicit Phase 9 eggsact adoption readiness disposition.
