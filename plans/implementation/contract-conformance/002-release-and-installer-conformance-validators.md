# Contract and Conformance Milestone 002 — Release and Mapping Validators

Status: ready for handoff

Repository baseline: `028668d7abf9f9cb2a040a9732ecd7a4c2f6d626` (Contract M001 closed; dist 0.33 evaluation closed with disposition C; latest hosted CI green)

External predecessor evidence:

- repository: `eggstack/eggup`
- reviewed main: `cf5b3d3819c168eb2dbf841daa8332f3eb28c915`
- source plan: `plans/implementation/distribution-bootstrap/003-release-installer-conformance-validators.md`
- source plan blob: `29c1f95d3f3bc3280241d9e4ae49de32efc50dca`

Source roadmap: `plans/subsystems/contract-conformance-roadmap.md`

Long-term requirements:

- `plans/000-long-term-specification.md#6-portable-distribution-contract`
- `plans/000-long-term-specification.md#7-artifact-forms`
- `plans/001-terminology-and-domain-model.md#4-distribution-contract`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: capability / infrastructure

## 1. Objective

Extend `eggpack-contract` with deterministic, pure conformance validators for caller-supplied release inventories, archive-member inventories, and product-supplied target mapping observations. This moves the validated intent of Eggup's distribution M003 to Eggpack after the contract migration. It must not discover releases, open archives, parse scripts, build installers, or perform deployment.

## 2. Readiness and dependencies

Hard dependency: Contract M001 is closed with schema-v1 direct/bundle/archive expansion and predecessor safety guarantees.

Interface evidence: Eggup's registered M003 validator plan was reviewed at the external baseline above. It is adapted here to the Eggpack crate/API boundary; Eggpack is not claiming Eggup implementation or closure.

Independent architecture evidence: the `dist` 0.33 spike is closed with disposition C (design prior art only). No external backend is production-authorized, and that result does not change this milestone's pure contract/conformance scope.

## 3. Current evidence

`eggpack-contract` provides strict schema-v1 parsing, globally unambiguous target/alias resolution, direct/bundle/archive expansion, bounded values, strict templates, portable filename collision checks, and traversal-free member-source validation. It has three representative fixtures. It does not currently model observed release files, observed archive listings, mapping observations, or structured conformance findings.

## 4. Invariants

- The contract expansion is the sole expected-state authority.
- Observations are explicit caller-provided data; validators perform no hidden I/O.
- Unknown targets, versions, names, members, or mappings are never guessed.
- Missing required files/mappings are failures. Extras are governed by a small explicit policy; default behavior permits unrelated release extras.
- Duplicate observed names fail rather than collapse; ASCII-case collision behavior matches schema v1.
- Archive validation consumes member names but never extracts or opens archives.
- Mapping comparison never parses shell, PowerShell, Rust, or workflow source.
- Reports/findings are bounded, typed, deterministic, and sorted.
- This validates names, presence, and mappings only; it does not calculate digests or establish authenticity.
- The crate remains synchronous and free of network, subprocess, archive, build, and Eggup dependencies.

## 5. Scope

### In scope

- typed expected release-file model derived from an expanded target;
- bounded release inventory and AllowExtras/Exact policy;
- missing/unexpected-file reporting;
- bounded archive-member inventory with the schema path safety rules;
- required archive-member checks and explicit extras policy;
- typed observed target mapping for canonical target, assets, sidecars, installs, and archive source/install pairs;
- structured deterministic findings/report and convenience conformance query/result methods;
- direct, CodeGG bundle, and Egress archive fixture coverage;
- package, MSRV, docs, dependency and hosted CI qualification.

### Out of scope

- HTTP/GitHub/crates.io access or live release inspection;
- digest computation or verification;
- archive listing/extraction subprocesses;
- arbitrary source/script parsing;
- installer generation or installation;
- signing, authenticity, release ordering, publication;
- consumer repository changes or CLI unless a separately justified narrow need is found.

## 6. Required production changes

### A. Expected release-file model

Derive each required asset and checksum sidecar from one contract target and version. Cover direct, each bundle entry, and archive. Provide stable logical labels for diagnostics and reuse the existing expansion/collision implementation.

### B. Release inventory and validation

Construct a bounded flat-name inventory from caller input. Reject empty, overlong, control/separator-containing, exact-duplicate, and ASCII-case-colliding entries. Validate missing required names. Support only `AllowExtras` (default) and `Exact`; do not add filtering expressions.

### C. Archive member inventory and validation

