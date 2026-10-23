# `eggpack-core` — Finalization stage (`src/finalization.rs`) — Deep Dive

Qualification-gated **local** release construction: the last of four producer
stages turns build attempts plus `QualificationEvidence` into a new output root
of contract-named artifacts and a `ReleaseManifest v1` over the FINAL bytes.
File owns exactly one public entry point, `finalize_release`
(`crates/eggpack-core/src/finalization.rs:67`), plus its four public data types;
everything else is private. This is one of four `eggpack-core` stage deep dives
(planning, build, qualification, finalization); it contains no planning, no
compilation, and no execution.

## Inputs and outputs

| Direction | Item | Source |
| --- | --- | --- |
| in | `&DistributionContract` (layout authority) | `finalization.rs:68` |
| in | `&ReleasePlan` (intent: selected targets, policy, `artifact_form`) | `finalization.rs:69` |
| in | `&FinalizationRequest` (identity + per-target evidence) | `finalization.rs:70` |
| in | `&Path` output root, must be absolute and absent | `finalization.rs:71` |
| out | `FinalizedRelease { root: PathBuf, manifest: ReleaseManifest }` | `finalization.rs:52-58` |

`FinalizationRequest` (`:35-49`) carries opaque `product_id` / `release_id` /
`source_revision`, a bounded `Vec<FinalizationTargetInput>` (`:25-32`: contract
triple-or-alias, the producing `BuildAttempt`, the M003 `QualificationEvidence`),
bounded `evidence_references`, and an `Option<ArchiveEncoding>` that must be
present exactly when some selected target expands to an archive (`:153-164`).

The function performs no I/O outside the output root and its parent, opens no
network connection, spawns no process, and mutates no tag or branch.

## The qualification gate

Required targets may not be finalized without passing evidence.

- Per-target status gate (`:195-199`): `QualificationStatus::Passed` is accepted;
  `Deferred` is accepted **only** when `planned.policy.support != SupportTier::Required`
  (`NonGating` / `Experimental`); everything else — including
  `Failed(QualificationFailure)` and `Deferred` on a `Required` target — returns
  `target qualification does not permit finalization`.
- Evidence must already match the build: `QualificationEvidence::validate_for`
  (`:191-194`, implemented in `qualification.rs:331`) rejects mismatched
  `release_id`/`source_revision`/target/classification/support, selector
  mismatch, unsorted evidence, and `Passed` without a smoke selector for
  `Native | DeferredNative | Emulated`.
- The build attempt must be identity-exact and successful (`:182-190`): same
  canonical target, `release_id`, `source_revision`, `strategy`, and
  `CommandOutcome::Success`. Duplicate targets are rejected here.
- Every selected target is mandatory. `expected_targets` from the plan must
  equal `actual_targets` collected from the request (`:166-167`, `:293-295`).
- Deferred non-required targets are not "skipped": they still need complete
  candidate byte evidence (`validated_candidates`, `:325-357`) and a full
  contract-named file set (`:277-286`).

Fail-closed shape: there is no partial-release path. Any rejection unwinds to
`finalize_release`, which removes the root it created and returns `Err` with no
`FinalizedRelease` (`:141-144`). Every `CoreError` payload is a fixed string
(`err(...)`); paths, digests, and file contents never appear in errors
(`lib.rs:72-80`).

## Output-root safety rules

The root is the finalizer's entire write surface, and the rules are enforced
here, in this file:

- Absolute path required (`:81`); no `CurDir` / `ParentDir` components
  (`:87-94`).
- The parent must exist, be a directory, and not be a symlink
  (`symlink_metadata` + `file_type().is_symlink()`, `:113-120`).
- The root's own file name must be present and non-empty (`:121-124`).
- The parent is canonicalized and the root is rebuilt from that canonical parent
  plus the literal file name (`:125-127`). Consequence: `FinalizedRelease.root`
  is the canonical-parent path, which may differ textually from the caller's
  argument when the parent chain contains symlinks.
- The root must be **absent** — `symlink_metadata` on the root must fail
  (`:128-130`). An existing populated root, an existing empty root, and a
  pre-existing symlink at that path are all refused; nothing is clobbered and
  no existing content is read or deleted. Test `finalization.rs:840-841` asserts
  a sentinel file inside an existing root survives the rejection.
- Root creation is a single `create_dir` (`:131`); every artifact and sidecar is
  then created with `create_new(true)` (`:409-413`, `:427-431`, `:475-479`), so
  intra-root name reuse fails instead of overwriting.
- On Unix the root is restricted to mode `0700` (`:133-138`).
- On Windows nothing is set or checked here: the doc comment (`:60-66`) states
  the parent must be caller-secured/private because the ACL is inherited. That
  is an explicit caller obligation, not code enforcement.
