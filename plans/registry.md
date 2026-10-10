# Eggpack Active Planning Registry

This file is the compact control surface for active interim planning. Detailed requirements live in canonical documents, subsystem roadmaps, implementation plans, closure records, and Git history.

## Canonical direction

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`

## Status vocabulary

- **proposed** — roadmap/plan exists but is not approved for execution.
- **ready** — hard dependencies/interfaces are satisfied.
- **active** — implementation or closure work is in progress.
- **blocked** — named dependency/evidence prevents progress.
- **closing** — implementation landed; closure evidence being gathered.
- **closed** — closure record accepted.
- **conditionally closed** — implementation substantially complete with named evidence condition outstanding.
- **superseded** — replaced by another document.
- **archived** — retained only for traceability.

## Accepted architecture decisions

| ADR | Decision |
|---|---|
| `ADR-0001-producer-consumer-release-boundary.md` | Eggpack owns producer release construction/evidence; Eggup owns consumer local deployment; product code owns release/install policy. |
| `ADR-0002-contract-plan-manifest-separation.md` | DistributionContract, ReleasePlan, ReleaseManifest, and Eggup InstallReceipt are distinct authorities. |
| `ADR-0003-checked-in-generated-ci-and-publication-gate.md` | Generated CI is checked in/reviewable; release publication remains explicit. |
| `ADR-0004-first-party-native-cargo-build-adapter.md` | Initial native execution uses first-party Cargo/cargo-zigbuild adapters; no dist backend or generic command DSL is authorized. |
| `ADR-0005-native-qualification-for-cross-tool-builds.md` | Build strategy and qualification intent are independent axes; `Qualification::Native` is valid for cross-tool-built candidates only when the effective qualification host matches the target OS/architecture and the exact candidate passes native qualification. |

## Current evidence baselines

### Eggpack

- repository began empty on 2026-09-22; Contract M001 and the dist 0.33 evaluation are closed;
- last reviewed code/implementation baseline before the planning-only producer/consumer reorientation: `e3452263225fa1ea262e03b557f40b395e6a52d8`;
- producer/consumer reorientation baseline: `5793ccecf5139d9b7b250534703ec5ee84fe44b8`; subsequent planning-only corrections preserve that ownership/cutover model;
- Contract M002 implementation: `a36803a7c34cc5b273559520bf99cb2400cc183a`; closure record: `plans/closure/contract-conformance/002-status.md`;
- Release Manifest M001 implementation: `b5df057a0ab8d30664aeccc26aa7944678626930`; historical closure: `plans/closure/release-manifest/001-status.md`;
- Release Manifest M001a implementation: `20a3084bbfbc7fac03b70e0c397f9059186169c8`; closure: `plans/closure/release-manifest/001a-status.md`. Install-name collisions are target-local; release-artifact filename collisions remain manifest-global;
- Eggup interface historical baseline: `99c9040a5d106cfa46a4ba02f9fa0cee8653166c`; Eggpack-side corrective M001a implementation: `8d9264b3c224f3f05a061f4038b0f328b1c5c95e`; closure: `plans/closure/eggup-interoperability/001a-status.md`; hosted CI: `35998756491`; Eggup adapter M001 implementation: `5fbb66853bdad59aaf2bd3c7bb43a43492d0b6ef`; Eggup adapter M001a qualification implementation: `19935ec3610a5238af33a9d4f05a14925ceac25c`; closure: `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001a-status.md`; hosted CI: `36014508645`;
- prior foundation handoffs closed: Manifest M002, Build/Qualification M001, Bootstrap M001, and Eggup Interop M001/M001a;
- Bootstrap M002 + CI M002 historical implementation: `12eb2c8`, `3afa8e1`, final `df0120c`; hosted CI `36142013486`; post-closure correctives registered: CI M002a `86321e1d7485e5e5eed923cbbff9e52140b3c16b`, Bootstrap M002a `074576f4548c68f0d9b8f862e375d5e512c27445`; both M002a correctives implemented at `4d2270afe7de10bdff92563ed0f51a41ba807a04`, qualified by hosted run `36154905956` (attempt 1, all lanes green), and closed via `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md` (closures: `plans/closure/ci-release-orchestration/002a-status.md`, `plans/closure/bootstrap-installers/002a-status.md`);
- operational producer handoffs registered: ADR-0004 `e8bc338ab193fa8ed5fc7debb0cd68b54ebf586b`, Build/Qualification M002 plan `1666df9ac68062f7a1be4ed757a4f1ccc21aad5b`, CI Orchestration M001 plan `b7270df591efd4b6a3b8c8e02af70d3960c72f0f`; post-closure Windows stability corrective M002a plan `59bf3621da94b7b6ae5d64c357dee21bb27b7527`;
- Build/Qualification M005 deterministic cross-tool provisioning is closed at `plans/closure/build-qualification/005-status.md` (implementation `7a206ba`, hosted run 36256831000 green); exact Zig 0.14.1 / cargo-zigbuild 0.23.3 provisioning, verified Zig path binding, and private cargo-zigbuild cache are landed; the handoff baseline was aligned at `86fa474d936b6041a06110b48e8b1c5d12f9ac33` and the roadmap registered at `2a53a43df627269844794c48b28c7a2035f44f15`;
- canonical architecture commit: `2022f44f4df9b0c2518ff22a53d87fd77f63992a`;
- Ecosystem M001 review baseline: `28f3630413c1fa6ae35ca1fdfc404a64b30b3b88`. M001 stopped at its §20 condition before implementation and landed no code, configuration, or workflow change in either repository. The blocking gap was that `Qualification::Native` was inadmissible for a `CargoZigbuild` target (`crates/eggpack-core/src/lib.rs` `validate_policy`; `crates/eggpack-ci/src/lib.rs` `CIPlan::validate`), which is exactly eggsact's two glibc-2.17-floored, natively qualified Linux targets. A candidate five-target configuration was authored in a scratch tree and driven through the real `eggpack ci` renderer: everything resolved except those two targets, then the scratch tree was removed. Repository green at that baseline (`cargo fmt --all -- --check`; `cargo test --workspace --all-targets --all-features --locked` -> 206 passed, 7 ignored). ADR-0005 is accepted with Option A and Build/Qualification M006 is now closed at `plans/closure/build-qualification/006-status.md` (implementation `398cd43`, hosted run 36484758546 green): native qualification is host-matched and independent of the builder, so M001's producer blocker is resolved. Native smoke remains distinct from independent proof of a declared glibc deployment floor; M006 records that residual boundary explicitly;
- eggsact consumer adoption is closed, not pending. The pre-cutover review baseline `174764c5c71130ec98fee18c445fcecb3e35eb25` (later `34aed3ab36da2637c22412f7ca65d35f1ca5021d`) and its "flagged for re-review before implementation" note are historical: the re-review happened, Ecosystem M001 landed and published on `v1.2.7`, and the updater transport moved to the qualified `eggup-eggfetch` / `eggfetch-core` crates. The consumer then closed its own Windows byte-reproducibility line at `eggstack/eggsact@685fa3739562a4c3c670c28e05ee9e09b07cdbd2` (implementation `f1352101dab748066c788e65e21e1bf303cfe995`, closure record `eggstack/eggsact: plans/closure/distribution-update-release/005a-status.md`), evidenced by independent double-build run `36880110434` and full Eggpack-pipeline rehearsal run `36886042696` attempts 1-2.
- Eggpack integration is closed, not pending. The old `current main@404f63e` snapshot and its M003h-as-future framing are historical: M003h is closed, M003a is closed, and `main` is far beyond that snapshot. The durable reference is CI M003h's closure record (`plans/closure/ci-release-orchestration/003h-status.md`), which captures synchronization merge `609d5fb`, full local verification, hosted CI, and the non-forced `main` fast-forward. Planning Hygiene M001 deliberately re-pointed this bullet at closure records instead of a new ephemeral `current main` SHA, which would rot the same way.

### Consumer live release evidence

Both adopted consumers have now completed a real public release through the Eggpack-generated pipeline. This is the evidence that closes Ecosystem M002 and the CI M003b rerun-reuse condition.

- eggsact: producer cutover published on `v1.2.7` (live run `36652731202`, exact 15-asset draft, maintainer-published). Consumer Windows byte reproducibility closed at `eggstack/eggsact@685fa3739562a4c3c670c28e05ee9e09b07cdbd2`; pipeline rehearsal run `36886042696` attempt 1 created draft `401132612` (`created: true, uploaded: 15, reused: 0`) and attempt 2 reused the same draft with `created: false, uploaded: 0, reused: 15`, identical digests, zero digest-mismatch refusals. The fix was entirely product-side (target-scoped MSVC `/BREPRO` + `/DEBUG:NONE` in a checked-in `.cargo/config.toml`); no Eggpack production change was made or required, and the generated workflow stayed byte-identical under `eggpack ci check` (drift guard run `36880067370`).
- stegoeggo: producer cutover at `3b96fae`, consumer closure `c75132a`, updater corrective M003 closed at `f80eebe3`. The live ordinary stable release landed as **`v0.5.0`** — not the `0.4.3` earlier planning anticipated. Consumer stopped `0.4.3` pre-publication on semver findings (`eggstack/stegoeggo@0e5235d3`, "plans: block 0.4.3 release on root API semver findings"), selected `0.5.0` as the next `0.x` minor boundary, and closed it in Release-Distribution M005 (`eggstack/stegoeggo@7bde933b`, record `plans/closure/release-distribution/005-status.md`), which marks M004 superseded and the consumer subsystem closed. Independently verified for this registry: release `v0.5.0` published 2026-10-04T06:34:19Z, non-draft/non-prerelease; lightweight tag `v0.5.0` -> `57ca94c910269e080b06aa8cb34c3767b3bf669d`; published `release-manifest.json` `schema_version: 1`, `release_id: v0.5.0`, `source_revision: 57ca94c910269e080b06aa8cb34c3767b3bf669d` (exact tag match); Eggpack run `37181914252` `workflow_dispatch` `head_sha` `57ca94c9…` conclusion `success`; 15 published assets; `stegoeggo-stego`/`stegoeggo`/`stegoeggo-cli` all at `0.5.0` on crates.io. Publication was manual, after inspection.
- Bootstrap M003 independently re-verified the live install surface for both consumers on 2026-10-05: each release's `install-exact.sh` / `install-exact.ps1` carries exactly the five published direct targets, and every embedded artifact name, install name, exact size, and SHA-256 equals that release's own published `release-manifest.json` in both installer forms; all 20 artifact/sidecar URLs resolve HTTP 200 with sizes equal to the manifest, and both exact installers also resolve at `latest/download`. No consumer hand-writes a digest. Details in `plans/closure/bootstrap-installers/003-status.md`.
- Not exercised on the stegoeggo release: exact rerun reuse (run `37181914252` was attempt 1 only, `uploaded: 15, reused: 0`) and the Windows A→B updater transition (macOS x86-64 satisfied the one-supported-target requirement). The rerun-reuse gap is substituted by the eggsact rehearsal above, which exercises the same producer path; see `plans/closure/ecosystem-adoption/002-status.md` §9.3 for the explicit reasoning.

### Eggup predecessor

Eggup's current planning assigns producer distribution authority to Eggpack. The relevant immutable predecessor is distribution M003, not current-main incidental state.

Distribution predecessor evidence:

- M001 implementation: `889a234cbe7f461d92def3df45c83c06a7d257e5`;
- M002 strict template/collision corrective: `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`;
- M003 conformance implementation: `9941c58d7039410c728860f9e4e382881d4ccf54`;
- M003 closure: `4169c8021b447fe73c8ee3ea71a80a535c940f54`;
- `eggup-dist` was unpublished predecessor evidence and has now been removed from Eggup;
- Eggup distribution M004 implementation `bc25885bd41b86bfdf2f32d1e42856e00829cd7a` is closed at `plans/closure/distribution-bootstrap/004-status.md` in `eggstack/eggup`;
- Contract M002 ported/qualified the closed M003 behavior before that retirement, so Eggpack is now the sole active producer distribution authority.

### External backend evidence

- `axodotdev/cargo-dist` / `dist` latest reviewed release: 0.33.0 (September 2026);
- upstream demonstrates generated release machinery, installers, manifest, checksums, and GitHub Artifact Attestations;
- no external production backend is adopted; ADR-0004 selects Eggpack-owned first-party Cargo/cargo-zigbuild adapters for the initial native execution slice while keeping the adapter boundary replaceable.

## New implementation handoff — 2026-10-08

- **Bootstrap M002b CLOSED:** `plans/closure/bootstrap-installers/002b-status.md`; implementation `88ddf2e796ac3674b306b51726c65ed7cc9bd29a`, hosted run `37806244313` green across Linux stable/MSRV, macOS, Windows.
- **CI M003i CLOSED:** `plans/closure/ci-release-orchestration/003i-status.md`; implementation `3ddac9d84fc2e10b8451d38672f66b02e06724df`, hosted run `37813044035` green on Linux stable/MSRV, macOS, and Windows. The finite opt-in mapping preserves exact-tag defaults and ReleaseManifest v1; its mapped runtime identity envelope is explicitly versioned.
- **Downstream handoff:** wg-basic Distribution M003 may resume against immutable producer SHA `3ddac9d84fc2e10b8451d38672f66b02e06724df`. Its own implementation, native qualification, and closure remain consumer-owned and were not changed in Eggpack.
- **Execution order:** M002b implementation → M002b strict four-platform closure → M003i implementation → M003i strict closure (complete) → separate wg-basic M003 handoff. These additions supersede historical 'no ready' snapshots below; earlier closure records remain unchanged.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies / blockers |
|---|---|---|---|---|
| Contract and conformance | **complete** | `plans/subsystems/contract-conformance-roadmap.md` | M001-M003 closed | M003 closed on `43fa2d7`; closure `plans/closure/contract-conformance/003-status.md`: additive local `eggpack contract expand` scalar projection over existing schema-v1 semantics, 18 new tests including a process-boundary stdout contract, parity proven against eggsact `d4e6e5c` and stegoeggo `v0.5.0` for every target/alias/field. Roadmap §11 completion definition fully met; no contract milestone remains registered |
| External backend evaluation | closed | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 closed (C) | no production backend adopted; new evidence/plan required to reopen |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M002/M003 closed; M003a closed | M003a closed as a docs-only post-publication downstream-closure reconciliation after Eggup M004 closed (`eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`; formal Eggup status cleanup `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`); closure `plans/closure/release-manifest/003a-status.md`; M003 remains closed and no runtime/package requalification was performed or implied |
| Build and qualification | active | `plans/subsystems/build-qualification-roadmap.md` | M001-M006 closed | M006 closed on `398cd43` + run 36484758546 under accepted ADR-0005 Option A; the milestone chain is closed and later work needs new evidence |
| Bootstrap installers | active; corrective closed | `plans/subsystems/bootstrap-installers-roadmap.md` | M002b closed; prior milestones closed | M002b implementation `88ddf2e`, hosted run `37806244313`; POSIX runtime remains on Unix and Windows PowerShell archive runtime passed. M003 completion definition remains satisfied |
| CI/release orchestration | active; M003j closed, M003k in progress | `plans/subsystems/ci-release-orchestration-roadmap.md` | M003k pre-checkout tag resolution + stage environment | Local gates pass; hosted matrix, producer PR review, and downstream pin/qualification remain |
| Eggup interoperability | active | `plans/subsystems/eggup-interoperability-roadmap.md` | Eggpack M001a + Eggup adapter M001/M001a + Eggup M003 real-consumer adoption + Eggup M004 registry promotion all closed | The whole producer/consumer package seam is closed end to end; Release Manifest M003a closed the remaining Eggpack-side planning drift, and no Eggpack work remains on this seam; Eggwork Operations M003 remains producer-only |
| Ecosystem adoption | active | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001/M002/M003a/M003b closed | Closure `plans/closure/ecosystem-adoption/003b-status.md`. Paired plans: Eggpack `plans/implementation/ecosystem-adoption/003b-eggsearch-seven-target-eggpack-producer-cutover.md` (`325d44e`); Eggsearch `plans/implementation/eggpack-release-adoption/001-seven-target-eggpack-producer-cutover.md` (`377f9e8`, registry `6767062`). Implementation landed at `eggstack/eggsearch@eabbf80`; hosted run `37531104901` staged eggsearch `v0.4.2` as draft `405157598` with the exact 19-asset inventory, glibc 2.17 re-proven on all three GNU targets, ARMv7 required consumer validator green, Windows ARM64 required and green, and zero OIDC permission in generated jobs. Provenance attested the staged bytes in run `37561960304`; the draft was published 2026-10-07, closing the named condition |
| Planning hygiene | active | `plans/subsystems/planning-hygiene-roadmap.md` | M001/M002 closed | M002 closed at `plans/closure/planning-hygiene/002-status.md`: docs/evidence-only post-M003a handoff reconciliation with an empty production/package/workflow diff; stale/duplicated control-surface text removed and the handoff bound to exact paired-plan paths |

**Cross-repo M003 mapping:** Eggpack interoperability M003 is the producer/interface view of Eggup's `plans/implementation/eggpack-manifest-interoperability/003-eggsact-real-consumer-manifest-adoption.md`, and that consumer milestone is now closed (`eggstack/eggup@538e3e5`, `eggstack/eggsact@65c916b`, hosted CI `36902758482` + drift `36902758396`). Ecosystem M001 / Eggsact M005 established the live producer contract and published `release-manifest.json`; runtime manifest acquisition/projection and update semantics remain Eggup/Eggsact-owned. Eggup M004a moved the active producer prerequisite to Release Manifest M003: publish the already-qualified leaf crate `eggpack-manifest 0.1.0` under `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md`. That publication is now closed: `eggpack-manifest 0.1.0` is live on crates.io, and the published `src/lib.rs` is byte-identical to the consumer-qualified pin `678bbf04`, so the Eggpack-owned prerequisite Eggup M004a identified is satisfied. That Eggup-owned chain has also completed: Eggup M004 published `eggup-acquisition 0.1.2` -> `eggup-eggfetch 0.1.2` -> `eggup-eggpack 0.1.2` in dependency order from publication source `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28` (hosted run `37090397398` green on Stable, MSRV, macOS, and Windows) and closed at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915`, with Eggup's own stale blocked-state prose formally reconciled at `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`. The producer/consumer registry seam is therefore closed end to end, and Release Manifest M003a closed the remaining Eggpack-side planning drift against that downstream truth. No Eggpack implementation work is unlocked by it; the next downstream action is Eggsact's own Git-to-registry migration, owned and authorized in `eggstack/eggsact`.

