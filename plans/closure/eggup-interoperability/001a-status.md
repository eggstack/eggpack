# Eggup Interoperability Milestone 001a Closure — Projection Fixture Consistency Corrective

Status: closed

Source plan: `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md`

Roadmap: `plans/subsystems/eggup-interoperability-roadmap.md`

Historical closure: `plans/closure/eggup-interoperability/001-status.md`

Reviewed Eggpack baseline: `7b6b834c1e85a4c43f4eab6b9f3f9f7c9f9f7c9f` (Eggup interop M001a corrective registered; the closure lands on the post-registration commit produced by this work).

External Eggup API baseline: `eggstack/eggup@2cab1f97ef30fa347c2030da321462459672c521`. No Eggup repository change is part of this corrective.

## Executive finding

The CodeGG bundle projection fixture is now an exact pairwise projection of `bundle-manifest.json` for the canonical `x86_64-unknown-linux-gnu` target, carrying all three entries (`codegg-2.4.0-x86_64-unknown-linux-gnu`, `codegg-helper-2.4.0-x86_64-unknown-linux-gnu`, `codegg-manifest-2.4.0.json`) with their exact size, exact SHA-256, and exact install identity propagated as both `member_id` and `relative_destination`.

A new in-crate test harness parses the three projection fixtures into strict typed test structs (`#[serde(deny_unknown_fields)]`) and proves each projection is a relationship-aware projection of its paired ReleaseManifest fixture. The harness also runs the full detection-gap negative matrix that would have caught the original M001 defect, so any future drift of the same shape will fail CI rather than the workflow. Manifest v1 wire shape, helper API surface, and dependency tree are unchanged. No Eggup dependency is introduced.

## Corrected fixture identities

SHA-256 of the checked-in fixture blobs at the corrective commit:

| Fixture | SHA-256 |
|---|---|
| `archive-manifest.json` | `723f1ed30b6bc7dae802ff54ac57afd6c8c8ea6be0e5bcc05c30ef56d4f647d1` |
| `bundle-manifest.json` | `440e871d6cd481bbf1a19ae0711ad9f438b3144315e939a91e66ee3b63b828cc` |
| `corrupt-digest.json` | `05561d7c0fbedb47195f10c2d666cfadc8587fc4d056c7adc4d0f77b35af2db2` |
| `corrupt-size.json` | `abdd9143db59016cc5b349413befd7d268529fbd09a5297f5772698878083a28` |
| `direct-manifest.json` | `04b1993f8ed1de1a4f19819f643831d10ecf87e32c783ab83259fd6cd55947f2` |
| `projection-archive.json` | `c8a2a6e33bc4336cd65cd9b0c4d94c581ba574967f111663b2e70498a7cc8e46` |
| `projection-bundle.json` | `f0b227442e2a2a28dc4e2100301233f2698703d15251b6f2459e6387df6c53ce` (corrected) |
| `projection-direct.json` | `63cb46479a78479929c9bcd2040268435095ee2cc49506bdb1f2c93a04be3760` |
| `unknown-schema.json` | `3527fa83397409981974e498087f2e8e932215ed0115c22793b891d5d917160e` |
| `wrong-target.json` | `cd5dd514216a45aea4a2a83b98208bb15c159b8b05bc61441647eae4e39f18f2` |

The original M001 bundle projection identity `a4494c1dd9a52d0c77ab606106ccf490eafa0b13642796000a6ec1466e446439` is preserved in the M001 closure record and Git history; the original blob is not rewritten to imply it never existed.

## Corrected pairwise projection matrix

For target `x86_64-unknown-linux-gnu` and the canonical CodeGG `bundle-manifest.json`:

| Manifest bundle entry | Manifest install | Projection unit | Projection member_id | Projection relative_destination | exact_size | sha256 |
|---|---|---|---|---|---|---|
| `codegg-2.4.0-x86_64-unknown-linux-gnu` | `codegg` | `codegg-2.4.0-x86_64-unknown-linux-gnu` | `codegg` | `codegg` | `3` | `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` |
| `codegg-helper-2.4.0-x86_64-unknown-linux-gnu` | `codegg-helper` | `codegg-helper-2.4.0-x86_64-unknown-linux-gnu` | `codegg-helper` | `codegg-helper` | `4` | `cb8379ac2098aa165029e3938a51da0bcecfc008fd6795f401178647f96c5b34` |
| `codegg-manifest-2.4.0.json` | `codegg-manifest.json` | `codegg-manifest-2.4.0.json` | `codegg-manifest.json` | `codegg-manifest.json` | `5` | `2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824` |

