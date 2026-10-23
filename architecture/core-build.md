# `eggpack-core` — Build stage (`src/builder.rs`) — Deep Dive

Explicit Cargo build bindings, the two first-party Cargo command adapters, bounded
cancellable process execution, private per-invocation target storage, and candidate byte
discovery. One of four `eggpack-core` stage deep dives (planning, build, qualification,
finalization); the stage between `ReleasePlan` (intent) and `QualificationEvidence`
(proof). Single file `crates/eggpack-core/src/builder.rs` (1734 lines,
`#![forbid(unsafe_code)]` inherited from the workspace), re-exported wholesale at crate
root (`crates/eggpack-core/src/lib.rs:11`).

## Inputs and outputs

- in: `&ReleasePlan`, `&PlannedTarget` — from `PackConfig::resolve`
  (`crates/eggpack-core/src/lib.rs:867`, `:758`).
- in: `&[BuildBinding]` — the caller's per-target slice, **not** coverage-validated here.
- in: `&DistributionContract` — only via `BuildBindingsV1::validate_for` (`:341`).
- in: `repository_root`, `work_root`, `invocation`, `&BuildCancellation`.
- out: `BuildAttempt` (`:284`) — identity, bounded `ProcessEvidence`, `Vec<CandidateArtifact>`.
- out: `ProcessEvidence` (`:256`) — outcome plus retained byte **counts** only.

`BuildAttempt` is identity-bound to `release_id` + `source_revision` (`:286`, `:288`) and to
the one `target` / `strategy` it ran (`:290`, `:292`). It records what was attempted, not
what was proved: this module never inspects a candidate's format, architecture, or runtime
behaviour, so `Success` means a command exited zero — never that the candidate qualifies.

## The binding model

A contract output slot is not a filesystem path, so each slot is bound by hand to a Cargo
package and binary target.

- `LogicalOutputSelector` (`:76`), tagged and `deny_unknown_fields` (`:75`): `Direct`
  (`:78`); `BundleEntry{index}` indexed by **contract order** (`:79-83`);
  `ArchiveMember{source}` keyed by the **exact contract source string** (`:84-88`).
- `BuildBinding{selector, package, binary}` (`:94-101`) pairs a slot with one Cargo
  `--package` / `--bin` pair.
- `BuildBindingsV1{schema_version, targets: BTreeMap<String, Vec<BuildBinding>>}`
  (`:106-111`), strict version 1. `from_toml` (`:303-308`) parses via `toml::from_str` then runs
  `validate_shape` (`:309-339`).

`validate_shape` (`:309-339`) is the fail-closed bound layer: `schema_version == 1` and
1..=256 targets (`:310-312`); per target a non-empty key and 1..=256 bindings (`:313-316`); unique
selectors plus `valid_identifier` package/binary (`:317-324`), where
`valid_identifier` (`:383-388`) allows only `[A-Za-z0-9_.-]`, ≤128 bytes.
`ArchiveMember.source` must be a non-empty ≤255-byte relative path with no leading `/`
and no empty/`.`/`..` component (`:325-335`).

**Exact plan coverage** is `validate_for` (`:341-381`). For every `plan.targets` entry it
looks up `self.targets[&target.target]` (`:348-351`, else `missing target bindings`),
expands the contract (`:352-354`), and derives the expected slot set from the expanded
`ExpandedAssets` variant — `Direct` ⇒ `{Direct}`; `Bundle` ⇒ `BundleEntry{0..entries.len()}`;
`Archive` ⇒ `ArchiveMember{source}` per member (`:355-365`). The binding selectors must
then be **equal as sets** (`:366-367`): equality in both directions means neither a
missing nor an extra slot is accepted, reported as the single `bindings do not exactly
cover contract Cargo slots` (`:368-371`). Any binding target key absent from the plan is
rejected as `bindings include target outside ReleasePlan` (`:373-379`). Contract expansion
is the authority: a `BuildBinding` cannot redefine layout, only map an already-contractual
slot onto a Cargo target.

## Command construction by build strategy

`cargo_command` (`:391-458`) is the only builder and is total over the two
`BuildStrategy` variants. It re-validates both identifiers (`:398-400`), so it is safe to
call with a hand-built binding.

