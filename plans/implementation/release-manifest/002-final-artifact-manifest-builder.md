# Release Manifest Milestone 002 — Final Artifact Manifest Builder

Status: closed

Closure record: `plans/closure/release-manifest/002-status.md`

Repository baseline: `0b1c3b9795280dea610c2fbe9d1533591d610b3f` (Contract M001/M002 closed; Release Manifest M001/M001a closed; hosted CI green)

Source roadmap: `plans/subsystems/release-manifest-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#5.3-eggpack-core`
- `plans/000-long-term-specification.md#11-artifact-finalization-and-checksums`
- `plans/000-long-term-specification.md#12-concrete-release-manifest`
- `plans/001-terminology-and-domain-model.md#9-finalization`
- `plans/001-terminology-and-domain-model.md#11-release-manifest`

Applicable ADR: `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: capability / infrastructure

## 1. Objective

Create the initial producer-side `eggpack-core` crate and implement a deterministic ManifestBuilder that turns an explicitly supplied, contract-conformant set of finalized local files into `eggpack_manifest::ReleaseManifest` v1.

The builder owns producer evidence construction only. It validates exact contract identity, reads only caller-named local final files, computes exact non-zero byte sizes and SHA-256 digests, preserves direct/bundle/archive relationships, and emits the already-fixed schema-v1 domain. It does not discover releases, build/package artifacts, select versions, publish, fetch, extract archives, or authorize consumer installation.

## 2. Readiness and dependencies

Hard dependencies are satisfied:

- Contract M001/M002 are closed and provide target expansion, expected release-file derivation, release/member inventory validation, canonical target mapping, and deterministic conformance.
- Release Manifest M001 plus corrective M001a are closed and provide the schema-v1 output domain, deterministic JSON, target-local install namespaces, and manifest-global release-artifact namespace.
- The external `dist` spike is closed disposition C and contributes prior art only; this builder must be backend-independent.

This milestone is the first code owner for `eggpack-core`. Build/Qualification M001 is separately ready but, within this batch, should extend the crate after this milestone establishes its package/dependency skeleton rather than independently creating a competing core crate.

## 3. Current evidence

Current workspace production crates are:

- `eggpack-contract`;
- `eggpack-manifest`.

No producer core crate exists yet. `eggpack-manifest` intentionally performs no filesystem I/O and must remain a leaf parser/serializer. Manifest production therefore belongs in `eggpack-core`, matching the canonical layer model.

Representative contract fixtures already exist for:

- Eggsact-style multi-target direct releases;
- CodeGG-style logical bundles;
- Egress-style archives with required member mappings.

## 4. Invariants

- `eggpack-manifest` remains filesystem/network/build independent.
- ManifestBuilder accepts one ProductId + ReleaseId + SourceRevision and never infers release identity from filenames, tags, Git, or directories.
- Canonical artifact/install/member identity comes from DistributionContract expansion; caller data cannot redefine it.
- Target aliases, if accepted at the input seam, are immediately resolved to canonical target triples; output contains canonical triples only.
- Release artifact filenames are exact contract-expanded names and remain manifest-global unique.
- Install names are target-local, preserving M001a.
- The builder reads only explicitly supplied paths; no directory scanning, globbing, release discovery, or network I/O.
- Hash/size evidence describes the exact final files supplied to the builder.
- SHA-256 is integrity evidence only.
- A complete manifest cannot be produced from missing, duplicate, extra-under-Exact, wrong-name, zero-size, or unreadable required artifacts.
- Archive member relationships are preserved exactly. This milestone does not open/extract an archive; member byte evidence is computed from explicitly supplied finalized member-source files produced by the caller/finalizer.
- Sidecar presence participates in release-inventory conformance, but this milestone does not assign authenticity to sidecars or implement a generic sidecar-content language.
- No subprocess, publication, Git, service, Eggup, or build-backend authority is introduced.

## 5. Scope

### In scope

- new workspace crate `crates/eggpack-core`;
- minimal producer-core dependency direction and README/rustdoc boundary;
- typed manifest-builder input for one resolved release;
- explicit final release-file paths and, for archives, explicit finalized member-source paths;
- exact contract/conformance validation before manifest construction;
- file metadata/read hashing with SHA-256;
- deterministic construction of direct/bundle/archive manifest records;
- bounded evidence-reference passthrough;
- typed diagnostics/errors that do not leak arbitrary file contents;
- representative direct/bundle/archive integration fixtures;
- package/MSRV/docs/cross-platform CI qualification.

### Out of scope

- build invocation or builder adapters;
- PackConfig/ReleasePlan (Build/Qualification M001);
- archive assembly or archive extraction;
- sidecar generation;
- code signing/notarization;
- qualification execution;
- release selection or Git tag inspection;
- network acquisition or publication;
- CLI commands;
- trust/signature/provenance verification;
- schema-v1 changes.

## 6. Required production changes

### A. Establish `eggpack-core`

Add `crates/eggpack-core` to the Rust 1.89 workspace.

Initial dependencies SHOULD remain small:

- `eggpack-contract` by path/version;
- `eggpack-manifest` by path/version;
- a maintained RustCrypto SHA-256 implementation (normally `sha2`);
- Serde only if required for producer-core types; do not add async/network/process/archive crates.

The crate MUST forbid unsafe code and deny missing docs, consistent with the existing workspace.

### B. Explicit finalized-input model

Define bounded producer-side types equivalent in purpose to:

- `FinalizedReleaseInput` — ProductId, ReleaseId, SourceRevision, selected finalized target inputs, bounded evidence references;
- `FinalizedTargetInput` — target-or-canonical identity plus explicit layout-shaped paths;
- `FinalizedDirectInput`;
- `FinalizedBundleInput`;
- `FinalizedArchiveInput`;
- `FinalizedArchiveMemberInput`.

Names are illustrative; preserve the semantic boundary rather than these exact Rust identifiers.

The input MUST require explicit local paths. Do not derive artifact identity from path names alone. The builder compares supplied logical file/member labels against contract expansion and rejects mismatch.

For release inventory conformance, each target input MUST account for all contract-required release files, including checksum sidecars. The builder may accept an explicit list/map of release-file paths keyed by expected contract labels. It MUST NOT scan the parent directory.

### C. Contract/conformance gate

For every selected target:

1. resolve through DistributionContract;
2. expand with the caller-supplied opaque ReleaseId/version;
3. derive `expected_release_files`;
4. construct a ReleaseInventory from the explicitly supplied release filenames;
5. validate with `ExtrasPolicy::Exact` for the builder input;
6. validate layout/member mapping against contract expansion;
7. for archive form, construct/validate the explicit member inventory before hashing;
8. only after conformance succeeds, read files and produce byte evidence.

Do not duplicate template/path/collision rules in core.

### D. Final byte evidence

For every manifest artifact record:

- require a regular file;
- reject symlinks and non-regular filesystem objects;
- compute exact non-zero byte size;
- stream SHA-256 rather than loading arbitrarily large files in memory;
- encode lowercase hexadecimal digest required by Manifest v1;
- surface bounded path-independent diagnostics where practical.

For archive members, compute size/digest from explicit caller-supplied member-source files. Record clearly that M002 validates the evidence inputs and relationship but does not independently prove that the final archive contains those exact member bytes; Phase 5 archive assembly/finalization will later make that guarantee by construction.

### E. Manifest construction

Construct the existing `ReleaseManifest` domain without bypassing its `validate()`/deterministic serialization rules.

The builder MUST:

- output canonical target triples;
- preserve direct artifact/install relationship;
- preserve each bundle artifact/install pair;
- preserve archive artifact plus source/install/member-byte relationships;
- pass through only bounded evidence-reference identifiers;
- reject duplicate canonical targets and mixed release/source identity before output;
- return a validated manifest value, not serialized bytes as the only API.

### F. Failure semantics

The builder is synchronous and side-effect-free except for reading caller-named files.

On any validation/read/hash failure:

- return an error;
- emit no partial manifest as success;
- modify no input files;
- create no public/release state;
- perform no cleanup outside resources it created (normally none).

## 7. Ordered work packages

1. Create `eggpack-core` package skeleton, docs, and dependency guard.
2. Define bounded finalized-input and builder error types.
3. Implement contract/conformance validation over explicit target/file/member inputs.
4. Implement regular-file size/SHA-256 evidence computation.
5. Implement direct manifest construction.
6. Implement bundle construction with relationship preservation.
7. Implement archive construction using explicit member-source evidence without archive opening.
8. Add multi-target deterministic aggregation and mixed/duplicate identity guards.
9. Add direct/CodeGG/Egress positive fixtures and negative matrices.
10. Qualify package/MSRV/docs/dependency direction and hosted CI.
11. Write closure evidence and update registry/roadmaps; only then consider Release Manifest Phase 3 complete.

## 8. Failure, restart, and contention semantics

There is no shared mutable service state and no retry loop. Re-running with unchanged explicit files/contract/input produces the same manifest value/JSON. If a file changes between metadata and hashing, the implementation MUST either derive size from bytes actually hashed or re-stat and reject inconsistency; do not report a digest/size pair known to come from different file states.

No lock/TOCTOU guarantee over producer-owned files is claimed beyond this check. Phase 5 owner-private staging/finalization will later provide stronger immutability by construction.

## 9. Compatibility and migration

This milestone consumes Manifest schema v1; it does not change it.

`eggpack-core` becomes the canonical home for producer-side manifest construction. `eggpack-manifest` remains independently usable by runtime consumers with no dependency on core.

Do not import `dist` types, Eggup types, or application-specific release types into the builder API.

## 10. Required tests

At minimum:

- direct multi-target build from Eggsact-shaped final files;
- CodeGG-like bundle with exact per-entry relationships;
- Egress-like archive with explicit member byte evidence;
- deterministic output independent of caller target/input ordering;
- alias input canonicalizes correctly if aliases are accepted;
- unknown target/release-name mismatch fails;
- missing sidecar/release file fails Exact inventory validation;
- extra explicitly supplied release file fails Exact validation;
- missing/extra/wrong archive member fails;
- artifact filename and ASCII-case collision remains rejected;
- repeated install names across canonical targets remain accepted;
- within-target install collision remains rejected by manifest domain;
- zero-length final artifact/member rejected;
- symlink/directory/non-regular input rejected;
- unreadable/missing file returns typed failure and no manifest;
- file-change size/hash consistency behavior;
- evidence-reference bounds retained;
- unchanged contract and manifest suites;
- dependency/source scan proving no network/process/archive/Eggup dependency.

## 11. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Record hosted Linux stable, Linux Rust 1.89, macOS stable, and Windows stable separately.

## 12. Documentation updates

Update:

- root README crate/layer overview;
- `crates/eggpack-core/README.md`;
- rustdoc for all public producer-input/builder/error types;
- release-manifest roadmap;
- build/qualification roadmap if its batch-order note changes;
- registry;
- closure record `plans/closure/release-manifest/002-status.md`.

Document explicitly why `eggpack-manifest` remains leaf-only and why archive members are explicit finalization inputs rather than discovered by opening archives in this milestone.

## 13. Acceptance criteria

M002 closes when:

- `eggpack-core` exists with a clean producer-side dependency boundary;
- explicit finalized direct/bundle/archive inputs are contract-conformant before hashing;
- exact final file sizes/SHA-256 values are computed from caller-named regular files;
- valid multi-target releases produce deterministic Manifest v1 values/JSON;
- incomplete/mismatched/ambiguous inputs cannot produce a successful manifest;
- schema v1 remains unchanged;
- no build/network/process/archive-extraction/Eggup authority leaks into the implementation;
- stable/MSRV/package/docs/dependency and hosted CI checks pass;
- no unresolved medium-or-higher finding remains.

## 14. Stop conditions

Stop for ADR/replanning if:

- implementation requires changing Manifest schema v1;
- archive-member correctness cannot be represented without giving the builder generic archive extraction/format authority;
- a second target/artifact naming authority is needed outside DistributionContract;
- producer manifest construction requires Eggup runtime types;
- a backend-specific type must become part of the canonical builder API;
- signing/authenticity semantics are needed to define successful manifest construction.

## 15. Closure evidence required

Record:

- exact implementation/reviewed baselines;
- `eggpack-core` public API/dependency inventory;
- direct/bundle/archive input-to-manifest matrix;
- inventory/member-conformance evidence;
- regular-file/symlink/zero-size/hash behavior;
- deterministic ordering/JSON evidence;
- unchanged Manifest v1 compatibility evidence;
- package/MSRV/docs/dependency-tree results;
- hosted CI run;
- unresolved findings;
- dependency transitions, including whether Phase 3 may be considered complete.

## 16. Handoff notes

This is the first producer-core implementation in the next batch. Build/Qualification M001 should extend the resulting `eggpack-core` package rather than create a competing crate. Bootstrap Installers M001 and Eggup Interoperability M001 do not need to wait for this implementation because they consume the already-closed manifest/contract interfaces, but they must not claim Manifest M002 closure or Phase 5 finalization behavior.
