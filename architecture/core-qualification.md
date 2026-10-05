# `eggpack-core` — Qualification stage (`src/qualification.rs`) — Deep Dive

Binds one fixed-argument smoke check per target, structurally inspects each
candidate's exact bytes, runs a bounded execution on a host-matched or emulated
runtime, and records `QualificationEvidence` about those bytes. 2610 lines
(`src/qualification.rs`; `:1243-2610` is the inline test module). This is one of
four `eggpack-core` stage deep dives; `core.md` holds the crate-level
orientation.

## Inputs and outputs

**Input** — `QualificationRequest<'a>` (`:477-494`) is all-or-nothing: a
`DistributionContract`, the invocation `ReleasePlan`, exactly one
`PlannedTarget`, a successful `BuildAttempt`, `BuildBindingsV1`,
`QualificationBindingsV1`, a `QualificationRuntime`, and a `BuildCancellation`.
There is no partial or defaulted input and no output-directory parameter.

**Output** — `Result<QualificationEvidence, QualificationError>` (`:497-522`).
The error variant means *the request was malformed*; it never means *the
candidate was bad*. A structurally wrong candidate, a non-matching host, a
failed smoke, and a mid-run mutation all return `Ok` with a typed
`QualificationStatus::Failed` record (`:654-663`, `:689-698`, `:800-810`,
`:873-887`, `:910-913`). Failures are evidence; only refused work is an error.

**Rejected before any inspection** (`:574-641`), in order: plan/attempt identity
— schema version, non-empty `release_id`/`source_revision`, target membership,
matching `release_id`/`source_revision`/`target`/`strategy`, a `Success` build
process, and 1..=256 candidates (`:574-589`); then `BuildBindingsV1::validate_for`
(`:590-592`) and `QualificationBindingsV1::validate_for` (`:593`), both re-run
internally rather than trusted from the caller; then the runtime (`:594`, `:982-994`);
then the QEMU-sysroot-requires-`Emulated` rule (`:595-599`); then an exact
selector/package/binary reconciliation of the attempt's candidates against the
build bindings (`:601-641`).

**Never in the evidence**: candidate paths, output contents, environment values.
Only the canonicalized working directory and the absolute candidate path reach
the child process (`:790`, `:849`, `:861`); `:1724-1725` asserts the serialized
evidence contains no candidate path.

**Module bounds** (`:22-26`): 256 targets, 128 args, 4096 bytes per arg, 256 KiB
retained capture, 4096-byte binary-header window. `schema_version` is exactly 1
on both `QualificationBindingsV1` (`:138`) and `QualificationEvidence` (`:325`,
`:941`).

## The method taxonomy

Requested classification is `Qualification` (intent, `src/lib.rs:336-355`);
the method actually applied is `QualificationMethod` (`:180-193`). The pairing
is closed — one classification admits exactly two or one methods, and any other
combination is rejected by `validate_for` (`:383-393`).

| Requested | Method (`:182-193`) | Executes | Proves | Does **not** prove |
|---|---|---|---|---|
| `Native` | `Native` | yes, direct | the exact pre-hashed bytes ran on a host whose OS/arch match the target triple, exited 0 under the fixed argv, and were byte-identical afterwards (`:683-701`, `:857-869`, `:896-916`) | that any other host runs it; that a declared floor is honoured |
| `DeferredNative` | `Deferred` | no | only that the bytes are structurally valid for the target, and that the required host has not yet run them (`:702-718`) | any execution — `Deferred` can never be `Passed` (`:400-401`, `:465`) |
| `DeferredNative` | `DeferredNativeOnNativeHost` | yes, direct | the same as `Native`, on the separately declared `qualification_host` (`:719-731`) | anything about the deferred hosts that were skipped |
| `Emulated` | `QemuUser` | yes, under QEMU | the target is `-linux-`, every candidate is ELF, the finite `qemu-*` mapping answers `--version`, and the bytes ran under it to a zero exit (`:733-767`, `:784-869`) | native execution on the target; performance; fidelity outside the emulated user-mode surface |
| `Structural` | `Structural` | never | file shape and header identity only: non-symlink regular file under a canonical root, size equal to the `BuildAttempt`, ELF/PE/thin-Mach-O header agreeing with the target's format and architecture, plus a SHA-256 (`:682`, `:769-780`, `:1087-1173`) | that the candidate runs anywhere |

