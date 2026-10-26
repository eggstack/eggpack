# `eggpack-core` — Qualification stage (`src/qualification.rs`) — Deep Dive

Binds one fixed-argument smoke check per target, structurally inspects each
candidate's exact bytes, runs a bounded execution on a host-matched or emulated
runtime, and records `QualificationEvidence` about those bytes. 2981 lines
(`src/qualification.rs`; `:1316-2981` is the inline test module). This is one of
four `eggpack-core` stage deep dives; `core.md` holds the crate-level
orientation.

## Inputs and outputs

**Input** — `QualificationRequest<'a>` (`:496-515`) is all-or-nothing: a
`DistributionContract`, the invocation `ReleasePlan`, exactly one
`PlannedTarget`, a successful `BuildAttempt`, `BuildBindingsV1`,
`QualificationBindingsV1`, a `QualificationRuntime`, and a `BuildCancellation`.
There is no partial or defaulted input and no output-directory parameter.

**Output** — `Result<QualificationEvidence, QualificationError>` (`:516-543`).
The error variant means *the request was malformed*; it never means *the
candidate was bad*. A structurally wrong candidate, a non-matching host, a
failed smoke, a contradicted floor, and a mid-run mutation all return `Ok` with
a typed `QualificationStatus::Failed` record (`:677-687`, `:708-725`,
`:736-746`, `:932-946`, `:955-978`). Failures are evidence; only refused work is
an error.

**Rejected before any inspection** (`:593-660`), in order: plan/attempt identity
— schema version, non-empty `release_id`/`source_revision`, target membership,
matching `release_id`/`source_revision`/`target`/`strategy`, a `Success` build
process, and 1..=256 candidates (`:593-604`); then `BuildBindingsV1::validate_for`
(`:609-611`) and `QualificationBindingsV1::validate_for` (`:612`), both re-run
internally rather than trusted from the caller; then the runtime (`:613`, `:1047-1060`);
then the QEMU-sysroot-requires-`Emulated` rule (`:614-618`); then an exact
selector/package/binary reconciliation of the attempt's candidates against the
build bindings (`:620-660`).

**Never in the evidence**: candidate paths, output contents, environment values.
Only the canonicalized working directory and the absolute candidate path reach
the child process (`:840-850`, `:901-913`, `:916-926`); `:1797-1798` asserts the
serialized evidence contains no candidate path.

**Module bounds** (`:22-26`): 256 targets, 128 args, 4096 bytes per arg, 256 KiB
retained capture, 4096-byte binary-header window. `schema_version` is exactly 1
on both `QualificationBindingsV1` (`:138`) and `QualificationEvidence` (`:295`,
`:337`).

## The method taxonomy

Requested classification is `Qualification` (intent, `src/lib.rs:487-506`);
the method actually applied is `QualificationMethod` (`:182-197`). The pairing
is closed — one classification admits exactly two or one methods, and any other
combination is rejected by `validate_for` (`:402-413`).

| Requested | Method (`:182-197`) | Executes | Proves | Does **not** prove |
|---|---|---|---|---|
| `Native` | `Native` | yes, direct | the exact pre-hashed bytes ran on a host whose OS/arch match the target triple, exited 0 under the fixed argv, and were byte-identical afterwards (`:730-749`, `:916-926`, `:955-978`) | that any other host runs it; that a declared floor is honoured |
| `DeferredNative` | `Deferred` | no | only that the bytes are structurally valid for the target, and that the required host has not yet run them (`:750-767`) | any execution — `Deferred` can never be `Passed` (`:419-420`) |
| `DeferredNative` | `DeferredNativeOnNativeHost` | yes, direct | the same as `Native`, on the separately declared `qualification_host` (`:768-781`) | anything about the deferred hosts that were skipped |
| `Emulated` | `QemuUser` | yes, under QEMU | the target is `-linux-`, every candidate is ELF, the finite `qemu-*` mapping answers `--version`, and the bytes ran under it to a zero exit (`:783-819`, `:837-886`) | native execution on the target; performance; fidelity outside the emulated user-mode surface |
| `Structural` | `Structural` | never | file shape and header identity only: non-symlink regular file under a canonical root, size equal to the `BuildAttempt`, ELF/PE/thin-Mach-O header agreeing with the target's format and architecture, plus a SHA-256 (`:729`, `:821-833`, `:1162-1246`) | that the candidate runs anywhere |

