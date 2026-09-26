# Build and Qualification Milestone 005 — Deterministic Cross-Tool Provisioning

Status: ready for handoff

Repository baseline: `0397de46183fdbc5c430ab473548da1265a4d758`

Source roadmaps:

- `plans/subsystems/build-qualification-roadmap.md`
- `plans/subsystems/ecosystem-adoption-roadmap.md`

First proving consumer:

- `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`

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
- select a new external build backend.

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

If backward compatibility requires a schema transition, preserve parsing of historical fixtures explicitly rather than treating missing Zig as an exact toolchain.

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

## 6. cargo-zigbuild provisioning

For CargoZigbuild jobs:

1. install the exact configured version with:
   `cargo install cargo-zigbuild --version <version> --locked`;
2. use an invocation-private Cargo install root or another deterministic runner-local location;
3. add only its bin directory to PATH;
4. require exact `cargo zigbuild --version` output;
5. bound install time.

Do not use an unversioned ambient cargo-zigbuild even if one happens to exist.

A provider policy may later support a verified preinstalled tool mode, but eggsact M001 uses explicit provisioning.

## 7. Zig provisioning

For the configured Zig version and host architecture:

1. derive the official archive name through a finite mapping:
   - Linux x86_64 -> `zig-x86_64-linux-<version>.tar.xz`;
   - Linux AArch64 -> `zig-aarch64-linux-<version>.tar.xz`;
2. download over HTTPS from fixed ziglang.org;
3. use bounded connect/total transfer time;
4. verify exact configured SHA-256 before extraction;
5. extract into an invocation-private directory using a deterministic top-level-strip contract;
6. add only that exact directory to PATH;
7. require `zig version` equals the configured Zig version;
8. never use apt/system package Zig.

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
- no arbitrary redirects unless specifically required and tested for ziglang.org;
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

M005 does not require caching.

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
- exact `zig version` comparison rendered;
- exact cargo-zigbuild version comparison rendered;
- glibc floor command remains unchanged;
- staging/source-identity behavior from M003c unchanged;
- no write permission introduced;
- deterministic render/check.

Add local fixture tests for archive checksum/extraction/path assumptions where possible without network.

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

Document the schema effect of adding Zig version/provisioning mode.

Do not silently make old CargoZigbuild configs mean "any Zig".

Historical closed plan fixtures may remain readable only under an explicitly historical parser path if necessary.

No DistributionContract/ReleaseManifest change is expected.

## 16. Acceptance criteria

M005 closes only when:

- CargoZigbuild policy names exact Rust, cargo-zigbuild, and Zig versions;
- generated CI can provision exact cargo-zigbuild without ambient dependency;
- generated CI can provision SHA-256-verified official Zig for Linux x86-64 and AArch64;
- exact versions are checked before build;
- glibc floor syntax remains correct;
- NativeCargo behavior is unchanged;
- no arbitrary setup command/URL surface is introduced;
- M003c source-integrity and least-privilege invariants remain green;
- all repository verification passes;
- no unresolved medium-or-higher toolchain reproducibility finding remains.

Eggpack Ecosystem M001 remains blocked until both Build M005 and CI M003d close.

## 17. Stop conditions

Stop and re-plan if:

- official Zig archives cannot be provisioned without arbitrary URL policy;
- cargo-zigbuild requires an unbounded/ambient installer path;
- exact Zig version must become product-specific shell rather than typed policy;
- provisioning requires weakening HTTPS/digest checks;
- BuildStrategy or Cargo command semantics must materially change;
- a third-party action is required but cannot be immutable-pinned and equivalently verified.

## 18. Closure evidence

Create:

`plans/closure/build-qualification/005-status.md`

Record:

- implementation SHA;
- schema compatibility decision;
- rendered x86-64/AArch64 provisioning snippets;
- Zig URL/archive/digest evidence;
- cargo-zigbuild exact-version evidence;
- glibc-floor command evidence;
- local/hosted matrix;
- any AArch64 operational evidence boundary;
- unresolved findings;
- explicit eggsact M001 readiness disposition.
