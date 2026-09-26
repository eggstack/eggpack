# Build and Qualification Milestone 005 — Deterministic Cross-Tool Provisioning

Status: ready for handoff

Repository baseline: `f3b1f3e53ab1ddca7f3cee9f1bc885a965d660a7` (architecture-documentation-only successor to the reviewed `b8062dff` implementation baseline)

Research refresh: 2026-09-26. CI M003d is closed at `plans/closure/ci-release-orchestration/003d-status.md`; M005 is now the sole remaining Eggpack-side prerequisite for the first eggsact consumer adoption.

Source roadmaps:

- `plans/subsystems/build-qualification-roadmap.md`
- `plans/subsystems/ecosystem-adoption-roadmap.md`

First proving consumer:

- `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25` (release-surface proving baseline)
- current eggsact `main` reviewed at `38aa6da3e8745ed4599362e2ecb0837bc20fca90`; changes since the proving baseline do not alter the release workflow/toolchain surfaces relevant to M005

Related decision:

- ADR-0004 first-party native Cargo/cargo-zigbuild adapter.

Primary class: infrastructure / toolchain integrity / generated CI

## 1. Objective

Make CargoZigbuild release jobs reproducible on real hosted runners by provisioning and verifying the exact cross-build tools declared by producer policy.

Current Eggpack generated CI only records whether a runner is expected to have `cargo-zigbuild` and Zig preinstalled. It verifies the cargo-zigbuild version but merely executes `zig version` without an expected version.

The first consumer, eggsact, requires:

- Zig 0.14.1;
- cargo-zigbuild 0.23.3;
- SHA-256-verified official Zig archives;
- Linux x86-64 and AArch64 runner support;
- GNU libc 2.17 target-floor syntax.

M005 must preserve ADR-0004's first-party Cargo/cargo-zigbuild boundary while removing ambient mutable toolchain assumptions.

## 2. Current incompatibility

At the reviewed Eggpack baseline:

- `ToolchainRequirement` carries Rust + optional cargo-zigbuild version, but no Zig version;
- `RunnerMapping` carries only boolean `cargo_zigbuild` / `zig` capability assertions;
- generated release jobs do not install cargo-zigbuild;
- generated release jobs do not install Zig;
- generated release jobs do not compare `zig version` with a configured value.

A real eggsact migration would therefore weaken its current release reproducibility and likely fail on ordinary hosted runners where cargo-zigbuild is absent.

## 3. Non-goals

M005 does not:

- adopt a generic package manager;
- use apt/Homebrew/Chocolatey for Zig;
- add arbitrary tool download URLs;
- add arbitrary setup commands;
- change Cargo build command semantics;
- change qualification/finalization semantics;
- broaden to non-Zig cross toolchains;
- select a new external build backend;
- upgrade eggsact from Zig 0.14.1 or cargo-zigbuild 0.23.3;
- replace the known-good `cargo install --version ... --locked` cargo-zigbuild acquisition path with upstream prebuilt binaries in this milestone;
- independently prove the emitted ELF symbol-version floor beyond preserving the existing `.2.17` cargo-zigbuild target semantics.

## 4. Toolchain requirement

Extend the CargoZigbuild toolchain model so exact Zig version is producer policy.

Preferred additive shape:

```text
ToolchainRequirement {
  rust
  cargo_zigbuild
  zig
}
```

Validation:

- NativeCargo => cargo_zigbuild absent, zig absent;
- CargoZigbuild => cargo_zigbuild present, zig present;
- all versions bounded by the existing safe-tool-version grammar.

Compatibility decision for this pre-stabilization milestone:

- keep `PackConfig`, `ReleasePlan`, and `CIPlan` at schema version 1 rather than cascading a version bump through the just-qualified M003d workflow surface;
- add Zig as an optional serialized toolchain field with an absent/default representation so historical documents remain parseable by the new implementation;
- make validation/resolution fail closed for `CargoZigbuild` when either cargo-zigbuild or Zig is absent;
- require both cross-tool fields to be absent for `NativeCargo`;
- never reinterpret historical `zig = None` as "any Zig" or permission to use an ambient Zig;
- record in M005 closure that newly generated schema-v1 documents containing the additive Zig field are a deliberate pre-1.0 forward-compatibility break for pre-M005 Eggpack binaries.

