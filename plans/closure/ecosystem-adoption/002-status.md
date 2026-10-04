# Ecosystem Adoption Milestone 002 — Closure Status

Status: closed

Condition-discharge annotation (2026-10-04, Planning Hygiene M001): the sole remaining condition — an ordinary stable StegoEggo release newer than 0.4.2 supplying live five-target draft/publication evidence — has occurred. StegoEggo published `v0.5.0` on 2026-10-04T06:34:19Z from source `57ca94c910269e080b06aa8cb34c3767b3bf669d`, produced by the Eggpack-generated pipeline (hosted run `37181914252`), carrying the exact 15-asset inventory including `release-manifest.json` and both Eggpack-generated exact installers, manually published after inspection, with a real public `0.4.2 -> 0.5.0` updater transition. M002 is **closed**; §1-§8 below are preserved unchanged as the conditional-closure record. Full criterion-by-criterion mapping, including the explicit disposition of item 6, is in §9.

Source plan:

- `plans/implementation/ecosystem-adoption/002-stegoeggo-direct-release-adoption-and-second-consumer-qualification.md`

Source roadmap:

- `plans/subsystems/ecosystem-adoption-roadmap.md`

Eggpack planning baseline:

- `56ed7e747fd39e4d6a32a9f1fe3e09dd44355069`

Consumer implementation evidence:

- `eggstack/stegoeggo@3b96fae753d003d5bf916141ff777c4102428593` — Eggpack producer adoption implementation;
- `eggstack/stegoeggo@c75132a09a0079c857a5b241dfbe254747ddb84d` — consumer M002 conditional closure;
- consumer closure: `eggstack/stegoeggo: plans/closure/release-distribution/002-status.md`;
- consumer standard CI: run `36893532265` green on `c75132a`;
- consumer release-drift guard: run `36893532365` green on `c75132a`.

Consumer follow-up corrective:

- plan: `eggstack/stegoeggo: plans/implementation/release-distribution/003-synchronous-eggup-updater-bridge-corrective.md`;
- registration: `eggstack/stegoeggo@622c36b6fcc21f3364deb536c495f0be50e334ab`;
- implementation: `eggstack/stegoeggo@e611c913f97c804620f26c7514d59e0cb84d34cc`;
- closure: `eggstack/stegoeggo@f80eebe3fb983a5e8b13f7e082fe573636d65756`;
- closure record: `eggstack/stegoeggo: plans/closure/release-distribution/003-status.md`;
- hosted CI: run `36902385826` green;
- hosted release-drift: run `36902385784` green.

## 1. Executive finding

Ecosystem M002's implementation/cutover portion is complete and the milestone
is conditionally closed.

StegoEggo is now the second independent repository to adopt Eggpack as producer
authority for its native CLI binary release. The consumer checked in
`release/eggpack/` configuration, replaced its handwritten five-target
producer workflow with generated Eggpack CI, added an immutable Eggpack drift
guard, moved release-contract checks off the handwritten target table, and
retained product ownership of crates.io ordering, public installer behavior,
self-update policy, and human publication.

No Eggpack Rust/API/schema change was required. That is the key second-consumer
architectural result: the direct-binary producer model generalized beyond
eggsact without adding StegoEggo-specific production behavior to Eggpack.

Full operational closure remains intentionally outstanding until the next
ordinary stable StegoEggo release newer than 0.4.2 provides the live
five-target draft/publication evidence required by the source plan.

## 2. Second-consumer implementation evidence

The consumer M002 closure records:

- 10 checked-in `release/eggpack/` configuration files;
- exact immutable Eggpack pin
  `56ed7e747fd39e4d6a32a9f1fe3e09dd44355069`;
- five direct targets:
  - Linux x86-64;
  - Linux AArch64;
  - macOS x86-64;
  - macOS AArch64;
  - Windows x86-64;
- deterministic Linux producer policy:
  - Zig 0.14.1;
  - cargo-zigbuild 0.23.3;
  - glibc floor 2.17;
