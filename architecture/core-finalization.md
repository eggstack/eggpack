# `eggpack-core` — Finalization stage (`src/finalization.rs`) — Deep Dive

Qualification-gated **local** release construction: the last of four producer
stages turns build attempts plus `QualificationEvidence` into a new output root
of contract-named artifacts and a `ReleaseManifest v1` over the FINAL bytes.
File owns exactly one public entry point, `finalize_release`
(`crates/eggpack-core/src/finalization.rs:68`), plus its four public data types;
everything else is private. This is one of four `eggpack-core` stage deep dives
(planning, build, qualification, finalization); it contains no planning, no
compilation, and no execution.

## Inputs and outputs

| Direction | Item | Source |
| --- | --- | --- |
| in | `&DistributionContract` (layout authority) | `finalization.rs:69` |
| in | `&ReleasePlan` (intent: selected targets, policy, `artifact_form`) | `finalization.rs:70` |
| in | `&FinalizationRequest` (identity + per-target evidence) | `finalization.rs:71` |
| in | `&Path` output root, must be absolute and absent | `finalization.rs:72` |
| out | `FinalizedRelease { root: PathBuf, manifest: ReleaseManifest }` | `finalization.rs:53-59` |

`FinalizationRequest` (`:36-50`) carries opaque `product_id` / `release_id` /
`source_revision`, a bounded `Vec<FinalizationTargetInput>` (`:26-33`: contract
triple-or-alias, the producing `BuildAttempt`, the M003 `QualificationEvidence`),
bounded `evidence_references`, and an `Option<ArchiveEncoding>` that must be
present exactly when some selected target expands to an archive (`:154-165`).

The function performs no I/O outside the output root and its parent, opens no
network connection, spawns no process, and mutates no tag or branch.

## The qualification gate

Required targets may not be finalized without passing evidence.

- Per-target status gate (`:196-200`): `QualificationStatus::Passed` is accepted;
  `Deferred` is accepted **only** when `planned.policy.support != SupportTier::Required`
  (`NonGating` / `Experimental`); everything else — including
  `Failed(QualificationFailure)` and `Deferred` on a `Required` target — returns
  `target qualification does not permit finalization`.
- Evidence must already match the build: `QualificationEvidence::validate_for`
  (`:192-195`, implemented in `qualification.rs:319`) rejects mismatched
  `release_id`/`source_revision`/target/classification/support, selector
  mismatch, unsorted evidence, and `Passed` without a smoke selector for
  `Native | DeferredNative | Emulated`.
- The build attempt must be identity-exact and successful (`:183-191`): same
  canonical target, `release_id`, `source_revision`, `strategy`, and
  `CommandOutcome::Success`. Duplicate targets are rejected here.
- Every selected target is mandatory. `expected_targets` from the plan must
  equal `actual_targets` collected from the request (`:167-168`, `:294-296`).
- Deferred non-required targets are not "skipped": they still need complete
  candidate byte evidence (`validated_candidates`, `:316-349`) and a full
  contract-named file set (`:278-287`).

Fail-closed shape: there is no partial-release path. Any rejection unwinds to
`finalize_release`, which removes the root it created and returns `Err` with no
`FinalizedRelease` (`:142-145`). Every `CoreError` payload is a fixed string
(`err(...)`); paths, digests, and file contents never appear in errors
(`lib.rs:72-83`).

## Output-root safety rules

The root is the finalizer's entire write surface, and the rules are enforced
here, in this file:

- Absolute path required (`:82`); no `CurDir` / `ParentDir` components
  (`:88-95`).
- The parent must exist, be a directory, and not be a symlink
  (`symlink_metadata` + `file_type().is_symlink()`, `:114-121`).
- The root's own file name must be present and non-empty (`:122-125`).
- The parent is canonicalized and the root is rebuilt from that canonical parent
  plus the literal file name (`:126-128`). Consequence: `FinalizedRelease.root`
  is the canonical-parent path, which may differ textually from the caller's
  argument when the parent chain contains symlinks.
- The root must be **absent** — `symlink_metadata` on the root must fail
  (`:129-131`). An existing populated root, an existing empty root, and a
  pre-existing symlink at that path are all refused; nothing is clobbered and
  no existing content is read or deleted. Test `finalization.rs:773-774` asserts
  a sentinel file inside an existing root survives the rejection.
