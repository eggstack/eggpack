# Eggpack Workspace Release Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#5.4-eggpack-cli`
- `plans/000-long-term-specification.md#15-publication-policy`
- `plans/002-long-term-roadmap.md#phase-8--release-staging-and-human-publication-gate`

Related decision:

- `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`

## 1. Purpose and ownership boundary

This subsystem tracks the human-controlled publication of Eggpack’s six workspace crates and the corresponding source-only GitHub release. It is separate from the independently versioned `eggpack-manifest` crate and from Eggup’s consumer-side deployment responsibilities.

Publication remains explicit and manual. This roadmap does not add an automated registry publisher or change the draft-only behavior of Eggpack’s product-release staging adapter.

## 2. Work classification

### Invariants

- Publish only the exact reviewed, committed source revision after clean local checks and exact-SHA hosted CI.
- Keep the six workspace crates and `eggpack-manifest` as distinct release units, even when their version numbers match.
- Never publish a dirty tree, move or overwrite a tag, retry a publication blindly, or claim that checksums prove authenticity.
- Registry publication is a human-authorized operation; credentials are never printed or committed.
- The public GitHub release contains source archives only for this milestone; it does not imply prebuilt CLI binaries, signatures, provenance, or runtime qualification.

### Capabilities

- Consumers can resolve the six workspace crates from crates.io at the approved version.
- Consumers can obtain the exact reviewed source through a stable GitHub tag and release.
- The published `eggpack-cli` package provides the `eggpack` executable through Cargo installation.

### Infrastructure

- Dependency-ordered Cargo packaging and publication.
- A stable project release tag and a public, source-only GitHub Release.
- Registry checksum, yanked-state, tag, and release readback recorded in a closure.

### Polish

- Automated publication is not part of this milestone. Any future automation requires a separate plan and must preserve the explicit human publication gate.

## 3. Non-goals

- Republishing or retagging `eggpack-manifest 0.1.2`.
- Changing any crate version, dependency declaration, schema, runtime code, workflow, or build policy.
- Building or attaching platform-specific CLI binaries.
- Signing, attestations, provenance claims, or a new authenticity trust model.
- Updating downstream repositories or exact dependency pins owned by Eggup or products.
- Automated Cargo or GitHub publication.

## 4. Current state

The six workspace packages—`eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-github`, `eggpack-ci`, and `eggpack-cli`—carry version `0.1.2` in the source tree and were absent from crates.io when this milestone was opened (2026-11-21). Their internal dependency requirements use the compatible `0.1.0` range. `eggpack-manifest 0.1.2` is already published and remains separate.

