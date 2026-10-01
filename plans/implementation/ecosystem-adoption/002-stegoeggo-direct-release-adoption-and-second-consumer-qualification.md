# Ecosystem Adoption Milestone 002 — stegoeggo Direct Release Adoption and Second-Consumer Qualification

Status: conditionally closed — consumer cutover landed; see `plans/closure/ecosystem-adoption/002-status.md`

Repository baseline: `32a0903936fcc283863e0bfb86151b13b4d75ce9`

Source roadmap:

- `plans/subsystems/ecosystem-adoption-roadmap.md`

Mirrored consumer baseline:

- `eggstack/stegoeggo@8c89e8cb1677a355d639ca1ead93c3dd2587e317`

Mirrored consumer plan:

- `eggstack/stegoeggo: plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md`

Hard/interface dependencies:

- Ecosystem M001 eggsact adoption — closed at `plans/closure/ecosystem-adoption/001-status.md`;
- Build/Qualification M006 — closed;
- CI M003e/M003f/M003g — closed;
- CI M003h integration/status reconciliation — closed at `plans/closure/ci-release-orchestration/003h-status.md`;
- Bootstrap M002a — closed;
- stegoeggo Plan 105 / release 0.4.2 five-target qualification — closed consumer evidence.

Operational dependency for full live closure:

- the next ordinary stable stegoeggo release newer than 0.4.2. Do not create a throwaway version merely to satisfy this milestone.

Primary class: capability / second-consumer adoption / operational qualification

## 1. Objective

Make stegoeggo the second independent real consumer of Eggpack's producer
release authority.

Replace stegoeggo's duplicated hand-written five-target binary release
authority with checked-in Eggpack configuration and generated release CI while
preserving stegoeggo-owned crates.io ordering, version/tag policy, installer
fallback/UX, self-update behavior, and human publication authority.

Use the migration to prove that the producer contract generalized beyond
eggsact without adding stegoeggo-specific logic to Eggpack.

## 2. Research finding: no new Eggpack producer primitive is currently required

The reviewed stegoeggo release shape fits the interfaces already exercised by
Ecosystem M001:

- five direct executable targets;
- Linux x86-64 and AArch64 built with cargo-zigbuild at glibc 2.17;
- native macOS x86-64/AArch64 and Windows x86-64 builds;
- one versionless public executable per target plus SHA-256 sidecar;
- product-owned `install.sh` / `install.ps1`;
- manual crates.io-first release authority;
- exact-tag manual dispatch;
- bounded native candidate execution;
- product-specific candidate validation;
- human publication after inspection.

The CLI package already declares `default = ["signatures"]`, so Eggpack's
existing package/bin build binding can produce the same distributed feature
set without a feature-flag extension. The legacy workflow's explicit
`--no-default-features --features signatures` is semantically equivalent to
a default-feature build of `stegoeggo-cli`.

Therefore M002 does not authorize an Eggpack Rust/API/schema change. If
implementation discovers a missing generic capability, stop and register a
separate Eggpack corrective/capability plan rather than special-casing
stegoeggo.

## 3. Ownership after adoption

### Eggpack owns

- the five-target producer contract;
- target build strategy, host architecture, toolchain pins, and compatibility
  floor declarations;
- Cargo command projection;
- canonical candidate handoff;
- native qualification evidence;
- invocation of the bounded stegoeggo consumer validator;
- finalized binary names and checksum sidecars;
- ReleaseManifest;
- generated exact-version installers;
- generated checked-in release workflow;
- draft GitHub Release creation/reconciliation;
- release-workflow drift detection.

### stegoeggo retains

- synchronized carrier/library/CLI version choice;
- crates.io publication order `stegoeggo-stego -> stegoeggo -> stegoeggo-cli`;
- the rule that crates.io publication completes before the matching tag;
- tag creation and dispatch timing;
- public installer wrapper behavior and install destinations;
- unsupported-target and exact-404 Cargo fallback;
- crates.io stable-version authority for `stegoeggo update`;
- Eggup acquisition/transaction/rollback behavior;
- product-specific protect/inspect/verify smoke semantics;
- the product decision on when a validated draft becomes public;
- Python/Node artifact workflows, which are not part of this native CLI
  producer migration.

## 4. Consumer baseline to preserve

Current public CLI matrix:

| Target | Legacy strategy/runner | Public asset |
|---|---|---|
| `x86_64-unknown-linux-gnu` | cargo-zigbuild, Linux x86-64, glibc 2.17 | `stegoeggo-x86_64-unknown-linux-gnu` |
| `aarch64-unknown-linux-gnu` | cargo-zigbuild, Linux AArch64, glibc 2.17 | `stegoeggo-aarch64-unknown-linux-gnu` |
| `x86_64-apple-darwin` | native Intel macOS | `stegoeggo-x86_64-apple-darwin` |
| `aarch64-apple-darwin` | native Apple Silicon | `stegoeggo-aarch64-apple-darwin` |
| `x86_64-pc-windows-msvc` | native Windows x86-64 | `stegoeggo-x86_64-pc-windows-msvc.exe` |

Each binary has a matching `.sha256` sidecar.

The legacy workflow uses Zig 0.13.0 but leaves cargo-zigbuild unpinned. M002
must not preserve that nondeterminism as a requirement. The migration should
reuse the already-live-qualified Eggpack cross-tool pair:

- Zig 0.14.1 with the exact official per-host archive digests already encoded
  by Eggpack policy;
- cargo-zigbuild 0.23.3;
- glibc floor 2.17.

This is an intentional deterministic-provisioning normalization, not a claim
of byte parity with 0.4.2. Before cutover, stegoeggo must prove the new pair
preserves target coverage, feature surface, native behavior, and the declared
glibc ceiling. If that proof fails, stop rather than weakening the release
contract.

## 5. Static release configuration

Use the proven consumer layout:

```text
release/eggpack/
  distribution.toml
  pack.toml
  build-bindings.toml
  qualification-bindings.toml
  consumer-validators.json
  installer-presentation.json
  install-policy.toml
  github-template.json
  github-policy.json
  workflow-shape.json
```

Static files contain no future version tag, release id, source SHA, artifact
digest, or mutable branch pin.

The generated workflow pins Eggpack to one exact qualified commit. Initial
planning baseline is current Eggpack `32a0903936fcc283863e0bfb86151b13b4d75ce9`;
implementation must re-review the pin if Eggpack production code advances
before cutover.

## 6. Distribution contract

Declare exactly the five current binary targets and exactly the public asset
names in §4.

Aliases should follow the established direct-consumer vocabulary:

- `linux-x64`;
- `linux-arm64`;
- `macos-x64`;
- `macos-arm64`;
- `windows-x64`.

Install names remain `stegoeggo` / `stegoeggo.exe`.

Do not add Python wheels, Node addons, unsupported source-only targets, or a
new executable form to M002.

## 7. Build policy and bindings

Use one direct build binding per target:

- Cargo package: `stegoeggo-cli`;
- binary: `stegoeggo`;
- logical selector: direct.

The CLI default feature set is already exactly `signatures`, matching the
legacy distributed CLI contract. Add a consumer-side/repository guard that
fails if the CLI default feature set drifts away from that assumption.

Build policy:

- Linux x86-64: CargoZigbuild, matching Linux x86-64 build/qualification host,
  glibc 2.17, Zig 0.14.1, cargo-zigbuild 0.23.3;
- Linux AArch64: CargoZigbuild, matching Linux AArch64
  build/qualification host, glibc 2.17, same exact tool versions;
- macOS x86-64/AArch64 and Windows x86-64: NativeCargo with matching native
  host and native qualification;
- support tier: required for all five.

Do not reduce native runner coverage merely to consolidate jobs.

## 8. Qualification and stegoeggo-owned consumer validation

Core Eggpack qualification should use one bounded direct candidate smoke:

```text
stegoeggo version
```

for every target.

Add a stegoeggo-owned validator, recommended
`scripts/smoke-release-binary.py PATH`, invoked by Eggpack for every exact
candidate. It should remain bounded and perform at least:

1. parse the expected workspace CLI version from the checked-out source;
2. require `PATH version` to equal `stegoeggo X.Y.Z`;
3. require `PATH --help` success;
4. protect the canonical checked-in PNG fixture with
   `--rights-policy prohibited-ai-ml-training --preset legal-notice`;
5. inspect the produced file successfully;
6. run `verify` when the produced evidence contract makes that valid;
7. clean temporary output.

For Linux candidates, preserve stegoeggo's existing release-level ABI evidence:
inspect required `GLIBC_*` symbol versions and fail if any requirement is
newer than 2.17. This is a **stegoeggo consumer validator**, not a change to
Eggpack's qualification semantics. Native smoke on a newer runner still does
not independently prove a compatibility floor.

