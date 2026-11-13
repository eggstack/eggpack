# `eggpack-contract` — Deep Dive

The workspace's sole authority for release layout and names. Every other `eggpack-*` crate references
its types and may not redefine a name it owns. One file, `crates/eggpack-contract/src/lib.rs`
(2449 lines), pure and sync: its only imports are `std::collections` and `std::fmt` (`lib.rs:11-12`)
and its only dependencies are `serde` (derive) and `toml` (parse + display).

## Responsibility and authority

One `DistributionContract` describes one product's release layout once: target triples, platform
aliases, asset file names, checksum sidecar names, archive members, and installed names
(`lib.rs:3-9`, `crates/eggpack-contract/README.md:3`). Everything name-shaped in a release — what is
uploaded, what a consumer downloads, what a checksum sidecar is called, what an installer writes —
derives from this one value.

Boundaries, stated at `lib.rs:3-9` and enforced by the absence of any I/O import: no file reads, no
process execution, no network, no archive opening or extraction, no building, no publishing, no
version ordering, no digest computation. The only crate attributes are `#![forbid(unsafe_code)]`
(`:1`), `#![deny(missing_docs)]` (`:2`), and the `#![doc]` header.

**"Portable" here is a name-property claim, not a host claim.** It means the names a contract
produces stay unambiguous across case-insensitive and case-preserving filesystems and across
separator conventions:

- Templates expand to flat names only; no `/` or `\` at any stage (`:733-737`, `:911-915`).
- Archive member sources are literal `/`-separated relative paths; absolute paths, `\`, drive
  prefixes, empty components, and `.`/`..` are rejected (`:747-791`).
- Uniqueness is keyed on ASCII-lowercased strings, so `App` and `app` collide (`:925`, `:1141`,
  `:1263`).

It claims no glibc or macOS version floor; a declared floor is not something this crate can prove
(see `core-qualification.md`).

A downstream crate may reference `SCHEMA_V1`, hold a `DistributionContract`, and call `expand`. It
may not create a second place where a release file name is defined: `expected_release_files`
(`:1074`) and `validate_release_inventory` (`:1176`) compare exact strings, and the label vocabulary
(`asset`, `sidecar`, `entries[0].asset`, `archive`) is part of the contract's diagnostic surface.
Downstream code may not relax validation either — there is no unknown-key knob; `ExtrasPolicy`
governs *unrelated* names, never *unknown* ones.

## Key types / functions (with `file:line`)

All paths are `crates/eggpack-contract/src/lib.rs` unless noted.

| Item | Line | Description |
| --- | --- | --- |
| `SCHEMA_V1: u32 = 1` | `:15` | The only version understood; else `UnsupportedVersion` (`:400-404`). |
| `MAX_OBSERVED_ENTRIES: usize = 256` | `:23` | Public cap on caller inventories and observations. |
| `NameNamespace` | `:28` | `ReleaseFiles` (`:30`) / `InstallNames` (`:32`); `Display` `:35`. |
| `DistError` | `:47` | Typed, `#[non_exhaustive]` (`:46`); see [Error surface](#error-surface). |
| `ProductIdentity` | `:155` | `id` feeds `{product}`; `display_name` never expands (`:159`). |
| `ChecksumSpec` | `:166` | `sidecar` template only; integrity, never authenticity (`:163`). |
| `BundleEntry` | `:175` | One bundle row: `asset` + `install`. |
| `ArchiveMember` | `:185` | `source` (literal path) + `install` (template). |
| `AssetForm` | `:195` | `Direct{asset,install}` `:197`, `Bundle{entries}` `:204`, `Archive{asset,members}` `:209`. Not serde-derived. |
| `TargetEntry` | `:219` | `triple`, `aliases`, `asset`, `checksum`. Not serde-derived. |
| `DistributionContract` | `:232` | `schema_version`, `product`, `targets`. Not serde-derived; TOML only. |
| `parse_toml_str` | `:283` | Parse plus structural validation via private `Raw*` shapes (`:241-273`). |
| `to_toml_string` | `:293` | Deterministic render via private `to_raw` (`:462`). |
| `resolve` | `:302` | Triple-or-alias to `&TargetEntry`. |
| `expand` | `:325` | Resolved, expanded, collision-checked `ExpandedTarget`. |
| `ExpandedDirect` | `:994` | `asset_file`, `install_name`, `sidecar_file`, all expanded. |
| `ExpandedBundle` | `:1005` | `entries: Vec<ExpandedDirect>`; one sidecar per entry. |
| `ExpandedArchiveMember` | `:1012` | `source` verbatim, `install_name` expanded. |
| `ExpandedArchive` | `:1021` | `archive_file`, `sidecar_file`, `members`. |
| `ExpandedAssets` | `:1032` | `Direct` / `Bundle` / `Archive`, mirroring `AssetForm`. |
| `ExpandedTarget` | `:1043` | `triple` + `assets`; the single value `expand` returns. |
| `ExtrasPolicy` | `:1052` | `AllowExtras` (default, `:1055`) / `Exact` (`:1057`). |
| `ExpectedReleaseFile` | `:1062` | `label` + `file_name`; `Serialize` + `Ord`. |
| `expected_release_files` | `:1074` | Required flat release names for one target/version. |
| `ReleaseInventory` | `:1115` | Bounded, sorted release file names; `new` `:1125`, `files` `:1154`. |
| `validate_release_inventory` | `:1176` | Expected files vs observed inventory to `ConformanceReport`. |
| `ArchiveMemberInventory` | `:1237` | Same for member paths; `new` `:1246`, `members` `:1276`. |
| `validate_archive_member_inventory` | `:1284` | Expanded archive vs members to `Result<ConformanceReport, DistError>`. |
| `ObservedDirectMapping` | `:1338` | `asset_file` / `sidecar_file` / `install_name` as a consumer saw them. |
| `ObservedArchiveMapping` | `:1350` | `source` + `install_name`. |
| `ObservedTargetAssets` | `:1360` | Internally tagged `kind` (`:1359`). |
| `ObservedTargetMapping` | `:1386` | `target`, `canonical_target`, `assets`. |
| `FindingKind` | `:1398` | 10 stable kinds (`:1400-1418`), `Serialize` snake_case. |
| `ConformanceFinding` | `:1423` | `kind`, `label`, `expected`, `observed`; derived `Ord` sorts reports. |
| `ConformanceReport` | `:1447` | `findings` + `truncated`; `is_conformant` `:1466`, `into_result` `:1471`. |
| `validate_observed_mapping` | `:1486` | One expansion vs one observation to `ConformanceReport`. |

