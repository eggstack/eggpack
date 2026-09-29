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
- eggsact consumer baseline re-review: the mirrored baseline `174764c5c71130ec98fee18c445fcecb3e35eb25` has advanced to `34aed3ab36da2637c22412f7ca65d35f1ca5021d` (17 commits). The five-target matrix, Zig 0.14.1 / cargo-zigbuild 0.23.3, official Zig archive digests, drafts-only assembly intent, and the mirrored plan's `blocked / planned` status are unchanged. eggsact has since moved the self-update transport to the qualified `eggup-eggfetch` / `eggfetch-core` crates (eggsact `40959b7`), so M001 §17's evidence must be restated in terms of the current updater crates. Per planning process §2 the baseline is flagged for re-review before implementation.

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

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies / blockers |
|---|---|---|---|---|
| Contract and conformance | active | `plans/subsystems/contract-conformance-roadmap.md` | M002 closed; M003 planned | Eggup M004 retirement closed; consumer evidence gates M003 |
| External backend evaluation | closed | `plans/subsystems/external-backend-evaluation-roadmap.md` | M001 closed (C) | no production backend adopted; new evidence/plan required to reopen |
| Release manifest | active | `plans/subsystems/release-manifest-roadmap.md` | M002 closed; M003 planned | M003 requires a real consumer |
| Build and qualification | active | `plans/subsystems/build-qualification-roadmap.md` | M001-M006 closed | M006 closed on `398cd43` + run 36484758546 under accepted ADR-0005 Option A; the milestone chain is closed and later work needs new evidence |
| Bootstrap installers | active | `plans/subsystems/bootstrap-installers-roadmap.md` | M001 closed; M002 historical; M002a closed | M002a closed on `4d2270a` + run 36154905956; M003 blocked on adoption evidence/candidate review |
| CI/release orchestration | active | `plans/subsystems/ci-release-orchestration-roadmap.md` | M003c closed; M003d closed; M003e closed; M003f active | reusable runtime identity + product-wrapper/consumer-validator composition landed; first-consumer end-to-end review proved no generated workflow can execute (M003e corrective closed at `b9062d4`, hosted run 36572608484 green); M001 live-draft first dispatch proved the generated tool-install command is rejected by Cargo (M003f corrective in progress, no mutation made); M003b live-draft proof now waits on the M001 re-dispatch |
| Eggup interoperability | active | `plans/subsystems/eggup-interoperability-roadmap.md` | Eggpack M001a + Eggup adapter M001/M001a closed | M003 real-consumer adoption ready to plan |
| Ecosystem adoption | active | `plans/subsystems/ecosystem-adoption-roadmap.md` | M001 ready (unblocked) | M001 hit its §20 stop condition before implementation; Build M006 is now closed, so M001 is ready subject to its own consumer-baseline re-review and re-pin. Mirrored eggsact M005 plan remains `blocked / planned` |

## Dependency-ready implementation work

Current dependency-ready implementation work:

| Subsystem | Milestone | Status | Plan | Dependencies |
|---|---|---|---|---|
| Release manifest | M002 final-artifact builder | closed | `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md` | Closure `plans/closure/release-manifest/002-status.md` |
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
| CI orchestration | M003b generated draft staging job + operational qualification | conditionally closed; corrective closed | `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md` | Source identity/tag-source findings corrected; live draft is its only remaining condition |
| CI orchestration | M003c staging source identity + bounded transfer corrective | closed | `plans/implementation/ci-release-orchestration/003c-staging-source-identity-and-bounded-transfer-corrective.md` | Closure `plans/closure/ci-release-orchestration/003c-status.md`; implementation `5c28099`; hosted run 36213316240 green |
| CI orchestration | M003d consumer release composition seam | closed | `plans/implementation/ci-release-orchestration/003d-consumer-release-composition-seam.md` | Closure `plans/closure/ci-release-orchestration/003d-status.md`; reusable static workflow/runtime identity, public product wrappers + generated exact installers, bounded Python3 consumer validator |
| CI orchestration | M003e generated release execution wiring | closed | `plans/implementation/ci-release-orchestration/003e-generated-release-execution-wiring-corrective.md` | Closure `plans/closure/ci-release-orchestration/003e-status.md`; implementation `b9062d4`; hosted run 36572608484 green; renderer installs tool before use and creates output dirs, CLI absolutizes generated relative paths. Post-closure live dispatch exposed the rejected `-p` install flag, corrected by M003f |
| CI orchestration | M003f generated tool-install command | active | `plans/implementation/ci-release-orchestration/003f-generated-tool-install-command-corrective.md` | `cargo install --git` rejects `-p`; package goes positional. Single emission-site fix; Ecosystem M001 re-pins to the M003f implementation for its live re-dispatch |
| Bootstrap installers | M002 bundle/archive bootstrap safety | closed historically; corrective closed | `plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md` | Historical closure retained; PowerShell archive runtime evidence gap corrected and qualified by M002a (`plans/closure/bootstrap-installers/002a-status.md`) |
| Bootstrap installers | M002a PowerShell archive runtime evidence | closed | `plans/implementation/bootstrap-installers/002a-powershell-archive-runtime-evidence-corrective.md` | Closure `plans/closure/bootstrap-installers/002a-status.md`; implementation `4d2270a`; hosted run 36154905956 (attempt 1, all lanes green) |
| Eggup interoperability | M001 Eggpack interface/fixtures | closed (historical) | `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md` | Closure `plans/closure/eggup-interoperability/001-status.md`; post-closure projection defect tracked by M001a |
| Eggup interoperability | M001a projection fixture consistency corrective | closed | `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md` | Closure `plans/closure/eggup-interoperability/001a-status.md`; corrected pairwise fixture baseline |

## Planned / blocked work

| Subsystem | Milestone | State | Blocker |
|---|---|---|---|

| CI orchestration | M003b generated draft staging + live qualification | blocked | M003c/M003d, Build M005, Build M006, and M003e are closed, but the live proof runs inside eggsact M001 (Ecosystem M001), which must re-review its consumer baseline and re-pin to the M003e implementation before implementing |
| CI orchestration | M003e generated release execution wiring | closed | Closure `plans/closure/ci-release-orchestration/003e-status.md`; implementation `b9062d4`; Ecosystem M001 re-pins to the M003e implementation |
| Bootstrap installers | M003 two-consumer adoption/receipt decision | blocked | real adoption evidence/candidate review (M002a precondition satisfied) |
| Eggup interoperability | M002 optional adapter | closed / qualified | Eggup implementation `5fbb66853bdad59aaf2bd3c7bb43a43492d0b6ef`; corrective implementation `19935ec3610a5238af33a9d4f05a14925ceac25c`; closure `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/001a-status.md` |
| Eggup interoperability | M003 real-consumer adoption | ready to plan | select a real consumer currently owning duplicated manifest-to-update mapping |
| Ecosystem adoption | M001 eggsact direct release adoption | ready (unblocked) | `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md`; §20 stop condition hit before implementation (no checked-in configuration, no generated workflow, no eggsact change) and is now resolved by `plans/closure/build-qualification/006-status.md`. Remaining gates: re-review the advanced consumer baseline, re-pin `eggpack_tool.revision` to the M003e implementation `b9062d4`, and one maintainer-authorized real release tag. Mirrored eggsact M005 plan registered at baseline `174764c`, consumer since advanced to `34aed3a` |
| Provenance/authenticity | future | planned | manifest/build evidence; trust ADR required |
| Python/wheel adapters | future | planned | native release pipeline maturity |

