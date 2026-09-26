# `eggpack-ci` — Deep Dive

Provider-neutral projection of resolved `ReleasePlan` + `BuildBindingsV1` into a
bounded `CIPlan`, with deterministic checked-in GitHub Actions rendering and
drift checking. Single module `crates/eggpack-ci/src/lib.rs` (~8100 lines).
Fixtures in `tests/fixtures/` (`native-direct`, `native-direct-multitarget`,
`mixed-direct-targets.toml`, `m002-{direct,bundle,archive,mixed}.yml`,
`m003b-*-staging.yml`).

Pipeline: `ReleasePlan → CIPlan → GitHub renderer → checked-in release.yml`
(+ `ci check` re-render/diff).

## Roles

- **CI graph** (`:42-107`, `:1941-1964`): M001 `CIPlan{release_id,
  source_revision, targets: Vec<TargetJob>,
  required_aggregation_dependencies}` — provider-neutral, canonically ordered,
  `QualificationState::Unresolved` only (never claims pass).
- **Renderer** (`:1290`, `:2536`, `:2559`): pure `render_github`,
  `render_release_github`, `render_reusable_release_github`. Deterministic bytes,
  stable job/step order, no timestamps. Runner labels + immutable action SHAs
  are policy inputs, never plan data (`:446-556`).
- **Drift check** (`:1411-1423`, `:2660`, `:3345`): `check_github`,
  `check_release_github`, `check_reusable_release_github → DriftReport`,
  CRLF→LF byte compare, never rewrites.
- **Staging** (M003b/c, `:1913-1939`, `:559-637`): optional
  `StagingJob{job_id:"stage", provider: GitHubDraft}` via
  `with_github_draft_staging()`. Exactly one `stage` job (`needs:aggregate`,
  `contents:write`; all prior jobs `contents:read`, no `id-token:write`,
  token via env only): `_prepare-stage` → `_stage-github-draft`.
- **Consumer seam** (M003d, `:3972-4094`): static identity-free
  `ReleaseWorkflowShapeV1` (canonical targets, bindings, consumer validators,
  staging intent; no release/revision/tag/digest) + runtime
  `resolve_runtime_release_plan()` via generated `resolve` job
  (`_resolve-release` + per-job `_verify-source` of `HEAD^{commit}` against
  `ReleasePlan.source_revision`), plus per-target `ConsumerValidatorV1`
  (Python3-only) post-core-qualification verifier.

## Key types / functions (`src/lib.rs`)

- `CiError(String)` (`:28`); `TargetJob{planned, job_id, required,
  qualification: QualificationIntent, outputs}` (`:58`);
  `QualificationIntent` (`:74`); `BuildOutput{selector, package, binary,
  handoff_name, executable, args}` (`:94`).
- `ReleaseCIPlanV1{ci_plan, qualifications, gate_job_id:"required_gate",
  aggregate, finalization, staging?, consumer_validators?}` (`:1944`) +
  `QualificationJob`, `AggregateJob`, `FinalizationSettings`;
  `StagingJob` (`:1926`), `StagingProvider::GitHubDraft` (`:1913`).
- `ReleaseWorkflowShapeV1{targets, selected_aliases, build_bindings,
  qualification_bindings, consumer_validators, staging?}` (`:3972`).
- `ConsumerValidatorV1{selector, interpreter: Python3, script, timeout_ms,
  stdout/stderr limits}` (`:3409`); `ConsumerValidationEvidenceV1`
  (identity + bounded outcome, no script output) (`:3504`).
- Policy: `ActionPin` (`:446`), `RunnerMapping` (`:454`), `WorkflowTrigger`
  (`:470`), `GitHubPolicy{runner mapping, action pins, triggers, timeouts,
  concurrency, EggpackToolPolicy}` (`:480`),
  `EggpackToolPolicy{repo:"eggpack-cli", exact rev, --locked}` (`:524`).
- Inputs: `GitHubReleaseInputsV1{contract, release_plan, build/qualification
  bindings, ci_plan, pack_config?, draft_template?, installer_presentation?,
  consumer_validators?}` (`:559`); `GitHubStagingPolicyV1{runner, owner,
  repository, tag_source, inputs, receipt_retention_days}` (`:614`);
  `StagingTagSource::RefName | DispatchInput` (`:632`).
- Constants (`:684-690`): `BUILD_HANDOFF_FILE`, `QUALIFICATION_EVIDENCE_FILE`,
  `CANDIDATES_DIR`, `GATE_OUTCOME_FILE`.
- `RunnerCommand` finite enum (`:715-868`): `VerifySource, CaptureBuild,
  QualifyTarget, EvaluateGate, Aggregate, PrepareStage, StageGithubDraft,
  ResolveRelease, ValidateConsumer, …` — shared by rendering and CLI; drift
  caught by round-trip, not YAML inspection.
- Functions: `project_ci_plan` (`:110`); `render_github` (`:1290`);
  `check_github` (`:1423`); `project_build_handoff` (`:1579`),
  `reconstruct_attempt` (`:1640`), `validate_build/qualification_artifact_dir`,
  `stage_build_artifact_dir`, `cargo_output_path`; `project_release_plan[_with_
  consumer]` (`:2095/2171`); `evaluate_gate[_with_consumer]`
  (`:2376/:2235`); `aggregate_finalize[_with_consumer]` (`:2436/:2299`);
  `with_github_draft_staging` (`:2342`); render/check release + reusable
  (`:2536/:2559/:2660/:3345`); `resolve_runtime_release_plan` (`:4094`).

Rendered jobs: `resolve` (M003d) → `preflight` → `build-<target>` →
`qualify-<target>` → [`consumer-validate-<target>`] → `required_gate` →
`aggregate` → [`stage`].

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
  M002a. Live draft proof outstanding (conditional close).
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

## Boundaries

Does not infer package/bin identities (via `eggpack_core::cargo_command`), run
builds/qualification, create manifests, stage, publish, access GitHub, or accept
arbitrary YAML/steps. No unpinned binaries, no auto-install toolchains, no
signing/staging in M002, no product-owned release semantics or script-output
recording in M003d. Core qual/final semantics stay in `eggpack-core`; CI only
projects/transports. `check_*` never writes.

## Dependencies / dependents

- Runtime (`Cargo.toml:14-19`): `eggpack-contract`, `eggpack-core`, `serde`,
  `serde_json`, `sha2`. Dev-only: `serde_yaml`, `eggpack-manifest`,
  `eggpack-bootstrap`, `eggpack-github`, `tokio` — production stays free of
  GitHub HTTP/async.
- Dependents: `eggpack-cli` only (calls project/render/check/handoff/gate/
  aggregate/resolve/runner commands).