Private bounds: `MAX_ID_LEN` 64 (`:18`), `MAX_NAME_LEN` 128 (`:19`), `MAX_TEMPLATE_LEN` 256
(`:20`), `MAX_DETAIL_LEN` 512 (`:21`), `MAX_CONFORMANCE_FINDINGS =
MAX_OBSERVED_ENTRIES * 2` = 512 (`:24`).

## Schema-v1 essentials

Documents decode into private `RawContract` (`:245`), `RawTarget` (`:253`), and `RawAsset` (`:263`).
All three carry `#[serde(deny_unknown_fields)]` (`:244`, `:252`, `:262`), as do the four reused
public serde types (`:154`, `:165`, `:174`, `:184`): a typo such as `typo_field` is a hard parse
failure, not a silently ignored key (tested at `:2442-2448`). `schema_version` is compared to
`SCHEMA_V1` before anything else is trusted (`:400-404`) and is required, not defaulted (`:246`).

```toml
schema_version = 1                 # required, must be 1

[product]
id = "eggsact"                     # 1..=64 bytes, [A-Za-z0-9-_.], not "."/".."
display_name = "eggsact"           # optional, 1..=256 bytes, no control chars, never expands

[[targets]]                       # non-empty; this order is the canonical order
triple = "x86_64-unknown-linux-gnu"
aliases = ["linux-x64"]            # optional, default []

[targets.asset]                   # exactly one of three forms
kind = "direct"                    # "direct" | "bundle" | "archive"
asset = "{product}-{version}-{target}"
install = "{product}"

[targets.checksum]
sidecar = "{asset}.sha256"         # may additionally use {asset}
```

- **Triple** must contain `-` (`:671-675`) and use only `[A-Za-z0-9-_.]`, with no separator, space,
  or `@?#:` (`:661-670`). An alias must be a bare token too but need not contain `-` (`:679-693`).