**Structural inspection is not executed evidence.** `Structural` records zero
processes and no `smoke_selector`, and that emptiness is a validated invariant
rather than an accident (`:402-403`, `:464`). A `Structural` pass is a statement
about bytes on disk; the three executing methods additionally carry
`QualificationProcessEvidence` — and a `Passed` record must carry exactly the
right number of them (`:459-466`): 1 for native, 2 for QEMU (preflight plus
smoke), 0 for structural, never for deferred.

The QEMU mapping is a closed table, not a template: `-linux-` targets only,
`x86_64` → `qemu-x86_64`, `aarch64` → `qemu-aarch64`, `armv7` → `qemu-arm`
(`:1036-1046`), and the argv is built solely from the optional declared sysroot,
the candidate path, and the binding's fixed argv (`:1048-1060`, asserted at
`:2080-2088`). The optional sysroot must be an absolute, non-symlink directory
(`:982-994`).

## Host matching: native qualification is builder-independent

`Qualification::Native` is host-matched proof about the exact candidate bytes,
and it is **independent of which builder produced them**. The `Native` branch
(`:683-701`) matches on the host and the target triple only; the single
`strategy` reference in the module's production code is the identity check
`attempt.strategy != target.policy.strategy` (`:581`). There is no strategy
term in the method selection, so a `CargoZigbuild`-built candidate reaching this
host is a legitimate `QualificationMethod::Native` record.
`Qualification::Native`'s own rustdoc states the same
(`src/lib.rs:340-348`), and `cross_tool_built_candidate_qualifies_natively_on_the_matching_host`
(`:1770-1889`) proves it end to end, including through the public
`qualify_target` entry point.

The rule, precisely:

1. The effective qualification host is `policy.qualification_host`, else the
   build host `{policy.host_os, policy.host_arch}` (`:684-687`).
2. Both conditions must hold: `host == required` **and**
   `target_matches_host(&target.target, host)` (`:688`).
   `target_matches_host` (`:1022-1034`) is a pure triple↔host mapping — OS by
   triple substring/suffix (`-linux-`, `-apple-darwin`, `-windows-`), arch by
   triple prefix (`x86_64-`, `aarch64-`, `armv7-`).
3. On failure the function returns `Ok` with
   `Failed(QualificationFailure::HostMismatch)`, method still `Native`, and
   `processes: Vec::new()` (`:689-698`). **Never a pass, never a skip, never an
   executed smoke.** The failed record is itself valid evidence
   (`:1863-1871`).

The same rule is re-enforced in reverse by `QualificationEvidence::validate_for`,
so a hand-edited or forged pass record cannot survive: a pass on a native method
requires `target_matches_host` (`:410-415`) and requires any declared
`qualification_host` to equal the observed `actual_host` for all three executing
classifications (`:416-433`). `Deferred` is asymmetric by design and is
constrained the other way (`:394-399`): method `Deferred`, no processes, a
declared `qualification_host` present, and it must *differ* from the actual
host. The test `native_host_mismatch_is_a_failure_and_never_runs_smoke`
(`:1937-1978`) and the off-host/`Deferred` cases at `:1575-1613` and
`:1980-2056` cover both directions.

Observed host facts come from `actual_host()` (`:1005-1020`), which maps only
`std::env::consts::OS`/`ARCH` for linux/macos/windows and x86_64/aarch64/
arm-v7, and returns an error for anything else. `qualify_target_for_host`
(`:525-556`) and `qualify_target_for_host_with_runner` (`:559-927`) exist so a
host and a process runner can be injected deterministically in tests; the public
`qualify_target` is a thin wrapper that supplies the real host.

## What native qualification does not prove

A successful native run proves the exact candidate bytes execute on *this*
host's operating system and architecture. It is **not** independent proof that
the produced binary honours a declared `CompatibilityFloor` — a glibc or macOS
deployment floor. This is a documented, deliberate limitation:

- `TargetPolicy::floor` is never read anywhere in this module. The only
  occurrences are test fixtures (`:1753-1760`, `:2536`, `:2583`). Qualification
  neither inspects GLIBC symbol versions nor any Mach-O load-command minimum
  version.
- The floor's own rustdoc says so: "Native qualification evidence proves that
  the exact candidate executes on the qualifying native host; it is not
  independent proof that the produced binary honours this declared minimum
  runtime" (`src/lib.rs:433-439`).
- The accepted decision is recorded in
  `plans/closure/build-qualification/006-status.md` (ADR-0005, Option A), which
  lists independent GLIBC-symbol-floor verification as explicitly not attempted
  and carried as residual risk.

The other residual: `target_matches_host` compares OS family and CPU
architecture only. It does not distinguish a Linux distribution or glibc
version, so two hosts on the same triple are treated as interchangeable
capabilities. This follows from `HostRequirement` carrying exactly
`{os, arch}` (`src/lib.rs:367-375`).

## The evidence lifecycle

1. **Pre-hash and structural inspection.** Every candidate is inspected before
   anything executes (`inspect_candidate`, `:1087-1173`): absolute path,
   non-symlink regular file, non-empty, size equal to the `BuildAttempt`'s
   recorded size (else `CandidateChanged`, `:1100-1102`); both parent
   directories non-symlink directories (`:1105-1120`); the canonicalized path
   contained by the canonicalized target directory (`:1121-1125`); the
   target-implied `(format, architecture)` matched against the parsed header
   (`:1062-1085`, `:1126-1132`); then a streaming SHA-256 (`:1133-1149`) with a
   post-read re-check of size, file type, and canonical identity (`:1150-1159`).
2. **Method selection and host gate** (`:681-768`). Structural failures
   short-circuit here with an *empty* candidate list (`:654-663`), which
   `validate_for` accommodates by relaxing the candidate-count equality for
   `Failed` records (`:334-335`).
3. **Execution** (`:781-888`): preflight for QEMU, then the smoke.
4. **Status from the process** (`:889`, `:956-980`): `Success` → `Passed`;
   `TimedOut` → `SmokeTimedOut`, or `EmulatorFailed` under QEMU;
   `OutputLimitExceeded` → `OutputLimitExceeded` for either method;
   `Failed(_)`/`Cancelled` → `SmokeFailed`, or `EmulatorFailed` under QEMU.
5. **Post-hash** (`:896-916`). Only on `Passed` are all candidates re-inspected;
   any size or digest difference — or any inspection error — downgrades the
   record to `Failed(CandidateChanged)`. This is what makes mid-run mutation
   detectable: the bytes that ran are bracketed by two independent hashes.
6. **Assembly** (`make_evidence`, `:930-954`): every record is built from the
   plan and the observed host. Nothing is inferred.

`validate_for` (`:319-473`) then re-reconciles a record against
`(plan, target, attempt)`: identity and bounds (`:325-342`), duplicate-selector
rejection (`:343-351`), per-candidate selector/package/binary/size agreement plus
digest *shape* (`:352-368`), strictly increasing selector order and process
bounds (`:369-382`), the method/classification pairing (`:383-393`), and the
status/method/process-count rules (`:394-466`). A deliberate limit: it checks
the digest's format, not its value, and never compares `format`/`architecture`
against the target. It reconciles evidence against the plan and attempt; it does
not re-read bytes. Candidate bytes are proved by `inspect_candidate`.

Determinism: `candidates` is emitted in `BTreeMap` selector order (`:612`,
`:643-644`) and required to be strictly increasing on validation (`:369-372`);
`processes` is in execution order. The record round-trips through JSON unchanged
(`:2265-2268`).

## Bounded execution

Every execution is a `CommandSpec` handed to the builder's bounded runner
(`:843-869`), with `env: BTreeMap::new()` and `expected_stdout: None` — a
cleared environment plus the builder's small allowlist, and no shell: the
candidate path and the fixed argv are separate OS arguments (`:859-860`). The
process is spawned as a group; on deadline or cancellation the whole group is
killed and waited before the outcome is classified (`src/builder.rs:478-527`).

