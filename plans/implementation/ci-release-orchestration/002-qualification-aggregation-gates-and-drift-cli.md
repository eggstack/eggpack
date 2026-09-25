# CI and Release Orchestration Milestone 002 — Qualification/Aggregation Gates and Drift CLI

Status: closed

Closure record: `plans/closure/ci-release-orchestration/002-status.md`

Repository baseline: 66f818a67f8681010d07301f4a1d7ca5aadf2f5b

Source roadmap: plans/subsystems/ci-release-orchestration-roadmap.md

Hard dependency closures:

- CI M001: plans/closure/ci-release-orchestration/001-status.md
- Build/Qualification M003: plans/closure/build-qualification/003-status.md
- Build/Qualification M004: plans/closure/build-qualification/004-status.md

Applicable ADRs:

- plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md
- plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md

Primary class: capability / orchestration / deterministic generation / CI security

## 1. Objective

Extend closed CI M001 so generated checked-in release workflows can build exact candidates, execute M003 qualification, gate required targets on typed evidence, call M004 local finalization only from complete evidence, upload the completed finalized release as an internal workflow artifact, and expose deterministic eggpack ci generate/check commands.

M002 stops at internal finalized workflow artifacts. It does not create or mutate GitHub Releases, publish registries, sign/notarize, or make public-release decisions.

## 2. Why this is ready

CI M001 supplies CIPlan, GitHub policy validation, deterministic rendering, and drift comparison.

Build M002/M002a supplies stable build bindings and command semantics.

Build M003 supplies QualificationBindingsV1, QualificationEvidence, qualification methods/status, and bounded execution.

Build M004 supplies FinalizationRequest, FinalizationTargetInput, ArchiveEncoding::TarGzip, FinalizedRelease, and finalize_release.

No release-staging API is required for M002.

## 3. Architecture boundary

~~~text
checked-in producer inputs
        |
        v
provider-neutral CI graph
        |
        v
GitHub renderer
        |
        +--> build candidate jobs
        +--> qualification jobs
        +--> required-evidence gate
        +--> aggregate/finalize job
                    |
                    v
         internal workflow artifact only
                    |
                    X  no GitHub Release/publication in M002
~~~

Core qualification/finalization semantics remain owned by eggpack-core. The CI crate may project and transport those semantics but must not reimplement them in YAML, shell, or PowerShell.

## 4. Invariants

- build success is never qualification success;
- qualification status comes only from M003 QualificationEvidence;
- final release bytes/manifests come only from M004 finalization;
- required targets with anything other than accepted passing qualification prevent aggregation;
- missing or corrupt build/qualification evidence prevents aggregation;
- non-gating or experimental target failure may keep the workflow non-red, but must suppress creation of a partial finalized release;
- aggregation never silently drops selected ReleasePlan targets;
- every handoff is identity-bound to release id, source revision, target, and logical selector;
- generated GitHub permissions remain read-only;
- no contents write, id-token write, release API, package publication, or secrets;
- action pins remain immutable full SHAs;
- Eggpack runtime tooling used inside generated CI is pinned;
- no generic shell/step/YAML DSL;
- no mutable remote reusable workflow authority;
- ci check is deterministic and never rewrites.

## 5. Scope

### In scope

- additive executable orchestration graph layered on M001 CIPlan;
- qualification jobs projected from M003 policy/bindings;
- required qualification gate;
- aggregate/finalize node using M004;
- bounded cross-job build/qualification handoff documents;
- complete/suppressed/failed aggregate outcome;
- deterministic GitHub build -> qualify -> aggregate rendering;
- minimal crates/eggpack-cli package with binary eggpack;
- eggpack ci generate and eggpack ci check;
- narrow internal CI runner commands that invoke M003/M004;
- pinned Eggpack tool provisioning policy;
- golden workflows and drift/evidence mutation tests;
- stable/Rust 1.89/macOS/Windows qualification.

### Out of scope

- draft/public GitHub Release creation;
- release asset upload to GitHub Releases;
- write permissions;
- registry publication;
- signing/notarization/provenance;
- generalized workflow scripting;
- version discovery/latest selection;
- changes to M003 or M004 semantics;
- consumer installation/update behavior.

## 6. Production model

### A. Preserve M001 CIPlan

Do not change the meaning of M001 CIPlan schema version 1, whose qualification state is intentionally unresolved.

Add a separate bounded executable type such as ReleaseCIPlanV1 containing:

- schema version;
- validated M001 CIPlan;
- qualification jobs;
- required qualification gate;
- aggregate job;
- deterministic handoff names;
- allowed finalization settings.

Historical M001 evidence must remain truthful.

### B. Qualification jobs

One qualification job per selected target.

Each carries canonical target, source build job id, exact candidate handoff identities/selectors, planned qualification classification, qualification host, support tier, deterministic qualification evidence handoff name, and required/non-gating state.