- Root creation is a single `create_dir` (`:132`); every artifact and sidecar is
  then created with `create_new(true)` (`:400-404`, `:418-422`, `:459-463`), so
  intra-root name reuse fails instead of overwriting.
- On Unix the root is restricted to mode `0700` (`:134-139`).
- On Windows nothing is set or checked here: the doc comment (`:61-67`) states
  the parent must be caller-secured/private because the ACL is inherited. That
  is an explicit caller obligation, not code enforcement.
- On error only the root created by this invocation is removed (`:143`);
  candidate sources and the parent are untouched (test `:788-789`).

Temporary archive-member staging uses `root/.eggpack-members` (`:170`,
`:237-241`), i.e. inside the owned root, and is removed before returning
(`:306-309`). A staging-cleanup failure is an error, which in turn removes the
whole root. `.eggpack-members` is not a contract release file and the exact
inventory check (`:278-287`) only inspects the `release_files` map, so the
staging directory never enters the manifest.

`release-manifest.json` is **not** written by this file — the manifest is
returned in memory as a value (`:310-313`). Serialization to a file beside the
finalized root happens in `eggpack-cli` (`crates/eggpack-cli/src/main.rs983-988`
and the M003a handoff at `:1674-1686`, which asserts the file is absent inside
the root) and, for staging, in `eggpack-github`
(`crates/eggpack-github/src/lib.rs:31`, `FIXED_MANIFEST_NAME`). The
"beside, never inside" rule is therefore enforced by those crates, not here.

## Archive assembly

- Encoding is `TarGzip` only. `ArchiveEncoding` has exactly one variant and no
  other encodings (`:17-22`), so no other encoding is even expressible; a
  request whose encoding presence does not match the selected layouts, or which
  is present without being `TarGzip`, is rejected (`:159-165`) rather than
  coerced.
- The contract archive filename must end in `.tar.gz` (`:232-236`).
- Members are the contract's `members` in declaration order, zipped with the
  staged candidates in the same order (`:262-268`); the tar entry path is the
  contract `source`, never the staging file name `member-{index}`
  (`:243-261`). The exact member set is additionally enforced by
  `validate_layout_selectors` (`:361-384`) against the layout and by
  `build_manifest` (`lib.rs:232-234`).
- Determinism: `GzBuilder::new().mtime(0)` (`:423-425`),
  `tar::HeaderMode::Deterministic` (`:427`), `Header::new_gnu()` with entry type
  `Regular`, mode `0o755`, `uid 0`, `gid 0`, `mtime 0`, and `set_cksum()`
  (`:432-442`). Member size comes from a fresh digest pass over the staged copy
  (`:429`). Byte-identical archives and equal manifests across two independent
  roots are asserted in `finalization.rs:726-757`.
- The archive is written with `create_new(true)` (`:418-422`).

## Checksum sidecars

`write_checksum` (`:456-473`) re-hashes the just-written final file and writes
one line: `"<sha256_hex>  <asset_file_name>\n"` (two spaces, GNU-style), using
`artifact.file_name()` — the final on-disk name, not a candidate path. The
sidecar filename always comes from the contract (`direct.sidecar_file`,
`entry.sidecar_file`, `archive.sidecar_file`; `:215`, `:225`, `:270`), and the
sidecar is itself registered in `release_files` (`:217`, `:228`, `:272-275`) so
it must appear in `expected_release_files` or the exact-inventory check fails
(`:278-287`). Sidecars are written for direct assets, every bundle entry, and
the archive — never for archive members.

`size` + `SHA-256` are integrity facts about bytes only. Nothing in this file
produces or asserts a signature, a provenance chain, a publisher identity, or
authenticity, and `evidence_references` are opaque bounded strings copied into
the manifest (`:303`) with no trust meaning.

## Manifest aggregation

`build_manifest` (`lib.rs:166-274`) is called with the finalized file map per
target (`:298-305`). The manifest is a value at this point, and the caller
decides whether to serialize it.

- Digests are re-computed at this stage, not copied from candidate or
  qualification claims: `digest_file` over the finalized paths
  (`lib.rs:192-203`, `:243`) after `copy_verified` has already re-verified each
  copy against evidence (`finalization.rs:410-413`). `digest_regular`
  (`:475-505`) additionally re-`symlink_metadata`s the path after reading and
  rejects a non-regular, symlinked, or zero-length file, so a file swapped
  mid-read fails.
