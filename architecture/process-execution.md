# Process Execution and Environment Handling — Deep Dive

This is the cross-cutting contract for spawning external tools in the Eggpack
workspace: bounded wall time, whole-group termination, capped output, a cleared
environment with a small allowlist, and captured output that never crosses the
API boundary. The contract is implemented in more than one crate, and the
implementations are not equivalent.

## Where external processes are spawned

There are exactly three production execution sites. Two share one implementation;
one is a fully independent reimplementation; one is unbounded.

| Site | Entry point | Spawn call | Deadline | Group handling | Environment |
| --- | --- | --- | --- | --- | --- |
| Core build + preflight | `run_bounded_cancellable` (`crates/eggpack-core/src/builder.rs:370`) | `group_spawn()` at `builder.rs:479` | `spec.timeout`, ≤ 86 400 s (`builder.rs:408`) | `command-group` group/job, `kill()` + `wait()` (`builder.rs:503-516`) | `env_clear()` + 20-var allowlist (`builder.rs:429`, `436-457`) |
| Core qualification smoke (native) | `run_qualification_process` (`builder.rs:377`) | same `run_bounded_inner` | `smoke.timeout_ms` (`qualification.rs:863`) | same shared path | narrower 5-var allowlist (`builder.rs:433-434`) |
| Core qualification smoke (QEMU) | `run_qemu_process` (`builder.rs:384`) | same `run_bounded_inner` | `smoke.timeout_ms` (`qualification.rs:851`) | same shared path | narrower 5-var allowlist |
| CI consumer validator | `run_consumer_validator` (`crates/eggpack-ci/src/lib.rs:4151`) → `run_validator_process` (`lib.rs:4625`) | `command.spawn()` at `lib.rs:4638` | `validator.timeout_ms`, 1 000–600 000 ms (`lib.rs:3917`) | **none** — `Child::kill()` only (`lib.rs:4665`, `4670`, `4678`, `4685`) | `env_clear()` + `PATH` (+ `SYSTEMROOT` on Windows) (`lib.rs:4347-4354`) |
| CI interpreter preflight | `run_interpreter_preflight` (`lib.rs:4372`) | `command.spawn()` at `lib.rs:4378` | `timeout_ms.min(30_000)` (`lib.rs:4387`) | **none** — `Child::kill()` at `lib.rs:4393` | same as above |
| CLI source verification | `verify_source_revision` (`crates/eggpack-cli/src/main.rs:71`) | `command.output()` at `main.rs:77` | **none** | **none** | **inherited unchanged** |

### What is shared and what is independent

The three `eggpack-core` entry points are thin wrappers over one private
function, `run_bounded_inner` (`builder.rs:391`), differing only in the
`ExecutableAllowance` passed: `Builder` (`builder.rs:374`), `Candidate`
(`builder.rs:381`), `Qemu` (`builder.rs:388`). One implementation, one place to
audit, one set of guarantees.

`eggpack-ci` does **not** share it. `crates/eggpack-ci/Cargo.toml` declares no
`command-group` dependency, and the validator uses `std::process::Command`
directly (`lib.rs:4343`, `4378`, `4638`), reimplementing bounds checking, output
capping, the poll loop, and the kill path. It matches the core contract on
timeout bounds, output bounds, null stdin, environment clearing, and wait-after-
kill; it does not match on process groups. This is the largest risk difference in
the workspace: a reimplementation that will not fail a compile-time or test-time
check when core's contract drifts.

The CLI `git` invocation is weaker again. It is a build-input trust check
(`eggpack ci _verify-source`, `main.rs:55-69`), not a build step, and it is
documented below as unbounded.

All `std::process::Command` uses in `eggpack-bootstrap` are inside its test
module (`crates/eggpack-bootstrap/src/lib.rs:1050`), and the remaining CLI
spawns are inside `mod tests` (`crates/eggpack-cli/src/main.rs:1166`). Neither
crate has a production spawn site.