**Structural inspection is not executed evidence.** `Structural` records zero
processes and no `smoke_selector`, and that emptiness is a validated invariant
rather than an accident (`:421-422`). A `Structural` pass is a statement
about bytes on disk; the three executing methods additionally carry
`QualificationProcessEvidence` — and a `Passed` record must carry exactly the
right number of them (`:478-485`): 1 for native, 2 for QEMU (preflight plus
smoke), 0 for structural, never for deferred.

The QEMU mapping is a closed table, not a template: `-linux-` targets only,
`x86_64` → `qemu-x86_64`, `aarch64` → `qemu-aarch64`, `armv7` → `qemu-arm`
(`:1101-1112`), and the argv is built solely from the optional declared sysroot,
the candidate path, and the binding's fixed argv (`:1113-1126`, asserted at
`:2154-2162` and `:2278-2286`). The optional sysroot must be an absolute, non-symlink directory
(`:1047-1060`).

## Host matching: native qualification is builder-independent

`Qualification::Native` is host-matched proof about the exact candidate bytes,
and it is **independent of which builder produced them**. The `Native` branch
(`:730-749`) matches on the host and the target triple only; the single
`strategy` reference in the module's production code is the identity check
`attempt.strategy != target.policy.strategy` (`:600`). There is no strategy
term in the method selection, so a `CargoZigbuild`-built candidate reaching this
host is a legitimate `QualificationMethod::Native` record.
`Qualification::Native`'s own rustdoc states the same
(`src/lib.rs:489-496`), and `cross_tool_built_candidate_qualifies_natively_on_the_matching_host`
(`:1846-1965`) proves it end to end, including through the public
`qualify_target` entry point.

The rule, precisely:

1. The effective qualification host is `policy.qualification_host`, else the
   build host `{policy.host_os, policy.host_arch}` (`:731-734`).
2. Both conditions must hold: `host == required` **and**
   `target_matches_host(&target.target, host)` (`:735`).
   `target_matches_host` (`:1087-1100`) is a pure triple↔host mapping — OS by
   triple substring/suffix (`-linux-`, `-apple-darwin`, `-windows-`), arch by
   triple prefix (`x86_64-`, `aarch64-`, `armv7-`).
3. On failure the function returns `Ok` with
   `Failed(QualificationFailure::HostMismatch)`, method still `Native`, and
   `processes: Vec::new()` (`:736-746`). **Never a pass, never a skip, never an
   executed smoke.** The failed record is itself valid evidence
   (`:2047-2052`).

The same rule is re-enforced in reverse by `QualificationEvidence::validate_for`,
so a hand-edited or forged pass record cannot survive: a pass on a native method
requires `target_matches_host` (`:429-434`) and requires any declared
`qualification_host` to equal the observed `actual_host` for all three executing
classifications (`:435-452`). `Deferred` is asymmetric by design and is
constrained the other way (`:414-418`): method `Deferred`, no processes, a
declared `qualification_host` present, and it must *differ* from the actual
host. The test `native_host_mismatch_is_a_failure_and_never_runs_smoke`
(`:2013-2053`) and the off-host/`Deferred` cases at `:1649-1708` and
`:2056-2164` cover both directions.

Observed host facts come from `actual_host()` (`:1070-1086`), which maps only
`std::env::consts::OS`/`ARCH` for linux/macos/windows and x86_64/aarch64/
arm-v7, and returns an error for anything else. `qualify_target_for_host`
(`:544-577`) and `qualify_target_for_host_with_runner` (`:578-990`) exist so a
host and a process runner can be injected deterministically in tests; the public
`qualify_target` is a thin wrapper that supplies the real host.

## What native qualification does not prove

A successful native run proves the exact candidate bytes execute on *this*
host's operating system and architecture. It is **not** proof that the produced
binary honours a declared `CompatibilityFloor` — a glibc or macOS deployment
floor. The floor is never established by a smoke run:

- The smoke's only observations are the exit status and the bounded output
  capture (`:900-928`). Nothing in the execution path reads the floor.
- The floor's own rustdoc says so: "Native qualification evidence proves that
  the exact candidate executes on the qualifying native host; it is not
  independent proof that the produced binary honours this declared minimum
  runtime" (`src/lib.rs:581-587`).

What *does* establish a floor is the separate, byte-level floor inspection
described below. It never infers a floor from execution.

