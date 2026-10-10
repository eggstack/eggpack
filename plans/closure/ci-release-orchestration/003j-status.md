# CI Release Orchestration M003j — Dispatch Tag Injection Corrective Closure

Status: **closed for its registered shell-injection contract**. The separately identified pre-checkout ordering and stage-environment requirements are registered in M003k and remain open.

Source plan: `plans/implementation/ci-release-orchestration/003j-dispatch-tag-shell-injection-corrective.md`.
Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`.
Reviewed baseline: `eggstack/eggpack@d61ca71fc0112be63e7e8ba31ba8fa2b1ce5a628`.
Implementation: `eggstack/eggpack@559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`.
Hosted verification: [CI run 37987890305](https://github.com/eggstack/eggpack/actions/runs/37987890305), exact implementation SHA, conclusion success.

## Finding and result

Before the corrective, reusable resolver and write-authorized draft stage rendered `inputs.release_tag` directly into executable Bash. M003j routes dispatch input through an environment variable, validates bounded canonical stable tags as data, and adds generator/source guards plus a command-substitution negative test. This closes the registered injection finding without adding any release mutation, signing, or publication path.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Dispatch text is absent from executable shell bodies | Rendered workflow static guard and source test in `crates/eggpack-ci/src/lib.rs`; downstream guard is repeated by wg-basic R001 | Pass |
| Canonical stable tag is validated as data | Environment-mediated tag validator accepts canonical `vX.Y.Z`, rejects malformed/oversized/injection-shaped values | Pass |
| Regression discriminates the historical flaw | M003j fixture demonstrates the old command-substitution marker executes and the corrected validator does not | Pass |
| Stage permission remains least privilege | Renderer tests retain `contents: write` only on the gated stage job; no publish path or signing secret added | Pass |
| M003i identity behavior remains compatible | Stable tag to manifest release-ID mapping remains finite and separately validated | Pass |
| Hosted platform qualification | Linux stable, Linux 1.89.0, macOS, and Windows jobs all succeeded at the exact implementation SHA in run `37987890305` | Pass |

## Verification actually observed

- `eggpack-ci` generator regression and workspace verification ran in hosted CI run `37987890305`; all four jobs passed at `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`.
- The run's job matrix was `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, and `portability (windows-latest)`; each concluded success.
- The consumer pin and generated workflow were inspected at the downstream wg-basic checkout. wg-basic added its own workflow security regression and hosted CI, but downstream integration is not claimed as part of this producer closure.
- No release tag, draft, signature, or publication was created.

## Security, failure, and compatibility review

The fix blocks shell interpretation of tag data before resolve/stage commands consume it. Existing source, tag, manifest, read-only job, write-scoped stage, and draft-only contracts remain in force. No dependency, manifest schema, CLI contract, privilege, or publication authority changed. Invalid input fails the workflow before draft mutation. No high/medium finding remains open for M003j's specific injection contract.

## Known limitation and handoff

The first `preflight` checkout still precedes the reusable resolver's stable-tag validation. That job currently runs only `cargo --version` and executes no checked-out source, but this does not satisfy the downstream R001 ordering requirement to validate and resolve a tag to an immutable commit before checkout. The staging policy also cannot yet emit a GitHub Environment reviewer gate. Both are separately scoped to M003k; this record does not claim those requirements are closed.

## Roadmap and registry disposition

M003j is closed. M003k is ready to resolve the remaining pre-checkout ordering and environment integration gaps. The downstream wg-basic R001 remains blocked until M003k is implemented and qualified, the consumer repins/regenerates, and maintainer repository settings and reviewer configuration are verified.
