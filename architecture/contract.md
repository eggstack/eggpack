# `eggpack-contract` — Deep Dive

Portable, versioned **expected-release-layout authority** + **pure conformance
validators**. Single-file crate (`src/lib.rs`, ~2300 lines,
`#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`).

Fidelity port of the qualified unpublished `eggup-dist` predecessor
(Eggup M002 corrective through M003). M001 imported schema/expansion; M002 added
expected/inventory/mapping validators. Both closed; M003 CLI polish planned only.

## Responsibility

- Describe one product's release layout once: target triples, platform aliases,
  asset file names, checksum sidecar names, archive members, install names
  (`crates/eggpack-contract/src/lib.rs:3-9`, `README.md:3`).
- Deterministic target expansion (`resolve` / `expand`).
- Derive required release files; validate caller-supplied release inventories,
  archive-member inventories, and observed target mappings.
- Synchronous, side-effect free. No network, process execution, archive
  extraction, installer generation, release selection, publication.

## Key types / functions (`crates/eggpack-contract/src/lib.rs`)

**Contract model:**

- `SCHEMA_V1: u32 = 1` (`:15`), `MAX_OBSERVED_ENTRIES = 256` (`:23`);
  internal bounds: findings 512, id 64B, name 128B, template 256B, detail 512B.
- `DistError` (`:47-80`): `InvalidInput`, `UnsupportedVersion{found}`,
  `DuplicateTarget`, `DuplicateAlias`, `UnknownTarget`, `UnknownPlaceholder`,
  `MissingInput`, `InvalidMemberPath`, `NameCollision{..}`,
  `InvalidReleaseInventory`, `InvalidArchiveMemberInventory`.
- `NameNamespace::{ReleaseFiles, InstallNames}` (`:28-33`).
- `ProductIdentity{id, display_name?}` (`:133-139`),
  `ChecksumSpec{sidecar}` (`:144-148`), `BundleEntry{asset, install}` (`:153-158`),
  `ArchiveMember{source, install}` (`:163-169`).
- `AssetForm::{Direct{asset,install}, Bundle{entries}, Archive{asset,members}}`
  (`:173-193`), `TargetEntry{triple, aliases, asset, checksum}` (`:197-206`),
  `DistributionContract{schema_version, product, targets}` (`:210-217`).
- `DistributionContract::parse_toml_str` (`:261`),
  `to_toml_string` (`:271`), `resolve(target_or_alias)` (`:280`),
  `expand(target_or_alias, version)` (`:300`).

**Expanded model:** `ExpandedDirect` (`:954`), `ExpandedBundle` (`:964`),
`ExpandedArchiveMember` (`:972`), `ExpandedArchive` (`:981`),
`ExpandedAssets::{Direct,Bundle,Archive}` (`:992`),
`ExpandedTarget{triple, assets}` (`:1003`).

**Conformance surface (M002):**

- `ExtrasPolicy::{AllowExtras (default), Exact}` (`:1012-1018`).
- `ExpectedReleaseFile{label, file_name}` (`:1022`) +
  `expected_release_files(contract, target_or_alias, version)` (`:1034`).
- `ReleaseInventory::new/files` (`:1079-1117`) +
  `validate_release_inventory(expected, observed, extras)` (`:1136`).
- `ArchiveMemberInventory::new/members` (`:1201-1239`) +
  `validate_archive_member_inventory(expected, observed, extras)` (`:1244`).
- `ObservedDirectMapping`, `ObservedArchiveMapping`,
  `ObservedTargetAssets::{Direct,Bundle,Archive}` (`:1298-1337`),
  `ObservedTargetMapping{target, canonical_target, assets}` (`:1346`).
- `FindingKind::{MissingReleaseFile, UnexpectedReleaseFile, TargetMismatch,
  AssetMismatch, SidecarMismatch, InstallNameMismatch, MissingArchiveMember,
  UnexpectedArchiveMember, ArchiveMappingMismatch, InvalidObservation}`
  (`:1358`), `ConformanceFinding{kind, label, expected, observed}` (`:1383`),
  `ConformanceReport{findings, truncated}` + `is_conformant()/into_result()`
  (`:1407-1438`), `validate_observed_mapping(contract, version, observed)`
  (`:1446`).

**Tests/fixtures:** `tests/fixtures.rs` (parse + expand + round-trip),
`tests/conformance.rs` (inventory/archive/mapping matrices),
`tests/fixtures/{simple-direct,codegg-bundle,egress-archive,
observed-simple,observed-codegg,observed-egress}.toml`.

## Schema-v1 essentials

- `schema_version = 1` required (`UnsupportedVersion` otherwise);
  `#[serde(deny_unknown_fields)]` everywhere — typos/future fields fail.
- `product.id` opaque `[A-Za-z0-9-_.]`, ≤64B, not `.`/`..`; `display_name` never
  expands.
- `triple` canonical Rust triple (contains `-`, no `/ \ @?#:`); `aliases`
  globally unambiguous, must not collide with any triple; `resolve` is
  exact-match only.
- Three fixed asset kinds: `direct` (`asset+install`), `bundle` (1–64
  `entries[]`), `archive` (`asset` + 1–64 `members[]`, no `install/entries`).
- Templates allow only `{product},{version},{target},{alias}` (plus `{asset}` in
  sidecars); braces reserved, no escaping/nesting; literals `[A-Za-z0-9-_.]`,
  flat (no `/\`), ≤256B. `{alias}` needs alias lookup; `{asset}` needs checksum
  context. Data substitution only.
- `version` opaque (no semver ordering) but filesystem-safe (no `/\`).
- Archive `members[].source`: literal `/`-separated relative path; rejects
  absolute, `:`, `\`, empty/`.`/`..` components.
- Uniqueness in two namespaces (release files, install names), exact +
  ASCII-case-insensitive, checked pre- and post-expansion.
- Checksums are integrity metadata only, never authenticity.
- Deterministic: order preserved, stable TOML output, sorted inventories, findings
  sorted by `(kind,label,expected,observed)`, truncated at 512.

## Boundaries (explicit non-goals)

Does not build, publish, download, verify, extract, discover releases, inspect
files, parse shell/PowerShell/Rust/workflow sources, order versions, compute
digests, sign, or open archives. Validators take caller-supplied inventories.
Roadmap non-goals: builder config, GitHub API, signatures, installer generation,
version ordering, schema-v2.

## Dependencies / dependents

- Own deps (`Cargo.toml:20-22`): `serde` (derive), `toml 0.8`
  (`parse`+`display`, no default features). No HTTP/async/archive/process/Git.
- Dependents (path, `0.1.0`): `eggpack-core`, `eggpack-bootstrap`,
  `eggpack-ci`, `eggpack-github`, `eggpack-cli`. `eggpack-manifest` deliberately
  does **not** depend on it. Fixture reuse (not package dep) across core/CI/GitHub
  tests.

## Tools / capabilities

Library-only, no binary. Capabilities: describe layout once; deterministic
expansion; derive required files; validate release / archive-member / mapping
observations. Verified via `scripts/check-local.sh`
(`test`, `cargo tree`, `cargo package`, stable + `1.89.0` lanes).
