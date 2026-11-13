# `eggpack-ci` — Deep Dive

Crate-level orientation for `eggpack-ci`: the largest component in the workspace
split across a thirteen-file module tree under `crates/eggpack-ci/src/`. This
file explains how that tree divides and what flows through it. It does not
restate per-area detail — three deep dives cover the parts.

Line references of the form `module.rs:line` are relative to
`crates/eggpack-ci/src/`.

`eggpack-ci` depends on `eggpack-contract` and `eggpack-core` only. It never
executes a release itself: it *plans* the release as a job graph, *renders* that
graph as a GitHub Actions workflow, *checks* the checked-in workflow for drift,
and *encodes/decodes* the artifacts the generated steps exchange. The actual
building, qualification, and finalization are performed by `eggpack-core` when
the generated workflow invokes the `eggpack` CLI.

Code baseline `fc072af`.

## The three deep dives

| Deep dive | Owns | Modules of `crates/eggpack-ci/src/` |
|---|---|---|
| [ci-rendering.md](ci-rendering.md) | policy types, `RunnerCommand`, the three renderers, drift checks | `policy.rs`, `runner.rs`, `render_github.rs`, `render_release.rs`, `drift.rs` |
| [ci-consumer-seam.md](ci-consumer-seam.md) | the external consumer validator, runtime identity, artifact handoff formats | `handoff.rs`, `consumer_validator.rs`, `identity.rs` |
| this file | crate orientation, the projection model, job vocabulary, gates, aggregation | `planning.rs`, `release_plan.rs`, `error.rs`, `validate.rs` |

Read this file first, then follow exactly one of the other two.

One review note up front: of the three renderers, only
`render_release_github` and `render_reusable_release_github` are reachable from
the CLI. `render_github` and `check_github` (the basic `CIPlan`-only pair) have
no caller anywhere in `crates/`, so they are public API that nothing exercises
outside this crate's own tests. Also note that `render_reusable_release_github`
emits no `workflow_call` trigger — "reusable" here means tag-independent, not
GitHub's calling convention. Both facts are detailed in
[ci-rendering.md](ci-rendering.md).

## The projection model

Eggpack projects a release through three nested structures, each strictly
richer than the last. Nothing is invented at the CI layer: the job graph is
derived from the plan, and the plan was derived from the contract.

```text
ReleasePlan                      (eggpack-core: intent, canonically sorted)
        |
        |  project_ci_plan(contract, plan, build_bindings, qualification_bindings)
        v
CIPlan                           (planning.rs:24   provider-neutral build graph)
        |                        targets: Vec<TargetJob>  (planning.rs:32)
        |                        required_aggregation_dependencies (planning.rs:34)
        |
        |  project_release_plan(...)            (release_plan.rs:280)
        |  project_release_plan_with_consumer(...) (release_plan.rs:356)
        v
ReleaseCIPlanV1                  (release_plan.rs:129 executable release graph)
                                 qualifications: Vec<QualificationJob> (release_plan.rs:135)
                                 gate_job_id                      (release_plan.rs:137)
                                 aggregate: AggregateJob          (release_plan.rs:139)
                                 finalization: FinalizationSettings (release_plan.rs:141)
                                 staging: Option<StagingJob>     (release_plan.rs:144)
                                 consumer_validators: BTreeMap   (release_plan.rs:148)
        |
        |  render_release_github / render_reusable_release_github
        v
release.yml                      (checked in, drift-checked)
```

Reusable workflow identity resolution defaults to exact-tag identity. An
optional `GitHubPolicy.release_identity_mode = "v_prefixed_stable_semver"`
selects the finite stable-version mapping described in
[`docs/release-identity-policy.md`](../docs/release-identity-policy.md). The
renderer emits this source-controlled mode as a fixed resolver argument. The
runtime source tag still selects checkout, the source revision still identifies
the checked-out commit, and only the manifest/artifact identity uses the
unprefixed version. Exact-tag workflows retain their prior serialization and
rendered bytes when the field is absent.

`ReleaseCIPlanV1::validate` (`release_plan.rs:170`) is the structural gate on this graph: exact
qualification/target count agreement, the fixed `required_gate` and `aggregate`
job ids, handoff-name bounds, canonical ordering, and `schema_version == 1`.
`from_json` (`release_plan.rs:159`) additionally bounds the document against
`MAX_RELEASE_PLAN_JSON` before parsing. A malformed or reordered graph is
rejected rather than rendered.

## Job vocabulary