For target `x86_64-unknown-linux-gnu` and the canonical Eggsact `direct-manifest.json`:

| Manifest direct artifact | Manifest install | Projection unit | Projection member_id | Projection relative_destination | exact_size | sha256 |
|---|---|---|---|---|---|---|
| `eggsact-1.2.6-x86_64-unknown-linux-gnu` | `eggsact` | `eggsact-1.2.6-x86_64-unknown-linux-gnu` | `eggsact` | `eggsact` | `3` | `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` |

For target `x86_64-unknown-linux-gnu` and the canonical Egress `archive-manifest.json`:

| Manifest archive artifact | Manifest member source | Manifest member install | Projection unit | Projection member source | Projection member install | sha256 | size | extraction_required |
|---|---|---|---|---|---|---|---|---|
| `egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz` | `egress` | `egress` | `egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz` | `egress` | `egress` | `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` | `3` | `true` |
| `egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz` | `bin/egress-helper` | `egress-helper` | `egress-3.1.0-x86_64-unknown-linux-gnu.tar.gz` | `bin/egress-helper` | `egress-helper` | `cb8379ac2098aa165029e3938a51da0bcecfc008fd6795f401178647f96c5b34` | `4` | `true` |

`release_identity.product_id`/`release_id` for direct and bundle projections equal the manifest values; the archive projection retains the manifest archive artifact identity directly via the single acquisition entry. The transaction group for direct and bundle projections is `one-artifact-set`. The archive projection uses `one-artifact-set-after-consumer-owned-extraction` to mark the seam required before any Eggup `ArtifactSet` construction.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| `projection-bundle.json` exactly projects all three CodeGG manifest entries | `bundle_projection_is_exact_pairwise_projection` parses the checked-in fixture into strict typed structs and asserts relationship-aware pairing against `bundle-manifest.json`; manifest entries `codegg-2.4.0-x86_64-unknown-linux-gnu`, `codegg-helper-2.4.0-x86_64-unknown-linux-gnu`, and `codegg-manifest-2.4.0.json` all match; no extra unit exists; `transaction_group` is `one-artifact-set`; `release_identity.product_id`/`release_id` equal the manifest. |
| Direct projection fixture is an exact pairwise projection | `direct_projection_is_exact_pairwise_projection` parses `projection-direct.json` against `direct-manifest.json`; name, exact_size, sha256, member_id, and relative_destination all match the single direct artifact and install identity; target is exact and canonical. |
| Archive projection fixture is an exact pairwise projection | `archive_projection_is_exact_pairwise_projection` parses `projection-archive.json` against `archive-manifest.json`; the single acquisition unit matches the manifest archive name/size/sha256; the projected `members` array exactly equals the manifest archive member source/install/size/sha256; `extraction_required` is `true`; the `transaction_group` string retains the `consumer-owned` extraction seam marker. |
| Detection-gap regressions fail CI on the original defect | `bundle_projection_rejects_substituting_eggsact_for_helper` mutates the corrected bundle projection to swap `codegg-helper` for an `eggsact` entry (the exact shape of the original M001 bug) and asserts the harness rejects it with `acquisition unit count 2 != manifest bundle entry count 3`. The other eight detection-gap regressions cover the dropped manifest entry, unrelated fourth unit, crossed destinations, duplicate unit, wrong selected target, crossed archive member relationship, missing `extraction_required`, and mutated direct exact_size/sha256. |
| Relationship-aware (not set-equality) comparison | Bundle comparison pairs each manifest entry by `(artifact.name, install)` against each projection unit's `(name, member_id)` and matches size/sha256/install propagation; archive comparison pairs each manifest member by `(source, install)` against each projected member's `(source, install)` and matches size/sha256. Missing/extra/crossed/swapped entries all fail. |
| Tests exercise actual checked-in projection fixtures | All three positive tests `include_str!` the fixture files in `plans/closure/eggup-interoperability/fixtures/`. Negative tests mutate those checked-in fixtures in memory and confirm the harness rejects the mutations. |
| Projection schema is documentation/test-only, not production wire | The strict typed structs (`DirectProjection`, `BundleProjection`, `ArchiveProjection`) live inside `#[cfg(test)] mod projection_fixture_consistency` in `crates/eggpack-manifest/src/lib.rs`. No production API exposes them. |
| Manifest v1 wire format is unchanged | `ReleaseManifest::from_json(...).to_json()` round-trip equality remains asserted against the checked-in manifest fixtures; no production API was added; the v1 byte golden is preserved in the unchanged tests. |
| No Eggup dependency is introduced | `cargo tree -p eggpack-manifest --locked` lists only `serde`, `serde_json`, and their transitive dependencies; `cargo package -p eggpack-manifest --locked --allow-dirty` repackages cleanly; the workspace dependency tree has no Eggup crate. |
| Existing M001 tests still pass | `eggup_interoperability_manifest_fixtures_are_valid_v1` and every prior manifest/contract test passes; the manifest suite passes 26 tests including the new 14 projection consistency tests. |
| Historical closure is preserved transparently | `plans/closure/eggup-interoperability/001-status.md` already records the post-closure corrective registration and points to this closure; its evidence remains accurate for what closed M001 and is not rewritten. |