| Aspect | `NativeCargo` (`:408`) | `CargoZigbuild` (`:409`) |
| --- | --- | --- |
| subcommand | `build` | `zigbuild` |
| shared args | `+<toolchain.rust>`, `--release`, `--locked`, `--target <triple>` | identical |
| glibc floor | not encoded | `args[5]` rewritten to `<triple>.<major>.<minor>` (`:415-418`) |
| macOS floor | n/a | rejected: `macOS floor cannot use cargo-zigbuild` (`:419-429`) |

Both append exactly `--package <package> --bin <binary>` (`:436-441`). The executable is
always the bare tool name `cargo` (`:448`); executable and argv are separate OS arguments, so
no shell is involved (`:158`). One environment variable is injected, `CARGO_TARGET_DIR` =
the private target directory (`:442-446`); `cwd` is the caller's (`:450`), both stream limits
are `MAX_OUTPUT` (256 KiB, `:20-28`, `:453-454`), and `expected_stdout` is `None` (`:455`).
`execute_target_cancellable` supplies a fixed 1800 s build timeout (`:1058`).

`preflight` (`:899-998`) runs before any build: `rustc +<toolchain> --version` (`:906`) and
`cargo +<toolchain> --version` (`:926`), each 10 s / 1024 bytes, failing closed on any
non-`Success` outcome (`:921-923`, `:941-943`). Under `CargoZigbuild` it also requires both
cross-tool versions to be declared (`:944-956`) and requires each declared pin to be a complete
`MAJOR.MINOR.PATCH` version. `cargo zigbuild --version` and `zig version` are matched by
**trimmed equality** (`expected_stdout_exact`), not by substring, so a pin of `0.23.3` is not
satisfied by a reported `0.23.30`; mismatch yields `cargo-zigbuild version mismatch` / `Zig
version mismatch`. The expected value is compared internally and never
surfaced (`:175-181`).

## Bounded cancellable process execution

All execution funnels through the private `run_bounded` (`:628-848`), reached through
`run_bounded_inner` (`:608-626`). Five entry points select an `ExecutableAllowance`
(`:127-156`):

| Entry | Allowance | Executable rule |
| --- | --- | --- |
| `run_bounded_cancellable` (`:461`) | `Builder` | `cargo` \| `rustc` \| `zig` (`:634-636`) |
| `run_qualification_process` (`:569`) | `Candidate` | must be an **absolute path** (`:637`) |
| `run_qemu_process` (`:580`) | `Qemu` | `qemu-x86_64` \| `qemu-aarch64` \| `qemu-arm` (`:638-640`) |
| `run_git_bounded` (`:512`) | `Git` | exactly `git` (`:641`) |
| `run_interpreter_bounded` (`:485`) | `Interpreter` | exactly `python3` \| `python` (`:642`) |

The `Interpreter` entry point is the consumer-validator case: it takes an
`InterpreterCommandSpec` (`:203-227`) whose argument vector is OS-native, so a validated
absolute path reaches the interpreter byte-exact, and it observes a caller-owned atomic
cancellation flag (`Cancellation::Flag`, `:143-146`) on the same cadence as the deadline.

This is the ADR-0004 adapter restriction enforced at the exec boundary: not a command DSL,
and callers cannot widen the set at runtime. `test :1278-1284` pins it by rewriting the
executable to `sh` and asserting rejection.

**Bound validation** (`:644-653`) rejects the whole call unless the executable is allowed,
`timeout` is non-zero and ≤86 400 s (`:645-646`), and both limits are non-zero and ≤ the
runner ceiling `MAX_PROCESS_OUTPUT` (`:647-650`, `:27`). The builder commands keep their own
tighter default of `MAX_OUTPUT` (`:20`). **Process-group control** uses
`command_group::CommandGroup` (`:4`):
`group_spawn()` (`:731`) places the child in its own group/job, and `kill()` + `wait()`
(`:754-759`, `:763-768`) terminate the **group**, so build scripts and descendants die with
the driver; termination failure is an error, not a swallowed outcome. The **poll loop**
(`:752-791`, 20 ms granularity at `:790`) tests cancellation **first** (`:753-761` →
`Cancelled`), then the deadline (`:762-770` → `TimedOut`), then the **output bound**
(`:771-782` → `OutputLimitExceeded`), then child exit (`:783-789`), so cancellation wins
over a concurrent timeout.

**Prompt output-limit detection.** `stream_over_limit` (`:876-878`) reads the same flag the
drain thread sets, so an over-limit stream is noticed at the moment of excess and the whole
group is terminated then (`:774-782`) instead of at the deadline. This was the CI validator's
own behaviour and it is now the shared runner's. The observable outcome is unchanged
(`OutputLimitExceeded`); only the moment of termination moves earlier.

