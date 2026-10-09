# CI/Release M003j — Dispatch-tag Shell Injection Corrective

Status: ready for implementation.
Repository baseline: `eggstack/eggpack@d61ca71fc0112be63e7e8ba31ba8fa2b1ce5a628` (2026-10-08).
Source roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`.
Primary class: invariant / security corrective.
Historical upstream authority: ADR-0003 checked-in generated CI and publication; CI M003i `plans/implementation/ci-release-orchestration/003i-explicit-tag-and-manifest-release-identity-corrective.md` and `plans/closure/ci-release-orchestration/003i-status.md` (closed; historic record remains unchanged).
Confirmed affected consumer: `dbowm91/wg-basic@ff40f851508ccda52171c27cefdcdaf4ec1da2d5` generated `.github/workflows/release-eggpack.yml`, Release Readiness R001 `plans/implementation/release-readiness/001-release-workflow-input-and-provenance-hardening.md` on `plans/release-readiness-security`.
Dependency: previous CI M001–M003i closed. No platform/release producer contract change required.

## 1. Objective

Eliminate shell command injection through a user-provided `workflow_dispatch.inputs.release_tag` in all Eggpack-rendered GitHub Actions scripts. Preserve deterministic generation, tag/source/manifest separation, exact-tag staging, read-only build jobs, write-only final draft staging, stable manifest schema, and existing consumer workflows. Supply a new immutable Eggpack revision for wg-basic R001 to pin/regenerate before any production signing/staging.

## 2. Reproduction and missed test surface

`crates/eggpack-ci/src/lib.rs` renders the unsafe pattern twice, within:
- reusable runtime identity `resolve` job, around the `Validate exact-tag source` step in `render_github_workflow`;
- final `stage` job (the job with `permissions: contents: write`), also in `Validate exact-tag source`.

Both emit the Bash command text `test -n "${{ inputs.release_tag }}"`. GitHub evaluates `${{...}}` interpolation before creating the shell script. A crafted dispatch-input value containing command-substitution or shell-special syntax can therefore become executable Bash source. This is a *source-proven generated script injection surface*; no attack against real CI or credentials has been performed. The affected downstream wg-basic generated workflow contains the exact two vulnerable emitted steps at its baseline.

Existing tests `m003c_tag_source_controls_checkout_input_and_stage_guard`, checked-in golden workflows and `eggpack ci check` validate source tag mapping and draft permissions, but do not assert that user-supplied values are never embedded inside executable shell snippets. M003i's identity binding is necessary but does not remove this interpretation hazard.

## 3. Invariants

- No runtime/event/dispatch expression with potentially untrusted text is interpolated directly into a generated `run:` Bash/PowerShell expression. Use an Actions `env:` binding with subsequent **quoted variable reference** or an equivalent typed input protocol.
- Validate canonical stable tag `vX.Y.Z` (no leading zeros, shell metacharacters, suffix, embedded newline, oversized value or alternate ref) and nonempty input before source resolution and privileged staging. Do not trust tag syntax to authorize source SHA; continue using the existing exact-source verification.
- No `eval`, `bash -c`, script templating from dispatch value, untrusted output-file append, unsafe variable re-evaluation, or unbounded shell string concatenation.
- `contents: write` remains limited to staging after all build/qualification/aggregation jobs pass. Never pass a signing secret or broaden permissions to make tests pass.
- Existing M003i `v_prefixed_stable_semver` mapped identity and legacy exact-tag modes, `_resolve-release`, `_verify-source`, tag/commit/manifest binding and default release-concurrency semantics remain compatible.
- Production CLI/workflow paths and Eggpack ReleaseManifest v1 schema remain unchanged. No new producer-side signing; key custody belongs to each consumer.
- A resolved GitHub tag or string in `with.ref` or non-executable concurrency group need not be blindly forbidden; prohibit the **executable shell interpolation sink** and separately validate tag/ref authority before build/stage.

## 4. Scope / non-goals

In scope: the two renderer sites and any other discovered equivalent executable interpolation, deterministic YAML rendering, finite syntax guard, golden fixtures/tests, producer documentation, CI, downstream rerender handoff.

Out of scope: wg-basic product-key management, first-install trust flow, publication itself, changing generated manifest schema, generic arbitrary workflow scripting, adding unbounded expression language, changing GitHub security settings without maintainer authorization, or rewriting historical M003i closure.

## 5. Ordered work packages

### WP1 — Failing discriminating tests first

Add a regression that inspects the **actual generated reusable dispatch + write-scoped stage workflow** and fails when a `run:` script includes `${{ inputs.release_tag }}` or another unsafe untrusted expression. Build a controlled local Bash test that expands a malicious fixture tag in the *old* script template and observes a harmless temporary marker, then asserts the corrected render never evaluates the marker. Do not use actual network calls, privileged staging or production GitHub credentials. Include benign `v1.2.3`, empty, malformed, newline, leading-zero, shell-substitution and punctuation cases.

### WP2 — Minimal generator correction

For both reusable `resolve` and final `stage` validations, use:

```yaml
- name: Validate exact-tag source
  shell: bash
  env:
    EGGPACK_RELEASE_TAG: ${{ inputs.release_tag }}
  run: |
    # reject invalid/noncanonical release_tag (strict stable SemVer)
    # test "$EGGPACK_RELEASE_TAG" as data, never as source
