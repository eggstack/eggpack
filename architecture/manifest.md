# `eggpack-manifest` — Deep Dive

`eggpack-manifest` is the schema-v1 **final-bytes evidence document** for one
finalized release: it binds one product / release / source revision to canonical
target records carrying the exact size and SHA-256 of every emitted artifact and
every required archive member. It is a pure leaf — `serde` plus `serde_json`, no
I/O — and it is also the only crate in this workspace published for third-party
consumption, so its public API is a compatibility surface.

Source of truth: `crates/eggpack-manifest/src/lib.rs` (1354 lines,
`#![forbid(unsafe_code)]` at `:1`, `#![deny(missing_docs)]` at `:2`). Citations
below are to that file unless another path is named.

## Responsibility and the evidence-only rule

The manifest describes the final bytes of a finished release and nothing else.
The workspace keeps three kinds of release statement strictly apart:

| Statement | Meaning | Owner |
| --- | --- | --- |
| `ReleasePlan` | intent: what was requested to be built | `eggpack-core` |
| `ReleaseManifest` | final-bytes evidence: what was produced and hashed | `eggpack-manifest` |
| Eggup install receipts | installed state: what a machine actually has | `eggstack/eggup` (separate repository) |

The separation is structural, not documentary. Every wire type here is an
`ArtifactRecord` or a `ByteEvidence` — a name, a `u64` size, and a 64-character
digest (`:113-120`, `:148-153`). `TargetRecord` carries only a canonical triple
and a form (`:78-83`); no field anywhere holds a plan, a qualification result, a
builder identity, a signature, or an install outcome. The one advisory-looking
field, `evidence_references`, is documented as making no trust claim (`:70-72`).
So a manifest entry is a claim about bytes that finalization hashed — never a
claim about intent, qualification, or installation (crate doc `:4`: this crate
"does not access files, networks, or trust authorities").

## Key types / functions (with `file:line`)

| Item | Line | Description |
| --- | --- | --- |
| `SCHEMA_V1: u32 = 1` | `:10` | The only supported schema version. |
| `MAX_DOCUMENT_BYTES: usize = 1_048_576` | `:12` | Input JSON ceiling (1 MiB), checked before parsing. |
| `MAX_TARGETS: usize = 256` | `:14` | Ceiling on the `targets` vector. |
| `MAX_RECORDS: usize = 256` | `:16` | Ceiling on any one bundle-entry or archive-member list. |
| `MAX_EVIDENCE_REFERENCES: usize = 64` | `:18` | Ceiling on `evidence_references`. |
| `ManifestError` | `:25-34` | `#[non_exhaustive]`: `Invalid(String)`, `UnsupportedVersion(u32)`, `TooLarge`. `Display` `:36-44`; `Error` `:45`. |
| `ReleaseManifest` | `:59-73` | Top-level document: identity block, `targets`, optional `evidence_references`. |
| `TargetRecord` | `:78-83` | `{ target, form }` for one canonical triple. |
| `ArtifactForm` | `:88-108` | Internally tagged `kind`: `Direct{artifact,install}`, `Bundle{entries}`, `Archive{artifact,members}`. |
| `ArtifactRecord` | `:113-120` | `{ name, size, sha256 }` for one emitted artifact. |
| `BundleRecord` | `:125-130` | `{ artifact, install }` — keeps each bundle artifact paired with its install identity. |
| `ArchiveMemberRecord` | `:135-142` | `{ source, install, bytes }` — keeps member source path, install name, and member facts together. |
| `ByteEvidence` | `:148-153` | `{ size, sha256 }` — size and digest without a name. |
| `ReleaseManifest::target` | `:157-163` | Exact canonical-triple lookup; re-validates first; never resolves aliases or nearest-target. |
| `ReleaseManifest::from_json` | `:165-172` | Byte-length bound → `serde_json` parse → full `validate()`. |
| `ReleaseManifest::validate` | `:175-235` | Version, bounds, uniqueness, cross-record relationships. No I/O. |
| `ReleaseManifest::to_json` | `:239-255` | `validate()`, sort a **clone** canonically, compact serialization. |
| `ArtifactRecord::sha256_bytes` | `:260-262` | Decode validated lowercase hex into `[u8; 32]`. |
| `ByteEvidence::sha256_bytes` | `:283-285` | Same hex decode, without a name. |
| Private `validate` helpers | `:263-279`, `:286-301` | Non-zero size, 64-char lowercase hex, filename registered in a caller-supplied set. |
| `decode_sha256` (private) | `:303-319` | Strict lowercase-hex pair decoding; rejects uppercase and wrong lengths. |
| `valid_name` / `valid_path` (private) | `:320-336` / `:337-347` | Flat-name safety + ASCII-case-folded uniqueness; relative-path safety for member `source`. |
| Tests | `:349-1354` | 26 inline tests, including `projection_fixture_consistency` (`:723-1353`). |