**Bounded capture**: stdout and stderr are taken from the group child (`:669-670`) and
drained on two threads by `drain` (`:747-750`, `:880-896`), which retains at most `limit` bytes and
raises an overflow flag on truncation (`:886-891`). Reader threads are joined under a bounded
budget (`:803-804`, `:856-879`), so a descendant that inherited a pipe cannot park the run
past the caller's deadline; a reader still parked at its budget is detached and the run is
failed closed — and the group is then terminated (`:805-811`), so the descendant that held
the pipe does not outlive the run either.

**Contents never returned.** `ProcessEvidence` (`:256-263`) carries only `outcome`,
`stdout_bytes`, `stderr_bytes` — counts, never bytes. The captured buffers are local to
`run_bounded` and dropped at return (`:816-823`), which is what lets
`expected_stdout` be checked internally (`:824-829`) without becoming a side channel, so a
failure is diagnosable only by outcome and counts.

**Outcome classification** (`:830-842`), in precedence order: `TimedOut` → `Cancelled` →
`OutputLimitExceeded` (over-limit, or a reader still parked at its budget) →
`Failed(-1)` when an `expected_stdout` substring is absent
(`:836-837`) → `Success` → `Failed(status.code().unwrap_or(-1))`. A signal-terminated child
also lands on `-1`, so a missing substring is indistinguishable from signal death. The
timeout/cancel path re-reads status after `wait()` and fails closed if unavailable
(`process cleanup incomplete`, `:812-815`). `BuildCancellation` (`:239`) is a `Clone` handle
over `Arc<AtomicBool>` whose `cancel()` is `SeqCst` and idempotent (`:240-255`).

## Environment handling and Windows requirements

`env_clear()` (`:667`) is unconditional: the child starts from an empty environment,
re-populated only from a fixed allowlist. `stdin` is `Stdio::null()` (`:668`), so a child
can never block on input or prompt. The `Candidate` and `Qemu` allowances get `PATH`,
`SystemRoot`, `WINDIR`, `TEMP`, `TMP` (`:686`); `Builder` gets those five plus `HOME`,
`USERPROFILE`, `CARGO_HOME`, `RUSTUP_HOME`, `LIB`, `LIBPATH`, `INCLUDE`, `VCToolsInstallDir`,
`VisualStudioVersion`, `VCINSTALLDIR`, `WindowsSdkDir`, `WindowsSDKVersion`,
`UniversalCRTSdkDir`, and `UCRTVersion` (`:688-708`). Each key is copied only if present in
the parent (`:711-715`); the rustup and MSVC homes are needed because `cargo`/`rustup`
resolve toolchains and the linker through them. `spec.env` is applied last (`:729`), so the
module's own override — in practice only `CARGO_TARGET_DIR` — wins.

**Windows Cargo builds require an already-initialized MSVC environment.** This module
neither provisions nor validates one; it only preserves one. On `#[cfg(windows)]`, when
`VCToolsInstallDir` is set, `$VCToolsInstallDir/bin/Hostx64/x64` is prepended to `PATH`
(`:716-728`) so `link.exe` resolves from the active VCTools instance. If the environment was
never initialized, those keys are simply absent, `cargo` cannot find a linker, and the failure
surfaces as a bounded non-`Success` `ProcessEvidence` (`Failed(code)`) or, if the toolchain
itself is unreachable, as `configured Rust toolchain preflight failed` (`:921-923`) — never as
captured linker diagnostics, since output contents are not returned. The obligation is met
outside this crate: CI uses `ilammy/msvc-dev-cmd@v1` and asserts `link.exe` resolves under
`VCToolsInstallDir` (`.github/workflows/ci.yml:52-64`). On naming, `discover_candidate`
appends `.exe` when the triple contains the `-windows-` component (`:1182-1186`).
The match is anchored on that component rather than a bare `windows` substring,
so a hypothetical `x86_64-unknown-notwindowsish` triple does not pick up a
Windows suffix; and the suffix is appended to the whole binary name via
`set_file_name`, so a dotted name such as `tool.cli` resolves to `tool.cli.exe`
as Cargo produces it, not to the truncating `tool.exe` that `set_extension`
would have yielded. `valid_identifier` (`:383-388`) admits `.`, so this case is
reachable rather than hypothetical.

## Candidate discovery and private target directories

