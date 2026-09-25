# CI and Release Orchestration Milestone 002a — Generated Workflow Execution Wiring Corrective

Status: ready for handoff

Repository baseline: 388a8b056b05fb324122b1ac618e20bcb2963f77

Historical M002 plan:

- plans/implementation/ci-release-orchestration/002-qualification-aggregation-gates-and-drift-cli.md

Historical M002 closure:

- plans/closure/ci-release-orchestration/002-status.md

Source roadmap:

- plans/subsystems/ci-release-orchestration-roadmap.md

Applicable ADRs:

- plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md
- plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md

Primary class: corrective / generated-workflow execution / cross-job evidence wiring

## 1. Objective

Make the generated CI M002 release workflow operationally executable end-to-end.

The M002 production libraries and CLI contain the intended build-handoff, qualification, gate, and aggregate primitives, but post-closure review found that the GitHub renderer does not wire those primitives together with the files and arguments the CLI actually requires.

M002a must correct the renderer/CLI handoff contract and prove execution using the generated workflow shape rather than only YAML/golden inspection.

CI M003 remains blocked until M002a closes, in addition to its separate staging-adapter prerequisite.

## 2. Post-closure defects requiring correction

At repository baseline 388a8b056b05fb324122b1ac618e20bcb2963f77:

1. M001 build jobs upload candidate bytes only. They do not invoke `eggpack ci _capture-build`.
2. Qualification jobs download `eggpack-build-handoff-<target>`, but no build job creates that artifact.
3. Qualification jobs render only `eggpack ci _qualify-target --target <target>`, while the CLI requires:
   - contract;
   - release plan;
   - build bindings;
   - qualification bindings;
   - target;
   - candidate directory;
   - build-handoff document;
   - output directory.
4. The generated qualification artifact does not contain the complete per-target input set the aggregate command expects.
5. The required gate job does not download qualification evidence and renders `eggpack ci _evaluate-gate` without required CLI inputs.
6. The aggregate job renders `eggpack ci _aggregate` without required CLI inputs.
7. The aggregate job downloads only evidence artifacts, while the CLI expects per-target build handoff, evidence, and candidate bytes.
8. Golden fixtures validate deterministic text/YAML shape but do not execute the generated build -> capture -> qualify -> gate -> aggregate path.

These are production wiring defects. The historical M002 closure remains useful evidence for the underlying library primitives, but it is insufficient evidence that generated release CI is executable.

## 3. Boundaries to preserve

Do not redesign:

- M001 CIPlan semantics;
- M003 qualification semantics;
- M004 finalization semantics;
- DistributionContract/ReleaseManifest;
- GitHub publication/staging;
- read-only permission policy.

Do not move core qualification/finalization logic into shell.

The corrective owns only the generated workflow execution contract, CLI file layout, and end-to-end qualification evidence.

## 4. Corrective invariants

- every generated CLI invocation includes all required explicit inputs;
- no hidden repository discovery is introduced;
- build jobs create the exact artifact qualification jobs consume;
- qualification output contains the exact complete per-target bundle aggregate consumes;
- gate evaluates downloaded structured evidence;
- aggregate receives exact per-target directories;
- no cross-job absolute paths are serialized;
- artifact transport is not trusted as integrity evidence;
- target/release/source/selector identities are revalidated at every boundary;
- optional target incompleteness suppresses finalization rather than creating a partial release;
- GitHub permissions remain read-only;
- all Actions and Eggpack runtime tooling remain immutable-pinned;
- M002a does not stage or publish a release.

## 5. Required production changes

### A. Explicit checked-in workflow input paths

The renderer needs explicit repository-relative input paths for the CLI files it invokes.

Add a bounded GitHub release-input policy/type, e.g. `GitHubReleaseInputsV1`, containing exact repository-relative paths for:

- DistributionContract;
- ReleasePlan;
- BuildBindingsV1;
- QualificationBindingsV1;
- ReleaseCIPlanV1.

Requirements:

- relative paths only;
- no `..`, empty segments, NUL, backslash ambiguity, drive prefixes, or absolute roots;
- bounded length;
- deterministic serialization;
- no file discovery/globbing;
- paths live in renderer/provider policy, not portable release identity.

The renderer must pass these paths explicitly to every runner command.

### B. Canonical build handoff artifact

