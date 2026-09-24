# Build and Qualification Milestone 003 — Qualification Execution and Evidence

Status: ready for handoff

Repository baseline: 25a6f185f865510be0ab26a51a819d949701674c

Source roadmap:

- plans/subsystems/build-qualification-roadmap.md

Closed prerequisites:

- Build/Qualification M001 — PackConfig and ReleasePlan;
- Build/Qualification M002 — native/cross Cargo builder seam;
- Build/Qualification M002a — Windows builder qualification stability;
- CI Orchestration M001 — provider-neutral CIPlan and deterministic GitHub renderer.

Applicable ADRs:

- plans/adrs/ADR-0001-producer-consumer-release-boundary.md
- plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md

Long-term references:

- plans/000-long-term-specification.md#10-qualification-model
- plans/000-long-term-specification.md#11-artifact-finalization-and-checksums
- plans/001-terminology-and-domain-model.md#8-build-and-qualification
- plans/002-long-term-roadmap.md#phase-5--local-packaging-and-release-aggregation

Primary class: capability / qualification execution / producer evidence

## 1. Objective

Implement Eggpack's first qualification execution model over M002 CandidateArtifact evidence.

M003 must turn one exact ReleasePlan target plus one exact successful BuildAttempt into bounded, machine-readable qualification evidence for the already-modeled classifications:

- Native;
- DeferredNative;
- Emulated;
- Structural.

The milestone must preserve the core invariant:

build success != qualification success.

Qualification evidence must bind to the exact candidate bytes that were checked. M003 therefore also strengthens M002 BuildAttempt identity with the release id and source revision already known by execute_target.

M003 stops before final artifact naming, archive assembly, signing/notarization, final checksums, ReleaseManifest construction, aggregation, CI qualification-job rendering, staging, or publication.

## 2. Readiness and architecture disposition

No new ADR is required.

The existing model already fixes the durable policy distinctions:

- ReleasePlan carries target, build host, optional qualification host, Qualification classification, and support tier.
- M002 returns exact candidate paths and logical source identities.
- ADR-0004 authorizes a first-party bounded process execution seam and rejects a generic shell/command DSL.
- The long-term specification explicitly requires native, deferred-native, emulated/structural qualification and bounded product-owned smoke hooks.
- CI M001 intentionally leaves qualification state unresolved for this milestone to define.

M003 must implement those semantics without widening PackConfig into a workflow language.

## 3. Current interface gap

Current BuildAttempt evidence contains target, strategy, tool summary, process evidence, and candidate artifacts, but not the ReleasePlan release id/source revision.

That is insufficient for durable qualification evidence because a BuildAttempt from another release with the same target shape could otherwise be paired with the current plan.

Before qualification execution is accepted, M003 must add:

- release_id;
- source_revision;

to BuildAttempt and populate them directly inside execute_target / execute_target_cancellable from the supplied ReleasePlan.

Qualification must reject any plan/attempt identity mismatch before reading or executing candidate bytes.

No DistributionContract, ReleaseManifest, or consumer wire format changes are required.

## 4. Qualification authority and non-goals

M003 owns:

- candidate-byte identity checks;
- structural binary target checks;
- bounded native candidate smoke execution;
- deferred-native disposition and later matching-host execution;
- bounded QEMU user-mode execution for supported Linux targets;
- typed qualification evidence.

M003 does not own:

- application test discovery;
- arbitrary scripts/workflow graphs;
- shell fragments;
- repository command execution;
- source checkout/build;
- final artifact transforms;
- final release hash identity;
- CI provider job placement;
- publication;
- consumer installation.

Eggpack does not interpret application-specific output. A smoke hook means only: execute one explicitly selected candidate with bounded fixed arguments and require successful process completion.

## 5. Required domain model

Add qualification types in eggpack-core, preferably in a dedicated qualifier.rs module exported through lib.rs.

### A. QualificationBindingsV1

Define a strict producer-side, schema-versioned qualification binding document.

Conceptually:

~~~text
QualificationBindingsV1
  schema_version = 1
  targets: canonical target -> TargetQualificationBinding

TargetQualificationBinding
  smoke: optional CandidateSmokeBinding
~~~

The document is producer configuration, not release/consumer identity.

Rules:

- canonical target keys only;
- unknown fields rejected;
- bounded target count;
- no duplicate canonical targets;
- no executable path supplied by the user;
- no shell;
- no environment map;
- no repository script path;
- no arbitrary expected-output regex/string;
- no final artifact filename.

