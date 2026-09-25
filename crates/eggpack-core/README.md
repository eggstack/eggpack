# eggpack-core

Producer-side planning, bounded Cargo building and qualification, local finalization, and manifest aggregation. PackConfig/ReleasePlan planning is pure. The first-party builder accepts strict `BuildBindingsV1` package/bin mappings for contract logical slots, checks the declared Rust/cargo-zigbuild tool intent, executes direct Cargo process groups with time/output bounds, and returns exact non-empty candidate paths. Each `BuildAttempt` carries the release id and source revision from its `ReleasePlan`. `QualificationBindingsV1` selects one exact candidate smoke check per executable target; fixed argv is passed directly to that candidate, with no shell, executable field, environment map, or repository script. `qualify_target` reconciles the plan, build bindings, attempt, candidate inventory, and qualification bindings; validates ELF, PE/COFF, or thin Mach-O format and architecture; hashes exact candidate bytes before and after execution; and returns deterministic identity-bound evidence. Native and deferred-native matching-host runs plus finite Linux QEMU user-mode runs use bounded process groups. Evidence contains host/classification/support, per-candidate size/hash/format/architecture, and bounded process outcomes, never local paths, output contents, or environment values.

`finalize_release` accepts identity-matched build and qualification evidence for the exact planned target set, enforces required-target qualification gates, copies direct and bundle candidates under contract names, and creates SHA-256 sidecars from final bytes. Archive output currently accepts only an explicit `ArchiveEncoding::TarGzip`; the contract archive asset filename must end in `.tar.gz`. Archive member names come from contract member paths, while tar/gzip timestamps and ownership metadata are normalized for deterministic output. Manifest v1 is constructed only after all artifacts and sidecars are complete, so its artifact digests cover final release bytes and its member digests cover the bytes written into the archive. The finalizer writes only into a new output root beneath a caller-secured parent (mode 0700 on Unix; Windows inherits the parent ACL) and returns no successful aggregate on failure. This remains producer-side file construction: no extraction, installation, publication, or authenticity claim is performed.

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