Projection validates QualificationBindingsV1 through the M003 API. Do not copy its validation logic.

Workflow execution delegates to eggpack_core::qualify_target through the Eggpack CLI.

### C. Build handoff schema

M001 uploads candidate bytes but BuildAttempt is a local-path type and must not become a cross-machine wire format accidentally.

Add a CI-specific bounded build handoff containing schema version, release id, source revision, target, build strategy, logical selector, package, binary, relative candidate path below the downloaded handoff root, byte size, and deterministic handoff identity.

No absolute runner path is serialized.

On a qualification runner, reconstruct an in-memory BuildAttempt from validated plan/bindings and downloaded regular files. Validate exact inventory, path containment, regular/non-symlink status, size, and identity before calling M003.

### D. Qualification evidence handoff

M003 QualificationEvidence may be serialized as deterministic JSON after validation.

The qualification handoff contains QualificationEvidence JSON plus exact candidate bytes required by later aggregation.

The aggregate job revalidates candidate bytes against evidence after download.

### E. Required qualification gate

Generate a distinct gate node.

Rules:

- every SupportTier::Required target must have QualificationStatus::Passed;
- required Deferred is not enough;
- required Failed fails;
- identity mismatch fails;
- missing required evidence fails.

The gate reads structured evidence, never log strings.

### F. Non-gating semantics

M004 requires complete candidate/evidence coverage for every selected target.

Therefore CI must never drop failed optional targets and still create a release.

If a non-required target is missing, failed, or unqualified, workflow policy may keep the run non-red, but finalization is suppressed and no finalized-release artifact is uploaded.

Represent a bounded aggregate outcome equivalent to Complete, SuppressedNonGatingIncomplete, FailedRequiredGate, or InvalidEvidence.

Later staging may proceed only from Complete.

### G. Aggregate/finalize job

The aggregate job downloads all target candidate/evidence handoffs, validates exact target set and identity, reconstructs FinalizationTargetInput values, calls eggpack_core::finalize_release, emits a bounded aggregate outcome, and uploads finalized root plus manifest only when Complete.

Archive encoding is TarGzip when archive targets require it. Do not infer or add another encoding.

### H. Minimal Eggpack CLI

Add crates/eggpack-cli with binary eggpack.

Keep dependencies narrow: Eggpack workspace crates plus a small argument parser if justified. No GitHub API client and no async runtime.

Required user-facing commands:

~~~text
eggpack ci generate
eggpack ci check
~~~

Required runner commands may use an explicitly internal namespace, for example:

~~~text
eggpack ci _capture-build
eggpack ci _qualify-target
eggpack ci _aggregate
~~~

Exact spelling is implementation-defined. These are deterministic file-in/file-out wrappers over core APIs, not a generic scripting interface.

### I. ci generate

Accept explicit input paths, at minimum executable CI plan + GitHub policy + output workflow.

Behavior:

- deterministic rendering;
- create/replace only the explicit workflow path;
- reject symlink output;
- write temp then atomically replace where supported;
- no network;
- bounded summary;
- no repository discovery.

### J. ci check

Inputs mirror generate.

Behavior:

- render in memory;
- compare via library drift semantics;
- exit 0 on exact match after documented newline normalization;
- nonzero on drift or invalid input;
- never modify;
- bounded diagnostic, not full workflow output.

### K. Pinned Eggpack runtime tool

Qualification/finalization jobs require Eggpack itself.

Do not assume an ambient unpinned binary.

Initial GitHub tool policy must support only the official Eggpack repository at an exact 40-hex revision, installed with cargo install --git using --rev, --locked, and the eggpack-cli package. Bound install timeout and verify the installed tool before use.

No arbitrary repository URL/package and no curl-pipe-shell.

A preinstalled mode may be added only as a finite enum with exact version/revision verification.

### L. Generated graph

~~~text
preflight
   |
   +--> build[A] --> qualify[A] --+
   +--> build[B] --> qualify[B] --+--> required_gate
   +--> build[C] --> qualify[C] --+        |
              \____________________________|
                                           v
                                      aggregate
                                           |
                                 [Complete only]
                                           v
                               internal final artifact
~~~

Use always-style provider semantics only where needed to collect typed evidence. Never use them to bypass required failures.

### M. Permissions/artifacts

M002 stays read-only: contents read only, no id-token write, no contents write.

Candidate/evidence/final artifacts use deterministic names and bounded retention.

Do not upload credentials, absolute paths, or raw process output.

## 7. Work packages