Define one exact per-target build artifact layout:

~~~text
build-handoff.json
candidates/
  <BuildHandoffOutput.relative_path>
~~~

Build jobs must:

1. build candidate bytes using existing M002 command semantics;
2. install/use pinned Eggpack CLI;
3. invoke `_capture-build`;
4. stage candidate bytes under the canonical relative names;
5. write exact observed sizes into `build-handoff.json`;
6. upload the entire canonical directory as one deterministic artifact.

Update `_capture-build` as needed so it can derive the actual Cargo output locations from ReleasePlan/BuildBindings and the known Cargo target root, then copy or hard-link bytes into the canonical handoff directory.

Do not require generated shell to reproduce Cargo output-discovery logic.

The CLI must reject:

- missing/extra candidate;
- symlink/non-regular candidate;
- candidate path escape;
- output directory symlink;
- mismatched target/binding;
- zero-byte candidate.

### C. Canonical qualification artifact

Define one exact per-target qualification artifact layout:

~~~text
build-handoff.json
evidence.json
candidates/
  <same exact candidate files>
~~~

Qualification jobs must:

1. download the matching build artifact into a target-specific private directory;
2. invoke `_qualify-target` with explicit contract/plan/build-bindings/qualification-bindings/target/candidate/handoff/output arguments;
3. reconstruct and validate the M002 BuildAttempt;
4. invoke M003 qualification;
5. write evidence;
6. copy/hard-link the already-validated build-handoff and candidate files into the qualification output directory;
7. upload the complete directory as one deterministic qualification artifact.

Do not re-download candidate bytes from another origin.

### D. Gate input layout

The required-gate job must download every qualification artifact to a deterministic target directory:

~~~text
eggpack-inputs/
  <canonical-target>/
    build-handoff.json
    evidence.json
    candidates/...
~~~

Prefer one explicit download step per target to avoid provider-specific pattern/merge ambiguity.

Update `_evaluate-gate` to consume either:

- `--inputs-dir` with the canonical target layout; or
- another equally explicit deterministic layout.

It must load each target's `evidence.json`, call the existing `evaluate_gate`, and write an exact gate outcome file.

Generated command must pass:

- `--ci-plan <explicit path>`;
- input directory;
- output outcome path.

Required failure semantics remain unchanged.

### E. Aggregate input layout

Aggregate must receive the same canonical per-target qualification directories.

Generated aggregate command must pass:

- contract path;
- release-plan path;
- CI-plan path;
- inputs-dir;
- private output-root;
- summary output path.

The CLI must reconstruct BuildAttempt from each target directory and call M004 through `aggregate_finalize`.

Upload the finalized internal release artifact only when the aggregate outcome is `Complete`.

If outcome is `SuppressedNonGatingIncomplete`, do not run/upload finalization output.

### F. Gate/aggregate job conditions

GitHub job dependency semantics must match typed gate semantics.

Required targets:

- build/qualification failure prevents Complete;
- evidence must still be collectable for diagnostic/gate evaluation where appropriate.

Optional targets:

- may use `continue-on-error` at the build/qualification job level;
- aggregate still must not create a partial release.

Use provider `if` conditions only to ensure typed evidence collection/evaluation occurs; do not bypass required failures.

Document the exact behavior in golden fixtures.

### G. Pinned CLI availability in build jobs

Because build jobs now invoke `_capture-build`, install/verify the same pinned Eggpack CLI in build jobs as qualification/gate/aggregate jobs.

No ambient unpinned `eggpack` executable.

### H. Qualification runtime policy

Audit generated handling of every M003 qualification classification.

If an Emulated classification requires runtime data such as QEMU sysroot, either:

- add a finite explicit provider runtime policy/path and pass it to the CLI; or
- reject generation for that classification until provisioned.

Do not silently invoke M003 with an empty runtime and claim full generated-CI support.

Native, DeferredNative, Emulated, and Structural behavior must match M003 semantics exactly.

## 6. End-to-end executable fixture requirement

M002a cannot close on YAML parsing/goldens alone.

Add a generated-workflow execution harness that uses the same rendered command/argument/layout contract.

Minimum end-to-end direct fixture:

~~~text
build candidate
  -> _capture-build
  -> build artifact layout
  -> _qualify-target
  -> qualification artifact layout
  -> _evaluate-gate
  -> _aggregate
  -> M004 finalized output + manifest
