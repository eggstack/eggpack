# Eggpack Long-Term Roadmap

Status: active canonical roadmap

This roadmap decomposes the long-term specification into dependency-ordered capability phases. It does not authorize implementation by itself; dependency-ready work requires subsystem roadmaps and bounded implementation plans.

## Guiding sequence

Eggpack should first preserve and relocate the already-qualified release-layout contract, then make conformance measurable, then introduce concrete release evidence. Only after those contracts are stable should it grow a larger build/CI engine.

```text
Foundation
   |
   +--> preserve/import DistributionContract v1
   |        |
   |        v
   |    conformance validators
   |        |
   |        v
   |    concrete ReleaseManifest
   |        |
   |        +--> Eggup adapter
   |        |
   |        +--> bootstrap generation
   |
   +--> external backend evaluation (parallel)
            |
            v
      build/qualification architecture
            |
            v
       CI generation/staging
            |
            v
        consumer adoption
            |
            v
   provenance + specialized adapters
```

## Phase 0 — Planning and ownership foundation

Objective: establish the Eggpack/Eggup boundary, canonical terminology, planning process, and migration constraints.

Exit criteria:

- canonical specification/terminology/roadmap exist;
- producer/consumer ADR accepted;
- contract-vs-manifest ADR accepted;
- CI/publication ADR accepted;
- initial subsystem roadmaps and implementation registry exist.

## Phase 1 — Repository bootstrap and DistributionContract v1 migration

Objective: create the Rust workspace and move the unpublished `eggup-dist` schema/expansion functionality into Eggpack without semantic drift. Eggup's later closed M003 conformance implementation is migrated in Phase 2.

Deliverables:

- Rust 1.89 workspace;
- `eggpack-contract`;
- copied/adapted schema-v1 implementation;
- simple direct, CodeGG bundle, and Egress archive fixtures;
- strict template grammar/collision behavior;
- package/MSRV/CI evidence;
- provenance note pointing to Eggup source/closure SHAs.

Important constraint: Eggup's copy MUST NOT be removed until both Eggpack Contract M001 and Contract M002 independently qualify the predecessor schema/conformance behavior. The Eggup-side cleanup plan already exists as distribution M004 and remains blocked until Contract M002 closure.

Exit criteria: all predecessor schema/expansion properties through Eggup distribution M002 are reproduced in Eggpack Contract M001.

## Phase 2 — Contract conformance engine

Objective: port and independently qualify the already-closed Eggup distribution M003 release/archive/mapping validators so Eggpack becomes the sole active conformance authority.

Deliverables:

- expected release-file model;
- ReleaseInventory;
- ArchiveMemberInventory;
- runtime/bootstrap mapping observations;
- bounded deterministic ConformanceReport;
- AllowExtras/Exact policy;
- golden direct/bundle/archive fixtures.

Exit criteria: drift among contract, release inventories, and consumer mappings is machine-detectable without network/source parsing; the closed Eggup M003 API/fixture behavior is accounted for; Eggup distribution M004 retirement becomes unblocked.

## Phase E1 — External `dist` capability/interoperability spike

This phase may execute in parallel with Phases 1-2.

Objective: determine whether `dist` 0.33.x can safely serve as an Eggpack backend for any part of the build/package/install/CI pipeline.

Required scenarios:

- eggsact/stegoeggo-like direct binary;
- Gregg-like sibling bundle;
- Egress-like archive pair;
- ARM/SBC target handling;
- glibc floor control;
- deferred/native/QEMU qualification representation;
- checked-in CI generation;
- manifest normalization;
- installer compatibility;
- GitHub Artifact Attestations;
- disabling/avoiding external updater ownership.

Exit criteria: evidence-backed disposition and ADR before a large native build engine is committed.

## Phase 3 — Concrete ReleaseManifest v1

Objective: create the stable producer-to-consumer artifact format.

Deliverables:

- `eggpack-manifest`;
- deterministic JSON schema;
- ProductId/ReleaseId/SourceRevision;
- target/artifact/member entries;
- final byte size and SHA-256;
- qualification/provenance references;
- direct/bundle/archive fixtures;
- canonical ordering/serialization rules sufficient for hashing.

Exit criteria: a completed local fixture release can produce and round-trip an unambiguous manifest independent of build backend.

## Phase 4 — Build and qualification planner

Objective: derive provider-neutral ReleasePlans/CIPlans from portable contract plus producer configuration.

Deliverables:

- PackConfig v1;
- native Cargo and cargo-zigbuild strategies or accepted backend equivalent;
- explicit libc/deployment floors;
- native/deferred/QEMU/structural qualification model;
- bounded product hooks;
- toolchain requirement diagnostics;
- structured plan output.