## Dependency-ready implementation work

Planning Hygiene M002, Ecosystem M003b, Bootstrap M002b, CI M003i, and CI M003j are closed. **Dependency-ready CI M003k:** `plans/implementation/ci-release-orchestration/003k-precheckout-tag-resolution-and-stage-environment.md`. It resolves the downstream wg-basic R001 pre-checkout and stage-environment gates while preserving M003j's safe dispatch handling.

The table below is the **milestone-to-closure index plus active ready work**, not an implication that every historical row is executable.

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Bootstrap installers | M002b Windows POSIX archive-test portability | **closed** | `plans/implementation/bootstrap-installers/002b-windows-posix-archive-test-portability-corrective.md` | Closure `plans/closure/bootstrap-installers/002b-status.md`; implementation `88ddf2e`; hosted run `37806244313` all lanes green |
| CI orchestration | M003i exact tag versus manifest release ID policy | **closed** | `plans/implementation/ci-release-orchestration/003i-explicit-tag-and-manifest-release-identity-corrective.md` | Closure `plans/closure/ci-release-orchestration/003i-status.md`; implementation `3ddac9d84fc2e10b8451d38672f66b02e06724df`; hosted run `37813044035` all four lanes green. wg-basic M003 may resume with this pin |
| CI orchestration | M003j dispatch-tag Bash interpolation corrective | closed | `plans/implementation/ci-release-orchestration/003j-dispatch-tag-shell-injection-corrective.md` | Closure `plans/closure/ci-release-orchestration/003j-status.md`; implementation `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce`; hosted run `37987890305` all four lanes green |
| CI orchestration | M003k pre-checkout tag resolution and stage approval gate | **in progress** | `plans/implementation/ci-release-orchestration/003k-precheckout-tag-resolution-and-stage-environment.md` | PR `eggstack/eggpack#1` at `23d9f0347d89fd6ecd916692c145152e39de687f`; local gate and hosted run `38027143275` passed; independent review/merge and downstream qualification pending |
| Planning hygiene | M001 current evidence baseline + blocker reconciliation | closed | `plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md` | Closure `plans/closure/planning-hygiene/001-status.md`; discharged CI M003b (eggsact `685fa373` + run `36886042696`) and closed Ecosystem M002 on stegoeggo `v0.5.0` (run `37181914252`); docs-only, zero production/package/workflow delta |
| Planning hygiene | M002 post-M003a cross-repository handoff reconciliation | closed | `plans/implementation/planning-hygiene/002-post-m003a-cross-repository-handoff-reconciliation.md` | Closure `plans/closure/planning-hygiene/002-status.md`; docs/evidence-only; stale M003/M003a/M003b control-surface text reconciled against exact paired-plan paths; empty production/package/workflow diff |
| Contract and conformance | M003 bounded direct-contract expansion CLI | closed | `plans/implementation/contract-conformance/003-bounded-direct-contract-expansion-cli.md` | Closure `plans/closure/contract-conformance/003-status.md`; implemented on `43fa2d7`; local-only scalar projection delegating to `DistributionContract`; `ci` family unchanged byte-for-byte; no new dependency, serialized schema, discovery, or framework |
| Bootstrap installers | M003 two-consumer adoption/receipt decision | closed | `plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md` | Closure `plans/closure/bootstrap-installers/003-status.md`; zero production delta on Eggpack `013e091`; live installer/manifest parity proven for both releases; wrapper delegation declined (no stable failure/outcome protocol exists to preserve 404-only fallback), Eggup receipt handoff rejected, roadmap §11 completion definition satisfied; optional consumer-side asset-name cleanup only |
| Ecosystem adoption | M003a Eggsearch seven-target compatibility preflight + migration design | closed | `plans/implementation/ecosystem-adoption/003a-eggsearch-seven-target-compatibility-preflight-and-migration-design.md` | Closure `plans/closure/ecosystem-adoption/003a-status.md`; read-only/scratch against `eggstack/eggsearch@ec437cb`, zero production delta; all seven targets rendered and `ci check: match`; G1-G4 all selected Option A; no producer prerequisite registered |
| Ecosystem adoption | M003b Eggsearch seven-target producer cutover | closed | `plans/implementation/ecosystem-adoption/003b-eggsearch-seven-target-eggpack-producer-cutover.md` | Closure `plans/closure/ecosystem-adoption/003b-status.md`. Paired Eggsearch plan registered at `eggstack/eggsearch@377f9e8` / registry `6767062`; implementation landed at `eabbf80`; hosted run `37531104901` staged eggsearch `v0.4.2` as draft `405157598`, exact 19 assets, ARMv7 required validator green; provenance attested the staged bytes in run `37561960304`; draft published 2026-10-07, discharging the named condition |
| Release manifest | M002 final-artifact builder | closed | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | Closure `plans/closure/release-manifest/002-status.md` |
| Release manifest | M003 consumer compatibility baseline + `eggpack-manifest 0.1.0` publication | closed | `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md` | Closure `plans/closure/release-manifest/003-status.md`; published `eggpack-manifest 0.1.0` from `8d661e4`, crates.io checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`, tag `eggpack-manifest-v0.1.0`, hosted run 37064833069 green, external registry-only `=0.1.0` consumer proof green |
| Release manifest | M003a post-publication downstream closure reconciliation | closed | `plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md` | Closure `plans/closure/release-manifest/003a-status.md`; M003 and Eggup M004 are both closed; Eggpack planning/closure surfaces reconciled against `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915` + `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`; docs-only, zero production/package/workflow delta |
| Build and qualification | M001 PackConfig/ReleasePlan | closed | `plans/implementation/build-qualification/001-pack-config-and-release-plan.md` | Closure `plans/closure/build-qualification/001-status.md` |
| Build and qualification | M002 native/cross builder seam | closed (historical) | `plans/implementation/build-qualification/002-native-cross-builder-execution-seam.md` | Closure `plans/closure/build-qualification/002-status.md`; post-closure Windows stability finding tracked by M002a |
| Build and qualification | M002a Windows qualification stability | closed | `plans/implementation/build-qualification/002a-windows-builder-qualification-stability-corrective.md` | Closure `plans/closure/build-qualification/002a-status.md`; three first-attempt hosted Windows runs passed |
| Build and qualification | M003 qualification execution | closed | `plans/implementation/build-qualification/003-qualification-execution-and-evidence.md` | Closure `plans/closure/build-qualification/003-status.md`; hosted run 36095915717 |
| Build and qualification | M004 finalization/aggregation | closed | `plans/implementation/build-qualification/004-finalization-and-local-aggregation.md` | Closure `plans/closure/build-qualification/004-status.md`; hosted run 36098072913 |
| Build and qualification | M005 deterministic cross-tool provisioning | closed | `plans/implementation/build-qualification/005-deterministic-cross-tool-provisioning.md` | Closure `plans/closure/build-qualification/005-status.md`; implementation `7a206ba`; hosted run 36256831000 green; exact Zig 0.14.1 / cargo-zigbuild 0.23.3 provisioning, verified Zig path binding, private cargo-zigbuild cache |
| Build and qualification | M006 native qualification for cross-tool builds | closed | `plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md` | Closure `plans/closure/build-qualification/006-status.md`; implementation `398cd43`; hosted run 36484758546 green; host-matched native qualification is now independent of the builder, with split-host qualification and eggsact-shaped five-target render evidence |
| Bootstrap installers | M001 generator/direct fixtures | closed | `plans/implementation/bootstrap-installers/001-direct-installer-generator.md` | Closure `plans/closure/bootstrap-installers/001-status.md` |
| CI orchestration | M001 CIPlan/GitHub renderer | closed | `plans/implementation/ci-release-orchestration/001-ci-plan-and-github-renderer.md` | Closure `plans/closure/ci-release-orchestration/001-status.md`; hosted run 36040032768 passes all lanes |
| CI orchestration | M002 qualification/aggregation gates + drift CLI | closed historically; corrective closed | `plans/implementation/ci-release-orchestration/002-qualification-aggregation-gates-and-drift-cli.md` | Historical closure retained; executable generated-workflow defect corrected and qualified by M002a (`plans/closure/ci-release-orchestration/002a-status.md`) |
| CI orchestration | M002a generated workflow execution wiring | closed | `plans/implementation/ci-release-orchestration/002a-generated-workflow-execution-wiring-corrective.md` | Closure `plans/closure/ci-release-orchestration/002a-status.md`; implementation `4d2270a`; hosted run 36154905956 (attempt 1, all lanes green) |
| CI orchestration | M003a local staging payload + GitHub draft adapter | closed historically; corrective closed | `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md` | Historical closure retained; M003c resolves transfer/pagination/origin/query findings |
| CI orchestration | M003b generated draft staging job + operational qualification | closed | `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md` | Real draft/inventory/draft-only/no-clobber proven by eggsact `v1.2.7` (run 36652731202); exact byte-identical rerun reuse proven by eggsact M005a rehearsal run 36886042696 attempt 2. Phase 8 satisfied; discharge in `plans/closure/ci-release-orchestration/003b-status.md` §6 |
| CI orchestration | M003c staging source identity + bounded transfer corrective | closed | `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md` | Closure `plans/closure/ci-release-orchestration/003c-status.md`; implementation `5c28099`; hosted run 36213316240 green |
| CI orchestration | M003d consumer release composition seam | closed | `plans/implementation/ci-release-orchestration/003d-consumer-release-composition-seam.md` | Closure `plans/closure/ci-release-orchestration/003d-status.md`; reusable static workflow/runtime identity, public product wrappers + generated exact installers, bounded Python3 consumer validator |
| CI orchestration | M003e generated release execution wiring | closed | `plans/implementation/ci-release-orchestration/003e-generated-release-execution-wiring-corrective.md` | Closure `plans/closure/ci-release-orchestration/003e-status.md`; implementation `b9062d4`; hosted run 36572608484 green; renderer installs tool before use and creates output dirs, CLI absolutizes generated relative paths. Post-closure live dispatch exposed the rejected `-p` install flag, corrected by M003f |
| CI orchestration | M003f generated tool-install command | closed | `plans/implementation/ci-release-orchestration/003f-generated-tool-install-command-corrective.md` | Closure `plans/closure/ci-release-orchestration/003f-status.md`; implementation `c190e77`; hosted run 36632209736 green; positional package selection. The second live dispatch exposed F9/F10, corrected by M003g |
| CI orchestration | M003g live qualification failures | closed | `plans/implementation/ci-release-orchestration/003g-live-qualification-corrective.md` | Closure `plans/closure/ci-release-orchestration/003g-status.md`; live run 36652731202 created the complete 15-asset eggsact draft |
| CI orchestration | M003h live qualification status reconciliation + main integration | closed | `plans/implementation/ci-release-orchestration/003h-live-qualification-status-reconciliation-and-main-integration.md` | Closure `plans/closure/ci-release-orchestration/003h-status.md`; synchronization merge `609d5fb`, full local verification, hosted CI, and non-forced main fast-forward recorded |
| Bootstrap installers | M002 bundle/archive bootstrap safety | closed historically; corrective closed | `plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md` | Historical closure retained; PowerShell archive runtime evidence gap corrected and qualified by M002a (`plans/closure/bootstrap-installers/002a-status.md`) |
| Bootstrap installers | M002a PowerShell archive runtime evidence | closed | `plans/implementation/bootstrap-installers/002a-powershell-archive-runtime-evidence-corrective.md` | Closure `plans/closure/bootstrap-installers/002a-status.md`; implementation `4d2270a`; hosted run 36154905956 (attempt 1, all lanes green) |
| Eggup interoperability | M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | Closure `plans/closure/eggup-interoperability/001-status.md`; post-closure projection defect tracked by M001a |
| Eggup interoperability | M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | Closure `plans/closure/eggup-interoperability/001a-status.md`; corrected pairwise fixture baseline |

## Planned / blocked work

Only genuinely open lines belong here. Closed milestones are indexed under
"Dependency-ready implementation work" above and in the per-subsystem roadmaps;
a closed row in this table is a contradiction, and the six that used to sit here
were removed rather than left to mislead.

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|
| Ecosystem adoption | M004 Gregg sibling bundle | blocked | Remains after Eggsearch target-diversity evidence; prior adoption evidence + Gregg bundle/service review |
| Ecosystem adoption | M005 CodeGG runfile bundle | blocked | Remains after a qualified sibling-bundle consumer; prior bundle evidence + CodeGG runfile review |
| Ecosystem adoption | M006 Egress archive pair | blocked | Remains after prior native adoption evidence; requires explicit archive/Python boundary review |
| Provenance/authenticity | future | planned | Researchable but not implementation-ready: requires a dedicated trust ADR. Eggsearch's existing GitHub Artifact Attestation release is prior art only and does not select Eggpack's trust model |
| Python/wheel adapters | future | planned | Remains behind broader native adoption (Phase 13); separate package-adapter planning, registry publication explicit |

Build/Qualification M002a/M003/M004/M005 remain closed with hosted cross-platform evidence. CI M002 and Bootstrap M002 retain their historical implementation/closure records. Both M002a correctives are closed on implementation `4d2270a` with hosted run 36154905956 green. CI M003a retains its historical evidence (`36f1cc1`/36180399698); M003c corrected M003a/M003b's post-closure source-integrity and transfer findings in `5c28099`, qualified by hosted run 36213316240 (all four lanes green). CI M003d then closed the reusable consumer-composition/runtime-identity seam, and Build M005 closed deterministic cross-tool provisioning on `7a206ba` with hosted run 36256831000 green.

Ecosystem M001 and M002 are both closed. M001 stopped at its §20 condition before implementation, resumed after Build M006, and completed on eggsact `v1.2.7`; M002's cutover landed with no new Eggpack primitive and its live evidence arrived as stegoeggo `v0.5.0`. CI M003b is closed and Phase 8 is satisfied. Bootstrap M003 is closed at `plans/closure/bootstrap-installers/003-status.md` with zero production delta, so the adoption completion definition remains satisfied; a new independent M002b Windows test-portability corrective is now ready. Contract M003 is closed at `plans/closure/contract-conformance/003-status.md` on implementation `43fa2d7`, which completes the contract-conformance subsystem. **Ecosystem M003a is closed** at `plans/closure/ecosystem-adoption/003a-status.md` with no producer prerequisite. **Ecosystem M003b is closed** at `plans/closure/ecosystem-adoption/003b-status.md`: both paired plans were registered and the implementation landed in `eggstack/eggsearch`, with live staging evidence at eggsearch `v0.4.2` (run `37531104901`, draft `405157598`, exact 19-asset inventory, ARMv7 required validator green), provenance attestation of the staged bytes (run `37561960304`), and publication of that draft on 2026-10-07, which discharged the named condition.

## Immediate execution graph

```text
Eggup distribution M003 [CLOSED / FROZEN PREDECESSOR]
               |
               v
