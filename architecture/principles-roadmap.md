# Cross-Cutting Concerns — Deep Dive

Domain model, roadmap, ADRs, tooling, Eggup boundary, and external-backend
evaluation. Canonical sources live under `plans/`; this file summarizes them
for review.

## 1. Domain model / terminology

Sources: `plans/001-terminology-and-domain-model.md` (normative),
`plans/000-long-term-specification.md`, `plans/README.md`.

- **Roles:** `Producer` (source repo/maintainer/CI) → `Eggpack` (producer-side
  construction + evidence) → `Consumer` installer → `Eggup` (consumer-side
  verified deployment). `Application policy` (product-owned): authoritative
  source, versions/channels, fallback, install roots, services, migrations.
- **Split:** Eggpack owns release production + evidence; Eggup owns local
  deployment; applications own release-selection/install policy.
- **Four objects:**
  `DistributionContract` (what should exist — portable layout intent) /
  `ReleasePlan` (what this invocation intends — intent, not evidence) /
  `ReleaseManifest` (what was finalized — concrete final-bytes evidence) /
  `Eggup receipt` (what was installed on one machine — Eggup-owned).
- **Key terms:** opaque `ProductId` / `ReleaseId` (SemVer common, no ordering
  unless adapter-owned); `SourceRevision` (immutable, normally Git SHA);
  canonical `TargetTriple` + unambiguous `TargetAlias`; `PackConfig` (producer
  policy — references contract identities, must not redefine names);
  `Direct` (1 file → 1 member) / `Bundle` (N files → 1 unit) / `Archive`
  (1 archive → N members); `BuildAttempt` / `CandidateArtifact` (unaccepted
  bytes); `Qualification`: `Native | DeferredNative | Emulated` (QEMU,
  explicitly labeled) `| Structural` (inspection only, never native proof) `|
  ProductHook` (bounded); `Finalization → FinalArtifact` (digests describe final
  bytes only); `Expected` vs `Observed` inventory → `ConformanceReport` +
  `ExtrasPolicy` (`AllowExtras`/`Exact`, not a DSL); `BootstrapInstaller`
  (pre-app first-install, not a self-update engine); `CIPlan` →
  `GeneratedWorkflow` (config/contract stays authority); `LocalStage` /
  `ReleaseStage` (remote draft) / `Publication` (explicit public transition).
- **Invariants:** determinism; integrity ≠ authenticity ≠ provenance ≠ safety;
  sidecar SHA-256 = integrity only; publication gated; registry publication
  separately authorized.
- **Layers:** `eggpack-contract` → `eggpack-manifest` → `eggpack-core`
  (host-neutral planning vs adapters) → `eggpack-cli` (`check/plan/package/
  qualify/collect/manifest/installer/ci` + staging/draft) → adapters (GitHub,
  Cargo/zigbuild, signing, SBOM).

## 2. Roadmap / milestones

Canonical: `plans/002-long-term-roadmap.md`, `plans/registry.md` (control
surface), `plans/003-planning-process.md`, `plans/subsystems/*-roadmap.md`.

- **Phase 0** planning/ownership; **Phase 1** bootstrap + Contract v1 migration
  (Rust 1.89, direct/bundle/archive fixtures); **Phase 2** conformance engine;
  **Phase E1** parallel external `dist` 0.33.x spike; **Phase 3** ReleaseManifest
  v1; **Phase 4** build/qualification planner; **Phase 5** local packaging;
  **Phase 6** bootstrap generation; **Phase 7** checked-in CI; **Phase 8**
  staging + human publication gate; **Phase 9** simple adoption (eggsact,
  stegoeggo); **Phase 10** Eggup interop; **Phase 11** broader adoption
  (eggsearch, Gregg, CodeGG, Egress, wheels); **Phase 12** provenance/
  authenticity; **Phase 13** specialized adapters; **Phase 14** 1.0 stabilization.
- **Subsystem status:** Contract M001/M002 closed, M003 planned; Manifest
  M001/M001a/M002 closed, M003 planned; Build M001/M002/M002a/M003/M004 closed,
  **M005 deterministic cross-tool provisioning (`005-…md`) ready** (exact Zig
  0.14.1 / cargo-zigbuild 0.23.3, first-consumer prerequisite); Bootstrap
  M001/M002/M002a closed, M003 blocked on adoption evidence; CI M001/M002/M002a/
  M003a/M003c/M003d closed, M003b conditionally closed (live-draft proof
  outstanding, sequenced after Build M005 + eggsact); Eggup M001/M001a + Eggup
  adapter closed, adoption ready; Ecosystem eggsact M001 blocked/planned
  (baseline `174764c`, waits Build M005, then performs M003b live-draft proof);
  External-backend evaluation closed (disposition C, below).

## 3. ADRs (`plans/adrs/`, all accepted)

