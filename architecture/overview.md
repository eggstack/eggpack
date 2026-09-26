# Eggpack Architecture Overview

Bird's-eye view of the Eggpack codebase: producer-side release construction and
distribution infrastructure for Eggstack. Eggpack centralizes portable release
contracts, target/build/qualification planning, artifact composition, release
manifests, bootstrap installers, and generated release CI.

It deliberately does **not** replace Eggup: Eggup remains the consumer-side
verified installation/update/rollback layer.

## Module map

```text
DistributionContract (eggpack-contract)          <- portable expected layout, single authority
        |
PackConfig -> ReleasePlan (eggpack-core)         <- invocation intent (pure, sorted, deterministic)
        |
BuildBindingsV1 -> cargo/zigbuild -> BuildAttempt (eggpack-core/builder.rs)
        |
QualificationBindingsV1 -> qualify_target -> QualificationEvidence (eggpack-core/qualification.rs)
        |
finalize_release -> FinalizedRelease + ReleaseManifest v1 (eggpack-core/finalization.rs, eggpack-manifest)
        |
render_posix/powershell (eggpack-bootstrap)     <- first-install scripts from contract + manifest
        |
CIPlan -> ReleaseCIPlanV1 -> generated release.yml (eggpack-ci)
        |
StagingPayloadV1 -> GitHub draft (eggpack-github)  <- draft-only, exact tag, no publication
        |
eggpack CLI (eggpack-cli)                        <- deterministic wiring for all of the above
```

## Crates

| Crate | Role | Deep dive |
|---|---|---|
| `eggpack-contract` | Portable schema-v1 release layout authority + pure conformance validators. Sync, side-effect free. No build/publish/network. | [contract.md](contract.md) |
| `eggpack-manifest` | Bounded schema-v1 JSON evidence for one finalized release (product/release/revision + exact size/SHA-256 per artifact). Leaf parser/serializer, no I/O. | [manifest.md](manifest.md) |
| `eggpack-core` | Pure PackConfig/ReleasePlan resolution, explicit Cargo build bindings, bounded candidate production, qualification evidence (native/deferred/QEMU/structural), producer-side finalization + manifest construction. | [core.md](core.md) |
| `eggpack-bootstrap` | Renders release-specific direct first-install shell + PowerShell scripts from contract + manifest. No release selection, no updates. SHA-256 = integrity, not authenticity. | [bootstrap.md](bootstrap.md) |
| `eggpack-ci` | Projects release plans + Cargo bindings into a provider-neutral CI graph, renders deterministic GitHub Actions workflows from caller-supplied runner/pin policy, checks drift, wires qualification gates + aggregate/finalize + draft staging + consumer seam. | [ci.md](ci.md) |
| `eggpack-github` | Local staging payload materializer + GitHub draft adapter (draft-only, exact existing tag, no publication). | [github.md](github.md) |
| `eggpack-cli` | Deterministic `eggpack` binary: `ci generate` / `ci check` (exact + reusable shape modes) plus narrow internal runner commands (`_verify-source`, `_resolve-release`, `_capture-build`, `_qualify-target`, `_validate-consumer`, `_evaluate-gate`, `_aggregate`, `_prepare-stage`, `_stage-github-draft`). | [cli.md](cli.md) |

## Cross-cutting concerns

| Topic | Deep dive |
|---|---|
| Domain model, terminology, four-object split (Contract / Plan / Manifest / Receipt) | [principles-roadmap.md](principles-roadmap.md) |
| Roadmap phases + subsystem milestones (M001–M005, M003a–d) | [principles-roadmap.md](principles-roadmap.md) |
| ADRs 0001–0004 (producer/consumer boundary, object separation, checked-in CI + publication gate, first-party Cargo adapter) | [principles-roadmap.md](principles-roadmap.md) |
| Tooling: workspace lints, `scripts/check-local.sh`, `.github/workflows/ci.yml`, closure/archive discipline | [principles-roadmap.md](principles-roadmap.md) |
| Eggup interop (consumer mapping, fixtures) | [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md), [principles-roadmap.md](principles-roadmap.md) |
| External backend evaluation (`dist` spike, disposition C) | [principles-roadmap.md](principles-roadmap.md) |

## Key invariants (apply everywhere)

- **Authority separation:** Contract owns layout/names; PackConfig owns policy;
  ReleasePlan is intent, not evidence; ReleaseManifest describes final bytes only;
  Eggup receipts describe installed state. No layer redefines another's names.
- **Determinism:** target order preserved/canonical sort, stable serialization,
  sorted inventories/findings, byte-identical renders for identical inputs.
- **Fail-closed validation:** `deny_unknown_fields`, `schema_version == 1`,
  bounded counts/sizes, exact-match resolution (no guessing, no alias fallback
  where identity matters), `AllowExtras` default with opt-in `Exact`.
- **Integrity ≠ authenticity:** SHA-256 + sizes are integrity facts, never trust
  or provenance claims. Signatures/attestations are a separate future phase.
- **Publication is gated:** staging targets drafts only on the exact existing tag;
  public release stays a separate human action. No `--clobber`, no tag mutation,
  no auto-publish, no immutable overwrite.
- **Safety:** `#![forbid(unsafe_code)]` workspace-wide (`unsafe_code = deny`),
  no shell interpretation of generated inputs, bounded process execution with
  timeouts/output limits/cancellation, env allowlists, symlink rejection.

## Data flow (producer pipeline)

1. Author `DistributionContract` (TOML, schema-v1) describing targets, aliases,
   asset names, sidecars, archive members, install names.
2. Author `PackConfig` (policy: builder/toolchain/qualification/support tier) and
   resolve it against the contract into a canonically sorted `ReleasePlan`.
3. Declare `BuildBindingsV1` (logical-output → package/bin) and
   `QualificationBindingsV1` (smoke bindings per target); both validated for
   exact plan coverage.
4. Build via first-party `cargo` / `cargo zigbuild` adapter into a `BuildAttempt`
   (candidate bytes only, identity-bound to release + source revision).
5. Qualify each target (`qualify_target`): format/arch inspection, byte hashing
   before/after, bounded native/deferred/QEMU/structural execution →
   `QualificationEvidence`.
6. Finalize (`finalize_release`): gate required targets on qualification, copy
   direct/bundle candidates under contract names, assemble `.tar.gz` archives,
   write checksum sidecars, aggregate `ReleaseManifest v1` over final bytes.
7. Render bootstrap installers (`eggpack-bootstrap`) and the local staging
   payload (`eggpack-github`), reconcile into a GitHub draft (exact tag).
8. Project the whole flow into checked-in CI (`eggpack-ci` + `eggpack` CLI) with
   drift checking, so CI replays steps 2–7 deterministically.

## Review guide

Each deep-dive file follows the same shape: purpose → key types/functions with
file paths → data/schema details → boundaries (explicit non-goals) →
dependencies/dependents → tools/capabilities. Start with
[contract.md](contract.md) and [manifest.md](manifest.md) (the two leaf
authorities), then [core.md](core.md) (the pipeline), then
[bootstrap.md](bootstrap.md), [ci.md](ci.md), [github.md](github.md),
[cli.md](cli.md), and finally [principles-roadmap.md](principles-roadmap.md)
for governance and milestones.