| Job | Type | Role |
|---|---|---|
| build (one per target) | `TargetJob` (`planning.rs:40`) | builds candidate bytes; carries `outputs: Vec<BuildOutput>` (`planning.rs:50`) mapping contract logical slots to an explicit Cargo package + binary |
| qualification (one per target) | `QualificationJob` (`release_plan.rs:28`) | runs the target's smoke binding and records evidence |
| required gate | id fixed to `required_gate` (`release_plan.rs:334`) | fails the release unless every required target's evidence passes |
| aggregate | `AggregateJob` (`release_plan.rs:52`) | finalizes, producing the manifest and the final handoff |
| staging (optional) | `StagingJob` (`release_plan.rs:111`) | materializes the staging payload and stages a draft; `Option`, so graphs without staging are valid |
| consumer validation (optional) | `ConsumerValidatorV1` (`consumer_validator.rs:43`) | per-target external validator, keyed by canonical target triple |

Two distinctions are load-bearing:

- **Qualification is intent in `CIPlan`, execution in `ReleaseCIPlanV1`.**
  `QualificationState` (`planning.rs:68`) has exactly one variant, `Unresolved`, and
  `QualificationIntent` (`planning.rs:56`) is documented as never reporting a pass. A
  `CIPlan` therefore cannot claim that anything was qualified. Evidence appears
  only after the qualification jobs run — see `core-qualification.md`.
- **Staging is optional and additive.** `staging` and `consumer_validators` are
  `#[serde(default, skip_serializing_if = ...)]` (`release_plan.rs:143`, `release_plan.rs:147`), so a graph
  that does not stage is a valid, smaller graph rather than a special case.

## Gates and aggregation

- `evaluate_gate` (`release_plan.rs:561`) and `evaluate_gate_with_consumer` (`release_plan.rs:420`) decide
  whether required targets passed. This is the CI-layer expression of the same
  fail-closed rule core enforces at finalization: missing or failing required
  evidence is a failure, never a pass and never a skip.
- `aggregate_finalize` (`release_plan.rs:621`) and `aggregate_finalize_with_consumer`
  (`release_plan.rs:484`) drive the aggregate job, producing the final handoff artifact.
- `with_github_draft_staging` (`release_plan.rs:527`) attaches the staging job to a graph.
- `AggregateOutcome` (`release_plan.rs:64`), `FinalizationSettings` (`release_plan.rs:78`), and
  `ArchiveEncodingWrapper` (`release_plan.rs:90`) describe what the aggregate job is
  permitted to do. The archive wrapper exists so the CI layer can constrain
  finalization to the allowed encodings without redefining them — `TarGzip`
  only, per `core-finalization.md`.

## The file-in/file-out seam with the CLI

This is the contract that makes generated CI work, and it is worth understanding
before reading either crate in detail.

Eggpack does not generate a workflow that calls library functions. It generates
a workflow that shells out to the `eggpack` binary, and the two sides agree on
a fixed set of file names and JSON documents:

| Constant | Value | Role |
|---|---|---|
| `BUILD_HANDOFF_FILE` (`policy.rs:542`) | `build-handoff.json` | build step output consumed by qualification |
| `CANDIDATES_DIR` (`policy.rs:548`) | `candidates` | candidate bytes directory |
| `QUALIFICATION_EVIDENCE_FILE` (`policy.rs:545`) | `evidence.json` | per-target qualification evidence |
| `GATE_OUTCOME_FILE` (`policy.rs:551`) | `gate-outcome.json` | gate decision consumed by aggregate |
| `CONSUMER_EVIDENCE_FILE` (`consumer_validator.rs:12`) | `consumer-evidence.json` | external validator output |
| `RUNTIME_IDENTITY_DIR` (`identity.rs:17`) | `eggpack-runtime` | reusable-workflow runtime identity directory |
| `RUNTIME_RELEASE_PLAN` (`identity.rs:20`) | `eggpack-runtime/release-plan.json` | runtime-resolved plan |
| `RUNTIME_CI_PLAN` (`identity.rs:23`) | `eggpack-runtime/release-ci-plan.json` | runtime-resolved CI plan |
| `RUNTIME_GITHUB_POLICY` (`identity.rs:26`) | `eggpack-runtime/github-draft.json` | runtime-resolved draft policy |