## Schema details and bounds

Wire shape, as asserted byte-for-byte by `direct_round_trip_stable` (`:384-400`):

```json
{"schema_version":1,"product_id":"eggsact","release_id":"1.0.0",
 "source_revision":"…","targets":[{"target":"x86_64-unknown-linux-gnu",
 "form":{"kind":"direct","artifact":{"name":"eggsact","size":3,
 "sha256":"…"},"install":"eggsact"}}]}
```

- **Identity block** (`:61-67`): `schema_version` must equal exactly `1`, else
  `UnsupportedVersion` (`:176-178`). `product_id` and `release_id` are opaque
  stable identifiers, `source_revision` the immutable source revision; none is
  interpreted, ordered, or parsed here.
- **`targets`** (`:69`): 1 to `MAX_TARGETS` (`:182-184`), each triple unique
  (`:192-194`).
- **`evidence_references`** (`:71-72`): `#[serde(default, skip_serializing_if =
  "Vec::is_empty")]`, so genuinely optional on the wire — the checked-in fixtures
  omit it entirely. Bounded list of bounded, non-trusting strings (`:185-187`,
  `:231-233`).
- **Form** (`:87`): `#[serde(tag = "kind", rename_all = "snake_case")]` — the
  discriminator is a nested `"kind"` object, not a sibling field.
- **Bundle entries** (`:99`): 1 to `MAX_RECORDS` (`:202-204`), each keeping
  `artifact` + `install` adjacent; duplicate artifact name rejected (`:209-211`).
- **Archive members** (`:106`): 1 to `MAX_RECORDS` (`:216-218`); duplicate
  `source` rejected (`:224-226`); `source` must be a normalized relative path
  (`:337-347`).
- **Strictness**: every wire struct carries `deny_unknown_fields` — `:58`, `:77`,
  `:87`, `:112`, `:124`, `:134`, `:147`; `evidence_references` is the only
  defaulted field (`:439-455`).

### Bounds

| Constant | Value | Enforced at | What it protects |
| --- | --- | --- | --- |
| `MAX_DOCUMENT_BYTES` | 1 MiB | `:166-168` | Memory and parse cost of one untrusted document; returns `TooLarge` before any parse work. |
| `MAX_TARGETS` | 256 | `:182-184` | Target-vector size and the duplicate-target scan. |
| `MAX_RECORDS` | 256 | `:202-204`, `:216-218` | Nested bundle-entry and archive-member lists. |
| `MAX_EVIDENCE_REFERENCES` | 64 | `:185-187` | Length of the advisory reference list. |
| `MAX_ID` / `MAX_REVISION` (private) | 128 B | `:179-181`, `:191` | `product_id`, `release_id`, `target`, `source_revision`. |
| `MAX_NAME` (private) | 255 B | `:321` | Every artifact filename and install name. |
| `MAX_EVIDENCE` (private) | 256 B | `:232` | Each evidence-reference string. |
| literal `1024` | 1024 B | `:338` | Archive member `source` path (inline, not a named constant). |
| `size != 0`; `sha256` shape | 64 lowercase hex | `:265-277`, `:287-299` | Rules out empty artifacts/members; one fixed, unambiguous digest encoding. |