`private_target_dir` (`:1118-1159`) establishes per-invocation storage isolation:
`work_root` must be absolute and must pre-exist as a real directory, and a symlink work
root is rejected (`:1123-1136`). The name is `{safe(invocation)}-{safe(target)}` (`:1139`),
where `safe_component` (`:1160-1170`) maps every character outside `[A-Za-z0-9_-]` to `_`, so
no caller string can traverse or inject a path; `invocation` is bounded to ≤128 bytes
(`:1124-1125`). `create_dir` makes an existing path a hard error, so invocations are private
rather than reused (`:1141-1147`); the created path is then re-canonicalized and must stay
under the canonicalized work root, which is defence against a pre-existing symlink inside
the root (`:1148-1152`). A `.eggpack-owner` marker records `1\n<invocation>\n<target>\n`
(`:1153-1157`).

The function is **not** pure: it mutates the filesystem. It returns `<private>/target` (`:1158`),
the directory handed to Cargo as `CARGO_TARGET_DIR`; the marker lives in the parent, not in
the returned path, which `execute_target_cancellable` then creates explicitly (`:1043-1045`)
and passes to `cargo_command` (`:1053-1059`). Isolation provided: no ambient or shared Cargo
target directory is ever written, so builds cannot contaminate one another or the caller's
tree.

`discover_candidate` (`:1173-1214`) resolves exactly one path,
`<target_dir>/<triple>/release/<binary>[.exe]` (`:1181-1186`) and validates before trusting
it: `binary` must be a valid identifier (`:1178-1180`); the entry must exist as a **non-empty
regular file, not a symlink** (`symlink_metadata` plus type and length checks, `:1187-1191`);
the canonicalized file must be contained in the canonicalized target directory (`:1192-1198`),
since the symlink check alone would not stop a hardlink or a raced path; and both parents
(`<triple>` and `release`) must be real directories, not symlinks (`:1199-1212`).

It returns `(path, metadata.len())` (`:1213`); nothing is read or hashed here, so the `size` on
`CandidateArtifact` (`:279`) is metadata only. A `CandidateArtifact` (`:267-280`) is bytes
plus identity (target, slot, package, binary); the build stage never installs, archives,
publishes, or authenticates.

## Key types / functions (with `file:line`)

Paths are `crates/eggpack-core/src/builder.rs` unless noted.

| Line | Item | Note |
| --- | --- | --- |
| `:20-28` | `MAX_OUTPUT`, `MAX_PROCESS_OUTPUT` | `256 * 1024` builder default; `8 MiB` runner ceiling |
| `:51` | `ensure_candidate_executable` | private; unix-only; `mode \|= 0o111` on a non-symlink regular file (mode is not part of recorded size/digest) |
| `:76` `:94` `:106` | `LogicalOutputSelector`, `BuildBinding`, `BuildBindingsV1` | slot / slot+package+bin / strict version-1 document |
| `:115` `:122` | `BuildError`, `build_err` | `Display` + `Error`; fixed internal message only, never process text |
| `:127` `:160` `:186` | `ExecutableAllowance`, `CommandSpec`, `BoundCommand` | `ExecutableAllowance` private; **`BoundCommand` is declared public but never constructed anywhere in the workspace** |
| `:224` `:239` `:256` | `CommandOutcome`, `BuildCancellation`, `ProcessEvidence` | `BuildCancellation` = `Clone` `Arc<AtomicBool>`, `new` `:242` / `cancel` `:246` / `is_cancelled` `:249` |
| `:267` `:284` | `CandidateArtifact`, `BuildAttempt` | the two public result types |
| `:303` `:309` `:341` `:383` | `from_toml`, `validate_shape`, `validate_for`, `valid_identifier` | first two private; `[A-Za-z0-9_.-]`, ≤128 bytes |
| `:391` `:899` | `cargo_command`, `preflight` | `preflight` private |
| `:461` `:485` `:569` `:580` `:608` `:628` `:880` | `run_bounded_cancellable`, `run_interpreter_bounded`, `run_qualification_process`, `run_qemu_process`, `run_bounded_inner`, `run_bounded`, `drain` | the middle two are `pub(crate)`; `run_bounded_inner` and `run_bounded` are private and hold all enforcement |
| `:203` `:127` `:143` | `InterpreterCommandSpec`, `ExecutableAllowance`, `Cancellation` | the interpreter spec carries OS-native arguments |
| `:999` `:1019` `:1118` `:1160` `:1173` | `execute_target`, `execute_target_cancellable`, `private_target_dir`, `safe_component`, `discover_candidate` | `safe_component` private |

