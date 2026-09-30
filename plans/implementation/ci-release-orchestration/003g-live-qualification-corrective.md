# CI M003g Corrective — Live Qualification Failures (Cross-Tool PATH, Consumer-Validation Diagnosis)

Status: closed — see `plans/closure/ci-release-orchestration/003g-status.md` (implementation `e5c81f2`, hosted runs 36638321253/36642184204/36646814734/36652203168 green)

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

### F10 root cause (proven 2026-09-29 from live evidence)

The re-dispatch reports `NonZeroExit` on both macOS targets, and the
downloaded qualify artifact (`eggpack-evidence-aarch64-apple-darwin`)
carries its candidate at mode 644. Root chain:

1. `actions/upload-artifact` documents Permission Loss: all files
   materialize as 644 after transfer, so every transferred candidate
   loses its exec bit on non-Windows runners.
2. The macOS qualify smoke therefore fails (`smoke_failed` in the
   downloaded `evidence.json`), yet the qualify JOB exits 0 by design
   (the gate must still receive the evidence to fail closed).
3. `_validate-consumer` never checked `evidence.status`, so it spawned
   the consumer script against a non-executable candidate: instant
   `EACCES`, instant `NonZeroExit`, no script output. Windows passed
   only because Windows has no exec-bit concept.

Fixes (both evidence-neutral, no validation-semantics change):

- F10a: restore exec bits (`mode | 0o111`, Unix only) immediately
  before spawning a candidate — in `run_bounded_inner` for the
  `Candidate` allowance (eggpack-core) and in `run_validator_process`
  after byte-identity verification (eggpack-ci). Recorded size/digest
  cover bytes only and cannot change.
- F10c: `_validate-consumer` refuses non-`Passed` evidence fast with
  `qualification evidence is not Passed: <status>`, before executing
  anything. Qualify still exits 0 on Failed evidence so the gate keeps
  full visibility and fails closed as designed.

## 4. Finding F11 — high: `_stage-github-draft` arity guard rejects the renderer's own call
The stage step emits `--payload/--github-policy/--staging-dir/--output-receipt`
(8 args) but `ci_stage_github_draft` guarded `args.len() > 6`, failing with
`too many arguments` before any network I/O (live run `36642916807`: all
five targets built, qualified, validated, gated, and aggregated; only stage
failed; no mutation). An audit of every other `ci_*` arity guard against the
rendered invocations shows this is the sole mismatch — all other commands
are already proven live.

Fix: bound `> 8`. One line; no flag-semantics change.

## 5. Finding F12 — high: stage fails in ~1s with the inner cause swallowed

Live run `36647375561` (all five targets built, qualified, validated, gated,
aggregated; payload and policy verified identical locally) fails in
`_stage-github-draft` with only `github draft staging failed`. Payload/policy
match, token present, endpoints and permissions verified, no mutation made —
but the inner `GithubError` is discarded by the CLI, so the failing call
(tag lookup, tag peel, release list) cannot be identified.

Fix: surface the inner message (`github draft staging failed: {error}`).
`GithubError` carries only static bounded messages (audited: no token, body,
or URL content), so naming it is safe. Evidence and receipt schemas unchanged.

## 6. Finding F13 — latent: `make_latest` sent as boolean, API expects string

`create_release` sends `"make_latest": false`. The GitHub API specifies a
string enum (`"true"`/`"false"`/`"legacy"`); a boolean risks 422 once the
create path is reached (it never has been — no draft exists yet). Caught by
reading the wire body while diagnosing F12; mock transports never validate
wire shapes against GitHub.

Fix: send `"make_latest": "false"`. One word; no other wire change.

## 7. Boundaries

- Renderer: F9 PATH export line only.
- CLI: failure-variant console print only; no evidence-schema change, no validation-semantics change.
- F10 root-cause fix: TBD after diagnosis; stop and re-plan if it requires relaxing a library validation.
- Out of scope: workspace restructure, caching, new actions/triggers/privileges.

## 8. Tests

- T1: renderer unit test asserting the zigbuild install step exports the install-root `bin` directory to `PATH` before invoking `cargo zigbuild`.
- T2: goldens regenerate (no fixture uses the provisioned cross-tools path, so none change; recorded).
- T3: existing suites unchanged.
- T4 (`m003g_validator_restores_exec_on_transferred_candidate`, Unix): a 644 candidate plus an executing validator script passes — fails without the F10a restore.
- T5 (`validate_consumer_refuses_failed_qualification_evidence`): Failed evidence is refused fast with its status — the script never runs.
- T6: `_stage-github-draft` accepts the renderer's four-flag call (8 args) and still rejects a ninth.
- T7: full local verification covers the F12 message surfacing and F13 wire fix (no new behavior to unit-test beyond existing mock suites; the live run is the proof).

## 9. Verification

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

## 10. Acceptance criteria

M003g closes only when:

- F9 is corrected per §2 with no other generated-output change except affected goldens;
- the failure-variant print lands and names the F10 cause on the live re-dispatch;
- the F10 root cause is fixed per §3 (F10a exec restore at both spawn sites, F10c fast refusal of non-Passed evidence; qualify still exits 0 on Failed so the gate keeps visibility);
- T1–T7 pass; full local verification passes; hosted CI passes on the implementation SHA;
- the live five-target run (same tag `v1.2.7`) goes green through stage, the draft carries the §15 inventory, the rerun reuses it without clobber, and the release stays draft;
- no unresolved medium-or-higher finding remains.

## 11. Stop conditions

Stop and re-plan if F10 requires relaxing a library validation, a new privilege/egress, or a product-code change outside Eggpack's adapter (that fix belongs to the consumer plan, referenced here).

## 12. Closure evidence

Create:

`plans/closure/ci-release-orchestration/003g-status.md`

Record implementation SHA, hosted run id, F9/F10 matrix, golden inventory, verification results, and the live run/draft evidence pointers.
