# Process Execution and Environment Handling — Deep Dive

This is the cross-cutting contract for spawning external tools in the Eggpack
workspace: bounded wall time, whole-group termination, capped output, a cleared
environment with a small allowlist, and captured output that never crosses the
API boundary. The contract has one implementation: `eggpack-core` owns the
bounded runner, and `eggpack-cli` and `eggpack-ci` both delegate to it. No
production spawn site is unbounded.

## Where external processes are spawned

There is exactly one production execution implementation. `eggpack-core` owns
the bounded runner; `eggpack-cli` and `eggpack-ci` call into it, so all three
share one set of guarantees.

| Site | Entry point | Spawn call | Deadline | Group handling | Environment |
| --- | --- | --- | --- | --- | --- |
| Core build + preflight | `run_bounded_cancellable` (`crates/eggpack-core/src/builder.rs:461`) | `group_spawn()` at `builder.rs:801` | `spec.timeout`, ≤ 86 400 s (`builder.rs:715-716`) | `command-group` group/job, `kill()` + `wait()` (`builder.rs:824-845`) | `env_clear()` + 19-variable builder allowlist (`builder.rs:737`, `758-779`) |
| Core qualification smoke (native) | `run_qualification_process` (`builder.rs:569`) | same `run_bounded` | `smoke.timeout_ms` | same shared path | narrower 5-var allowlist (`builder.rs:755-756`) |
| Core qualification smoke (QEMU) | `run_qemu_process` (`builder.rs:580`) | same `run_bounded` | `smoke.timeout_ms` | same shared path | narrower 5-var allowlist |
| Core git revision verification | `run_git_bounded` (`builder.rs:512`) | same `run_bounded` | 30 s (`GIT_TIMEOUT`, `builder.rs:29`) | same shared path | 7-var allowlist incl. `HOME` (`builder.rs:742-753`) |
| CLI source verification | `verify_source_revision` (`crates/eggpack-cli/src/main.rs:410`) | delegates to `run_git_bounded` | same 30 s | same shared path | same 7-var allowlist |
| CI consumer validator | `run_consumer_validator` (`crates/eggpack-ci/src/consumer_validator.rs:323`) → `run_validator_process` (`consumer_validator.rs:521`) | delegates to `run_interpreter_bounded` (`builder.rs:485`) | `validator.timeout_ms`, 1 000–600 000 ms (`consumer_validator.rs:87`) | same shared path — group/job kill on every failure path | same narrow 5-var allowlist, plus the caller's explicit `PATH` override (`consumer_validator.rs:299-300`) |
| CI interpreter preflight | `run_interpreter_preflight` (`consumer_validator.rs:488`) | delegates to `run_interpreter_bounded` | `timeout_ms.min(30_000)` (`consumer_validator.rs:499`) | same shared path | same as above |

### What is shared and what is independent

The five `eggpack-core` entry points are thin wrappers over one private
function, `run_bounded` (`builder.rs:698`), reached through `run_bounded_inner`
(`builder.rs:608`), differing only in the `ExecutableAllowance` passed:
`Builder` (`builder.rs:128`), `Candidate`, `Qemu`, `Git`, and `Interpreter`
(`builder.rs:132`). One implementation, one place to audit, one set of
guarantees. The allowance is what keeps the set narrow: `Git` admits only the
literal `git` (`builder.rs:711`), exactly as `Qemu` admits only the three named
QEMU binaries and `Interpreter` admits only `python3` and `python`
(`builder.rs:708-710`).

`eggpack-ci` shares it. The crate still declares no `command-group`
dependency; it reaches the runner through `eggpack-core`, which already owns
it. The consumer validator builds an `InterpreterCommandSpec`
(`builder.rs:203`) — the interpreter name, an OS-native argument vector, the
working directory, the explicit environment overrides, the deadline, the output
bounds, and the optional version substring — and calls `run_interpreter_bounded`
(`builder.rs:485`). The interpreter argv is OS-native so a validated absolute
path reaches the interpreter byte-exact. Everything else — bounds checking,
output capping, the poll loop, the group kill, the bounded reader joins, and
the environment allowlist — is the shared implementation, so the crate cannot
drift from the core contract without failing a compile-time check.

The CLI `git` invocation no longer diverges. `verify_source_revision`
(`main.rs:410`) validates the expected object name, then delegates to
`eggpack_core::run_git_bounded` (`builder.rs:512`) and reads only the resulting
`CommandOutcome`. It inherits the deadline, the process group, the environment
clear, and the output cap, and it never receives captured bytes.