## Process groups and jobs

A plain `std::process::Child::kill` terminates exactly one process. Cargo spawns
`rustc`, which spawns build scripts, which may spawn linkers; a QEMU smoke spawns
a guest candidate that may fork. Killing only the direct child leaves
grandchildren running with inherited pipes, holding the output readers open and
the job's build directory. The workspace therefore never uses bare `Child::kill`
on a build or qualification path; it uses `command-group` 5.0.1
(`crates/eggpack-core/Cargo.toml:22`, pinned in `Cargo.lock:61-64`).

`group_spawn()` (`builder.rs:479`) returns a `GroupChild` owning a group/job with
two different implementations:

- **Unix:** the child is spawned with `process_group(0)`, i.e. `setpgid(0, 0)`,
  making it a group leader (`command-group` `src/stdlib/unix.rs:26`). `kill()`
  is `killpg(pgid, SIGKILL)`, reaching every member
  (`src/stdlib/child/unix.rs:61-64`).
- **Windows:** the child is created `CREATE_SUSPENDED` and assigned to a job
  object (`src/stdlib/windows.rs:31`); `kill()` is
  `TerminateJobObject(job, 1)`, terminating the whole job
  (`src/stdlib/child/windows.rs:62-64`).

Killing is not enough. The group must then be *waited for*, twice over. In the
timeout and cancellation branches, `kill()` is immediately followed by `wait()`
(`builder.rs:505-507` for cancellation, `builder.rs:514-516` for the deadline). On
Unix that `wait()` loops `waitpid` over the negated pgid precisely so that
already-exited group members are reaped rather than left as zombies
(`command-group` `src/stdlib/child/unix.rs:71-78`); on Windows it drains the job
completion port with `INFINITE` before waiting on the child
(`src/stdlib/child/windows.rs:97-99`). Second, after the loop breaks,
`run_bounded_inner` requires a final `try_wait()` to return `Some`; a `None` is
the error `process cleanup incomplete` (`builder.rs:530-533`). Skipping either
wait is what leaves orphans and zombies behind a timeout, and both are present.

The CI validator waits after killing (`lib.rs:4394`, `4666`, `4671`, `4679`,
`4686`) but only reaps the direct child, because it never created a group.

## Timeouts and cancellation

`run_bounded_inner` validates its own bounds before spawning and fails closed
with `invalid process bounds or executable` (`builder.rs:406-415`): timeout must
be non-zero and at most 86 400 s, and both output limits must be in `1..=MAX_OUTPUT`
where `MAX_OUTPUT = 256 * 1024` (`builder.rs:18`).

The deadline is `Instant::now() + spec.timeout` (`builder.rs:499`) enforced by a
poll loop (`builder.rs:500-527`), not a blocking wait: each iteration calls
`child.try_wait()` (`builder.rs:519-522`) and sleeps 20 ms (`builder.rs:526`).
Polling is what makes cancellation observable at all — a blocking `wait()` would
only notice a flag after the child had already exited.

Ordering inside the loop is the contract. Cancellation is tested first
(`builder.rs:501`), the deadline second (`builder.rs:510`), exit status last. When
both apply in the same iteration, the run breaks with `(false, true)` and is
classified `Cancelled` (`builder.rs:508`, `549`). A cancelled build therefore never
reports as timed out. The CI validator uses the same precedence: cancellation at
`lib.rs:4661`, over-limit at `4669`, deadline at `4677`, each returning its own
typed failure. A pre-loop cancellation check also exists in
`qualification.rs:831-842`, so a cancelled qualification does not spawn at all.

Timeouts in use: builds get 1800 s (`builder.rs:736`); toolchain preflights get
10 s each (`builder.rs:598`, `617`, `646`, `662`); the QEMU emulator preflight
gets 10 s (`qualification.rs:792`); smoke runs get `smoke.timeout_ms` bounded to
`1..=86_400_000` (`qualification.rs:151-152`); CI validator runs get
`1_000..=600_000` ms (`lib.rs:3917`) with the preflight further capped at 30 s
(`lib.rs:4387`).

