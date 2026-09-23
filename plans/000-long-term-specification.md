# Eggpack Long-Term Architecture and Product Specification

Status: canonical long-term implementation directive

Companion documents:

- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`

This document defines the intended end state for Eggpack. The keywords MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, and MAY are normative.

## 1. Product definition

Eggpack is a Rust-native developer-side distribution and release-construction system for Eggstack applications and independently reusable software projects.

Eggpack owns the producer side of software distribution:

- declarative distribution contracts;
- target/build/qualification planning;
- artifact composition;
- release-inventory conformance;
- checksums and concrete release manifests;
- bootstrap-installer generation or validation;
- CI/release-plan generation;
- staging and draft-release assembly;
- optional provenance/attestation generation;
- adapters for specialized package ecosystems when justified.

Eggpack does not own live installation/update transactions on user machines. Eggup remains the consumer-side deployment substrate.

The durable split is:

```text
source repository
      |
      v
   Eggpack
contract -> plan -> build -> qualify -> package -> manifest
      |                                      |
      +---------- release assets ------------+
                                             |
                                      release hosting
                                             |
                                             v
                                  application / Eggup
                         resolve -> acquire -> verify
                         -> install/update/rollback
```

## 2. Primary goals

Eggpack MUST:

1. Eliminate duplicated target matrices, asset naming, checksum logic, release aggregation, installer mapping, and workflow guards across Eggstack repositories.
2. Preserve one deterministic portable distribution contract for direct, bundle, and archive release layouts.
3. Produce a concrete machine-readable release manifest describing the bytes actually emitted by one release.
4. Keep producer build/qualification policy separate from consumer install/update policy.
5. Make release correctness testable locally without requiring publication or public network access.
6. Generate or validate readable bootstrap installers without turning shell/PowerShell into independent policy authorities.
7. Support native and cross-built Rust artifacts across Linux, macOS, and Windows, including SBC-relevant ARM targets when consumers require them.
8. Permit application-specific qualification hooks without embedding application semantics into Eggpack.
9. Keep publication deliberately gated; ordinary build/qualification MUST NOT imply public release.
10. Integrate cleanly with Eggup without making Eggup depend on build-time tooling.
11. Reuse proven external tooling such as `dist` when evidence shows it reduces machinery without weakening Eggpack contracts.
12. Leave room for provenance, attestations, signatures, SBOMs, and future trust policies without misrepresenting checksums as authenticity.

## 3. Non-goals

Eggpack is not:

- a package manager for end users;
- a live updater;
- an OS deployment transaction engine;
- a service manager;
- a fleet deployment orchestrator;
- a general-purpose CI system;
- a replacement for GitHub Actions;
- a general shell/workflow DSL;
- a source-code build farm;
- a crates.io/PyPI/Homebrew registry replacement;
- a mandatory publisher of releases;
- a requirement that all products support the same target set;
- a reason to move consumer policy from applications into Eggpack.

## 4. Producer/consumer ownership rule

The central architectural rule is:

**Eggpack owns release production and release evidence. Eggup owns local deployment mechanism. Applications own release-selection and installation policy.**

Eggpack MUST NOT:

- stop/start services on an end-user host as part of normal self-update;
- decide that a user should upgrade to a particular version;
- own rollback of a live installation;
- silently choose package-manager/source-build fallback;
- create destination ownership rules for installed applications.

Eggup MUST NOT be required to:

- run compilers;
- construct release archives;
- generate CI;
- publish assets;
- understand build runners;
- interpret Eggpack build policy.

## 5. Layer model

The intended implementation is layered.

### 5.1 eggpack-contract

A small deterministic library owning the portable distribution contract:

- schema versioning;
- product identity;
- canonical target triples and aliases;
- direct/bundle/archive artifact forms;
- asset/install/checksum name templates;
- strict template grammar;
- namespace collision checks;
- archive-member path validation;
- deterministic expansion.

The initial implementation MUST preserve the currently qualified `eggup-dist` schema-v1 semantics before adding Eggpack-specific features.

It MUST NOT perform network I/O, run subprocesses, extract archives, or publish releases.

### 5.2 eggpack-manifest

A small library owning the concrete release-manifest format.

A release manifest describes what actually exists for one resolved release. It SHOULD include:

- schema version;
- product identity;
- release identifier/version as opaque data;
- source revision when available;
- canonical target identity;
- logical artifact/member identity;
- final asset filenames;
- install names;
- final byte sizes;
- final SHA-256 digests;
- archive member mappings where applicable;
- qualification result references;
- provenance/attestation references when produced.

The manifest MUST describe finalized bytes. Hashes MUST be computed after transformations such as archive assembly or code signing that change artifact bytes.

The manifest MUST NOT itself decide release authority, version ordering, or whether an installation is allowed.

### 5.3 eggpack-core

Producer-side planning and conformance machinery:

- loading producer configuration and portable contracts;
- release-plan construction;
- target/build/qualification graph resolution;
- release and archive inventory validation;
- deterministic aggregation;
- artifact finalization;
- manifest production;
- installer/CI generation inputs;
- structured diagnostics/events.

Core logic SHOULD remain host-neutral where practical and MUST separate pure planning from subprocess/network adapters.

### 5.4 eggpack-cli

The operator surface:

- `check`;
- `plan`;
- `package`;
- `qualify`;
- `collect` / release aggregation;
- `manifest`;
- `installer generate/check`;
- `ci generate/check`;
- staging/draft-release commands when later enabled.

CLI commands MUST distinguish validation, build, staging, and publication. A command that mutates a public release MUST be explicit.

### 5.5 adapters

Specialized adapters MAY exist for:

- GitHub Releases;
- `dist`;
- Cargo/cargo-zigbuild;
- maturin/Python wheels;
- code signing;
- SBOM/provenance providers.

Adapters MUST consume stable Eggpack domain contracts. Their external types MUST NOT become canonical Eggpack identities.

## 6. Portable distribution contract

Eggpack begins with the proven DistributionContract v1 behavior from `eggup-dist`.

The portable contract owns:

- product id;
- target triples;
- target aliases;
- release asset names;
- checksum sidecar names;
- install names;
- direct/bundle/archive layout;
- required archive-member source paths.

It deliberately does not own:

- CI runner;
- compiler/toolchain selection;
- GitHub repository URL;
- release selection;
- service configuration;
- install root;
- privilege policy;
- fallback policy;
- signing standard.

Producer-only build policy MAY be expressed in a separate versioned Eggpack configuration that references portable target identities. Producer configuration MUST NOT redefine asset names or create a second target identity authority.

## 7. Artifact forms

V1 supports three fundamental artifact forms:

```text
direct
  one release asset -> one installed member