~~~

The test must assert:

- exact final artifact bytes;
- exact manifest identity/digest;
- exact build/evidence handoff layout;
- no missing required argument;
- no implicit path discovery.

Also execute orchestration fixtures for:

- bundle;
- archive;
- optional-target suppression;
- required qualification failure;
- evidence/candidate tamper.

These may use a local workflow-command harness rather than invoking GitHub itself, provided the renderer's exact arguments/layout are sourced from the same structured command model.

## 7. Renderer architecture requirement

Avoid another drift between CLI signatures and string-formatted YAML.

Prefer introducing a typed runner-command representation used by both:

- GitHub rendering;
- local executable orchestration tests.

For example:

~~~text
RunnerCommand
  program
  args[]
  env[]
  expected inputs
  expected outputs
~~~

The GitHub renderer serializes it to shell/YAML; tests can invoke it directly.

Do not add a generic arbitrary command DSL: the command enum remains finite to Eggpack's internal CI operations.

If implementation retains direct string rendering, every internal CLI command must have an exact parser/renderer round-trip test that proves all required arguments are present.

## 8. Required negative tests

At minimum:

- build artifact requested but never produced => renderer/integration test fails;
- missing contract path;
- missing ReleasePlan path;
- missing build bindings;
- missing qualification bindings;
- missing CI plan;
- wrong target handoff;
- wrong release/source identity;
- missing candidate;
- extra candidate;
- symlink candidate;
- tampered candidate after qualification;
- missing qualification artifact;
- gate input layout mismatch;
- aggregate input layout mismatch;
- optional target failure => no finalized output;
- required target failure => failing gate;
- Emulated qualification without required runtime policy rejects;
- output artifact upload step absent unless Complete.

## 9. Verification

Run at minimum:

~~~bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
git diff --check
~~~

Add a focused generated-orchestration integration command/test and record it explicitly in closure.

Hosted Linux stable, Linux Rust 1.89, macOS, and Windows must pass.

Where generated jobs are host-specific, closure must show at least one real end-to-end direct orchestration path on a supported hosted runner or an equivalent disposable local repository workflow execution. A parse-only golden is insufficient.

## 10. Planning/closure reconciliation

On registration:

- preserve CI M002 as historical implementation/closure evidence;
- annotate `plans/closure/ci-release-orchestration/002-status.md` with the post-closure executable-wiring defect;
- mark CI M002a as the active corrective;
- re-block CI M003 on M002a closure plus the existing explicit staging-adapter plan.

Do not alter Bootstrap status from this corrective.

## 11. Acceptance criteria

M002a closes only when:

- generated build jobs create the exact handoff qualification jobs download;
- every generated CLI invocation supplies all required arguments;
- qualification artifacts carry build handoff + evidence + candidate bytes in the canonical layout;
- gate downloads and evaluates real structured evidence;
- aggregate receives the exact layout it expects;
- direct generated orchestration executes through M004 finalization successfully;
- bundle/archive orchestration fixtures execute successfully;
- optional failure suppresses release output;
- required failure fails closed;
- Emulated qualification is either correctly provisioned or explicitly rejected;
- GitHub rendering remains deterministic/read-only/pinned;
- stable/MSRV/macOS/Windows CI passes;
- no unresolved medium-or-higher workflow execution finding remains.

Only after closure may CI M003 return to "blocked solely on staging adapter plan" / ready-for-staging-plan review.

## 12. Stop conditions

Stop and re-plan if:

- fixing the wiring requires repository discovery heuristics;
- the workflow must embed qualification/finalization logic in shell;
- write permissions become necessary;
- BuildAttempt/QualificationEvidence must become unstable public wire formats;
- arbitrary user commands/YAML are required;
- M003/M004 public semantics must materially change.

## 13. Closure evidence

Record:

- corrective implementation SHA;
- exact root cause;
- generated command model/input-path model;
- before/after golden workflow;
- build artifact directory example;
- qualification artifact directory example;
- exact gate/aggregate invocation arguments;
- direct end-to-end execution transcript/result;
- bundle/archive orchestration results;
- optional/required failure results;
- Emulated-runtime disposition;
- hosted matrix;
- unresolved findings;
- explicit CI M003 readiness disposition.