How a timeout reaches the caller: never as an error. `run_bounded_inner` returns
`Ok(ProcessEvidence)` with `CommandOutcome::TimedOut` (`builder.rs:546-547`,
enum at `builder.rs:142-153`). The build path converts any non-`Success` outcome
into a `BuildAttempt` carrying the evidence and no candidates
(`builder.rs:739-749`). A timeout is therefore a recorded, typed,
non-success *result*, not a raised error — which is what keeps a hung build from
looking like a crash.

One fidelity caveat in that classification: a failed `expected_stdout` check maps
to `CommandOutcome::Failed(-1)` (`builder.rs:552-553`), and a signal-terminated
process also maps to `Failed(-1)` via `status.code().unwrap_or(-1)`
(`builder.rs:557`). Callers cannot distinguish a version mismatch from a
signal kill by outcome alone.

## Output bounding and the no-output-leak rule

Both stdout and stderr are piped and drained by dedicated threads
(`builder.rs:430-431`, threads at `builder.rs:495` and `498`). `drain`
(`builder.rs:565-581`) reads in 8 KiB chunks, keeps at most
`limit - already_captured` bytes, and sets an overflow flag when a chunk does not
fit (`builder.rs:572-576`). Oversized output is *dropped*, not buffered to disk
and not truncated-with-a-tail. Both reader threads are joined before the outcome
is computed (`builder.rs:528-529`).

Overflow is a first-class failure. If either flag is set, the outcome is
`CommandOutcome::OutputLimitExceeded` (`builder.rs:550-551`), ranked below
timeout and cancellation but above exit status — an over-limit run is never
reported as a build failure, so an unbounded log flood cannot be mistaken for a
compiler error.

The CI validator caps the same way (`read_limited` at `lib.rs:4095`,
`read_limited_stderr` at `lib.rs:4119`) and additionally detects the overflow
*promptly* rather than at the deadline: `take_finished_over`
(`lib.rs:4416-4423`) joins only finished readers, and a completed over-limit
reader kills the run at once (`lib.rs:4669-4673`). One weakness: these readers
append the full chunk before comparing against the limit (`lib.rs:4105-4111`),
so retained bytes can exceed the configured limit by up to one 8 KiB chunk,
whereas `drain` truncates exactly to the limit.

The decisive rule is that **captured output contents never cross the API
boundary.** `ProcessEvidence` carries only an outcome and two byte counts
(`builder.rs:174-179`, constructed at `builder.rs:559-563`); it has no output
fields at all. The `expected_stdout` version substring is compared inside
`run_bounded_inner` (`builder.rs:542-545`) and is never stored, logged, or
returned — the field's own doc comment says so (`builder.rs:123-124`).
`ConsumerValidationEvidenceV1` is built from identity, interpreter, outcome,
size, and digest only (`lib.rs:4080-4090`).

This is a secrecy rule, not a size optimization. A captured compiler error can
contain absolute build paths, environment values, and — on a misconfigured host —
linker or interpreter diagnostics that reveal the internal environment. Because
only counts and typed outcomes leave the runner, an eggpack log, evidence
document, or CI artifact can state *that* a step failed without carrying the
failure text anywhere.

## Environment clearing and the allowlist

Every bounded spawn begins with `env_clear()` (`builder.rs:429`; CI at
`lib.rs:4347`). Nothing is inherited implicitly. Each variable is then re-added
only if it is present in the parent (`builder.rs:459-463`), and finally the
per-spec explicit overrides are applied with `command.envs(&spec.env)`
(`builder.rs:477`) — for a build, that is exactly `CARGO_TARGET_DIR`
(`builder.rs:351-355`).