- On error only the root created by this invocation is removed (`:142`);
  candidate sources and the parent are untouched (test `:856-857`).

Temporary archive-member staging uses `root/.eggpack-members` (`:169`,
`:236-240`), i.e. inside the owned root, and is removed before returning
(`:305-308`). A staging-cleanup failure is an error, which in turn removes the
whole root. `.eggpack-members` is not a contract release file and the exact
inventory check (`:277-286`) only inspects the `release_files` map, so the
staging directory never enters the manifest.

`release-manifest.json` is **not** written by this file — the manifest is
returned in memory as a value (`:309-312`). Serialization to a file beside the
finalized root happens in `eggpack-cli` (`crates/eggpack-cli/src/main.rs:1450-1462`
and the M003a handoff at `crates/eggpack-cli/src/main.rs:2141-2155`, which asserts the file is absent inside
the root) and, for staging, in `eggpack-github`
(`crates/eggpack-github/src/lib.rs:32`, `FIXED_MANIFEST_NAME`). The
"beside, never inside" rule is therefore enforced by those crates, not here.

## Archive assembly

- Encoding is `TarGzip` only. `ArchiveEncoding` has exactly one variant and no
  other encodings (`:16-21`), so no other encoding is even expressible; a
  request whose encoding presence does not match the selected layouts, or which
  is present without being `TarGzip`, is rejected (`:158-164`) rather than
  coerced.
- The contract archive filename must end in `.tar.gz` (`:231-235`).
- Members are the contract's `members` in declaration order, zipped with the
  staged candidates in the same order (`:261-267`); the tar entry path is the
  contract `source`, never the staging file name `member-{index}`
  (`:242-260`). The exact member set is additionally enforced by
  `validate_layout_selectors` (`:370-393`) against the layout and by
  `build_manifest` (`lib.rs:341-343`).
- Determinism: `GzBuilder::new().mtime(0)` (`:432-434`),
  `tar::HeaderMode::Deterministic` (`:436`), `Header::new_gnu()` with entry type
  `Regular`, mode `0o755`, `uid 0`, `gid 0`, `mtime 0`, and `set_cksum()`
  (`:441-458`). Member size comes from a fresh digest pass over the staged copy
  (`:438`). Byte-identical archives and equal manifests across two independent
  roots are asserted in `finalization.rs:710-751`.
- The archive is written with `create_new(true)` (`:427-431`).

## File-digest authority

`eggpack-core` has exactly **one** file-digest implementation: `file_digest`
(`crates/eggpack-core/src/digest.rs:102-114`), with a typed refusal reason for
every way a digest can be refused (`DigestError`, `digest.rs:31-46`). Manifest
aggregation (`lib.rs:372`, `lib.rs:419`), finalization
(`:316-323`), and candidate qualification (`qualification.rs:1218-1221`) all
route through it, and `eggpack-ci` / `eggpack-github` consume the same public
helper rather than re-implementing one. This section states the rule once; the
other sections of this document only name the helper.

The rule: a recorded digest MUST describe the bytes of the exact regular file
that was inspected, and the recorded size and digest MUST be mutually
consistent. Concretely, the path is inspected with `symlink_metadata` before
the open and refused unless it is a regular non-symlink file; an empty file is
refused **before** it is read, so a zero-length input can never produce a
digest record; the bytes are streamed through a fixed 64 KiB buffer
(`digest.rs:117-136`) and the byte count is accumulated with overflow checking
(`digest.rs:139-141`); and the path is inspected again after the read
(`digest.rs:144-150`), refusing a path that became a symlink, stopped being a
regular file, or changed length while it was read.

That last step is the time-of-check/time-of-use guard. The symlink rejection is
a property of the path policy, so it stays a path observation; the read then
happens through the handle that was opened, and the evidence recorded is of
those bytes. Without the post-read re-check, a path replaced between the check
and the open could make the digest describe some other file than the one that
was inspected, and a file rewritten mid-read could produce a size and a digest
that disagree with each other.

The helper changes no hash semantics: the hash function, the encoding, and the
lowercase hex representation are exactly what the producer has always recorded,
so identical bytes still produce an identical digest and size. A refused digest
is a hard failure of the enclosing operation — never a warning, never a retry
against the same path, never an empty or placeholder digest.

## Checksum sidecars

`write_checksum` (`:472-489`) re-hashes the just-written final file through the
file-digest authority above and writes one line:
`"<sha256_hex>  <asset_file_name>\n"` (two spaces, GNU-style), using
`artifact.file_name()` — the final on-disk name, not a candidate path. The
sidecar filename always comes from the contract (`direct.sidecar_file`,
`entry.sidecar_file`, `archive.sidecar_file`; `:214`, `:224`, `:269`), and the
sidecar is itself registered in `release_files` (`:216`, `:226-227`, `:271-274`)
so it must appear in `expected_release_files` or the exact-inventory check fails
(`:277-286`). Sidecars are written for direct assets, every bundle entry, and
the archive — never for archive members.