Use a bounded local tool such as `readelf --version-info` or the existing
documented `strings | grep GLIBC_` method; absence/failure of the inspection
tool fails that Linux validator rather than silently dropping the check.

## 9. Installer presentation and public inventory

Use ProductWrappers presentation:

- `packaging/install.sh` -> public `install.sh`;
- `packaging/install.ps1` -> public `install.ps1`.

Keep those wrappers product-owned and preserve:

- latest vs explicit-version selection;
- exact-404/unsupported-target Cargo fallback only;
- checksum/network/candidate failures as hard failures;
- current destination/PATH behavior.

Eggpack also stages generated exact-version installers:

- `install-exact.sh`;
- `install-exact.ps1`.

The post-cutover native CLI draft inventory becomes 15 assets:

- 5 binaries;
- 5 checksum sidecars;
- `release-manifest.json`;
- `install.sh`;
- `install.ps1`;
- `install-exact.sh`;
- `install-exact.ps1`.

Update stegoeggo's release asset audit accordingly. Do not make
`release-check-assets.sh` reject the three new Eggpack-owned artifact forms.

## 10. Release-authority cutover

The legacy flow creates a GitHub Release first, then attaches assets with
`gh release upload --clobber`. That is not retained.

After M002:

1. maintainer publishes the three crates in dependency order;
2. maintainer creates/pushes the immutable exact `vX.Y.Z` tag;
3. maintainer manually dispatches the generated Eggpack workflow with that tag;
4. Eggpack creates/reuses a **draft** release and refuses same-name
   different-digest replacement;
5. maintainer inspects the complete draft;
6. maintainer explicitly publishes it.

Generated automation must never publish, force-move a tag, delete a public
release, or use a clobber path.

Before activating the generated workflow, render it to a non-triggering
location and compare target set, build/qualification hosts, toolchain/floor,
asset names, checksums, product wrappers, permissions, and product smoke.
Only then replace `.github/workflows/release-binaries.yml`. Do not leave two
active native binary release authorities.

## 11. Release-contract duplication cleanup

The current producer facts are duplicated among:

- `.github/workflows/release-binaries.yml`;
- `scripts/release-targets.txt`;
- `scripts/release-binary-preflight.sh`;
- `scripts/release-check-assets.sh`;
- installer target mapping;
- updater `TARGETS`.

After adoption:

- target/build/asset/checksum authority comes from `release/eggpack/`;
- preflight calls the pinned Eggpack drift/resolve checks rather than parsing
  target/asset strings out of generated YAML;
- asset audit derives expected producer inventory from the Eggpack contract or
  a checked-in/generated projection, not a second handwritten five-target
  table;
- updater and public installers remain self-contained product policy but gain
  parity tests against the Eggpack contract;
- `scripts/release-targets.txt` is retired as authority (delete it or make it
  an explicitly generated compatibility projection if a temporary migration
  need remains).

Do not move updater runtime target selection into Eggpack.

## 12. Ordinary CI drift guard

Add a lightweight release drift workflow/check that installs Eggpack from the
exact pinned revision and runs `eggpack ci check` against the checked-in
workflow/config.

The release preflight must run the same drift check.

No mutable `main`, floating tag, or unverified downloaded Eggpack binary is
an acceptable producer dependency.

## 13. Shared next-release evidence with stegoeggo Release-Distribution M001

Stegoeggo's existing M001 / flat Plan 106 is blocked only because no stable
release B newer than 0.4.2 exists. M002 does not depend on inventing B and may
implement now.

The first **ordinary** stable release after M002 cutover should be used for
both milestones:

### M002 evidence

- publish carrier/library/CLI manually;
- create immutable exact tag after crates.io visibility;
- dispatch generated Eggpack workflow;
- all five builds/qualifications/product validators pass;
- complete draft inventory is staged;
- Eggpack leaves it draft;
- maintainer publishes after inspection;
- exact/latest installer smoke succeeds.

### Existing stegoeggo M001 / Plan 106 evidence

After B is public and updater-ready:

- install the real public 0.4.2 binary in isolation;
- run its real `stegoeggo update`;
- require transition to B through current crates.io + GitHub endpoints;
- verify checksum, exact identity/version, rollback/failure semantics;
- verify already-current no-op on B;
- rerun deterministic updater regressions.

