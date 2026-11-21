# Workspace Release M001 Closure — Eggpack 0.1.2

Status: closed

Source plan: `plans/implementation/workspace-release/001-eggpack-0.1.2-crates-and-github-source-release.md`

Roadmap: `plans/subsystems/workspace-release-roadmap.md`

Reviewed plan baseline: `23ebb10941bb2b38b808f61edddff3ed3d1f2961` (merged main after PR #6). The release source is `35b3fc26de63437947aa4f36764c17bd0ebb9f65` (merged main after PR #8), and the annotated release tag peels to that exact commit.

## Executive finding

M001 is closed. All six approved workspace crates were published to crates.io at `0.1.2` in dependency order and read back as non-yanked with the checksums below. Public GitHub release [Eggpack workspace 0.1.2](https://github.com/eggstack/eggpack/releases/tag/v0.1.2) was published on 2026-11-21 for annotated tag `v0.1.2`, which points at the same release source commit. It is not a draft or prerelease and has no uploaded custom assets. The only release downloads are GitHub-generated source archives.

The separately versioned `eggpack-manifest 0.1.2` package and `eggpack-manifest-v0.1.2` tag were not republished or altered. No prebuilt CLI binaries, signatures, attestations, or provenance claims were added. No production code, package metadata, dependency declarations, schema, or workflow change was required for M001.

## Implementation commits and production evidence

There was no M001 production-code implementation commit: the six crate sources and version declarations already existed in the reviewed release source. The release source commit is `35b3fc26de63437947aa4f36764c17bd0ebb9f65` (merge commit for PR #8's package-check correction); it is the immutable source identity used for the registry packages, tag, and GitHub release. The release documentation/closure updates do not change the tagged source.

## Requirement-to-evidence matrix

| Requirement | Evidence and result |
|---|---|
| Publish exactly the six approved crates at `0.1.2`, in dependency order, without republishing the manifest crate | All six crates were published and read back as `yanked: false`. Order: contract → core → bootstrap → github → ci → cli. `eggpack-manifest` was excluded. See registry table below. |
| Verify exact release source before publication | Canonical `scripts/check-local.sh` passed on source SHA `35b3fc26de63437947aa4f36764c17bd0ebb9f65`; hosted run [44946729915](https://github.com/eggstack/eggpack/actions/runs/44946729915) passed all four lanes on that exact SHA. Six clean `cargo package --locked` checks passed with the dependency patch sets from the plan and without `--allow-dirty`. |
| Dry-run, publish once, and verify each registry record before continuing | For each crate, `cargo publish -p <crate> --locked --dry-run` and then one `cargo publish -p <crate> --locked` succeeded. Each version, checksum, and yanked state was read back before the next dependent publication. No uncertain publication was blindly retried. |
| Prove registry consumption and CLI availability | An external consumer check with exact `=0.1.2` registry dependencies and no path/Git overrides passed; its lockfile resolved all six exact versions. Cargo warned that `eggpack-cli` is binary-only and is ignored as a library dependency, so the CLI was separately installed from crates.io into ignored `target/` and `eggpack --help` succeeded. |
| Cut the source tag and public release only after all six registry records were verified | Remote annotated tag object `d6dbe9307f11a1116689c8534fc994461b8adfee` for `v0.1.2` peels to `35b3fc26de63437947aa4f36764c17bd0ebb9f65`. Release metadata is public, stable, non-draft, non-prerelease, and has `assets: []`. |
| Preserve scope and security boundaries | No binaries, custom assets, signatures, attestations, or provenance were supplied. Checksums are integrity facts only. Existing Cargo/GitHub authentication stores were used without exposing credentials. |

## Crates.io publication evidence

Each record is crates.io version `0.1.2`, non-yanked, published from source SHA `35b3fc26de63437947aa4f36764c17bd0ebb9f65`.

| Order | Crate | Registry record | SHA-256 checksum | Yanked |
|---:|---|---|---|---|
| 1 | `eggpack-contract` | [0.1.2](https://crates.io/crates/eggpack-contract/0.1.2) | `7ebc5d638b024a46bbc0bbe8cdae3c1e7d93597b0547880915b62daf09599264` | false |
| 2 | `eggpack-core` | [0.1.2](https://crates.io/crates/eggpack-core/0.1.2) | `cab8120cbf84d0e96e356be38deae0fa61e7c041234d6beae433108dc76edb92` | false |
| 3 | `eggpack-bootstrap` | [0.1.2](https://crates.io/crates/eggpack-bootstrap/0.1.2) | `6951ce753ae7b34ded55df7d82581d81388704e1bd7d08b460d569b4e7d564b9` | false |
| 4 | `eggpack-github` | [0.1.2](https://crates.io/crates/eggpack-github/0.1.2) | `47df8bfc7cc828d09598448e629d781df00461c0f1a941b91bf7f980353a61d4` | false |
| 5 | `eggpack-ci` | [0.1.2](https://crates.io/crates/eggpack-ci/0.1.2) | `3e368a8716fc234313300a06d21bc556769f79809a5f6a8ec96f9e32d03d04a6` | false |
| 6 | `eggpack-cli` | [0.1.2](https://crates.io/crates/eggpack-cli/0.1.2) | `90d9b1e46e26a25edc372de3344cb546d1cf61e89eb57c37a51d5f31202c8e78` | false |

## Verification executed

- `scripts/check-local.sh` — passed on the exact release source SHA.
- Hosted main CI run [44946729915](https://github.com/eggstack/eggpack/actions/runs/44946729915) — success on `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, and `portability (windows-latest)`, on the exact release source SHA.
- All six clean package checks — passed using the exact per-crate dependency patch closures recorded in source plan §10; no `--allow-dirty` was used.
- For each crate in the order above, the planned `cargo publish -p <crate> --locked --dry-run` followed by one `cargo publish -p <crate> --locked` — passed; each registry readback matched version, checksum, and non-yanked state before proceeding.
- Registry-only external consumer — exact `=0.1.2` requirements, no path/Git overrides; check passed and lockfile included all six versions. The binary-only CLI caveat and separate installation smoke check are recorded above.
- Cargo installation smoke check — `cargo install --registry crates-io eggpack-cli --version 0.1.2 --locked --root target/eggpack-cli-install-35b3fc2`, followed by `target/eggpack-cli-install-35b3fc2/bin/eggpack --help` — passed.
- Remote tag readback — annotated object and peeled commit matched the values above.
- GitHub release readback — [v0.1.2](https://github.com/eggstack/eggpack/releases/tag/v0.1.2), published 2026-11-21T20:55:10Z, `isDraft: false`, `isPrerelease: false`, `assets: []`.

## Invariant, failure, and recovery review

- **Release authority and immutability:** publication was explicit and sequential. Every package was read back before the next dependency was attempted. The tag was created only after all six crate records were verified; it was not force-pushed or moved. The GitHub release was created against the verified existing tag and has no custom assets.
- **Determinism and scope:** no source, schema, version, dependency, build, qualification, workflow, or downstream repository changes were made for publication. `eggpack-manifest` and its tag were left unchanged. No invariant or validation rule was weakened.
- **Failure/recovery:** no registry publication had an ambiguous result; all six publishes and readbacks succeeded. No partial public state needed recovery. No blind retry, yanking, retagging, overwrite, or force-push occurred.
- **Security:** Cargo/GitHub credentials remained in their existing authentication stores and were not read or emitted. Registry checksums were recorded as byte-integrity evidence only; no authenticity or provenance property is inferred.
- **Compatibility/migration:** this release introduced no schema or API change. Consumer pins remain consumer-owned; no Eggup or product repository was changed. The external consumer's binary-only warning is not hidden: CLI availability is evidenced separately by the successful registry install and help invocation.

## Documentation and operations

Current-facing release status is recorded in `docs/release-process.md`. The subsystem roadmap, active planning registry, and root `AGENTS.md` handoff are reconciled to this closure. The closure record is the detailed evidence source; earlier release-manifest closure records remain historical and unchanged.

## Unresolved findings and disposition

No unresolved M001 finding remains. The intentional limitations are: no prebuilt CLI binaries or custom assets; no signing, attestations, or provenance; the CLI is a binary-only crate and therefore cannot be exercised as a consumer library dependency. Its registry install and `--help` smoke check passed. Integrity checksums do not establish authenticity.

Roadmap disposition: M001 is closed; the workspace-release roadmap is complete for this milestone. Registry disposition: the M001 plan is closed and this record is the closure authority. `eggpack-manifest 0.1.2` and its separate tag remain untouched.
