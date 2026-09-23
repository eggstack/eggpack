# Release Manifest Milestone 001 — V1 Domain and Deterministic Serialization

Status: ready for handoff

Repository baseline: `a74009ac5726fb775bbfef0926dc763d81a9a706` (Contract M002 closed; expected-file and conformance interface available)

Source roadmap: `plans/subsystems/release-manifest-roadmap.md`

Long-term requirements:

- `plans/000-long-term-specification.md#12-concrete-release-manifest`
- `plans/001-terminology-and-domain-model.md#11-release-manifest`

Applicable ADR: `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: invariant / infrastructure

## 1. Objective

Define `eggpack-manifest` schema v1 as a bounded, deterministic description of one finalized release's bytes. It identifies one product and release and records canonical-target artifact identity, layout, size, and digest without becoming build, selection, hosting, installation, or trust authority.

## 2. Readiness and dependencies

Hard dependency: Contract M002 is closed. `eggpack-contract` exposes stable expected-file derivation and deterministic conformance reports over direct, bundle, and archive layouts. The manifest schema can consume canonical contract expansion and complete caller-supplied final inventory without depending on a future build engine.

The manifest remains an independent leaf format. It must not require Eggup or producer build/CI crates to parse.

## 3. Current evidence

Contract M001 and M002 are closed. The contract supplies target/layout identity and expected release filenames; M002 reports observed completeness. No manifest crate or format exists in Eggpack. The closed `dist` 0.33 spike is design prior art only and does not select a format or backend.

## 4. Invariants

- A manifest describes finalized bytes only and identifies exactly one product and release.
- Entries use canonical target identity; layout form remains direct, bundle, or archive without ambiguity.
- Sizes and digests are exact values; integrity is not described as authenticity.
- Serialization and entry ordering are deterministic.
- Inputs, strings, and collections are bounded; unknown schema majors and fields fail closed.
- The manifest contains no URLs, credentials, hosting authority, installed-state claims, or arbitrary CI logs.
- Parsing and validation are synchronous and do not access the network or filesystem.

## 5. Scope

### In scope

- Leaf crate `eggpack-manifest`;
- bounded product/release identity and schema-v1 manifest domain;
- direct, bundle, and archive artifact representation with canonical target identity;
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

Define bounded `ProductId`, `ReleaseId`, schema version, canonical target entries, and artifact-form structures. Preserve direct/bundle/archive identity and the relationship between contract-required artifact names and final observed artifact records. Each artifact record carries a safe flat file name, byte size, and exact SHA-256 digest representation.

### B. Validation

Require one product/release identity, unique canonical target entries, unique artifact names under exact and ASCII-case comparison, valid layout cardinality, bounded values, and valid digest encoding/length. Reject absent, ambiguous, or inconsistent layout identity. Use `eggpack-contract` only if it remains a light leaf dependency; do not duplicate schema authority.

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
- invalid digest, size, and mixed product/release identity negatives;
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

M001 closes when a strict bounded schema-v1 manifest represents finalized direct/bundle/archive bytes with canonical target identity, deterministic JSON, stable parse/serialize behavior, complete negative fixtures, no trust/hosting/install authority, and package/MSRV/docs/hosted CI qualification.

## 14. Stop conditions

Stop for ADR review if signing requires a byte-canonical format commitment, manifest fields imply authenticity or release-selection authority, target/layout facts cannot remain unambiguous, or a leaf parser would acquire build/network/deployment dependencies.

## 15. Closure evidence required

Record implementation and reviewed baselines; schema/API inventory; layout and validation matrices; JSON goldens; bounds and deterministic-order evidence; dependency/package/MSRV/docs/hosted CI results; security and compatibility review; unresolved findings; and downstream dependency transitions.

## 16. Handoff notes

M001 supplies the schema consumed by Release Manifest M002 builder, Bootstrap Installers M001, Eggup interoperability M001, CI/build aggregation, and eventual provenance planning. The builder remains blocked until this schema milestone closes. Contract M002 is closed; the `dist` spike remains disposition C, and no backend is production-authorized.