| Bound | Source | Value |
|---|---|---|
| Smoke deadline | `timeout_ms` (`:851`, `:863`); validated `1..=86_400_000` (`:151-152`) | per-binding, ≤24 h |
| QEMU preflight deadline | hardcoded (`:792`) | 10 s |
| Retained stdout/stderr | per-binding (`:852-853`, `:864-865`); validated `1..=262_144` (`:153-156`) | ≤256 KiB each |
| Preflight capture | hardcoded (`:793-794`) | 8 KiB each |
| Args | ≤128, ≤4096 B each, no NUL or ASCII control characters (`:157-162`) | fixed, never shell-expanded |
| Executable allowlist | `src/builder.rs:396-415` | candidate must be an absolute path; QEMU only `qemu-x86_64`/`qemu-aarch64`/`qemu-arm` |

**Cancellation** is checked twice and both paths are typed. Before the smoke, an
already-cancelled flag yields `Failed(SmokeFailed)` (`:831-842`); during the run,
the builder's poll loop kills the group and returns `CommandOutcome::Cancelled`
(`src/builder.rs:501-509`), which `status_from_process` maps to `SmokeFailed`
(`:972-978`). There is deliberately no `Cancelled` failure variant — the
distinction lives in the bounded process evidence, not the status taxonomy. The
four typed terminal outcomes (non-zero, timeout, cancellation, output limit) are
asserted by `native_smoke_failure_timeout_cancellation_and_output_limit_are_typed`
(`:2404-2431`).

Two details: the smoke's working directory is the candidate's *grandparent*,
canonicalized (`:996-1003`) — the per-target root, not `release/`; and the
builder restores the candidate's exec bit before spawn
(`src/builder.rs:416-424`), a mode-only change that leaves the recorded size and
digest unaffected.

Information boundary: the runner closure discards process error detail — every
runner error is mapped to `()` and then to a typed `QualificationFailure`
(`:536-543`), so spawn failure, bounds rejection, and process-group cleanup
failure are indistinguishable in the evidence. The record says what happened, not
why the harness could not report it.

## Key types / functions (`crates/eggpack-core/src/qualification.rs`)

**Bindings (producer intent):**

- `QualificationError(String)` (`:30-36`); private `qerr` (`:37-39`).
- `CandidateSmokeBinding{selector, argv, timeout_ms, stdout_limit, stderr_limit}`
  (`:44-56`).
- `TargetQualificationBinding{smoke?}` (`:61-65`).
- `QualificationBindingsV1{schema_version, targets}` (`:70-75`) + impl
  (`:77-170`): `from_toml` (`:79-83`), `validate_for(plan, build_bindings)`
  (`:86-135`) — exact target-inventory equality with the plan, smoke present
  *iff* the classification executes (`:107-115`), the smoke selector must
  exist in the build bindings (`:116-127`), and `DeferredNative` requires
  `qualification_host` (`:128-132`); private `validate_shape` (`:137-169`).
- `QualificationRuntime{qemu_sysroot?}` (`:174-177`) — the only runtime input.

**Method and outcome taxonomy:**

- `QualificationMethod::Native | Deferred | DeferredNativeOnNativeHost |
  QemuUser | Structural` (`:182-193`).
- `QualificationStatus::Passed | Deferred | Failed(QualificationFailure)`
  (`:198-205`).
- `QualificationFailure::{StructuralMismatch, CandidateChanged, HostMismatch,
  SmokeFailed, SmokeTimedOut, OutputLimitExceeded, EmulatorUnavailable,
  EmulatorFailed, InvalidRuntimeEnvironment}` (`:210-229`).
- `CandidateFormat::{Elf, PeCoff, MachO}` (`:234-241`); thin Mach-O only.
- `CandidateArchitecture::{X86_64, Aarch64, Armv7}` (`:246-253`).

**Evidence:**