- native qualification on matching target hosts;
- bounded StegoEggo candidate validation;
- consumer-owned final-ELF GLIBC ceiling validation;
- product wrappers plus Eggpack generated exact installers;
- one write-authorized draft staging job;
- no release clobber, publication, or tag-mutation authority;
- a 15-asset post-cutover release inventory contract;
- removal of `scripts/release-targets.txt` as producer authority;
- 15 dedicated `release_eggpack` regression tests.

This is independent second-repository evidence for the same generic producer
interfaces used by eggsact M001.

## 3. Authority disposition

### Eggpack now owns for StegoEggo

- five-target native producer contract;
- build strategy/toolchain/floor projection;
- canonical candidate handoff;
- native qualification evidence;
- consumer-validator orchestration;
- binary/checksum finalization;
- ReleaseManifest generation;
- exact installer generation;
- generated workflow shape;
- draft staging/reconciliation;
- workflow drift detection.

### StegoEggo still owns

- synchronized crate versioning;
- crates.io publication order;
- exact release tag creation;
- dispatch timing;
- public installer wrapper semantics;
- Cargo fallback policy;
- crates.io stable-version authority;
- Eggup-backed self-update transaction behavior;
- protect/inspect/verify smoke semantics;
- human publication.

No ownership boundary moved into Eggpack merely to complete adoption.

## 4. Verification disposition

Consumer closure records successful:

- `eggpack ci check` with zero generated-workflow drift;
- release-contract parity checks;
- bounded release-binary smoke;
- mock 15-asset audit accept/reject paths;
- `./scripts/check.sh`;
- CLI all-feature tests;
- installer rehearsal;
- release preflight;
- Rust 1.89 checks;
- cargo-deny;
- hosted standard CI;
- hosted release-drift guard.

No new medium-or-higher Eggpack producer defect was found.

## 5. Consumer-owned updater corrective

The consumer M002 closeout also reproduced a pre-existing medium-severity
StegoEggo updater panic.

Ownership review after closure showed:

- StegoEggo creates a Tokio runtime in `run_update`;
- that runtime enters `run_update_async`;
- the updater then calls Eggup's synchronous `eggup-eggfetch` seam;
- Eggup's sync adapter owns its own private current-thread runtime;
- `update_to` is declared async despite containing no await points.

This produces a nested-runtime panic and is consumer integration debt, not an
Eggpack producer defect and not presently evidence of an Eggup adapter defect.

StegoEggo Release-Distribution M003 implemented the synchronous bridge at
`e611c913f97c804620f26c7514d59e0cb84d34cc` and closed at
`f80eebe3fb983a5e8b13f7e082fe573636d65756`. Its closure records a green
end-to-end updater rehearsal, ten focused seam/runtime regressions, direct
Tokio moved out of production dependencies, zero Eggpack workflow drift, and
green hosted CI/release-drift runs `36902385826` / `36902385784`.

The updater-runtime code blocker is therefore resolved. Ecosystem M002 remains
conditionally closed only because the producer cutover has not yet been
exercised by an ordinary stable B > 0.4.2 live release.

## 6. Outstanding operational evidence

Full M002 closure still requires the next **ordinary** stable StegoEggo release
B > 0.4.2.

That release must provide:

1. normal manual crates.io publication in product-owned dependency order;
2. exact immutable tag;
3. generated Eggpack five-target build/qualification/consumer-validation run;
4. complete 15-asset draft;
5. staging receipt;
6. rerun exact reuse or correctly owned fail-closed nondeterminism finding;
7. human publication after inspection;
8. exact/latest public installer smoke;
9. no unresolved medium-or-higher Eggpack producer regression.

No throwaway stable release is authorized.

The same ordinary B is expected to provide StegoEggo Release-Distribution M001
/ flat Plan 106's real public 0.4.2 -> B updater proof. StegoEggo M003 is
already closed, so no updater-runtime code blocker remains before that event.