The runner matches an exact expected value internally rather than returning it.
`CommandSpec::expected_stdout` is a substring containment check, which is right
for a tool banner (`builder.rs:175-176`) but too weak for a revision identity; the
new `expected_stdout_exact` field (`builder.rs:177-181`) compares the trimmed stdout
for equality (`builder.rs:888-893`), and a miss becomes
`CommandOutcome::Failed(-1)` (`builder.rs:900-901`) rather than an error carrying
output.

The Git allowlist deliberately keeps `HOME` and `USERPROFILE`
(`builder.rs:747-748`) that the narrower candidate and QEMU allowlists drop.
`git` resolves helpers from `PATH`, and `safe.directory` is commonly set in the
global config; clearing `HOME` would make self-hosted runners fail closed for a
reason unrelated to release integrity.

All `std::process::Command` uses in `eggpack-bootstrap` are inside its test
module (`crates/eggpack-bootstrap/src/lib.rs:1066`), and the remaining CLI
spawns are inside `mod tests` (`crates/eggpack-cli/src/main.rs:1711`). Neither
crate has a production spawn site.

## Process groups and jobs

A plain `std::process::Child::kill` terminates exactly one process. Cargo spawns
`rustc`, which spawns build scripts, which may spawn linkers; a QEMU smoke spawns
a guest candidate that may fork. Killing only the direct child leaves
grandchildren running with inherited pipes, holding the output readers open and
the job's build directory. The workspace therefore never uses bare `Child::kill`
on a build or qualification path; it uses `command-group` 5.0.1
(`crates/eggpack-core/Cargo.toml:24`, pinned in `Cargo.lock:61-64`).

`group_spawn()` (`builder.rs:801`) returns a `GroupChild` owning a group/job with
two different implementations:

- **Unix:** the child is spawned with `process_group(0)`, i.e. `setpgid(0, 0)`,
  making it a group leader (`command-group` `src/stdlib/unix.rs:26`). `kill()`
  is `killpg(pgid, SIGKILL)`, reaching every member
  (`src/stdlib/child/unix.rs:58-64`).
- **Windows:** the child is created `CREATE_SUSPENDED` and assigned to a job
  object (`src/stdlib/windows.rs:29-37`); `kill()` is
  `TerminateJobObject(job, 1)`, terminating the whole job
  (`src/stdlib/child/windows.rs:62-64`).

Killing is not enough. The group must then be *waited for*, twice over. In the
timeout and cancellation branches, `kill()` is immediately followed by `wait()`
(`builder.rs:823-828` for cancellation, `builder.rs:830-835` for the deadline). On
Unix that `wait()` loops `waitpid` over the negated pgid precisely so that
already-exited group members are reaped rather than left as zombies
(`command-group` `src/stdlib/child/unix.rs:71-85`); on Windows it drains the job
completion port with `INFINITE` before waiting on the child
(`src/stdlib/child/windows.rs:97-99`). Second, after the loop breaks,
`run_bounded_inner` requires a final `try_wait()` to return `Some`; a `None` is
the error `process cleanup incomplete` (`builder.rs:876-879`). Skipping either
wait is what leaves orphans and zombies behind a timeout, and both are present.

The CI validator creates no group of its own: it delegates to the shared
runner, which creates the group or job object and kills and waits for it.

### Rechecking a failed process-group signal

`command-group` sends Unix group termination with `killpg`. Linux can signal a
zombie member, while macOS may report `EPERM` when a group contains an
unreaped zombie. `ESRCH` remains the kernel's direct indication that the group
does not exist.

M003p correctly addressed the macOS output-limit failure, but its exception
was too broad: `GroupChild::try_wait` reports only the supervised leader's
state. Reaping that leader does not prove the process group is empty; a live
descendant can remain, including one that closed stdout and stderr. The drain
budget detects descendants that keep a pipe open, but pipe completion is not
evidence that every group member has exited.

The corrected rule (`terminate_process_group` and
`group_signal_failure_is_benign` in `crates/eggpack-core/src/builder.rs`) is:

- Initial `ESRCH` is benign because the group is absent.
- For another initial signal error, a live or unverified supervised child is
a hard failure.
- If the supervised child has exited, `try_wait` reaps it and the runner
  retries the group signal. Retry success means the group signal was sent to
  remaining members; retry `ESRCH` proves the group is empty. Every other retry
  error, including `EPERM`, remains a hard failure.