Eggpack contract M001 [CLOSED]
               |
               v
contract conformance M002 [CLOSED: M003 PORT QUALIFIED]
        |                      |
        |                      +--> Eggup M004 retire eggup-dist [CLOSED]
        v
ReleaseManifest M001 [CLOSED HISTORICALLY]
        |
        v
Manifest M001a corrective [CLOSED: target-local installs, global artifact filenames]
        |
        +--> Manifest M002 [CLOSED; establishes eggpack-core and final-byte manifest builder]
        |        |
        |        `--> build/qualification M001 [CLOSED; ReleasePlan interface established]
        |                         |
        |                         v
        |              ADR-0004 [ACCEPTED: first-party Cargo/zigbuild]
        |                         |
        |                         v
        |              build/qualification M002 [CLOSED HISTORICALLY]
        |                         |
        |                         v
        |              build/qualification M002a [CLOSED: WINDOWS STABILITY QUALIFIED]
        |                         |
        |                         +--> qualification M003 [CLOSED]
        |                         |             |
        |                         |             `--> Build M004 [CLOSED]
         |                         `--> CI Orchestration M001 [CLOSED; consumes M002 interface]
         |                                      |
         |                                      `--> CI M002 [CLOSED HISTORICALLY]
         |                                                    |
         |                                                    `--> CI M002a [CLOSED: IMPL 4d2270a QUALIFIED RUN 36154905956]
         |                                                                  |
         |                                                                  v
         |                                                      CI M003a [HISTORICAL CLOSE; CORRECTIVE CLOSED]
         |                                                                  |
         |                                                                  `--> CI M003b [CLOSED: rerun reuse discharged by eggsact M005a]
         |                                                                               |
         |                                                                               v
         |                                                                    CI M003c [CLOSED]
         |                                                                               |
         |                                                                    CI M003d [CLOSED] ----------+
         |                                                                                                 |
         |              Build M004 [CLOSED] --> Build M005 [CLOSED] ---------------------+
         |                                                                                                 v
         |                                                                      ADR-0005 [ACCEPTED: OPTION A]
         |                                                                                     |
         |                                                                                     v
         |                                                                          Build M006 [CLOSED]
         |                                                                                     |
         |                                                                                     v
         |                                                                          eggsact M001 [CLOSED]
         |                                                                                     |
         |                                                                                     `--> M003b exact rerun reuse [CLOSED: eggsact M005a run 36886042696]
         |                                                                                     |
         |                                                                                     `--> Phase 8 [SATISFIED] --> Planning Hygiene M001 [CLOSED]
         |                                                                                                        |
         |                                                                                                        v
         |                                                                                        stegoeggo M002 live `v0.5.0` [CLOSED: run 37181914252]
         |                                                                                                        |
         |                    +-----------------------+------------------------+
         |                    |                       |                        |
         |                    v                       v                        v
         |        Bootstrap M003 [CLOSED: EVIDENCE/OWNERSHIP, NO M003a]  Contract M003 [CLOSED]     Ecosystem M003a eggsearch preflight [CLOSED]
         |                    (roadmap 11 completion satisfied)             (bounded CLI surface)                  |
         |                                                                         `--> Ecosystem M003b adoption [CONDITIONALLY CLOSED: eggsearch v0.4.2 staged, publication pending]
         |
         `--> bootstrap installers M001 [CLOSED; direct first-install generator]
                        |
                        `--> Bootstrap M002 [CLOSED HISTORICALLY]
                                      |
                                      `--> Bootstrap M002a [CLOSED: IMPL 4d2270a QUALIFIED RUN 36154905956]
        `--> Eggup interoperability M001 [CLOSED HISTORICALLY]
                       |
                       v
             Eggup interoperability M001a [CLOSED]
                       |
                       `--> Eggup adapter M001 [CLOSED HISTORICALLY]
                                  |
                                  v
                           Eggup adapter M001a [CLOSED]
                                  |
                                   `--> Eggsact runtime manifest adoption [CLOSED IN EGGUP/EGGSACT]
                                                 |
                                                 v
                                    Release Manifest M003 [CLOSED: eggpack-manifest 0.1.0 PUBLISHED]
                                                  |
                                                  +--> Eggup M004 external registry promotion [CLOSED DOWNSTREAM]
                                                  |
                                                  `--> Release Manifest M003a status/closure reconciliation [CLOSED: DOCS ONLY]