The other residual: `target_matches_host` compares OS family and CPU
architecture only. It does not distinguish a Linux distribution or glibc
version, so two hosts on the same triple are treated as interchangeable
capabilities. This follows from `HostRequirement` carrying exactly
`{os, arch}` (`src/lib.rs:518-532`).

## Compatibility floor verification status

The floor status is the producer's record of *what was actually done* with a
declared floor. It is a finite, closed enum — `FloorVerificationStatus`
(`src/floor.rs:72-89`) — and it is **not** a free-form string and **not** an
`Option<bool>`. It is recorded in two places:

- on the plan, as `PlannedTarget::floor_status` (`src/lib.rs:618-637`), so the
  declared floor and its status are visible in the producer's intent artifact;
- on the evidence, as `QualificationEvidence::floor` (`:293-328`), so the
  status is bound to the exact bytes of that one candidate.

| Status | Concrete observation |
|---|---|
| `NotDeclared` | the target declares `CompatibilityFloor::None` |
| `ToolchainBound` | the floor was bound into the build toolchain by the cargo-zigbuild triple rewrite, and the toolchain is trusted to honour it |
| `Verified` | the produced bytes were independently inspected and their highest required `GLIBC_x.y` version is not newer than the declared floor |
| `Unverified(reason)` | a floor was declared and the bytes did not confirm it, for the bounded reason given |
| `NotSatisfied` | the produced bytes were independently inspected and require a newer runtime than the declared floor |

`Unverified` carries a closed reason enum, `FloorUnverifiedReason`
(`src/floor.rs:30-71`): `PendingInspection`, `UnreadableCandidate`,
`MalformedCandidate`, `NoVersionRequirement`, `UnrecognizedVersionRequirement`,
`FloorKindNotVerifiable`.

**The comparison direction is the point.** A candidate satisfies a declared
floor only when `required <= declared`: a binary that requires a *newer* glibc
than the floor declares does not satisfy it. `GLIBC_2.34` against a declared
`2.17` floor is `NotSatisfied`, not `Verified`.

**The status is computed from bytes, never from execution.** The inspection
runs inside `inspect_candidate` (`:1162-1246`), on the same canonical path
whose bytes the crate's single file-digest authority then digests
(`:1211-1232`), so the recorded status describes the bytes the record
describes. The reader itself lives in `src/floor.rs`: the ELF header
(`:285-333`), the bounded program-header table (`:334-390`), the bounded
dynamic table (`:391-434`), the virtual-address mapping (`:435-451`), the
bounded version-name read (`:452-480`), and the version parse (`:481-508`).
Every table and every offset is bounded; an over-large program-header or
dynamic-entry count is refused rather than parsed.

**Failure semantics.** A floor that is declared and *contradicted* by the bytes
is false, so it is a hard failure of the qualification for that target:
`Failed(QualificationFailure::FloorNotSatisfied)` with the floor status
`NotSatisfied` (`:705-725`). It is never reported as `Unverified`, and it is
detected before anything executes. A floor that is declared and merely
*unverifiable* — an unreadable, malformed, truncated, or non-ELF candidate, or
a candidate with no `GLIBC_` version requirement — is a recorded status, not a
failed release: the qualification still passes and the reason says exactly why
the floor was not confirmed.

**Cross-checks.** A plan that declares a floor and does not state a status for
it is invalid: the status is a pure function of the policy
(`planned_floor_status`, `src/floor.rs:105-132`), so a document that omits it
is read as `NotDeclared` and then refused (`ReleasePlan::validate`,
`src/lib.rs:646-675`). An evidence record whose floor status contradicts the
plan's declared floor is likewise refused (`:350-360`): the status must be
admissible for the declared floor and strategy
(`floor_status_admissible`, `src/floor.rs:133-155`), and a `NotSatisfied`
status is admissible only together with the matching failure.

**Scope.** A macOS deployment floor is not independently verified: reading the
Mach-O `minos` load command is a named follow-on, so a declared macOS floor is
recorded as `Unverified(FloorKindNotVerifiable)`. A `CargoZigbuild` target is
`ToolchainBound` — the floor was bound into the toolchain by the triple
rewrite, and the toolchain is trusted to honour it; no independent byte
inspection is claimed for it.

## The evidence lifecycle

