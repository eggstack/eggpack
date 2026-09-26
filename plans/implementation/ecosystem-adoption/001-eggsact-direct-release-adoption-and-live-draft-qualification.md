# Ecosystem Adoption Milestone 001 — eggsact Direct Release Adoption and Live Draft Qualification

Status: ready

Repository baseline: `16118c5896519ae10e3d296e77d5974869d89354`

Source roadmap:

- `plans/subsystems/ecosystem-adoption-roadmap.md`

Mirrored consumer baseline:

- `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`

Mirrored consumer plan:

- `eggstack/eggsact: plans/implementation/distribution-update-release/005-eggpack-producer-adoption-and-live-draft-qualification.md` (must be registered before implementation)

Hard dependencies:

- CI M003d consumer release composition seam — closed at `plans/closure/ci-release-orchestration/003d-status.md`;
- Build M005 deterministic cross-tool provisioning — closed at `plans/closure/build-qualification/005-status.md` (implementation `7a206ba`, hosted run 36256831000);
- CI M003c — closed;
- Bootstrap M002a — closed;
- Build/Qualification M003/M004 — closed.

Operational dependency:

- one maintainer-authorized real eggsact version tag created through the normal crates.io-first release process.

Primary class: capability / first-consumer adoption / operational qualification

## 1. Objective

Make eggsact the first real Eggpack producer consumer.

Replace duplicated eggsact producer-side release authority with checked-in Eggpack configuration and generated release CI while preserving eggsact-owned release selection, installer fallback/UX, self-update policy, crates.io ordering, and manual public publication.

Use the first normal eggsact release after implementation as the real GitHub draft qualification required to fully close CI M003b / Phase 8.

## 2. Ownership after adoption

### Eggpack owns

- five-target release contract;
- target build strategy/host/tool versions/floors;
- Cargo build command projection;
- core candidate qualification/evidence;
- bounded eggsact consumer-validator orchestration;
- final artifact names + checksum sidecars;
- ReleaseManifest;
- generated exact-release installers;
- generated checked-in release workflow;
- draft GitHub Release staging/reconciliation.

### eggsact retains

- crates.io version authority and manual publication;
- tag creation only after crates.io publication succeeds;
- decision when to dispatch binary assembly;
- public `install.sh` / `install.ps1` wrapper behavior;
- latest/version selection;
- Cargo fallback policy;
- install destinations/PATH UX;
- `eggsact update` version selection and Eggup transaction semantics;
- MCP handshake validation script semantics;
- final human draft review and publication.

No Eggpack code may absorb those eggsact product policies merely to simplify adoption.

## 3. Adoption baseline to preserve

Current eggsact release matrix:

| Target | Host/strategy | Published asset |
|---|---|---|
| x86_64-unknown-linux-gnu | Linux x86-64, cargo-zigbuild, glibc 2.17 | `eggsact-x86_64-unknown-linux-gnu` |
| aarch64-unknown-linux-gnu | Linux AArch64, cargo-zigbuild, glibc 2.17 | `eggsact-aarch64-unknown-linux-gnu` |
| x86_64-apple-darwin | macOS Intel native | `eggsact-x86_64-apple-darwin` |
| aarch64-apple-darwin | macOS Apple Silicon native | `eggsact-aarch64-apple-darwin` |
| x86_64-pc-windows-msvc | Windows x86-64 native | `eggsact-x86_64-pc-windows-msvc.exe` |

Every binary has a matching `.sha256` sidecar.

Linux cross tools:

- Zig 0.14.1;
- cargo-zigbuild 0.23.3;
- exact official archive digest per runner architecture.

ARMv7 remains recognized by product installers/updater as a Cargo-fallback-only host and is NOT added to the Eggpack published target set in M001.

## 4. Release configuration layout

After M003d/M005 interfaces close, add a single checked-in eggsact release configuration root, recommended:

```text
release/eggpack/
  distribution.toml
  pack.toml
  build-bindings.toml
  qualification-bindings.toml
  consumer-validators.toml
  installer-presentation.toml
  github-template.json
  github-policy.json
  workflow-plan.json
```

Exact filenames follow the final M003d interfaces.

Rules:

- static files contain no future release tag or source revision;
- workflow plan is reusable across future releases;
- runtime ReleasePlan/GitHubDraftPolicy are generated into workflow-private storage from the exact selected tag + HEAD;
- no checked-in file attempts to contain its own Git SHA;
- all config paths are explicit and drift-checked.

## 5. DistributionContract

Define direct release outputs for exactly the five current binary targets.

Artifact names must match the existing public contract exactly.

Install name for direct generated exact bootstrap remains `eggsact` / `eggsact.exe` as appropriate.

No version is inserted into current binary asset filenames.

No ARMv7 artifact is declared.

Contract fixtures/tests compare the expected Eggpack release inventory with the current eggsact release-contract table before the legacy workflow is replaced.

## 6. PackConfig / toolchain policy

Configure:

### Linux x86-64

- strategy: CargoZigbuild;
- host: Linux x86-64;
- Rust: final repository release toolchain policy;
- cargo-zigbuild: 0.23.3;
- Zig: 0.14.1;
- glibc floor: 2.17;
- support: required;
- qualification: native on x86-64 Linux.