Accept explicit archive member paths from the caller, normalize/validate using the same path constraints as contract sources, reject duplicates/case collisions, and report missing required members. Exact mode may report extras. Do not infer install identity from undeclared members.

### D. Observed mapping

Define a small serializable observation for a target containing the resolved canonical target and relevant asset, sidecar, install, and archive source/install facts. Exclude URLs, shell fragments, privilege commands, package fallbacks, service policy, and arbitrary configuration.

Compare the observation exactly to the contract expansion. Aliases may be accepted only through existing target resolution, with reports retaining canonical target identity. No target fallback is allowed.

### E. Deterministic reports

Findings must identify the category and relevant logical name/path in bounded fields. Sort findings deterministically. A successful report has no findings. Provide an `is_conformant` or equivalent and a typed result conversion without replacing structured detail with one opaque error string.

### F. Fixtures and documentation

Add positive/negative integration tests using the imported direct, bundle, and archive fixtures. Document how consumers extract observations from their own code/tests without making Eggpack parse those sources. Keep API examples in rustdoc/README.

## 7. Ordered work packages

1. Define bounded expected-file, inventory, extras policy, finding, and report types.
2. Implement release inventory construction and validation.
3. Implement archive-member inventory and validation.
4. Implement observed target mapping and exact contract comparison.
5. Add direct/bundle/archive positive and negative test matrices and deterministic report goldens.
6. Review whether a thin local-only CLI is justified; defer it unless consumer evidence shows library fixtures are insufficient.
7. Run stable/MSRV/package/docs/dependency/hosted CI qualification.
8. Write closure record and update roadmap/registry; re-evaluate Release Manifest M001 readiness and author/register its implementation plan only if this milestone closes the expected-file interface without new blockers.

## 8. Failure, restart, and contention semantics

Validators return structured findings/errors and do not mutate observations or contracts, fetch missing data, or infer alternative target/version data. They are pure and safely repeatable. No cancellation/restart or contention behavior applies. Entry counts and string/path lengths must be bounded.

## 9. Compatibility and migration

This is additive to the imported, unpublished schema-v1 API. Valid v1 documents retain identical interpretation. No Eggup repository changes are included. Any discovered need to change v1 meaning stops this plan and requires a corrective plan/ADR review.

## 10. Required tests

- Direct/bundle/archive complete inventories.
- Missing asset and sidecar; missing bundle member; archive asset plus sidecar.
- Extras accepted by default and rejected under Exact.
- Exact and ASCII-case duplicate names; invalid flat filenames.
- Complete, missing, extra, nested-valid, traversal, absolute, backslash, and duplicate archive members.
- Exact direct mapping, alias-to-canonical result, wrong target, asset, sidecar, install, and archive source/install mappings.
- Stable deterministic finding ordering.
- Entry-count and field-length bounds.
- Existing M001 tests and fixtures continue passing.
- Static/dependency guard for no process/network/archive execution or Eggup dependency.

## 11. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-contract --locked
cargo package -p eggpack-contract --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-contract --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Record hosted Linux/macOS/Windows CI separately. Update the script if validator-specific qualification needs to be added.

## 12. Documentation updates

- crate README and public rustdoc/examples;
- source roadmap;
- registry;
- closure record.

No canonical architecture document changes are expected.

## 13. Acceptance criteria

M002 closes only when release-file completeness, archive required-member presence, and runtime/bootstrap mapping can be compared deterministically for all three layout forms; reports and inputs are typed, bounded, and stable; extras policy is small and explicit; predecessor schema semantics remain unchanged; no source parser, network client, extractor, generator, or runtime deployment authority is introduced; and package/MSRV/docs/hosted CI pass.

## 14. Stop conditions

Stop and prepare corrective/ADR review if real mappings require arbitrary source parsing, v1 cannot derive unique expected names, archive validation pulls extraction ownership into Eggpack, consumers require a general filter/policy language, or validator logic would alter release/version authority.

## 15. Closure evidence required

Record implementation SHA, public API inventory, release/archive/mapping matrices, fixtures and report goldens, extras-policy and CLI decisions, dependency/package/MSRV/doc results, hosted CI, no-runtime-I/O evidence, unresolved findings, and dependency transitions.

## 16. Handoff notes

This milestone supplies the expected-file/conformance interface needed by Release Manifest M001 and installer planning. It does not authorize an Eggup migration or any consumer deletion. After closure, re-evaluate Release Manifest M001 and Bootstrap Installers M001 dependencies from their roadmaps.