1. **Pre-hash and structural inspection.** Every candidate is inspected before
   anything executes (`inspect_candidate`, `:1162-1246`): absolute path,
   non-symlink regular file, non-empty, size equal to the `BuildAttempt`'s
   recorded size (else `CandidateChanged`, `:1168-1177`); both parent
   directories non-symlink directories (`:1180-1195`); the canonicalized path
   contained by the canonicalized target directory (`:1196-1200`); the
   target-implied `(format, architecture)` matched against the parsed header
   (`:1127-1154`, `:1201-1207`); then the compatibility-floor inspection
   (`:1208-1215`) and a streaming SHA-256 (`:1216-1222`) with a
   post-read re-check of size, file type, and canonical identity (`:1223-1232`).
2. **Method selection and host gate** (`:728-833`). Structural failures
   short-circuit here with an *empty* candidate list (`:677-687`), which
   `validate_for` accommodates by relaxing the candidate-count equality for
   `Failed` records (`:346-347`). A floor contradicted by the bytes
   short-circuits here too (`:705-725`), before anything executes.
3. **Execution** (`:834-948`): preflight for QEMU, then the smoke.
4. **Status from the process** (`:949`, `:1021-1046`): `Success` → `Passed`;
   `TimedOut` → `SmokeTimedOut`, or `EmulatorFailed` under QEMU;
   `OutputLimitExceeded` → `OutputLimitExceeded` for either method;
   `Failed(_)`/`Cancelled` → `SmokeFailed`, or `EmulatorFailed` under QEMU.
5. **Post-hash** (`:955-978`). Only on `Passed` are all candidates re-inspected;
   any size or digest difference — or any inspection error — downgrades the
   record to `Failed(CandidateChanged)`. This is what makes mid-run mutation
   detectable: the bytes that ran are bracketed by two independent hashes.
6. **Assembly** (`make_evidence`, `:993-1020`): every record is built from the
   plan and the observed host. Nothing is inferred.

`validate_for` (`:331-492`) then re-reconciles a record against
`(plan, target, attempt)`: identity and bounds (`:337-365`), duplicate-selector
rejection (`:366-374`), per-candidate selector/package/binary/size agreement plus
digest *shape* (`:375-387`), strictly increasing selector order and process
bounds (`:388-401`), the method/classification pairing (`:402-413`), the
status/method/process-count rules (`:414-490`), and the floor-status agreement
with the plan's declared floor (`:350-360`). A deliberate limit: it checks
the digest's format, not its value, and never compares `format`/`architecture`
against the target. It reconciles evidence against the plan and attempt; it does
not re-read bytes. Candidate bytes are proved by `inspect_candidate`.

Determinism: `candidates` is emitted in `BTreeMap` selector order (`:624-630`,
`:662-668`) and required to be strictly increasing on validation (`:388-392`);
`processes` is in execution order. The record round-trips through JSON unchanged
(`:2341-2344`).

## Bounded execution

Every execution is a `CommandSpec` handed to the builder's bounded runner
(`:840-850`, `:901-926`), with `env: BTreeMap::new()` and `expected_stdout: None` — a
cleared environment plus the builder's small allowlist, and no shell: the
candidate path and the fixed argv are separate OS arguments (`:917-918`). The
process is spawned as a group; on deadline or cancellation the whole group is
killed and waited before the outcome is classified (`src/builder.rs532-581`).

| Bound | Source | Value |
|---|---|---|
| Smoke deadline | `timeout_ms` (`:908`, `:921`); validated `1..=86_400_000` (`:151-152`) | per-binding, ≤24 h |
| QEMU preflight deadline | hardcoded (`:845`) | 10 s |
| Retained stdout/stderr | per-binding (`:909-910`, `:922-923`); validated `1..=262_144` (`:155-156`) | ≤256 KiB each |
| Preflight capture | hardcoded (`:846-847`) | 8 KiB each |
| Args | ≤128, ≤4096 B each, no NUL or ASCII control characters (`:157-162`) | fixed, never shell-expanded |
| Executable allowlist | `src/builder.rs435-455` | candidate must be an absolute path; QEMU only `qemu-x86_64`/`qemu-aarch64`/`qemu-arm` |

