# `eggpack-core` — Build stage (`src/builder.rs`) — Deep Dive

Explicit Cargo build bindings, the two first-party Cargo command adapters, bounded
cancellable process execution, private per-invocation target storage, and candidate byte
discovery. One of four `eggpack-core` stage deep dives (planning, build, qualification,
finalization); the stage between `ReleasePlan` (intent) and `QualificationEvidence`
(proof). Single file `crates/eggpack-core/src/builder.rs` (1301 lines,
`#![forbid(unsafe_code)]` inherited from the workspace), re-exported wholesale at crate
root (`crates/eggpack-core/src/lib.rs:8`).

## Inputs and outputs

- in: `&ReleasePlan`, `&PlannedTarget` — from `PackConfig::resolve`
  (`crates/eggpack-core/src/lib.rs:456`, `:468`).
- in: `&[BuildBinding]` — the caller's per-target slice, **not** coverage-validated here.
- in: `&DistributionContract` — only via `BuildBindingsV1::validate_for` (`:261`).
- in: `repository_root`, `work_root`, `invocation`, `&BuildCancellation`.
- out: `BuildAttempt` (`:202`) — identity, bounded `ProcessEvidence`, `Vec<CandidateArtifact>`.
- out: `ProcessEvidence` (`:174`) — outcome plus retained byte **counts** only.

`BuildAttempt` is identity-bound to `release_id` + `source_revision` (`:204`, `:206`) and to
the one `target` / `strategy` it ran (`:208`, `:210`). It records what was attempted, not
what was proved: this module never inspects a candidate's format, architecture, or runtime
behaviour, so `Success` means a command exited zero — never that the candidate qualifies.

## The binding model

A contract output slot is not a filesystem path, so each slot is bound by hand to a Cargo
package and binary target.

- `LogicalOutputSelector` (`:49`), tagged and `deny_unknown_fields` (`:48`): `Direct`
  (`:51`); `BundleEntry{index}` indexed by **contract order** (`:53-56`);
  `ArchiveMember{source}` keyed by the **exact contract source string** (`:58-61`).
- `BuildBinding{selector, package, binary}` (`:67-74`) pairs a slot with one Cargo
  `--package` / `--bin` pair.
- `BuildBindingsV1{schema_version, targets: BTreeMap<String, Vec<BuildBinding>>}`
  (`:79-84`), strict version 1. `from_toml` (`:221`) parses via `toml::from_str` then runs
  `validate_shape` (`:227`).

`validate_shape` (`:227-257`) is the fail-closed bound layer: `schema_version == 1` and
1..=256 targets (`:228`); per target a non-empty key and 1..=256 bindings (`:232`); unique
selectors plus `valid_identifier` package/binary (`:237-242`), where
`valid_identifier` (`:301-306`) allows only `[A-Za-z0-9_.-]`, ≤128 bytes.
`ArchiveMember.source` must be a non-empty ≤255-byte relative path with no leading `/`
and no empty/`.`/`..` component (`:243-253`).

**Exact plan coverage** is `validate_for` (`:259-299`). For every `plan.targets` entry it
looks up `self.targets[&target.target]` (`:266-269`, else `missing target bindings`),
expands the contract (`:270-272`), and derives the expected slot set from the expanded
`ExpandedAssets` variant — `Direct` ⇒ `{Direct}`; `Bundle` ⇒ `BundleEntry{0..entries.len()}`;
`Archive` ⇒ `ArchiveMember{source}` per member (`:273-283`). The binding selectors must
then be **equal as sets** (`:284-289`): equality in both directions means neither a
missing nor an extra slot is accepted, reported as the single `bindings do not exactly
cover contract Cargo slots` (`:286-288`). Any binding target key absent from the plan is
rejected as `bindings include target outside ReleasePlan` (`:291-297`). Contract expansion
is the authority: a `BuildBinding` cannot redefine layout, only map an already-contractual
slot onto a Cargo target.

## Command construction by build strategy

`cargo_command` (`:309-366`) is the only builder and is total over the two
`BuildStrategy` variants. It re-validates both identifiers (`:316-318`), so it is safe to
call with a hand-built binding.

| Aspect | `NativeCargo` (`:321-328`) | `CargoZigbuild` (`:329-336`) |
| --- | --- | --- |
| subcommand | `build` | `zigbuild` |
| shared args | `+<toolchain.rust>`, `--release`, `--locked`, `--target <triple>` | identical |
| glibc floor | not encoded | `args[5]` rewritten to `<triple>.<major>.<minor>` (`:338-340`) |
| macOS floor | n/a | rejected: `macOS floor cannot use cargo-zigbuild` (`:341-343`) |

