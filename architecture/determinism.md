# Determinism and Stable Serialization — Deep Dive

This document describes the workspace-wide requirement that identical inputs
produce byte-identical outputs, and the mechanisms that implement it: canonical
sort, stable `serde` field order, deterministic archive headers, and drift
checking. It is a cross-cutting concern rather than one crate's — the guarantee
spans `contract`, `manifest`, `core`, `bootstrap`, `ci`, `github`, and the
wiring in `cli`, so a change in any one of them can silently break output other
crates and third parties depend on.

## The contract

The invariant is *identical inputs, byte-identical outputs*. Five distinct
mechanisms uphold it, of differing strength:

1. **Canonical sort at construction** — collections are sorted when the value is
   built (`ReleasePlan.targets`, `ReleaseInventory`, `CIPlan` targets, staging
   assets).
2. **Canonical sort at serialization** — a clone is sorted inside `to_json`
   immediately before emission, so output is canonical regardless of how the
   in-memory value was assembled (`ReleaseManifest::to_json`).
3. **Validation rejecting non-canonical input** — `CIPlan::validate` fails
   unless targets are strictly ascending, so a hand-edited plan cannot enter a
   render out of order.
4. **Explicitly zeroed archive metadata** — every tar and gzip header field that
   could carry host or time information is written as a constant.
5. **Drift checking** — `eggpack ci check` re-renders and compares, so the
   checked-in workflow is verified rather than trusted.

The two kinds of ordering are different guarantees. Where order is *preserved
from input* (contract declaration order, bundle index order), the producer's
bytes are stable but not canonical: two semantically equal contracts declaring
`b` before `a` produce different archives. Where order is *canonically imposed*,
the same facts always yield the same bytes.

## Canonical sort

| What is ordered | Sort key | Where | Guarantee |
| --- | --- | --- | --- |
| `ReleasePlan.targets` | `target` (canonical triple) | `crates/eggpack-core/src/lib.rs:575` | imposed at construction |
| `ReleasePlan` on re-serialize | *not re-sorted* | `crates/eggpack-core/src/lib.rs:489-498` | relies on construction order |
| `ReleaseInventory.files` | lexical filename | `crates/eggpack-contract/src/lib.rs:1109` | imposed at construction |
| `ArchiveMemberInventory.members` | lexical member path | `crates/eggpack-contract/src/lib.rs:1231` | imposed at construction |
| `ConformanceReport.findings` | derived `Ord`: kind, label, expected, observed | `crates/eggpack-contract/src/lib.rs:1416`; derive `:1382` | imposed before truncation |
| `ReleaseManifest.targets` | `target` | `crates/eggpack-manifest/src/lib.rs:242` | imposed at serialization |
| `ReleaseManifest` bundle entries | `(artifact.name, install)` | `crates/eggpack-manifest/src/lib.rs:245-247` | imposed at serialization |
| `ReleaseManifest` archive members | `(source, install)` | `crates/eggpack-manifest/src/lib.rs:249` | imposed at serialization |
| Finalization manifest targets | `target` | `crates/eggpack-core/src/finalization.rs:297` | imposed before `build_manifest` |
| `StagingPayloadV1.assets` | `name` | `crates/eggpack-github/src/lib.rs:247` (serialize), `:1589` (build), `:883` (wrapper variant) | both, belt and braces |
| `GitHubDraftReceiptV1.assets` | `name` | `crates/eggpack-github/src/lib.rs:2194-2195` | imposed at construction |
| `CIPlan` job outputs | `selector` | `crates/eggpack-ci/src/lib.rs:182` | imposed at construction |
| `CIPlan.targets` (jobs) | `planned.target` | `crates/eggpack-ci/src/lib.rs:195` | imposed at construction |
| `CIPlan.required_aggregation_dependencies` | lexical job id | `crates/eggpack-ci/src/lib.rs:196` | imposed at construction |
| `ReleaseCIPlanV1.qualifications` | `target` | `crates/eggpack-ci/src/lib.rs:2504` | imposed at construction |
| `LogicalOutputSelector` comparison | derived `Ord`: `Direct` < `BundleEntry{index}` < `ArchiveMember{source}` | `crates/eggpack-core/src/builder.rs:47-62` | derived, declaration-ordered |
| Qualification evidence candidates | `selector` | `crates/eggpack-core/src/finalization.rs:632` | imposed before evidence assembly |
| Bootstrap installer cases | `(os, arch)` | `crates/eggpack-bootstrap/src/lib.rs:613-620` | imposed at projection |
| Archive comparison source set | lexical, then `dedup` | `crates/eggpack-contract/src/lib.rs:1661-1662` | imposed over a merged `HashMap` key set |
| Bundle conformance comparison | `asset_file` both sides, before positional zip | `crates/eggpack-contract/src/lib.rs:1581-1582` | imposed before zip |

