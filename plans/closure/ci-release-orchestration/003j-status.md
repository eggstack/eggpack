# CI M003j Closure — Dispatch-Tag Shell Injection Corrective

Status: **closed**

Source plan: `plans/implementation/ci-release-orchestration/003j-dispatch-tag-shell-injection-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed Eggpack baseline: `d61ca71fc0112be63e7e8ba31ba8fa2b1ce5a628`.

Implementation commit and immutable producer pin: `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`.

Reviewed consumer configurations: wg-basic `0b814fe02b5e09654e9363c8f61caf59b25ecc92` (R001 status/registry/roadmap-only update; workflow/config unchanged), Eggsearch `2e876e46724c02c502d78bf968e74a1ad6affb71`, Eggsact `d4e6e5c5cd7e9665373fbe312f4e1fb122fb5729`, and StegoEggo `a01a0222c6a5022a9eb3561f153b1e72b6e6c42a`. The source plan's originally confirmed wg-basic security-review baseline was `dbowm91/wg-basic@ff40f851508ccda52171c27cefdcdaf4ec1da2d5`. No consumer workflow or release-policy configuration was changed.

Hosted CI: [run 37987890305](https://github.com/eggstack/eggpack/actions/runs/37987890305), push, attempt 1, `head_sha` exactly `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`, conclusion `success`. Linux stable, Linux 1.89.0, macOS, and Windows portability all passed.

## Executive finding

CI M003j is closed. Workflow-dispatch release tags are validated as bounded canonical stable `vX.Y.Z` data before the reusable resolver or write-scoped staging job checks out the selected source. The generated Bash reads the value through the quoted `EGGPACK_RELEASE_TAG` environment variable. The reusable resolver now uses this data path for both mapped and legacy exact-tag identity modes; the legacy mode previously placed the dispatch expression directly in the `_resolve-release` shell argument. A renderer guard rejects dispatch-tag expressions in any generated `run:` script while permitting data use in action inputs such as checkout `with.ref` and in concurrency keys.

No build/qualification authority, staging permission, manifest schema, publication authority, or signing trust decision changed. No real dispatch, draft creation, signing, or publication occurred.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Remove executable interpolation in reusable resolver and stage validation | `dispatch_tag_validation_step` emits an `env` binding and fixed Bash validator; resolve and stage branches place it before checkout. `m003c_tag_source_controls_checkout_input_and_stage_guard` asserts safe `run` bodies, environment binding, and validation-before-checkout ordering. | Pass |
| Route resolver CLI tag argument through data for legacy and mapped modes | Runtime `ResolveRelease` always uses the renderer-only `$release_tag` marker, which renders as the quoted `"$EGGPACK_RELEASE_TAG"`; resolver step binds the tag for both modes. `m003i_mapped_workflow_binds_tag_safely_and_preserves_event_mapping` remains green; M003j reusable render assertions cover dispatch mode. | Pass |
| Reject malformed or injected tags before source checkout | Validator caps input at 64 characters and accepts only canonical stable `vX.Y.Z`. Unix regression covers valid, empty, non-prefixed, leading-zero, prerelease, build suffix, newline, shell substitution, punctuation, and over-bound input. | Pass |
| Prove the test distinguishes the old vulnerable script | The controlled old-template Bash run creates a temporary marker from `$(touch ...)`; the rendered validator rejects the same fixture without creating the marker. No network or credentials are used. | Pass |
| Fail closed if unsafe dispatch expressions return to generated scripts | `contains_unsafe_dispatch_expression_in_run` rejects direct `inputs.release_tag` expressions in scalar and block `run:` forms. Mutation test inserts the old expression and verifies the guard detects it and `check_release_github` reports drift. | Pass |
| Preserve ref-name, mapped identity, staging scope, and release schema | RefName stays push/tag-only; DispatchInput remains dispatch-only; exact-source verification, M003i mapping, permissions and handoff remain unchanged. No schema or CLI document shape changed. Existing M003c/M003i regressions and the full workspace suite pass. | Pass |
| Qualify the exact implementation on required hosted platforms | Hosted run 37987890305 is green on Stable Linux, Rust 1.89.0 Linux, macOS, and Windows. | Pass |
| Re-render the affected consumer configuration without modifying it | From wg-basic's current checked-in workflow shape, contract, and GitHub policy: generation produced 33,370 bytes and `ci check: match`. Diff is exactly the two validation steps: each now has a bounded environment-based validator moved ahead of checkout. No policy key changed; the policy's pinned Eggpack revision remains consumer-owned and must be repinned by its plan. | Pass |
| Identify other adopted DispatchInput consumers | Current Eggsact, StegoEggo, and Eggsearch policies all select `dispatch_input`; their checked-in `release-binaries.yml` / `release-eggpack.yml` contain direct executable interpolation. Fresh render sizes were 56,534 / 56,596 / 76,328 bytes and self-check passed; `ci check` against each checked-in workflow correctly reports drift. Generated diffs replace both validation snippets and, in legacy exact-identity mode, replace the direct `_resolve-release --tag '${{ inputs.release_tag }}'` argument with `"$EGGPACK_RELEASE_TAG"`. | Confirmed; consumer remediation remains open |

## Production implementation evidence

- `crates/eggpack-ci/src/lib.rs`: one canonical bounded Bash validator is rendered into reusable `resolve` and `stage`; both run it before checkout. `ResolveRelease` takes the tag from a quoted environment reference in every identity mode.
- The generator scans emitted `run:` scalar and block forms for direct dispatch-tag interpolation and returns a bounded render error if one appears. Dispatch expressions remain permitted in non-executable action inputs and concurrency metadata.
- `crates/eggpack-ci/README.md`, `architecture/ci-rendering.md`, `architecture/ci.md`, and `architecture/overview.md` describe the new behavior and current source line counts/relevant anchors.
- Golden workflows for RefName staging are byte-identical; no golden fixture update was needed because the affected DispatchInput reusable flow is exercised from its actual parsed generated YAML in tests.

Before/after consumer render: the two old `run: "test -n \"${{ inputs.release_tag }}\""` steps are replaced by a multiline validator using `EGGPACK_RELEASE_TAG`; the new steps precede checkout in both jobs. For legacy exact-identity configurations, `_resolve-release --tag '${{ inputs.release_tag }}'` also becomes `_resolve-release --tag "$EGGPACK_RELEASE_TAG"` with the expression moved into `env`. Other generated workflow sections and consumer policy keys are unchanged by the rerender.

## Exact verification executed

| Command/evidence | Result |
|---|---|
| `cargo test -p eggpack-ci --locked m003c_tag_source_controls_checkout_input_and_stage_guard -- --nocapture` | Passed |
| `cargo test -p eggpack-ci --locked m003j_dispatch_tag_validation_treats_values_as_data -- --nocapture` | Passed; controlled shell-substitution marker negative proved |
| `cargo test -p eggpack-ci --locked m003i_mapped_workflow_binds_tag_safely_and_preserves_event_mapping -- --nocapture` | Passed |
| `cargo run -p eggpack-cli --locked -- ci generate --workflow-shape ../wg-basic/release/eggpack/workflow-shape.json --contract ../wg-basic/release/eggpack/distribution.toml --github-policy ../wg-basic/release/eggpack/github-policy.json --output /tmp/eggpack-wg-basic-m003j.yml` | Generated 33,370 bytes |
| `cargo run -p eggpack-cli --locked -- ci check --workflow-shape ../wg-basic/release/eggpack/workflow-shape.json --contract ../wg-basic/release/eggpack/distribution.toml --github-policy ../wg-basic/release/eggpack/github-policy.json --workflow /tmp/eggpack-wg-basic-m003j.yml` | `ci check: match (33370 bytes)` |
| Generate and self-check Eggsact, StegoEggo, and Eggsearch reusable outputs | Passed at 56,534 / 56,596 / 76,328 bytes. Against the checked-in workflows, `ci check` reported expected drift (55,901 / 55,963 / 75,695 actual bytes) pending consumer repin/regeneration. |
| `scripts/check-local.sh` | Passed: format, check, clippy, per-crate tests, workspace tests, docs, dependency trees, package verification, and Rust 1.89.0 check/tests |
| Hosted [run 37987890305](https://github.com/eggstack/eggpack/actions/runs/37987890305) | Passed on exact implementation SHA across Linux stable, Linux 1.89.0, macOS, Windows |
| `git diff --check` | Passed |

The package phase printed the pre-existing registry warning that locked `yoke-derive v0.8.3` is yanked; every package verification succeeded. No lockfile or dependency was changed.

## Invariant review

- **Data/code separation:** dispatch text is passed only as environment data and a quoted variable argument; the finite validator does not evaluate or re-expand it.
- **Validation order:** `resolve` and privileged `stage` validate before checkout. The earlier preflight checkout remains a pinned action input followed only by `cargo --version`; no checked-out workflow source executes there. The actual build graph depends on successful `resolve`.
- **Identity/source authority:** exact-source SHA verification and M003i tag/manifest mapping remain authoritative; canonical tag syntax does not authorize a source revision.
- **Privilege:** only stage retains `contents: write`, after aggregate. No signing secret, OIDC permission, publication step, or new network action was added.
- **Determinism:** canonical helper output is fixed; workflow generation remains byte-stable and consumer regeneration has zero drift.
- **Release evidence:** ReleaseManifest v1 and final-byte meaning are unchanged; no signature or provenance claim was added.

Nothing was weakened to reach this milestone.

## Failure, recovery, compatibility, and security review

Invalid or oversized dispatch values fail before the resolver or stage checks out the requested tag. A failure leaves no build or draft mutation. A valid value proceeds through the existing exact-source checks. Reruns remain concurrency-serialized and staging remains draft-only/no-clobber.

This changes generated reusable-workflow bytes and requires consumers to repin the immutable producer commit and regenerate their checked-in workflow under their own plan. Exact-tag workflow identities, RefName push behavior, `StagingTagSource`, CLI interfaces, manifest compatibility, and existing handoff documents remain compatible. No consumer workflow or release-policy configuration was edited; wg-basic received only the plan/registry status update cited below.

Security review found no residual Eggpack dispatch-tag-to-shell path in generated `run:` text. The tag remains visible in YAML data positions (`with.ref`, environment binding, and concurrency group) by design. No live attack, dispatch, credentials, or privileged staging was attempted; only a harmless local marker fixture was executed.

## Documentation and operations

Producer pin for wg-basic Release Readiness R001 and other affected consumers: `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`. Each consumer plan must update its Eggpack tool revision, regenerate its checked-in workflow, and require its own `eggpack ci check` before signing or staging. No consumer key/config changed here.

## Unresolved findings

No unresolved producer finding remains. **High residual consumer finding:** wg-basic, Eggsact, StegoEggo, and Eggsearch still have checked-in dispatch workflows that embed `inputs.release_tag` in executable Bash. Their current configs were positively confirmed as `DispatchInput`; until each owner repins and regenerates through its own plan, those checked-in workflows remain exposed to this injection class. wg-basic Release Readiness R001 is now **ready** after a status-only update at `dbowm91/wg-basic@0b814fe02b5e09654e9363c8f61caf59b25ecc92`; workflow regeneration, provenance proof, and strict closure remain outstanding. Eggpack cannot close or silently edit the other external consumer plans.

## Roadmap disposition and registry updates

- CI M003j: `ready` → **closed**, this closure is its evidence record.
- The known downstream blocker is discharged: wg-basic Release Readiness R001 may resume using the immutable producer pin above. This is eligibility only; its implementation and closure remain consumer-owned.
- wg-basic updated R001's status to `ready` through its own plan/registry/roadmap in commit `0b814fe02b5e09654e9363c8f61caf59b25ecc92`; no consumer workflow, policy key, or production behavior changed in that commit.
- Eggsact, StegoEggo, and Eggsearch are also confirmed DispatchInput consumers with stale generated workflows. No corresponding corrective plan was found or changed in their repositories during this producer milestone; their maintainers need to register consumer-owned repin/regenerate work before treating those workflows as remediated.
- No other Eggpack implementation plan is promoted by this closure. The registry has no remaining ready Eggpack plan; future Eggpack implementation requires new plan authoring against the then-current evidence.
- CI M003i's historical closure is unchanged.
