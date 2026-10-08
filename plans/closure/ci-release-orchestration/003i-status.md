# CI M003i Closure — Explicit Tag and Manifest Release Identity

Status: **closed**

Source plan: `plans/implementation/ci-release-orchestration/003i-explicit-tag-and-manifest-release-identity-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed Eggpack baseline: `3ef806fedc9d7683948e5678ec0d3c7b78c06f0e`.

External consumer baseline reviewed: `dbowm91/wg-basic@125a6a7975a36c65f9b380d8a00cda3604a9241e`.

Implementation commits: `7c20b68ac1b72b44bff0799b56674728ef5a6a0c` (implementation), `1b763a34c326f8349ae88df50a3f45d546110a60` (wg-basic-shaped consumer fixture), and `3ddac9d84fc2e10b8451d38672f66b02e06724df` (explicitly versioned runtime identity envelope). Final immutable implementation SHA: `3ddac9d84fc2e10b8451d38672f66b02e06724df`.

Hosted CI: [run 37813044035](https://github.com/eggstack/eggpack/actions/runs/37813044035), push, attempt 1, `head_sha` exactly `3ddac9d84fc2e10b8451d38672f66b02e06724df`, conclusion `success`.

## Executive finding

CI M003i is closed. Reusable release workflows now have a finite checked-in opt-in mapping from exact source tag `vMAJOR.MINOR.PATCH` to manifest release ID `MAJOR.MINOR.PATCH`. Absence of the policy field retains exact-tag behavior and existing workflow/policy serialization. Mapped runtime policy carries an explicitly versioned identity envelope (currently version 1) with the mapped mode, exact tag, manifest ID, and exact source revision; each mapped-mode job checks the event-selected tag, policy, release plan, checkout HEAD, and local tag peel before release work. Finalization writes the mapped ID into contract-expanded filenames and the manifest, while installers and GitHub draft reconciliation keep the exact source tag.

The public `ReleaseManifest v1` format, Eggup behavior, product policy, and publishing authority did not change. No signature, provenance, or authenticity claim was added.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Finite stable-tag mapping; exact mode remains default | `ReleaseIdentityMode` and strict mapping tests in `eggpack-ci` and `eggpack-github`; omitted `GitHubPolicy.release_identity_mode` remains exact-tag mode. | Pass |
| Reject malformed, injected, overflowing, prerelease/build, and Unicode-lookalike tags before writes | CI/GitHub resolver matrix and CLI resolver tests; rejected mappings create no runtime output. Exact tag is passed through an environment variable in generated shell, and the mode is a renderer-supplied enum literal. | Pass |
| Preserve dispatch-input and push/ref-name behavior | `m003i_mapped_workflow_binds_tag_safely_and_preserves_event_mapping` exercises both valid pairings and rejects mismatched trigger/tag-source combinations. | Pass |
| Keep a versioned identity tuple bound across runner processes | Every mapped reusable release job downloads the runtime plan and draft policy and verifies event tag, mapped plan ID, source revision, `HEAD`, and local tag peel. Draft policy requires envelope version 1; tests reject an absent or unknown envelope version. Runner verification is omitted in legacy exact mode, preserving old workflow bytes. | Pass |
| Produce mapped final bytes and stage under exact tag | `m003i_mapped_identity_survives_two_target_capture_qualification_finalize_and_stage` runs typed CLI commands through resolution, two structural qualifications, gate, finalization, manifest parse, installer generation, fake staging, and exact-byte rerun reuse. It asserts tag `v1.2.3`, manifest ID `1.2.3`, matching revision, and two targets. | Pass |
| Match wg-basic M001 contract and Eggup projection | The integration fixture uses product/install ID `wg-basic` and versionless target asset names. Its generated manifest was parsed by `eggpack-manifest 0.1.0` and projected for both Linux targets using `eggup-eggpack 0.1.3`; both yielded product `wg-basic`, release `1.2.3`, the expected target asset, and install name. The existing wg-basic M001 signed-manifest fixture test also passed. | Pass |
| Preserve draft safety and byte inventory | GitHub staging tests confirm exact-tag origin, mapped manifest ID/revision checks, exact assets, and idempotent reuse; existing no-clobber, immutable/published-release refusal, digest, size, and inventory tests remain green. | Pass |
| Preserve legacy workflows and public schemas | Existing exact-tag workflow goldens and old draft-policy serialization tests pass. No `ReleaseManifest v1` or Eggup schema changes. | Pass |

## Production implementation evidence

- `eggpack-ci` owns the checked-in identity mode, stable tag grammar, reusable workflow policy validation, literal resolver argument, safe event-tag environment handoff, and per-job source/policy verification step.
- `eggpack-core` exposes bounded local tag peeling through the existing sanitized Git process runner. It rejects invalid Git ref syntax and compares the peeled commit exactly without returning captured output.
- `eggpack-cli` resolves the mapped manifest ID independently from the exact source tag, emits a mapped GitHub draft policy, and checks the tuple before runtime writes and later release stages.
- `eggpack-github` carries optional mapped mode/ID/revision fields, omitted in exact mode. It compares those facts with the finalized manifest before materializing exact-tag staging assets.
- Documentation and architecture notes describe the policy field, grammar, trigger pairing, runner checks, and downstream boundary.

The added two-target pipeline fixture uses structural qualification. It demonstrates producer identity continuity and final-byte/staging behavior, not host-native execution or any declared OS ABI floor.

## Exact verification executed

Local commands on the final implementation tree:

```text
cargo fmt --all -- --check                                           passed
cargo check --workspace --all-targets --locked                       passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked        61 passed, 3 ignored
cargo test -p eggpack-cli --all-targets --all-features --locked       35 passed
cargo test -p eggpack-github --all-targets --all-features --locked    37 passed
cargo test -p eggpack-core --all-targets --all-features --locked      47 passed, 6 ignored
cargo test --workspace --all-targets --all-features --locked          261 passed, 9 ignored
cargo doc --workspace --no-deps --locked                              passed
cargo +1.89.0 check --workspace --all-targets --locked                passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                passed
cargo +1.89.0 test -p eggpack-cli --all-targets --locked               passed
./scripts/check-local.sh                                              passed (exit 0)
git diff --check                                                      passed
```

The independent scratch projection used the wg-basic-pinned `eggpack-manifest 0.1.0` and `eggup-eggpack 0.1.3` crates against the manifest emitted by the end-to-end fixture; it did not edit the wg-basic checkout.

Hosted run 37813044035 on the final implementation SHA:

| Lane | Result |
|---|---|
| `linux (stable)` | Passed format, workspace check/tests, Clippy, and docs. |
| `linux (1.89.0)` | Passed workspace check and tests. |
| `portability (macos-latest)` | Passed workspace check and tests. |
| `portability (windows-latest)` | Passed MSVC setup, focused builder/process tests, Windows CI tests, `pwsh 7` + `tar.exe` prerequisite, focused PowerShell archive runtime, focused orchestration checks, workspace check, and workspace tests. |

Run `37809876382` also passed on the immediately preceding implementation commit, but the closure relies on the final-SHA run above.

## Invariant review

The checked-in producer policy remains the identity authority; no arbitrary dispatch mode, heuristic prefix stripping, or version discovery was added. `ReleasePlan` remains intent, `ReleaseManifest v1` remains final-byte evidence, and Eggup receipts remain installed state. Draft staging remains on the exact existing tag, exact bytes are inventoried, and publication remains a separate human action. SHA-256 and sizes are integrity facts only. No tag creation/movement, `--clobber`, signing material, OIDC permission, or auto-publish path was added. Nothing was weakened to obtain green results.

## Failure and recovery review

All malformed identity cases fail before runtime plan/policy output. Git command failures, unsafe ref names, wrong tag peels, event-tag mismatch, source revision mismatch, policy tampering, and manifest tuple mismatch fail closed with bounded diagnostics. A failed check cannot create or modify a published release or overwrite a mismatched draft asset. The fake-provider rerun reuses every exact asset. The only local investigation failure was an initially invalid two-target test fixture: both Linux x86_64 targets projected to the same installer runtime pair. The fixture was corrected to distinct target architectures and later aligned to wg-basic's actual target asset/install contract; no production validation was relaxed.

## Compatibility and migration review

Existing exact-tag consumers need no migration. The optional `GitHubPolicy` field is absent by default and skipped during serialization when absent. Exact-mode workflow goldens remain byte-identical. The new stable mapping is opt-in and only represents `vMAJOR.MINOR.PATCH`; it does not accept prerelease/build metadata or rewrite tags downstream. Published schemas and crates were not changed.

## Security review

The event tag is passed through a YAML environment variable rather than interpolated into shell source. Mapping mode and the chosen trigger/tag-source pair are checked at render time. Runtime checks bind the selected tag, static policy, release-plan ID, commit, and local tag peel in every job. Git execution remains bounded, process-grouped, environment-cleared, and redacted. Staging retains exact-tag origin and existing no-clobber reconciliation. No authenticity is claimed from checksums.

## Documentation and operations evidence

Updated `docs/release-identity-policy.md`, the root/docs indices, and the applicable architecture deep dives for core planning, CI rendering, CI consumer handoff, CLI verification, and GitHub draft identity. The policy spelling is `"release_identity_mode": "v_prefixed_stable_semver"`; configure exactly `RefName` + push or `DispatchInput` + workflow_dispatch. Architecture citations for changed runtime entry points were checked by symbol. Historical unrelated citations explicitly documented as indicative remain outside this corrective's touched claims.

## Unresolved findings

No M003i implementation findings remain. Product-native qualification and declared platform-floor proof are downstream wg-basic M003 obligations. No live GitHub draft, production credential, signature, provenance claim, or public release was exercised or required. Cargo packaging emitted the existing informational warning that locked `yoke-derive 0.8.3` is yanked; packaging and verification succeeded and no dependency update was made.

## Roadmap disposition and dependency transitions

| Milestone | Disposition |
|---|---|
| Bootstrap M002b | Closed on `88ddf2e796ac3674b306b51726c65ed7cc9bd29a`, hosted run `37806244313`; strict Windows portability dependency satisfied. |
| CI M003i | Closed on `3ddac9d84fc2e10b8451d38672f66b02e06724df`, hosted run `37813044035`. |
| wg-basic Distribution M003 | Eggpack producer prerequisite is now satisfied and the external plan may resume with this immutable SHA. Its implementation, native qualification, and closure remain owned by wg-basic and were not changed here. |
| Other Eggpack plans | No additional implementation plan became ready. With M003i closed, the next Eggpack action is plan authoring when a new dependency-ready line is identified. |

## Registry updates

The CI roadmap and `plans/registry.md` mark M003i closed, link this closure, and remove it from the ready queue. The cross-repository handoff records the new immutable producer pin and states that wg-basic M003 can resume, while leaving its repository and status record untouched. Earlier closure records remain unchanged.
