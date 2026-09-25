# CI and Release Orchestration Milestone 001 Closure — CIPlan and GitHub Renderer

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: `00a3399773045121baabbe86f062d41a4c2ce1fb` (Build/Qualification M002 closure; shared binding/command interface)

Implementation commits: `5e79f0b91b478f766124224c1e9c6382778fef14`, `69baab129963892505f1034a605669b83bb59a5a`.

Hosted CI: [run 36040032768](https://github.com/eggstack/eggpack/actions/runs/36040032768) passed Linux stable, Linux Rust 1.89, macOS, and Windows. The Windows job ran workspace checks and tests; the Linux stable lane ran formatting, check, tests, Clippy, and docs; the Rust 1.89 lane ran check, tests, Clippy, and docs. The previous run exposed CRLF checkout behavior in golden fixture comparisons; the final run passed with fixture comparisons normalizing CRLF to LF.

## Executive finding

The isolated `eggpack-ci` crate now projects canonical release intent and Build M002 source/command bindings into a provider-neutral CIPlan, then renders deterministic GitHub Actions workflows under an explicit bounded policy. Golden direct and mixed native/cross workflows parse with an independent YAML parser. Generated workflows use explicit read-only permissions, immutable policy-supplied action pins, bounded execution policy, and deterministic candidate handoff paths. Qualification remains unresolved graph intent; no qualification success, aggregation, release staging, or publication is claimed or executed. M001 acceptance criteria are met.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Isolated generator crate with minimal surface | `crates/eggpack-ci` depends on contract/core and serialization/hash support; YAML parser is dev-only. No GitHub client, async runtime, release API client, or Eggup dependency is introduced. |
| Provider-neutral, canonical CI graph | Versioned CIPlan/domain validation projects ReleasePlan jobs in canonical order without runner labels, action references, YAML fragments, secrets, or publication operations. |
| Reuse Build M002 bindings and command intent | Projection validates `BuildBindingsV1` and delegates command argument construction to `eggpack_core::cargo_command`; no parallel source schema or command renderer is introduced. |
| Explicit runner and action policy | Bounded GitHub policy validates runner mapping/capability declarations, finite triggers, timeout/concurrency/retention bounds, and full-SHA action references. |
| Deterministic, safe GitHub output | Renderer emits stable YAML with explicit Bash, read-only permissions, per-build private target directories, immutable caller-provided action pins, and exact candidate upload paths. No arbitrary YAML/command input, unpinned installation, secrets, write permission, or publication step is rendered. |
| Qualification and aggregation boundary | CIPlan preserves qualification classification as unresolved intent; rendered workflow does not claim qualification or release completeness. Aggregation remains a future boundary. |
| Handoff and artifact metadata | Canonical logical identity determines collision-safe internal handoff names; direct, bundle, and archive projection metadata are covered. |
| Drift check and parseability | Pure drift comparison normalizes newline conventions and detects byte edits; independent YAML parsing validates direct and mixed native/cross golden fixtures. |
| Stable/MSRV/package/docs/host portability | Full local `scripts/check-local.sh` passed before the CRLF-only assertion helper adjustment; focused all-target/all-feature locked CI crate tests, Clippy, and diff checks passed after it. `cargo package -p eggpack-ci` with local path patches passed. Hosted run 36040032768 passed all four matrix lanes, including Linux stable and Rust 1.89 check/test/Clippy/docs, macOS and Windows check/test. |

## Production implementation evidence

`crates/eggpack-ci` contains schema-v1 CIPlan graph/domain and validation, ReleasePlan projection, bounded GitHub policy, deterministic renderer, drift comparison, and tests. `crates/eggpack-ci/README.md` documents its boundaries and the root README describes the generator. `scripts/check-local.sh`, workspace membership, and Cargo.lock include the crate. Golden fixtures cover direct single-target and mixed native/cross multi-target plans. The two implementation commits are listed above; the Windows line-ending correction changes test comparison only.

## Verification executed

- `./scripts/check-local.sh` — passed on the implementation before the final test-only line-ending normalization.
- `cargo test -p eggpack-ci --all-targets --all-features --locked` — passed after the normalization.
- `cargo clippy -p eggpack-ci --all-targets --all-features --locked -- -D warnings` — passed after the normalization.
- `cargo package -p eggpack-ci --locked --allow-dirty` using local path patches — passed.
- `git diff --check` — passed.
- Hosted [run 36040032768](https://github.com/eggstack/eggpack/actions/runs/36040032768) — Linux stable, Linux 1.89, macOS, Windows all passed.

## Invariant, failure/recovery, and security review

Projection and rendering are pure; invalid plans or policy fail before producing a successful workflow. Check mode does not rewrite input. Permission output remains read-only (`contents: read`) with no write or id-token grant. Action SHAs and runner labels are caller policy, validated before rendering. Artifact uploads contain internal candidate outputs only. The CI crate itself does not launch processes or access the network; core is linked to reuse M002 command construction but no builder execution is invoked. A caller must supply valid production action SHAs and truthful runner capabilities, including preinstalled cross-build tools where declared. Recovery from drift is to regenerate and review the checked-in workflow; policy validation errors require correcting policy or plan inputs.

## Compatibility/migration review

This adds internal CIPlan v1 and a GitHub renderer without claiming a stable public format. Existing consumer workflows remain authoritative until separate adoption closures prove parity. No DistributionContract or ReleaseManifest format changed. CLI wiring, qualification execution, aggregation, staging, and publication remain outside M001; no migration is required.

## Documentation/operations evidence

Root README and `crates/eggpack-ci/README.md` describe projection, rendering, candidate handoff, and the later qualification/aggregation/staging boundaries. `scripts/check-local.sh` now covers the crate. CI roadmap and registry record this closure and downstream dependency disposition.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher CI generation, determinism, or security finding. | M001 acceptance criteria pass. |
| Informational | The renderer does not execute `cargo-zigbuild`/Zig or validate actual hosted runner images. | Tool capability is explicit policy/preflight intent; execution belongs to build/qualification and future runner operations. |

## Roadmap disposition and dependency transitions

CI M001 is closed. Build/Qualification M003 remains `ready to plan` because M002 candidate evidence is closed. Build M004 remains blocked on Build M003 qualification and Manifest M002. CI M002 qualification/aggregation/drift CLI remains blocked because it depends on the future Build M003/M004 interfaces as well as CI M001; closing CI M001 alone does not define those interfaces. CI M003 draft staging remains blocked on CI M002 and an explicit staging adapter plan. No downstream blocked status can be promoted yet.

| Milestone | Status | Blocker |
|---|---|---|
| CI M001 CIPlan + GitHub renderer | closed | Hosted run 36040032768 passed all required lanes. |
| Build/Qualification M003 qualification execution | ready to plan | Build M002 candidate evidence interface is closed. |
| Build/Qualification M004 finalization/aggregation | blocked | Build M003 closure and Manifest M002. |
| CI M002 qualification/aggregation gates + drift CLI | blocked | Build M003/M004 interfaces and CI M001 closure. |
| CI M003 draft release staging | blocked | CI M002 and an explicit staging adapter plan. |


## Post-closure dependency update

After this CI milestone closed, post-closure review of Build/Qualification M002 found intermittent Windows qualification instability: a Windows workspace-test failure on run 36040609714 attempt 1 passed on rerun attempt 2 without a production-code change, with an earlier similar failure on run 36035978249.

Build corrective M002a is registered at `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md`.

This does not reopen CI M001: its deterministic CIPlan/renderer evidence remains valid. It does withdraw the historical statement that Build M003 is currently ready to plan. Build M003 is blocked until M002a closes; consequently CI M002 remains blocked as before.

M002a subsequently closed with three first-attempt Windows qualification passes on one corrective SHA; see `plans/closure/build-qualification/002a-status.md`. Build M003 is again `ready to plan`. This status update does not unblock CI M002: it still requires the future Build M003/M004 qualification and finalization interfaces.

Subsequent disposition: Build M003 closed with cross-platform evidence at `plans/closure/build-qualification/003-status.md`; Build M004 is now active. CI M002 remains blocked until M004 closes and supplies the finalization interface.

Later disposition: Build M004 closed with finalization/aggregation evidence at `plans/closure/build-qualification/004-status.md`. CI M002 is now ready to plan; CI M003 remains blocked on M002 and a separately reviewed staging adapter plan.
