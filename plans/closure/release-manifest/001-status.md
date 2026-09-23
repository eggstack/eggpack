# Release Manifest Milestone 001 Closure — V1 Domain and Deterministic Serialization

Status: closed

Source plan: `plans/implementation/release-manifest/001-release-manifest-v1-domain.md`

Roadmap: `plans/subsystems/release-manifest-roadmap.md`

Reviewed baseline: `c5fd88f5a44a17104f00f93c9888b568272a01ea` (clean baseline after Contract M002 closure)

Implementation and closure commit: `b5df057a0ab8d30664aeccc26aa7944678626930`.

## Executive finding

`eggpack-manifest` now defines and validates bounded schema-v1 JSON evidence for one finalized release. The domain binds one product, release, and source revision to canonical target records, explicit direct/bundle/archive forms, and exact non-zero sizes with lowercase SHA-256 digests. Bundle artifact/install pairs and archive source/install/member-byte identities remain associated. Serialization sorts target and nested records deterministically. Parsing rejects unknown fields and unsupported versions and performs no I/O.

The crate is a leaf dependency with only Serde and serde_json. It contains no build, process, network, archive, Eggup, installation, publication, release-selection, or trust authority. Evidence references are bounded strings and do not assert trust. Checksums and sizes express integrity facts only.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Bounded ProductId, ReleaseId, and SourceRevision | `ReleaseManifest::validate`; per-field byte bounds, non-empty/control-free checks |
| Schema version and strict parsing | `from_json`; schema-v1 validation, `deny_unknown_fields`; unsupported-version and unknown-field tests |
| Canonical target and explicit layout identity | `TargetRecord` with tagged `ArtifactForm::{Direct, Bundle, Archive}`; duplicate canonical target rejection |
| Direct artifact and install identity | `ArtifactRecord` and explicit install field |
| Relationship-preserving bundle entries | `BundleRecord` groups one artifact with its install identity; bundle round-trip test |
| Relationship-preserving archive members | `ArchiveMemberRecord` groups normalized source, install identity, and `ByteEvidence`; archive round-trip test |
| Exact non-zero size and SHA-256 | Artifact/member validation requires non-zero size and 64 lowercase hexadecimal characters |
| Duplicate/case collision handling | Release artifact and install namespaces reject exact and ASCII-case collisions |
| Deterministic JSON | Stable compact serde representation; lexical sorting of targets, bundle entries, and archive members; exact direct JSON golden and order-independence test |
| Bounds and unsafe data rejection | 1 MiB JSON cap; target, nested record, evidence-reference and string limits; flat filename and normalized archive-path checks |
| Evidence references carry identity only | Optional bounded opaque strings; no validation or trust interpretation beyond bounds |
| No I/O or dependency leakage | Dependency tree has only serde/serde_json and transitives; implementation exposes parse/validate/serialize only |
| Consumer boundary and docs | Crate README, crate-level docs, root README; explicitly no acquisition/install/trust authority |

## Production implementation evidence

- Added workspace crate `crates/eggpack-manifest` with public domain types, typed errors, strict JSON parsing, validation, and deterministic serialization.
- Added six unit tests for direct/bundle/archive round trips, a byte-exact JSON golden, canonical ordering, unsupported schema/unknown fields, invalid digest/size, and collision/crossed-install negatives.
- Added crate README and updated root README, release-manifest roadmap, registry, and this source plan's status.
- No changes to `eggpack-contract` or ADR-0002 were required.

## Exact verification executed

All listed commands passed in the implementation worktree:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test -p eggpack-manifest --all-targets --all-features --locked` (6 tests)
- `cargo test --workspace --all-targets --all-features --locked` (45 tests)
- `cargo doc --workspace --no-deps --locked`
- `cargo tree -p eggpack-manifest --locked` (serde and serde_json only, plus transitives)
- `cargo package -p eggpack-manifest --locked --allow-dirty`
- `cargo +1.89.0 check --workspace --all-targets --locked`
- `cargo +1.89.0 test -p eggpack-manifest --all-targets --locked` (6 tests)
- `./scripts/check-local.sh`
- `git diff --check`

Hosted GitHub Actions CI run [`35860964277`](https://github.com/eggstack/eggpack/actions/runs/35860964277) passed for implementation commit `b5df057a0ab8d30664aeccc26aa7944678626930`: Linux stable passed format, check, workspace tests, Clippy, and docs; Linux Rust 1.89 passed check and workspace tests; macOS and Windows passed check and workspace tests. The workflow does not run package publication verification; the local `cargo package` check passed separately.

## Invariant, failure, and recovery review

- The manifest represents facts supplied by a producer after finalization; it does not compute or discover them.
- One top-level product/release/source identity applies to all targets and their artifacts, preventing mixed nested identity values.
- Entry relationships are structural fields, not independent name sets. All layout forms are tagged and mutually exclusive.
- Malformed, oversized, empty, overlong, control-bearing, path-bearing, duplicate, or unsupported data fails with a typed error. No fallback, retry, mutation, or I/O occurs.
- Determinism is application-level stable serialization, not a signing-grade canonical JSON promise.

## Compatibility, security, and documentation review

This is the first Eggpack manifest schema and carries an explicit version. The only production dependencies are Serde and serde_json. Parsing does not fetch referenced evidence or validate authenticity/provenance. No URLs, credentials, installed-state claims, or arbitrary logs are modeled. The schema remains independent of build and consumer deployment crates.

## Unresolved findings

No implementation finding blocks closure. JSON Schema export remains out of scope pending a real consumer. No signing-byte canonicalization is claimed.

## Roadmap disposition and dependency transitions

- Release Manifest M001 is closed.
- Release Manifest M002 final-artifact builder is ready to plan, consuming the now-fixed v1 domain and the already-closed conformance inventory interface.
- Build/Qualification M001 PackConfig/ReleasePlan is ready to plan; backend replaceability remains required, and the `dist` disposition C still authorizes no production backend.
- Bootstrap Installers M001 direct generator model is ready to plan; the bootstrap roadmap already allows local fake-release tests without a build engine.
- Eggup Interoperability M001 is unblocked to plan on the Eggpack side. Any Eggup repository change still requires a plan registered in Eggup; no cross-repository change is claimed here.
- CI orchestration remains blocked on the build-plan interface. Ecosystem adoption remains blocked on the required core producer capabilities.
- No corrective plan or ADR is required. ADR-0002 remains satisfied.

The registry and affected roadmaps record these transitions. The manifest builder remains a separate milestone; this closure does not claim generated artifacts or consumer adoption.


## Post-closure corrective registration

Subsequent review after this closure identified a validation-scope defect: install-name collision tracking in `ReleaseManifest::validate()` is manifest-global, while DistributionContract semantics and checked-in multi-target fixtures require install-name uniqueness to be target-local. The historical implementation/CI evidence above remains accurate for the code that closed M001; it is not rewritten to conceal the later finding.

Corrective plan: `plans/implementation/release-manifest/001a-cross-target-install-namespace-corrective.md`.

Corrective M001a has since closed. Its implementation, verification, and dependency transitions are recorded separately in `plans/closure/release-manifest/001a-status.md`; the historical M001 findings and conclusions above remain unchanged.