The same resolved `TargetPolicy` remains the tool-version authority when embedded in workflow-shape/CI documents; provider policy must not duplicate or override the configured Zig version.

## 5. Provider provisioning policy

Replace/augment boolean runner assertions with a finite provisioning policy for CargoZigbuild jobs.

Suggested shape:

```text
CrossToolProvisioningV1 {
  cargo_zigbuild: CargoInstallLocked
  zig: OfficialArchive {
    linux_x86_64_sha256
    linux_aarch64_sha256
  }
}
```

The Zig version comes from the resolved TargetPolicy, not from the provider policy.

Production Zig source is fixed to the official `https://ziglang.org/download/<version>/...` origin.

Supported initial provider hosts:

- Linux x86_64;
- Linux AArch64.

Do not accept arbitrary archive URLs or mirrors in schema v1.

The provider policy controls *how* the finite approved tools are provisioned; `ToolchainRequirement` controls *which exact versions* are required. Runner labels continue to describe host OS/architecture capability only.

## 6. cargo-zigbuild provisioning

For CargoZigbuild jobs:

1. install the exact configured version with:
   `cargo install cargo-zigbuild --version <version> --locked`;
2. use an invocation-private Cargo install root or another deterministic runner-local location;
3. add only its bin directory to PATH;
4. require exact `cargo zigbuild --version` output;
5. bound install time;
6. set an invocation-private `CARGO_INSTALL_ROOT` and prepend only its `bin` directory for the build job;
7. reject success unless `cargo zigbuild --version` exactly equals `cargo-zigbuild <configured-version>`.

Do not use an unversioned ambient cargo-zigbuild even if one happens to exist.

Upstream cargo-zigbuild 0.23.4 exists, but eggsact's qualified producer baseline is 0.23.3 and M005 MUST preserve 0.23.3. The 0.23.4 linker/target changes are not required for this migration and belong to an independent upgrade review.

Upstream 0.23.3 also publishes prebuilt Linux x86-64/AArch64 binaries with checksums. Consuming those binaries could reduce CI provisioning time and provide exact executable-byte provenance, but changing acquisition mode is deliberately deferred. M005 retains the eggsact-proven `cargo install cargo-zigbuild --version 0.23.3 --locked` path.

A provider policy may later support a verified preinstalled tool mode, but eggsact M001 uses explicit provisioning.

## 7. Zig provisioning

For the configured Zig version and host architecture:

1. derive the official archive name through a finite mapping:
   - Linux x86_64 -> `zig-x86_64-linux-<version>.tar.xz`;
   - Linux AArch64 -> `zig-aarch64-linux-<version>.tar.xz`;
2. download over HTTPS from fixed ziglang.org;
3. use bounded connect/total transfer time;
4. verify exact configured SHA-256 before extraction;
5. validate the archive's expected single top-level layout before or during extraction and reject traversal, symlinked executable, or unexpected layout cases;
6. extract into an invocation-private directory using a deterministic top-level-strip contract;
7. require the resulting `zig` path to be a regular non-symlink executable;
8. bind cargo-zigbuild directly to that verified executable with `CARGO_ZIGBUILD_ZIG_PATH=<private>/zig` rather than relying only on PATH ordering;
9. set `CARGO_ZIGBUILD_CACHE_DIR` to an invocation-private directory so wrapper/cache state cannot leak across unrelated jobs or ambient user state;
10. require exact equality: `$("$CARGO_ZIGBUILD_ZIG_PATH" version) == <configured-version>`;
11. PATH may include the verified Zig directory for direct diagnostic commands, but `CARGO_ZIGBUILD_ZIG_PATH` is the build authority;
12. never use apt/system package Zig.