`size` + `SHA-256` are integrity facts about bytes only. Nothing in this file
produces or asserts a signature, a provenance chain, a publisher identity, or
authenticity, and `evidence_references` are opaque bounded strings copied into
the manifest (`:302`) with no trust meaning.

## Manifest aggregation

`build_manifest` (`lib.rs:341-451`) is called with the finalized file map per
target (`:297-304`). The manifest is a value at this point, and the caller
decides whether to serialize it.

- Digests are re-computed at this stage, not copied from candidate or
  qualification claims: the file-digest authority over the finalized paths
  (`lib.rs:372`, `lib.rs:419`) after `copy_verified` has already
  re-verified each copy against evidence (`:419-422`). The same authority
  re-inspects each path after the read, so a file swapped mid-read fails
  instead of being recorded.
- Targets are sorted by canonical triple (`:296`) and the whole document is
  re-validated against schema-v1 bounds before return (`lib.rs:447-449`).
- Bounds come from `eggpack-manifest`: `MAX_TARGETS = 256`, `MAX_RECORDS = 256`,
  `MAX_EVIDENCE_REFERENCES = 64`, `MAX_DOCUMENT_BYTES = 1_048_576`
  (`crates/eggpack-manifest/src/lib.rs:12-18`); `finalization.rs` independently
  caps plan and request target counts at 256 (`:77-80`).
- Collision rules: release-artifact filename uniqueness is **manifest-global**
  (enforced by `ReleaseManifest::validate`; also reflected here by
  `create_new` on the shared root), while `install` name uniqueness is
  **target-local** (`manifest.md`, "Namespace rule"). This file never computes
  install names — they come from contract expansion inside `build_manifest`
  (`lib.rs:387`, `lib.rs:401`, `lib.rs:423`).
- `release_files` for a direct/bundle target must be empty of archive members
  (`lib.rs:382-383`, `lib.rs:391-392`), and an archive target's member map must
  match the contract member count exactly (`lib.rs:408-410`).

## Key types / functions (with `file:line`)

| Item | Location | Note |
| --- | --- | --- |
| `ArchiveEncoding::TarGzip` | `:16-21` | sole variant |
| `FinalizationTargetInput` | `:25-32` | target + attempt + qualification |
| `FinalizationRequest` | `:35-49` | identity, targets, evidence refs, encoding |
| `FinalizedRelease` | `:52-58` | `root` + `eggpack_manifest::ReleaseManifest` |
| `finalize_release` | `:67-145` | public entry point; root safety, cleanup |
| `finalize_into` | `:147-313` | per-target copy/archive/manifest work |
| `digest_candidate_bytes` / `digest_finalized_bytes` | `:316-323` | diagnostic subjects over the shared authority |
| `validated_candidates` | `:325-357` | re-hash candidates vs qualification evidence |
| `get_candidate` | `:360-368` | exact selector lookup |
| `validate_layout_selectors` | `:370-393` | candidate set == contract layout set |
| `copy_verified` | `:395-423` | `create_new` copy + post-copy digest re-check |
| `create_tar_gzip` | `:426-470` | deterministic tar+gzip assembly |
| `write_checksum` | `:472-489` | `{digest}  {name}` sidecar from final bytes |
| `file_digest` | `digest.rs:102-114` | the single producer file-digest authority |
| inline tests | `:492-928` | direct/bundle inventory, archive determinism, negative cases, concurrency |

## Boundaries / non-goals

- No publication, release creation, upload, or tag mutation; no `--clobber`.
- No network client of any kind.
- No installed-state receipts and no update/rollback logic: that is Eggup's
  seam (`architecture/eggup-manifest-consumer-v1.md`).
- No signature, provenance, or authenticity production.
- No archive extraction, no installer rendering, no platform-install logic.
- No CLI surface: the CLI reaches this stage through `eggpack-ci`
  (`crates/eggpack-ci/src/release_plan.rs:621`, `crates/eggpack-ci/src/release_plan.rs:484`).
- Consumer-script validation happens before this stage, not here.

## Dependencies / dependents

- Own deps used by this file (`crates/eggpack-core/Cargo.toml:14-23`):
  `eggpack-contract` (`expected_release_files`, `DistributionContract`,
  `ExpandedAssets`, `:7`), `eggpack-manifest` (returned document type, `:57`),
  `flate2` (`:8`), `tar` (`:436`, `:442`), plus `std::fs` / `std::io`. The
  SHA-256 digest authority is `sha2`, but it is used by `digest.rs`, not here:
  this file only calls the shared helper. `serde` / `serde_json` / `toml` /
  `command-group` are crate-level and unused here.
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