## 7. Downstream disposition

The second-consumer **implementation** evidence now exists, but the ordered
ecosystem roadmap still requires M002's live release evidence before advancing
to M003 eggsearch target/qualification diversity.

Therefore:

- Ecosystem M003 remains blocked on full M002 live evidence;
- Bootstrap M003 remains blocked on real two-consumer adoption/receipt evidence;
- no Eggpack production corrective is opened;
- StegoEggo Release-Distribution M003 is closed and the updater rehearsal is
  green; the remaining M002 blocker is operational only;
- Eggup Interoperability M003 remains independently ready in Eggup/Eggsact and
  is not blocked by StegoEggo M002.

## 8. Final status

**Conditionally closed.**

The second-consumer cutover is landed, verified, and does not require a new
Eggpack producer primitive. The consumer-owned updater corrective is also
closed. The sole remaining M002 condition is the next ordinary StegoEggo
stable B > 0.4.2 release event needed for live operational evidence.

## 9. Live release condition discharged — full closure (2026-10-04, Planning Hygiene M001)

§1-§8 above are preserved exactly as written and remain accurate for their date. This section discharges the remaining condition and closes the milestone.

### 9.1 The live event

The expected ordinary stable B landed as **`v0.5.0`**, not the `0.4.3` that this closure originally anticipated. StegoEggo attempted `0.4.3`, stopped before publication (`eggstack/stegoeggo@0e5235d3`, "plans: block 0.4.3 release on root API semver findings" — two stable public enum changes were not semver-compatible), and selected `0.5.0` as the next `0.x` minor boundary. No `0.4.3` artifact was ever created. This is the same live event M002 was waiting for; only the version number differs, and the consumer recorded the substitution itself in Release-Distribution M005.

Verified independently for this closure:

- release `v0.5.0`, non-draft, non-prerelease, published `2026-10-04T06:34:19Z`;
- tag `v0.5.0` is lightweight and points at `57ca94c910269e080b06aa8cb34c3767b3bf669d`;
- the published `release-manifest.json` records `schema_version: 1`, `product_id: stegoeggo`, `release_id: v0.5.0`, `source_revision: 57ca94c910269e080b06aa8cb34c3767b3bf669d` — an exact match to the tag, with `direct`-form per-target artifact name/size/SHA-256 and `install` binding;
- hosted run `37181914252` (`Eggpack candidate builds`, `workflow_dispatch`, `head_sha` `57ca94c9…`, conclusion `success`) — the tag source and the build source are the same commit;
- 15 published assets: 5 target binaries, 5 `.sha256` sidecars, 2 product wrapper installers, 2 Eggpack-generated exact installers, 1 `release-manifest.json`;
- `stegoeggo-stego`, `stegoeggo`, and `stegoeggo-cli` are all at `0.5.0` on crates.io, confirmed by direct registry readback;
- consumer closure: `eggstack/stegoeggo: plans/closure/release-distribution/005-status.md` (`7bde933b`), which closes Release-Distribution M001, M002, M003, and M005, marks M004 superseded, and declares the consumer subsystem closed.

### 9.2 The nine required items from §6

