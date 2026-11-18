# Preparing an Eggpack release

This guide covers release preparation for Eggpack itself. A version bump and a
green CI run do **not** create a tag, stage a draft, publish a crate, or publish
a GitHub release.

## Release units and current candidates

Eggpack has two independently versioned release units:

| Unit | Version source | Version in this tree | Publication state |
|---|---|---:|---|
| The six workspace crates inheriting the workspace version, including `eggpack-cli` | `[workspace.package].version` in the root `Cargo.toml` | `0.1.1` (from `0.1.0`) | The six crates remain unpublished to crates.io; a workspace version bump does not publish them or produce a CLI artifact release. |
| `eggpack-manifest` | Its explicit `version` in `crates/eggpack-manifest/Cargo.toml` | `0.1.2` (from `0.1.1`) | Published 2026-11-18; release identity and verification evidence are below. |

Only `eggpack-manifest` has been published from this workspace. It has its own
version because its compatibility and publication history is independent of the
CLI/workspace version. The schema remains v1 unless a separately reviewed
compatibility change says otherwise. `eggup-eggpack` has an exact `=0.1.0`
manifest dependency; a registry release does not update that downstream pin,
which is owned and planned in the Eggup repository.

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

No GitHub draft or GitHub release was created, and no CLI/workspace crate or
CLI artifact was published.

## References

- [Quickstart](quickstart.md) — verified local producer and workflow-generation walkthrough.
- [Release identity policy](release-identity-policy.md) — accepted `vMAJOR.MINOR.PATCH` mapping.
- [Local verification gate](../scripts/check-local.sh) and [hosted CI](../.github/workflows/ci.yml).
- [Draft staging boundary](../architecture/github.md) and [manifest compatibility record](../plans/closure/release-manifest/004-status.md).