### Linux AArch64

- strategy: CargoZigbuild;
- host: Linux AArch64;
- cargo-zigbuild: 0.23.3;
- Zig: 0.14.1;
- glibc floor: 2.17;
- support: required;
- qualification: native on AArch64 Linux.

### macOS Intel / Apple Silicon / Windows x86-64

- strategy: NativeCargo;
- matching native host;
- support: required;
- native qualification.

Do not add unsupported host emulation merely to reduce runner count.

## 7. Build/qualification bindings

One direct output per target:

- Cargo package: eggsact;
- binary: eggsact;
- direct logical selector.

Core candidate smoke must preserve a bounded CLI-level check. At minimum require a deterministic candidate invocation sufficient to prove it executes and identifies correctly; if one fixed argv cannot cover both current `--version` and `--help`, keep the stronger identity check in the core smoke and cover help in consumer validation/repository tests without weakening current release evidence.

M003d consumer validation runs:

`scripts/smoke-mcp-binary.py <exact candidate>`

against every natively executable required target.

This validation must gate aggregation.

## 8. Installer presentation

Use M003d ProductWrappers mode.

Public product-owned assets:

- `packaging/install.sh` -> `install.sh`;
- `packaging/install.ps1` -> `install.ps1`.

Eggpack-generated exact release installers:

- `install-exact.sh`;
- `install-exact.ps1`.

Public wrappers remain unchanged in semantic ownership:

- no version argument => latest release;
- explicit version => exact release;
- unsupported host => Cargo fallback;
- exact binary 404 => Cargo fallback;
- checksum/TLS/5xx/version failures => hard error;
- eggsact-owned destination/PATH behavior remains.

M001 does not require wrappers to delegate to the generated exact installers. That may be considered only after adoption evidence exists.

## 9. Generated workflow initial trigger policy

Use `StagingTagSource::DispatchInput` for first adoption.

Initial generated release workflow is manual-dispatch only.

Rationale:

- crates.io publication remains an explicit maintainer action;
- tag remains created only after successful crates.io publication;
- maintainer can wait for crates.io indexing/visibility before binary assembly;
- the first live Eggpack draft can be deliberately authorized and observed;
- no legacy tag-trigger race occurs during cutover.

The required input is the exact existing `vX.Y.Z` tag.

A later follow-up may switch eggsact to RefName/tag-push staging after M001 closes. Do not mix that ergonomic change into first adoption.

## 10. Runtime release identity

For a dispatch:

1. checkout exact `release_tag`;
2. resolve HEAD commit;
3. M003d runtime resolver creates ReleasePlan with:
   - `release_id = exact tag`;
   - `source_revision = exact HEAD`;
   - five selected canonical targets;
4. create exact GitHub draft policy from the static eggsact template;
5. upload runtime identity documents internally;
6. every subsequent job consumes the same documents and verifies source identity.

Eggpack must never infer the release from the dispatch branch.

## 11. Eggpack tool pin

Generated eggsact workflow pins Eggpack to the exact M003d/Build-M005-qualified implementation revision.

No floating main branch, tag, or crate version.

The tool pin is updated only by an explicit eggsact release-maintenance plan/corrective.

## 12. Legacy workflow cutover

Current `.github/workflows/release-binaries.yml` remains production authority until all static parity checks pass.

Implementation sequence:

1. create Eggpack config;
2. generate candidate workflow to a temporary/non-triggering path;
3. compare target matrix, runner architecture, build floors, artifact names, checksums, qualification, permissions, installer assets, draft-only authority;
4. run Eggpack drift check;
5. run eggsact release-contract/merge gates;
6. only then replace the active `.github/workflows/release-binaries.yml` with the generated workflow.

Do not leave both old and new active release workflows.

Git history plus this adoption closure is sufficient predecessor evidence; no active legacy YAML copy is required.

## 13. Release-contract script migration

Refactor `scripts/check-release-contract.py` so it no longer duplicates producer facts that become Eggpack authority.

Remove assertions whose only purpose is checking the hand-written workflow contains:

- each target;
- each asset name;
- Zig setup shell fragments;
- direct `contents: write` release assembly details.

Retain/replace guards for eggsact-owned policy:

- public wrappers still expose expected latest/version/fallback semantics;
- updater remains self-contained and no-curl;
- updater mapping remains compatible with the published target contract;
- ARMv7 remains fallback-only;
- release workflow remains generated/drift-checked through Eggpack;
- only stage gets write authority.

If practical, add a release-contract fixture exported from Eggpack config rather than maintaining a second five-target list in Python.

## 14. Ordinary CI drift guard

Add a repository CI guard that runs `eggpack ci check` against the pinned Eggpack revision and checked-in generated release workflow.

Avoid mutable Eggpack installation.

The final mechanism may:

- install pinned Eggpack CLI into a runner temp root with Cargo and cache it by revision; or
- use another immutable Eggpack provisioning mechanism established by M003d.

The release workflow itself also runs a preflight drift check before builds.

