# Preparing an Eggpack release

This guide covers release preparation for Eggpack itself. A version bump and a
green CI run do **not** create a tag, stage a draft, publish a crate, or publish
a GitHub release.

## Release units and current candidates

Eggpack has two independently versioned release units:

| Unit | Version source | Version in this tree | Publication state |
|---|---|---:|---|
| The six workspace crates inheriting the workspace version, including `eggpack-cli` | `[workspace.package].version` in the root `Cargo.toml` | `0.1.2` (from `0.1.1`) | Published to crates.io on 2026-11-21; public source-only GitHub release `v0.1.2` is linked below. The release contains no uploaded binaries or custom assets. |
| `eggpack-manifest` | Its explicit `version` in `crates/eggpack-manifest/Cargo.toml` | `0.1.2` (from `0.1.1`) | Published 2026-11-18; release identity and verification evidence are below. |

Both release units have now been published at `0.1.2`; their matching version
number does not merge their independent release identities or publication
histories. `eggpack-manifest` retains its own compatibility and publication
history. The schema remains v1 unless a separately reviewed compatibility
change says otherwise. `eggup-eggpack` has an exact `=0.1.0` manifest dependency;
a registry release does not update that downstream pin, which is owned and
planned in the Eggup repository.

## Verification before release

1. Review the exact source diff and release scope. Confirm that the workspace and
   manifest versions are intended, that dependent version requirements still
   admit those package versions, and that public manifest behavior has been
   reviewed for compatibility.
2. Update `Cargo.lock` and current-facing version documentation. Historical
   closure records are evidence of past releases and must not be rewritten.
3. Run the canonical local gate from the intended source tree:

   ```sh
   scripts/check-local.sh
   ```

   It runs formatting, locked checks and tests, Clippy, docs, dependency trees,
   package checks, and Rust 1.89 checks/tests. Its `cargo package --allow-dirty`
   invocations are local verification only; they are not release or publication
   commands.
4. Require hosted CI on the exact candidate commit. The current workflow checks
   Linux stable, Linux 1.89, macOS stable, and Windows stable; the Windows lane
   also checks its MSVC environment and focused portability/orchestration tests.
   A prior green run on another SHA is not evidence for the candidate commit.
5. Release only from the reviewed, committed source. Resolve release identity
   against the committed checkout and exact tag; do not treat uncommitted tree
   state as part of the release.

## Release and publication boundaries

The repository currently has only `.github/workflows/ci.yml`, which grants
`contents: read`; it does not define a self-release workflow that builds and
publishes Eggpack CLI assets. The reusable/generated workflows described by the
quickstart are for product repositories, not a configured Eggpack self-release.

When configured for a product, Eggpack's optional staging path requires an
already-existing exact tag and can create or reconcile a **draft** only. It does
not create or move tags, publish a release, or overwrite immutable assets. A
human must separately review and publish any draft.

A crates.io publication of `eggpack-manifest` is a separate, explicit
human-controlled action with package-source, compatibility, and
post-publication evidence requirements. Use the completed `eggpack-manifest`
0.1.1 M004 plan and closure as the precedent, and never use `--allow-dirty`
for publication.