- `QualifiedCandidateEvidence{selector, package, binary, size, sha256, format,
  architecture}` (`:258-273`).
- `QualificationProcessEvidence{purpose, selector?, process}` (`:278-285`) —
  `purpose` is `candidate_smoke` or `qemu_preflight`; no output contents, no
  environment.
- `QualificationEvidence{schema_version, release_id, source_revision, target,
  planned_classification, method, actual_host, support, status, candidates,
  smoke_selector?, processes}` (`:290-315`) + impl (`:317-474`).

**Entry points and internals:**

- `QualificationRequest<'a>` (`:477-494`); `qualify_target` (`:497-522`).
- `qualify_target_for_host` (`:525-556`); `qualify_target_for_host_with_runner`
  (`:559-927`) — the whole stage body.
- `make_evidence` (`:930-954`); `status_from_process` (`:956-980`);
  `validate_runtime` (`:982-994`); `qualification_cwd` (`:996-1003`);
  `actual_host` (`:1005-1020`); `target_matches_host` (`:1022-1034`);
  `qemu_for_target` (`:1036-1046`); `qemu_args` (`:1048-1060`);
  `target_identity` (`:1062-1085`); `inspect_candidate` (`:1087-1173`);
  `parse_binary_header` (`:1175-1241`) — ELF magic/class/machine
  (`:1182-1195`), `MZ` + bounded `PE\0\0` header offset (`:1196-1219`), thin
  Mach-O magic plus `cputype` in either endianness (`:1220-1239`). The parser is
  format-directed: it is told which format the target implies and fails when the
  magic disagrees, so a fat/universal Mach-O is rejected (`:1476-1478`).

**Tests** (`:1243-2610`): host-adaptive fixtures using the test binary itself as
the candidate, minimal ELF/PE/Mach-O byte builders (`:1407-1445`), and coverage
for strict bindings, exact smoke selectors, inventory reconciliation, the
cross-tool native pass, host mismatch, both deferred paths, the QEMU argv
contract, cancellation/timeout/output-limit typing, symlink and relative-path
rejection, and direct/bundle/archive inventories.

## Boundaries / non-goals

- **Does not build.** It consumes a `BuildAttempt` and requires its process
  outcome to be `Success` (`:582`). A build success never implies a
  qualification pass: `inspect_candidate` always runs first (`:643-670`).
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
  field (`:44-56`).
- **No `Qualification` growth.** Neither the classification nor the method enum
  gained a variant for any new topology, and `schema_version` stayed 1
  (`plans/closure/build-qualification/006-status.md`).
- **No degenerate records.** Every executing classification needs a bounded smoke
  binding (`:781`, `:404-409`); `Structural` with a smoke is refused (`:111-115`);
  a `Passed` record with a non-`Success` process is refused (`:434-438`).

## Dependencies / dependents

- Module dependencies: `crate::builder::{run_qemu_process,
  run_qualification_process, CandidateArtifact, CommandSpec, ProcessEvidence}`
  (`:3-10`), the planning types (`BuildAttempt`, `BuildBindingsV1`,
  `BuildCancellation`, `HostArch`, `HostOs`, `HostRequirement`,
  `LogicalOutputSelector`, `PlannedTarget`, `Qualification`, `ReleasePlan`,
  `SupportTier`), `eggpack_contract::DistributionContract`, `serde`, and `sha2`
  (`:11-13`). Crate-level dependencies are `Cargo.toml:14-23`; this file has no
  independent set.
- In-crate dependents: `finalization.rs` re-validates this record
  (`finalization.rs:192-195`) and gates on it — `Passed`, or `Deferred` when the
  support tier is not `Required` (`finalization.rs:196-200`).
- External: `eggpack-ci` encodes, decodes, and aggregates `QualificationEvidence`
  (`eggpack-ci/src/lib.rs:2725-2730`, `:2743-2785`); `eggpack-cli` drives it
  through the `ci _qualify-target` runner (`eggpack-cli/src/main.rs:528-599`),
  which is why candidate paths are absolutized there (`main.rs:533-535`) —
  `inspect_candidate` requires absolute paths (`:1093`).
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