Local `scripts/release-check.sh` should expose a deterministic way to run the same check when the pinned Eggpack CLI is available; do not silently download mutable main.

## 15. Live draft qualification / Phase 8 evidence

Use the first maintainer-authorized normal eggsact release after implementation.

Release ordering remains:

1. clean source/release checks;
2. manual `cargo publish --locked`;
3. confirm crates.io accepted/indexed the exact version;
4. create annotated `vX.Y.Z` tag at the exact verified commit;
5. push tag;
6. manually dispatch generated Eggpack workflow with that exact tag;
7. inspect draft;
8. rerun the same workflow/tag;
9. verify exact draft/assets are reused without clobber;
10. keep release unpublished until maintainer review is complete.

Required draft inventory:

- five binaries;
- five checksum sidecars;
- `release-manifest.json`;
- public `install.sh`;
- public `install.ps1`;
- `install-exact.sh`;
- `install-exact.ps1`.

The draft must remain draft after both first run and rerun.

This run is the outstanding real GitHub evidence for Eggpack CI M003b.

## 16. Publication/post-publication evidence

Eggpack never publishes the draft.

If the maintainer publishes it as part of the normal eggsact release:

- verify exact-tag public installer URLs;
- verify `releases/latest/download/install.sh`;
- verify `releases/latest/download/install.ps1`;
- run pinned wrapper install smoke on representative supported hosts where practical;
- confirm Cargo fallback policy remains unchanged;
- confirm `eggsact update` still resolves the published version.

If publication has not yet happened, eggsact M005 may remain conditionally closed while Eggpack Phase 8 can still close on draft-only evidence.

## 17. Self-update boundary

Do not migrate `src/update.rs` to Eggpack in M001.

Retain:

- crates.io `max_stable_version` authority;
- exact GitHub asset/checksum URL policy;
- 404-only Cargo fallback;
- Eggup acquisition/install transaction;
- current target mapping.

Add tests comparing updater published-target names with the Eggpack contract/config to prevent drift, but keep runtime product policy local.

Eggup Interoperability M003 can revisit deeper manifest-based consumption separately.

## 18. Verification

Eggpack-side:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
```

eggsact-side minimum:

```bash
cargo fmt --all -- --check
cargo run --locked --features dev-tools --bin generate-docs -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features -- --skip parity --test-threads=4
cargo test --locked --doc
python3 scripts/check-release-contract.py
bash -n packaging/install.sh
scripts/release-check.sh
```

Plus:

- `eggpack ci check`;
- generated-workflow YAML parse/permission audit;
- Linux x86-64 real CargoZigbuild provisioning/build/smoke;
- Linux AArch64 real CargoZigbuild provisioning/build/smoke;
- native Intel/ARM macOS checks;
- native Windows x86-64 checks;
- consumer MCP smoke on exact candidates.

## 19. Acceptance criteria

Ecosystem M001 closes only when:

- M003d and Build M005 are closed and pinned;
- eggsact has checked-in reusable Eggpack release configuration;
- generated workflow replaces the old hand-maintained matrix;
- no future source SHA/tag is checked into static workflow config;
- all five current target/assets remain exact;
- glibc 2.17 / Zig 0.14.1 / cargo-zigbuild 0.23.3 parity holds;
- exact candidate MCP handshake remains a gating check;
- public product wrappers retain latest/version/Cargo fallback semantics;
- generated exact installers coexist under non-public-wrapper names;
- updater semantics remain consumer-owned and unchanged;
- real eggsact draft staging succeeds;
- exact rerun reuses draft/assets without clobber;
- draft remains unpublished by Eggpack;
- no unresolved medium-or-higher adoption regression remains.

After draft evidence:

- CI M003b may fully close;
- Phase 8 may exit;
- eggsact is the first qualified Eggpack consumer;
- Ecosystem M002 stegoeggo becomes eligible for planning.

## 20. Stop conditions

Stop and re-plan if:

- M003d cannot preserve public wrapper semantics;
- Build M005 cannot reproduce eggsact cross-tool versions/digests;
- generated workflow cannot express all five current targets/runners;
- exact MCP candidate smoke cannot remain gating;
- adoption requires moving crates.io publication/tag authority into Eggpack;
- updater policy must be changed to complete producer migration;
- live draft requires `--clobber`, tag mutation, or automatic publication;
- generated workflow and legacy workflow would both be active for the same release event.

## 21. Closure evidence

Create:

`plans/closure/ecosystem-adoption/001-status.md`

Record:

- Eggpack implementation pin;
- eggsact implementation SHA;
- before/after authority map;
- static release config inventory;
- generated workflow diff/parity evidence;
- five-target artifact matrix;
- toolchain provisioning evidence;
- MCP consumer-validation evidence;
- public wrapper parity evidence;
- updater parity evidence;
- real GitHub draft run id/tag;
- exact staged asset inventory/digests;
- rerun receipt/reuse evidence;
- proof draft remained unpublished by Eggpack;
- eggsact hosted CI/release checks;
- unresolved findings;
- CI M003b/Phase 8 closure disposition;
- stegoeggo M002 readiness disposition.