Bounded input matters most here because this is the only published crate: a third
party may call `from_json` on a file or a response it did not produce, in a
short-lived CLI or a service. Every bound is checked before the corresponding
allocation can grow unbounded — length first, then parse, then structure. Because
the four string bounds are private, a consumer can read the four public `MAX_*`
constants but must take per-field string limits from the documentation.

## `ByteEvidence`: integrity, not authenticity

A validated `(size, sha256)` pair asserts exactly this: *a byte sequence of
exactly this length, whose SHA-256 is this value, is the artifact named by the
surrounding name/install fields.* It is a checkable equality against downloaded
bytes, and nothing more. The schema carries no signature, no attestation, no
transparency-log entry, no build provenance, no builder or platform claim beyond
the caller's chosen canonical triple, and no statement that the publisher is who
it claims to be. `sha256_bytes()` (`:260-262`, `:283-285`) decodes an
already-validated hex string; it does not authenticate it.

This limitation is deliberate and load-bearing: producer and verifier are
decoupled, and the verifier learns nothing about the producer. Downstream systems
wanting authenticity must add it outside this schema.

## Canonical ordering

`to_json` (`:239-255`) validates, then clones and sorts the clone, so the
caller's value is never mutated. Sort keys:

| Collection | Key | Line |
| --- | --- | --- |
| `targets` | `target` (byte-lexical) | `:242` |
| bundle `entries` | `(artifact.name, install)` | `:245-247` |
| archive `members` | `(source, install)` | `:248-250` |
| `ArtifactForm::Direct` | nothing to sort | `:251` |

Ordering is independent of the order records were discovered in, so two runs
over the same bytes produce the same document. Verified by
`canonical_order_is_independent_of_input_order` (`:702-721`), which reverses
bundle entries and asserts identical output; the checked-in fixtures are stored
canonically, since the interop test asserts `to_json() == fixture.trim()` (`:652`).

Three precise limits. The sort is `String` ordering — UTF-8 byte order, not locale
or Unicode collation. **`evidence_references` is not sorted**; it is emitted in
input order (the canonicalization serializes the clone as-is). And the
document-size guarantee is closed at validation, not at serialization:
`validate` ends with `check_document_bound`, which measures the canonical
encoding and returns `TooLarge` past `MAX_DOCUMENT_BYTES`. `to_json` validates
first, so it cannot emit a document its own `from_json` would refuse — the
structural bounds now *imply* the document bound rather than merely approximating
it. The method documents itself as "stable application serialization, not
signing-grade canonical JSON" (`:238`).

## Collision rules: global vs target-local

The asymmetry is the point, and it is enforced by *where the set is created*, not
by a flag:

| Name space | Set created | Scope | Effect |
| --- | --- | --- | --- |
| Release artifact filename | `release_names`, once per `validate()` — `:189` | **manifest-global** | Two targets may not claim the same release filename, even across targets or forms. Threaded in at `:198`, `:207`, `:215`. |
| Install name | `installs`, once **per target** — `:195` | **target-local** | The same install name in two targets is fine; a collision inside one target is rejected. Used at `:199`, `:208`, `:222`. |

Both sets receive `name.to_ascii_lowercase()` (`:330`), so uniqueness is
ASCII-case-insensitive: `App` and `app` collide. Tests:
`release_artifact_collisions_remain_manifest_global` (`:603-623`),
`install_collisions_remain_rejected_within_target` (`:560-601`), the positive
target-local cases at `:486-558` (same install name across two targets, distinct
artifact filenames), and `rejects_case_collision_and_crossed_install`
(`:457-484`) for both halves in one target.

Why the asymmetry is correct: a release filename identifies a *downloadable object
in a release*, so two targets claiming one filename makes the release ambiguous
for any consumer fetching by name. An install name identifies a *position inside
one target's install root*, so two targets can legitimately name the same binary
`eggsact` while shipping distinct bytes. One subtlety: target, bundle-artifact,
and archive-member duplicates are checked by exact byte comparison (`:192`,
`:209`, `:224`), while names and install names are compared ASCII-case-folded.

## The install-name projection