| # | Required item | Disposition | Evidence |
|---|---|---|---|
| 1 | normal manual crates.io publication in product-owned dependency order | met | `stegoeggo-stego` → `stegoeggo` → `stegoeggo-cli` 0.5.0, each verified by registry readback before tagging; no automation holds publication credentials |
| 2 | exact immutable tag | met | `v0.5.0` → `57ca94c9…`, pushed once, matching the manifest `source_revision` exactly |
| 3 | generated Eggpack five-target build/qualification/consumer-validation run | met | Run `37181914252`: preflight, resolve, five builds, clean-host qualifications, five validations, gate, aggregate, and stage all succeeded |
| 4 | complete 15-asset draft | met | Staging receipt `release_id=v0.5.0`, source SHA matched, `uploaded=15, reused=0`; the published release carries exactly those 15 assets |
| 5 | staging receipt | met | Receipt artifact `eggpack-staging-receipt` from run `37181914252` |
| 6 | rerun exact reuse **or** correctly owned fail-closed nondeterminism finding | **not exercised — substituted, see §9.3** | Run `37181914252` was attempt 1 only; no rerun and no nondeterminism event arose |
| 7 | human publication after inspection | met | The draft was manually published after inspection; Eggpack never published |
| 8 | exact/latest public installer smoke | met | Downloaded public `stegoeggo-x86_64-apple-darwin` reports `stegoeggo 0.5.0`; asset audit passed; protect→inspect→verify smoke passed post-update |
| 9 | no unresolved medium-or-higher Eggpack producer regression | met | Consumer M005 records no critical/high/medium/low finding in the release scope; the one parallel-test cleanup failure was transient and passed in isolation and in the serial stage |

Eight of nine items are met by the release itself.

### 9.3 Item 6 disposition — explicit substitution, not a silent pass

Item 6 was written as a disjunction whose branches were "exact rerun reuse" or "a correctly owned fail-closed nondeterminism finding". `v0.5.0` satisfied **neither** branch: it was a clean first attempt, so no rerun was needed and no nondeterminism appeared.

It is recorded as substituted rather than met, on the following basis. Item 6 exists to prove that the producer's rerun-reuse and digest-refusal path behaves correctly against a real consumer's artifacts — not to force a rerun on a release that did not need one. That path is now proven on real consumer bytes by Eggsact Distribution M005a: rehearsal run `36886042696` attempt 2 reused draft `401132612` with `created: false, uploaded: 0, reused: 15` and identical digests for all 15 assets, zero refusals. The same generated code path, the same fail-closed refusal, the same receipt schema; a different consumer's binaries. The same evidence independently discharges CI M003b and Phase 8 (`plans/closure/ci-release-orchestration/003b-status.md` §6).

Had the rerun path not been independently proven, this item would have kept M002 conditionally closed regardless of how clean the 0.5.0 release was. It is called out here so a later reader can disagree with the substitution rather than discover it silently.

What StegoEggo did **not** do is recorded too: the Windows A→B updater transition was not performed. The requirement was one supported target, which macOS x86-64 satisfies, and no Windows transition is claimed.

### 9.4 Authority boundary held

Nothing moved into Eggpack to close this milestone. Crates.io ordering, version authority, exact tag creation, dispatch timing, publication, and updater/fallback policy all remained StegoEggo-owned. The maintainer selected `0.5.0` over `0.4.3` on semver grounds, which is precisely a product-owned release decision Eggpack has no authority over and did not influence. No Eggpack production, workflow, package, or schema change was made or required to accept this evidence.

### 9.5 Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| Info | Item 6's exact rerun-reuse branch was never exercised on a StegoEggo release | Substituted by eggsact M005a evidence; see §9.3 |
| Info | Windows A→B updater transition not performed | Accepted; one supported target was required and satisfied |
| Info | The 0.5.0 line sits on `release/0.5.0` (`7bde933b`), 8 commits ahead of consumer `main` (`a01a022`), and is not yet merged there | StegoEggo-side housekeeping. Does not affect this closure: tag, manifest `source_revision`, and run `head_sha` all agree on `57ca94c9…` |
| None | No medium-or-higher Eggpack producer defect | Confirmed by consumer M005 release-scope findings |

### 9.6 Final status

**Closed.**

The second-consumer cutover was landed and verified without a new Eggpack producer primitive, the consumer-owned updater corrective was closed, and the live ordinary stable release `v0.5.0` supplied the five-target draft, staging receipt, manual publication, public installer, and real `0.4.2 -> 0.5.0` updater evidence that the milestone required. The direct-binary producer model is now proven end to end on two independent repositories, which is the ecosystem milestone's actual architectural claim.