- **Asset form exclusivity**: `direct` requires `asset`+`install` and rejects `entries`/`members`
  (`:516-531`); `bundle` requires non-empty `entries` and rejects
  `asset`/`install`/`members` (`:532-566`); `archive` requires `asset`+`members` and rejects
  `install`/`entries` (`:567-613`). Entry and member counts are bounded to 1..=64 (`:541-546`,
  `:580-587`).
- **Templates** accept only `{product}`, `{version}`, `{target}`, `{alias}`, plus `{asset}` when
  `allow_asset` is set — checksum sidecars only (`:838-845`, `:743`). Braces are reserved: no
  escaping, no nesting, no stray `}`, no deferred syntax (`:809-865`); literals are
  `[A-Za-z0-9-_.]` (`:852-856`). Expansion is data substitution only, and the result is re-checked
  for emptiness, length, leftover braces, and separators (`:905-915`).
- **`version`** is opaque — no SemVer ordering, no prerelease semantics — but portable: non-empty,
  ≤128 bytes, and `[A-Za-z0-9-_.+]` only. It is interpolated into release file names, so the
  alphabet is the intersection of what `eggpack-manifest` accepts as an artifact file name and what
  every supported platform can store: `:` (rejected by the manifest) and `* ? " < > |` and spaces
  (not representable on Windows) are refused at expansion. `+` is admitted for SemVer build
  metadata such as `1.2.3-rc.1+build.5` (`validate_version`).
- **Member `source`** is literal, never templated; `{` and `}` are rejected (`:758-762`).
- **Pre-expansion uniqueness** covers bundle entry assets and installs, archive member installs, and
  archive member `source` paths, using bounded labels such as `entries[0].asset` (`:551-564`,
  `:605-611`). Member sources are keyed on the ASCII-lowercased value, matching
  `ArchiveMemberInventory::new` and `validate_observed_mapping`: a case-colliding pair is rejected
  here rather than parsing into an archive contract that its own inventory would refuse. Sidecar collisions
  cannot be checked here — one sidecar template per target, expanded once per asset — so they
  surface at `expand` time.

## Target and alias resolution

`resolve` (`:302-312`) is a linear scan over `targets`: for each target it compares the input
against `triple`, then against each alias, with plain `String` equality. No case folding, no
trimming, no separator normalisation, no nearest-architecture fallback — an unknown input is
`UnknownTarget` (`:311`).

Alias resolution is safe because the two lookup spaces are provably disjoint by validation, not by
convention:

- An alias equal to *any* declared triple is rejected at parse time, even when it belongs to that
  triple's own target (`:430-434`).
- An alias owned by two different targets is rejected; the same alias repeated by one target is
  tolerated (`:435-442`).
- Triples must contain `-` while aliases need not, so no triple-shaped string can be a mis-spelled
  alias (`:671-675`).

Given those, the per-target triple-then-alias order in `resolve` cannot produce an order-dependent
answer: at most one target can match, and within it the two branches cannot both be live. This is
the one place in the workspace where a caller-supplied string is *resolved* rather than matched
exactly against a declared identity; the exact-match rule for artifact identity is described in
`validation-model.md`.

`ExtrasPolicy` has **no** effect on resolution. It appears only in the two inventory validators
(`:1179`, `:1220`, `:1287`, `:1320`).

`expand` records *how* the lookup happened: `{alias}` expands to the alias used for the lookup, and
a triple lookup leaves the alias input empty, so a contract using `{alias}` fails with
`MissingInput` when expanded by triple (`:334-338`, `:887-894`).

## The `expand` projection

`expand(target_or_alias, version)` (`:325-397`) runs a fixed pipeline:

1. `validate_version` (`:330`).
2. `resolve` (`:331`).
3. Build `ExpansionCtx { product, version, target: &target.triple, alias: alias_used }`
   (`:339-344`). `{target}` is always the *canonical triple*, never the alias the caller typed.
4. Expand per form (`:345-390`). The sidecar is expanded with the already-expanded asset name
   supplied as `{asset}` (`:349-350`, `:362-363`, `:374-375`), which is why a bundle yields one
   sidecar per entry.
5. `validate_expanded_names` on the result (`:395`).

Why a projection exists: the input types are authority-shaped and carry unresolved templates plus
whatever the caller constructed by hand; the `Expanded*` family is a fully resolved,
collision-checked view in which every release name and install name is a flat concrete string.
Downstream stages consume `ExpandedAssets` directly
(`crates/eggpack-core/src/builder.rs:5`, `crates/eggpack-bootstrap/src/lib.rs:4`,
`crates/eggpack-github/src/lib.rs:13`) and never re-implement template expansion or re-derive the
sidecar name.

