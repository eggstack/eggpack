# `eggpack-core` — Build stage (`src/builder.rs`) — Deep Dive

Explicit Cargo build bindings, the two first-party Cargo command adapters, bounded
cancellable process execution, private per-invocation target storage, and candidate byte
discovery. One of four `eggpack-core` stage deep dives (planning, build, qualification,
finalization); the stage between `ReleasePlan` (intent) and `QualificationEvidence`
(proof). Single file `crates/eggpack-core/src/builder.rs` (1734 lines,
`#![forbid(unsafe_code)]` inherited from the workspace), re-exported wholesale at crate
root (`crates/eggpack-core/src/lib.rs:10`).

## Inputs and outputs

- in: `&ReleasePlan`, `&PlannedTarget` — from `PackConfig::resolve`
  (`crates/eggpack-core/src/lib.rs:709`, `:618`).
- in: `&[BuildBinding]` — the caller's per-target slice, **not** coverage-validated here.
- in: `&DistributionContract` — only via `BuildBindingsV1::validate_for` (`:284`).
- in: `repository_root`, `work_root`, `invocation`, `&BuildCancellation`.
- out: `BuildAttempt` (`:227`) — identity, bounded `ProcessEvidence`, `Vec<CandidateArtifact>`.
- out: `ProcessEvidence` (`:199`) — outcome plus retained byte **counts** only.

`BuildAttempt` is identity-bound to `release_id` + `source_revision` (`:229`, `:231`) and to
the one `target` / `strategy` it ran (`:233`, `:235`). It records what was attempted, not
what was proved: this module never inspects a candidate's format, architecture, or runtime
behaviour, so `Success` means a command exited zero — never that the candidate qualifies.

## The binding model

A contract output slot is not a filesystem path, so each slot is bound by hand to a Cargo
package and binary target.

- `LogicalOutputSelector` (`:68`), tagged and `deny_unknown_fields` (`:67`): `Direct`
  (`:70`); `BundleEntry{index}` indexed by **contract order** (`:71-75`);
  `ArchiveMember{source}` keyed by the **exact contract source string** (`:76-80`).
- `BuildBinding{selector, package, binary}` (`:86-93`) pairs a slot with one Cargo
  `--package` / `--bin` pair.
- `BuildBindingsV1{schema_version, targets: BTreeMap<String, Vec<BuildBinding>>}`
  (`:98-103`), strict version 1. `from_toml` (`:246-251`) parses via `toml::from_str` then runs
  `validate_shape` (`:252-282`).

`validate_shape` (`:252-282`) is the fail-closed bound layer: `schema_version == 1` and
1..=256 targets (`:253-255`); per target a non-empty key and 1..=256 bindings (`:256-259`); unique
selectors plus `valid_identifier` package/binary (`:260-267`), where
`valid_identifier` (`:326-331`) allows only `[A-Za-z0-9_.-]`, ≤128 bytes.
`ArchiveMember.source` must be a non-empty ≤255-byte relative path with no leading `/`
and no empty/`.`/`..` component (`:268-278`).

**Exact plan coverage** is `validate_for` (`:284-324`). For every `plan.targets` entry it
looks up `self.targets[&target.target]` (`:291-294`, else `missing target bindings`),
expands the contract (`:295-297`), and derives the expected slot set from the expanded
`ExpandedAssets` variant — `Direct` ⇒ `{Direct}`; `Bundle` ⇒ `BundleEntry{0..entries.len()}`;
`Archive` ⇒ `ArchiveMember{source}` per member (`:298-308`). The binding selectors must
then be **equal as sets** (`:309-310`): equality in both directions means neither a
missing nor an extra slot is accepted, reported as the single `bindings do not exactly
cover contract Cargo slots` (`:311-314`). Any binding target key absent from the plan is
rejected as `bindings include target outside ReleasePlan` (`:316-322`). Contract expansion
is the authority: a `BuildBinding` cannot redefine layout, only map an already-contractual
slot onto a Cargo target.

## Command construction by build strategy

`cargo_command` (`:334-401`) is the only builder and is total over the two
`BuildStrategy` variants. It re-validates both identifiers (`:341-343`), so it is safe to
call with a hand-built binding.

| Aspect | `NativeCargo` (`:351`) | `CargoZigbuild` (`:352`) |
| --- | --- | --- |
| subcommand | `build` | `zigbuild` |
| shared args | `+<toolchain.rust>`, `--release`, `--locked`, `--target <triple>` | identical |
| glibc floor | not encoded | `args[5]` rewritten to `<triple>.<major>.<minor>` (`:358-361`) |
| macOS floor | n/a | rejected: `macOS floor cannot use cargo-zigbuild` (`:362-372`) |