External dist 0.33 spike [CLOSED, disposition C] --> prior art only
```

## Initial architecture constraints

- Rust baseline: 1.89 unless changed by ADR.
- `eggpack-contract` migration is fidelity-first; no opportunistic schema redesign.
- `eggup-dist` was frozen through the migration and has now been removed by closed Eggup M004 after Eggpack Contract M002 qualification.
- concrete release manifests describe final bytes only.
- generated CI is checked in and drift-checked.
- staging/qualification does not automatically publish.
- checksums are integrity, not authenticity.
- no external backend is production-authorized; ADR-0004 authorizes only the first-party Cargo/cargo-zigbuild adapter boundary.
- Eggup core must remain independent of producer tooling.

## Next handoff

Every closed milestone retains its closure record, so do not read this section as a shortlist: Contract M001/M002, Release Manifest M001/M001a/M002/M003/M003a, Build/Qualification M001–M006, CI M001/M002/M002a/M003a–M003h, Bootstrap M001/M002/M002a, Eggup Interoperability M001/M001a, Ecosystem M001/M002, External backend evaluation M001, and Planning Hygiene M001 all have accepted closure records under `plans/closure/`. Eggup Interoperability M003 is additionally closed downstream on the real Eggsact consumer. Per-subsystem detail is in `plans/subsystems/*-roadmap.md`; the milestone-to-closure index is the table above.

M003h is closed on the synchronized, verified candidate. **Ecosystem M002 stegoeggo is closed** at `plans/closure/ecosystem-adoption/002-status.md` §9: consumer implementation `3b96fae` and closure `c75132a` prove the second-repo cutover without a new Eggpack producer primitive, and the live evidence arrived as stegoeggo `v0.5.0` — Eggpack run `37181914252`, published 2026-10-04T06:34:19Z from source `57ca94c9…`, exact 15-asset inventory, manual publication, real public `0.4.2 -> 0.5.0` updater transition. The `0.4.3` milestone earlier planning anticipated was stopped pre-publication by the consumer on semver grounds and superseded by `0.5.0`.

The Eggpack-owned prerequisite that Eggup M004a identified is now satisfied, and the consumer that consumed it has finished. **Release Manifest M003 is closed** at `plans/closure/release-manifest/003-status.md`: `eggpack-manifest 0.1.0` was published to crates.io from `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0` (checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`, tag `eggpack-manifest-v0.1.0`, hosted run 37064833069 green), and an external registry-only consumer resolved exact `=0.1.0` with no Git or path source. The published `src/lib.rs` is byte-identical to the consumer-qualified pin `678bbf04f5a02827003a1d9ab83ba4f0e6360e41`, so Eggup could move its manifest dependency to a registry pin with no semantic requalification implied. Only `eggpack-manifest` was published; the other six Eggpack workspace crates remain unpublished.

Eggup completed that publication sequence and closed M004 at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915` from publication source `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28` (hosted run `37090397398` green on Stable, MSRV, macOS, and Windows), then formally reconciled its own stale blocked-state prose at `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`. **Release Manifest M003a is closed** at `plans/closure/release-manifest/003a-status.md`: it reconciled Eggpack's registry, both affected subsystem roadmaps, and the M003 downstream-receipt addendum against that downstream truth, with an empty production/package/workflow diff and M003's historical closure body preserved.

**No Eggpack work remains on the M003 -> Eggup M004 registry seam.** Planning Hygiene M001 is now closed at `plans/closure/planning-hygiene/001-status.md`: it discharged CI M003b, closed Ecosystem M002, and gave Contract M003, Bootstrap M003, and Ecosystem M003 exact dispositions.

**Bootstrap M003 is closed** at `plans/closure/bootstrap-installers/003-status.md`. It was the smallest of the three ready lines — an evidence/ownership decision with no assumed production change — and it closed on 2026-10-05 with zero production delta on Eggpack `013e091`. Three decisions are settled: the existing exact-installer generator generalizes across two live products without a new producer primitive; wrapper-to-exact-installer delegation is **not warranted** and would need a new stable machine-readable failure/outcome protocol first (`curl --fail` collapses 404 and 5xx to exit 22, and the PowerShell installer exposes no exit codes at all); and an Eggup receipt handoff is **rejected** because bootstrap installs exact bytes while Eggup owns transaction/rollback state. The roadmap §11 completion definition is satisfied, so the bootstrap-installers subsystem has no open work. The only follow-up is optional and consumer-owned: deleting the wrapper asset-name reconstruction once Contract M003 lands, under a plan registered in the consumer's own repository.

**Contract M003 is closed** at `plans/closure/contract-conformance/003-status.md`, on implementation `43fa2d7`. It delivered exactly the bounded projection it was registered as: `eggpack contract expand` reads one local contract through the shared bounded non-symlink reader, delegates to `DistributionContract::parse_toml_str` and `::expand`, and prints exactly one of `canonical-target`/`asset`/`sidecar`/`install`. It adds no dependency, no serialized schema, no repository or network discovery, and no release selection, and the historical `ci` family is unchanged byte-for-byte. Two design points are worth carrying forward: the command parses its own arguments because the shared `get_flag` helper silently keeps the first of a repeated flag, which is wrong for output a consumer script captures; and `canonical-target` resolves for every asset form while the three direct-only fields fail closed for bundle/archive — the first implementation had that inverted and the test lane caught it. Consumer-shaped parity was proven by running the binary against the real eggsact and stegoeggo contracts for every target, every alias, and all four fields. The subsystem's completion definition is now fully met, so the only downstream effect is that each consumer's optional asset-name reconstruction cleanup became actionable, under plans in those repositories.

**Ecosystem M003a is closed** at `plans/closure/ecosystem-adoption/003a-status.md`. It was the last registered plan, and it closed read-only/scratch with zero production delta. The four compatibility gaps the split was created to resolve all resolved with existing producer capability: the Zig 0.13.0 archive-layout gap is real (confirmed 404 against ziglang.org for the name Eggpack constructs, 200 for the name that exists) but the pin is an unpublished implementation detail while the glibc 2.17 floor is the actual guarantee and is re-asserted post-build; ARMv7 keeps its runtime proof by classifying core qualification `Structural` and executing under a required product-owned consumer validator that still gates aggregation, while the alternative `Emulated` path renders a `--qemu-sysroot` flag while provisioning nothing; Artifact Attestations are preserved by a separate read-only product-owned workflow that attests exact already-staged bytes without requesting OIDC in any generated job; and no-clobber staging is strictly stronger than the `--clobber` it replaces. All seven targets rendered and `ci check` reported `match`.

**Historical pre-2026-10-08 snapshot (superseded by M002b/M003i handoff): no Eggpack plan was ready;** prior lines remain closed. **Ecosystem M003b is closed** at `plans/closure/ecosystem-adoption/003b-status.md`: the mirrored Eggsearch plan was registered at `eggstack/eggsearch@377f9e8` (registry `6767062`), the Eggsearch cutover landed on `eabbf80`, hosted run `37531104901` staged eggsearch `v0.4.2` as draft `405157598` with the exact 19-asset inventory and the ARMv7 required consumer validator green, run `37561960304` attested those staged bytes, and the draft was published on 2026-10-07 — discharging the condition that conditional closure had named. No OIDC permission was ever requested in a generated job. The current Eggpack action is M002b; this earlier plan-authoring statement is historical.

Eggsearch **M003b passed implementation and is conditionally closed.** M003a settled all five questions it was holding, and the Eggsearch implementation answered each one with existing producer capability and no new Eggpack primitive: the Zig/cargo-zigbuild pair migrated to the already-qualified 0.14.1/0.23.3 pair with the glibc 2.17 ceiling re-proven per GNU target on the exact staged bytes; ARMv7 kept its runtime proof through `Structural` core qualification plus a required product-owned consumer validator that gated `required_gate` and `aggregate`; Windows ARM64 stayed a required target on `windows-11-arm`; Artifact Attestation was preserved by a separate read-only product-owned workflow, leaving every generated job free of OIDC; and Eggpack's fail-closed staging superseded `--clobber`. The mirrored-plan precondition in planning process §9 was satisfied before any external edit, at `eggstack/eggsearch@377f9e8`.

Longer-horizon lines stay as recorded: Ecosystem M004-M007 behind completed Eggsearch adoption, Phase 12 provenance behind a dedicated trust ADR, Phase 13 package adapters behind broader native adoption, Phase 14 stabilization premature. Eggsact's Git-to-registry migration remains separately owned downstream.

## Downstream unblock disposition

The live evidence closes Ecosystem M001 and M002, discharges CI M003b, and satisfies Phase 8. Closing M003e alone only removed the execution-wiring blocker; later evidence resolved the remaining M001 dependencies, and consumer-side evidence resolved M003b and M002.

| Plan | Disposition |
|---|---|
| Ecosystem M001 eggsact adoption | Closed. Implemented, live-qualified, and published on release `v1.2.7`; see `plans/closure/ecosystem-adoption/001-status.md`. |
| CI M003b full closure | **Closed.** Draft, exact inventory, draft-only behavior, and no-clobber were proven by eggsact `v1.2.7`; the exact rerun-reuse condition is discharged by eggsact M005a — rehearsal run `36886042696` attempt 2 reused draft `401132612` with `created: false, uploaded: 0, reused: 15`, identical digests, zero refusals. All eleven §13 criteria met; the fix was product-side and no Eggpack change was required. See `plans/closure/ci-release-orchestration/003b-status.md` §6. |
| Phase 8 exit | **Satisfied.** Generated draft staging, exact inventory, draft-only behavior, no-clobber refusal, and exact rerun reuse are all proven on a real maintainer-authorized repository, with publication remaining a separate human action throughout. |
| Ecosystem M002 stegoeggo | **Closed.** Cutover landed/qualified, consumer updater corrective M003 closed, and live evidence delivered by stegoeggo `v0.5.0` (run `37181914252`, published 2026-10-04T06:34:19Z). Rerun-reuse was not exercised on that release and is substituted by eggsact M005a evidence; see `plans/closure/ecosystem-adoption/002-status.md` §9.3. |
| Contract M003 | **Closed.** Implementation `43fa2d7`; closure `plans/closure/contract-conformance/003-status.md`; `scripts/check-local.sh` green including the 1.89.0 MSRV lane; parity proven against both adopted consumers. Optional consumer-side asset-name cleanup is now actionable, under plans in those repositories only. |
| Bootstrap M003 | **Closed.** Evidence/ownership decision, no production change. Closure `plans/closure/bootstrap-installers/003-status.md`; live installer/manifest parity proven for eggsact `v1.2.7` and stegoeggo `v0.5.0`; wrapper delegation declined and Eggup receipt handoff rejected; roadmap §11 completion definition satisfied; no M003a required. Optional consumer-side asset-name cleanup becomes eligible once Contract M003 closes, and only under a plan registered in that consumer repository. |
| Ecosystem M003a eggsearch preflight | **Closed.** Closure `plans/closure/ecosystem-adoption/003a-status.md`; read-only/scratch, zero production delta; all seven targets render and Windows ARM64 is proven; G1-G4 decided with evidence and no new Eggpack capability required. |
| Ecosystem M003b eggsearch adoption | **Closed.** Closure `plans/closure/ecosystem-adoption/003b-status.md`. Paired plans were registered before any external edit; the Eggsearch cutover landed at `eggstack/eggsearch@eabbf80` and hosted run `37531104901` staged eggsearch `v0.4.2` as draft `405157598` with the exact 19-asset inventory, glibc 2.17 re-proven on all three GNU targets, ARMv7 required consumer validator green, Windows ARM64 required and green, provenance in a separate read-only workflow with no OIDC in generated jobs, and the legacy writer retired. Provenance then attested the staged bytes in run `37561960304`, and the draft was published on 2026-10-07. |
| Build/Qualification M006 | Closed under accepted ADR-0005 Option A; implementation `398cd43`, hosted run 36484758546 green. |
| Eggup Interoperability M003 | Closed in Eggup/Eggsact. Consumer `eggstack/eggsact@65c916b`; Eggup closure `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/003-status.md`; hosted CI `36902758482` + drift `36902758396` green. |
| Release Manifest M003 | Closed. `eggpack-manifest 0.1.0` published from `8d661e4` (checksum `2a08f24b…b629`, tag `eggpack-manifest-v0.1.0`), published bytes byte-identical to the consumer-qualified pin `678bbf04`, and exact registry-only `=0.1.0` resolution proven by an external consumer. This removed the Eggpack-owned prerequisite identified by Eggup M004a. |
| Eggup M004 | Closed downstream in Eggup at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915` after publishing `eggup-acquisition 0.1.2` -> `eggup-eggfetch 0.1.2` -> `eggup-eggpack 0.1.2` from publication source `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28` under hosted run `37090397398`; formal Eggup status cleanup `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d`. Eggpack does not claim that work as its own. The chain is no longer a blocker, and `eggup-eggpack 0.1.2` now consumes registry `eggpack-manifest =0.1.0`. |
| Eggsact Git-to-registry migration | Not an Eggpack item. Downstream, Eggsact-owned, and separately authorized in `eggstack/eggsact`; it does not gate any Eggpack milestone. |
| Release Manifest M003a | Closed. Docs-only reconciliation of Eggpack active planning and the M003 downstream-receipt addendum against closed Eggup M004; plan `plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md`, closure `plans/closure/release-manifest/003a-status.md`. M003 remains closed and no further Eggpack work is owed on this seam. |
| Other Eggpack package publication | Not authorized by M003 and not started. `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-ci`, `eggpack-github`, and `eggpack-cli` remain unpublished; a new plan plus evidence is required before any of them is considered for publication. |
| StegoEggo `release/0.5.0` not merged to consumer `main` | Not an Eggpack blocker. Consumer-side housekeeping. The tag, the published manifest `source_revision`, and the Eggpack run `head_sha` all agree on `57ca94c9…`, so the evidence is sound regardless of consumer branch topology. |

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