**Tests (`:1217-1902`):** explicit non-shell command and `sh` rejection (`:1247-1284`); zigbuild
glibc rewrite and macOS-floor rejection (`:1286-1327`) plus the byte-unchanged regression pin for
the triple rewrite and the macOS-under-zigbuild refusal (`:1329-1393`); binding duplicate and
unknown-field rejection (`:1395-1408`); exact direct-slot coverage (`:1410-1439`); candidate
regularity and non-emptiness (`:1441-1456`); relative and reused work roots rejected
(`:1585-1591`); bounded output, nonzero exit, and overflow classified `OutputLimitExceeded`
(`:1593-1660`); timeout and cancellation each killing and waiting for a real Cargo build
script over three iterations (`:1662-1790`), plus the parked-descendant and parked-reader
cases (`:1792-1848`); a real local Cargo fixture producing a direct candidate
(`:1847-1902`).

## Boundaries / non-goals

- **Does not qualify** (no format, architecture, or runtime inspection — build success is
  not qualification; `qualification.rs` owns that) and **does not finalize** (no
  contract-named copying, archiving, manifest aggregation, or `release-manifest.json`).
- **Does not touch the network directly** — but it also does not forbid the child from
  reaching it. `cargo_command` passes `--locked`, not `--offline` (`:405-413`); the in-file
  fixture test adds `--offline` explicitly (`:1683`), which production does not.
- **No shell and no generic command DSL.** Executable and argv are separate OS arguments,
  and the executable set is a closed per-allowance enumeration checked at spawn
  (`:633-653`), per ADR-0004. A `CommandSpec` is a description, not an interpreter; there
  is no plugin or scripting seam.
- **No binding-coverage validation inside the build path.** `validate_for` (`:341`) is
  caller-invoked; `execute_target_cancellable` validates only plan membership,
  `schema_version == 1`, and non-empty `release_id` / `source_revision` (`:1028-1036`). An
  empty `bindings` slice therefore yields `Ok(BuildAttempt)` whose `process.outcome` is the
  `Success` initializer (`:1047-1051`) with zero candidates.
- **No output evidence** (failures are not diagnosable from captured text, by design), no
  cross-tool provisioning (`preflight` requires declared versions to be present; it does
  not install them), and no archive encoding, extraction, publication, signing, or install.

## Dependencies / dependents

- Own dependencies used here: `command-group 5.0.1` (`Cargo.toml:21`, process groups), `toml
  0.8` (`Cargo.toml:20`, `from_toml` at `builder.rs:303`), `serde` derive (`Cargo.toml:17`),
  `eggpack-contract` (`Cargo.toml:15`, `DistributionContract` + `ExpandedAssets` at
  `builder.rs:5`), and `std` process/thread/IO primitives. `sha2` is a **crate-level**
  dependency but is not used in this file (it is used by the digest authority in
  `src/digest.rs`).
- In-crate dependents: `qualification.rs:3-11` imports `run_qualification_process`,
  `run_qemu_process`, `CommandSpec`, `CandidateArtifact`, `BuildAttempt`, `BuildBindingsV1`,
  and `BuildCancellation`; `finalization.rs:4` imports `BuildAttempt`, `CandidateArtifact`,
  and `LogicalOutputSelector`.
- External dependents: `eggpack-ci` calls `cargo_command` to render the build command into
  generated workflows and its drift check (`crates/eggpack-ci/src/planning.rs:146`,
  `crates/eggpack-ci/src/drift.rs:68`), and
  reconstructs `BuildAttempt` / `CandidateArtifact` in memory from CI handoff data rather
  than executing a build (`crates/eggpack-ci/src/handoff.rs:186`). `eggpack-github` and
  `eggpack-cli` depend on `eggpack-core` transitively.

## Sibling deep dives

- [Planning stage](core-planning.md) — `PackConfig` → `ReleasePlan`; the `PlannedTarget` and
  `TargetPolicy` this module reads.
- [Qualification stage](core-qualification.md) — consumer of `BuildAttempt`; owns
  `run_qualification_process` / `run_qemu_process` dispatch and candidate inspection.
- [Finalization stage](core-finalization.md) — consumes `BuildAttempt` to copy, archive, and
  aggregate the release manifest.
- [`eggpack-core` overview](core.md) and [workspace overview](overview.md).
- [Process execution (cross-cutting)](process-execution.md) — the exec, timeout, cancellation,
  and environment rules here are one instance of the crate-wide contract.