**Cancellation** is checked twice and both paths are typed. Before the smoke, an
already-cancelled flag yields `Failed(SmokeFailed)` (`:887-898`); during the run,
the builder's poll loop kills the group and returns `CommandOutcome::Cancelled`
(`src/builder.rs555-563`), which `status_from_process` maps to `SmokeFailed`
(`:1021-1046`). There is deliberately no `Cancelled` failure variant — the
distinction lives in the bounded process evidence, not the status taxonomy. The
four typed terminal outcomes (non-zero, timeout, cancellation, output limit) are
asserted by `native_smoke_failure_timeout_cancellation_and_output_limit_are_typed`
(`:2489-2516`).

Two details: the smoke's working directory is the candidate's *grandparent*,
canonicalized (`:1061-1069`) — the per-target root, not `release/`; and the
builder restores the candidate's exec bit before spawn
(`src/builder.rs456-464`), a mode-only change that leaves the recorded size and
digest unaffected.

Information boundary: the runner closure discards process error detail — every
runner error is mapped to `()` and then to a typed `QualificationFailure`
(`:851-852`, `:929-931`), so spawn failure, bounds rejection, and process-group cleanup
failure are indistinguishable in the evidence. The record says what happened, not
why the harness could not report it.

## Key types / functions (`crates/eggpack-core/src/qualification.rs`)

**Bindings (producer intent):**

- `QualificationError(String)` (`:30-36`); private `qerr` (`:37-43`).
- `CandidateSmokeBinding{selector, argv, timeout_ms, stdout_limit, stderr_limit}`
  (`:44-60`).
- `TargetQualificationBinding{smoke?}` (`:61-69`).
- `QualificationBindingsV1{schema_version, targets}` (`:70-76`) + impl
  (`:77-173`): `from_toml` (`:79-85`), `validate_for(plan, build_bindings)`
  (`:86-135`) — exact target-inventory equality with the plan, smoke present
  *iff* the classification executes (`:107-115`), the smoke selector must
  exist in the build bindings (`:116-127`), and `DeferredNative` requires
  `qualification_host` (`:128-132`); private `validate_shape` (`:137-173`).
- `QualificationRuntime{qemu_sysroot?}` (`:174-181`) — the only runtime input.

**Method and outcome taxonomy:**

- `QualificationMethod::Native | Deferred | DeferredNativeOnNativeHost |
  QemuUser | Structural` (`:182-197`).
- `QualificationStatus::Passed | Deferred | Failed(QualificationFailure)`
  (`:198-209`).
- `QualificationFailure::{StructuralMismatch, CandidateChanged, HostMismatch,
  SmokeFailed, SmokeTimedOut, OutputLimitExceeded, EmulatorUnavailable,
  EmulatorFailed, InvalidRuntimeEnvironment, FloorNotSatisfied}` (`:210-236`).
  `FloorNotSatisfied` is the contradicted-floor failure; it is the only
  failure a `NotSatisfied` floor status may accompany.
- `CandidateFormat::{Elf, PeCoff, MachO}` (`:237-248`); thin Mach-O only.
- `CandidateArchitecture::{X86_64, Aarch64, Armv7}` (`:249-260`).

**Evidence:**

- `QualifiedCandidateEvidence{selector, package, binary, size, sha256, format,
  architecture}` (`:261-280`).
- `QualificationProcessEvidence{purpose, selector?, process}` (`:281-292`) —
  `purpose` is `candidate_smoke` or `qemu_preflight`; no output contents, no
  environment.
- `QualificationEvidence{schema_version, release_id, source_revision, target,
  planned_classification, method, actual_host, support, status, floor,
  candidates, smoke_selector?, processes}` (`:293-328`) + impl (`:329-493`).
  `floor` is the compatibility-floor verification status of the inspected
  bytes (`FloorVerificationStatus`, `src/floor.rs:72-89`).

**Entry points and internals:**

- `QualificationRequest<'a>` (`:496-515`); `qualify_target` (`:516-543`).
- `qualify_target_for_host` (`:544-577`); `qualify_target_for_host_with_runner`
  (`:578-990`) — the whole stage body.
- `make_evidence` (`:993-1020`); `status_from_process` (`:1021-1046`);
  `validate_runtime` (`:1047-1060`); `qualification_cwd` (`:1061-1069`);
  `actual_host` (`:1070-1086`); `target_matches_host` (`:1087-1100`);
  `qemu_for_target` (`:1101-1112`); `qemu_args` (`:1113-1126`);
  `target_identity` (`:1127-1154`); `inspect_candidate` (`:1162-1246`);
  `parse_binary_header` (`:1247-1314`) — ELF magic/class/machine
  (`:1255-1266`), `MZ` + bounded `PE\0\0` header offset (`:1268-1291`), thin
  Mach-O magic plus `cputype` in either endianness (`:1293-1310`). The parser is
  format-directed: it is told which format the target implies and fails when the
  magic disagrees, so a fat/universal Mach-O is rejected (`:1549-1551`).

