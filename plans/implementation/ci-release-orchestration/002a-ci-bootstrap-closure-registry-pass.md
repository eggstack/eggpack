# CI/Bootstrap M002a Coordinated Closure and Registry Pass

Status: closed

Closure disposition (2026-09-25): this coordinating pass is complete and creates no third capability closure. Completion evidence: `plans/closure/ci-release-orchestration/002a-status.md` and `plans/closure/bootstrap-installers/002a-status.md` (both citing implementation SHA `4d2270afe7de10bdff92563ed0f51a41ba807a04` and hosted run 36154905956, attempt 1, all lanes green); reconciled `plans/subsystems/ci-release-orchestration-roadmap.md`, `plans/subsystems/bootstrap-installers-roadmap.md`, and `plans/registry.md` with CI M002a and Bootstrap M002a closed; required local verification passed with no unresolved medium-or-higher finding; final repository CI green. CI M003 remains blocked solely on the explicit staging-adapter plan; Bootstrap M003 remains blocked solely on real adoption evidence/candidate review. No downstream implementation plan is authorized by this closeout.

Repository baseline: `4d2270afe7de10bdff92563ed0f51a41ba807a04`

Primary corrective plans:

- `plans/implementation/ci-release-orchestration/002a-generated-workflow-execution-wiring-corrective.md`
- `plans/implementation/bootstrap-installers/002a-powershell-archive-runtime-evidence-corrective.md`

Historical M002 closures:

- `plans/closure/ci-release-orchestration/002-status.md`
- `plans/closure/bootstrap-installers/002-status.md`

Source roadmaps:

- `plans/subsystems/ci-release-orchestration-roadmap.md`
- `plans/subsystems/bootstrap-installers-roadmap.md`

Observed hosted evidence at handoff:

- implementation SHA `4d2270afe7de10bdff92563ed0f51a41ba807a04`;
- hosted CI run `36154905956`, attempt 1, completed successfully;
- Linux stable passed;
- Linux Rust 1.89 passed;
- macOS passed;
- Windows passed;
- Windows focused `m002a_powershell_archive_runtime` passed;
- Windows focused `m002a_generated_orchestration_executes_end_to_end` passed;
- Windows focused `generated_orchestration_cli_executes_capture_to_aggregate` passed.

Primary class: closure / verification / planning-control reconciliation

## 1. Objective

Close CI Orchestration M002a and Bootstrap Installers M002a only after independently verifying that the implementation at the named baseline satisfies both corrective plans and that the hosted evidence actually proves the required runtime paths.

Then reconcile the two subsystem roadmaps and `plans/registry.md` so:

- both M002a correctives are marked closed;
- the historical M002 closures remain historical records with their post-closure annotations intact;
- CI M003 is no longer blocked by M002a, but remains blocked until an explicit staging-adapter implementation plan is reviewed/registered;
- Bootstrap M003 is no longer blocked by M002a, but remains blocked until real consumer-adoption evidence and candidate review are available;
- no downstream implementation plan is silently authorized by this closeout alone.

This pass is not authorization for production-code refactoring, staging/publication work, or consumer adoption.

## 2. Why this pass is ready

The corrective implementation has landed at `4d2270afe7de10bdff92563ed0f51a41ba807a04`.

Post-implementation review already confirms the principal corrective mechanisms are present:

### CI M002a

- `GitHubReleaseInputsV1` supplies explicit bounded repository-relative inputs;
- finite `RunnerCommand` owns the CLI invocation contract;
- generated build jobs invoke `_capture-build`;
- build jobs upload canonical `build-handoff.json + candidates/` artifacts;
- qualification jobs download that exact handoff and pass contract, ReleasePlan, build bindings, qualification bindings, target, candidate directory, handoff path, and output directory;
- qualification output contains the complete handoff/evidence/candidate layout;
- gate jobs download per-target qualification directories and invoke `_evaluate-gate --inputs-dir ...`;
- aggregate jobs receive the same canonical per-target layout and invoke `_aggregate` with explicit contract/plan/CI-plan/input/output paths;
- Emulated qualification requires explicit provider sysroot policy or generation rejects;
- direct/bundle/archive executable orchestration tests exist;
- the CLI harness exercises capture -> qualify -> gate -> aggregate using the same `RunnerCommand` argument model.

### Bootstrap M002a