Thus the zombie-only macOS case can resolve to an empty group on retry, but a
leader's exit alone never suppresses a group-signal failure. The rule is
Unix-only. Windows job objects use `TerminateJobObject` and a different error
space, where the value `3` means `ERROR_PATH_NOT_FOUND`, not `ESRCH`; their
behavior is unchanged.

The regression suite pins the decision with injected signal outcomes,
retains the real empty-group case, and exercises termination with a live
descendant after its leader exits. The output-overflow test continues to cover
the actual bounded-runner path. M003pa is not closed until the local gate and
hosted macOS, Linux stable/MSRV, and Windows lanes pass on the same candidate.

## Timeouts and cancellation

`run_bounded_inner` validates its own bounds before spawning and fails closed
with `invalid process bounds or executable` (`builder.rs:714-723`): timeout must
be non-zero and at most 86 400 s, and both output limits must be in `1..=MAX_OUTPUT`
where `MAX_OUTPUT = 256 * 1024` (`builder.rs:20`).

The deadline is `Instant::now() + spec.timeout` (`builder.rs:821`) enforced by a
poll loop (`builder.rs:822-855`), not a blocking wait: each iteration calls
`child.try_wait()` (`builder.rs:847-852`) and sleeps 20 ms (`builder.rs:854`).
Polling is what makes cancellation observable at all — a blocking `wait()` would
only notice a flag after the child had already exited.

Ordering inside the loop is the contract. Cancellation is tested first
(`builder.rs:823`), the deadline second (`builder.rs:830`), the output bound
third (`builder.rs:840`), exit status last (`builder.rs:847`). When cancellation
and the deadline apply in the same iteration, the run breaks with
`(false, true, false)` and is classified `Cancelled` (`builder.rs:828`,
`builder.rs:232`). A cancelled build therefore never reports as timed out, and
an over-limit run is reported as an output-limit failure rather than as a build
failure. The CI validator inherits that precedence unchanged and maps each
runner outcome onto its own typed failure. A pre-loop cancellation check also
exists in `qualification.rs:887-899`, so a cancelled qualification does not spawn
at all.

Timeouts in use: builds get 1800 s (`builder.rs:1122`); toolchain preflights get
10 s each (`builder.rs:977`, `builder.rs:997`, `builder.rs:1027`,
`builder.rs:1046`); the QEMU emulator preflight gets 10 s
(`qualification.rs:845`); smoke runs get `smoke.timeout_ms` bounded to
`1..=86_400_000` (`qualification.rs:151-152`); CI validator runs get
`1_000..=600_000` ms (`consumer_validator.rs:87`) with the preflight further capped at 30 s
(`consumer_validator.rs:499`).

How a timeout reaches the caller: never as an error. `run_bounded_inner` returns
`Ok(ProcessEvidence)` with `CommandOutcome::TimedOut` (`builder.rs:894-895`,
enum at `builder.rs:224-235`). The build path converts any non-`Success` outcome
into a `BuildAttempt` carrying the evidence and no candidates
(`builder.rs:1125-1134`). A timeout is therefore a recorded, typed,
non-success *result*, not a raised error — which is what keeps a hung build from
looking like a crash.

One fidelity caveat in that classification: a failed `expected_stdout` check maps
to `CommandOutcome::Failed(-1)` (`builder.rs:900-901`), and a signal-terminated
process also maps to `Failed(-1)` via `status.code().unwrap_or(-1)`
(`builder.rs:905`). Callers cannot distinguish a version mismatch from a
signal kill by outcome alone.

## Output bounding and the no-output-leak rule

Both stdout and stderr are piped and drained by dedicated threads
(`builder.rs:739-740`, threads at `builder.rs:817`, `820`, and `498`). `drain`
(`builder.rs:944-960`) reads in 8 KiB chunks, keeps at most
`limit - already_captured` bytes, and sets an overflow flag when a chunk does not
fit (`builder.rs:947-955`). Oversized output is *dropped*, not buffered to disk
and not truncated-with-a-tail. Both reader threads are joined before the outcome
is computed, under a bounded budget (`join_drained`, `builder.rs`): a reader
returns only at pipe EOF, which needs every write end closed, so a descendant
that inherited stdout and outlived the child would otherwise park the join for
that descendant's whole lifetime. The budget is what remains of the caller's own
deadline, clamped to `MIN_DRAIN_GRACE`..`MAX_DRAIN_GRACE`; a reader still parked
at it is detached and the run fails closed as `OutputLimitExceeded`. This also
removes a false-success path: a child that exits zero while a descendant holds
the pipe used to be reported as a success with incomplete captured output.

