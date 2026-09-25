# Build and Qualification Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#8-release-planning`
- `plans/000-long-term-specification.md#9-build-and-target-model`
- `plans/000-long-term-specification.md#10-qualification-model`
- `plans/002-long-term-roadmap.md#phase-4--build-and-qualification-planner`
- `plans/002-long-term-roadmap.md#phase-5--local-packaging-and-release-aggregation`

Related ADRs:

- ADR-0001;
- `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`.

## 1. Purpose and ownership boundary

Own provider-neutral producer planning, build strategy selection, qualification classification, product hooks, local staging/finalization, and artifact aggregation.

It consumes portable artifact identity from the contract subsystem and produces finalized artifacts/evidence for the manifest subsystem.

It must not own live installation or public release authority.

## 2. Work classification

### Invariants

- build success != qualification;
- cross-built bytes cannot claim native proof without matching execution evidence;
- product hooks are bounded and explicit;
- artifact identity comes from DistributionContract;
- final hashes occur after byte-changing transformations;
- mixed source revisions/releases cannot aggregate successfully.

### Capabilities

- plan selected targets;
- invoke supported builders/backends;
- qualify on native/deferred/QEMU/structural paths;
- assemble direct/bundle/archive final artifacts;
- emit structured result data.

### Infrastructure

- PackConfig;
- ReleasePlan;
- builder trait/adapters;
- qualifier model;
- owner-private staging;
- artifact collection.

### Polish

- progress UI;
- diagnostics;
- cache optimization.

## 3. Non-goals

- remote build farm;
- arbitrary workflow DSL;
- application-specific test semantics;
- automatic public publication.

## 4. Current state

Current Eggstack repos provide evidence for:

- native Cargo;
- cargo-zigbuild with glibc 2.17 floors;
- native ARM64 hosted runners;
- QEMU ARMv7 smoke;
- Windows ARM64 deferred/native qualification;
- sibling binary bundles;
- archive pairs;
- product-owned smoke scripts.

The external-backend spike closed with disposition C: dist is design prior art only. ADR-0004 now selects a first-party Cargo/cargo-zigbuild adapter boundary for the initial native slice while keeping the adapter replaceable and retaining Eggpack's own identity, qualification, and finalization authority.

## 5. Target architecture

```text
DistributionContract + PackConfig
              |
              v
          ReleasePlan
        /      |      \
 builder   qualifier   finalizer
        \      |      /
          FinalArtifactSet
              |
              v
        manifest builder
```

## 6. Dependency graph

Hard dependencies:

- contract M001/M002;
- release manifest M001 interface;
- external backend M001 disposition (closed C; satisfied, no production adoption).

## 7. Milestones

### M001 — PackConfig and ReleasePlan

Closed: `eggpack-core` now parses strict PackConfig v1 and resolves a deterministic, contract-derived ReleasePlan with provider-neutral host, toolchain, compatibility-floor, qualification, and support intent. Planning performs no build or external effects.

### M002 — Native/cross builder seam

Closed historically: ADR-0004 first-party Cargo/cargo-zigbuild execution has explicit logical-output source bindings, bounded cancellable process-group execution, tool preflight, private build directories, and exact candidate evidence. See `plans/closure/build-qualification/002-status.md`. Post-closure Windows stability concerns are tracked by M002a.

Implementation plan: `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md`.

### M002a — Windows builder qualification stability corrective

Closed: the intermittent Windows builder test is attributable, required cases are visible and cannot skip, MSVC is explicitly initialized, and the timeout/cancellation fixtures run with isolated state. Three first-attempt hosted runs pass on the corrective SHA. See `plans/closure/build-qualification/002a-status.md`. M002 production code is unchanged.

Implementation plan: `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md`.

### M003 — Qualification execution model

Closed: `plans/closure/build-qualification/003-status.md`. M003 binds BuildAttempt to release/source identity, validates exact qualification bindings and candidate inventory, structurally checks ELF/PE/COFF/thin Mach-O, and records bounded native/deferred/QEMU/structural evidence.

### M004 — Finalization and local aggregation

Closed: `plans/closure/build-qualification/004-status.md`. Direct/bundle/archive, final size/hash, mixed-release guards. M003 and Manifest M002 dependencies are closed.

## 8. Cross-cutting requirements

Timeouts/output bounds, explicit tool versions, secret redaction, path safety, cancellation cleanup, deterministic plan output.

## 9. Verification strategy

Synthetic fixture builders first, then native target matrix. Negative tests for wrong architecture/version, missing qualifier, hook timeout, partial artifacts, source mismatch.

## 10. Risks and decision points

The largest risk is over-generalizing CI/build configuration into a DSL. Keep strategies enumerated and hooks narrow.

## 11. Completion definition

Eggpack can construct and qualify real direct/bundle/archive native releases locally or via accepted backend with accurate evidence classification.

## 12. Milestone status

M001 PackConfig and ReleasePlan is closed. Release Manifest M001/M001a and Contract M001/M002 are closed. Backend-evaluation evidence is closed with disposition C, which does not authorize a backend.

| Milestone | Status | Implementation plan | Closure record | Blockers / sequencing |
|---|---|---|---|---|
| M001 PackConfig + ReleasePlan | closed | `plans/implementation/build-qualification/001-pack-config-and-release-plan.md` | `plans/closure/build-qualification/001-status.md` | implementation and hosted CI passed |
| M002 native/cross builder seam | closed (historical) | `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md` | `plans/closure/build-qualification/002-status.md` | post-closure Windows stability finding tracked by M002a |
| M002a Windows qualification stability corrective | closed | `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md` | `plans/closure/build-qualification/002a-status.md` | three repeated hosted Windows stability runs passed |
| M003 qualification execution | closed | `plans/implementation/build-qualification/003-qualification-execution-and-evidence.md` | `plans/closure/build-qualification/003-status.md` | local verification and hosted matrix passed |
| M004 finalization/aggregation | closed | `plans/implementation/build-qualification/004-finalization-and-local-aggregation.md` | `plans/closure/build-qualification/004-status.md` | Local and hosted verification passed |