The current eggsact proving hashes for Zig 0.14.1 are:

- Linux x86_64: `24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c`;
- Linux AArch64: `f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b`.

These are consumer fixture evidence, not a permanent globally selected Zig version.

## 8. Download implementation boundary

Generated GitHub Actions may use a finite rendered curl/tar sequence for provisioning because:

- URLs are fixed by Eggpack;
- version/archive mapping is finite;
- digest is explicit;
- no user-supplied command is rendered;
- this is build-tool bootstrap, not product acquisition.

Requirements:

- `curl --proto '=https' --tlsv1.2 --fail --silent --show-error`;
- bounded connect/total time;
- no credentials;
- no arbitrary redirects unless specifically required and tested for ziglang.org; when redirects are needed, render `--location` together with HTTPS-only redirect policy (for example `--proto-redir '=https'`);
- configured provider-policy SHA-256 is the authority; do not trust a remotely fetched checksum file in place of the checked-in digest;
- SHA-256 verification occurs before extraction;
- extraction destination is private;
- unexpected archive layout fails.

If official Zig hosting requires redirects, allow only a bounded HTTPS-preserving policy and test it; do not use unrestricted provider URLs.

## 9. Runner model

Runner mappings continue to select host OS/architecture labels.

Do not represent mutable ambient tool availability as the qualification authority when explicit provisioning is selected.

A finite mode may preserve historical tests:

- `PreinstalledVerified`;
- `Provisioned`.

For `PreinstalledVerified`, both exact configured versions must still be checked.

For eggsact, use `Provisioned`.

## 10. Generated workflow ordering

CargoZigbuild target job:

```text
checkout exact source
 -> verify source identity
 -> setup Rust target
 -> install/verify cargo-zigbuild
 -> download/hash/extract/verify Zig
 -> export CARGO_ZIGBUILD_ZIG_PATH to the verified private Zig executable
 -> export invocation-private CARGO_ZIGBUILD_CACHE_DIR
 -> cargo zigbuild ... <target>.2.17
 -> capture handoff
```

Tool setup runs before product build and before any candidate handoff.

No provisioning job has write repository permissions.

## 11. Cache policy

Do not make correctness depend on mutable caches.

If caching Zig archive or cargo-installed tool is later added:

- cache key includes exact version + host arch + digest;
- cache hit is reverified before use;
- cache miss follows the same bounded provisioning path.

M005 does not require cross-run caching. It DOES require an invocation-private `CARGO_ZIGBUILD_CACHE_DIR`; that directory is scratch state for the selected cargo-zigbuild invocation, not a reusable CI correctness cache.

## 12. Tests

Required:

- NativeCargo renders no Zig/cargo-zigbuild provisioning;
- CargoZigbuild requires both exact versions;
- x86-64 official archive URL/name mapping;
- AArch64 official archive URL/name mapping;
- configured archive SHA rendered exactly;
- wrong/empty SHA policy rejects;
- no apt/Homebrew/package-manager fallback;
- cargo-zigbuild exact install version rendered;
- exact Zig version equality check rendered against the verified executable path;
- `CARGO_ZIGBUILD_ZIG_PATH` points to that exact verified executable;
- invocation-private `CARGO_ZIGBUILD_CACHE_DIR` is rendered and cannot resolve to an ambient/shared user directory;
- exact cargo-zigbuild version comparison rendered;
- glibc floor command remains unchanged;
- staging/source-identity behavior from M003c unchanged;
- no write permission introduced;
- deterministic render/check;
- historical CargoZigbuild schema-v1 input without Zig remains parseable by the new parser but fails current execution/resolution validation rather than selecting ambient Zig;
- NativeCargo historical fixtures remain behaviorally unchanged;
- renderer output contains no cargo-zigbuild 0.23.4 upgrade and no upstream prebuilt cargo-zigbuild acquisition path.

Add local fixture tests for archive checksum/extraction/path assumptions where possible without network, including wrong digest, extra top-level member/layout, and symlinked `zig` rejection.

