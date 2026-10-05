# `eggpack-ci` — Deep Dive

Crate-level orientation for `eggpack-ci`: the largest component in the workspace
at 10221 lines in a single `src/lib.rs`. This file explains how that file divides
and what flows through it. It does not restate per-area detail — three deep
dives cover the parts.

`eggpack-ci` depends on `eggpack-contract` and `eggpack-core` only. It never
executes a release itself: it *plans* the release as a job graph, *renders* that
graph as a GitHub Actions workflow, *checks* the checked-in workflow for drift,
and *encodes/decodes* the artifacts the generated steps exchange. The actual
building, qualification, and finalization are performed by `eggpack-core` when
the generated workflow invokes the `eggpack` CLI.

Code baseline `fc072af`.

## The three deep dives

| Deep dive | Owns | Approximate region of `src/lib.rs` |
|---|---|---|
| [ci-rendering.md](ci-rendering.md) | policy types, `RunnerCommand`, the three renderers, drift checks | `:418-1843`, `:2924-3844` |
| [ci-consumer-seam.md](ci-consumer-seam.md) | the external consumer validator, runtime identity, artifact handoff formats | `:1844-2207`, `:3845-4430`, `:4430-4594` |
| this file | crate orientation, the projection model, job vocabulary, gates, aggregation | `:42-453`, `:2208-2740`, `:2741-2923` |

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
CIPlan                           (:42   provider-neutral build graph)
        |                        targets: Vec<TargetJob>  (:58)
        |                        required_aggregation_dependencies (:52)
        |
        |  project_release_plan(...)            (:2460)
        |  project_release_plan_with_consumer(...) (:2536)
        v
ReleaseCIPlanV1                  (:2309 executable release graph)
                                 qualifications: Vec<QualificationJob> (:2315)
                                 gate_job_id                      (:2317)
                                 aggregate: AggregateJob          (:2319)
                                 finalization: FinalizationSettings (:2321)
                                 staging: Option<StagingJob>     (:2324)
                                 consumer_validators: BTreeMap   (:2328)
        |
        |  render_release_github / render_reusable_release_github
        v
release.yml                      (checked in, drift-checked)
```

`ReleaseCIPlanV1::validate` (`:2350`) is the structural gate on this graph: exact
qualification/target count agreement, the fixed `required_gate` and `aggregate`
job ids, handoff-name bounds, canonical ordering, and `schema_version == 1`.
`from_json` (`:2339`) additionally bounds the document against
`MAX_RELEASE_PLAN_JSON` before parsing. A malformed or reordered graph is
rejected rather than rendered.

## Job vocabulary

| Job | Type | Role |
|---|---|---|
| build (one per target) | `TargetJob` (`:58`) | builds candidate bytes; carries `outputs: Vec<BuildOutput>` (`:94`) mapping contract logical slots to an explicit Cargo package + binary |
| qualification (one per target) | `QualificationJob` (`:2208`) | runs the target's smoke binding and records evidence |
| required gate | id fixed to `required_gate` (`:2357`) | fails the release unless every required target's evidence passes |
| aggregate | `AggregateJob` (`:2232`) | finalizes, producing the manifest and the final handoff |
| staging (optional) | `StagingJob` (`:2291`) | materializes the staging payload and stages a draft; `Option`, so graphs without staging are valid |
| consumer validation (optional) | `ConsumerValidatorV1` (`:3873`) | per-target external validator, keyed by canonical target triple |

Two distinctions are load-bearing:

- **Qualification is intent in `CIPlan`, execution in `ReleaseCIPlanV1`.**
  `QualificationState` (`:86`) has exactly one variant, `Unresolved`, and
  `QualificationIntent` (`:74`) is documented as never reporting a pass. A
  `CIPlan` therefore cannot claim that anything was qualified. Evidence appears
  only after the qualification jobs run — see `core-qualification.md`.
- **Staging is optional and additive.** `staging` and `consumer_validators` are
  `#[serde(default, skip_serializing_if = ...)]` (`:2323`, `:2327`), so a graph
  that does not stage is a valid, smaller graph rather than a special case.

## Gates and aggregation

- `evaluate_gate` (`:2741`) and `evaluate_gate_with_consumer` (`:2600`) decide
  whether required targets passed. This is the CI-layer expression of the same
  fail-closed rule core enforces at finalization: missing or failing required
  evidence is a failure, never a pass and never a skip.
- `aggregate_finalize` (`:2801`) and `aggregate_finalize_with_consumer`
  (`:2664`) drive the aggregate job, producing the final handoff artifact.
- `with_github_draft_staging` (`:2707`) attaches the staging job to a graph.
- `AggregateOutcome` (`:2244`), `FinalizationSettings` (`:2258`), and
  `ArchiveEncodingWrapper` (`:2270`) describe what the aggregate job is
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
| `BUILD_HANDOFF_FILE` (`:941`) | `build-handoff.json` | build step output consumed by qualification |
| `CANDIDATES_DIR` (`:945`) | `candidates` | candidate bytes directory |
| `QUALIFICATION_EVIDENCE_FILE` (`:943`) | `evidence.json` | per-target qualification evidence |
| `GATE_OUTCOME_FILE` (`:947`) | `gate-outcome.json` | gate decision consumed by aggregate |
| `CONSUMER_EVIDENCE_FILE` (`:3845`) | `consumer-evidence.json` | external validator output |
| `RUNTIME_IDENTITY_DIR` (`:4430`) | `eggpack-runtime` | reusable-workflow runtime identity directory |
| `RUNTIME_RELEASE_PLAN` (`:4432`) | `eggpack-runtime/release-plan.json` | runtime-resolved plan |
| `RUNTIME_CI_PLAN` (`:4434`) | `eggpack-runtime/release-ci-plan.json` | runtime-resolved CI plan |
| `RUNTIME_GITHUB_POLICY` (`:4436`) | `eggpack-runtime/github-draft.json` | runtime-resolved draft policy |

The encode/decode pairs (`encode_qualification_evidence` `:2725`,
`decode_qualification_evidence` `:2730`, `decode_consumer_evidence` `:2586`,
`reconstruct_attempt` `:2005`, `validate_build_artifact_dir` `:2062`,
`validate_qualification_artifact_dir` `:2111`, `stage_build_artifact_dir`
`:2152`) are what make the two processes agree. Each is a fail-closed
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
  restating them. `BuildOutput` (`:94`) is the one place where Cargo-level
  detail enters, and it is explicit: package and binary are named, never
  inferred.
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
contract that core enforces is **not** shared with this crate — the consumer
validator has its own independent runner. See `process-execution.md` for that
gap.

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
- [process-execution.md](process-execution.md) — the independent runner used by
  the consumer validator
- [github.md](github.md) — where staging intent is actually performed
- [testing-and-portability.md](testing-and-portability.md) — the golden corpus
  and the Windows lane for this crate
- [principles-roadmap.md](principles-roadmap.md) — ADR-0003 (checked-in generated
  CI and the publication gate) and ADR-0004 (the build adapter restriction)
