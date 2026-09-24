# Eggup Interoperability Milestone 001 Closure — Consumer Contract and Fixtures

Status: closed

Source plan: `plans/implementation/eggup-interoperability/001-manifest-consumer-contract-and-fixtures.md`

Roadmap: `plans/subsystems/eggup-interoperability-roadmap.md`

Reviewed Eggpack baseline: `bc0c9714fd808be0906df00a049c5941ea4f6d24` (Bootstrap M001 closed; Eggpack-side interface work then proceeding). Final implementation baseline: `99c9040a5d106cfa46a4ba02f9fa0cee8653166c`.

External Eggup baseline: `eggstack/eggup@2cab1f97ef30fa347c2030da321462459672c521`.

Hosted CI: [run 35956171209](https://github.com/eggstack/eggpack/actions/runs/35956171209), all Linux stable, Linux Rust 1.89, macOS, and Windows jobs passed.

## Executive finding

The Eggpack-side ReleaseManifest v1 → Eggup seam is documented and fixture-backed. Direct and bundle facts preserve exact identity, artifact/install relationships, sizes, and SHA-256 requirements without importing producer build types. Archive fixtures preserve archive acquisition and member facts and explicitly mark that consumer-owned qualified extraction is required before producing local files for an Eggup `ArtifactSet`/`InstallPlan`. Manifest v1 wire data is unchanged. No Eggup repository changes were made.

## External API evidence reviewed

At the pinned Eggup commit, these exact files were read:

- `crates/eggup-core/src/domain.rs`: `ProductId`, `ReleaseId`, and `MemberId` are opaque validated string wrappers; `IntegrityRequirement::Sha256([u8; 32])` carries integrity only; `ArtifactMember` accepts a local source and normalized relative destination; `ArtifactSet` is non-empty with unique member ids; `InstallPlan::new` accepts an explicit installation root.
- `crates/eggup-acquisition/src/lib.rs`: `AcquisitionRequest` stores one caller-selected HTTP(S) URL without discovery/fallback; `FetchLimits` carries maximum artifact bytes and connect/total timeouts (defaults include 128 MiB, 10 seconds, and 120 seconds).

Eggup identity constructors impose generic nonempty/control-character bounds and no version ordering. These APIs support direct/bundle projection without Eggpack build/core dependencies. They do not establish generic archive extraction. Eggpack did not run or modify Eggup.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Durable ownership and mapping contract | `architecture/eggup-manifest-consumer-v1.md` defines opaque identities, exact target selection, direct/bundle/archive mapping, exact-size semantics, and caller-owned origin/root/ownership. |
| Direct, bundle, archive release facts | Canonical JSON fixtures `direct-manifest.json`, `bundle-manifest.json`, and `archive-manifest.json`; all parse and serialize to the same deterministic v1 JSON. |
| Expected consumer projections | `projection-direct.json`, `projection-bundle.json`, and `projection-archive.json` record target, units, size/digest, member/install identities, grouping, and archive extraction-required state. They are explicitly non-production documentation/test fixtures. |
| Negative target/schema/digest/size behavior | `wrong-target.json`, `unknown-schema.json`, `corrupt-digest.json`, and `corrupt-size.json`; manifest tests reject unsupported schema/digest/size and exact canonical lookup rejects target mismatch without alias fallback. |
| Generic, wire-neutral helper APIs | `ReleaseManifest::target`, `ArtifactRecord::sha256_bytes`, and `ByteEvidence::sha256_bytes`; tests confirm exact target behavior, byte conversion, invalid digest rejection through schema parsing, and unchanged JSON output. |
| Product/release opacity and relationship retention | Tests retain direct product/release strings, require all three CodeGG bundle entries with their paired installs, and preserve Egress archive/member identities and exact size/digest fields. |
| No Eggup dependency or wire-format change | `cargo tree -p eggpack-manifest --locked` shows only Serde/serde_json dependencies; no production Cargo dependency on Eggup was added; fixtures round-trip through the unchanged v1 parser/serializer. |

## Fixture inventory and identities

SHA-256 of the checked-in fixture blobs at the implementation commit:

| Fixture | SHA-256 |
|---|---|
| `archive-manifest.json` | `723f1ed30b6bc7dae802ff54ac57afd6c8c8ea6be0e5bcc05c30ef56d4f647d1` |
| `bundle-manifest.json` | `440e871d6cd481bbf1a19ae0711ad9f438b3144315e939a91e66ee3b63b828cc` |
| `corrupt-digest.json` | `05561d7c0fbedb47195f10c2d666cfadc8587fc4d056c7adc4d0f77b35af2db2` |
| `corrupt-size.json` | `abdd9143db59016cc5b349413befd7d268529fbd09a5297f5772698878083a28` |
| `direct-manifest.json` | `04b1993f8ed1de1a4f19819f643831d10ecf87e32c783ab83259fd6cd55947f2` |
| `projection-archive.json` | `c8a2a6e33bc4336cd65cd9b0c4d94c581ba574967f111663b2e70498a7cc8e46` |
| `projection-bundle.json` | `a4494c1dd9a52d0c77ab606106ccf490eafa0b13642796000a6ec1466e446439` |
| `projection-direct.json` | `63cb46479a78479929c9bcd2040268435095ee2cc49506bdb1f2c93a04be3760` |
| `unknown-schema.json` | `3527fa83397409981974e498087f2e8e932215ed0115c22793b891d5d917160e` |
| `wrong-target.json` | `cd5dd514216a45aea4a2a83b98208bb15c159b8b05bc61441647eae4e39f18f2` |

## Verification executed

Passed locally:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-manifest --locked
cargo package -p eggpack-manifest --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
git diff --check
```

`scripts/check-local.sh` exited successfully. The complete workspace suite passed 60 tests, including deterministic fixture round trips and negative schema/digest/size cases. No Eggup dependency appeared in the manifest dependency tree.

## Invariant, failure, compatibility, and security review

The seam gives no release-selection, origin, installation-root, ownership, rollback, service, authenticity, or receipt authority to the manifest. SHA-256 remains integrity evidence; exact size is an acquisition ceiling plus post-fetch exactness check. Target choice is exact/canonical and there is no fallback. Direct and bundle entries map to one coherent consumer transaction. Archive acquisition is kept distinct from member installation and does not fabricate an `InstallPlan` from the archive file.

No wire fields or serialization semantics changed. Helpers are additive and non-serialized. The projection fixtures are clearly test/documentation data rather than a new runtime format. No unresolved medium-or-higher interface ambiguity remains for the direct/bundle seam; archive extraction remains explicitly outside M001.

## Roadmap disposition and dependency transitions

Eggpack Interoperability M001 is closed. It is now safe to author and register the optional Eggup-side M002 adapter plan against Eggup baseline `2cab1f97ef30fa347c2030da321462459672c521` and this fixture/interface baseline. The adapter must remain an Eggup repository plan; this closure does not claim Eggup migration or authorize code there. The next plan is unblocked for authoring only, and no Eggup repository change occurred under this milestone.


## Post-closure corrective registration

Subsequent review found that `projection-bundle.json` does not correspond to its paired `bundle-manifest.json`: the projection omitted required CodeGG bundle entries and substituted an unrelated `eggsact` unit. Existing M001 tests round-tripped the manifest fixture but did not mechanically compare the bundle projection against it.

The historical implementation/CI evidence above remains accurate for the code and fixtures that closed M001; it is not rewritten to conceal the later finding.

Corrective plan: `plans/implementation/eggup-interoperability/001a-projection-fixture-consistency-corrective.md`.

The prior transition authorizing Eggup adapter M002 plan authoring is withdrawn until M001a closes with corrected pairwise direct/bundle/archive projection evidence. A separate `plans/closure/eggup-interoperability/001a-status.md` will record the corrective closure.