`Expanded*` mirrors rather than flattens `AssetForm`: `ExpandedAssets` has the same three variants,
`ExpandedTarget.triple` is the canonical triple, and `ExpandedBundle.entries` preserves declaration
order — only release *files* and archive *members* are sorted, and only inside validators.
`ExpandedArchiveMember.source` is copied through unchanged (`:380`); only `install` is a template.
Because the projection holds expanded strings, the `Expanded*` types derive `PartialEq`/`Eq` and can
be compared directly against observations.

Collision checking (`validate_expanded_names`, `:950-988`) is **per expanded target**, not per
contract: it collects only that target's release and install names, so two different targets may
legitimately install the same names — which is what `tests/fixtures/egress-archive.toml` does
(`egress`, `egress-helper` on both its Linux and macOS targets). What is rejected is a collision
inside one target, including ASCII-case-only collisions.

## Two inventories, two validators

| | `ReleaseInventory` | `ArchiveMemberInventory` |
| --- | --- | --- |
| Struct / constructor | `:1115` / `new` `:1125` | `:1237` / `new` `:1246` |
| Accessor | `files()` `:1154` | `members()` `:1276` |
| Name rule | flat, no separators, no control chars, ≤256 B (`:1159-1170`) | same traversal-free `/`-path rules as schema v1, via `validate_member_source` (`:1260`) |
| Duplicate rule | exact and ASCII-case duplicate rejected (`:1141-1146`) | exact and ASCII-case duplicate rejected (`:1263-1268`) |
| Bound failure | `InvalidReleaseInventory` (`:1134`, `:1140`, `:1143`) | `InvalidArchiveMemberInventory` (`:1255`, `:1261`, `:1265`) |
| Determinism | `names.sort()` (`:1149`) | `paths.sort()` (`:1271`) |
| Expected side | `&[ExpectedReleaseFile]` from `expected_release_files` (`:1177`) | `&ExpandedTarget`, which must be an archive target (`:1284-1289`) |
| Return type | `ConformanceReport` (`:1180`) | `Result<ConformanceReport, DistError>` (`:1288`) |
| Findings | `MissingReleaseFile`, `UnexpectedReleaseFile` (`:1213`, `:1224`) | `MissingArchiveMember`, `UnexpectedArchiveMember` (`:1313`, `:1324`) |
| Finding labels | contract field labels: `asset`, `sidecar`, `entries[0].asset`, `archive` (`:1083-1100`) | the member source path itself (`:1314`) |
| `AllowExtras` | extras ignored; only missing required files reported (`:1210-1219`) | same (`:1310-1319`) |
| `Exact` | undeclared release file becomes `UnexpectedReleaseFile` (`:1220-1231`) | undeclared member path becomes `UnexpectedArchiveMember` (`:1320-1331`) |
| Wrong target type | not applicable | `Err(InvalidInput)` "archive member validation requires an archive target" (`:1289-1293`) |

Why separate: they answer different questions about different namespaces with different grammar
and different producers. A release file is a flat name a client downloads from a release; an
archive member is a nested literal path inside a `.tar.gz` produced by the builder. Their safety
rules differ (`dir/asset` is rejected as a release file, `bin/tool` is a legal member), their error
variants differ, and `validate_archive_member_inventory` needs the whole `ExpandedTarget` because
member sources live on the archive projection, not on `ExpectedReleaseFile`. Neither validator
opens a file, an archive, or a release: both take caller-supplied names.

Both are fail-closed on bad *expected* input: an over-limit or malformed expected set is reported
as an `InvalidObservation` finding rather than ignored (`:1181-1194`, `:1294-1306`).

## Conformance validation

`validate_observed_mapping(contract, version, observed)` (`:1486-1535`) compares what a consumer
claims it shipped against what the contract says it should ship:

1. `valid_observation` gate (`:1491`, `:1537-1594`) — non-empty bounded `target` /
   `canonical_target` without control characters, flat names, traversal-free member sources,
   ASCII-unique release names, and a total name count at or under `MAX_OBSERVED_ENTRIES`. Failure
   yields one `InvalidObservation` finding.