- Targets are sorted by canonical triple (`:297`) and the whole document is
  re-validated against schema-v1 bounds before return (`lib.rs:270-272`).
- Bounds come from `eggpack-manifest`: `MAX_TARGETS = 256`, `MAX_RECORDS = 256`,
  `MAX_EVIDENCE_REFERENCES = 64`, `MAX_DOCUMENT_BYTES = 1_048_576`
  (`crates/eggpack-manifest/src/lib.rs:12-16`); `finalization.rs` independently
  caps plan and request target counts at 256 (`:78-81`).
- Collision rules: release-artifact filename uniqueness is **manifest-global**
  (enforced by `ReleaseManifest::validate`; also reflected here by
  `create_new` on the shared root), while `install` name uniqueness is
  **target-local** (`manifest.md`, "Namespace rule"). This file never computes
  install names — they come from contract expansion inside `build_manifest`
  (`lib.rs:211`, `:225`, `:246`).
- `release_files` for a direct/bundle target must be empty of archive members
  (`lib.rs:206-208`, `:215-217`), and an archive target's member map must match
  the contract member count exactly (`lib.rs:232-234`).

## Key types / functions (with `file:line`)

| Item | Location | Note |
| --- | --- | --- |
| `ArchiveEncoding::TarGzip` | `:17-22` | sole variant |
| `FinalizationTargetInput` | `:26-33` | target + attempt + qualification |
| `FinalizationRequest` | `:36-50` | identity, targets, evidence refs, encoding |
| `FinalizedRelease` | `:53-59` | `root` + `eggpack_manifest::ReleaseManifest` |
| `finalize_release` | `:68-146` | public entry point; root safety, cleanup |
| `finalize_into` | `:148-314` | per-target copy/archive/manifest work |
| `validated_candidates` | `:316-349` | re-hash candidates vs qualification evidence |
| `get_candidate` | `:351-359` | exact selector lookup |
| `validate_layout_selectors` | `:361-384` | candidate set == contract layout set |
| `copy_verified` | `:386-415` | `create_new` copy + post-copy digest re-check |
| `create_tar_gzip` | `:417-454` | deterministic tar+gzip assembly |
| `write_checksum` | `:456-473` | `{digest}  {name}` sidecar from final bytes |
| `digest_regular` | `:475-505` | 64 KiB streaming SHA-256 + pre/post stat |
| inline tests | `:507-860` | direct/bundle inventory, archive determinism, negative cases, concurrency |

## Boundaries / non-goals

- No publication, release creation, upload, or tag mutation; no `--clobber`.
- No network client of any kind.
- No installed-state receipts and no update/rollback logic: that is Eggup's
  seam (`architecture/eggup-manifest-consumer-v1.md`).
- No signature, provenance, or authenticity production.
- No archive extraction, no installer rendering, no platform-install logic.
- No CLI surface: the CLI reaches this stage through `eggpack-ci`
  (`crates/eggpack-ci/src/lib.rs2666-2700`, `:2795-2832`).
- Consumer-script validation happens before this stage, not here.

## Dependencies / dependents

- Own deps used by this file (`crates/eggpack-core/Cargo.toml:14-23`):
  `eggpack-contract` (`expected_release_files`, `DistributionContract`,
  `ExpandedAssets`, `:7`), `eggpack-manifest` (returned document type, `:58`),
  `sha2` (`:9`), `flate2` (`:8`), `tar` (`:426`, `:432`), plus `std::fs` /
  `std::io`. `serde` / `serde_json` / `toml` / `command-group` are crate-level and
  unused here.
- Dependents: `eggpack-ci` (`aggregate_finalize`, `aggregate_finalize_with_consumer`),
  `eggpack-cli` (aggregation + sidecar manifest write), `eggpack-github`
  (`prepare_staging_payload`, which re-validates the finalized root against
  contract and manifest before draft staging).

## Sibling deep dives

- [core-planning.md](core-planning.md) — `ReleasePlan`, `PackConfig::resolve`
- [core-build.md](core-build.md) — `BuildAttempt`, `CandidateArtifact`
- [core-qualification.md](core-qualification.md) — `QualificationEvidence` and `validate_for`
- [core.md](core.md) — crate-level overview of all four stages
- [overview.md](overview.md) — workspace and producer pipeline map
- [manifest.md](manifest.md) — the schema-v1 evidence document this stage writes
- [contract.md](contract.md) — the layout authority whose names are copied here