Exit criteria: current eggsact/stegoeggo/eggsearch target matrices can be represented without duplicating artifact identity.

## Phase 5 — Local packaging and release aggregation

Objective: turn qualified target outputs into exact final release sets.

Deliverables:

- owner-private staging;
- deterministic asset naming from contract;
- bundle/archive assembly;
- final-size/final-digest calculation;
- cross-target aggregation;
- mixed-source/release rejection;
- manifest production;
- local release verification.

Exit criteria: direct, bundle, and archive releases can be assembled locally with no public network dependency.

## Phase 6 — Bootstrap installer generation/conformance

Objective: remove duplicated target/checksum mapping from shell/PowerShell bootstrap installers.

Deliverables:

- POSIX shell generator or deterministic checked fragments;
- PowerShell generator or deterministic checked fragments;
- target mapping from contract;
- bounded downloads;
- checksum verification;
- candidate identity hooks;
- local fake-release tests;
- install policy injection points.

Exit criteria: two simple consumers use one contract for runtime/release/bootstrap mapping.

## Phase 7 — Checked-in CI generation

Objective: replace hand-maintained release target matrices/workflow invariants with generated reviewable CI.

Deliverables:

- provider-neutral CIPlan;
- GitHub Actions generator;
- `eggpack ci generate`;
- `eggpack ci check`;
- stable action/tool pin policy;
- qualification gates;
- aggregate artifact/manifest job;
- local static drift tests.

Exit criteria: generated workflow changes are deterministic and reviewable; manual edits are detected.

## Phase 8 — Release staging and human publication gate

Objective: support safe draft assembly without turning qualification into automatic publication.

Deliverables:

- GitHub Releases staging adapter or accepted backend;
- exact tag/source preflight;
- reject public immutable overwrite;
- upload complete artifact set + manifest + installers;
- draft/staging status output;
- explicit final publication boundary.

Exit criteria: a maintainer can create a fully qualified draft release and inspect it before public publication.

## Phase 9 — Simple consumer adoption

Objective: prove Eggpack on uncomplicated real products.

Initial targets:

1. eggsact;
2. stegoeggo.

Remove duplicated:

- target matrices;
- asset/checksum naming;
- release-set validation;
- installer mapping;
- release workflow boilerplate where replaced.

Exit criteria: both ship/qualify through Eggpack contracts without weakening current release behavior.

## Phase 10 — Eggup interoperability

Objective: prove the producer/consumer seam after producer authority has already moved to Eggpack. This phase is manifest consumption, not migration of `eggup-dist`.

Deliverables:

- optional `eggup-eggpack` adapter plan/implementation in Eggup;
- manifest parse/validation;
- current-platform artifact-set resolution;
- size/digest/install-name translation;
- no `eggup-core` dependency on build tooling;
- end-to-end self-update fixture from Eggpack manifest.

Exit criteria: Eggup can consume an Eggpack-built release without hard-coded duplicate asset mapping.

## Phase 11 — Broader native adoption

Objective: prove complex release forms and target strategies.

Expected sequence:

- eggsearch — ARMv7/Windows ARM64/qualification diversity;
- Gregg — two-binary bundle and service-oriented consumer;
- CodeGG — multi-runfile bundle;
- Egress — archive containing `eggress` + `pproxy`.

Exit criteria: direct, bundle, and archive forms are all proven by real consumers.

## Phase 12 — Provenance and authenticity

Objective: bind release evidence to build identity and later publisher trust.

Potential work:

- GitHub Artifact Attestations;
- SBOM generation/attestation;
- manifest attestation;
- detached signatures/pinned keys;
- offline verification;
- key rotation;
- rollback/freeze metadata considerations.

Authenticity enforcement requires a dedicated ADR.

## Phase 13 — Specialized package adapters

Objective: reuse Eggpack's planning/evidence machinery for ecosystems beyond native release binaries where it reduces duplication.

Candidates:

- maturin/Python wheel matrices for Eggserve/Egress;
- Cargo package topology/dry-run sequencing;
- Homebrew/package metadata generation.

Registry publication remains explicit and separately authorized.

## Phase 14 — Public API/format stabilization

Objective: stabilize toward 1.0.

Deliverables:

- schema compatibility policy;
- manifest compatibility policy;
- SemVer/public crate policy;
- MSRV policy;
- migration guide;
- feature/dependency matrix;
- fuzz/property testing;
- consumer compatibility tests.

Exit criteria: at least two independent consumers rely on each stabilized public contract and no known high-severity correctness/security issue remains.