Both append exactly `--package <package> --bin <binary>` (`:379-384`). The executable is
always the bare tool name `cargo` (`:391`); executable and argv are separate OS arguments, so
no shell is involved (`:126`). One environment variable is injected, `CARGO_TARGET_DIR` =
the private target directory (`:385-389`); `cwd` is the caller's (`:393`), both stream limits
are `MAX_OUTPUT` (256 KiB, `:18-19`, `:396-397`), and `expected_stdout` is `None` (`:398`).
`execute_target_cancellable` supplies a fixed 1800 s build timeout (`:891`).

`preflight` (`:732-831`) runs before any build: `rustc +<toolchain> --version` (`:739`) and
`cargo +<toolchain> --version` (`:759`), each 10 s / 1024 bytes, failing closed on any
non-`Success` outcome (`:754-756`, `:774-776`). Under `CargoZigbuild` it also requires both
cross-tool versions to be declared (`:777-789`) and requires each declared pin to be a complete
`MAJOR.MINOR.PATCH` version. `cargo zigbuild --version` and `zig version` are matched by
**trimmed equality** (`expected_stdout_exact`), not by substring, so a pin of `0.23.3` is not
satisfied by a reported `0.23.30`; mismatch yields `cargo-zigbuild version mismatch` / `Zig
version mismatch`. The expected value is compared internally and never
surfaced (`:143-149`).

## Bounded cancellable process execution

All execution funnels through the private `run_bounded_inner` (`:488-690`). Three entry
points select an `ExecutableAllowance` (`:119-124`):

| Entry | Allowance | Executable rule |
| --- | --- | --- |
| `run_bounded_cancellable` (`:405`) | `Builder` | `cargo` \| `rustc` \| `zig` (`:494-496`) |
| `run_qualification_process` (`:474`) | `Candidate` | must be an **absolute path** (`:497`) |
| `run_qemu_process` (`:481`) | `Qemu` | `qemu-x86_64` \| `qemu-aarch64` \| `qemu-arm` (`:498-501`) |

This is the ADR-0004 adapter restriction enforced at the exec boundary: not a command DSL,
and callers cannot widen the set at runtime. `test :1111-1113` pins it by rewriting the
executable to `sh` and asserting rejection.

**Bound validation** (`:504-513`) rejects the whole call unless the executable is allowed,
`timeout` is non-zero and ≤86 400 s (`:505-506`), and both limits are non-zero and ≤`MAX_OUTPUT`
(`:507-510`). **Process-group control** uses `command_group::CommandGroup` (`:4`):
`group_spawn()` (`:591`) places the child in its own group/job, and `kill()` + `wait()`
(`:614-619`, `:623-628`) terminate the **group**, so build scripts and descendants die with
the driver; termination failure is an error, not a swallowed outcome. The **poll loop**
(`:612-639`, 20 ms granularity at `:638`) tests cancellation **first** (`:613-621` →
`Cancelled`), then the deadline (`:622-630` → `TimedOut`), then child exit (`:631-637`), so
cancellation wins over a concurrent timeout.

**Bounded capture**: stdout and stderr are taken from the group child (`:529-530`) and
drained on two threads by `drain` (`:607-610`, `:713-729`), which retains at most `limit` bytes and
raises an overflow flag on truncation (`:719-724`). Reader threads are always joined
(`:651-652`, `:698-712`), so nothing is lost before the pipes are read.

**Contents never returned.** `ProcessEvidence` (`:199-206`) carries only `outcome`,
`stdout_bytes`, `stderr_bytes` — counts, never bytes. The captured buffers are local to
`run_bounded_inner` and dropped at return (`:657-664`), which is what lets
`expected_stdout` be checked internally (`:665-671`) without becoming a side channel, so a
failure is diagnosable only by outcome and counts.

**Outcome classification** (`:672-684`), in precedence order: `TimedOut` → `Cancelled` →
`OutputLimitExceeded` → `Failed(-1)` when an `expected_stdout` substring is absent
(`:678-679`) → `Success` → `Failed(status.code().unwrap_or(-1))`. A signal-terminated child
also lands on `-1`, so a missing substring is indistinguishable from signal death. The
timeout/cancel path re-reads status after `wait()` and fails closed if unavailable
(`process cleanup incomplete`, `:653-656`). `BuildCancellation` (`:182`) is a `Clone` handle
over `Arc<AtomicBool>` whose `cancel()` is `SeqCst` and idempotent (`:183-198`).

## Environment handling and Windows requirements

