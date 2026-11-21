# Workspace Release M001 — Eggpack 0.1.2 Crates.io Publication and GitHub Source Release

Status: active

Subsystem roadmap: `plans/subsystems/workspace-release-roadmap.md`

Repository baseline: `23ebb10941bb2b38b808f61edddff3ed3d1f2961` — merged `main` commit after PR [#6](https://github.com/eggstack/eggpack/pull/6).

## 1. Objective

Publish the six previously unpublished Eggpack workspace crates at their existing source version `0.1.2`, in dependency order, and create one public GitHub source-only release tagged `v0.1.2` at the exact reviewed source commit. Do not republish `eggpack-manifest 0.1.2`, which is an independently versioned crate and is already published.

The user approved the six-crate crates.io publication, a GitHub `v0.1.2` release containing source only, and merging PR #6 first. The user explicitly declined prebuilt CLI binaries. This plan does not broaden that scope.

## 2. Why ready and current evidence

The prior active implementation corrective, CI/Release M003pb, is closed in `plans/closure/ci-release-orchestration/003pb-status.md`. PR #6 merged to `main` at `23ebb10941bb2b38b808f61edddff3ed3d1f2961`; its tree hash `880352cfea924f0434e4b176cbcf8a81b7fdbd3c` equals the exact implementation tree at `edce774bf3597a37789ba59587cc2a5be56e3cee`. Main push run [44941146518](https://github.com/eggstack/eggpack/actions/runs/44941146518) passed all four lanes on the exact merge SHA; push run [44935151048](https://github.com/eggstack/eggpack/actions/runs/44935151048) and PR run [44935155340](https://github.com/eggstack/eggpack/actions/runs/44935155340) passed on the exact implementation SHA.

The source version for the six workspace crates is already `0.1.2`. The six crate IDs were absent from crates.io at plan opening; `eggpack-manifest 0.1.2` is already published. No project tag `v0.1.2` or GitHub release exists. The manifest tag `eggpack-manifest-v0.1.2` is separate and must remain untouched.

The repository has no self-release workflow for Eggpack. The release is therefore an explicit, manually controlled Cargo and GitHub operation, consistent with ADR-0003’s human publication gate. No automatic publication workflow or new permission is introduced.

## 3. Invariants

- Publish exactly `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-github`, `eggpack-ci`, and `eggpack-cli`, each at `0.1.2`.
- Publish from the clean, reviewed final `main` source SHA after this plan/closure change is merged and hosted CI passes on that exact SHA.
- Preserve the independent `eggpack-manifest 0.1.2` release and its existing `eggpack-manifest-v0.1.2` tag.
- Use topological, sequential publication. Do not publish a dependent crate until its required registry dependencies are visible and verified.
- Never use `--allow-dirty` for package publication; never issue bare `cargo publish`; never blind-retry an operation with an uncertain result.
- Create `v0.1.2` only if absent, point it at the exact source commit, and never move or overwrite it.
- The GitHub Release is public, stable, and source-only: no custom binaries or other uploaded assets. GitHub-generated source archives are expected.
- Record SHA-256 checksums and registry states as integrity facts only. Make no signature, authenticity, or provenance claim.
- Do not change package versions, dependency declarations, code, generated workflows, schemas, or downstream repositories.

## 4. Scope

### In scope

- Reconfirm the six package IDs, versions, dependency edges, and absence from crates.io.
- Run the canonical local gate and clean-tree package verification for all six crates.
- Publish each crate once in the approved dependency order and verify its registry state before continuing.
- Create and verify the annotated `v0.1.2` tag and public source-only GitHub Release after all six registry publications succeed.
- Record the exact source SHA, hosted run, per-crate checksum/yanked state, publication order, tag peel, release URL, and external consumer verification in the closure and release documentation.

### Out of scope

- Republishing or altering `eggpack-manifest`.
- Prebuilt platform binaries, release signing, attestations, or provenance.
- Automatic crates.io/GitHub publishing workflows.
- Version, API, dependency, schema, build, or runtime changes.
- Updating `eggup-eggpack =0.1.0` or any other consumer-owned pin.

## 5. Required production changes

None. The package versions are already set in the reviewed source. Only planning, closure, and current-facing release documentation are changed. Do not modify the generated CI workflow or introduce release automation.

## 6. Ordered work packages

1. Close M003pb and register this release plan. Merge the documentation PR through the normal GitHub review path.
2. Fetch the exact resulting `main` commit. Confirm a clean tree, unchanged package versions/dependencies, no existing `v0.1.2` tag/release, and no existing 0.1.2 entries for the six crates.
3. Run `scripts/check-local.sh` and `git diff --check` on the release tree. Require its hosted CI on exact SHA `main` (Linux stable, Linux 1.89.0, macOS, Windows).
4. Run clean `cargo package` verification for all six crates, using the exact package/dependency patch flags in `scripts/check-local.sh` with `--allow-dirty` removed. Do not publish unless every package check succeeds.
5. Publish and read back the crates in this order: `eggpack-contract`; `eggpack-core`; `eggpack-bootstrap`; `eggpack-github`; `eggpack-ci`; `eggpack-cli`. For each package, after its dependencies are visible, run `cargo publish -p <crate> --locked --dry-run`; if successful, run exactly one `cargo publish -p <crate> --locked`. Confirm crates.io shows version `0.1.2`, the returned checksum, and `yanked: false` before proceeding.
6. After all six publications are verified, recheck that `v0.1.2` and the corresponding GitHub release remain absent. Create an annotated tag at the exact source SHA and push that tag without force. Read back the remote tag object and peeled commit.
7. Create a public, non-draft, non-prerelease GitHub Release for `v0.1.2`, with the reviewed release notes and no custom assets. Use a verify-existing-tag option so release creation cannot synthesize a different tag.
8. Verify release metadata, source-only asset inventory, tag/commit identity, crate versions/checksums/yanked states, and the registry-only consumer/CLI installation smoke checks. Update `docs/release-process.md`, the roadmap, registry, and M001 closure with observed—not inferred—evidence.

## 7. Package dependency order

The release source must be rechecked before acting. Expected internal graph:

- `eggpack-contract`: no internal dependency.
- `eggpack-core`, `eggpack-bootstrap`: depend on contract and the already-published manifest crate.
- `eggpack-github`: depends on contract, core, bootstrap, manifest, and published `eggfetch-core`.
- `eggpack-ci`: depends on contract/core and has dev-dependencies on manifest/bootstrap/GitHub, so publish after GitHub.
- `eggpack-cli`: depends on the published workspace crates, so publish last.

The version requirements currently written as `0.1.0` admit `0.1.2` under Cargo’s compatible version range; revalidate rather than assuming if any manifest changes. Do not include `eggpack-manifest` in the publication list.

## 8. Failure, restart, and partial-publication semantics

A visible crates.io version cannot be rolled back through this plan. On any publish timeout/error, query the crate’s exact version and checksum before deciding what happened. Never repeat a successful or uncertain publish command blindly. If a crate is visible, mark it complete and continue only after confirming its non-yanked state and checksum. If a package is not visible or a dependent dry-run fails, stop, preserve the partial public state, and diagnose the registry/index response. Do not create the project tag or GitHub Release until every one of the six packages is verified. If the tag or release unexpectedly exists, stop; never retag, overwrite, or reuse an unrelated release.

Credentials stay in the existing Cargo/GitHub auth stores and are never read, printed, written to files, or returned in diagnostics. No publication command runs from a dirty tree or uses `--allow-dirty`.

## 9. Compatibility and migration

No schema or public API changes are part of this release. The six packages are newly published; their versions are the already-reviewed `0.1.2` values. The existing manifest crate is not bumped. Consumers opt into these crates by adding normal crates.io version requirements. Exact consumer pins remain consumer-owned; no Eggup or product repository changes are included.

## 10. Exact verification commands expected

Before publication:

```sh
scripts/check-local.sh
git diff --check
cargo package -p eggpack-contract --locked
cargo package -p eggpack-core --locked --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo package -p eggpack-bootstrap --locked --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo package -p eggpack-github --locked --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"' --config 'patch.crates-io.eggpack-core.path="crates/eggpack-core"' --config 'patch.crates-io.eggpack-bootstrap.path="crates/eggpack-bootstrap"'
cargo package -p eggpack-ci --locked --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"' --config 'patch.crates-io.eggpack-core.path="crates/eggpack-core"' --config 'patch.crates-io.eggpack-bootstrap.path="crates/eggpack-bootstrap"' --config 'patch.crates-io.eggpack-github.path="crates/eggpack-github"'
cargo package -p eggpack-cli --locked --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"' --config 'patch.crates-io.eggpack-core.path="crates/eggpack-core"' --config 'patch.crates-io.eggpack-bootstrap.path="crates/eggpack-bootstrap"' --config 'patch.crates-io.eggpack-github.path="crates/eggpack-github"'
```

These are the exact dependency-patch closures used by `scripts/check-local.sh`, with `--allow-dirty` removed. Do not change the patch set.

Hosted CI must report success for the exact final release SHA on Linux stable, Linux 1.89.0, macOS, and Windows. After publication, create an external Cargo consumer under the ignored workspace `target/` directory with exact `=0.1.2` dependencies on the six crates and no path/Git overrides, then run `cargo check --manifest-path <consumer>/Cargo.toml`. Install `eggpack-cli =0.1.2` into an ignored `target/` root, not the user’s system Cargo bin directory.

Tag/release readback must confirm: annotated tag `v0.1.2` peels to the final release SHA; GitHub Release is public, non-draft, non-prerelease, associated with that tag, and has no uploaded custom assets; all six registry entries are `0.1.2`, non-yanked, and have checksums recorded.

## 11. Documentation and closure evidence

Update `docs/release-process.md` only after publication facts are observed. The closure at `plans/closure/workspace-release/001-status.md` must list each crate’s crates.io URL, exact version, checksum, yanked state, source SHA, publication order, dry-run/publish results, external consumer result, annotated tag object and peel, GitHub Release URL/state, hosted run IDs/lane results, and any remaining limitations. State explicitly that no custom binaries, signatures, or provenance were supplied.

## 12. Acceptance criteria and stop conditions

Accept only when all six crates are independently verified at 0.1.2, the registry-only consumer resolves them without path/Git dependencies, CLI installation succeeds into the workspace target root, and the public GitHub `v0.1.2` release points to the same exact reviewed SHA with source archives only. Stop for a changed version, unclean source, failed exact-SHA lane, missing package dependency, dry-run failure, registry mismatch, existing tag/release, publication uncertainty, or any need to expand the approved scope. Any such failure leaves the already-public state unchanged and is reported before further publication.