The builder allowlist (`builder.rs:436-457`):

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
`WINDIR`, `TEMP`, `TMP` (`builder.rs:433-434`). The untrusted binary being
smoke-tested gets no `CARGO_HOME`, no `RUSTUP_HOME`, and no MSVC variables. This
is the meaningful containment: a candidate that can observe the toolchain roots
learns where the build machine's toolchain lives.

Because the environment is cleared rather than extended, ambient configuration
such as `RUSTFLAGS`, `CARGO_BUILD_*`, or a developer's `~/.cargo/config` export
does not silently alter a build. That is a determinism property as much as a
security one.

The CI validator allowlist is `PATH` always (`lib.rs:4348`) plus `SYSTEMROOT` on
Windows only (`lib.rs:4349-4354`), which CPython needs to locate its runtime.
`PATH` may be replaced wholesale by a caller-supplied directory list
(`lib.rs:4240-4253`), which is what keeps missing-interpreter tests hermetic;
that override is a test-and-fixture facility, and its value is fully
caller-controlled.

## Windows and MSVC

eggpack does not provision a Windows C toolchain and does not pretend to. The
MSVC variables in the allowlist are *preserved if already present* and otherwise
simply absent (`builder.rs:459-462`). Provisioning is the caller's and CI's
responsibility: `.github/workflows/ci.yml:43-46` runs
`ilammy/msvc-dev-cmd@v1` with `arch: x64` before any Windows build, and
`ci.yml:47-56` then verifies that `VCToolsInstallDir` is set and that `link.exe`
resolves underneath it.

The one adjustment eggpack makes is path ordering. On Windows, if
`VCToolsInstallDir` is set, `builder.rs:464-476` prepends
`%VCToolsInstallDir%/bin/Hostx64/x64` to the copied `PATH`. `env_clear()` has
discarded the environment `msvc-dev-cmd` carefully assembled, and this restores
the single entry that determines which `link.exe` is used; the wider MSVC
variables still have to survive the clear, which is why the allowlist is explicit
about them.

The consequence on an uninitialized Windows host is bounded and quiet. `cargo` is
found through `PATH`, finds no `link.exe`, exits non-zero, and the run is recorded
as `CommandOutcome::Failed(code)` (`builder.rs:556-557`) — or `Failed(-1)` if the
tool was signal-terminated. The diagnostic that would explain *why* the link
failed stays inside the discarded stderr buffer: only `stderr_bytes` is reported
(`builder.rs:562`). An operator sees "build failed, N bytes of stderr", and the
missing-environment diagnosis has to come from the CI step that checks for it,
not from the captured linker output.

## The executable allowlist (ADR-0004 boundary)

`ExecutableAllowance` (`builder.rs:99-104`) is the per-strategy restriction on
what may be executed. It is evaluated at `builder.rs:396-415`, *before* any
`Command` is constructed, and a non-matching executable is rejected with
`invalid process bounds or executable` (`builder.rs:406-414`).

| Allowance | Permitted executable | Enforcement |
| --- | --- | --- |
| `Builder` | exactly `cargo`, `rustc`, or `zig` | `matches!(spec.executable.as_str(), ...)` (`builder.rs:398`) |
| `Candidate` | any **absolute** path, which must then be made executable | `Path::new(&spec.executable).is_absolute()` (`builder.rs:400`) |
| `Qemu` | exactly `qemu-x86_64`, `qemu-aarch64`, or `qemu-arm` | `matches!(...)` (`builder.rs:401-404`) |

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
(`qualification.rs:785-788`). `Candidate` is *not* a closed list: it permits any
absolute path. What constrains it is provenance rather than identity — the
executable is `candidate.path` from a discovered candidate artifact
(`qualification.rs:858-859`), which was itself produced by a bounded build,
re-verified, and made executable. So the candidate allowance trusts a path's
origin, not its name. That is a deliberate difference in mechanism, and it is
why `ensure_candidate_executable` runs at `builder.rs:420-424` and a failure
there is a build error rather than a spawn.

