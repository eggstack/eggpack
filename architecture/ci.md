# `eggpack-ci` — Deep Dive

Provider-neutral projection of resolved `ReleasePlan` + `BuildBindingsV1` into a
bounded `CIPlan`, with deterministic checked-in GitHub Actions rendering and
drift checking. Single module `crates/eggpack-ci/src/lib.rs` (10221 lines).
Fixtures in `tests/fixtures/` (`native-direct`, `native-direct-multitarget`,
`mixed-direct-targets.toml`, `m002-{direct,bundle,archive,mixed}.yml`,
`m003b-*-staging.yml`).

Pipeline: `ReleasePlan → CIPlan → GitHub renderer → checked-in release.yml`
(+ `ci check` re-render/diff).

## Roles

- **CI graph** (`CIPlan`, `ReleaseCIPlanV1`): M001 `CIPlan{release_id,
  source_revision, targets: Vec<TargetJob>,
  required_aggregation_dependencies}` — provider-neutral, canonically ordered,
  `QualificationState::Unresolved` only (never claims pass).
- **Renderer** (`render_github`, `render_release_github`,
  `render_reusable_release_github`): pure deterministic bytes, stable job/step
  order, no timestamps. Runner labels + immutable action SHAs are policy inputs,
  never plan data.
- **Drift check** (`check_github`, `check_release_github`,
  `check_reusable_release_github → DriftReport`): CRLF→LF byte compare, never
  rewrites.
- **Staging** (M003b/c): optional `StagingJob{job_id:"stage", provider:
  GitHubDraft}` via `with_github_draft_staging()`. Exactly one `stage` job
  (`needs:aggregate`, `contents:write`; all prior jobs `contents:read`, no
  `id-token:write`, token via env only): `_prepare-stage` →
  `_stage-github-draft`.
- **Consumer seam** (M003d): static identity-free `ReleaseWorkflowShapeV1`
  (canonical targets, bindings, consumer validators, staging intent; no
  release/revision/tag/digest) + runtime `resolve_runtime_release_plan()` via
  generated `resolve` job (`_resolve-release` + per-job `_verify-source` of
  `HEAD^{commit}` against `ReleasePlan.source_revision`), plus per-target
  `ConsumerValidatorV1` (Python3-only) post-core-qualification verifier.

## Key types / functions (`src/lib.rs`)

- `CiError(String)` (`:28`); `TargetJob{planned, job_id, required,
  qualification: QualificationIntent, outputs}` (`:58`);
  `QualificationIntent` (`:74`); `BuildOutput{selector, package, binary,
  handoff_name, executable, args}` (`:94`).
- `ReleaseCIPlanV1{ci_plan, qualifications, gate_job_id:"required_gate",
  aggregate, finalization, staging?, consumer_validators?}` (`:2309`) +
  `QualificationJob` (`:2208`), `AggregateJob` (`:2232`), `FinalizationSettings`
  (`:2258`); `StagingJob` (`:2291`), `StagingProvider::GitHubDraft` (`:2278`).
- `ReleaseWorkflowShapeV1{targets, selected_aliases, build_bindings,
  qualification_bindings, consumer_validators, staging?}` (`:4472`).
- `ConsumerValidatorV1{selector, interpreter: Python3, script, timeout_ms,
  stdout/stderr limits}` (`:3873`); `ConsumerValidationEvidenceV1`
  (identity + bounded outcome, no script output) (`:3968`).
- Policy: `ActionPin` (`:453`), `RunnerMapping` (`:468`),
  `ZigOfficialArchiveV1` (`:487`), `CrossToolProvisioningV1` (`:503`, M005),
  `WorkflowTrigger` (`:718`), `GitHubPolicy{runner mapping, action pins,
  triggers, timeouts, concurrency, EggpackToolPolicy}` (`:728`),
  `EggpackToolPolicy{repo:"https://github.com/eggstack/eggpack",
  package:"eggpack-cli", exact 40-hex revision, --locked}` (`:781`).
- Inputs: `GitHubReleaseInputsV1{contract, release_plan, build/qualification
  bindings, ci_plan, pack_config?, draft_template?, installer_presentation?,
  consumer_validators?}` (`:816`); `GitHubStagingPolicyV1{runner, owner,
  repository, tag_source, inputs, receipt_retention_days}` (`:871`);
  `StagingTagSource::RefName | DispatchInput` (`:889`).
- Constants (`:941-947`): `BUILD_HANDOFF_FILE`, `QUALIFICATION_EVIDENCE_FILE`,
  `CANDIDATES_DIR`, `GATE_OUTCOME_FILE`; plus `CONSUMER_EVIDENCE_FILE` (`:3845`)
  and `RUNTIME_*` identity paths (`:4430-4438`).
- `RunnerCommand` finite enum (`:972-1116`): `VerifySource, CaptureBuild,
  QualifyTarget, EvaluateGate, Aggregate, PrepareStage, StageGithubDraft,
  ValidateConsumer, ResolveRelease, …` — shared by rendering and CLI; drift
  caught by round-trip, not YAML inspection.