`env_clear()` (`:527`) is unconditional: the child starts from an empty environment,
re-populated only from a fixed allowlist. `stdin` is `Stdio::null()` (`:528`), so a child
can never block on input or prompt. The `Candidate` and `Qemu` allowances get `PATH`,
`SystemRoot`, `WINDIR`, `TEMP`, `TMP` (`:546`); `Builder` gets those five plus `HOME`,
`USERPROFILE`, `CARGO_HOME`, `RUSTUP_HOME`, `LIB`, `LIBPATH`, `INCLUDE`, `VCToolsInstallDir`,
`VisualStudioVersion`, `VCINSTALLDIR`, `WindowsSdkDir`, `WindowsSDKVersion`,
`UniversalCRTSdkDir`, and `UCRTVersion` (`:548-568`). Each key is copied only if present in
the parent (`:571-575`); the rustup and MSVC homes are needed because `cargo`/`rustup`
resolve toolchains and the linker through them. `spec.env` is applied last (`:589`), so the
module's own override — in practice only `CARGO_TARGET_DIR` — wins.

**Windows Cargo builds require an already-initialized MSVC environment.** This module
neither provisions nor validates one; it only preserves one. On `#[cfg(windows)]`, when
`VCToolsInstallDir` is set, `$VCToolsInstallDir/bin/Hostx64/x64` is prepended to `PATH`
(`:576-588`) so `link.exe` resolves from the active VCTools instance. If the environment was
never initialized, those keys are simply absent, `cargo` cannot find a linker, and the failure
surfaces as a bounded non-`Success` `ProcessEvidence` (`Failed(code)`) or, if the toolchain
itself is unreachable, as `configured Rust toolchain preflight failed` (`:754-756`) — never as
captured linker diagnostics, since output contents are not returned. The obligation is met
outside this crate: CI uses `ilammy/msvc-dev-cmd@v1` and asserts `link.exe` resolves under
`VCToolsInstallDir` (`.github/workflows/ci.yml:44-56`). On naming, `discover_candidate`
appends `.exe` when the triple contains the `-windows-` component (`:1015-1019`).
The match is anchored on that component rather than a bare `windows` substring,
so a hypothetical `x86_64-unknown-notwindowsish` triple does not pick up a
Windows suffix; and the suffix is appended to the whole binary name via
`set_file_name`, so a dotted name such as `tool.cli` resolves to `tool.cli.exe`
as Cargo produces it, not to the truncating `tool.exe` that `set_extension`
would have yielded. `valid_identifier` (`:326-331`) admits `.`, so this case is
reachable rather than hypothetical.

## Candidate discovery and private target directories

`private_target_dir` (`:951-992`) establishes per-invocation storage isolation:
`work_root` must be absolute and must pre-exist as a real directory, and a symlink work
root is rejected (`:956-969`). The name is `{safe(invocation)}-{safe(target)}` (`:972`),
where `safe_component` (`:993-1003`) maps every character outside `[A-Za-z0-9_-]` to `_`, so
no caller string can traverse or inject a path; `invocation` is bounded to ≤128 bytes
(`:957-958`). `create_dir` makes an existing path a hard error, so invocations are private
rather than reused (`:974-980`); the created path is then re-canonicalized and must stay
under the canonicalized work root, which is defence against a pre-existing symlink inside
the root (`:981-985`). A `.eggpack-owner` marker records `1\n<invocation>\n<target>\n`
(`:986-990`).

The function is **not** pure: it mutates the filesystem. It returns `<private>/target` (`:991`),
the directory handed to Cargo as `CARGO_TARGET_DIR`; the marker lives in the parent, not in
the returned path, which `execute_target_cancellable` then creates explicitly (`:876-878`)
and passes to `cargo_command` (`:886-892`). Isolation provided: no ambient or shared Cargo
target directory is ever written, so builds cannot contaminate one another or the caller's
tree.

`discover_candidate` (`:1006-1047`) resolves exactly one path,
`<target_dir>/<triple>/release/<binary>[.exe]` (`:1014-1019`) and validates before trusting
it: `binary` must be a valid identifier (`:1011-1013`); the entry must exist as a **non-empty
regular file, not a symlink** (`symlink_metadata` plus type and length checks, `:1020-1024`);
the canonicalized file must be contained in the canonicalized target directory (`:1025-1031`),
since the symlink check alone would not stop a hardlink or a raced path; and both parents
(`<triple>` and `release`) must be real directories, not symlinks (`:1032-1045`).

It returns `(path, metadata.len())` (`:1046`); nothing is read or hashed here, so the `size` on
`CandidateArtifact` (`:222`) is metadata only. A `CandidateArtifact` (`:210-223`) is bytes
plus identity (target, slot, package, binary); the build stage never installs, archives,
publishes, or authenticates.

## Key types / functions (with `file:line`)

Paths are `crates/eggpack-core/src/builder.rs` unless noted.