Build/Qualification M002a/M003/M004/M005 remain closed with hosted cross-platform evidence. CI M002 and Bootstrap M002 retain their historical implementation/closure records. Both M002a correctives are closed on implementation `4d2270a` with hosted run 36154905956 green. CI M003a/M003b retain their historical evidence (`36f1cc1`/36180399698 and `84e4e4f`/36193654633); M003c corrected their post-closure source-integrity and transfer findings in `5c28099`, qualified by hosted run 36213316240 (all four lanes green). CI M003d then closed the reusable consumer-composition/runtime-identity seam, and Build M005 closed deterministic cross-tool provisioning on `7a206ba` with hosted run 36256831000 green.

Ecosystem M001 eggsact stopped at its §20 condition before implementation, so the chain did not reach the M003b real-draft/rerun proof. Build M005 provisioned the cross tools but left `Qualification::Native` inadmissible for a `CargoZigbuild` target, and eggsact must cross-build both Linux targets for a glibc 2.17 floor while qualifying them natively on matching runners. Build M006 has now implemented accepted ADR-0005 Option A, so M001 is unblocked and the chain can resume at M001. Bootstrap M003 remains blocked on adoption evidence/candidate review.

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
         |                                                                  `--> CI M003b [HISTORICAL CONDITIONAL CLOSE]
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
         |                                                                          eggsact M001 [READY]
         |                                                                                     |
         |                                                                                     `--> M003b live draft proof
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
                                  `--> consumer adoption [READY TO PLAN]

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

Manifest M002, Build/Qualification M001-M005, CI M001/M002/M002a/M003a/M003b/M003c/M003d, Bootstrap M001/M002/M002a, and Eggup Interoperability M001a retain closure records.

Build/Qualification M006 is closed at `plans/closure/build-qualification/006-status.md` (implementation `398cd43`, hosted run 36484758546) under accepted ADR-0005 Option A, which completes the build/qualification milestone chain through M006. CI M003e is closed at `plans/closure/ci-release-orchestration/003e-status.md` (implementation `b9062d4`, hosted run 36572608484), which corrects the generated-workflow execution wiring the first-consumer review proved broken. The next dependency-ready implementation milestone is **Ecosystem M001**: `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md`, now `ready`. M001 must re-review the current eggsact baseline, re-pin `eggpack_tool.revision` at the M003e implementation `b9062d4`, and then perform the real M003b draft qualification.

Eggup Interoperability M003 remains independently ready to plan.

## Downstream unblock disposition

Answering the standing question for this pass: **closing CI M003e
unblocks Ecosystem M001 and removes the execution-wiring blocker the
first-consumer review proved. M001 itself still has not closed, so no later ecosystem milestone is
unblocked by this pass.**

| Plan | Disposition |
|---|---|
| Ecosystem M001 eggsact adoption | Ready / unblocked. Its §20 stop condition is resolved by `plans/closure/build-qualification/006-status.md` and its execution-wiring blocker by `plans/closure/ci-release-orchestration/003e-status.md`; remaining gates are its own consumer-baseline re-review, the `eggpack_tool.revision` re-pin to `b9062d4`, and one maintainer-authorized real release tag. |
| CI M003b full closure | Stays conditionally closed. Its sole remaining condition is the real draft/rerun proof, which runs inside M001; M001's producer blocker is gone. |
| Phase 8 exit | Stays open. Exit criteria require a maintainer-inspectable fully qualified draft release. |
| Ecosystem M002 stegoeggo | Stays blocked on M001 closure. The shared native-qualification gap it expected Build M006 to resolve is now resolved. |
| Ecosystem M003 onward | Stays blocked on M001/M002 evidence. |
| Bootstrap M003 | Stays blocked on real adoption evidence/candidate review. |
| Build/Qualification M006 | Closed under accepted ADR-0005 Option A; implementation `398cd43`, hosted run 36484758546 green. |
| Eggup Interoperability M003 | Unaffected; remains ready to plan on its own merits. |

## Registry update rule

Keep this file compact: active/ready work, recent closure context, blockers, execution order, and baselines. Detailed requirements belong in source documents.