- Functions: `project_ci_plan` (`:110`); `render_github` (`:1660`);
  `check_github` (`:1788`); `project_build_handoff` (`:1944`),
  `reconstruct_attempt` (`:2005`), `validate_build/qualification_artifact_dir`,
  `stage_build_artifact_dir`, `cargo_output_path`; `project_release_plan`
  (`:2460`) / `project_release_plan_with_consumer` (`:2536`);
  `evaluate_gate` (`:2741`) / `evaluate_gate_with_consumer` (`:2600`);
  `aggregate_finalize` (`:2801`) / `aggregate_finalize_with_consumer`
  (`:2664`); `with_github_draft_staging` (`:2707`); render/check release +
  reusable (`render_release_github :2924`, `render_reusable_release_github
  :2947`, `check_reusable_release_github :3048`, `check_release_github :3809`);
  `resolve_runtime_release_plan` (`:4594`); `run_consumer_validator` (`:4151`).

Rendered jobs: `preflight` → `resolve` (M003d reusable only, `needs:preflight`)
→ `build-<target>` → `qualify-<target>` → [`consumer-validate-<target>`] →
`required_gate` → `aggregate` → [`stage`].

## Evolution

- **M001:** provider-neutral `CIPlan` + deterministic renderer. Read-only,
  unresolved qual intent, internal handoffs only.
- **M002:** executable `ReleaseCIPlanV1` — per-target qual jobs, required-evidence
  gate over structured `QualificationEvidence` (never logs), `aggregate/finalize`
  invoking M004. Required must pass; non-gating suppresses release (never
  partial). Still read-only, internal artifacts, pinned `eggpack-cli --locked`.
- **M002a (corrective):** every CLI invocation carries explicit
  `GitHubReleaseInputsV1` repo-relative paths (no discovery); `CaptureBuild`
  with known target root + `build-handoff.json+candidates/`; `QualifyTarget`
  with all 8 args; gate/aggregate via `eggpack-inputs/<target>/`.
- **M003a:** local staging payload + `eggpack-github` draft adapter (no Actions
  wiring yet; lives mostly in `eggpack-github`).
- **M003b:** wires adapter into the generated `stage` job; concurrency
  release-scoped/tag-aware; static guards reject `id-token:write`, `gh release`,
  `--clobber`, publish, tag mutation, raw `curl`. Without staging, byte-equal to
  M002a. Live draft/inventory/draft-only/no-clobber proven by the eggsact
  `v1.2.7` 15-asset draft (run 36652731202); only byte-identical rerun reuse
  remains outstanding (conditional close, blocked on eggsact M005a Windows
  byte determinism).
- **M003c (corrective):** one exact source per run (`RefName` = tag-push
  `github.ref_name`, `DispatchInput` = explicit `release_tag`); every job
  `HEAD^{commit}`-verifies vs `source_revision`; streamed uploads, bounded
  pagination, hardened upload URL/query.
- **M003d:** consumer seam without absorbing product semantics — static shape +
  runtime `_resolve-release` (fixes self-SHA paradox), `resolve` job + preflight
  artifact + `_verify-source`; `ConsumerValidatorV1` (Python3 → `python3|python`
  + `--version` preflight, no DSL) + identity/outcome-only evidence;
  product-wrapper staging (`--installer-presentation` + `--source-root`; absent
  = M003c-compatible). Without validators/wrappers, byte-identical to M003c.
- **M003e/f/g (live-qualification correctives):** M003e tool-install-before-use
  ordering, `mkdir_step` for runtime/consumer/gate/finalized dirs, CLI
  absolutization of generated relative paths (impl `b9062d4`, run 36572608484);
  M003f positional package selection for the tool-install command (rejected
  `-p` flag; impl `c190e77`, run 36632209736); M003g second-dispatch fixes
  (zigbuild PATH timing, exec-bit restore after transfer, consumer/gate arity
  and error surfacing) with the final live run staging the complete 15-asset
  eggsact draft (run 36652731202).
- **M005:** provisioned `cargo-zigbuild` / official `ziglang.org` Zig with SHA
  pin (`CrossToolProvisioningV1` / `ZigOfficialArchiveV1`); `PreinstalledVerified`
  legacy path.

## Boundaries

Does not infer package/bin identities (via `eggpack_core::cargo_command`), run
builds/qualification, create manifests, stage, publish, access GitHub, or accept
arbitrary YAML/steps. No unpinned binaries, no auto-install toolchains, no
signing/staging in M002, no product-owned release semantics or script-output
recording in M003d. Core qual/final semantics stay in `eggpack-core`; CI only
projects/transports. `check_*` never writes.

## Dependencies / dependents

- Runtime (`Cargo.toml:15-19`): `eggpack-contract`, `eggpack-core`, `serde`,
  `serde_json`, `sha2`. Dev-only: `serde_yaml`, `eggpack-manifest`,
  `eggpack-bootstrap`, `eggpack-github`, `tokio` — production stays free of
  GitHub HTTP/async.
- Dependents: `eggpack-cli` only (calls project/render/check/handoff/gate/
  aggregate/resolve/runner commands).