2. `contract.expand(&observed.target, version)` (`:1499`); an unresolvable target or alias becomes
   a `TargetMismatch` finding, and every other expansion failure becomes an `InvalidObservation`
   labelled `expansion` (`:1501-1522`) — the function never returns `Err`.
3. Canonical triple comparison (`:1525-1532`).
4. `compare_observed_assets` (`:1533`).

- **Bundle entries are order-insensitive.** Both sides are sorted by `asset_file` before zipping
  (`:1635-1636`), so a reordered observation is not drift. A differing entry count produces one
  `AssetMismatch` labelled `bundle entry count` (`:1661-1677`).
- **Archive members are diffed as a `source -> install_name` map** over the sorted union of both key
  sets (`:1701-1728`), so an absent member and a member with a wrong install name are the same
  `ArchiveMappingMismatch` kind, with `expected` or `observed` as `None`.
- **A form mismatch** (contract says `archive`, consumer claims `direct`) is one `AssetMismatch`
  labelled `asset form` (`:1730-1735`).

`FindingKind` (`:1398`) is `Copy` + `Ord` + `Serialize` (snake_case, `:1397`); `ConformanceFinding`
(`:1423`) derives `Ord` over `(kind, label, expected, observed)`, and `ConformanceReport::new` sorts
then truncates at 512 findings, setting `truncated` (`:1455-1463`).

A report rather than a boolean, because the consumer of a conformance check is a human or a golden
comparison, not a branch: the first error hides the rest of the diff, a sorted list is byte-stable
across runs, and `into_result` (`:1471`) returns the whole `ConformanceReport` in the error position
so a caller can render every finding before failing. `is_conformant` (`:1466`) remains for
boolean-style callers.

The observation types carry a deliberate negative boundary, stated at `:1381-1383`: no URLs, version
policy, commands, privileges, scripts, or service behaviour. Eggpack compares data facts; it does
not parse consumer shell, PowerShell, Rust, or workflow sources.

## Error surface

`DistError` (`:47-80`) is `#[non_exhaustive]` (`:46`) so adding variants is not a breaking change for
matchers. `Display` is hand-written (`:88-119`) and implements `std::error::Error` (`:121`).

| Variant | Line | Raised when |
| --- | --- | --- |
| `InvalidInput(String)` | `:49` | Any schema violation; also wraps TOML parse/render failures (`:285`, `:295`). |
| `UnsupportedVersion { found }` | `:51` | `schema_version != 1` (`:401`). |
| `DuplicateTarget(String)` | `:56` | Two targets share a canonical triple (`:418`). |
| `DuplicateAlias(String)` | `:58` | Alias collides with a triple, or with another target's alias (`:431`, `:437`). |
| `UnknownTarget(String)` | `:60` | No target matches (`:311`). |
| `UnknownPlaceholder(String)` | `:62` | Placeholder outside the v1 grammar (`:844`). |
| `MissingInput(String)` | `:64` | `{alias}` on a triple lookup, or `{asset}` without checksum context (`:889`, `:897`). |
| `InvalidMemberPath(String)` | `:66` | Unsafe member source path (`:749` and siblings in `:747-791`). |
| `NameCollision { namespace, first, second }` | `:68` | Raw or expanded name collision (`:927`). |
| `InvalidReleaseInventory(String)` | `:77` | Bad `ReleaseInventory` input. |
| `InvalidArchiveMemberInventory(String)` | `:79` | Bad `ArchiveMemberInventory` input. |

The fail-closed rule is structural: every validator returns `Result` or a report; there is no
best-effort path, no default target, and no partial contract. Messages stay bounded by `bound()`
(`:132-137`), which truncates any string to 512 bytes, applied through `DistError::invalid`
(`:83-85`) and directly at `:311`, `:418`, `:429`, `:747-790`, `:889`, `:897`, `:1140`, `:1261`.

On leaking: this crate runs no process and reads no file, so there is no captured external output to
leak. Error strings originate in the caller's own TOML document, in bounded field labels, or in the
caller's own name arguments. The one indirect case is
`DistError::invalid(format!("TOML parse: {e}"))` (`:285`), which quotes the `toml` crate's diagnostic
for the document the caller supplied — parser output over caller input, not external command
output, and bounded by `bound()`.

## Tests and fixtures

