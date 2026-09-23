# Release Manifest Milestone 001 — V1 Domain and Deterministic Serialization

Status: closed

Repository baseline: `c5fd88f` (Contract M002 formally closed; implementation started from the clean planning baseline)

Closure record: `plans/closure/release-manifest/001-status.md`

Source roadmap: `plans/subsystems/release-manifest-roadmap.md`

Long-term requirements:

- `plans/000-long-term-specification.md#12-concrete-release-manifest`
- `plans/001-terminology-and-domain-model.md#11-release-manifest`

Applicable ADR: `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: invariant / infrastructure

## 1. Objective

Define `eggpack-manifest` schema v1 as a bounded, deterministic description of one finalized release's accepted bytes and evidence identity. It identifies one product, release, and source revision; records canonical-target artifact/member identity, layout, exact size, and digest; and may carry bounded evidence references without becoming build, selection, hosting, installation, provenance-verification, or trust authority.

## 2. Readiness and dependencies

Hard dependency: Contract M002 is closed at `82f799f3d971b2999ac14c2d8fc1b965370e0f58`. Its finalized code surface at `a36803a7c34cc5b273559520bf99cb2400cc183a` exposes stable expected-file derivation, bounded release/member inventories, canonical target mapping observations, and deterministic conformance reports over direct, bundle, and archive layouts. The manifest schema can consume those identities without depending on a future build engine.

The manifest remains an independent leaf format. It must not require Eggup or producer build/CI crates to parse.

## 3. Current evidence

Contract M001 and M002 are closed. The contract supplies canonical target/layout identity, expected release filenames, archive-member mappings, and deterministic conformance. M002 also corrected the predecessor bundle-mapping blind spot, so manifest layout identity must preserve per-entry relationships rather than treating names as independent sets. No manifest crate or format exists in Eggpack. The closed `dist` 0.33 spike is design prior art only and does not select a format or backend.

## 4. Invariants

- A manifest describes finalized accepted bytes only and identifies exactly one product, release, and source revision.
- Entries use canonical target identity; layout form remains direct, bundle, or archive without ambiguity.
- Direct/bundle/archive relationships are explicit: bundle entries preserve asset/install association; archive entries preserve archive identity plus required member source/install identity.
- Every emitted artifact and logical member represented by v1 carries exact size and SHA-256 digest evidence.
- Evidence references, if present, are bounded identifiers only; they do not assert provenance validity or publisher authenticity.
- Sizes and digests are exact values; integrity is not described as authenticity.
- Serialization and entry ordering are deterministic.
- Inputs, strings, and collections are bounded; unknown schema majors and fields fail closed.
- The manifest contains no URLs, credentials, hosting authority, installed-state claims, or arbitrary CI logs.
- Parsing and validation are synchronous and do not access the network or filesystem.

## 5. Scope

### In scope

- Leaf crate `eggpack-manifest`;
- bounded product/release/source-revision identity and schema-v1 manifest domain;
- direct, bundle, and archive artifact/member representation with canonical target identity and explicit per-entry relationships;
- bounded optional qualification/provenance evidence references that carry identity only, not trust conclusions;
- deterministic JSON serialization and strict parsing;
- structural/semantic validation, ordering, and representative fixtures;
- compatibility and dependency qualification.

### Out of scope

- Computing artifact sizes or digests;
- constructing manifests from build outputs (M002);
- signing, authenticity, provenance, or trust policy;
- release selection, artifact fetching, or hosting URLs;
- installation receipts or deployment behavior;
- JSON Schema export (M003, subject to consumer evidence).

## 6. Required production changes

### A. Domain model

Define bounded `ProductId`, `ReleaseId`, `SourceRevision`, schema version, canonical target entries, evidence-reference identifiers, and artifact-form structures. Preserve direct/bundle/archive identity and the relationship between contract-required names and final accepted records. Direct and bundle records carry safe flat release filenames, install identity where applicable, byte size, and exact SHA-256 digest. Archive records carry the final archive filename/size/digest plus required member source/install relationships, with exact member size/digest evidence. Keep bundle/member ordering deterministic and relationship-preserving.

### B. Validation

Require one product/release/source-revision identity, unique canonical target entries, unique artifact names under exact and ASCII-case comparison, unique archive member identities, valid layout cardinality/relationships, bounded evidence-reference values, and valid non-zero sizes/digest encoding. Reject absent, ambiguous, crossed, or inconsistent layout identity. Use `eggpack-contract` only if it remains a light leaf dependency; do not duplicate schema authority.

### C. Deterministic JSON

Choose and document the v1 JSON representation and canonical ordering. Parsing rejects unknown fields and unsupported schema versions. Serialization must be stable for equal values. Do not claim signing-grade canonical JSON semantics in this milestone.

### D. Fixtures and consumer boundary

Add direct, bundle, and archive round-trip goldens plus duplicate, collision, malformed digest/size, mixed identity, unknown schema, and unknown-field negatives. Document that a manifest records finalized release facts; it does not authorize acquisition or installation.

## 7. Ordered work packages

1. Freeze bounded v1 identity, artifact, digest, and layout domain against canonical documents/ADR.
2. Implement the leaf crate and strict parse/validate API.
3. Implement deterministic JSON serialization and stable ordering.
4. Add direct/bundle/archive fixtures and negative compatibility cases.
5. Review dependency direction and verify there is no build/network/deployment dependency.
6. Add README/rustdoc and package, MSRV, docs, and CI qualification.
7. Record closure evidence and update roadmap/registry; defer the manifest builder until this milestone closes.

## 8. Failure, restart, and contention semantics

Malformed or over-limit data returns typed errors. Parsing and serialization do not mutate caller data or perform I/O. No retry, restart, or contention behavior applies.

## 9. Compatibility and migration

This establishes the first Eggpack manifest schema. Schema version is explicit from its first release. Unsupported future major versions and unknown fields fail closed. Do not import `dist` output as canonical without a deliberate documented mapping. Trust/signing or ownership-boundary decisions require a separate ADR.

## 10. Required tests

- Direct, bundle, and archive round-trip and deterministic JSON goldens;
- canonical target identity and unambiguous artifact form;
- duplicate exact/ASCII-case names and target negatives;
- empty/overlong/control/path-bearing fields and count bounds;
- invalid digest, zero/invalid size, mixed product/release/source-revision identity, crossed bundle relationship, and archive member relationship negatives;
- unknown schema major and unknown-field rejection;
- unchanged Contract M001/M002 suites;
- no build, network, process, archive, Eggup, or install dependency leakage.

## 11. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-manifest --locked
cargo package -p eggpack-manifest --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Record hosted Linux, macOS, and Windows CI separately.

## 12. Documentation updates

Update the new crate README/rustdoc, release-manifest roadmap, registry, and closure record. No canonical architecture change is expected within ADR-0002.

## 13. Acceptance criteria

M001 closes when a strict bounded schema-v1 manifest represents finalized direct/bundle/archive bytes and logical members with ProductId + ReleaseId + SourceRevision, canonical target identity, explicit relationship-preserving layout semantics, exact size/SHA-256 evidence, deterministic JSON, stable parse/serialize behavior, complete negative fixtures, bounded non-trust-bearing evidence references, no trust/hosting/install authority, and package/MSRV/docs/hosted CI qualification.

## 14. Stop conditions

Stop for ADR review if signing requires a byte-canonical format commitment, manifest fields imply authenticity or release-selection authority, target/layout facts cannot remain unambiguous, or a leaf parser would acquire build/network/deployment dependencies.

## 15. Closure evidence required

Record implementation and reviewed baselines; schema/API inventory; layout and validation matrices; JSON goldens; bounds and deterministic-order evidence; dependency/package/MSRV/docs/hosted CI results; security and compatibility review; unresolved findings; and downstream dependency transitions.

## 16. Handoff notes

M001 supplies the schema consumed by Release Manifest M002 builder, Bootstrap Installers M001, Eggup interoperability M001, CI/build aggregation, and eventual provenance planning. The builder remains blocked until this schema milestone closes. Contract M002 is closed; the `dist` spike remains disposition C, and no backend is production-authorized.