1. Add executable graph types layered on M001 CIPlan.
2. Add build handoff schema and reconstruction validation.
3. Add qualification projection using QualificationBindingsV1.
4. Add deterministic qualification evidence handoff.
5. Add required gate model.
6. Add aggregate outcome and optional-target suppression.
7. Add M004 aggregate/finalize adapter.
8. Extend GitHub policy with pinned Eggpack tool provisioning.
9. Render qualify/gate/aggregate jobs.
10. Add eggpack-cli.
11. Implement ci generate/check.
12. Implement narrow runner commands.
13. Add golden direct, mixed native/cross, bundle, and archive workflows.
14. Add drift/tamper/missing-evidence/gating tests.
15. Update docs/roadmap/registry and write closure.

## 8. Failure/restart/contention

Build/qualification jobs are immutable attempts for one release identity.

Aggregate uses a new private output root and returns no finalized artifact on invalid or incomplete required evidence.

Optional incompleteness suppresses output instead of creating a partial release.

No public mutation occurs.

ci generate atomically replaces only its explicit output path. ci check is read-only.

## 9. Security review

Explicitly audit action SHA validation, official Eggpack repo plus immutable revision, command/YAML injection, shell quoting, path traversal/symlinks, artifact-name collisions, evidence swapping, candidate tampering, required gate bypass through provider conditions, permissions, and evidence leakage.

M004 must revalidate final bytes; artifact transport is not trusted as proof.

## 10. Required tests

### Graph/projection

- M001 CIPlan remains unchanged;
- executable graph validates exact CIPlan + qualification bindings;
- deterministic ordering;
- qualification host retained;
- required gate exact;
- missing/extra qualification target rejects;
- smoke selector mismatch rejects.

### Handoff

- absolute path rejects;
- traversal rejects;
- symlink rejects;
- missing/extra candidate rejects;
- wrong size rejects;
- release/source/target/selector/package/binary swap rejects;
- duplicate handoff rejects.

### Qualification/gate

- required Passed permits;
- required Deferred rejects;
- required Failed rejects;
- missing required evidence rejects;
- optional failure yields suppression, not partial release;
- structural/deferred behavior matches M003/M004.

### Aggregation

- complete direct release finalizes;
- complete bundle finalizes exact members;
- complete archive uses TarGzip;
- missing optional target suppresses output;
- tampered candidate rejects;
- mixed source/release rejects;
- completed release cannot omit selected target.

### Renderer

- qualification jobs run on qualification hosts;
- immutable Eggpack tool pin;
- exact gate dependencies;
- read-only permissions;
- no release API/write permission;
- no arbitrary command;
- deterministic handoff names;
- independent YAML parse.

### CLI

- generate equals library renderer;
- symlink output rejects;
- check passes exact file;
- CRLF normalization only as documented;
- one-byte drift fails;
- check never modifies;
- invalid/oversized input rejects;
- bounded diagnostics.

## 11. Verification

~~~bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-ci --locked
cargo tree -p eggpack-cli --locked
cargo package -p eggpack-ci --locked --allow-dirty
cargo package -p eggpack-cli --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
git diff --check
~~~

Use the repository-established local path patch procedure when package commands need unpublished workspace dependencies.

Hosted CI must pass Linux stable, Linux Rust 1.89, macOS, and Windows.

## 12. Documentation

Update root README, crates/eggpack-ci/README.md, new crates/eggpack-cli/README.md, CI roadmap, registry, and plans/closure/ci-release-orchestration/002-status.md.

Document that internal artifacts are not public releases, optional failures suppress release output, staging remains separate, tool provisioning is pinned, and ci check is non-mutating.

## 13. Acceptance criteria

M002 closes only when:

- executable provider-neutral orchestration graph exists;
- M003 qualification is invoked, not copied;
- M004 finalization is invoked, not copied;
- required gates fail closed;
- incomplete optional targets cannot produce partial releases;
- complete direct/bundle/archive paths produce internal finalized artifacts;
- generated workflow is deterministic and read-only;
- Eggpack runtime tooling is pinned;
- ci generate/check exist and are deterministic;
- drift check is non-mutating;
- stable/MSRV/macOS/Windows CI passes;
- no unresolved medium-or-higher orchestration/supply-chain finding remains.

Closure must state whether CI M003 is ready to plan. M002 closure alone does not authorize staging implementation without the separately required staging adapter plan.

## 14. Stop conditions

Stop and re-plan if qualification/finalization logic must be copied into generated shell, workflow needs GitHub write permission, unpinned Eggpack tooling is required, generic step/YAML language is needed, partial releases must be produced, M003/M004 need wire-format redesign, staging becomes necessary, or CLI needs repository discovery heuristics.

## 15. Closure evidence

Record implementation SHA, M003/M004 API baselines, executable graph types, handoff schemas, gate matrix, direct/bundle/archive fixtures, golden workflow hashes, pinned tool evidence, permissions audit, ci generate/check behavior, drift mutation evidence, package/dependency/MSRV/docs results, hosted matrix, unresolved findings, and explicit CI M003 readiness disposition.