The projection relates a manifest's three asset forms to a consumer-shaped view of
install names. **It is not on the wire.** No projection type appears in any
serialized struct; the projection types exist only inside the `#[cfg(test)] mod
projection_fixture_consistency` harness (`:723-1353`), as test-local
`#[serde(deny_unknown_fields)]` structs — `ReleaseIdentity` (`:760-765`), the
direct/bundle `*AcquisitionUnit` pairs (`:767-794`), `DirectProjection`
(`:777-784`), `BundleProjection` (`:796-803`), `ArchiveAcquisitionUnit`
(`:805-811`), `ArchiveProjectedMember` (`:813-820`), and `ArchiveProjection`
(`:822-830`). The module doc states the intent directly: "Projection fixtures
are explicitly documentation/test evidence, not a production wire format"
(`:726-727`). The same holds for `ReleaseManifest::target` and `sha256_bytes`:
`exact_target_and_sha_helpers_are_non_wire_additions` (`:625-637`) asserts the
serialized output is byte-identical before and after using them.

What is projected: **direct** yields exactly one acquisition unit (`:866-871`)
whose `member_id` and `relative_destination` both equal the manifest `install`
(`:891-902`), with `transaction_group == "one-artifact-set"` (`:903-908`).
**Bundle** yields one unit per paired entry, checked **bidirectionally** — counts
match (`:942-948`), every manifest entry finds a matching unit (`:949-963`), and
every unit finds a matching manifest entry (`:964-978`). **Archive** yields
exactly one acquisition unit for the archive artifact (`:1006-1012`) plus
per-member projection, again bidirectional (`:1031-1065`).

`extraction_required` (`:1066-1070`) is the archive-specific fact: an archive
artifact is not itself an installable file, so a consumer must extract the
required members before constructing its artifact set. The check demands
`extraction_required == true` and a `transaction_group` containing
`consumer-owned` (`:1071-1076`); the fixture value is
`one-artifact-set-after-consumer-owned-extraction`.

The negative fixtures prove the relation is exact, not merely plausible — each
mutation still parses as JSON and is then rejected: substituting `eggsact` for
`codegg-helper` (`:1124-1146`), dropping the `codegg-manifest` entry
(`:1149-1168`), an unrelated fourth unit (`:1171-1190`), crossed unit
destinations and digests (`:1193-1230`), a duplicated unit (`:1233-1247`), the
wrong selected target (`:1250-1260`), crossed archive member relationships
(`:1263-1290`), dropped `extraction_required` (`:1293-1303`), and mutated size
(`:1306-1319`) or digest (`:1320-1333`).
`wrong_target_projection_fixture_is_negative_evidence` (`:1337-1352`) pins the
wrong-target case: no exact match, no alias or nearest-target fallback.

## Round-trip and consumer contract

Parse/serialize stability is asserted in both directions: `to_json` output is
compared against a literal expected string (`:390-395`) and re-parsed to prove
idempotence (`:396-399`), while bundle and archive round-trips assert
`from_json(to_json(m)) == m` so paired relationships survive serialization
(`:402-419`, `:421-437`).

`eggup_interoperability_manifest_fixtures_are_valid_v1` (`:639-700`) is the
compatibility baseline. It parses the three checked-in fixtures from
`plans/closure/eggup-interoperability/fixtures/`, asserts each is already
canonical (`to_json() == fixture.trim()`, `:652`), and asserts that
`unknown-schema.json`, `corrupt-digest.json`, and `corrupt-size.json` are
rejected (`:654-665`). It also pins consumer-visible facts: the wrong triple
`x86_64-pc-windows-gnu` is an error (`:670`), identity is `eggsact` / `1.2.6`
(`:671-672`), the bundle has 3 entries (`:678`), the archive has the expected
size and 2 members with a specific source path (`:686-690`), and the archive
projection declares `extraction_required`, 1 unit, 2 members (`:693-699`).

A consumer therefore relies on: strict rejection of unknown fields; exact
`schema_version` gating; exact target resolution with no fallback; stable,
input-order-independent bytes; and a pinned fixture set. There are no in-crate
fixture files — tests `include_str!` fixtures from
`plans/closure/eggup-interoperability/fixtures/`, coupling this crate's suite to
those checked-in bytes.

## Publication status and the compatibility surface