The same release event may satisfy both plans, but their closure records stay
separate and each records its own evidence.

## 14. Rerun/no-clobber evidence

Rerun the generated workflow for the same tag before publication when
practical.

Expected behavior is exact reuse. If a platform rebuild is not
byte-reproducible, Eggpack must refuse same-name/different-digest replacement.

A proven consumer-side nondeterminism finding does not authorize clobbering.
Record it in stegoeggo and create a consumer corrective if warranted. M002 may
close conditionally on such a clearly owned reproducibility issue if first-run
artifact correctness, qualification, inventory, publication, and fail-closed
rerun behavior are all proven; do not relabel it as an Eggpack defect without
evidence.

## 15. Out of scope

- changing crates.io publication authority;
- automated crate publication;
- changing updater version-selection or Eggup transaction semantics;
- migrating updater runtime mapping to ReleaseManifest;
- Python wheel or Node addon producer migration;
- code signing/notarization;
- expanding the five-target matrix;
- changing Eggpack schemas/API solely for stegoeggo;
- claiming Eggpack native qualification alone proves glibc 2.17;
- forcing closure of the existing stegoeggo M001 before an ordinary B release.

## 16. Verification

Eggpack producer:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
```

Stegoeggo minimum before cutover:

```text
./scripts/check.sh
cargo test -p stegoeggo-cli --all-features
./scripts/release-binary-preflight.sh --tag=<exact test/release tag as applicable>
./scripts/test-release-installers.sh
./scripts/test-release-updater.sh
eggpack ci check ...
```

Also require:

- generated-workflow parse/permission audit;
- target/asset/updater/installer parity tests;
- native five-target release run at operational closure;
- Linux glibc-symbol-floor validator;
- public asset audit after publication;
- ordinary stegoeggo CI on all M002 implementation commits.

Existing unrelated Python/Node binding CI failures at the reviewed stegoeggo
baseline are not reclassified as M002 defects. Any failure caused by M002
changes must be corrected before closure.

## 17. Acceptance criteria

M002 closes when:

- the mirrored stegoeggo M002 plan is registered and implemented;
- no Eggpack product-specific production change was required, or any newly
  discovered generic gap was separately planned/closed first;
- `release/eggpack/` is checked in and drift-gated;
- the generated workflow is the sole active native CLI binary producer;
- five target/asset names are preserved exactly;
- CLI distributed feature set remains `signatures`;
- deterministic Linux cross-tool pins are explicit and qualified;
- Linux glibc 2.17 evidence remains product-gated;
- exact candidates pass version/help/protect/inspect(/verify) validation;
- product wrappers retain existing latest/version/fallback semantics;
- updater semantics remain stegoeggo/Eggup-owned and target mapping is
  contract-parity-gated;
- release automation stages a draft and contains no clobber/tag/publication
  authority;
- the next ordinary stable release completes the live five-target draft and
  publication evidence;
- no unresolved medium-or-higher Eggpack producer regression remains.

Closure record:

`plans/closure/ecosystem-adoption/002-status.md`.

## 18. Stop conditions

Stop and re-plan if:

- generated Eggpack CI cannot express the current five-target native matrix;
- `stegoeggo-cli` requires build flags not representable by the current
  package/bin binding and default-feature equivalence is no longer true;
- the deterministic cross-tool normalization raises the Linux runtime floor or
  breaks a supported target;
- preserving installer/updater semantics requires moving product policy into
  Eggpack;
- migration requires auto-publishing crates or auto-publishing the GitHub
  release;
- the active legacy and generated workflows would overlap for a real release;
- a new generic Eggpack capability is required.

## 19. Closure evidence

Record:

- final Eggpack pin;
- stegoeggo implementation SHA;
- before/after release-authority map;
- config inventory;
- five-target build/qualification matrix;
- deterministic cross-tool versions/digests;
- Linux GLIBC ceiling evidence;
- candidate product-validator evidence;
- public wrapper and updater parity evidence;
- generated workflow drift/permission evidence;
- live release version/tag/run;
- first staging receipt and 15-asset inventory;
- rerun reuse or fail-closed nondeterminism disposition;
- human-publication evidence;
- exact/latest installer smoke;
- relationship to stegoeggo M001 / Plan 106 A→B evidence;
- unresolved findings by severity/owner;
- disposition for Ecosystem M003 eggsearch planning.