The encode/decode pairs (`encode_qualification_evidence` `release_plan.rs:545`,
`decode_qualification_evidence` `release_plan.rs:550`, `decode_consumer_evidence` `release_plan.rs:406`,
`reconstruct_attempt` `handoff.rs:186`, `validate_build_artifact_dir` `handoff.rs:243`,
`validate_qualification_artifact_dir` `handoff.rs:292`, `stage_build_artifact_dir`
`handoff.rs:333`) are what make the two processes agree. Each is a fail-closed
validation of a document read from disk, not a trust boundary — see
`ci-consumer-seam.md`.

The consequence for review: a change to a file name, a JSON shape, or a
validation rule here is a change to the contract between two independently
versioned processes, and it will fail at CI runtime rather than at compile time.

## Boundaries / non-goals

- **No execution.** `eggpack-ci` plans, renders, and checks. It does not build,
  qualify, or finalize anything itself; the generated workflow causes `eggpack`
  to do that.
- **No network, no credentials, no publication.** All GitHub interaction lives
  in `eggpack-github`. Staging intent is *described* here and *performed* there.
- **No policy invention.** Runner images, action pins, and tool policy are
  supplied by the caller. See `ci-rendering.md`.
- **No redefinition of upstream vocabulary.** Job graph types reference
  `eggpack-core` and `eggpack-contract` types (`PlannedTarget`,
  `LogicalOutputSelector`, `Qualification`, `HostRequirement`) rather than
  restating them. `BuildOutput` (`planning.rs:76`) is the one place where Cargo-level
  detail enters, and it is explicit: package and binary are named, never
  inferred.
- **No producer rule of its own.** The host/target match, the compatibility
  floor, the exact-tag safety rule, the SHA-256 hex acceptance set, and the
  file digest each have one owner outside this crate. `CIPlan::validate`
  (`planning.rs:234`) calls `eggpack_core::validate_policy` and
  `eggpack_core::validate_floor_applicability` instead of re-deriving the
  toolchain, floor, or native-qualification host rules;
  `validate_sha256_hex` (`policy.rs:161`) delegates to the manifest rule through core's
  re-export; the exact-tag check calls
  `eggpack_core::validate_exact_tag_safety`; and candidate hashing calls
  `eggpack_core::file_digest`, whose refusal is a typed error rather than a
  zero-size placeholder. Each call site keeps its own diagnostic wording, so
  the messages this crate emits are unchanged. The full rule map is in
  [validation-model.md](validation-model.md).
- **No authenticity claim.** Digests and sizes are integrity facts.
- **No release selection, update, or rollback.** Those belong to the product
  repository and to Eggup respectively.

## Dependencies / dependents

Dependencies: `eggpack-contract`, `eggpack-core`, `serde`, `serde_json`,
`sha2`. Dev-dependencies: `serde_yaml` (for test assertions only), plus
`eggpack-manifest`, `eggpack-bootstrap`, `eggpack-github`, and `tokio` so the
crate's own end-to-end orchestration tests can drive the real downstream crates
without a network.

Notably absent: `command-group` and any YAML emitter. The process-execution
contract is nevertheless **shared**: the consumer validator runs through
`eggpack_core::run_interpreter_bounded` (`crates/eggpack-ci/src/consumer_validator.rs:535`),
so the crate gets core's process-group (or job-object) termination, bounded
reader-thread joins, the shared environment allowlist, and the shared output
bounds without taking a `command-group` dependency of its own. See
`process-execution.md`.

Dependents: `eggpack-cli`, which exposes `ci generate` and `ci check` and the
nine internal `ci _*` runner commands that the generated workflow invokes.

## Related deep dives

- [overview.md](overview.md) — workspace-level view
- [ci-rendering.md](ci-rendering.md) — renderers, policy types, drift checking
- [ci-consumer-seam.md](ci-consumer-seam.md) — consumer validator, runtime
  identity, artifact handoffs
- [core.md](core.md) — the pipeline this crate projects
- [cli.md](cli.md) — the `eggpack` binary that executes the generated steps
- [determinism.md](determinism.md) — canonical ordering and the hand-concatenated
  YAML emitter
- [validation-model.md](validation-model.md) — the fail-closed rules enforced on
  every document in this crate
- [process-execution.md](process-execution.md) — the shared bounded runner the
  consumer validator runs through
- [github.md](github.md) — where staging intent is actually performed
- [testing-and-portability.md](testing-and-portability.md) — the golden corpus
  and the Windows lane for this crate
- [principles-roadmap.md](principles-roadmap.md) — ADR-0003 (checked-in generated
  CI and the publication gate) and ADR-0004 (the build adapter restriction)