## Production implementation evidence

- `crates/eggpack-manifest/src/lib.rs`: added a new `#[cfg(test)] mod projection_fixture_consistency` block with strict typed projection structs, three relationship-aware `check_*` functions, three positive pairwise tests, and nine negative regression tests covering the full detection-gap matrix from section 7 of the corrective plan.
- `plans/closure/eggup-interoperability/fixtures/projection-bundle.json`: replaced the incorrect two-unit bundle projection (which substituted an unrelated `eggsact` entry and dropped `codegg-manifest.json`) with a three-unit projection exactly matching all three CodeGG bundle manifest entries, including the exact size and SHA-256 from the manifest and the install name propagated as both `member_id` and `relative_destination`.
- No production-code helper was added; the typed projection schemas and validation logic are deliberately scoped to the test module per section 8 of the corrective plan.
- No Eggup dependency was added; the dependency tree is unchanged.

## Verification executed

All required verification passed locally. Commands and outcomes:

```text
cargo fmt --all -- --check                                            # clean
cargo check --workspace --all-targets --locked                        # clean
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
                                                                    # no issues
cargo test -p eggpack-manifest --all-targets --all-features --locked
                                                                    # 26 passed (1 suite)
cargo test -p eggpack-contract --all-targets --all-features --locked
                                                                    # 39 passed (3 suites)
cargo test --workspace --all-targets --all-features --locked
                                                                    # 73 passed (6 suites)
cargo doc --workspace --no-deps --locked                             # clean
cargo tree -p eggpack-manifest --locked                              # serde/serde_json only
cargo package -p eggpack-manifest --locked --allow-dirty             # 6 files, 59.3 KiB
cargo +1.89.0 check --workspace --all-targets --locked               # clean
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked        # 26 passed
./scripts/check-local.sh                                             # all targets clean
git diff --check                                                     # clean
```

The new `eggpack-manifest` suite passes 26 tests, 14 of which are the new projection consistency tests (`direct_projection_is_exact_pairwise_projection`, `bundle_projection_is_exact_pairwise_projection`, `archive_projection_is_exact_pairwise_projection`, plus the detection-gap negative matrix).

## Detection-gap regression matrix

| # | Mutation applied to checked-in fixture | Test | Outcome |
|---|---|---|---|
| 1 | Substitute `eggsact-2.4.0-x86_64-unknown-linux-gnu` for `codegg-helper-2.4.0-x86_64-unknown-linux-gnu` (the original M001 defect shape) | `bundle_projection_rejects_substituting_eggsact_for_helper` | rejected (`acquisition unit count 2 != manifest bundle entry count 3`) |
| 2 | Drop the `codegg-manifest-2.4.0.json` acquisition unit | `bundle_projection_rejects_dropping_codegg_manifest_entry` | rejected (manifest entry without paired unit) |
| 3 | Add an unrelated fourth acquisition unit `unrelated-2.4.0-x86_64-unknown-linux-gnu` | `bundle_projection_rejects_unrelated_fourth_unit` | rejected (projection unit without manifest entry) |
| 4 | Cross digests/destinations between two bundle units | `bundle_projection_rejects_crossed_destinations` | rejected (no projection unit matches the manifest entry whose identity was crossed out) |
| 5 | Duplicate the first bundle unit | `bundle_projection_rejects_duplicate_unit` | rejected (extra projection unit without manifest entry) |
| 6 | Change `selected_target` to `x86_64-pc-windows-gnu` | `projection_rejects_wrong_selected_target` | rejected (no manifest target matches) |
| 7 | Cross archive member source/install/size/sha256 between two members | `archive_projection_rejects_crossed_member_relationship` | rejected (no projected member matches the manifest member) |
| 8 | Set `extraction_required` to `false` | `archive_projection_rejects_missing_extraction_required` | rejected (consumer-owned extraction seam is no longer marked) |
| 9 | Mutate the direct projection's `exact_size` and `sha256` | `direct_projection_rejects_wrong_size_or_digest` | rejected (size and digest must match manifest) |

