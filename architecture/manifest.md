# `eggpack-manifest` — Deep Dive

Bounded `schema-v1` JSON evidence for **one finalized release**: binds one
`product_id` + `release_id` + `source_revision` to canonical target records with
exact artifact/member `size` + `SHA-256`. Explicit `direct` / `bundle` /
`archive` forms preserve pairing relationships.

Synchronous leaf parser/serializer (`crates/eggpack-manifest/src/lib.rs`,
`#![forbid(unsafe_code)]`); no FS, no network, no trust authorities. M001
domain/serialization closed, M001a namespace corrective closed, M002 builder
(in `eggpack-core`) closed.

## Key types / functions (`crates/eggpack-manifest/src/lib.rs`)

- `SCHEMA_V1 = 1` (`:10`); `MAX_DOCUMENT_BYTES = 1_048_576` (`:12`);
  `MAX_TARGETS = 256`, `MAX_RECORDS = 256`, `MAX_EVIDENCE_REFERENCES = 64`
  (`:14-18`); private `MAX_ID = 128`, `MAX_NAME = 255`, `MAX_REVISION = 128`,
  `MAX_EVIDENCE = 256`.
- `ManifestError::{Invalid(String), UnsupportedVersion(u32), TooLarge}`
  (`:27-34`, `#[non_exhaustive]`).
- `ReleaseManifest{schema_version, product_id, release_id, source_revision,
  targets, evidence_references}` (`:59-73`, `deny_unknown_fields`).
- `TargetRecord{target, form}` (`:78-83`); `target` is the canonical triple.
- `ArtifactForm::{Direct{artifact,install}, Bundle{entries},
  Archive{artifact,members}}` (`:88-108`, `#[serde(tag="kind")]`).
- `ArtifactRecord{name, size, sha256}` (`:113-120`),
  `BundleRecord{artifact, install}` (`:125-130`),
  `ArchiveMemberRecord{source, install, bytes}` (`:135-142`),
  `ByteEvidence{size, sha256}` (`:148-153`).
- `ReleaseManifest::target(canonical_triple)` (`:157-163`) — exact match only.
- `ReleaseManifest::from_json` (`:165-172`) — length check → parse → validate.
- `ReleaseManifest::validate` (`:175-235`) — version/bounds/uniqueness/relations.
- `ReleaseManifest::to_json` (`:239-255`) — validate + lexical sort
  (targets, bundle entries, archive members), compact output; stable app JSON,
  explicitly not signing-grade canonical JSON.
- `ArtifactRecord::sha256_bytes` (`:260-263`),
  `ByteEvidence::sha256_bytes` (`:283-285`).
- Tests inline (`:349-1354`): round-trip, negatives, multi-target namespace
  (M001a), `eggup_interoperability_manifest_fixtures_are_valid_v1`,
  `projection_fixture_consistency`.

## Schema details

Top-level: `{"schema_version":1,"product_id":…,"release_id":…,
"source_revision":…,"targets":[…],"evidence_references"?:[…]}`.

- `schema_version` exactly `1`, else `UnsupportedVersion`.
- `product_id`/`release_id`/`source_revision`: non-empty, ≤128B, no controls.
- `targets`: 1–256, unique canonical strings (≤128B).
- `evidence_references`: ≤64 identifiers, no trust claim.
- `ArtifactRecord.name`: safe flat (≤255B, rejects `./..///:\`), globally unique
  per manifest (exact + ASCII-case-folded). `size != 0`. `sha256` = 64 lowercase
  hex chars.
- `Bundle`: 1–256 entries, unique `artifact.name`, per-target unique `install`.
- `Archive`: 1–256 members; `source` normalized relative path (≤1024B); unique
  `source` per archive target; per-target unique `install`.
- **Namespace rule (M001a):** `install` uniqueness is target-local;
  `artifact.name` uniqueness is manifest-global (e.g. same install name on
  linux-gnu + macOS with distinct artifact filenames).
- Fail-closed: `deny_unknown_fields`, unknown schema/digest/size → error;
  `>1MiB → TooLarge`. No URLs/credentials/hosting/installed-state in schema.

No in-crate fixture files; tests `include_str!` the Eggup-interop fixtures under
`plans/closure/eggup-interoperability/fixtures/`
(`direct/bundle/archive-manifest.json` + `projection-*.json` doc/test data +
`unknown-schema/corrupt-digest/corrupt-size/wrong-target.json` negatives).

## Boundaries

Parses/validates/serializes bounded v1 evidence; exact target lookup; hex
decode; deterministic ordering. Does **not**: touch FS/network, build or
extract, compute sizes/digests, select releases, resolve origins/mirrors,
authorize downloads, assert provenance/authenticity, describe installed state
(Eggup-owned), embed secrets/logs, or do alias/nearest-target fallback.

## Dependencies / dependents

- Own deps (`Cargo.toml:19-21`): `serde` (derive), `serde_json` only.
- Dependents: `eggpack-core` (owns `ManifestBuilder`, M002),
  `eggpack-bootstrap`, `eggpack-ci`, `eggpack-cli`, `eggpack-github`.
  Direction invariant: manifest never depends on core/contract/bootstrap/CI/Eggup.

## Consumer contract (Eggup interop)

Normative mapping in [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md)
(interface note, not a new wire format):

- `product_id/release_id` opaque → Eggup `ProductId`/`ReleaseId`; no ordering.
- Caller selects exactly one canonical triple; zero/multiple → fail.
- `direct`: 1 acquisition unit → 1 member. `bundle`: 1 unit per paired entry →
  one `ArtifactSet`; reject partial/mixed. `archive`: 1 acquisition + retained
  per-member facts; qualified consumer extraction required before
  `ArtifactSet`/`InstallPlan` (extraction seam undefined).
- `size` exact (acquisition max, then exact check); SHA-256 integrity only;
  `install` flat target-local identity, not path/ownership authorization.