`LogicalOutputSelector` derives `Ord` from enum variant declaration order, so
`Direct` sorts before every `BundleEntry` regardless of index, and `BundleEntry`
before every `ArchiveMember` (`crates/eggpack-core/src/builder.rs:47`). That
ordering is what makes `CIPlan::validate`'s strict-ascending check
(`crates/eggpack-ci/src/lib.rs:333`) and finalization's candidate matching
(`crates/eggpack-core/src/finalization.rs:395`) agree.
Reordering the variants would silently change both — the main structural hazard
in this table.

Order that is *preserved*, not imposed:

- Contract declaration order for `members` and `entries` survives expansion
  (`crates/eggpack-contract/src/lib.rs:351-358`) and reaches the archive.
  `expected_release_files` documents its results as "ordered by contract
  declaration" (`crates/eggpack-contract/src/lib.rs:1032`).
- `DistributionContract::to_toml_string` explicitly preserves stored field and
  target order (`crates/eggpack-contract/src/lib.rs:269-270`).
- `BundleEntry { index }` carries contract order inside the selector itself
  (`crates/eggpack-core/src/builder.rs:53-56`), so bundle slot identity is
  declaration-derived.

The net guarantee is split: the release *manifest* is canonical (sorted at
serialization) while the archive *byte layout* follows contract declaration
order. Both are deterministic; only the manifest is order-independent.

## Stable serialization

Every serialized document derives `serde::Serialize` from a struct definition,
so emission order is field order. Stability therefore depends on field
declarations being treated as part of the wire contract rather than as
incidental layout. Examples: `ReleaseManifest`
(`crates/eggpack-manifest/src/lib.rs:59-72`), `StagingPayloadV1`
(`crates/eggpack-github/src/lib.rs:206-228`), `ReleaseCIPlanV1`
(`crates/eggpack-ci/src/lib.rs:2309`).

Two conventions make this fail closed:

- `deny_unknown_fields` on essentially every document type
  (`crates/eggpack-manifest/src/lib.rs:58`, `:77`, `:112`, `:124`, `:134`,
  `:147`; `crates/eggpack-github/src/lib.rs:187`, `:205`, `:297`;
  `crates/eggpack-ci/src/lib.rs:1861`, `:2308`;
  `crates/eggpack-core/src/builder.rs:66`, `:78`). An unrecognized field is a
  parse error, not a dropped key, so a third party's re-serialization cannot
  lose or reorder facts.
- Internally tagged enums with explicit tag name and
  `rename_all = "snake_case"`: `ArtifactForm`
  (`crates/eggpack-manifest/src/lib.rs:87`), `LogicalOutputSelector`
  (`crates/eggpack-core/src/builder.rs:48`), `FindingKind`
  (`crates/eggpack-contract/src/lib.rs:1356-1357`), and `PlannedAssetForm`
  (`crates/eggpack-core/src/lib.rs:478`).

`Option` fields are not elided unless declared. The `skip_serializing_if` uses
are narrow and deliberate: `evidence_references` on `ReleaseManifest`
(`crates/eggpack-manifest/src/lib.rs:71`), `smoke` on
`TargetQualificationBinding`
(`crates/eggpack-core/src/qualification.rs:63`), and `consumer_validators` on
the CI orchestration graph (`crates/eggpack-ci/src/lib.rs:2327-2328`), the last
keyed by `BTreeMap` so its object key order is lexical when present. A
`BTreeMap` serialized by `serde_json` always emits keys sorted, so those
documents are stable without an explicit sort.

**Unordered collections reaching output.** Every `HashMap` in the workspace was
checked against the serialization boundary; none reaches serialized output.

- `crates/eggpack-contract/src/lib.rs:11` imports `HashMap`. Its uses are
  name-collision bookkeeping (`:883`), a required/observed membership set
  (`:1155`, `:1167-1168`) queried only via `contains` (`:1171`, `:1182`), and
  the archive mapping comparison (`:1647-1655`). That last one is the only
  place a `HashMap` key set is iterated, and the keys are collected, sorted,
  and deduplicated first (`:1656-1663`), so emitted findings are order-stable.
- `crates/eggpack-manifest/src/lib.rs:7` uses only `HashSet`, for validation
  (`:188`, `:195`, `:205`, `:219`, `:263`, `:320`); none is serialized.