## 13. Operational qualification

M005 closure requires ordinary Eggpack hosted CI, but the first real Linux x86-64/AArch64 execution of this provisioning path is also recorded in eggsact M001.

If Eggpack CI lacks an AArch64 Linux hosted runner, record that as an explicit consumer-qualification boundary rather than fabricating local AArch64 evidence.

## 14. Verification

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted Linux stable, Rust 1.89, macOS, and Windows remain green.

## 15. Compatibility review

Apply the compatibility decision from section 4 exactly:

- schema version remains 1 for PackConfig/ReleasePlan/CIPlan during this pre-stabilization migration;
- historical documents missing Zig remain parseable by the new implementation;
- current CargoZigbuild validation rejects missing Zig and never falls back to ambient state;
- NativeCargo documents remain unchanged;
- pre-M005 readers are not promised to understand newly emitted schema-v1 documents carrying the additive Zig field, and closure must state that explicitly.

No DistributionContract/ReleaseManifest change is expected.

## 16. Acceptance criteria

M005 closes only when:

- CargoZigbuild policy names exact Rust, cargo-zigbuild, and Zig versions;
- generated CI can provision exact cargo-zigbuild without ambient dependency;
- generated CI can provision SHA-256-verified official Zig for Linux x86-64 and AArch64;
- exact versions are checked before build;
- cargo-zigbuild is bound to the exact verified Zig path via `CARGO_ZIGBUILD_ZIG_PATH`;
- cargo-zigbuild wrapper/cache state is isolated with invocation-private `CARGO_ZIGBUILD_CACHE_DIR`;
- Zig archive layout/executable type is validated before use;
- glibc floor syntax remains correct;
- NativeCargo behavior is unchanged;
- no arbitrary setup command/URL surface is introduced;
- M003c source-integrity and least-privilege invariants remain green;
- all repository verification passes;
- no unresolved medium-or-higher toolchain reproducibility finding remains.

CI M003d is already closed. Build M005 closure therefore directly unblocks Eggpack Ecosystem M001 / mirrored eggsact M005, which will provide the first real x86-64/AArch64 provisioning runs and the outstanding M003b live-draft/rerun proof.

## 17. Stop conditions

Stop and re-plan if:

- official Zig archives cannot be provisioned without arbitrary URL policy;
- cargo-zigbuild requires an unbounded/ambient installer path;
- exact Zig version must become product-specific shell rather than typed policy;
- provisioning requires weakening HTTPS/digest checks;
- BuildStrategy or Cargo command semantics must materially change;
- a third-party action is required but cannot be immutable-pinned and equivalently verified;
- cargo-zigbuild cannot be made to honor the exact verified Zig executable path without reverting to ambiguous ambient PATH resolution.

## 18. Deferred follow-ups that do not require a new M005 plan

The following are intentionally outside M005 and MUST NOT delay first-consumer adoption:

- evaluating cargo-zigbuild 0.23.4 or later against eggsact after M005 closes;
- optionally replacing `cargo install --locked` with SHA-256-pinned upstream cargo-zigbuild release binaries for CI speed/executable-byte provenance;
- adding independent ELF `GLIBC_*` symbol-floor auditing beyond preserving cargo-zigbuild's `.2.17` target request.

Open separate implementation work only if post-M005 evidence justifies one of these changes.

## 19. Closure evidence

Create:

`plans/closure/build-qualification/005-status.md`

Record:

- implementation SHA;
- schema compatibility decision and historical-input behavior;
- rendered x86-64/AArch64 provisioning snippets;
- `CARGO_ZIGBUILD_ZIG_PATH` and private `CARGO_ZIGBUILD_CACHE_DIR` evidence;
- Zig URL/archive/digest evidence;
- cargo-zigbuild exact-version evidence;
- glibc-floor command evidence;
- local/hosted matrix;
- any AArch64 operational evidence boundary;
- unresolved findings;
- explicit eggsact M001 readiness disposition.