Both append exactly `--package <package> --bin <binary>` (`:345-350`). The executable is
always the bare tool name `cargo` (`:357`); executable and argv are separate OS arguments, so
no shell is involved (`:106`). One environment variable is injected, `CARGO_TARGET_DIR` =
the private target directory (`:351-355`); `cwd` is the caller's (`:359`), both stream limits
are `MAX_OUTPUT` (256 KiB, `:18`, `:362-363`), and `expected_stdout` is `None` (`:364`).
`execute_target_cancellable` supplies a fixed 1800 s build timeout (`:736`).

`preflight` (`:584-674`) runs before any build: `rustc +<toolchain> --version` (`:589`) and
`cargo +<toolchain> --version` (`:608`), each 10 s / 1024 bytes, failing closed on any
non-`Success` outcome (`:605-607`, `:624-626`). Under `CargoZigbuild` it also requires both
cross-tool versions to be declared (`:628-639`) and substring-checks `cargo zigbuild
--version` against `cargo-zigbuild <expected>` (`:649`) and `zig version` against the
expected Zig version (`:665`); mismatch yields `cargo-zigbuild version mismatch` / `Zig
version mismatch` (`:654`, `:670`). The expected substring is compared internally and never
surfaced (`:124`).

## Bounded cancellable process execution

All execution funnels through the private `run_bounded_inner` (`:391-563`). Three entry
points select an `ExecutableAllowance` (`:100-104`):

| Entry | Allowance | Executable rule |
| --- | --- | --- |
| `run_bounded_cancellable` (`:370`) | `Builder` | `cargo` \| `rustc` \| `zig` (`:397-399`) |
| `run_qualification_process` (`:377`) | `Candidate` | must be an **absolute path** (`:400`) |
| `run_qemu_process` (`:384`) | `Qemu` | `qemu-x86_64` \| `qemu-aarch64` \| `qemu-arm` (`:401-404`) |

This is the ADR-0004 adapter restriction enforced at the exec boundary: not a command DSL,
and callers cannot widen the set at runtime. `test :947-949` pins it by rewriting the
executable to `sh` and asserting rejection.

**Bound validation** (`:406-415`) rejects the whole call unless the executable is allowed,
`timeout` is non-zero and ≤86 400 s (`:407-408`), and both limits are non-zero and ≤`MAX_OUTPUT`
(`:409-412`). **Process-group control** uses `command_group::CommandGroup` (`:4`):
`group_spawn()` (`:479`) places the child in its own group/job, and `kill()` + `wait()`
(`:502-507`, `:511-516`) terminate the **group**, so build scripts and descendants die with
the driver; termination failure is an error, not a swallowed outcome. The **poll loop**
(`:499-527`, 20 ms granularity at `:526`) tests cancellation **first** (`:501-509` →
`Cancelled`), then the deadline (`:510-518` → `TimedOut`), then child exit (`:519-525`), so
cancellation wins over a concurrent timeout.

**Bounded capture**: stdout and stderr are taken from the group child (`:481-490`) and
drained on two threads by `drain` (`:565-581`), which retains at most `limit` bytes and
raises an overflow flag on truncation (`:572-576`). Reader threads are always joined
(`:528-529`), so nothing is lost before the pipes are read.

**Contents never returned.** `ProcessEvidence` (`:174-181`) carries only `outcome`,
`stdout_bytes`, `stderr_bytes` — counts, never bytes. The captured buffers are local to
`run_bounded_inner` and dropped at return (`:534-541`, `:559-563`), which is what lets
`expected_stdout` be checked internally (`:542-545`) without becoming a side channel, so a
failure is diagnosable only by outcome and counts.

**Outcome classification** (`:546-558`), in precedence order: `TimedOut` → `Cancelled` →
`OutputLimitExceeded` → `Failed(-1)` when an `expected_stdout` substring is absent
(`:552-553`) → `Success` → `Failed(status.code().unwrap_or(-1))`. A signal-terminated child
also lands on `-1`, so a missing substring is indistinguishable from signal death. The
timeout/cancel path re-reads status after `wait()` and fails closed if unavailable
(`process cleanup incomplete`, `:530-533`). `BuildCancellation` (`:157`) is a `Clone` handle
over `Arc<AtomicBool>` whose `cancel()` is `SeqCst` and idempotent (`:164-169`).

## Environment handling and Windows requirements

`env_clear()` (`:429`) is unconditional: the child starts from an empty environment,
re-populated only from a fixed allowlist. `stdin` is `Stdio::null()` (`:430`), so a child
can never block on input or prompt. The `Candidate` and `Qemu` allowances get `PATH`,
`SystemRoot`, `WINDIR`, `TEMP`, `TMP` (`:434`); `Builder` gets those five plus `HOME`,
`USERPROFILE`, `CARGO_HOME`, `RUSTUP_HOME`, `LIB`, `LIBPATH`, `INCLUDE`, `VCToolsInstallDir`,
`VisualStudioVersion`, `VCINSTALLDIR`, `WindowsSdkDir`, `WindowsSDKVersion`,
`UniversalCRTSdkDir`, and `UCRTVersion` (`:436-456`). Each key is copied only if present in
the parent (`:459-463`); the rustup and MSVC homes are needed because `cargo`/`rustup`
resolve toolchains and the linker through them. `spec.env` is applied last (`:477`), so the
module's own override — in practice only `CARGO_TARGET_DIR` — wins.