PR [eggstack/eggpack#6](https://github.com/eggstack/eggpack/pull/6) merged to `main` at `23ebb10941bb2b38b808f61edddff3ed3d1f2961`. Its source tree is identical to implementation commit `edce774bf3597a37789ba59587cc2a5be56e3cee`; exact implementation and merged-main CI are green. No project tag `v0.1.2` or GitHub release exists. The existing `eggpack-manifest-v0.1.2` tag is not the workspace release tag and must not be changed.

The user approved publication of the six crates at their existing `0.1.2` version and a GitHub `v0.1.2` source-only release, and authorized merging PR #6. Prebuilt CLI binaries were explicitly excluded. The active milestone plan is `plans/implementation/workspace-release/001-eggpack-0.1.2-crates-and-github-source-release.md`.

## 5. Release sequence and authority

1. Merge the reviewed release-plan/closure documentation and validate the exact resulting `main` SHA locally and in hosted CI.
2. Confirm the tree is clean, versions remain exactly `0.1.2`, all six crate versions and `v0.1.2` remain absent from the registries, and the manifest release remains unchanged.
3. Run the canonical local gate and clean-tree package checks. Do not use `--allow-dirty` for release checks.
4. Publish manually in dependency order: contract; core and bootstrap; GitHub adapter; CI; CLI. For each crate, run its dry-run after its registry dependencies are visible, then publish exactly once and read back the version, checksum, and yanked state before proceeding.
5. If any package operation has an uncertain result, query crates.io before retrying. Never republish an already-visible version. Stop on partial publication; do not create the project tag or GitHub Release until all six packages are verified.
6. Create annotated tag `v0.1.2` at the exact source commit, refusing any existing tag. Verify the remote tag resolves to that commit.
7. Create one public, non-draft, non-prerelease GitHub Release for the verified tag. Attach no custom assets; GitHub’s source archives are the only release downloads.
8. Verify registry, tag, and release readback and record checksums, URLs, lane evidence, and the publication order in the closure and current release documentation.

## 6. Dependency order

The approved package set and publication sequence are:

1. `eggpack-contract 0.1.2` — no internal workspace dependency.
2. `eggpack-core 0.1.2` and `eggpack-bootstrap 0.1.2` — both require contract and the already-published manifest crate.
3. `eggpack-github 0.1.2` — requires contract, core, bootstrap, and manifest.
4. `eggpack-ci 0.1.2` — requires contract/core and has workspace dev-dependencies on bootstrap/GitHub for package verification.
5. `eggpack-cli 0.1.2` — depends on the workspace crates including CI.

Publication is sequential, not parallel, so each registry dependency can be read back before a dependent package is attempted. The exact package metadata and dependency edges must be rechecked on the release source; any mismatch stops the plan.

## 7. Milestones

### M001 — Publish the six workspace crates and cut GitHub source release v0.1.2

Plan: `plans/implementation/workspace-release/001-eggpack-0.1.2-crates-and-github-source-release.md` (**active**).

Acceptance requires all six exact crate versions to be visible and non-yanked with recorded checksums; an external registry-only Cargo consumer resolves and builds the six exact versions; the annotated `v0.1.2` tag peels to the reviewed source SHA; and the GitHub Release is public, non-draft, non-prerelease, source-only, and names that tag. The `eggpack-manifest-v0.1.2` publication is not repeated or altered.

## 8. Cross-cutting security and recovery requirements

Never read or log Cargo/GitHub tokens. Use the existing authenticated clients without exposing credentials. Publication is irreversible in normal operation: stop on any error or ambiguous result, inspect the remote state, and continue only from the first not-yet-published crate after confirming exact state. Never force-push, retag, overwrite a release, or attach unreviewed assets. Record integrity checksums as integrity data only, not provenance or authenticity evidence.

## 9. Verification strategy

- Canonical local gate: `scripts/check-local.sh` on the exact release source.
- Clean package checks for the six crates using the dependency-patch configuration from `scripts/check-local.sh`, with `--allow-dirty` omitted.
- Per-crate `cargo publish -p <crate> --locked --dry-run` after its dependencies are visible, followed by one explicit publish operation.
- Hosted CI on the exact final release SHA: Linux stable, Linux 1.89.0, macOS, and Windows.
- After publication, a registry-only Cargo consumer with exact `=0.1.2` requirements for the six crates, plus a Cargo install into an ignored workspace `target/` root for `eggpack-cli` (never a system-wide install).
- Read back each crates.io checksum/yanked state, the annotated tag peel, and the GitHub release metadata/assets.

## 10. Risks and decision points

Crates.io publication cannot be rolled back by this plan; a later correction requires a new version. A partial publish leaves earlier crates public, so the plan stops and records the partial state instead of deleting or blindly retrying. Registry index propagation can delay dependent package verification; inspect the remote package state before retrying. A pre-existing version, tag, or release is a hard stop. The six-crate set, version, source-only release contents, and main-branch source policy are approved; any change requires a new decision.

## 11. Completion definition

M001 closes only after all six crates are independently verified, the public tag and GitHub source release point to the same exact reviewed source, the registry-only consumer and CLI installation checks pass, and the final closure records exact identifiers and checksums. No package is considered released solely because its source version is set or its tarball packages locally.
