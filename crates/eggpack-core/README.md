# eggpack-core

Producer-side planning, bounded Cargo building, and finalized-manifest construction. PackConfig/ReleasePlan planning is pure. The first-party builder accepts strict `BuildBindingsV1` package/bin mappings for contract logical slots, checks the declared Rust/cargo-zigbuild tool intent, executes direct Cargo process groups with time/output bounds, and returns exact non-empty candidate paths. Candidate bytes are not qualified or finalized release bytes; this crate does not assign final release names, hash candidates into a manifest, publish, acquire, or install releases.

`cargo_command` exposes typed shell-free Cargo/CargoZigbuild intent for provider renderers. `execute_target` uses a caller-owned absolute repository root and a pre-existing private work root; each invocation/target gets a new Cargo target directory. The local process runner only launches Cargo, rustc, or Zig, clears inherited environment state except a small toolchain/path allowlist, and never returns captured output contents. Process-tree timeout cleanup uses `command-group` process groups/jobs.

Example logical binding document:

```toml
schema_version = 1

[targets]
"x86_64-unknown-linux-gnu" = [
  { selector = { kind = "direct" }, package = "eggsact", binary = "eggsact" },
]
```