`eggpack-manifest 0.1.0` is published. Per `plans/registry.md:249` and
`plans/registry.md:98`, it was published to crates.io from `8d661e4`, checksum
`2a08f24b…b629`, tag `eggpack-manifest-v0.1.0`, hosted run `37064833069` green;
the published `src/lib.rs` is byte-identical to the consumer-qualified pin
`678bbf04`, and an external registry-only consumer resolved exact `=0.1.0` with
no Git or path source. The same entry records that only `eggpack-manifest` was
published and the other six crates remain unpublished, and
`plans/registry.md:270` records that `eggup-eggpack 0.1.2` consumes registry
`eggpack-manifest =0.1.0`.

One clarification against a natural assumption: `homepage`, `documentation`,
`keywords`, and `categories` are **not** unique to this crate —
`crates/eggpack-contract/Cargo.toml:8-14` carries the same fields. The
distinguishing fact is the recorded publication, not the metadata; the other five
crates (`core`, `bootstrap`, `ci`, `github`, `cli`) carry no such metadata.

The consequence is that this crate's public API is a compatibility surface for a
downstream consumer in another repository, so changes to serialized field names,
bounds, collision rules, or ordering are breaking for that consumer. Two
change-control signals are already present: `#[non_exhaustive]` on
`ManifestError` (`:26`) and the private string bounds (`:19-22`).

## Boundaries / non-goals

| Not done here | Evidence |
| --- | --- |
| No filesystem, network, or process access | Crate doc `:4`; dependencies are `serde` + `serde_json` only (`crates/eggpack-manifest/Cargo.toml:19-21`). |
| No hashing | The only digest code is hex *decoding* (`decode_sha256` `:303-319`) and format checks (`:268-277`, `:290-299`). No hash function and no `sha2` dependency; digests are computed upstream and consumed here. |
| No build, compile, or archive extraction | No toolchain, process, or archive dependency; no code path outside parse/validate/sort/serialize. |
| No release selection or target aliasing | `target()` is exact match only (`:157-163`), pinned by `:1337-1352`. |
| No origin, mirror, or URL fields | No such field in any wire struct (`:59-153`). |
| No authenticity, signature, or provenance | See `ByteEvidence` above. |
| No installed-state modeling | Owned by Eggup; see [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md). |
| `unsafe` | Forbidden crate-wide (`:1`). |

## Dependencies / dependents

Leaf: `serde` (derive) + `serde_json` (`crates/eggpack-manifest/Cargo.toml:19-21`).
It depends on no other workspace crate and no `eggpack-contract` type, which is
what keeps it usable by an external consumer without pulling in the layout
authority.

| Dependent | Kind | Location |
| --- | --- | --- |
| `eggpack-core` | normal | `crates/eggpack-core/Cargo.toml:16`; builds manifests (`crates/eggpack-core/src/finalization.rs:58`, `crates/eggpack-core/src/lib.rs:16`) |
| `eggpack-bootstrap` | normal | `crates/eggpack-bootstrap/Cargo.toml:16` |
| `eggpack-github` | normal | `crates/eggpack-github/Cargo.toml:16` |
| `eggpack-cli` | normal | `crates/eggpack-cli/Cargo.toml:22` |
| `eggpack-ci` | dev | `crates/eggpack-ci/Cargo.toml` `[dev-dependencies]` |
| `eggup-eggpack` | external | `plans/registry.md:270` |

Direction invariant: the manifest never depends on `core`, `contract`,
`bootstrap`, `ci`, or Eggup.

## Related deep dives

- [overview.md](overview.md) — module map and pipeline placement.
- [contract.md](contract.md) — the layout authority this crate records facts about.
- [core-finalization.md](core-finalization.md) — where manifest evidence is produced.
- [validation-model.md](validation-model.md) — workspace-wide fail-closed validation.
- [determinism.md](determinism.md) — canonical ordering and stable serialization.
- [testing-and-portability.md](testing-and-portability.md) — test and portability lanes.
- [bootstrap.md](bootstrap.md) — an in-workspace consumer of this schema.
- [github.md](github.md) — draft staging of finalized releases.
- [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md) — normative
  consumer mapping; an interface note, not a wire format.