Overflow is a first-class failure. If either flag is set, the outcome is
`CommandOutcome::OutputLimitExceeded` (`builder.rs:898-899`), ranked below
timeout and cancellation but above exit status — an over-limit run is never
reported as a build failure, so an unbounded log flood cannot be mistaken for a
compiler error.

The CI validator used to cap the same way with readers of its own and to detect
the overflow *promptly* rather than at the deadline. It no longer has readers
of its own: the shared `drain` (`builder.rs:944`) does the capping, and the
shared runner detects the overflow promptly — `stream_over_limit`
(`builder.rs:940`) reads the same flag the drain thread sets, and the runner
terminates the whole group at the moment of excess (`builder.rs:840-845`).
That prompt detection was CI's strength and it is now the shared runner's: a
process that exceeds its output bound is killed when it exceeds it, not when
its deadline arrives. The observable outcome is unchanged.

The decisive rule is that **captured output contents never cross the API
boundary.** `ProcessEvidence` carries only an outcome and two byte counts
(`builder.rs:256-263`, constructed at `builder.rs:907-911`); it has no output
fields at all. The `expected_stdout` version substring is compared inside
`run_bounded_inner` (`builder.rs:888-893`) and is never stored, logged, or
returned — the field's own doc comment says so (`builder.rs:175-176`).
`ConsumerValidationEvidenceV1` is built from identity, interpreter, outcome,
size, and digest only (`consumer_validator.rs:267-277`).

This is a secrecy rule, not a size optimization. A captured compiler error can
contain absolute build paths, environment values, and — on a misconfigured host —
linker or interpreter diagnostics that reveal the internal environment. Because
only counts and typed outcomes leave the runner, an eggpack log, evidence
document, or CI artifact can state *that* a step failed without carrying the
failure text anywhere.

## Environment clearing and the allowlist

Every bounded spawn begins with `env_clear()` (`builder.rs:737`); the CI
validator reaches that same call through the shared runner. Nothing is
inherited implicitly. Each variable is then re-added
only if it is present in the parent (`builder.rs:781-785`), and finally the
per-spec explicit overrides are applied with `command.envs(&spec.env)`
(`builder.rs:799`) — for a build, that is exactly `CARGO_TARGET_DIR`
(`builder.rs:442-446`).

The builder allowlist (`builder.rs:758-779`):

| Variable | Why it is allowed |
| --- | --- |
| `PATH` | Locate the toolchain and linker binaries. |
| `HOME` | Cargo/rustup registry and git cache roots on Unix. |
| `USERPROFILE` | Same role on Windows. |
| `SystemRoot` | Required by the Windows loader and by MSVC tooling. |
| `WINDIR` | Windows system directory; paired with `SystemRoot`. |
| `TEMP`, `TMP` | Temp directory for cargo and link scratch files. |
| `CARGO_HOME` | Registry and installed-subcommand location (`cargo-zigbuild`). |
| `RUSTUP_HOME` | Installed toolchains; the `+toolchain` proxy argument resolves here. |
| `LIB` | MSVC/CRT import and static library search path. |
| `LIBPATH` | Legacy MSVC library search path. |
| `INCLUDE` | MSVC/CRT/SDK headers required by the compiler. |
| `VCToolsInstallDir` | MSVC toolset root; also the anchor for the linker path fix below. |
| `VisualStudioVersion`, `VCINSTALLDIR` | MSVC install identity consumed by the build. |
| `WindowsSdkDir`, `WindowsSDKVersion` | Windows SDK root and version. |
| `UniversalCRTSdkDir`, `UCRTVersion` | Universal CRT root and version. |

Candidate and QEMU runs get a deliberately narrower set — `PATH`, `SystemRoot`,
`WINDIR`, `TEMP`, `TMP` (`builder.rs:755-756`). The untrusted binary being
smoke-tested gets no `CARGO_HOME`, no `RUSTUP_HOME`, and no MSVC variables. This
is the meaningful containment: a candidate that can observe the toolchain roots
learns where the build machine's toolchain lives.

Because the environment is cleared rather than extended, ambient configuration
such as `RUSTFLAGS`, `CARGO_BUILD_*`, or a developer's `~/.cargo/config` export
does not silently alter a build. That is a determinism property as much as a
security one.

