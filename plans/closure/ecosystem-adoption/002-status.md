# Ecosystem Adoption Milestone 002 — Closure Status

Status: conditionally closed

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
