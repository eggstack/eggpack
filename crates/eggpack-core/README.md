# eggpack-core

Producer-side planning, bounded Cargo building and qualification, and finalized-manifest construction. PackConfig/ReleasePlan planning is pure. The first-party builder accepts strict `BuildBindingsV1` package/bin mappings for contract logical slots, checks the declared Rust/cargo-zigbuild tool intent, executes direct Cargo process groups with time/output bounds, and returns exact non-empty candidate paths. `qualify_target` consumes one matching ReleasePlan target and successful BuildAttempt, rechecks candidate containment and bytes, and returns identity-bound native, deferred-native, emulated, or structural evidence. Native/emulated runs and explicit product hooks use shell-free process groups with deadlines, cancellation, output bounds, and a small inherited environment allowlist; evidence stores outcome/counts and candidate hashes, never captured output, paths, or environment values. Qualification evidence is separate from finalized release bytes; this crate does not yet assign final release names, generate sidecars, assemble archives, or aggregate qualification into a manifest.

`cargo_command` exposes typed shell-free Cargo/CargoZigbuild intent for provider renderers. `execute_target` uses a caller-owned absolute repository root and a pre-existing private work root; each invocation/target gets a new Cargo target directory. The local process runner only launches Cargo, rustc, or Zig, clears inherited environment state except a small toolchain/path allowlist, and never returns captured output contents. Process-tree timeout cleanup uses `command-group` process groups/jobs.

Windows Cargo builds require an initialized Visual Studio developer environment. The Windows CI lane runs `ilammy/msvc-dev-cmd` before builder qualification and verifies that `link.exe` resolves beneath `VCToolsInstallDir`; this supplies the MSVC linker, SDK, and library environment used by Cargo and build scripts.

Example logical binding document:

```toml
schema_version = 1

[targets]
"x86_64-unknown-linux-gnu" = [
  { selector = { kind = "direct" }, package = "eggsact", binary = "eggsact" },
]
```