**Windows Cargo builds require an already-initialized MSVC environment.** This module
neither provisions nor validates one; it only preserves one. On `#[cfg(windows)]`, when
`VCToolsInstallDir` is set, `$VCToolsInstallDir/bin/Hostx64/x64` is prepended to `PATH`
(`:464-476`) so `link.exe` resolves from the active VCTools instance. If the environment was
never initialized, those keys are simply absent, `cargo` cannot find a linker, and the failure
surfaces as a bounded non-`Success` `ProcessEvidence` (`Failed(code)`) or, if the toolchain
itself is unreachable, as `configured Rust toolchain preflight failed` (`:606`) — never as
captured linker diagnostics, since output contents are not returned. The obligation is met
outside this crate: CI uses `ilammy/msvc-dev-cmd@v1` and asserts `link.exe` resolves under
`VCToolsInstallDir` (`.github/workflows/ci.yml:44-56`). On naming, `discover_candidate`
appends `.exe` when the triple contains the `-windows-` component (`:916-919`).
The match is anchored on that component rather than a bare `windows` substring,
so a hypothetical `x86_64-unknown-notwindowsish` triple does not pick up a
Windows suffix; and the suffix is appended to the whole binary name via
`set_file_name`, so a dotted name such as `tool.cli` resolves to `tool.cli.exe`
as Cargo produces it, not to the truncating `tool.exe` that `set_extension`
would have yielded. `valid_identifier` (`:311-316`) admits `.`, so this case is
reachable rather than hypothetical.

## Candidate discovery and private target directories

`private_target_dir` (`:790-831`) establishes per-invocation storage isolation:
`work_root` must be absolute and must pre-exist as a real directory, and a symlink work
root is rejected (`:795-808`). The name is `{safe(invocation)}-{safe(target)}` (`:811`),
where `safe_component` (`:832-842`) maps every character outside `[A-Za-z0-9_-]` to `_`, so
no caller string can traverse or inject a path; `invocation` is bounded to ≤128 bytes
(`:797`). `create_dir` makes an existing path a hard error, so invocations are private
rather than reused (`:813-818`); the created path is then re-canonicalized and must stay
under the canonicalized work root, which is defence against a pre-existing symlink inside
the root (`:820-824`). A `.eggpack-owner` marker records `1\n<invocation>\n<target>\n`
(`:825-829`).

The function is **not** pure: it mutates the filesystem. It returns `<private>/target` (`:830`),
the directory handed to Cargo as `CARGO_TARGET_DIR`; the marker lives in the parent, not in
the returned path, which `execute_target_cancellable` then creates explicitly (`:722-723`)
and passes to `cargo_command` (`:735`). Isolation provided: no ambient or shared Cargo
target directory is ever written, so builds cannot contaminate one another or the caller's
tree.

`discover_candidate` (`:845-884`) resolves exactly one path,
`<target_dir>/<triple>/release/<binary>[.exe]` (`:853-856`) and validates before trusting
it: `binary` must be a valid identifier (`:850-852`); the entry must exist as a **non-empty
regular file, not a symlink** (`symlink_metadata` plus type and length checks, `:857-861`);
the canonicalized file must be contained in the canonicalized target directory (`:862-868`),
since the symlink check alone would not stop a hardlink or a raced path; and both parents
(`<triple>` and `release`) must be real directories, not symlinks (`:869-882`).

It returns `(path, metadata.len())` (`:883`); nothing is read or hashed here, so the `size` on
`CandidateArtifact` (`:197`) is metadata only. A `CandidateArtifact` (`:185-198`) is bytes
plus identity (target, slot, package, binary); the build stage never installs, archives,
publishes, or authenticates.

## Key types / functions (with `file:line`)

Paths are `crates/eggpack-core/src/builder.rs` unless noted.