The `eggpack-manifest 0.1.2` publication completed on 2026-11-18 from source
commit `d924a44bb6cd640998f197be7c2eb723cc891cbd`. Hosted CI run
[44322496383](https://github.com/eggstack/eggpack/actions/runs/44322496383)
passed all four lanes on that commit. `cargo package -p eggpack-manifest
--locked` and `cargo publish -p eggpack-manifest --locked --dry-run` passed from
the clean tree. Crates.io reports `0.1.2` as non-yanked with checksum
`4bbbe8d12026adf7c8e8ddaecbed0c6ac403a5621d4dd1ac9949f8daf8149a09`; the
annotated tag `eggpack-manifest-v0.1.2` points at the same source commit. An
external consumer pinned to `=0.1.2` resolved the registry checksum and passed
a schema-v1 JSON parse/round-trip smoke test. Relative to `0.1.1`, the crate
change is version/package metadata and a test comment only; the runtime
implementation and public API are unchanged.

The `eggpack-manifest 0.1.2` publication did not create a GitHub release. The
separate workspace-crate release completed later and is recorded below.

## Workspace crates 0.1.2 and source release

The six workspace crates were published from source commit
`35b3fc26de63437947aa4f36764c17bd0ebb9f65`, after the canonical local gate,
clean package checks, and hosted CI passed on that exact source. Hosted run
[44946729915](https://github.com/eggstack/eggpack/actions/runs/44946729915) passed
`linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, and
`portability (windows-latest)`. All six clean `cargo package --locked` checks
passed without `--allow-dirty`. Each crate's `cargo publish --locked --dry-run`
passed before its single publish operation; publication and registry readback
were sequential in the dependency order below.

| Order | Crate | crates.io version | SHA-256 checksum | Yanked |
|---:|---|---|---|---|
| 1 | [`eggpack-contract`](https://crates.io/crates/eggpack-contract/0.1.2) | `0.1.2` | `7ebc5d638b024a46bbc0bbe8cdae3c1e7d93597b0547880915b62daf09599264` | no |
| 2 | [`eggpack-core`](https://crates.io/crates/eggpack-core/0.1.2) | `0.1.2` | `cab8120cbf84d0e96e356be38deae0fa61e7c041234d6beae433108dc76edb92` | no |
| 3 | [`eggpack-bootstrap`](https://crates.io/crates/eggpack-bootstrap/0.1.2) | `0.1.2` | `6951ce753ae7b34ded55df7d82581d81388704e1bd7d08b460d569b4e7d564b9` | no |
| 4 | [`eggpack-github`](https://crates.io/crates/eggpack-github/0.1.2) | `0.1.2` | `47df8bfc7cc828d09598448e629d781df00461c0f1a941b91bf7f980353a61d4` | no |
| 5 | [`eggpack-ci`](https://crates.io/crates/eggpack-ci/0.1.2) | `0.1.2` | `3e368a8716fc234313300a06d21bc556769f79809a5f6a8ec96f9e32d03d04a6` | no |
| 6 | [`eggpack-cli`](https://crates.io/crates/eggpack-cli/0.1.2) | `0.1.2` | `90d9b1e46e26a25edc372de3344cb546d1cf61e89eb57c37a51d5f31202c8e78` | no |

The external registry-only consumer check passed with exact `=0.1.2`
dependencies and no path/Git overrides. Cargo warned that `eggpack-cli` is
binary-only and ignored as a library dependency; its version nevertheless
resolved in the consumer lockfile. Separately, `cargo install --registry
crates-io eggpack-cli --version 0.1.2 --locked` into an ignored workspace
`target/` root succeeded, and `eggpack --help` ran successfully.

Annotated tag `v0.1.2` has tag object
`d6dbe9307f11a1116689c8534fc994461b8adfee` and peels to the release source
commit above. The [public GitHub release](https://github.com/eggstack/eggpack/releases/tag/v0.1.2),
published 2026-11-21, is non-draft and non-prerelease with no uploaded assets;
GitHub-generated source archives are the only release downloads. The separate
`eggpack-manifest-v0.1.2` tag and manifest package were not changed.

No prebuilt CLI binaries, custom assets, signatures, attestations, or provenance
were supplied. SHA-256 values above are integrity checks only and do not establish
authenticity or provenance. Detailed milestone evidence is in
[`plans/closure/workspace-release/001-status.md`](../plans/closure/workspace-release/001-status.md).

## References

- [Quickstart](quickstart.md) — verified local producer and workflow-generation walkthrough.
- [Release identity policy](release-identity-policy.md) — accepted `vMAJOR.MINOR.PATCH` mapping.
- [Local verification gate](../scripts/check-local.sh) and [hosted CI](../.github/workflows/ci.yml).
- [Draft staging boundary](../architecture/github.md) and [manifest compatibility record](../plans/closure/release-manifest/004-status.md).