**Tests** (`:1316-2981`): host-adaptive fixtures using the test binary itself as
the candidate, minimal ELF/PE/Mach-O byte builders (`:1480-1520`), and coverage
for strict bindings, exact smoke selectors, inventory reconciliation, the
cross-tool native pass, host mismatch, both deferred paths, the QEMU argv
contract, cancellation/timeout/output-limit typing, symlink and relative-path
rejection, and direct/bundle/archive inventories.

## Boundaries / non-goals

- **Does not build.** It consumes a `BuildAttempt` and requires its process
  outcome to be `Success` (`:601`). A build success never implies a
  qualification pass: `inspect_candidate` always runs first (`:662-694`).
- **Does not finalize.** No output root, no copying, no archive, no manifest. Its
  only output is the evidence record.
- **Does not assert authenticity or provenance.** SHA-256 and size are integrity
  facts about specific bytes. No signature, signing key, attestation, or trust
  anchor exists in this module.
- **Does not produce installed-state receipts.** It observes a candidate *before*
  installation; Eggup's receipts are the installed-state authority.
- **No network, no cross-tool provisioning, no repository scripts.** The only
  executables that can run are the exact candidate path and the three fixed
  `qemu-*` names; the binding schema has no executable, environment, or command
  field (`:44-60`).
- **No `Qualification` growth.** Neither the classification nor the method enum
  gained a variant for any new topology, and `schema_version` stayed 1
  (`plans/closure/build-qualification/006-status.md`).
- **No degenerate records.** Every executing classification needs a bounded smoke
  binding (`:834`, `:423-428`); `Structural` with a smoke is refused (`:421-422`);
  a `Passed` record with a non-`Success` process is refused (`:453-457`).

## Dependencies / dependents

- Module dependencies: `crate::builder::{run_qemu_process,
  run_qualification_process, CandidateArtifact, CommandSpec, ProcessEvidence}`
  (`:3-8`), the planning types (`BuildAttempt`, `BuildBindingsV1`,
  `BuildCancellation`, `FloorUnverifiedReason`, `FloorVerificationStatus`,
  `HostArch`, `HostOs`, `HostRequirement`, `LogicalOutputSelector`,
  `PlannedTarget`, `Qualification`, `ReleasePlan`, `SupportTier`),
  `eggpack_contract::DistributionContract`, `serde`, and `std` (`:9-23`). The
  candidate bytes are digested by the crate's single file-digest authority
  (`crate::file_digest`), not by a private SHA-256 stream in this module.
  Crate-level dependencies are `Cargo.toml:17-26`; this file has no
  independent set.
- In-crate dependents: `finalization.rs` re-validates this record
  (`finalization.rs:193-196`) and gates on it — `Passed`, or `Deferred` when the
  support tier is not `Required` (`finalization.rs:197-200`).
- External: `eggpack-ci` encodes, decodes, and aggregates `QualificationEvidence`
  (`eggpack-ci/src/lib.rs2727-2732`, `:2743-2785`); `eggpack-cli` drives it
  through the `ci _qualify-target` runner (`eggpack-cli/src/main.rs517-588`),
  which is why candidate paths are absolutized there (`main.rs522-524`) —
  `inspect_candidate` requires absolute paths (`:1168`).
- `eggpack-github` and `eggpack-bootstrap` do not use this module.

## Sibling deep dives

- [core.md](core.md) — crate-level orientation and cross-stage data contracts.
- [core-planning.md](core-planning.md) — `PackConfig` → `ReleasePlan`, the policy
  types this stage reads.
- [core-build.md](core-build.md) — the `BuildAttempt` and candidate bytes
  consumed here, and the bounded runner it also provides.
- [core-finalization.md](core-finalization.md) — the stage that gates on this
  record and writes final bytes.
- [overview.md](overview.md) — module map and the producer pipeline.
- [process-execution.md](process-execution.md) — cross-cutting process groups,
  environment clearing, and cancellation.