- Windows CI explicitly verifies `pwsh` 7 and `tar.exe`;
- focused PowerShell TarGzip runtime executes on the Windows lane and may not silently skip there;
- positive archive installation verifies exact installed bytes and nested-source flattening;
- wrong archive size/SHA cases are distinct outer-integrity failures;
- malicious inner archive fixtures update outer artifact size/SHA to match the tampered archive so inner defenses are actually reached;
- traversal, absolute, backslash, symlink, directory, missing, and extra member cases are represented;
- member size/SHA evidence failures are exercised;
- POSIX malicious-member tests use the same valid-outer-digest principle;
- rollback/pre-existing preservation behavior is covered by the implementation/test matrix.

No new architecture decision is required to close these correctives.

## 3. Invariants

This pass must preserve all of the following:

- historical M002 closure records are not rewritten to conceal the later defects;
- M002a closure records cite the actual implementation and observed verification, not intended behavior;
- one green workspace run is not substituted for focused corrective evidence;
- closure records distinguish tests actually executed from tests merely present in source;
- unavailable evidence is recorded as unavailable rather than inferred;
- CI M002a closure does not authorize GitHub Release staging or publication;
- Bootstrap M002a closure does not authorize ordinary update/rollback/service authority;
- CI M003 remains blocked on a separate staging-adapter plan;
- Bootstrap M003 remains blocked on real consumer evidence/candidate review;
- no production source changes are made during this pass unless verification exposes a real defect;
- if a medium-or-higher defect is found, stop closure and write a new corrective rather than weakening acceptance criteria.

## 4. In scope

- re-review implementation SHA `4d2270afe7de10bdff92563ed0f51a41ba807a04`;
- inspect hosted run `36154905956` and its job/step outcomes;
- run the closure verification command set;
- verify CI M002a acceptance evidence;
- verify Bootstrap M002a acceptance evidence;
- write `plans/closure/ci-release-orchestration/002a-status.md`;
- write `plans/closure/bootstrap-installers/002a-status.md`;
- update both subsystem roadmaps;
- update `plans/registry.md`;
- update README/docs only if status language is stale;
- record downstream readiness/blocking precisely;
- run final planning consistency and CI checks.

## 5. Out of scope

- CI M003 staging adapter design or implementation;
- GitHub Release API integration;
- publication credentials/permissions;
- Bootstrap M003 consumer adoption;
- Eggup receipt design;
- new archive formats;
- new qualification classes;
- refactoring already-qualified production code;
- rewriting historical M002 closure records except to add a factual cross-reference if strictly needed.

## 6. CI M002a closure work

Create `plans/closure/ci-release-orchestration/002a-status.md` following `plans/closure/README.md`.

The closure must include:

### Reviewed baseline

- corrective plan path;
- historical M002 closure path;
- reviewed implementation SHA `4d2270afe7de10bdff92563ed0f51a41ba807a04`;
- hosted run `36154905956`, attempt 1.

### Root-cause/corrective finding

State the historical defect precisely:

- generated M002 workflows did not create the build handoff qualification consumed;
- internal CLI commands were rendered without their mandatory file/path arguments;
- gate/aggregate jobs did not receive the canonical per-target input layout;
- YAML/golden validity therefore did not prove operational execution.

Then show how M002a corrects this through:

- `GitHubReleaseInputsV1`;
- finite `RunnerCommand`;
- canonical build handoff layout;
- canonical qualification layout;
- explicit gate/aggregate inputs;
- Emulated runtime policy;
- executable orchestration tests.

### Requirement-to-evidence matrix

At minimum cover:

- build job creates canonical handoff;
- qualification consumes exact build handoff;
- every generated CLI command contains required explicit arguments;
- qualification artifact includes handoff + evidence + candidates;
- gate consumes structured evidence from canonical target directories;
- aggregate receives canonical target directories;
- required failures fail closed;
- optional/non-gating incomplete evidence suppresses finalized output;
- direct orchestration finalizes successfully;
- bundle orchestration finalizes successfully;
- archive orchestration finalizes successfully;
- candidate/evidence identity/tamper checks remain enforced;
- Emulated qualification without provider runtime rejects;
- generated workflow remains deterministic, pinned, and read-only.

### Hosted evidence

Record the exact Windows focused steps from run `36154905956`:

1. PowerShell archive prerequisites;
2. PowerShell archive runtime;
3. generated orchestration execution;
4. orchestration CLI harness;
5. workspace check/tests.