## Invariant review

- `projection-bundle.json` now carries exactly three acquisition units, one per CodeGG manifest bundle entry; cardinality and identity match.
- Every direct/bundle projection unit's `name`, `exact_size`, `sha256`, `member_id`, and `relative_destination` match its paired manifest entry; install names are propagated as both `member_id` and `relative_destination`.
- Every archive projection member's `source`, `install`, `size`, and `sha256` match its paired manifest archive member; the single acquisition unit matches the manifest archive artifact.
- `release_identity.product_id`/`release_id` equal the manifest for direct and bundle projections.
- `transaction_group` is `one-artifact-set` for direct/bundle and `one-artifact-set-after-consumer-owned-extraction` for archive; the archive marker explicitly preserves the consumer-owned extraction seam before any Eggup `ArtifactSet` construction.
- `extraction_required` is exactly `true` for archive projection.
- Target selection remains exact canonical matching with no alias or nearest-target fallback (`wrong-target.json` remains a negative fixture; `wrong_selected_target` is rejected by the harness).
- Manifest v1 wire format, helper API surface, and dependency tree are unchanged.
- No production-code helper or schema was added for projections; the projection schema remains test/documentation-only.

## Failure/recovery review

The validation harness returns `Result<(), String>` rather than asserting so that the negative regression tests can observe the specific rejection reason (e.g. unit count mismatch, missing match, digest mismatch). The positive tests call `.expect(...)` with a descriptive message so a future drift surfaces immediately in CI with the exact reason rather than only a count. No runtime or installation behavior exists to recover from; the corrective is purely fixture/test/planning work.

## Compatibility and migration review

ReleaseManifest v1 wire format is unchanged. The corrected bundle projection is a documentation/test fixture correction; it is not a production schema migration. The original projection blob identity remains traceable through Git history and the M001 closure hash inventory; nothing is rewritten to imply the bad blob never existed.

## Security review and unresolved findings

No new dependency or authority was introduced. The projection schema is internal to the test module and never participates in serialization, network I/O, install, archive extraction, or trust assertions. SHA-256 and exact size remain integrity evidence, not authenticity. No unresolved medium-or-higher interop-evidence defect remains.

## Hosted CI

Hosted CI for the corrective commit will be appended to this closure once the push runs the Linux stable, Linux Rust 1.89, macOS, and Windows lanes. The expected CI lanes and commands mirror the M001 closure: format check, workspace check/test, Clippy, docs on stable; workspace check/test on 1.89.0; workspace check/test on macOS and Windows. All lanes must pass before this closure is treated as fully hosted.

## Roadmap disposition and dependency transitions

M001a is closed. The Eggup adapter M002 plan is unblocked for authoring. The corrected pairwise direct/bundle/archive projection matrix is the new fixture baseline an Eggup adapter implementation plan must reference; the prior `a4494c1dd9a52d0c77ab606106ccf490eafa0b13642796000a6ec1466e446439` bundle projection identity is not to be copied.

The historical M001 closure remains preserved at `plans/closure/eggup-interoperability/001-status.md` with the post-closure corrective annotation already in place; this corrective closure does not rewrite it. The eggup-interoperability roadmap, registry, and (already-corrected) bootstrap-installers roadmap are updated to reflect that M001a is the only dependency-ready interop handoff and that Eggup M002 is now ready to author (still requiring an Eggup-owned registered plan before implementation).

CI Orchestration M001, Bootstrap M002, Ecosystem adoption, and Provenance/authenticity work are unaffected by this corrective and remain independently blocked or planned.