- `crates/eggpack-core/src/lib.rs:177`, `:507`, `:547` use `HashSet` for
  duplicate detection.
- `crates/eggpack-github/src/lib.rs` and `crates/eggpack-ci/src/lib.rs` contain
  no `HashMap` at all; both standardize on `BTreeMap`/`BTreeSet` for exactly
  this reason.
- `crates/eggpack-bootstrap/src/lib.rs:1401` uses `HashMap` in test scaffolding
  that never reaches a render.

Where a directory must be read, the code funnels through `BTreeMap` rather than
trusting `read_dir` order — see the finalized-root inventory at
`crates/eggpack-github/src/lib.rs:682` and `:1358`, and the provisioned Zig
listing, collected then sorted (`crates/eggpack-ci/src/lib.rs:681-689`).
`validate_build_artifact_dir` deliberately does not rely on directory order:
`read_dir` only rejects extras and counts
(`crates/eggpack-ci/src/lib.rs:2087-2105`), matching membership against the
already sorted handoff list.

The honest caveat: this is a property of the current source, not a lint.
Nothing mechanically prevents a future `HashMap` from entering a serialized
type. `serde_json` would then emit keys in random per-process order; the
document would still parse and still validate, and the only symptom would be an
unreproducible diff.

## Deterministic archives

`create_tar_gzip` is the only production archive writer
(`crates/eggpack-core/src/finalization.rs:417-454`), and it is explicit
rather than default-driven.

| Field | Value | Line |
| --- | --- | --- |
| Member order | contract declaration order, iterated as given | `crates/eggpack-core/src/finalization.rs:428`, sourced `:262-267` |
| Header format | GNU (`ustar ` magic) | `crates/eggpack-core/src/finalization.rs:432` |
| Entry type | `EntryType::Regular` | `crates/eggpack-core/src/finalization.rs:433` |
| `mode` | `0o755` | `crates/eggpack-core/src/finalization.rs:438` |
| `uid` / `gid` | `0` / `0` | `crates/eggpack-core/src/finalization.rs:439-440` |
| `mtime` | `0` | `crates/eggpack-core/src/finalization.rs:441` |
| `size` | measured from the candidate file | `crates/eggpack-core/src/finalization.rs:429`, `:437` |
| `uname` / `gname` | never set; zero-filled by `new_gnu` | `crates/eggpack-core/src/finalization.rs:432` |
| Builder mode | `HeaderMode::Deterministic` | `crates/eggpack-core/src/finalization.rs:427` |
| gzip `MTIME` | `0` via `GzBuilder::mtime` | `crates/eggpack-core/src/finalization.rs:424` |
| gzip OS byte | unset → `255` | `crates/eggpack-core/src/finalization.rs:423-425` |
| gzip FNAME / FCOMMENT | unset | `crates/eggpack-core/src/finalization.rs:423-425` |
| Compression level | `Compression::default()` | `crates/eggpack-core/src/finalization.rs:425` |

`tar::Header::new_gnu` already zeroes the block and calls `set_mtime(0)`; the
production code re-sets mtime, uid, gid, and mode explicitly
(`crates/eggpack-core/src/finalization.rs:437-442`) and calls `set_cksum` after
all mutations (`:442`), which a valid header requires. No
`set_username`/`set_grouping` call exists, so `uname`/`gname` stay NUL.
`HeaderMode::Deterministic` is belt and braces here: it governs headers filled
from filesystem metadata, and no header in this function is.

Two fields are *not* controlled, both inherited from dependency defaults rather
than Eggpack code:

- Compression level is `Compression::default()`. Deterministic for a given
  `flate2` version, but not a value Eggpack asserts.
- `flate2` resolves with default features, so the pure-Rust `miniz_oxide`
  backend is used (`crates/eggpack-core/Cargo.toml:23`; `cargo tree -p
  eggpack-core` confirms the `flate2 → miniz_oxide` path). Switching to the
  `zlib-rs` backend would change compressed bytes for identical input; the
  workspace does not enable it.

Member order is the archive's weakest link. `create_tar_gzip` iterates `members`
as supplied (`crates/eggpack-core/src/finalization.rs:428`), and that slice is
built by zipping the contract's declared members with staged candidates
(`:262-267`). Two contracts declaring the same members in different orders
produce different archive bytes. Deterministic, not canonical.