Unit tests live in `src/lib.rs` `mod tests` (`:1780-2449`), 26 tests, driven by three inline TOML
constants `SIMPLE` (`:1784`), `BUNDLE` (`:1803`), `ARCHIVE` (`:1832`) and the builders `bundle_with`
(`:1858`), `archive_with` (`:1893`). They cover version rejection, duplicate triple and alias,
alias/triple collision, unknown placeholder, the malformed-template matrix (`:2036-2057`), pre- and
post-expansion collision cases, `{alias}` input, member-path traversal, opaque-but-safe versions,
byte-stable round-trip (`:2430-2439`), and unknown-field rejection.

`tests/fixtures.rs` (5 tests) loads files from `tests/fixtures/` via `CARGO_MANIFEST_DIR`, asserts
expansion per layout, then that parse → `to_toml_string` → parse is value-stable (`:49-59`).
Contract M003 added `consumer_direct_targets_fixture_matches_the_live_consumer_shape`
(`:63`), which pins the exact public asset, sidecar, and install names of the adopted eggsact
contract, including the `.exe` rule for `*-pc-windows-msvc` that the older
`eggsact-direct-targets.toml` fixture does not carry, and pins ARMv7 as deliberately absent.
`tests/conformance.rs` (12 tests) is the inventory and mapping matrix: default-`AllowExtras` acceptance
(`:29`), the golden observation pairs (`:43`), missing + exact-extra ordering (`:58`), per-entry
bundle requirements (`:74`), archive member validation without opening an archive (`:88`), input
rejection including `MAX_OBSERVED_ENTRIES + 1` (`:134`), per-field mapping drift (`:190`), bundle
reorder insensitivity (`:306`), and finding-sort stability (`:341`).

| Fixture | Represents |
| --- | --- |
| `simple-direct.toml` | `eggsact`, two `direct` targets (`linux-x64`, `macos-arm64`), one asset + one sidecar each. |
| `codegg-bundle.toml` | `codegg`, one target, three-entry bundle (runfile, helper, manifest JSON). |
| `egress-archive.toml` | `egress`, two targets, `.tar.gz` archive with two literal members; same install names on both targets. |
| `eggsact-direct-targets.toml` | `eggsact`, five `direct` targets, version-bearing names and no `.exe` rule. Used by `eggpack-ci` (`crates/eggpack-ci/src/tests.rs:4967`); no longer used by `eggpack-cli`, whose M003 test moved to `consumer-direct-targets.toml`. |
| `consumer-direct-targets.toml` | Byte-identical copy of the adopted eggsact contract at `eggstack/eggsact@d4e6e5c` (`release/eggpack/distribution.toml`): five `direct` targets, version-free public asset names, and the contract-level `.exe` rule. Added by Contract M003 so the CLI's consumer-shaped test compares against the names consumers actually publish. |
| `observed-simple.toml` | `ObservedTargetMapping` for `simple-direct.toml` at `1.2.6`. |
| `observed-codegg.toml` | `ObservedTargetMapping` for `codegg-bundle.toml` at `0.9.0`, looked up by alias. |
| `observed-egress.toml` | `ObservedTargetMapping` for `egress-archive.toml` at `2.1.0`, looked up by alias. |

The three `observed-*` files are the *positive* goldens: each must conform (`conformance.rs:43-55`),
and each pins the serde wire shape of `ObservedTargetMapping` — `[assets]` with `kind = "direct" |
"bundle" | "archive"`, matching the internally tagged enum at `:1359`. Their value is that they are
written by hand rather than produced by `expand`, so they would drift the moment a template rule
changed. The negative evidence is the in-test mutation set instead: wrong asset/sidecar/install
(`:190-210`), wrong `canonical_target` (`:181-186`), a corrupted member install name (`:295-302`), a
swapped bundle with shifted install names (`:323-337`), and an over-limit observation (`:343-359`).

Publication metadata in `crates/eggpack-contract/Cargo.toml`: `readme` (`:11`), `documentation`
(`:9`), `keywords` (`:13`), `categories` (`:14`), and `exclude = ["tests/", "benches/"]` (`:15`).
`eggpack-manifest` also carries `documentation`/`categories`/`keywords`; the `exclude` and the
`lints` workspace inheritance (`:17-18`) are contract-specific. No other crate declares publication
categories.

## Tools / capabilities

Library only — `src/` contains `lib.rs` and no `[[bin]]`. The canonical call sequence, as used in
production by `eggpack-core`:

```rust
let contract = DistributionContract::parse_toml_str(&text)?;                  // lib.rs:283
let expanded = contract.expand(&target.target, &release_id)?;                  // lib.rs:325
let expected = expected_release_files(&contract, &target.target, &release_id)?; // :1074
let inventory = ReleaseInventory::new(names)?;                                 // :1125
if !validate_release_inventory(&expected, &inventory, ExtrasPolicy::Exact).is_conformant() {
    return Err(/* … */);
}
```

That is the pattern at `crates/eggpack-core/src/lib.rs:353-366`. Every step is pure: parsing,
resolving, expanding, comparing names.

Contract M003 added the crate's one consumer-facing entry point, which is a CLI
projection over the first two calls and nothing else:

```sh
eggpack contract expand --contract <file> --release-id <opaque> \
  --target <triple-or-alias> --field <canonical-target|asset|sidecar|install>
```

It delegates to `parse_toml_str` and `expand` and adds no template grammar, no target or alias
resolution, and no serialized expansion document. The one judgement it makes — `canonical-target`
answers for every asset form, while `asset`/`sidecar`/`install` fail closed for bundle and archive
targets — is stated at `crates/eggpack-cli/src/main.rs:296-336`. See
[cli.md](cli.md) for the argument contract and the deliberately absent capabilities. No consumer may
read consumer policy (latest/exact selection, Cargo fallback, install destination, updater,
compatibility floors) out of this output; the crate has no opinion about any of it. Anything needing the outside world — listing a release,
opening a `.tar.gz`, hashing a file — happens in the caller, which hands names back through the two
inventory types. The crate-level rustdoc is the `#![doc]` header at `lib.rs:3-9`; the worked example
lives in `crates/eggpack-contract/README.md:38-52`.

## Dependencies / dependents

Dependencies (`crates/eggpack-contract/Cargo.toml:20-22`): `serde` with `features = ["derive"]` for
the four reusable public serde types plus the private `Raw*` shapes; `toml` 0.8 with
`default-features = false, features = ["parse", "display"]`, narrowed to exactly the two
capabilities the crate calls — `toml::from_str` (`:285`) and `toml::to_string` (`:295`). No HTTP,
async, archive, process, or Git dependency exists.

Determinism consequence: rendered TOML field order comes from `RawContract`'s derived field order
(`:462-509`), not from the input document's key order, and `targets` is a `Vec` (`:248`), so target
order is declaration order and is preserved. Round-trip stability is asserted at `:2430-2439` and in
`tests/fixtures.rs:49-59`.

Dependents, each by path plus `version = "0.1.0"`: `eggpack-core`
(`crates/eggpack-core/Cargo.toml:18`), `eggpack-bootstrap`
(`crates/eggpack-bootstrap/Cargo.toml:15`), `eggpack-ci` (`crates/eggpack-ci/Cargo.toml:15`),
`eggpack-github` (`crates/eggpack-github/Cargo.toml:15`), and `eggpack-cli`
(`crates/eggpack-cli/Cargo.toml:19`). `eggpack-manifest` does **not** depend on this crate; it has
no `eggpack-*` path dependency and instead mirrors the contract fixture values in its
own tests (`crates/eggpack-manifest/src/lib.rs:615-642`).

Verification: `scripts/check-local.sh:7`
(`test -p eggpack-contract --all-targets --all-features --locked`), plus
`cargo tree -p eggpack-contract` (`:13`), `cargo package -p eggpack-contract` (`:20`), and the
`1.89.0` check and test lanes (`:46-47`).

## Related deep dives

- [overview.md](overview.md) — module map and the producer pipeline.
- [core-planning.md](core-planning.md) — plan construction consuming `DistributionContract`.
- [core-finalization.md](core-finalization.md) — `expected_release_files` and
  `validate_release_inventory` at manifest build time.
- [manifest.md](manifest.md) — the evidence record derived from expanded names.
- [validation-model.md](validation-model.md) — exact-match resolution and the fail-closed model shared
  workspace-wide.
- [determinism.md](determinism.md) — canonical sort and stable serialization.
- [testing-and-portability.md](testing-and-portability.md) — fixture reuse across crates.
- [bootstrap.md](bootstrap.md) — installer rendering from `ExpandedAssets`.
- [ci.md](ci.md) — workflow rendering from expanded target names.