| Line | Item | Note |
| --- | --- | --- |
| `:18` | `MAX_OUTPUT` | `256 * 1024`; ceiling on each captured stream |
| `:24` | `ensure_candidate_executable` | private; unix-only; `mode \|= 0o111` on a non-symlink regular file (mode is not part of recorded size/digest) |
| `:49` `:67` `:79` | `LogicalOutputSelector`, `BuildBinding`, `BuildBindingsV1` | slot / slot+package+bin / strict version-1 document |
| `:88` `:95` | `BuildError`, `build_err` | `Display` + `Error`; fixed internal message only, never process text |
| `:100` `:108` `:129` | `ExecutableAllowance`, `CommandSpec`, `BoundCommand` | `ExecutableAllowance` private; **`BoundCommand` is declared public but never constructed anywhere in the workspace** |
| `:142` `:157` `:174` | `CommandOutcome`, `BuildCancellation`, `ProcessEvidence` | `BuildCancellation` = `Clone` `Arc<AtomicBool>`, `new` `:160` / `cancel` `:164` / `is_cancelled` `:167` |
| `:185` `:202` | `CandidateArtifact`, `BuildAttempt` | the two public result types |
| `:221` `:227` `:259` `:301` | `from_toml`, `validate_shape`, `validate_for`, `valid_identifier` | first two private; `[A-Za-z0-9_.-]`, ≤128 bytes |
| `:309` `:584` | `cargo_command`, `preflight` | `preflight` private |
| `:370` `:377` `:384` `:391` `:565` | `run_bounded_cancellable`, `run_qualification_process`, `run_qemu_process`, `run_bounded_inner`, `drain` | the middle two are `pub(crate)`; `run_bounded_inner` private and holds all enforcement |
| `:677` `:697` `:790` `:832` `:845` | `execute_target`, `execute_target_cancellable`, `private_target_dir`, `safe_component`, `discover_candidate` | `safe_component` private |

**Tests (`:886-1301`):** explicit non-shell command and `sh` rejection (`:916-950`); zigbuild
glibc rewrite and macOS-floor rejection (`:952-988`); binding duplicate and unknown-field
rejection (`:990-1003`); exact direct-slot coverage (`:1005-1034`); candidate regularity and
non-emptiness (`:1036-1051`); relative and reused work roots rejected (`:1053-1059`); bounded
output, nonzero exit, and overflow classified `OutputLimitExceeded` (`:1061-1124`); timeout
and cancellation each killing and waiting for a real Cargo build script over three
iterations (`:1126-1244`); a real local Cargo fixture producing a direct candidate
(`:1246-1300`).

## Boundaries / non-goals

- **Does not qualify** (no format, architecture, or runtime inspection — build success is
  not qualification; `qualification.rs` owns that) and **does not finalize** (no
  contract-named copying, archiving, manifest aggregation, or `release-manifest.json`).
- **Does not touch the network directly** — but it also does not forbid the child from
  reaching it. `cargo_command` passes `--locked`, not `--offline` (`:322-336`); the in-file
  fixture test adds `--offline` explicitly (`:1147`), which production does not.
- **No shell and no generic command DSL.** Executable and argv are separate OS arguments,
  and the executable set is a closed per-allowance enumeration checked at spawn
  (`:396-415`), per ADR-0004. A `CommandSpec` is a description, not an interpreter; there
  is no plugin or scripting seam.
- **No binding-coverage validation inside the build path.** `validate_for` (`:259`) is
  caller-invoked; `execute_target_cancellable` validates only plan membership,
  `schema_version == 1`, and non-empty `release_id` / `source_revision` (`:706-714`). An
  empty `bindings` slice therefore yields `Ok(BuildAttempt)` whose `process.outcome` is the
  `Success` initializer (`:725-729`) with zero candidates.
- **No output evidence** (failures are not diagnosable from captured text, by design), no
  cross-tool provisioning (`preflight` requires declared versions to be present; it does
  not install them), and no archive encoding, extraction, publication, signing, or install.

## Dependencies / dependents

- Own dependencies used here: `command-group 5.0.1` (`Cargo.toml:21`, process groups), `toml
  0.8` (`:20`, `from_toml` at `builder.rs233`), `serde` derive (`:17`), `eggpack-contract`
  (`:15`, `DistributionContract` + `ExpandedAssets` at `builder.rs:5`), and `std`
  process/thread/IO primitives. `sha2` is a **crate-level** dependency but is not used in
  this file (it is used in `lib.rs`, `qualification.rs`, and `finalization.rs`).
- In-crate dependents: `qualification.rs:4-8` imports `run_qualification_process`,
  `run_qemu_process`, `CommandSpec`, `CandidateArtifact`, `BuildAttempt`, `BuildBindingsV1`,
  and `BuildCancellation`; `finalization.rs:4` imports `BuildAttempt`, `CandidateArtifact`,
  and `LogicalOutputSelector`.
- External dependents: `eggpack-ci` calls `cargo_command` to render the build command into
  generated workflows and its drift check (`eggpack-ci/src/lib.rs:164`, `:348`), and
  reconstructs `BuildAttempt` / `CandidateArtifact` in memory from CI handoff data rather
  than executing a build (`eggpack-ci/src/lib.rs:2010`, `:2034`). `eggpack-github` and
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