The claim is tested, not asserted:
`archive_is_explicit_deterministic_and_contains_exact_contract_members`
finalizes the same inputs into two roots and asserts both the manifests and the
raw `.tar.gz` bytes are equal
(`crates/eggpack-core/src/finalization.rs:726-735`). The bootstrap test-only
writer mirrors the same constants
(`crates/eggpack-bootstrap/src/lib.rs:1757-1778`).

## Byte-identical rendering and drift checking

`eggpack-ci` emits YAML by string concatenation into a `String`, not through a
YAML serializer (`crates/eggpack-ci/src/lib.rs:1660-1765`). That is a
determinism choice: there is no key-ordering policy to inherit, so the byte
sequence is a direct function of the validated plan and policy. Two
consequences: scalars are quoted through `serde_json::to_string`
(`yaml_scalar`, `crates/eggpack-ci/src/lib.rs:1767-1769`), so every interpolated
value is escaped and cannot break out of its YAML context;
and job order is `plan.targets` order, canonically sorted at construction
(`crates/eggpack-ci/src/lib.rs:195`). Triggers render in policy vector order
(`:1663-1668`) — policy-supplied and validated, not sorted.

Drift checking is what keeps the checked-in workflow honest. `check_github`
(`crates/eggpack-ci/src/lib.rs:1788-1817`) re-renders from plan and policy,
bounds the existing bytes (`MAX_WORKFLOW_BYTES`, `:1794`), normalizes both
sides, compares, and reports the first differing offset. The other entry points
have the same shape: `check_reusable_release_github` (`:3058-3059`) and
`check_release_github` (`:3818-3819`).

`normalize_newlines` (`crates/eggpack-ci/src/lib.rs:1818-1831`) performs one
transformation: `\r\n` → `\n`, byte by byte; everything else passes through. It
does not trim, collapse runs, or touch a lone `\r`. That narrowness is the
point — the only tolerated difference between the checked-in file and the
rendered bytes is the line-ending convention a repository's `.gitattributes`
may impose, and a lone `\r` or a trailing-whitespace edit is still drift.

`eggpack ci check` never writes: it reads, re-renders, and reports
(`crates/eggpack-cli/src/main.rs:358-425`), returning an error naming byte
counts and first difference. `eggpack ci generate` is the only writer, writes
only the explicit `--output` path, and rejects a symlink target
(`crates/eggpack-cli/src/main.rs:301-355`; `atomic_write` at
`crates/eggpack-cli/src/main.rs:269-270`).

Tests pin both directions: identical bytes and a CRLF-converted copy both report
a match while a one-token change to `contents: read` is drift
(`crates/eggpack-ci/src/lib.rs:4885-4904`); the five-target matrix renders and
checks with no drift (`:8971`); rendering twice is asserted equal
(`:4834-4835`).

## Round-trip stability

`ReleaseManifest::to_json` validates, clones, canonically sorts the clone, and
serializes (`crates/eggpack-manifest/src/lib.rs:239-255`). Sorting a *clone* is
what makes the guarantee hold regardless of how the value was built: a caller
who reverses `targets` or bundle entries gets the same bytes, which
`canonical_order_is_independent_of_input_order` proves by reversing both and
asserting equality (`crates/eggpack-manifest/src/lib.rs:702-721`). The doc
comment is precise about the claim's strength — "stable application
serialization, not signing-grade canonical JSON" (`:237-238`). There is no
number canonicalization, no Unicode normalization, and no signature, consistent
with the workspace's integrity-is-not-authenticity rule.

Round-trip is asserted directly: `direct_round_trip_stable` compares `to_json`
output against a hard-coded literal and re-encodes the decoded manifest to the
same bytes (`crates/eggpack-manifest/src/lib.rs:384-400`);
`bundle_pairing_round_trip` and `archive_relationship_round_trip` assert
structural round-trips (`:402-437`).

The same check runs at every boundary where a manifest crosses into consumer
space: the CLI re-encodes and fails if the round-trip is not byte-identical
before writing `release-manifest.json` beside the finalized root
(`crates/eggpack-cli/src/main.rs:981-999`, test at `:1678-1684`); staging does
the same for the manifest it writes
(`crates/eggpack-github/src/lib.rs:774-790`, `:1443-1459`);
`eggpack-github` re-sorts the payload clone on every `to_json`
(`crates/eggpack-github/src/lib.rs:246-247`); `CIPlan` validates canonical
ordering on parse (`crates/eggpack-ci/src/lib.rs:242-249`, ordering at `:309`)
and round-trips in tests (`:4837-4842`); the contract has a golden TOML
round-trip (`crates/eggpack-contract/src/lib.rs:2283-2291`).

## Limits: where determinism is not claimed