The CI side closes the same hole by construction rather than by allowlist. The
interpreter is a single-variant enum, `ValidatorInterpreterV1::Python3`, and
anything else is rejected (`lib.rs:3913-3915`); `python_interpreter_exe`
(`lib.rs:4032`) maps it to `python3` or `python` by host
(`lib.rs:4037-4042`). The argv is then fixed at exactly two arguments — script
path, candidate path — or one `--version` (`lib.rs:4359-4368`). There is no
field in `ConsumerValidatorV1` for a caller-chosen executable, so there is
nothing for an allowlist to filter.

## Preflight checks and their strength

`preflight` (`builder.rs:584-674`) runs before any build, called from
`execute_target_cancellable` at `builder.rs:720`. The checks are not of uniform
strength, and stating otherwise would misrepresent the guarantee.

| Tool | Command | `expected_stdout` | Strength |
| --- | --- | --- | --- |
| `rustc` | `+<toolchain> --version` (`builder.rs:589-604`) | `None` (`builder.rs:601`) | **Exit status only** |
| `cargo` | `+<toolchain> --version` (`builder.rs:608-623`) | `None` (`builder.rs:620`) | **Exit status only** |
| `cargo-zigbuild` | `zigbuild --version` (`builder.rs:640-652`) | `Some("cargo-zigbuild <v>")` (`builder.rs:649`) | **Substring verified** |
| `zig` | `version` (`builder.rs:656-668`) | `Some(<v>)` (`builder.rs:665`) | **Substring verified** |
| QEMU | `<emulator> --version` (`qualification.rs:787-796`) | `None` (`qualification.rs:795`) | **Exit status only** |
| Python 3 | `--version` (`lib.rs:4365`, run at `lib.rs:4372`) | checked at `lib.rs:4412` | **Substring verified** (`"Python 3"`) |

So the Zig toolchain is version-verified; the Rust toolchain is not. For `rustc`
and `cargo`, the preflight asserts only that the rustup proxy resolved the
`+<toolchain>` argument and exited zero — it never compares the emitted version
string, so it passes for a proxy that succeeds without confirming the requested
toolchain was the one used. The consequence is bounded (the subsequent build
would fail), but the *evidence* for "the configured toolchain was present" is
weaker than the preflight's name suggests. The QEMU check is likewise
exit-status-only, though there tool identity is a fixed finite mapping
(`qualification.rs:785-786`).

A missing interpreter is a failure, never a skip: an unusable `python3` returns
`InterpreterUnavailable` (`lib.rs:4256-4261`) rather than passing validation
vacuously. A missing cargo-zigbuild or Zig declaration is rejected before spawn
(`builder.rs:632`, `638`).

## No shell interpretation

Every production spawn passes an argument vector to `Command::new` plus
`.args`/`.arg`. There is no `sh -c`, no `cmd /C`, and no shell string anywhere in
a spawn path. Core: `Command::new(&spec.executable).args(&spec.args)`
(`builder.rs:425-428`). CI: `Command::new(exe)` then `command.arg(script)` and
`command.arg(candidate)` (`lib.rs:4343`, `4361-4362`).

Because no shell is involved, the workspace does not need to escape, quote, or
reject shell metacharacters. Instead it validates the *inputs*:

- Cargo identifiers are restricted to ASCII alphanumerics, `_`, `-`, `.`, and at
  most 128 bytes (`valid_identifier`, `builder.rs:301-306`), applied to package
  and binary before an argv is built (`builder.rs:316-317`).
- Smoke argv is length- and count-capped and rejects NUL and all ASCII control
  characters (`qualification.rs:157-162`).
- The CI script path must be absolute, and the interpreter is not configurable.

`RunnerCommand::to_shell` / `argv` (`lib.rs:1120`) do produce shell text, but
only as rendered YAML for GitHub Actions to execute. That text is never executed
by eggpack, and its command enum is closed by construction — the doc comment at
`lib.rs:969-970` records that no generic arbitrary-command DSL is authorized.

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