bundle
  several release assets -> one logical release

archive
  one release archive -> several required installed members
```

This model MUST remain intentionally small. A generic recursive package language is a non-goal until real consumers prove it necessary.

## 8. Release planning

A ReleasePlan is invocation-specific producer intent resolved from checked-in configuration.

It MAY contain:

- source revision;
- release identifier;
- selected target set;
- builder strategy;
- toolchain/version requirements;
- CI runner requirements;
- qualification strategy;
- artifact form;
- staging paths;
- required aggregation gates.

Planning MUST be deterministic for identical repository/configuration inputs.

Planning MUST NOT publish, download arbitrary remote code, or mutate public release state.

## 9. Build and target model

Eggpack MUST support a target model capable of representing current Eggstack evidence, including:

- Rust target triple;
- OS and architecture;
- libc family where relevant;
- minimum compatibility/deployment floor;
- build strategy such as native Cargo or cargo-zigbuild;
- build runner class;
- qualification mode;
- support tier;
- artifact form.

Initial important target families include:

- x86_64 Linux GNU;
- aarch64 Linux GNU;
- ARMv7 Linux GNU where a consumer supports it;
- x86_64 macOS;
- aarch64 macOS;
- x86_64 Windows MSVC;
- aarch64 Windows MSVC where a consumer supports it.

Musl and Python-wheel platform families are expected later but MUST NOT complicate the first native-binary slice unnecessarily.

Products remain free to support a subset.

## 10. Qualification model

Build success is not release qualification.

Eggpack MUST be able to represent:

- native execution qualification;
- cross-build with deferred native qualification;
- QEMU/container qualification;
- structural-only qualification when execution is impossible;
- product-owned bounded smoke hooks.

A target MUST NOT claim native qualification merely because compilation succeeded on another architecture.

Product hooks MUST be explicit, bounded in duration/output, and invoked with controlled environment handling. Eggpack MUST NOT parse arbitrary application semantics.

## 11. Artifact finalization and checksums

The finalization order MUST be explicit.

Conceptually:

```text
build bytes
  -> product qualification as appropriate
  -> packaging/archive assembly
  -> signing/notarization if configured
  -> final qualification where byte transformation requires it
  -> final size/hash
  -> release manifest
  -> aggregation