Also record Linux stable, Linux 1.89, and macOS success.

### Roadmap disposition

If no medium-or-higher finding remains:

- CI M002a -> closed;
- CI M002 -> closed historically, corrective satisfied;
- CI M003 -> still blocked, but now only on the explicit staging-adapter plan/review;
- no staging/publication implementation plan is created by this pass.

## 7. Bootstrap M002a closure work

Create `plans/closure/bootstrap-installers/002a-status.md` following `plans/closure/README.md`.

The closure must include:

### Reviewed baseline

- corrective plan path;
- historical M002 closure path;
- implementation SHA `4d2270afe7de10bdff92563ed0f51a41ba807a04`;
- hosted run `36154905956`, attempt 1.

### Root-cause/corrective finding

State the historical evidence gap precisely:

- PowerShell archive output was previously parsed but not runtime-qualified;
- malicious archive fixtures could fail at outer archive integrity before reaching intended inner member defenses.

Then show how M002a corrects this through:

- Windows hard requirement for `pwsh` 7 + `tar.exe`;
- focused PowerShell archive runtime test;
- raw test-only tar fixture support for unsafe names/types;
- outer manifest digest recomputation for tampered archives;
- intended inner guard assertions;
- member evidence negative tests;
- no-overwrite/pre-existing preservation/rollback evidence.

### Requirement-to-evidence matrix

At minimum cover:

- positive PowerShell TarGzip install;
- exact installed bytes;
- nested source flattening;
- repeat/no-overwrite failure;
- pre-existing destination preservation;
- wrong outer size;
- wrong outer SHA;
- missing member after valid outer digest;
- extra member after valid outer digest;
- traversal member after valid outer digest;
- absolute member after valid outer digest;
- backslash member after valid outer digest;
- symlink member after valid outer digest;
- directory member after valid outer digest;
- wrong member size;
- wrong member SHA;
- missing `tar.exe` bounded failure;
- rollback/pre-existing-path evidence;
- corrected POSIX inner-defense matrix;
- deterministic PowerShell rendering/parser checks.

### Hosted evidence

Record that the Windows lane explicitly verified `pwsh` 7 and `tar.exe` before running the focused M002a PowerShell archive runtime test, and that the focused test passed.

Record Linux stable, Linux 1.89, macOS, and Windows overall success.

### Roadmap disposition

If no medium-or-higher finding remains:

- Bootstrap M002a -> closed;
- Bootstrap M002 -> closed historically, corrective satisfied;
- Bootstrap M003 -> remains blocked on real consumer-adoption evidence/candidate review only;
- no adoption/receipt implementation plan is created by this pass.

## 8. Verification commands

Run and record exact results:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test -p eggpack-ci --all-targets --all-features --locked m002a_generated_orchestration_executes_end_to_end -- --nocapture
cargo test -p eggpack-cli --all-targets --all-features --locked generated_orchestration_cli_executes_capture_to_aggregate -- --nocapture
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked
./scripts/check-local.sh
git diff --check
```

On Windows qualification evidence, confirm from hosted run/job steps rather than claiming local non-Windows execution:

```text
Verify PowerShell archive prerequisites (pwsh 7 + tar.exe)
Focused PowerShell archive runtime (Bootstrap M002a)
Focused generated-orchestration execution (CI M002a)
Focused orchestration CLI harness (CI M002a)
```

Do not re-run hosted workflows merely to inflate evidence unless the existing run is unavailable or a new code/doc change affects executable behavior.

## 9. Closure consistency checks

Before writing status transitions:

- confirm implementation SHA is ancestor/current baseline being closed;
- confirm run `36154905956` corresponds to that SHA and attempt 1;
- confirm all four hosted lanes passed;
- confirm Windows focused steps passed;
- confirm no newer production-code commit invalidates the evidence;
- confirm no open medium-or-higher finding emerged during local verification;
- confirm closure paths do not already exist with conflicting evidence.

## 10. Registry and roadmap reconciliation

After both closure records are complete:

### CI roadmap

Change:

- M002 from "closed historically / corrective active" to "closed historically; corrective closed";
- M002a from "ready" to "closed";
- add closure path `plans/closure/ci-release-orchestration/002a-status.md`;
- M003 blocker becomes "explicit staging adapter plan" only.

### Bootstrap roadmap

Change:

- M002 from "closed historically / corrective active" to "closed historically; corrective closed";
- M002a from "ready" to "closed";
- add closure path `plans/closure/bootstrap-installers/002a-status.md`;
- M003 blocker becomes "real adoption evidence and candidate review" only.

### Registry

Update all of:

- current evidence baselines;
- active subsystem roadmap statuses;
- dependency-ready implementation table;
- blocked/planned table;
- narrative state paragraph;
- immediate execution graph;
- next handoff.

The registry must not state that CI M003 or Bootstrap M003 implementation is ready.

The correct post-close state is:

```text
CI M002a          CLOSED
CI M003           BLOCKED — staging adapter plan not yet reviewed/registered