The CI validator no longer restates an allowlist. It runs under the narrow
five-variable allowance that the candidate and QEMU runs use — `PATH`,
`SystemRoot`, `WINDIR`, `TEMP`, `TMP` (`builder.rs:755-756`) — which is the
same allowance the interpreter needs to locate its runtime. `PATH` may be
replaced wholesale by a caller-supplied directory list (`consumer_validator.rs:299-300`),
which is what keeps missing-interpreter tests hermetic; that override is a
test-and-fixture facility, and its value is fully caller-controlled.

## Windows and MSVC

eggpack does not provision a Windows C toolchain and does not pretend to. The
MSVC variables in the allowlist are *preserved if already present* and otherwise
simply absent (`builder.rs:781-785`). Provisioning is the caller's and CI's
responsibility: `.github/workflows/ci.yml:43-46` runs
`ilammy/msvc-dev-cmd@v1` with `arch: x64` before any Windows build, and
`.github/workflows/ci.yml:47-56` then verifies that `VCToolsInstallDir` is set and that `link.exe`
resolves underneath it.

The one adjustment eggpack makes is path ordering. On Windows, if
`VCToolsInstallDir` is set, `builder.rs:786-798` prepends
`%VCToolsInstallDir%/bin/Hostx64/x64` to the copied `PATH`. `env_clear()` has
discarded the environment `msvc-dev-cmd` carefully assembled, and this restores
the single entry that determines which `link.exe` is used; the wider MSVC
variables still have to survive the clear, which is why the allowlist is explicit
about them.

The consequence on an uninitialized Windows host is bounded and quiet. `cargo` is
found through `PATH`, finds no `link.exe`, exits non-zero, and the run is recorded
as `CommandOutcome::Failed(code)` (`builder.rs:904-905`) — or `Failed(-1)` if the
tool was signal-terminated. The diagnostic that would explain *why* the link
failed stays inside the discarded stderr buffer: only `stderr_bytes` is reported
(`builder.rs:910`). An operator sees "build failed, N bytes of stderr", and the
missing-environment diagnosis has to come from the CI step that checks for it,
not from the captured linker output.

## The executable allowlist (ADR-0004 boundary)

`ExecutableAllowance` (`builder.rs:127-132`) is the per-strategy restriction on
what may be executed. It is evaluated at `builder.rs:703-713`, *before* any
`Command` is constructed, and a non-matching executable is rejected with
`invalid process bounds or executable` (`builder.rs:714-723`).

| Allowance | Permitted executable | Enforcement |
| --- | --- | --- |
| `Builder` | exactly `cargo`, `rustc`, or `zig` | `matches!(run.executable, ...)` (`builder.rs:704-706`) |
| `Candidate` | any **absolute** path, which must then be made executable | `Path::new(run.executable).is_absolute()` (`builder.rs:707`) |
| `Qemu` | exactly `qemu-x86_64`, `qemu-aarch64`, or `qemu-arm` | `matches!(...)` (`builder.rs:708-710`) |

This is the enforcement point for ADR-0004's rejection of a generic shell DSL
and of a "run whatever" escape hatch (`architecture/principles-roadmap.md:94-98`).
It is a security boundary, not a convenience, for one reason: the executable
name is the one string in a `CommandSpec` that is resolved through `PATH` by the
operating system. Everything else — args, working directory, environment
overrides — is data the workspace validates and controls. Without this check, any
`CommandSpec` that reached the runner would be an arbitrary-program execution
primitive, and the runner's carefully bounded timeout, group kill, and cleared
environment would simply be a safe way to run an attacker's chosen binary.

The two non-`Builder` allowances are worth being precise about. `Qemu` is a
closed three-name list, matching the finite target→emulator mapping
(`qualification.rs:838-839`). `Candidate` is *not* a closed list: it permits any
absolute path. What constrains it is provenance rather than identity — the
executable is `candidate.path` from a discovered candidate artifact
(`qualification.rs:917`), which was itself produced by a bounded build,
re-verified, and made executable. So the candidate allowance trusts a path's
origin, not its name. That is a deliberate difference in mechanism, and it is
why `ensure_candidate_executable` runs at `builder.rs:728-731` and a failure
there is a build error rather than a spawn.

The CI side closes the same hole twice over. The interpreter is a
single-variant enum, `ValidatorInterpreterV1::Python3`, and anything else is
rejected; `python_interpreter_exe` (`consumer_validator.rs:219`) maps it to `python3` or
`python` by host (`consumer_validator.rs:224`). The argv is then fixed at exactly two
arguments — script path, candidate path (`consumer_validator.rs:530-533`), or one
`--version` (`consumer_validator.rs:497`). There is no field in `ConsumerValidatorV1` for a
caller-chosen executable. And the shared runner enforces the allowance anyway:
the `Interpreter` allowance (`builder.rs:132`) admits only `python3` and
`python`, so a spec that named anything else is refused before any `Command` is
constructed.