```

SHA-256 is an integrity mechanism, not publisher authenticity.

Checksum sidecars MAY remain part of the portable contract for bootstrap compatibility even when a concrete release manifest also embeds hashes.

## 12. Concrete release manifest

The concrete release manifest is the principal Eggpack-to-consumer interoperability artifact.

It MUST:

- be schema-versioned;
- be deterministic;
- identify one product and one release;
- describe finalized emitted assets;
- bind each logical artifact/member to exact size and digest;
- preserve canonical target identity;
- represent direct, bundle, and archive releases;
- reject ambiguous duplicate/case-colliding names;
- avoid secrets;
- remain usable without Eggpack's build engine.

It SHOULD be small enough for runtime consumers such as an optional future `eggup-eggpack` adapter.

The initial serialization SHOULD be JSON because it is machine-produced and widely consumable. Canonical/deterministic serialization rules MUST be documented before signatures over manifest bytes are introduced.

## 13. Bootstrap installers

Bootstrap installers exist before application-owned Eggup code is available.

Eggpack SHOULD generate or validate POSIX shell and PowerShell bootstrap installers from authoritative contract/configuration data.

Generated installers MUST:

- resolve supported OS/architecture deterministically;
- fail closed on unsupported targets;
- use fixed or validated release origins supplied by product policy;
- stage privately;
- bound network operations;
- verify final artifact integrity before execution/installation;
- validate candidate identity/version when practical;
- avoid implicit privilege escalation;
- preserve an existing installation on verification failure;
- support logical bundles atomically enough to avoid obvious mixed-version bootstrap states;
- remain readable and testable.

Normal in-process self-update SHOULD use Eggup rather than re-running the bootstrap installer.

A future installer MAY emit an Eggup-compatible installation receipt, but the receipt schema remains owned by Eggup.

## 14. CI and workflow generation

Eggpack SHOULD make checked-in generated CI the default model.

The intended interface is conceptually:

```text
eggpack ci generate
eggpack ci check
```

Generated workflows MUST be reviewable and deterministic from checked-in configuration. They SHOULD pin third-party action revisions according to repository policy.

Eggpack MAY use reusable workflows internally, but a remote mutable workflow MUST NOT become the hidden policy authority for a repository release.

CI generation MUST separate:

- preflight;
- build;
- qualification;
- aggregation;
- provenance;
- staging/draft release;
- publication.

## 15. Publication policy

Building and qualifying do not imply publishing.

Initial release automation SHOULD support staging/draft assembly while keeping final publication a deliberate maintainer action.

Eggpack MUST NOT automatically publish crates.io/PyPI packages merely because artifacts qualified. Registry publication remains explicit and separately authorized.

A previously published immutable release MUST NOT be silently overwritten.

## 16. Eggup interoperability

Eggpack and Eggup MUST interoperate through narrow data contracts, not mutual build/runtime dependencies.

Expected future shape:

```text
eggpack-manifest
        ^
        |
  eggup-eggpack
        |
        v