- **ADR-0001 producer/consumer boundary (2026-09-22):** separate Eggpack
  producer vs Eggup consumer. Neither core depends on the other's machinery.
  `eggup-dist` frozen after Eggup M003; Eggpack M002 qualified equivalence;
  Eggup M004 retired the duplicate.
- **ADR-0002 contract/plan/manifest separation:** four distinct objects +
  producer-only PackConfig. Manifest = finalized bytes only; plan ≠ evidence;
  manifest ≠ install claim.
- **ADR-0003 checked-in generated CI + publication gate:** deterministic
  checked-in workflows by default, `eggpack ci generate/check`, drift-detected,
  least-privilege (read build vs write staging), no silent immutable overwrite.
  Reusable workflows may do leaf ops, must not be hidden authority.
- **ADR-0004 first-party native Cargo build adapter (2026-09-24):** initial
  native slice uses first-party `NativeCargo` + `CargoZigbuild`, not `dist`
  (raw-direct vs archive, bundle, evidence, publication mismatches) nor a
  generic shell DSL. Plan-derived intent + explicit logical-output→package/bin
  bindings, direct exec, `--locked`, preflight, private target dir, candidates
  only.

## 4. Tooling

- `scripts/check-local.sh`: `fmt --check`; `check/clippy/test/doc` workspace;
  `cargo tree` + `cargo package` per crate (intra-workspace path patches);
  MSRV lane `cargo +1.89.0 check/test`.
- `.github/workflows/ci.yml` (`contents: read`): `linux` job
  (ubuntu × stable/1.89.0: fmt/check/test/clippy/doc); `portability` job
  (macOS/Windows: stable, MSVC setup, focused Windows tests — cargo fixture
  build, process-group kill, single-threaded core, `eggpack-ci`,
  `m002a_powershell_archive_runtime`, orchestration round-trips).
- Root `Cargo.toml`: resolver 2; 7 members
  (`eggpack-{contract,manifest,core,bootstrap,ci,github,cli}`); edition 2021;
  `rust-version = 1.89`; MIT; `unsafe_code = deny`; clippy `all = warn`.
- **Closure/archive discipline:** `plans/closure/<subsystem>/NNN-status.md`
  records status/baseline/commits/requirements/evidence/verification/invariants/
  compat/security/docs/unresolved/disposition; compilation ≠ closure; history
  never rewritten — correctives are new `NNNa` plans. `plans/archive/` retains
  superseded interim planning.

## 5. Eggup producer/consumer boundary

Normative: `plans/000-long-term-specification.md §§4,16`, ADR-0001,
`architecture/eggup-manifest-consumer-v1.md` (interface note, not a wire
format), `plans/subsystems/eggup-interoperability-roadmap.md`.

- Eggpack must not stop/start services, decide upgrades, own rollback/fallback,
  or infer ownership; Eggup must not require compilers, archive construction,
  CI generation, publishing, or build-policy understanding.
- Interop shape: `eggpack-manifest → eggup-eggpack` (optional adapter in Eggup)
  `→ eggup acquisition/core`; `eggup-core` must not depend on Eggpack.
- Mapping: opaque ids, no ordering; caller selects exactly one canonical triple
  (no alias/fallback); origin/root from consumer policy. Direct = 1 URL →
  1 member; bundle = 1 unit per entry → one `ArtifactSet`, reject partial/mixed;
  archive = 1 acquisition + retained per-member facts, qualified consumer
  extraction required (seam undefined). Size exact (max then exact check);
  SHA-256 integrity only; `install` flat target-local identity.
- Fixtures: `plans/closure/eggup-interoperability/fixtures/`
  (`direct/bundle/archive-manifest.json` + `projection-*.json` doc/test data +
  negatives); contract fixtures in `crates/eggpack-contract/tests/fixtures/`.
- History: `eggup-dist` frozen/removed (Eggup M004) after Eggpack Contract M002
  qualified M003 behavior; Eggpack now sole producer authority.

## 6. External backend evaluation (`dist` spike, disposition C)

Sources: `plans/subsystems/external-backend-evaluation-roadmap.md` (closed),
`plans/implementation/external-backend-evaluation/001-…spike.md`,
`plans/closure/external-backend-evaluation/001-status.md`.

- Baseline `axodotdev/cargo-dist` / `dist` v0.33.0: manifests/installers/
  checksums/attestations demonstrated. **Disposition C — design prior art only,
  no production backend adopted.**
- Mismatches: direct fixture emits executable-zip, not raw direct; multi-package
  yields separate releases/archives, not one sibling bundle; manifest lacks byte
  sizes, source revision, qualification evidence; generated tag workflow
  auto-publishes; broad write perms; shell checksum may skip; glibc/macOS floors
  unproven/unrepresented; updater must stay disabled (Eggup ownership).
- Normalization direction (separately approved experiment only): treat dist
  output as observations, map via Eggpack contract, independently validate final
  files; never import dist types as canonical API, never use its workflow as
  authority. Production adoption needs a new evidence plan + ADR.