| Line | Item | Note |
| --- | --- | --- |
| `:18-19` | `MAX_OUTPUT` | `256 * 1024`; ceiling on each captured stream |
| `:43` | `ensure_candidate_executable` | private; unix-only; `mode \|= 0o111` on a non-symlink regular file (mode is not part of recorded size/digest) |
| `:68` `:86` `:98` | `LogicalOutputSelector`, `BuildBinding`, `BuildBindingsV1` | slot / slot+package+bin / strict version-1 document |
| `:107` `:114` | `BuildError`, `build_err` | `Display` + `Error`; fixed internal message only, never process text |
| `:119` `:128` `:154` | `ExecutableAllowance`, `CommandSpec`, `BoundCommand` | `ExecutableAllowance` private; **`BoundCommand` is declared public but never constructed anywhere in the workspace** |
| `:167` `:182` `:199` | `CommandOutcome`, `BuildCancellation`, `ProcessEvidence` | `BuildCancellation` = `Clone` `Arc<AtomicBool>`, `new` `:185` / `cancel` `:189` / `is_cancelled` `:192` |
| `:210` `:227` | `CandidateArtifact`, `BuildAttempt` | the two public result types |
| `:246` `:252` `:284` `:326` | `from_toml`, `validate_shape`, `validate_for`, `valid_identifier` | first two private; `[A-Za-z0-9_.-]`, ≤128 bytes |
| `:334` `:732` | `cargo_command`, `preflight` | `preflight` private |
| `:405` `:474` `:481` `:488` `:713` | `run_bounded_cancellable`, `run_qualification_process`, `run_qemu_process`, `run_bounded_inner`, `drain` | the middle two are `pub(crate)`; `run_bounded_inner` private and holds all enforcement |
| `:832` `:852` `:951` `:993` `:1006` | `execute_target`, `execute_target_cancellable`, `private_target_dir`, `safe_component`, `discover_candidate` | `safe_component` private |

**Tests (`:1050-1734`):** explicit non-shell command and `sh` rejection (`:1080-1114`); zigbuild
glibc rewrite and macOS-floor rejection (`:1116-1157`) plus the byte-unchanged regression pin for
the triple rewrite and the macOS-under-zigbuild refusal (`:1159-1223`); binding duplicate and
unknown-field rejection (`:1225-1238`); exact direct-slot coverage (`:1240-1269`); candidate
regularity and non-emptiness (`:1271-1286`); relative and reused work roots rejected
(`:1415-1421`); bounded output, nonzero exit, and overflow classified `OutputLimitExceeded`
(`:1423-1489`); timeout and cancellation each killing and waiting for a real Cargo build
script over three iterations (`:1491-1619`), plus the parked-descendant and parked-reader
cases (`:1621-1676`); a real local Cargo fixture producing a direct candidate
(`:1678-1734`).

## Boundaries / non-goals

- **Does not qualify** (no format, architecture, or runtime inspection — build success is
  not qualification; `qualification.rs` owns that) and **does not finalize** (no
  contract-named copying, archiving, manifest aggregation, or `release-manifest.json`).
- **Does not touch the network directly** — but it also does not forbid the child from
  reaching it. `cargo_command` passes `--locked`, not `--offline` (`:348-356`); the in-file
  fixture test adds `--offline` explicitly (`:1512`), which production does not.
- **No shell and no generic command DSL.** Executable and argv are separate OS arguments,
  and the executable set is a closed per-allowance enumeration checked at spawn
  (`:493-513`), per ADR-0004. A `CommandSpec` is a description, not an interpreter; there
  is no plugin or scripting seam.
- **No binding-coverage validation inside the build path.** `validate_for` (`:284`) is
  caller-invoked; `execute_target_cancellable` validates only plan membership,
  `schema_version == 1`, and non-empty `release_id` / `source_revision` (`:861-869`). An
  empty `bindings` slice therefore yields `Ok(BuildAttempt)` whose `process.outcome` is the
  `Success` initializer (`:880-884`) with zero candidates.
- **No output evidence** (failures are not diagnosable from captured text, by design), no
  cross-tool provisioning (`preflight` requires declared versions to be present; it does
  not install them), and no archive encoding, extraction, publication, signing, or install.

## Dependencies / dependents

- Own dependencies used here: `command-group 5.0.1` (`Cargo.toml:21`, process groups), `toml
  0.8` (`Cargo.toml:20`, `from_toml` at `builder.rs:246`), `serde` derive (`Cargo.toml:17`),
  `eggpack-contract` (`Cargo.toml:15`, `DistributionContract` + `ExpandedAssets` at
  `builder.rs:5`), and `std` process/thread/IO primitives. `sha2` is a **crate-level**
  dependency but is not used in this file (it is used by the digest authority in
  `src/digest.rs`).
- In-crate dependents: `qualification.rs:3-8` imports `run_qualification_process`,
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
