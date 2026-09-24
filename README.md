# eggpack

Developer-side release construction and distribution infrastructure for Eggstack.

Eggpack is intended to centralize portable release contracts, target/build/qualification planning, artifact composition, release manifests, bootstrap installers, and generated release CI. It deliberately does **not** replace Eggup: Eggup remains the consumer-side verified installation/update/rollback layer.

The `eggpack-contract` crate implements the portable
DistributionContract schema-v1 authority used to describe target and artifact
names across Eggpack tooling. It is synchronous, side-effect free, and does not
build or publish releases. See [the crate README](crates/eggpack-contract/README.md)
for the schema and boundaries.

The `eggpack-manifest` crate defines bounded schema-v1 JSON evidence for
finalized release artifacts and members. It is a synchronous leaf parser and
serializer; see [its README](crates/eggpack-manifest/README.md) for the format
and consumer boundary.

Planning and architecture references:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/registry.md`

The contract implementation preserves the qualified unpublished `eggup-dist`
schema-v1 predecessor. Contract conformance validators are closed, and the
ReleaseManifest v1 domain is implemented.

`eggpack-core` owns pure PackConfig/ReleasePlan resolution and producer-side
manifest construction from explicitly named finalized files.
`eggpack-bootstrap` renders release-specific direct first-install shell and
PowerShell scripts from the contract and manifest; it does not select releases
or update existing installations. SHA-256 checks establish integrity, not
authenticity.