### B. CandidateSmokeBinding

One bounded smoke invocation may be configured per target.

Fields:

- exact LogicalOutputSelector;
- fixed argv list;
- timeout;
- stdout byte limit;
- stderr byte limit.

Constraints:

- argv count and per-argument length bounded;
- control/NUL characters rejected;
- timeout > 0 and capped;
- output limits > 0 and capped;
- exit code 0 is the only passing process exit in M003;
- executable is always the selected CandidateArtifact path, never a caller-supplied program.

This is the M003 "product-owned bounded smoke hook." It deliberately does not become a generic hook/script DSL.

### C. QualificationStatus

Define a typed result state equivalent to:

- Passed;
- Deferred;
- Failed.

Failed must carry only a bounded typed reason, not arbitrary child output.

Suggested failure classes:

- StructuralMismatch;
- CandidateChanged;
- HostMismatch;
- SmokeFailed;
- SmokeTimedOut;
- OutputLimitExceeded;
- EmulatorUnavailable;
- EmulatorFailed;
- InvalidRuntimeEnvironment.

Invalid plan/binding/evidence shape remains a QualificationError rather than a Failed qualification result.

### D. QualificationEvidence

One evidence record must bind at minimum:

- schema version;
- release id;
- source revision;
- canonical target;
- planned Qualification classification;
- actual qualification method;
- actual host capability;
- status;
- support tier;
- ordered per-candidate byte evidence;
- selected smoke selector when applicable;
- process evidence summary when execution occurred.

Do not include:

- timestamps as canonical identity;
- absolute candidate paths;
- secrets;
- stdout/stderr contents;
- final release filenames;
- publication state.

Evidence ordering must be canonical and serialization deterministic.

### E. QualifiedCandidateEvidence

For every candidate in the BuildAttempt, record:

- logical selector;
- package;
- binary;
- exact byte size;
- SHA-256 of the candidate bytes observed for qualification;
- structural format;
- structural architecture.

This is pre-finalization qualification evidence, not the final ReleaseManifest digest authority.

## 6. Exact input reconciliation

Before any qualification operation:

1. validate ReleasePlan schema/release/source identity;
2. find the exact PlannedTarget by canonical target;
3. require BuildAttempt.release_id == ReleasePlan.release_id;
4. require BuildAttempt.source_revision == ReleasePlan.source_revision;
5. require BuildAttempt.target == PlannedTarget.target;
6. require BuildAttempt.strategy == PlannedTarget.policy.strategy;
7. require BuildAttempt process outcome == Success;
8. require the BuildAttempt candidate selector/package/bin inventory to equal the target's validated BuildBindingsV1 inventory exactly;
9. reject duplicate/missing/extra candidates;
10. reject candidate paths that are relative, symlinks, non-regular, empty, or unavailable;
11. require current size to equal CandidateArtifact.size.

No qualification is produced from an unsuccessful or partial build attempt.

## 7. Candidate byte snapshot and mutation guard

Qualification must bind to bytes, not only paths.

For every candidate:

1. read/hash SHA-256 with bounded streaming;
2. record exact size;
3. perform structural checks;
4. execute smoke/emulation when required;
5. re-stat and re-hash every candidate before returning Passed;
6. require the post-qualification size/digest to equal the pre-qualification snapshot.

If a candidate changes during qualification, return Failed(CandidateChanged) and no Passed evidence.

Deferred evidence may contain the structural/preflight snapshot, but it must be clearly marked Deferred and must not be accepted by later finalization as Passed qualification.

M004 will decide how pre-finalization evidence is reconciled after byte-changing finalization; M003 must not claim final-byte identity.

## 8. Structural binary qualification

Implement a small bounded parser for the native executable formats needed by current target families. Do not add a large executable-loader framework unless implementation evidence proves necessary.

### ELF

Validate:

- ELF magic;
- 32/64-bit class appropriate to target architecture;
- supported endianness;
- e_machine matches x86_64, AArch64, or ARM/armv7 as appropriate.

### PE/COFF

Validate:

- DOS MZ header;
- bounded PE header offset;
- PE signature;
- machine type matches x86_64, ARM64, or supported ARM target.

### Mach-O

Validate:

- thin Mach-O magic;
- CPU type matches x86_64 or arm64 target.

