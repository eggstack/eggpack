# Release Manifest Milestone 003 Closure — `eggpack-manifest 0.1.0` Consumer Compatibility Baseline and Registry Publication

Status: closed

Source plan: `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md`

Roadmap: `plans/subsystems/release-manifest-roadmap.md`

Plan authoring baseline: `ed1bef885eb3e396a945165d680ed3073decf2a5`

Publication source commit: `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`

Consumer-qualified source baseline: `678bbf04f5a02827003a1d9ab83ba4f0e6360e41`

Publication source tag: `eggpack-manifest-v0.1.0` (annotated, `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`)

Hosted CI: [run 37064833069](https://github.com/eggstack/eggpack/actions/runs/37064833069) on `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`; `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, and `portability (windows-latest)` all passed.

No production source change was required or made. This milestone is packaging, qualification, manual publication, registry proof, and planning closure only. Every published byte traces to the already consumer-qualified source.

## Executive finding

`eggpack-manifest 0.1.0` is published to crates.io, is non-yanked, and is registry-resolvable at the exact version Eggup's qualified adapter pins (`eggpack-manifest = "=0.1.0"`).

The publication is byte-faithful to the consumer-qualified source rather than merely version-compatible: `git diff 678bbf04f5a02827003a1d9ab83ba4f0e6360e41..8d661e4eb9da1806e5d7c7606939d24e9aceb2c0 -- crates/eggpack-manifest` is empty, and the `src/lib.rs` extracted from the published crates.io tarball hashes to the same SHA-256 as the file at the consumer-qualified pin, `c9aec6df64ef8a362a777713d3b4aa11ef7f11eac9a28a6af154a7d7a79d8e8d`. The plan's "no-semantic-delta" fast path was therefore valid, and no post-qualification compatibility review was required.

A temporary consumer project created outside this repository, with `eggpack-manifest = "=0.1.0"` and no other Eggpack dependency, resolved from the registry, compiled, and passed a schema-v1 parse/target-resolution/digest/round-trip/determinism smoke plus fail-closed negatives. Its lockfile records `source = "registry+https://github.com/rust-lang/crates.io-index"` with checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`; no Git or path source appears anywhere in the graph.

Publication was bounded to exactly one package. No other Eggpack workspace crate was uploaded, no automatic publication workflow was added, and no Eggup crate, Eggsact source, schema version, digest algorithm, or validation rule changed.

## Immutable published identity

Record these values permanently. `0.1.0` is now immutable on crates.io and MUST NOT be overwritten; any post-publication defect requires normal versioning plus downstream requalification.

| Identity | Value |
|---|---|
| Cargo package | `eggpack-manifest` |
| Published version | `0.1.0` |
| Yanked | `false` |
| crates.io checksum (`.crate` SHA-256) | `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629` |
| Compressed crate size | `11337` bytes |
| Published at | `2026-10-02T21:13:36.087273Z` |
| Publisher / owner | `dbowm91` (crates.io user id `391532`) |
| Audit action | `publish` by `dbowm91` at `2026-10-02T21:13:36.087273Z` |
| Edition | `2021` |
| `rust-version` (MSRV) | `1.89` |
| License | `MIT` |
| Repository | `https://github.com/eggstack/eggpack` |
| Homepage | `https://github.com/eggstack/eggpack` |
| Documentation | `https://docs.rs/eggpack-manifest` |
| Description | `Bounded schema-v1 evidence for finalized Eggpack release bytes` |
| Manifest wire schema | `schema_version = 1` (independent of Cargo version) |
| Registry URL | `https://crates.io/crates/eggpack-manifest` |

Crate-independent identity of the published source files:

| Published file | SHA-256 |
|---|---|
| `src/lib.rs` | `c9aec6df64ef8a362a777713d3b4aa11ef7f11eac9a28a6af154a7d7a79d8e8d` |
| `README.md` | `972156b783970a9970bf35a1662ec8c2e97a58781f3dc3c172718bac8aa91ff3` |

`src/lib.rs` is byte-identical across the consumer-qualified pin `678bbf04`, the publication source commit `8d661e4`, and the published crates.io tarball. `README.md` is byte-identical across `8d661e4` and the published tarball.

## Published package contents

```text
eggpack-manifest-0.1.0/.cargo_vcs_info.json
eggpack-manifest-0.1.0/Cargo.lock
eggpack-manifest-0.1.0/Cargo.toml
eggpack-manifest-0.1.0/Cargo.toml.orig
eggpack-manifest-0.1.0/README.md
eggpack-manifest-0.1.0/src/lib.rs
```

Six files, 59.3 KiB unpacked, 11.1 KiB compressed. No repository, planning, architecture, workflow, script, or credential content is present. The packaged `Cargo.lock` contains 11 `registry+https://github.com/rust-lang/crates.io-index` sources and no `git+` or local `path =` entry.

Published `.cargo_vcs_info.json` embeds the publication source commit:

```json
{ "git": { "sha1": "8d661e4eb9da1806e5d7c7606939d24e9aceb2c0" }, "path_in_vcs": "crates/eggpack-manifest" }
```

## Requirement-to-evidence matrix

| Requirement | Acceptance evidence |
|---|---|
| M003 consumer gate is satisfied | Eggup interoperability M003 closure `eggstack/eggup@538e3e5605cf3c315c10e5be200c8896de7379b1`; qualified consumer `eggstack/eggsact@65c916b`; hosted CI `36902758482` + release-drift `36902758396` |
| Qualified source identity is preserved | `git diff 678bbf04f5a02827003a1d9ab83ba4f0e6360e41..8d661e4eb9da1806e5d7c7606939d24e9aceb2c0 -- crates/eggpack-manifest` produced no output; independently, published `src/lib.rs` SHA-256 equals the pin's SHA-256 (`c9aec6df…d8e8d`) |
| Package identity is exact | crates.io version record: name `eggpack-manifest`, version `0.1.0`, edition `2021`, `rust_version` `1.89`, license `MIT`, repository/homepage `https://github.com/eggstack/eggpack`, documentation `https://docs.rs/eggpack-manifest` |
| No hidden producer dependency | `cargo tree -p eggpack-manifest --locked` resolves only `serde 1.0.229` and `serde_json 1.0.151` plus their registry transitives (`serde_core`, `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `itoa`, `memchr`, `zmij`); the registry-only consumer tree is identical |
| Clean package | `cargo package -p eggpack-manifest --locked` from a clean tree at `8d661e4`: "Packaged 6 files, 59.3KiB (11.1KiB compressed)"; verification compiled the generated package at `target/package/eggpack-manifest-0.1.0`, proving the artifact builds outside workspace paths |
| Clean dry-run | `cargo publish -p eggpack-manifest --locked --dry-run` reached "Uploading eggpack-manifest v0.1.0" then aborted on dry run; no `--allow-dirty` was used anywhere |
| Stable/MSRV behavior | `cargo fmt --all -- --check`, `cargo check/clippy/doc/test -p eggpack-manifest --locked` green; `cargo +1.89.0 check/test -p eggpack-manifest --all-targets --locked` green (26 tests); hosted run `37064833069` green on the exact candidate across stable, 1.89.0, macOS, and Windows |
| First-publication conflict ruled out | Immediately before upload, `GET /api/v1/crates/eggpack-manifest` returned `{"errors":[{"detail":"crate `eggpack-manifest` does not exist"}]}`; a `q=eggpack` registry query returned only `eggup-archive`, so no unrelated party claimed an `eggpack-*` name |
| Exact version published | crates.io `max_version`/`newest_version`/`max_stable_version` = `0.1.0`, `yanked` = `false` |
| Registry-only usability | External temp project resolved `=0.1.0`, `cargo generate-lockfile` + `cargo check` + smoke all green; lockfile checksum equals the crates.io checksum; no `git+` or `path =` entry |
| Publication remains bounded | Only `cargo publish -p eggpack-manifest --locked` was executed; no other package uploaded; no publication workflow added; `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-ci`, `eggpack-github`, and `eggpack-cli` remain unpublished |
| Downstream truth reconciled | Eggpack registry/roadmaps updated; Eggup handoff recorded below; Eggup's own `0.1.2` publication chain remains Eggup-owned and unfinished |
| Closure traceability | This record carries source commit, package checksum, registry evidence, tag, commands, CI run, and unresolved findings |

## Compatibility baseline actually executed

The published bytes are the qualification baseline; the existing strict schema-v1 harness is the compatibility harness. All 26 in-crate tests passed on both stable and `1.89.0`:

```text
tests::direct_round_trip_stable
tests::bundle_pairing_round_trip
tests::archive_relationship_round_trip
tests::canonical_order_is_independent_of_input_order
tests::exact_target_and_sha_helpers_are_non_wire_additions
tests::rejects_unknown_fields_version_and_bad_digest_or_size
tests::rejects_case_collision_and_crossed_install
tests::install_collisions_remain_rejected_within_target
tests::release_artifact_collisions_remain_manifest_global
tests::direct_install_names_are_target_local_like_eggsact_contract
tests::bundle_install_names_are_target_local
tests::archive_install_names_are_target_local_like_egress_contract
tests::eggup_interoperability_manifest_fixtures_are_valid_v1
tests::projection_fixture_consistency::direct_projection_is_exact_pairwise_projection
tests::projection_fixture_consistency::bundle_projection_is_exact_pairwise_projection
tests::projection_fixture_consistency::archive_projection_is_exact_pairwise_projection
tests::projection_fixture_consistency::direct_projection_rejects_wrong_size_or_digest
tests::projection_fixture_consistency::projection_rejects_wrong_selected_target
tests::projection_fixture_consistency::wrong_target_projection_fixture_is_negative_evidence
tests::projection_fixture_consistency::bundle_projection_rejects_crossed_destinations
tests::projection_fixture_consistency::bundle_projection_rejects_dropping_codegg_manifest_entry
tests::projection_fixture_consistency::bundle_projection_rejects_duplicate_unit
tests::projection_fixture_consistency::bundle_projection_rejects_substituting_eggsact_for_helper
tests::projection_fixture_consistency::bundle_projection_rejects_unrelated_fourth_unit
tests::projection_fixture_consistency::archive_projection_rejects_crossed_member_relationship
tests::projection_fixture_consistency::archive_projection_rejects_missing_extraction_required
```

Schema-v1 constant surface re-confirmed unchanged in the published source: `SCHEMA_V1 = 1`, `MAX_DOCUMENT_BYTES = 1_048_576`, `MAX_TARGETS = 256`, `MAX_RECORDS = 256`, `MAX_EVIDENCE_REFERENCES = 64`. Public types remain `ReleaseManifest`, `TargetRecord`, `ArtifactForm` (`Direct`/`Bundle`/`Archive`), `ArtifactRecord`, `BundleRecord`, `ArchiveMemberRecord`, `ByteEvidence`, `ManifestError`, with `target`, `from_json`, `validate`, `to_json`, `sha256_bytes` helpers.

## Verification executed

Executed from a clean tree at `8d661e4eb9da1806e5d7c7606939d24e9aceb2c0`, no `--allow-dirty`:

```text
cargo fmt --all -- --check                                             # clean
cargo check -p eggpack-manifest --all-targets --locked                  # Finished
cargo clippy -p eggpack-manifest --all-targets --all-features --locked -- -D warnings
                                                                      # Finished, no warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked   # 26 passed
cargo doc -p eggpack-manifest --no-deps --locked                       # Finished
cargo tree -p eggpack-manifest --locked                                # registry-only, leaf-sized
cargo +1.89.0 check -p eggpack-manifest --all-targets --locked         # Finished
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked          # 26 passed
cargo package -p eggpack-manifest --locked --list                      # 6 files listed above
cargo package -p eggpack-manifest --locked                             # 6 files, 59.3KiB (11.1KiB compressed)
                                                                      # "Verifying eggpack-manifest v0.1.0 (target/package/...)"
                                                                      # "Compiling eggpack-manifest v0.1.0"
cargo publish -p eggpack-manifest --locked --dry-run                   # reached upload, aborted on dry run
git diff --check                                                       # exit 0
git status --short                                                     # empty
```

Full workspace regression, strongly preferred by the plan because the crate is a workspace member consumed by Eggpack itself:

```text
cargo test --workspace --all-targets --all-features --locked           # 219 passed, 7 ignored, 9 suites
```

No unrelated workspace failure was encountered, so no evidence was narrowed. The 7 ignored tests are the pre-existing platform-gated suites documented in `AGENTS.md` and prior closure records, not new skips.

Publication and post-publication commands:

```text
# first-publication gate, immediately before upload
GET https://crates.io/api/v1/crates/eggpack-manifest                  # crate does not exist
GET https://crates.io/api/v1/crates?q=eggpack&per_page=20              # ["eggup-archive"] only

cargo publish -p eggpack-manifest --locked
# Packaged 6 files, 59.3KiB (11.1KiB compressed)
# Uploaded eggpack-manifest v0.1.0 to registry `crates-io`
# Published eggpack-manifest v0.1.0 at registry `crates-io`

# post-publication registry proof
GET https://crates.io/api/v1/crates/eggpack-manifest                   # 0.1.0, yanked false
GET https://crates.io/api/v1/crates/eggpack-manifest/versions          # checksum 2a08f24b…b629
GET https://crates.io/api/v1/crates/eggpack-manifest/owners            # [("dbowm91","user")]
sha256sum target/package/eggpack-manifest-0.1.0.crate                 # 2a08f24b…b629
GET https://static.crates.io/crates/eggpack-manifest/eggpack-manifest-0.1.0.crate | sha256sum
                                                                      # 2a08f24b…b629 (byte-identical to local)
```

## Registry-only consumer proof

A temporary project was created outside this repository at `/tmp/opencode/m003-registry-smoke` with a single dependency, `eggpack-manifest = "=0.1.0"`, `publish = false`, edition 2021. It is not part of the Eggpack workspace, contains no path or Git dependency on Eggpack, and was deleted after the run.

```text
cargo generate-lockfile    # Locking 12 packages to highest compatible versions
cargo check                # Finished
cargo run                  # all smoke checks passed
```

Lockfile resolution proof:

```text
[[package]]
name = "eggpack-manifest"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629"
```

`grep -n 'git+\|path = ' Cargo.lock` returned nothing: no Git or path source is selected anywhere. The resolved graph is exactly `eggpack-manifest 0.1.0` → `serde 1.0.229` / `serde_json 1.0.151` plus registry transitives, matching the in-workspace tree. `cargo check --offline` also succeeded, proving the consumer resolves from registry cache rather than any workspace path.

The smoke parsed the checked-in cross-repo fixtures from `plans/closure/eggup-interoperability/fixtures/` through the published crate and asserted, per fixture (`direct-manifest.json`, `bundle-manifest.json`, `archive-manifest.json`):

- `schema_version == SCHEMA_V1`;
- exact target count;
- exact-match `target()` resolution succeeds and a non-existent target fails, proving no alias resolution or guessing;
- `sha256_bytes()` decodes to 32 bytes for direct artifacts, every bundle entry, and every archive member, so exact digest evidence is present and usable;
- `to_json` → `from_json` → `to_json` is byte-identical, proving deterministic serialization survives the published artifact.

Fail-closed negatives were then re-confirmed against the published bytes:

- `unknown-schema.json` rejected (unsupported schema version);
- `corrupt-digest.json` rejected (digest/size inconsistency);
- an injected unknown top-level field rejected (`deny_unknown_fields` active in the published build);
- a document exceeding `MAX_DOCUMENT_BYTES` rejected before unbounded growth.

Smoke output:

```text
ok direct-manifest.json product=eggsact release=1.2.6 targets=2
ok bundle-manifest.json product=codegg release=2.4.0 targets=1
ok archive-manifest.json product=egress release=3.1.0 targets=1
ok negatives unknown_schema+corrupt_digest+unknown_field+oversize
ALL REGISTRY-ONLY SMOKE CHECKS PASSED
```

`eggup-eggpack` is not yet published by Eggup; this is expected and outside Eggpack's authority. docs.rs metadata resolves for `0.1.0`, but the first documentation build was still queued at closure time, so no docs.rs build result is claimed. Per the plan, docs.rs is optional follow-up evidence and does not gate the Eggup handoff.

## Invariant, failure, compatibility, and security review

- **Publication authority unchanged.** The crate is still a bounded synchronous JSON parser/serializer. It gained no network, filesystem, release-selection, installation, replacement, authenticity, or credential authority by being packaged. Uploading to a registry distributes the same inert bytes; it does not convert evidence into trust or install authority.
- **Integrity is not authenticity.** The crate and README continue to describe SHA-256 and size as integrity facts. No signature, attestation, or provenance claim was added by this milestone.
- **Validation not weakened.** No source byte changed, so unknown-field rejection, unsupported-version rejection, size/count/name/path bounds, and digest validation are exactly as qualified. The registry-only smoke re-confirmed four of these directly against the published artifact.
- **Schema-v1 wire semantics untouched.** `schema_version = 1` remains; Cargo `0.1.0` and wire schema `1` remain independent identities, as the plan required.
- **Leaf shape preserved.** Runtime dependencies remain `serde` + `serde_json` with registry transitives only. No Git/path runtime dependency was added, and no Eggup or consumer code was vendored.
- **Bounded publication.** Exactly one package was uploaded from one explicit command. No automatic publication was added to CI; publication remains a human action, consistent with ADR-0003.
- **Credential hygiene.** The crates.io token was read from the local `~/.cargo/credentials.toml` only. It was never echoed, never written to the repository, never placed in CI configuration, and never copied into this record. No secret appears in the published package or its diagnostics.
- **Immutability.** `0.1.0` is treated as permanent. No `--clobber` semantics were used, no tag was moved, and the plan forbids planning an overwrite. `eggpack-manifest-v0.1.0` is an annotated tag; the published checksum is now the authority if the two ever disagree.
- **No force-fix-forward.** No stop condition in plan §10 was reached. The manifest subtree was empty-diff, the name was available, no other workspace package became a prerequisite, and package/dry-run/MSRV/hosted qualification all passed without code or API change.
- **Tag honesty.** The repository had no pre-existing tag convention, so the plan's preferred crate-specific default `eggpack-manifest-v0.1.0` was used. No generic workspace `v0.1.0` tag was created, which would have falsely implied the whole workspace was published.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| low | The published `README.md` points readers at `architecture/eggup-manifest-consumer-v1.md`, a repository-only document that is not part of the package, so the reference is dangling on crates.io and docs.rs. | Not a publication blocker and not a compatibility defect. Deliberately not corrected, because editing the README after publication would make repository source diverge from the immutable published bytes, which plan §13 classifies as a compatibility incident. Carry to a future `0.1.1`/`0.2` metadata polish. |
| informational | docs.rs had not finished its first `0.1.0` build at closure time. | Not required to unblock Eggup. Re-check later; if the build fails, that is a docs-infra signal, not an evidence or integrity defect. |
| informational | The crates.io API exposes no non-website endpoint for `GET /v1/me`, so the token's owner could not be read back programmatically before upload. | Resolved by the post-publication evidence instead: the audit action records `publish` by `dbowm91`, matching the sole owner of the existing `eggup-archive` package and the crate's `authors = ["David Bowman"]` metadata. Publication authority is therefore established, not assumed. |

No medium-or-higher finding remains open. Nothing here reopens this milestone.

## Documentation and roadmap disposition

- `plans/closure/release-manifest/003-status.md` (this record) added.
- `plans/subsystems/release-manifest-roadmap.md`: M003 moved to `closed` with its closure record and registry evidence.
- `plans/registry.md`: Release Manifest subsystem and M003 status, immediate execution graph, next handoff, and downstream unblock disposition updated.
- `plans/subsystems/eggup-interoperability-roadmap.md`: the cross-repo dependency line now points at satisfied M003 registry evidence instead of pending publication.
- `plans/implementation/release-manifest/003-eggpack-manifest-0.1.0-publication-and-compatibility-baseline.md`: status advanced to `closed`.
- `crates/eggpack-manifest/README.md`: intentionally unchanged; see the low-severity finding.

### Cross-repository planning handoff

`eggstack/eggup` planning was updated so Eggup's active surfaces no longer record `eggpack-manifest 0.1.0` as absent. Per planning process §9 this is recorded as a reviewed actual commit, not a claim:

| Repository | Commit | Scope |
|---|---|---|
| `eggstack/eggup` | `3f4e99e381b233bfd4be1a676218e9ba2cdce2d4` | `plans/registry.md` and `plans/subsystems/eggpack-manifest-interoperability-roadmap.md` only; docs-only, no crate, package metadata, dependency graph, or published version touched |

Eggup hosted CI on that commit is green: run `37068275917` (`Stable checks`, `MSRV check`, `macOS tests`, `Windows archive, acquisition, and service tests and check` all passed).

The Eggup update moves the M004 status from "blocked, first gate is Eggpack Release Manifest M003" to "blocked on Eggup-owned prerequisites only", records the published identity (date, source commit `8d661e4`, checksum `2a08f24b…b629`, tag, Eggpack hosted run `37064833069`, and this closure record), records a fresh registry re-audit of the M004a-proven set, and states that the M004a compatibility-incident branch is not triggered because the published bytes match the consumer-qualified Git source. It deliberately leaves the `eggup-eggpack` Git pin on `eggpack-manifest` in place, since switching to a registry pin is M004's decision. It does not author or implement M004.

Per planning process §9, that Eggup commit is a planning-truth reconciliation carrying the producer evidence this closure recorded. It is not a claim that Eggup's M004 is implemented, authorized, or complete, and it is not a claim that Eggup was migrated: no Eggup code, package, or published artifact changed.

## Downstream handoff and dependency transitions unlocked

The Eggpack-owned prerequisite that Eggup M004a proved missing is now satisfied: `eggpack-manifest 0.1.0` is registry-resolvable at the exact version Eggup's qualified adapter pins. Eggup can move its `eggpack-manifest` dependency from a Git/path pin to a registry dependency with `=0.1.0` and no semantic requalification is implied by this milestone, because the published bytes are identical to the bytes it already qualified.

Unblocked or advanced by this closure:

- **Eggup interoperability M003 producer side** — the publication half of that seam is now closed on real evidence, completing the Eggpack-owned portion of the Eggsact runtime adoption path.
- **Release Manifest M003** — closed. The manifest subsystem has a real registry consumer evidence chain (`eggpack-manifest 0.1.0` → registry-only smoke consumer), which was the capability the roadmap's completion definition required.

Explicitly **not** unlocked and **not** claimed:

- **Eggup M004.** Eggup retains its proven `eggup-acquisition 0.1.2` → `eggup-eggfetch 0.1.2` → `eggup-eggpack 0.1.2` publication sequence and its separately authorized M004 implementation. As of this closure, `eggup-acquisition` and `eggup-eggfetch` are published at `0.1.1` and `eggup-eggpack` is unpublished, so Eggup's chain is demonstrably unfinished. That is Eggup's work, under its own evidence and authorization rules.
- **Any other Eggpack package publication.** `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-ci`, `eggpack-github`, and `eggpack-cli` remain unpublished. `eggpack-cli` in particular may eventually need a published dependency graph, but this milestone did not evaluate or authorize that, and no plan may assume it.
- **Formal JSON Schema export.** Remains non-blocking polish for the Release Manifest subsystem, exactly as the plan scoped it.
- **Ecosystem M002, Bootstrap M003, CI M003b rerun reuse.** Unchanged by this milestone; their existing conditions still govern.

Eggup's remaining sequence, with only the first link newly satisfied:

```text
eggpack-manifest 0.1.0        [Eggpack; CLOSED by this milestone]
        |
        v
eggup-acquisition 0.1.2       [Eggup; unpublished — currently 0.1.1]
        |
        v
eggup-eggfetch 0.1.2          [Eggup; unpublished — currently 0.1.1]
        |
        v
eggup-eggpack 0.1.2           [Eggup; unpublished — crate absent]
        |
        v
Eggup M004 registry promotion / consumer migration
```

If Eggup finds that the registry bytes or API differ from the consumer-qualified Git source despite this record, that is a compatibility incident: stop Eggup promotion, preserve the published `0.1.0` record, and version/requalify normally rather than attempting to overwrite the published version.

## Post-closure addendum — downstream receipt (added 2026-10-03)

This addendum was appended by `plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md` (closure `plans/closure/release-manifest/003a-status.md`). **Nothing above this line was edited.** The historical body, its commands, its evidence table, its unpublished-at-the-time claims, and its findings all stand exactly as recorded at M003 closure time.

### Chronology distinction

| | Statement at M003 closure (2026-10-02) | Later downstream completion (2026-10-03) |
|---|---|---|
| `eggpack-manifest 0.1.0` | published, non-yanked, registry-resolvable at `=0.1.0` | unchanged and still non-yanked; checksum `2a08f24b…b629` re-verified |
| Eggup `0.1.2` chain | `eggup-acquisition`/`eggup-eggfetch` published at `0.1.1`; `eggup-eggpack` absent — therefore unfinished | all three published at `0.1.2` in dependency order, none yanked |
| Eggup M004 | separately authorized, not implemented, not Eggpack's to close | closed in Eggup at `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915` |

The statements in the body above about the chain being unfinished and about `eggup-eggpack` being absent were **correct at M003 closure time**. They are preserved unchanged. What follows is new evidence; it does not correct them.

### Downstream receipt

Eggup consumed this milestone's handoff and completed the promotion that was waiting on it. Evidence reviewed from `eggstack/eggup` (read-only, per planning process §9 — Eggpack did not perform, authorize, or claim this work):

| Item | Value |
|---|---|
| Eggup M004 closure | `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915` — "plans: close M004 with the 0.1.2 registry publication and promotion evidence" |
| Eggup publication source | `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28` |
| Eggup formal status reconciliation | `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d` — "plans: formally close M004 plan status and reconcile stale blocked-state prose" |
| Eggup hosted CI | run `37090397398` on `02a1d32`; `Stable checks`, `MSRV check`, `macOS tests`, and `Windows archive, acquisition, and service tests and check` all passed |
| Eggup closure record | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/004-status.md` |
| Published downstream set | `eggup-acquisition 0.1.2` (`0b01deb8…f170`), `eggup-eggfetch 0.1.2` (`2e483152…3528`), `eggup-eggpack 0.1.2` (`9dbfdfb7…3fef`); all `yanked = false` |
| Producer edge consumed | `eggup-eggpack 0.1.2` declares `[dependencies.eggpack-manifest] version = "=0.1.0"` with no `git` or `path` source, confirmed against the published `.crate` |

The publication order was a hard mechanical constraint discovered during execution, because `cargo package`/`cargo publish` resolve the *packaged* manifest's dependencies from crates.io: `eggup-eggfetch 0.1.2` could not verify before `eggup-acquisition 0.1.2` was visible, and `eggup-eggpack 0.1.2` could not be packaged at all. This is Eggup's packaging constraint, recorded here only because it confirms the chain completed in dependency order.

### What this receipt does and does not change

Confirms:

- the handoff recorded above was **successfully consumed**; the Eggpack-owned prerequisite was sufficient, and no further Eggpack publication or requalification was needed to unblock it;
- the registry-only seam is now consumable end to end — a consumer can depend on `eggpack-manifest 0.1.0` plus the full Eggup `0.1.2` set with no Eggpack or Eggup Git checkout;
- Eggup's M004a compatibility-incident branch was **not** triggered, consistent with this record's byte-identity finding.

Does **not** change:

- no M003 source, publication, checksum, tag, hosted run, or qualification claim changes;
- no finding in this closure is upgraded, reopened, or closed;
- the low-severity published `README.md` dangling-link finding remains deferred to a future crate version and was not edited;
- `0.1.0` remains immutable and non-yanked;
- the six other Eggpack workspace crates remain unpublished, and no automatic publication mechanism exists;
- the low-severity and informational findings above keep their original dispositions and severities.

### Dependency state after the receipt

The sequence in the body's final diagram is now fully closed downstream. The only remaining item, Eggup M004, is closed in Eggup. Eggpack therefore owes nothing further on the M003 -> Eggup M004 registry seam. The next downstream action is Eggsact's own Git-to-registry migration, which is Eggsact-owned and separately authorized in `eggstack/eggsact`; per the Eggup M004 closure it is explicitly not claimed there and is not an Eggpack dependency-ready item.
