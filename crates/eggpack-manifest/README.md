# eggpack-manifest

`eggpack-manifest` defines schema-v1 JSON evidence for one finalized release. It binds one product, release, and source revision to canonical target records and exact artifact/member size and SHA-256 values. Direct, bundle, and archive forms are explicit; bundle artifact/install pairs and archive source/install/member-byte relationships stay together.

The crate is a synchronous leaf parser and serializer. It performs no filesystem or network access and does not build artifacts, select releases, authorize downloads, verify provenance, assert authenticity, or describe installed state. Evidence references are bounded identifiers only. SHA-256 and size express integrity facts, not trust.

Serialization sorts target triples, bundle entries, and archive members lexically. This creates stable application JSON, but does not claim signing-grade canonical JSON. Unknown fields and unsupported schema versions fail closed. Input is limited to 1 MiB; collections and strings have explicit bounds.

This crate also owns the SHA-256 hex acceptance rule for the whole workspace:
`validate_sha256_hex` accepts exactly 64 lowercase hexadecimal characters and
refuses the all-zeros placeholder, which no real byte sequence hashes to
(ADR-0006). `eggpack-core`, `eggpack-ci`, and `eggpack-github` apply that rule
rather than restating it. The JSON wire shape, the field set, and
`schema_version: 1` are unchanged by it.

# Consumer integration

Consumers can select an exact canonical target with `ReleaseManifest::target`
and decode validated SHA-256 evidence with `ArtifactRecord::sha256_bytes` or
`ByteEvidence::sha256_bytes`. These helpers do not change the JSON wire shape.
See
[the Eggup consumer mapping](https://github.com/eggstack/eggpack/blob/main/architecture/eggup-manifest-consumer-v1.md)
for the archive extraction boundary.