Universal/fat Mach-O is out of scope for M003 and must fail structural qualification rather than silently choosing a slice.

### General structural rules

- read only bounded header ranges needed for parsing;
- reject truncated/ambiguous headers;
- derive expected format from canonical target OS;
- derive expected architecture from canonical target architecture;
- structural qualification does not prove glibc or macOS deployment-floor compatibility;
- structural qualification does not execute bytes.

All qualification classifications should perform structural checks before any execution. Structural classification stops after these checks and may return Passed.

## 9. Host detection

Add one provider-neutral actual-host helper for supported hosts:

- Linux / macOS / Windows;
- x86_64 / aarch64 / armv7 where representable.

Public qualification execution must use actual local host facts rather than a caller assertion.

For deterministic unit tests, internal/test-only host injection is acceptable.

Unsupported host OS/architecture returns a typed execution error and never Passed evidence.

## 10. Native qualification semantics

For Qualification::Native:

- actual host must match the target's required native qualification host;
- PackConfig validation already prevents CargoZigbuild from claiming Native;
- CandidateSmokeBinding is required;
- selected smoke selector must exist in the exact candidate set;
- execute the selected candidate path directly, shell-free;
- stdin is null;
- inherited environment is cleared except the same bounded runtime/path/temp allowlist needed by M002 execution;
- no arbitrary environment additions from config;
- enforce smoke timeout/output limits;
- process group/job cleanup must complete before result;
- exit 0 => execution portion passes;
- timeout/output overflow/nonzero => typed Failed qualification.

A Native target with no smoke binding must fail validation rather than treating structural checks alone as native proof.

## 11. Deferred-native semantics

For Qualification::DeferredNative:

- structural checks always run;
- qualification_host MUST be present in the target policy;
- if actual host does not match qualification_host:
  - return Deferred evidence;
  - do not execute the smoke hook;
  - do not label the target Passed;
- if actual host matches qualification_host:
  - require CandidateSmokeBinding;
  - execute exactly as native qualification;
  - successful execution returns Passed with method DeferredNativeOnNativeHost.

This allows a build host to emit bounded deferred evidence while a later CI/native-host job can execute the same candidate bytes and produce Passed evidence.

M003 itself does not transport candidates between hosts; CI M002 will later own graph/handoff integration.

## 12. Emulated qualification semantics

Initial Emulated execution is deliberately finite:

- support QEMU user-mode only;
- support Linux ELF target candidates only;
- select emulator by target architecture from a finite mapping:
  - x86_64 -> qemu-x86_64;
  - aarch64 -> qemu-aarch64;
  - armv7 -> qemu-arm;
- candidate smoke binding is required;
- candidate selector must be exact;
- QEMU preflight is bounded and shell-free;
- optional emulator runtime policy may provide one absolute sysroot path for QEMU -L;
- the QEMU executable itself comes from the finite target mapping/PATH, not arbitrary config;
- current host must satisfy qualification_host when one is declared;
- stdin null, output/time bounded, process cleanup required;
- exit 0 => Passed;
- missing QEMU/sysroot or process failure => typed Failed.

Do not support:

- arbitrary emulator executable names;
- arbitrary QEMU flags;
- containers;
- Wine;
- macOS/Windows emulation;
- system emulation;
- network setup.

Those require later evidence and possibly a new plan.

If QEMU is not installed on the standard Eggpack CI image, M003 may close with exact command/preflight/unit evidence plus a clearly recorded operational limitation, provided Native, DeferredNative, and Structural execution are qualified live. Do not auto-install QEMU merely to manufacture a passing closure without a separate CI provisioning decision.

## 13. Process execution reuse

Do not expose M002's process runner as an arbitrary public "run this executable" API.

Refactor the bounded process implementation internally only as needed so both builder and qualifier can use the same:

- environment clearing/allowlist;
- stdin nulling;
- stdout/stderr byte bounds;
- deadlines;
- process-group/Windows job termination;
- wait/cleanup guarantees.

Builder public execution must remain restricted to Cargo/rustc/Zig authority.

Qualifier execution must only accept:

- the already validated candidate path; or
- the finite QEMU executable mapping.

No caller-provided generic executable reaches the shared runner.

## 14. QualificationBindings validation

QualificationBindingsV1 must validate against ReleasePlan + BuildBindingsV1.

Rules by classification:

- Native => exactly one smoke binding required;
- DeferredNative => exactly one smoke binding required and qualification_host required;
- Emulated => exactly one smoke binding required;
- Structural => smoke binding prohibited in v1.

For every smoke selector:

- selector must exist in the exact target build binding set;
- package/bin identity is inherited from BuildAttempt/BuildBindings, not duplicated in qualification config.

Reject qualification binding entries for targets not present in the selected ReleasePlan.

Do not infer a "primary" bundle member.

## 15. Support-tier semantics

M003 records SupportTier but does not decide release publication/gating policy.

In particular:

- Required + Failed/Deferred is still evidence, not a release decision;
- NonGating/Experimental does not convert a failed qualification into Passed;
- CI M002/M004 later decide whether aggregation can proceed.

No method may silently downgrade Qualification classification based on SupportTier.

## 16. Failure, retry, restart, and contention semantics

Qualification is read-only with respect to candidate bytes.

It may create only owner-private temporary execution state needed by the process runner/emulator.

No automatic retries.

A caller may explicitly rerun qualification; each run independently re-hashes candidate bytes.

Timeout/cancellation must kill/wait for the owned process group before returning.

Concurrent qualification of the same immutable candidate bytes is allowed if execution does not mutate them; the before/after digest guard must detect mutation.

No qualification operation writes public release state.

## 17. Ordered work packages

1. Add release_id/source_revision to BuildAttempt and update M002 tests/docs.
2. Add qualifier module, QualificationBindingsV1, smoke binding, typed status/failure/evidence types.
3. Add exact plan/build-attempt/binding reconciliation.
4. Add candidate streaming SHA-256 snapshot and before/after mutation guard.
5. Implement bounded ELF/PE/Mach-O structural target parser.
6. Add actual-host detection.
7. Refactor bounded process runner internally without widening public executable authority.
8. Implement Native candidate smoke execution.
9. Implement DeferredNative deferred and matching-host execution paths.
10. Implement finite Linux QEMU user-mode execution/preflight/runtime policy.
11. Add deterministic qualification evidence serialization/validation.
12. Add focused direct/bundle/archive-candidate tests.
13. Add live native qualification fixture on supported hosted lanes.
14. Add deferred-host and structural fixtures.
15. Add QEMU command/preflight/live evidence as available under the environment policy.
16. Update core docs, build roadmap, CI roadmap dependency notes, registry, and closure record.

## 18. Required positive test matrix

At minimum:

### Identity/evidence

- BuildAttempt carries exact release id/source revision from execute_target;
- exact ReleasePlan/BuildAttempt target identity passes;
- candidate inventory matches BuildBindings exactly;
- QualificationEvidence deterministic serialization roundtrip;
- candidate evidence is canonically ordered.

### Structural

- x86_64 ELF accepted for x86_64 Linux target;
- AArch64 ELF accepted for AArch64 Linux target;
- armv7 ELF accepted for armv7 Linux target;
- x86_64 PE accepted for x86_64 Windows target;
- ARM64 PE accepted for AArch64 Windows target;
- x86_64 thin Mach-O accepted for x86_64 macOS target;
- arm64 thin Mach-O accepted for AArch64 macOS target.

### Native

- real host-native Cargo fixture is built by M002 and qualified by candidate smoke;
- successful candidate exit produces Passed;
- evidence records exact pre/post SHA-256 and size;
- Windows native fixture uses initialized MSVC environment and remains stable with M002a tests.

### DeferredNative

- non-matching host returns Deferred without executing smoke;
- matching qualification host runs smoke and can return Passed;
- Deferred evidence is never represented as Passed.

### Emulated

- finite target->QEMU executable mapping;
- deterministic QEMU command with optional -L sysroot;
- bounded preflight;
- successful fake/injected runner path returns Passed;
- live QEMU evidence recorded if tool is available under approved CI policy.

## 19. Required negative test matrix

At minimum:

- BuildAttempt release id mismatch;
- source revision mismatch;
- target mismatch;
- strategy mismatch;
- unsuccessful BuildAttempt;
- missing candidate;
- extra candidate;
- duplicate selector;
- package/bin mismatch;
- relative candidate path;
- candidate symlink;
- directory/empty/missing candidate;
- candidate current size differs from BuildAttempt size;
- candidate mutates between pre/post qualification hash;
- malformed/truncated ELF;
- wrong ELF architecture;
- malformed PE/e_lfanew overflow;
- wrong PE machine;
- malformed Mach-O;
- fat/universal Mach-O rejected;
- wrong Mach-O CPU;
- Native on wrong host;
- Native without smoke binding;
- DeferredNative without qualification_host;
- matching DeferredNative host with missing smoke binding;
- Structural with smoke binding;
- smoke selector absent from exact candidate set;
- smoke non-zero exit;
- smoke timeout;
- smoke stdout/stderr limit exceeded;
- process cleanup incomplete cannot produce Passed;
- Emulated non-Linux target rejected;
- unsupported emulated architecture rejected;
- missing QEMU;
- invalid/relative sysroot;
- qualification binding unknown field/version rejected;
- overlong args/too many args/invalid control characters rejected.

## 20. Verification commands

Run and record:

~~~bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
git diff --check
~~~

Hosted qualification:

- Linux stable;
- Linux Rust 1.89;
- macOS;
- Windows;
- M002a named Windows builder tests must remain green;
- at least one real Native qualification smoke on each host family where the fixture target matches;
- DeferredNative mismatch behavior must be covered deterministically;
- Structural parser tests must be platform-independent.

QEMU operational coverage must state exactly whether a real emulator ran or only finite command/preflight behavior was qualified.

## 21. Compatibility and migration

This milestone may add fields to internal BuildAttempt and new producer-side qualification types.

It must not change:

- DistributionContract wire semantics;
- ReleaseManifest v1;
- Eggup interfaces;
- CIPlan v1 semantics except for later consumption of new qualification evidence;
- existing artifact filenames/install identities.

CI M001 remains closed. M003 may add types that CI M002 will consume later, but it must not modify the CI renderer to fake qualification jobs during this milestone.

## 22. Documentation updates

Update:

- crates/eggpack-core/README.md;
- root README qualification capability summary;
- plans/subsystems/build-qualification-roadmap.md;
- plans/subsystems/ci-release-orchestration-roadmap.md dependency note if the M003 evidence type closes cleanly;
- plans/registry.md;
- plans/closure/build-qualification/003-status.md.

Document clearly:

- candidate build evidence vs qualification evidence;
- structural vs execution proof;
- Deferred is not Passed;
- candidate qualification digest vs final ReleaseManifest digest;
- QEMU scope/limitations;
- no generic hooks/scripts.

## 23. Acceptance criteria

M003 closes only when:

- BuildAttempt is bound to release/source identity;
- exact candidate inventory reconciliation is enforced;
- qualification hashes exact candidate bytes before/after execution;
- structural architecture checks cover current ELF/PE/Mach-O host families;
- Native execution is live-qualified and bounded;
- DeferredNative semantics distinguish Deferred from Passed and execute on a matching qualification host;
- Emulated has a finite QEMU user-mode implementation with truthful operational evidence;
- candidate smoke hooks are bounded, candidate-only, and shell-free;
- no generic command/script DSL is introduced;
- qualification evidence is typed, bounded, deterministic, and contains no secret/output contents;
- M002/M002a builder regressions remain green;
- stable/MSRV/macOS/Windows hosted CI passes;
- no unresolved medium-or-higher qualification correctness/security finding remains.

Closure must state whether:

- Build M004 finalization/aggregation is ready to plan;
- CI M002 remains blocked or is partially interface-ready.

## 24. Stop conditions

Stop and prepare a new plan/ADR if implementation requires:

- arbitrary repository scripts or shell commands;
- a generic hook/workflow DSL;
- container qualification as a requirement for closure;
- arbitrary emulator command/flags;
- changing Qualification enum semantics;
- weakening native host matching;
- treating Deferred as Passed;
- finalization/archive/signing behavior to define qualification success;
- consumer/publication policy;
- a new external build backend.

## 25. Closure evidence required

Record:

- implementation SHA(s);
- BuildAttempt identity delta;
- QualificationBindings schema/examples;
- QualificationEvidence schema/examples;
- structural parser support matrix;
- before/after digest mutation-guard evidence;
- native hosted smoke evidence;
- deferred-host evidence;
- QEMU implementation and operational limitation/evidence;
- process timeout/output/cancellation matrix;
- Windows M002a regression evidence;
- package/dependency/MSRV/docs results;
- hosted Linux stable/Linux 1.89/macOS/Windows runs;
- unresolved findings;
- explicit Build M004 and CI M002 readiness dispositions.