Bootstrap M002a   CLOSED
Bootstrap M003    BLOCKED — real consumer adoption evidence/candidate review
```

Eggup Interoperability M003 remains independently ready to plan if its existing prerequisites remain unchanged.

## 11. Documentation cleanup

Review:

- root README;
- `crates/eggpack-ci/README.md`;
- `crates/eggpack-cli/README.md`;
- `crates/eggpack-bootstrap/README.md`.

Only correct stale status/evidence wording.

Do not rewrite architecture descriptions that are already accurate.

## 12. Failure/restart semantics

This pass is evidence-first.

If any required verification fails:

- do not write a closed status;
- leave the affected M002a in closing/active state;
- record the failure;
- determine whether the issue is environment-only or a product/test defect;
- if medium-or-higher product/test defect exists, write a new corrective plan.

If one subsystem closes and the other fails, close only the proven subsystem and keep the other active. Registry/roadmaps must reflect the split state.

No "all-or-nothing" administrative transaction is required across the two closures.

## 13. Compatibility/security review

Confirm closure does not alter:

- public contract/manifest schemas;
- M003 qualification semantics;
- M004 finalization semantics;
- first-install-only bootstrap boundary;
- GitHub permission policy;
- publication authority;
- Eggup ownership.

Security closure must explicitly retain:

- read-only generated CI permissions;
- immutable action/Eggpack pins;
- bounded repository-relative release input paths;
- candidate/evidence identity validation;
- outer archive integrity before extraction;
- inner archive inventory/path/type defenses;
- no elevation/update authority in bootstrap scripts.

## 14. Acceptance criteria

This closure/registry pass is complete only when:

- CI M002a closure record exists and is evidence-complete;
- Bootstrap M002a closure record exists and is evidence-complete;
- both closures cite implementation SHA `4d2270afe7de10bdff92563ed0f51a41ba807a04` unless a later verification-only/doc SHA is explicitly distinguished;
- hosted run `36154905956` is accurately recorded;
- required local verification passes;
- no unresolved medium-or-higher finding remains;
- both subsystem roadmaps match the closure records;
- registry matches both roadmaps;
- historical M002 closures remain intact;
- CI M003 remains blocked only on staging-adapter planning;
- Bootstrap M003 remains blocked only on adoption evidence/candidate review;
- final repository CI is green.

## 15. Stop conditions

Stop and write a new corrective instead of closing if:

- the generated release workflow still cannot execute the CLI contract it renders;
- optional-target behavior can produce a partial finalized release;
- Emulated qualification can silently run without explicit provider runtime;
- PowerShell archive runtime can skip on the qualifying Windows lane;
- malicious archive tests still fail before intended inner guards;
- rollback can remove pre-existing paths;
- closure would require weakening either corrective plan's acceptance criteria;
- new production code is required beyond a trivial verification-only fix.

## 16. Closure evidence for this handoff

This coordinating handoff does not create a third capability closure. Its completion evidence is:

- `plans/closure/ci-release-orchestration/002a-status.md`;
- `plans/closure/bootstrap-installers/002a-status.md`;
- reconciled CI and Bootstrap roadmaps;
- reconciled `plans/registry.md`;
- final green repository CI.

The implementation agent should commit the two closure records and planning reconciliation together when practical, but split commits are acceptable if each intermediate planning state remains truthful.

## 17. Handoff notes

Start from `4d2270afe7de10bdff92563ed0f51a41ba807a04`.

Do not modify production code simply because this is an implementation-plan handoff. The expected work is verification, closure evidence, and planning reconciliation.

If verification confirms the reviewed state, close both M002a correctives. Do not author CI M003 or Bootstrap M003 implementation plans in the same pass.
