# Build and Qualification Milestone 002 — Native/Cross Builder Execution Seam

Status: blocked at closure — implementation landed; Windows native execution qualification is blocked by the hosted runner toolchain

Repository baseline: `e8bc338ab193fa8ed5fc7debb0cd68b54ebf586b`

Source roadmap: `plans/subsystems/build-qualification-roadmap.md`

Required ADR: `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`

Long-term references:

- `plans/000-long-term-specification.md#5.3-eggpack-core`
- `plans/000-long-term-specification.md#5.5-adapters`
- `plans/000-long-term-specification.md#8-release-planning`
- `plans/000-long-term-specification.md#9-build-and-target-model`
- `plans/001-terminology-and-domain-model.md#8-build-and-qualification`

Primary class: capability / infrastructure / process execution

## 1. Objective

Implement Eggpack's first executable builder seam for the already-modeled `NativeCargo` and `CargoZigbuild` strategies.

M002 turns one resolved `ReleasePlan` target plus explicit producer build-source bindings into bounded `BuildAttempt` / `CandidateArtifact` evidence. It executes only Cargo/cargo-zigbuild, never a shell or arbitrary command language.

The milestone stops at candidate bytes. It does not claim qualification, create final release filenames/checksum sidecars, assemble archives, build a ReleaseManifest, generate CI, or publish.

## 2. Why this milestone is ready

Hard dependencies are closed:

- Contract M001/M002;
- Manifest M001/M001a/M002;
- Build/Qualification M001 PackConfig + ReleasePlan;
- external-backend spike disposition C;
- ADR-0004 selects the initial first-party Cargo/cargo-zigbuild adapter boundary and explicitly rejects a generic command backend.

The remaining execution detail—mapping logical contract output slots to Cargo package/bin sources—is producer adapter input, not portable release identity.

## 3. Current evidence

`ReleasePlan` already carries:

- canonical target;
- NativeCargo versus CargoZigbuild strategy;
- exact Rust toolchain requirement;
- exact cargo-zigbuild version when selected;
- compatibility floor;
- host requirement;
- qualification intent;
- support tier;
- contract-derived direct/bundle/archive form.

It intentionally does not encode Cargo workspace package/bin names.

DistributionContract remains the authority for release filenames/install identities. M002 must not infer Cargo sources from those names.

## 4. Invariants

- no shell invocation;
- no arbitrary command/plugin DSL;
- no final artifact naming authority outside DistributionContract;
- no build success => qualification claim;
- no tool auto-install/update;
- exact configured toolchain/tool versions are checked;
- all process output and runtime are bounded;
- build work occurs in caller-approved owner-private directories;
- candidate discovery is explicit from package/bin bindings, never glob/nearest-name guessing;
- symlinks/non-regular candidate outputs fail;
- one target build cannot silently consume output from another target or release invocation;
- command/env diagnostics redact secrets and do not dump arbitrary environment state;
- candidate evidence is deterministic for identical declared inputs, excluding inherently variable compiler output/log text;
- no network client is added to Eggpack; Cargo may use its configured registry/network behavior, but Eggpack does not claim offline/reproducible builds from that fact alone.

## 5. Scope

### In scope

- first-party Cargo builder adapter;
- first-party cargo-zigbuild adapter;
- explicit versioned producer-side build bindings;
- logical contract-slot -> Cargo package/bin source mapping;
- deterministic command specs;
- process runner with deadlines/output bounds/cancellation cleanup;
- tool preflight;
- private Cargo target directory;
- candidate artifact discovery/validation;
- structured BuildAttempt/CandidateArtifact results;
- direct releases as required closure path;
- bundle/archive logical source binding representation where all required members are Cargo binaries, without final assembly;
- synthetic/fake-process tests plus real host Cargo smoke tests;
- Rust 1.89/macOS/Windows hosted qualification.

### Out of scope

- qualification execution;
- product hooks;
- archive assembly;
- code signing/notarization;
- checksum sidecars;
- final release filenames;
- final hashing/ManifestBuilder invocation;
- GitHub Actions;
- public release staging;
- generic external backends;
- source-build fallback in consumers.

## 6. Required production model

### A. Build-source bindings

Add a strict versioned producer-side adapter document/type, e.g. `BuildBindingsV1`.

It must reference contract logical slots, not final filenames.

Use a typed logical selector equivalent to:

- `Direct`;
- `BundleEntry { index }`;
- `ArchiveMember { source }`.

Each binding maps one logical slot to an explicit Cargo source:

- package name;
- binary target name.

Requirements:

- selected target must have a complete, exact binding set for every Cargo-produced logical slot M002 is asked to build;
- duplicate selectors fail;
- unknown/out-of-range bundle indices fail against the resolved contract;
- unknown archive member sources fail;
- bindings do not contain asset/install/checksum names;
- package/bin identifiers are bounded and validated;
- bindings are deterministic/strict with unknown fields rejected.

M002 may reject layouts containing required non-Cargo-generated members rather than inventing a generic file/command source. That keeps CodeGG's generated JSON/runfile complexity for later evidence-driven work.

### B. CommandSpec

Define a pure command-description layer before execution.

A command spec includes:

- executable;
- ordered argv;
- working directory;
- filtered environment additions/overrides;
- deadline;
- stdout/stderr byte limits;
- private Cargo target directory.

It must never contain shell source.

NativeCargo command intent:

- exact configured Rust toolchain;
- `cargo build`;
- release mode;
- `--locked`;
- explicit target triple;
- exact package;
- exact bin.

CargoZigbuild command intent:

- exact configured Rust toolchain;
- `cargo zigbuild`;
- release mode;
- `--locked`;
- explicit package/bin;
- explicit target;
- GNU glibc floor encoded using cargo-zigbuild's supported target-floor syntax when configured.

Do not add arbitrary extra args in M002.

### C. Tool preflight

Before an accepted build attempt:

- confirm configured Rust toolchain is available;
- confirm Cargo reports the selected toolchain;
- for CargoZigbuild, confirm cargo-zigbuild is present and its reported version equals configured intent;
- confirm Zig is available when the selected strategy requires it;
- do not install any missing tool;
- bound all version-command output.

A version mismatch is a typed failure before candidate success.

### D. Process runner

Implement a narrow process execution abstraction suitable for deterministic tests and a real local runner.

Requirements:

- direct executable spawn only;
- bounded stdout/stderr;
- deadline enforcement;
- child termination + wait on timeout/cancellation;
- no successful result if process cleanup is incomplete;
- explicit exit status;
- no raw environment dump;
- caller-visible summaries remain bounded/redacted.

If reliable child-tree termination cannot be achieved on supported platforms with proportionate dependencies, stop and re-plan rather than weakening the timeout invariant.

### E. Private build workspace

Caller supplies:

- absolute repository/workspace root;
- absolute owner-private Eggpack work root.

M002 creates target-specific subdirectories below the work root.

Set `CARGO_TARGET_DIR` to that private location. Do not depend on or overwrite the repository's ordinary `target/`.

Reject:

- relative roots;
- symlink work root where canonical ownership cannot be established safely;
- work paths escaping the supplied root.

M002 does not clean arbitrary pre-existing caller paths. It owns only subdirectories it creates.

### F. Candidate discovery

After successful process exit, derive the expected Cargo target output path from:

- private Cargo target dir;
- canonical target;
- release profile;
- explicit binary target name;
- platform executable suffix rules.

Do not scan recursively or choose "closest" files.

Require candidate output to be:

- present;
- regular;
- non-symlink;
- non-empty.

Copy or hard-link into an Eggpack-owned candidate area only if the operation preserves bytes and ownership semantics; otherwise return the validated Cargo output path as candidate evidence. Do not rename to the final DistributionContract asset name in M002.

### G. BuildAttempt / CandidateArtifact

Add bounded structured evidence containing at minimum:

- canonical target;
- logical output selector;
- strategy;
- package/bin source identity;
- tool/version summary;
- command outcome classification;
- candidate local path handle/path;
- candidate byte size;
- bounded diagnostic/log references or summaries.

Do not put final digest/manifest claims here unless needed strictly as diagnostic evidence; final-release hashing remains M004/ManifestBuilder territory.

No `Qualification::Native` result is emitted by M002.

## 7. Ordered work packages

1. Add strict build-binding types/parser/validation.
2. Add logical-slot validation against DistributionContract + ReleasePlan.
3. Add pure NativeCargo command rendering.
4. Add pure CargoZigbuild command rendering, including GNU glibc-floor case.
5. Add tool preflight.
6. Add bounded process runner with timeout/output cleanup semantics.
7. Add private target/work-root handling.
8. Add exact candidate discovery and BuildAttempt/CandidateArtifact result model.
9. Add fake-runner negative matrix.
10. Add real local native Cargo fixture build.
11. Add cargo-zigbuild integration coverage where CI environment provides the tool; otherwise qualify command rendering + explicit environment limitation without claiming execution proof.
12. Update docs/roadmap/registry and write closure.

## 8. Failure/restart/contention semantics

A failed/timeout/cancelled attempt:

- returns no successful CandidateArtifact;
- kills/waits for owned process execution;
- preserves no claim of qualification;
- does not modify public release state;
- may leave only the owned private work directory, documented for retry/inspection.

Retries are explicit caller actions. M002 performs no automatic retry.

Concurrent attempts must use distinct invocation/target work directories. Refuse reuse if ownership/identity metadata does not match the same release/source/target invocation.

## 9. Compatibility

No DistributionContract or ReleaseManifest wire change.

ReleasePlan remains intent.

BuildBindings is producer-side adapter configuration and is not a consumer/public release contract.

No dist/cargo-dist runtime dependency.

Existing repository build workflows remain untouched by this milestone.

## 10. Required tests

At minimum:

- strict BuildBindings parse/unknown-field/version rejection;
- direct slot complete binding;
- duplicate/unknown slot rejection;
- bundle index/source validation;
- archive-member source validation;
- non-Cargo-required-member rejection where unsupported;
- deterministic command spec;
- no shell;
- NativeCargo exact package/bin/target/locked/release args;
- CargoZigbuild exact package/bin/target/locked/release args;
- glibc floor encoded only for compatible GNU target;
- no macOS/glibc misuse;
- missing Rust toolchain;
- mismatched cargo-zigbuild version;
- missing Zig;
- non-zero exit;
- timeout;
- stdout overflow;
- stderr overflow;
- cleanup failure cannot report success;
- relative/escaping work root rejection;
- missing candidate;
- symlink candidate;
- directory candidate;
- empty candidate;
- candidate from wrong target path rejected;
- no final release filename/checksum/manifest/publication production;
- real simple direct Cargo fixture succeeds on host;
- unchanged planning/manifest/contract suites.

## 11. Verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Record hosted Linux stable, Linux Rust 1.89, macOS, and Windows separately.

If cargo-zigbuild/Zig are not installed in the standard Eggpack CI image, record that operational limitation and require exact command/preflight regressions. Do not auto-install tools merely to satisfy M002 closure unless a separate CI/tool provisioning policy is explicitly approved.

## 12. Documentation updates

Update:

- `crates/eggpack-core/README.md`;
- root README producer capability section;
- build/qualification roadmap;
- CI roadmap only if builder result shape changes its assumptions;
- registry;
- closure `plans/closure/build-qualification/002-status.md`.

Document clearly that candidate bytes are not final/qualified release bytes.

## 13. Acceptance criteria

M002 closes when:

- first-party NativeCargo execution is implemented and qualified;
- CargoZigbuild command/preflight semantics are implemented and qualified to the available environment;
- logical output sources are explicit and never inferred from release filenames;
- process execution is bounded and shell-free;
- tool mismatch/missing tools fail closed;
- candidate discovery is exact and safe;
- no qualification/finalization/publication authority leaks into the builder;
- stable/MSRV/macOS/Windows CI passes, including Windows native execution and process-tree timeout/cancellation evidence;
- no unresolved medium-or-higher builder safety/correctness issue remains.

Closure must state whether M003 qualification execution is now ready to plan.

Closure disposition (2026-09-24): implementation and Linux stable, Linux Rust 1.89, and macOS qualification evidence are recorded in `plans/closure/build-qualification/002-status.md`. Windows hosted CI is blocked because `windows-latest` does not expose an MSVC `link.exe` in PATH (the only discovered `link.exe` is Git's utility); both the real Cargo smoke and Cargo-backed timeout/cancellation fixture consequently fail before exercising the intended code. The implementation has not weakened the invariant or skipped those tests. M002 therefore remains blocked rather than closed, and M003 and CI M001 remain blocked pending an approved/provisioned Windows builder environment and successful Windows rerun.

## 14. Stop conditions

Stop/re-plan if:

- implementation needs a generic shell/plugin language;
- final artifact identity must be duplicated in build bindings;
- safe process-tree timeout cleanup cannot be provided;
- Cargo/cargo-zigbuild cannot expose exact candidate outputs without heuristic scanning;
- PackConfig/ReleasePlan public semantics must materially change;
- an external backend becomes required rather than optional;
- code signing/finalization is needed to define builder success.

## 15. Closure evidence required

Record:

- implementation SHA;
- ADR-0004 baseline;
- public/internal type inventory;
- build-binding examples;
- exact NativeCargo/CargoZigbuild command specs;
- tool preflight evidence;
- timeout/output-bound matrix;
- candidate discovery matrix;
- real host build evidence;
- cargo-zigbuild operational evidence/limitations;
- dependency tree;
- package/MSRV/docs/hosted CI;
- unresolved findings;
- explicit M003 readiness disposition.