eggup acquisition/core
```

`eggup-core` MUST NOT depend on Eggpack.

An optional Eggup adapter may:

- parse/validate an Eggpack release manifest;
- select the exact current-platform manifest entry after consumer policy resolves the release;
- construct Eggup artifact/acquisition inputs;
- pass expected digests/sizes and install names;
- retain bundle/archive identity.

It MUST NOT let the manifest decide whether an update is allowed.

## 17. External backend policy

Eggpack MUST evaluate existing release tooling before reimplementing generic machinery.

As of the initial planning baseline, `axodotdev/cargo-dist` / `dist` 0.33.0 is an active Rust release system that already emits release manifests, installers, checksums, CI, and attestations.

Eggpack MUST perform a bounded interoperability/capability spike before building a large native build engine.

Possible outcomes include:

- use `dist` as a backend for suitable simple releases;
- normalize `dist` output into Eggpack manifests;
- reuse only selected concepts;
- reject it for particular requirements and implement native machinery.

No production dependency is accepted until evidence is recorded and an ADR selects the boundary.

Eggpack MUST NOT adopt an external updater in place of Eggup merely because a packaging backend offers one.

## 18. Supply-chain provenance

Eggpack distinguishes:

- integrity — bytes match an expected digest;
- provenance — evidence of where/how bytes were built;
- authenticity — evidence that a trusted publisher authorized an artifact/manifest;
- safety — a separate judgment not proven by the preceding evidence.

GitHub Artifact Attestations are a candidate provenance mechanism for GitHub-hosted releases. SBOM attestations MAY be supported.

A signing/authenticity standard requires a dedicated ADR before runtime enforcement is introduced.

The preferred long-term signing unit is the release manifest or a manifest digest that transitively binds all release assets.

## 19. Security model

Eggpack MUST account for:

- malicious or malformed configuration;
- unsafe archive paths;
- filename collisions;
- symlink/path traversal in staging;
- oversized manifests/inventories;
- unbounded subprocess output;
- hung product hooks;
- secrets in environment or URLs;
- mutable tool downloads;
- accidental artifact replacement;
- mixing artifacts from different commits/releases;
- hashing pre-finalized rather than final bytes;
- cross-build artifacts falsely labeled as qualified;
- workflow/configuration drift;
- untrusted third-party actions;
- compromised build dependencies.

Core parsers and planners SHOULD forbid unsafe code unless explicitly justified.

Supply-chain incidents in build dependencies reinforce that release tooling itself must keep dependencies proportionate and auditable.

## 20. Determinism and reproducibility

Eggpack MUST produce deterministic plans, expected inventories, and manifest serialization for equivalent inputs.

It SHOULD support reproducibility evidence where practical, but byte-for-byte reproducible builds are not a universal v1 requirement because platform signing/notarization and toolchains may introduce nondeterminism.

The release manifest MUST record enough source/build identity to distinguish releases even when reproducibility is not available.

## 21. Observability

Operations SHOULD expose structured progress/results suitable for CI and JSON consumers.

Errors and findings MUST have stable machine-readable categories.

Logs/events MUST NOT leak tokens, credentials, or unbounded command output.

## 22. Dependency and footprint policy

The repository baseline is Rust 1.89 unless an accepted ADR changes it.

`eggpack-contract` and `eggpack-manifest` SHOULD remain small, deterministic libraries suitable for downstream reuse.

Heavy build, network, archive, GitHub, or signing dependencies MUST stay outside leaf schema crates.

External command/tool dependencies MUST be explicit and version-constrained where release reproducibility/security requires it.

## 23. Consumer adoption requirement

No abstraction is mature solely because Eggpack's own tests pass.

At least two independent consumers MUST adopt each broadly reusable contract before it is considered stable.

Initial expected progression:

1. eggsact;
2. stegoeggo;
3. eggsearch;
4. Gregg;
5. CodeGG;
6. Egress;
7. Eggserve/Python-wheel work where justified.

## 24. Testing requirements

Eggpack MUST eventually provide deterministic evidence for:

- contract parse/expand and collision handling;
- release inventory conformance;
- archive member conformance;
- manifest round-trip and deterministic ordering;
- direct/bundle/archive fixtures;
- unsupported target failure;
- malformed config/path rejection;
- native/deferred/QEMU qualification classification;
- partial artifact collection;
- cross-release mixing rejection;
- final hash/size correctness;
- installer target mapping;
- installer checksum mismatch;
- CI generation drift;
- publication dry-run/staging behavior;
- external backend normalization when enabled.

Network-independent local tests MUST cover core correctness.

## 25. Release philosophy

Eggpack's own publication SHOULD remain conservative.

Ordinary CI proves correctness. Publication is a separate maintainer action.

Crates intended for external consumption MUST pass `cargo package` / `cargo publish --dry-run` qualification before their package boundary is considered stable.

## 26. Completion definition

Eggpack reaches its intended 1.0 architecture when:

- the portable contract has migrated out of Eggup cleanly;
- release conformance and concrete manifests are stable;
- native release planning/build/qualification is proven or deliberately delegated to an accepted backend;
- bootstrap installers and checked-in CI derive from authoritative configuration;
- Eggup consumes manifests through an optional narrow adapter without core coupling;
- direct, bundle, and archive consumers are proven;
- producer publication remains explicitly gated;
- security-sensitive defaults are documented;
- no known high-severity correctness or supply-chain defect remains.
