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
- future backend-selection ADR.

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

The external-backend spike closed with disposition C: dist is design prior art only. No backend is selected; a later build plan must keep the backend replaceable and retain Eggpack's own identity, qualification, and finalization authority.

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

Represent build/runner/qualification policy without redefining artifact identity.

### M002 — Native/cross builder seam

Implement selected backend/native strategy.

### M003 — Qualification execution model

Native/deferred/QEMU/structural + bounded hooks.

### M004 — Finalization and local aggregation

Direct/bundle/archive, final size/hash, mixed-release guards.

## 8. Cross-cutting requirements

Timeouts/output bounds, explicit tool versions, secret redaction, path safety, cancellation cleanup, deterministic plan output.

## 9. Verification strategy

Synthetic fixture builders first, then native target matrix. Negative tests for wrong architecture/version, missing qualifier, hook timeout, partial artifacts, source mismatch.

## 10. Risks and decision points

The largest risk is over-generalizing CI/build configuration into a DSL. Keep strategies enumerated and hooks narrow.

## 11. Completion definition

Eggpack can construct and qualify real direct/bundle/archive native releases locally or via accepted backend with accurate evidence classification.

## 12. Milestone status

M001 PackConfig and ReleasePlan is ready. Release Manifest M001 and corrective M001a are closed, so build planning can rely on the corrected schema-v1 boundary. Contract M001/M002 and backend-evaluation evidence are closed; disposition C does not authorize a backend.