## Preflight checks and their strength

`preflight` (`builder.rs:963-1060`) runs before any build, called from
`execute_target_cancellable` at `builder.rs:1083`. The checks are not of uniform
strength, and stating otherwise would misrepresent the guarantee.

| Tool | Command | `expected_stdout` | Strength |
| --- | --- | --- | --- |
| `rustc` | `+<toolchain> --version` (`builder.rs:968-982`) | `None` (`builder.rs:980`) | **Exit status only** |
| `cargo` | `+<toolchain> --version` (`builder.rs:988-1002`) | `None` (`builder.rs:1000`) | **Exit status only** |
| `cargo-zigbuild` | `zigbuild --version` (`builder.rs:1021-1034`) | `Some("cargo-zigbuild <v>")` (`builder.rs:1033`) | **Substring verified** |
| `zig` | `version` (`builder.rs:1040-1052`) | `Some(<v>)` (`builder.rs:1051`) | **Substring verified** |
| QEMU | `<emulator> --version` (`qualification.rs:840-850`) | `None` (`qualification.rs:848`) | **Exit status only** |
| Python 3 | `--version` (`consumer_validator.rs:497`, run at `consumer_validator.rs:503`) | `expected_stdout` = `"Python 3"` (`consumer_validator.rs:502`) | **Substring verified** (`"Python 3"`) |

So the Zig toolchain is version-verified; the Rust toolchain is not. For `rustc`
and `cargo`, the preflight asserts only that the rustup proxy resolved the
`+<toolchain>` argument and exited zero — it never compares the emitted version
string, so it passes for a proxy that succeeds without confirming the requested
toolchain was the one used. The consequence is bounded (the subsequent build
would fail), but the *evidence* for "the configured toolchain was present" is
weaker than the preflight's name suggests. The QEMU check is likewise
exit-status-only, though there tool identity is a fixed finite mapping
(`qualification.rs:838-839`).

A missing interpreter is a failure, never a skip: an unusable `python3` returns
`InterpreterUnavailable` (`consumer_validator.rs:551-554`) rather than passing validation
vacuously. A missing cargo-zigbuild or Zig declaration is rejected before spawn
(`builder.rs:1014`, `builder.rs:1020`).

## No shell interpretation

Every production spawn passes an argument vector to `Command::new` plus
`.args`/`.arg`. There is no `sh -c`, no `cmd /C`, and no shell string anywhere in
a spawn path. Core: `Command::new(run.executable).args(run.args)`
(`builder.rs:733-736`). CI: the same shared call, with the argv built as
OS-native arguments — script path and candidate path (`consumer_validator.rs:530-533`).

Because no shell is involved, the workspace does not need to escape, quote, or
reject shell metacharacters. Instead it validates the *inputs*:

- Cargo identifiers are restricted to ASCII alphanumerics, `_`, `-`, `.`, and at
  most 128 bytes (`valid_identifier`, `builder.rs:383-388`), applied to package
  and binary before an argv is built (`builder.rs:436-441`).
- Smoke argv is length- and count-capped and rejects NUL and all ASCII control
  characters (`qualification.rs:157-162`).
- The CI script path must be absolute, and the interpreter is not configurable.

`RunnerCommand::to_shell` / `argv` (`runner.rs:460`, `runner.rs:169`) do produce shell
text, but
only as rendered YAML for GitHub Actions to execute. That text is never executed
by eggpack, and its command enum is closed by construction — the doc comment at
`runner.rs:10-11` records that no generic arbitrary-command DSL is authorized.

## Related deep dives

- [overview.md](overview.md) — module map and producer pipeline.
- [core-build.md](core-build.md) — planning, preflight, and candidate discovery.
- [core-qualification.md](core-qualification.md) — smoke bindings, QEMU, and
  host matching.
- [ci-consumer-seam.md](ci-consumer-seam.md) — the consumer validator and
  evidence linkage.
- [validation-model.md](validation-model.md) — fail-closed validation rules.
- [determinism.md](determinism.md) — canonical ordering and byte-identical
  rendering.
- [testing-and-portability.md](testing-and-portability.md) — platform coverage and
  the Windows builder lane.