```

An equivalent stable private generator helper is acceptable. Preserve dispatch validation/checkout/source preflight ordering; ensure validation executes *before* any write-scoped action that consumes the tag or stages a draft. If checkout currently precedes validation, assess whether a malicious ref can cause branch-controlled code to execute, and add an earlier static validation step before checkout where necessary, without altering the source authority. Add a deterministic bounded tag syntax function/reusable private snippet rather than repeated unsafe string formatting.

### WP3 — Generator fixtures and compatibility

Update relevant `crates/eggpack-ci/tests/fixtures/*` golden YAML and current expectation tests. Confirm `StagingTagSource::RefName` legacy behavior (push-only tag), DispatchInput mapped identity, `github.ref_type` use, exact permitted source-ref checks, and `v_prefixed_stable_semver` identity normalization retain their existing semantics. Verify `eggpack ci check` catches a reintroduced unsafe `run:` interpolation even if generated schema/format otherwise matches; prefer a small static security guard in generator tests.

### WP4 — Consumer handoff

Generate the wg-basic workflow from its pinned config using the corrected Eggpack revision, verify no drift under `eggpack ci check`, and prove the exact unsafe source expression is absent in both consumer sites. Record immutable Eggpack implementation SHA and any changed consumer policy keys; do not modify wg-basic's repository from Eggpack except via the separate consumer R001 plan.

Check whether other adopted Eggpack consumers enable `DispatchInput` staging and document impacted workflows; do not claim all consumers affected solely from renderer reachability.

### WP5 — Verification and docs

Execute Cargo fmt, clippy, all workspace tests, docs, Rust 1.89 MSRV, generated workflow fixture and independent `ci check` including a changed-producer negative. Require hosted Stable Linux, MSRV, macOS and Windows full qualification on the implementation SHA; record actual runs. Update `architecture/ci.md`, `architecture/github.md`, `crates/eggpack-ci/README.md`, subsystem roadmap and registry.

## 6. Failure, contention and authorization

Unsafe/malformed dispatch input must fail before any candidate-build or draft-staging mutation. Multiple dispatch jobs for the same canonical tag remain concurrency-serialized, but concurrency text is not authorization. An input racing a moving tag must still fail exact-source verification. Do not silently accept a branch where a stable tag is required. Stop on a bypass for the script guard or ref-source mismatch.

## 7. Compatibility/security and test commands

No new dependency, public enum variant, release-manifest schema or ordinary binary API break. No additional contents-write scope, no signing credential access and no arbitrary subprocess execution. Changes limited to generator and tests, plus docs and golden workflows.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
git diff --check
```

Add direct narrowed `cargo test -p eggpack-ci --locked` and the actual generated workflow security fixture/CI validation. If the upstream repository uses a different supported command, record the exact equivalent. Require a smoke against the wg-basic pinned `release/eggpack/github-policy.json` fixture, but do not require production signing or root service install in Eggpack.

## 8. Acceptance

Both source renderer sites use safe data handling; generator tests fail against the baseline vulnerable render and pass after correction; malicious input never becomes shell commands, and invalid tag fails before privileged stage; golden workflows and `ci check` remain deterministic; source/tag/release identity semantics retained; platform CI green; exact corrected producer SHA is recorded for wg-basic adoption; no public signature/publish work occurs.

## 9. Stop conditions

Stop on tag-check logic that is bypassable with shell quoting/newline; a workaround that edits only wg-basic generated YAML; stage job running unvalidated tagged-source code; changes to manifest semantics or publication authority; privilege expansion; regressions in existing CI M003i mapped identity; or inability to prove source-ref/runner-stage isolation.

## 10. Closure evidence

After implementation, write `plans/closure/ci-release-orchestration/003j-status.md` with baseline/implementation SHA, before/after generated YAML and controlled injection negative proof, test/CI run URLs, released pin guidance, exact consumer rerender diff, unchanged scope/manifest checks, findings severity and explicit wg-basic R001 unblock. Prior CI M003i closure remains historical and unchanged.
