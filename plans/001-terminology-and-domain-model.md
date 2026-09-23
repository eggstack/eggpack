# Eggpack Terminology and Domain Model

Status: canonical normative terminology

This document defines the terms used by Eggpack plans, APIs, configuration, manifests, tests, and downstream integrations. Where older Eggup distribution documents use overlapping terminology, this document governs Eggpack once migration occurs.

## 1. Product roles

**Producer** — the source repository/maintainer/CI context that constructs a release.

**Consumer** — software or a user-side installer/updater that consumes already-produced release artifacts.

**Eggpack** — producer-side release construction and evidence machinery.

**Eggup** — consumer-side verified local deployment machinery.

**Application policy** — product-owned decisions such as authoritative release source, allowed versions/channels, source fallback, install roots, service configuration, and migration behavior.

## 2. Product and release identity

**ProductId** — bounded opaque stable identifier for one distributed product. It is not a repository URL, crate name, executable basename, or display name, although these may coincide.

**ReleaseId** — opaque identifier for one producer release. A SemVer string is common but not mandatory.

**SourceRevision** — immutable source identifier, normally a Git commit SHA, associated with a ReleasePlan/ReleaseManifest when available.

A ReleaseId MUST NOT be treated as an ordering language by generic Eggpack code unless an explicit adapter owns that version semantics.

## 3. Target identity

**TargetTriple** — canonical build/runtime target identifier, initially Rust target-triple shaped where applicable.

**TargetAlias** — alternate bounded identifier mapped unambiguously to one TargetTriple.

**PlatformFamily** — descriptive grouping such as Linux GNU AArch64, not a stable identity.

**BuildHost** — environment that produces bytes.

**QualificationHost** — environment in which runtime qualification executes.

BuildHost and QualificationHost may differ.

## 4. Distribution Contract

A **DistributionContract** is portable release-layout intent.

It answers:

- which targets/aliases exist;
- which release filenames correspond to them;
- which install names result;
- whether the release is direct, bundle, or archive;
- which checksum sidecar names are expected;
- which archive members are required.

It does not answer:

- which runner builds the target;
- which version should ship;
- whether an update should occur;
- where the product is installed;
- which GitHub repository is trusted;
- whether Cargo fallback is allowed.

The initial DistributionContract schema is inherited from `eggup-dist` schema v1 and MUST retain its strict template/path/collision semantics during migration.

## 5. Producer configuration

**PackConfig** — Eggpack-specific producer policy/configuration used to construct ReleasePlans.

PackConfig may specify builder strategy, toolchain constraints, qualification strategy, CI runner class, staging behavior, and generation options.

PackConfig references DistributionContract target identities and MUST NOT redefine artifact names, install names, or target identity in a competing authority.

## 6. Artifact forms

**DirectArtifact** — one release file corresponds to one installed member.

**BundleArtifactSet** — multiple independently released files form one logical release unit.

**ArchiveArtifact** — one release archive contains multiple required members.

**ArtifactSet** — the logical collection that must be internally version-consistent for one target/release. This term is compatible conceptually with Eggup's multi-artifact transaction input but does not imply the same Rust type.

**ArchiveMember** — required relative path inside an archive plus its logical/install mapping.

## 7. Release Plan

A **ReleasePlan** is a resolved producer-side plan for one invocation.

It combines:

- DistributionContract;
- PackConfig;
- selected ReleaseId;
- SourceRevision;
- selected targets;
- builder/qualification strategy;
- required aggregation gates.

A ReleasePlan is intent. It is not evidence that any artifact exists.

## 8. Build and qualification

**BuildAttempt** — one attempt to produce candidate bytes for a planned target/artifact.

**CandidateArtifact** — built bytes not yet accepted as final release bytes.

**Qualification** — evidence-producing checks proving the candidate/final artifact meets declared release criteria.

**NativeQualification** — execution on a matching supported native host.

**DeferredNativeQualification** — cross-build first, then execution on a separate matching native host.

**EmulatedQualification** — execution through QEMU/container/emulation explicitly recorded as such.

**StructuralQualification** — non-execution inspection only; MUST NOT be labeled native runtime proof.

**ProductHook** — bounded product-owned command/script invoked by Eggpack for semantic release checks.

## 9. Finalization

**Finalization** — transformations after build that establish the exact bytes intended for distribution, including archive assembly, code signing, or notarization as applicable.

**FinalArtifact** — immutable bytes after all byte-changing release transformations.

Digests and manifest sizes describe FinalArtifacts, not pre-finalization candidates.

## 10. Release inventory

**ExpectedReleaseInventory** — deterministic filename set derived from the DistributionContract for a concrete target/release.

**ObservedReleaseInventory** — bounded caller-supplied filename set being checked.

**ConformanceReport** — deterministic structured findings comparing expected and observed state.

**ExtrasPolicy** — small policy controlling whether unrelated additional release files are allowed or rejected. It is not a filtering DSL.

## 11. Release Manifest

A **ReleaseManifest** is concrete producer evidence for one completed release.

It binds ProductId + ReleaseId + target/artifact identities to exact final filenames, sizes, digests, and relevant evidence references.

It is distinct from DistributionContract:

```text
DistributionContract = what should exist
ReleasePlan          = what this invocation intends to make
ReleaseManifest      = what was actually finalized and accepted
Eggup receipt        = what was actually installed on one machine
```

A ReleaseManifest MUST NOT claim that an artifact is installed or trusted merely because it exists.

## 12. Integrity, provenance, authenticity, safety

**Integrity** — final bytes match an expected digest.

**Provenance** — evidence linking bytes to source/build process.

**Authenticity** — evidence that a configured trusted publisher authorized the artifact/manifest.

**Safety** — broader security/correctness property not proven merely by integrity/provenance/authenticity.

A SHA-256 sidecar is integrity metadata, not independent authenticity.

## 13. Bootstrap installer

A **BootstrapInstaller** is a pre-application installer script/program used when native Eggup-based application update machinery is not yet available.

It derives target/artifact mapping from Eggpack authority but may contain product-supplied origin/install policy.

It is not the normal self-update engine after installation.

## 14. CI plan and workflow

**CIPlan** — provider-neutral release graph derived from ReleasePlan/PackConfig.

**GeneratedWorkflow** — checked-in CI provider representation, initially GitHub Actions.

The generated workflow is a projection. The checked-in Eggpack configuration/contract remains authority.

## 15. Staging and publication

**LocalStage** — owner-controlled filesystem area containing candidate/final assets before publication.

**ReleaseStage** — remote non-public or draft area used to assemble a release.

**Publication** — transition making release/package artifacts publicly available/authoritative according to product policy.

Build, qualification, aggregation, staging, and publication are separate operations.

## 16. Receipts

**Eggpack Evidence Record** — producer-side record about build/qualification/release outputs.

**Eggup Install Receipt** — consumer-side record of an installed artifact set. Owned by Eggup.

Eggpack MAY generate data sufficient for an Eggup-compatible bootstrap receipt later, but MUST NOT redefine Eggup receipt semantics.

## 17. External backend

**Backend** — an external or internal implementation of selected producer operations.

**dist Backend** — possible adapter using `dist`/cargo-dist for supported build/package/install-script/CI operations.

**Normalization** — conversion from backend-specific outputs into canonical Eggpack ReleaseManifest/conformance types.

External backend types are never canonical Eggpack identity.

## 18. Status language

Planning status uses:

- proposed;
- ready;
- active;
- blocked;
- closing;
- closed;
- conditionally closed;
- superseded;
- archived.

Artifact/qualification status MUST use domain-specific types rather than reusing planning status strings.
