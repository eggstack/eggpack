# CI M003g Corrective — Live Qualification Failures (Cross-Tool PATH, Consumer-Validation Diagnosis)

Status: active

Source findings: Ecosystem M001 live-draft second dispatch (eggsact run `36634754073`, tag `v1.2.7`, tool pin M003f `c190e77`). Tool provisioning now works; `resolve` succeeds; 3/5 builds, 4/5 qualifies, and Windows consumer validation pass. Two failures remain; the failed run made no release mutation (no draft exists).

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

## 1. Objective

Get the live five-target run green: fix the cross-tool PATH defect, instrument the consumer-validation failure so the runner log names the cause, diagnose and fix the macOS consumer-validation failure, and re-dispatch the same tag.

## 2. Finding F9 — high: `cargo zigbuild --version` runs before its PATH export takes effect

`provision_cross_tools_steps` installs cargo-zigbuild into an isolated `$install_root`, appends `$install_root/bin` to `$GITHUB_PATH`, then invokes `cargo zigbuild --version` in the SAME step. `GITHUB_PATH` exports take effect only in subsequent steps, so the check fails with `error: no such command: 'zigbuild'` (exit 101). Both Linux cross builds failed exactly this way; the legacy workflow never hit it because it installed into the default Cargo root already on PATH.

Fix: `export PATH="$install_root/bin:$PATH"` in the same step immediately after install, before the version check. Keep the `$GITHUB_PATH` export for later steps. One emission site; no provisioning-semantics change.

## 3. Finding F10 — high: macOS consumer validation fails with no diagnosis (cause TBD)

Both macOS `validate` jobs fail with only `eggpack: consumer validation failed for <target>`. Windows validation of the same script shape passes; macOS qualify (`--version`) passes. The adapter deliberately keeps script output out of evidence, and the CLI discards the `ConsumerValidationFailure` variant, so the log names no cause.

Instrumentation (evidence-neutral, console only): on `Failed(reason)`, print `consumer validation failed for {target}: {reason:?}`. Evidence JSON bytes unchanged. The next live run then reports one of `NonZeroExit | Timeout | OutputLimit | InterpreterUnavailable | CandidateMismatch | ScriptUnavailable`, directing the root-cause fix inside this plan.

## 4. Boundaries

- Renderer: F9 PATH export line only.
- CLI: failure-variant console print only; no evidence-schema change, no validation-semantics change.
- F10 root-cause fix: TBD after diagnosis; stop and re-plan if it requires relaxing a library validation.
- Out of scope: workspace restructure, caching, new actions/triggers/privileges.

## 5. Tests

- T1: renderer unit test asserting the zigbuild install step exports the install-root `bin` directory to `PATH` before invoking `cargo zigbuild`.
- T2: goldens regenerate (only fixtures with provisioned cross tools change, plus nothing else).
- T3: existing suites unchanged.

## 6. Verification

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Plus a hosted CI run on the exact implementation SHA (all four lanes green), and the live re-dispatch evidence.

## 7. Acceptance criteria

M003g closes only when:

- F9 is corrected per §2 with no other generated-output change except affected goldens;
- the failure-variant print lands and names the F10 cause on the live re-dispatch;
- the F10 root cause is fixed (or proven external with a bounded workaround recorded here);
- T1–T3 pass; full local verification passes; hosted CI passes on the implementation SHA;
- the live five-target run (same tag `v1.2.7`) goes green through stage, the draft carries the §15 inventory, the rerun reuses it without clobber, and the release stays draft;
- no unresolved medium-or-higher finding remains.

## 8. Stop conditions

Stop and re-plan if F10 requires relaxing a library validation, a new privilege/egress, or a product-code change outside Eggpack's adapter (that fix belongs to the consumer plan, referenced here).

## 9. Closure evidence

Create:

`plans/closure/ci-release-orchestration/003g-status.md`

Record implementation SHA, hosted run id, F9/F10 matrix, golden inventory, verification results, and the live run/draft evidence pointers.