- **Timestamps.** Tar and gzip timestamps are zeroed by Eggpack, but nothing
  claims the *candidate bytes* are time-free — compiled artifacts carry whatever
  the toolchain embeds. Native Cargo and `CargoZigbuild` builds of the same
  source can differ in bytes, and the archives differ accordingly. The claim is
  scoped to archive *encoding* given identical member bytes.
- **External service ordering.** GitHub's asset listing order is not under
  Eggpack's control. Eggpack imposes its own order on the payload and the
  receipt
  (`crates/eggpack-github/src/lib.rs:247`, `:2194-2195`) but makes no claim
  about how GitHub presents them.
- **Receipts are evidence of a remote interaction, not reproducible artifacts.**
  `GitHubDraftReceiptV1` carries `github_release_id`, `created`, `uploaded`, and
  `reused` (`crates/eggpack-github/src/lib.rs:311-324`) — values that
  legitimately differ between runs against the same release.
- **Compression level and backend are defaults**, not asserted values
  (see the archive section).
- **Platform-dependent output.** Rendered workflow bytes and manifests are
  host-independent by construction, but `size` and `sha256` evidence is only as
  stable as the file described, and the archive member set is gated on
  host-matched qualification. Another host cannot reproduce the release, only
  the encoding.
- **Preserved order in contracts.** Reordering members or entries changes
  archive bytes and manifest member order. Deterministic, not canonical.
- **The no-`HashMap` property is unenforced.** It holds by inspection of current
  source, not by lint or test.
- **Deterministic is not correct.** A byte-identical render can still be a
  workflow that does the wrong thing. Drift checking proves the checked-in file
  matches the renderer; it does not prove the renderer is right. That separation
  is why the renderer is driven by a validated plan and policy rather than by
  hand-edited YAML.

## Verification (how a reviewer checks this)

Read-only:

- Re-derive the sort table: `sort_by|sort\(|sort_unstable|dedup` across
  `crates/*/src/*.rs`, matching each hit to a row.
- `grep -n 'HashMap' crates/*/src/*.rs` and confirm each hit is validation
  bookkeeping or is sorted before iteration.
- Confirm the archive constants at
  `crates/eggpack-core/src/finalization.rs:423-442`.
- Confirm `normalize_newlines` is still the byte-wise `\r\n` → `\n` transform
  (`crates/eggpack-ci/src/lib.rs:1818-1831`) and that the drift entry points
  stay read-only.

Executable:

- `cargo test -p eggpack-manifest --all-targets --locked` —
  `canonical_order_is_independent_of_input_order`, `direct_round_trip_stable`,
  `bundle_pairing_round_trip`, `archive_relationship_round_trip`.
- `cargo test -p eggpack-core --all-targets --locked \
  archive_is_explicit_deterministic`
  — finalizes twice into separate roots, then compares manifest and raw archive
  bytes.
- `cargo test -p eggpack-ci --all-targets --locked` —
  `drift_check_normalizes_only_crlf_and_detects_manual_edits`,
  `rendering_is_deterministic_parseable_read_only_and_hands_off_candidate`,
  `m006_eggsact_five_target_matrix_renders_and_checks_without_drift`.
- `cargo test -p eggpack-github --all-targets --locked` —
  `direct_payload_is_exact_and_deterministic`,
  `installers_deterministic_and_exact_tag_origin`.
- `cargo test -p eggpack-contract --all-targets --locked` —
  `deterministic_round_trip_golden`,
  `observations_reject_oversized_or_unsafe_inputs_and_sort_findings`.
- End to end: `eggpack ci generate` then `eggpack ci check` over a real policy
  and plan; the second must report `match`.

The manifest, core, ci, github, and contract suites were run during this review
and all passed. No source was modified.

## Related deep dives

- [overview.md](overview.md) — module map and the producer pipeline these
  guarantees sit inside.
- [validation-model.md](validation-model.md) — why determinism is enforced as
  fail-closed validation rather than convention.
- [core-finalization.md](core-finalization.md) — the finalizer that writes the
  archive and assembles the manifest.
- [manifest.md](manifest.md) — the canonical release-manifest document and its
  byte-level contract.
- [contract.md](contract.md) — layout authority, expansion order, and the
  inventories cited above.
- [ci-rendering.md](ci-rendering.md) — workflow rendering, pinning, and the
  shape/template inputs behind it.
- [github.md](github.md) — staging payloads, receipts, and what the remote
  service does not preserve.
- [testing-and-portability.md](testing-and-portability.md) — the cross-platform
  lanes that hold these byte-level claims honest.
