# Eggpack ReleaseManifest v1 consumer mapping

This document defines the Eggpack side of the consumer seam. It is an interface note, not a new wire format. ReleaseManifest v1 remains unchanged.

Reviewed external baseline: `eggstack/eggup@2cab1f97ef30fa347c2030da321462459672c521`. The API files reviewed were `crates/eggup-core/src/domain.rs` (`ProductId`, `ReleaseId`, `MemberId`, `IntegrityRequirement`, `ArtifactMember`, `ArtifactSet`, `InstallPlan`) and `crates/eggup-acquisition/src/lib.rs` (`AcquisitionRequest`, `FetchLimits`). The pinned checkout confirmed validated opaque identity constructors, a `Sha256([u8; 32])` integrity value, local-source plus relative-destination artifact members, non-empty sets, explicit installation roots, exact caller-selected HTTP(S) URLs, and byte/time limits.

## Authority and identity

`product_id` and `release_id` map to Eggup `ProductId` and `ReleaseId` as opaque strings. Consumer constructors may reject values against their own generic bounds; no version ordering or reinterpretation occurs. `source_revision` is producer evidence. The manifest does not select releases, origins, mirrors, roots, destinations, ownership, services, fallback, authenticity, or receipts.

The caller selects one exact canonical target triple. The adapter requires exactly one matching target record; it does not use aliases or nearest-target fallback. The origin and installation root are supplied by consumer/application policy.

## Artifact mapping

| Manifest form | Acquisition units | Eggup transaction projection |
|---|---|---|
| direct | one exact URL plus exact size and SHA-256 | one member; flat `install` is a default member identity and relative basename |
| bundle | one unit for each paired artifact | all paired members form one `ArtifactSet`; reject partial or mixed release input |
| archive | one archive acquisition with exact size and SHA-256 | retain each `source`, `install`, size, and digest; qualified consumer extraction is required before member local files and `InstallPlan` |

`size` is exact. It can be passed as an acquisition maximum, then the received file must be checked for exact size before digest verification and staging. SHA-256 is integrity evidence only. `install` is a flat target-local identity, not a path root or replacement authorization.

The archive extraction seam is deliberately undefined here: archive bytes → consumer-owned, qualified extraction → local member files → `ArtifactSet`/`InstallPlan`. Current Eggup APIs do not imply generic archive extraction.

## Fixtures

The `plans/closure/eggup-interoperability/fixtures/` files pair direct, bundle, and archive ReleaseManifest JSON with illustrative `projection-*.json` examples. Projection examples are test/documentation data, not a production schema. The contract fixtures remain under `crates/eggpack-contract/tests/fixtures/`.
