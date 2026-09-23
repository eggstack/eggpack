# Contract and Conformance Milestone 002 — Release and Mapping Validators

Status: ready for handoff

Repository implementation baseline: `e3452263225fa1ea262e03b557f40b395e6a52d8` (Contract M001 closed; dist 0.33 evaluation closed with disposition C). Producer/consumer reorientation planning baseline: `5793ccecf5139d9b7b250534703ec5ee84fe44b8` (Eggpack producer authority and the Eggup M003/M004 migration cutover dependencies registered).

External predecessor evidence:

- repository: `eggstack/eggup`
- terminal predecessor implementation: `9941c58d7039410c728860f9e4e382881d4ccf54`
- predecessor closure: `4169c8021b447fe73c8ee3ea71a80a535c940f54`
- source plan: `plans/implementation/distribution-bootstrap/003-release-installer-conformance-validators.md`
- Eggup retirement plan: `plans/implementation/distribution-bootstrap/004-retire-eggup-dist-authority.md`

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

Port the closed Eggup distribution M003 conformance surface into `eggpack-contract` and independently qualify it as Eggpack-owned behavior. The target surface covers caller-supplied release inventories, archive-member inventories, product-supplied target mapping observations, expected-release-file derivation, and deterministic structured reports. This is a fidelity migration first, not a new conformance design. It must not discover releases, open archives, parse scripts, build installers, or perform deployment.

## 2. Readiness and dependencies

Hard dependencies are satisfied: Contract M001 is closed with schema-v1 direct/bundle/archive expansion and predecessor safety guarantees, and the terminal Eggup M003 implementation/closure is available as immutable predecessor evidence.

Predecessor evidence: Eggup M003 is implemented and closed at the immutable SHAs above. Its public types, bounds, deterministic ordering, fixtures, and negative cases are the migration source. Eggpack must reproduce or explicitly map every closed behavior before refactoring. The Eggup repository has frozen `eggup-dist`; its M004 retirement remains blocked on this closure.

Independent architecture evidence: the `dist` 0.33 spike is closed with disposition C (design prior art only). No external backend is production-authorized, and that result does not change this milestone's pure contract/conformance scope.

## 3. Current evidence

`eggpack-contract` provides strict schema-v1 parsing, globally unambiguous target/alias resolution, direct/bundle/archive expansion, bounded values, strict templates, portable filename collision checks, and traversal-free member-source validation. It has three representative fixtures. It does not yet contain the closed predecessor M003 additions: `MAX_OBSERVED_ENTRIES`, `ExpectedReleaseFile`, `ReleaseInventory`, `ArchiveMemberInventory`, `ObservedTargetMapping` and related asset mapping types, `FindingKind`, `ConformanceFinding`, `ConformanceReport`, `expected_release_files`, `validate_release_inventory`, `validate_archive_member_inventory`, and `validate_observed_mapping`.

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

1. Inventory the closed Eggup M003 public API, constants, fixtures, bounds, and test matrix and record a predecessor-to-Eggpack mapping.
2. Port the expected-file, inventory, extras-policy, finding, and report types without semantic redesign.
3. Port release inventory construction and validation.
4. Port archive-member inventory and validation.
5. Port observed target mapping and exact contract comparison.
6. Port/adapt direct/bundle/archive positive and negative tests and deterministic report goldens, adding differential/golden checks where practical.
7. Review whether a thin local-only CLI is justified; defer it unless consumer evidence shows library fixtures are insufficient.
8. Run stable/MSRV/package/docs/dependency/hosted CI qualification.
9. Write closure record and update roadmap/registry; explicitly unblock Eggup distribution M004 retirement if equivalence is proven; re-evaluate Release Manifest M001 readiness and author/register it only if the expected-file interface closes without new blockers.

## 8. Failure, restart, and contention semantics

Validators return structured findings/errors and do not mutate observations or contracts, fetch missing data, or infer alternative target/version data. They are pure and safely repeatable. No cancellation/restart or contention behavior applies. Entry counts and string/path lengths must be bounded.

## 9. Compatibility and migration

This is additive to the imported schema-v1 API and is a migration of the unpublished Eggup M003 behavior. Valid v1 documents retain identical interpretation. This plan does not mutate Eggup; its successful closure supplies the external evidence needed by Eggup M004 to delete the duplicate. Any discovered need to change v1 or closed M003 meaning stops this plan and requires a corrective plan/ADR review.

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
- Predecessor M003 public API/behavior matrix is complete, including bounds and deterministic ordering.
- Direct/bundle/archive predecessor fixtures and negative cases are ported or explicitly mapped with equivalent coverage.
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

M002 closes only when release-file completeness, archive required-member presence, and runtime/bootstrap mapping can be compared deterministically for all three layout forms; reports and inputs are typed, bounded, and stable; extras policy is small and explicit; every closed Eggup M003 public behavior/fixture is ported or explicitly mapped to equivalent behavior; predecessor schema/conformance semantics remain unchanged; no source parser, network client, extractor, generator, or runtime deployment authority is introduced; and package/MSRV/docs/hosted CI pass.

## 14. Stop conditions

Stop and prepare corrective/ADR review if real mappings require arbitrary source parsing, v1 cannot derive unique expected names, archive validation pulls extraction ownership into Eggpack, consumers require a general filter/policy language, or validator logic would alter release/version authority.

## 15. Closure evidence required

Record implementation SHA, predecessor-to-Eggpack API/behavior matrix, release/archive/mapping matrices, fixtures and report goldens, bounds/deterministic-order evidence, extras-policy and CLI decisions, dependency/package/MSRV/doc results, hosted CI, no-runtime-I/O evidence, unresolved findings, and an explicit statement whether Eggup M004 retirement is unblocked.

## 16. Handoff notes

This milestone supplies the expected-file/conformance interface needed by Release Manifest M001 and installer planning and is the final producer-contract migration gate. After successful closure, Eggup distribution M004 is authorized to retire `eggup-dist`; no further producer distribution work should occur in Eggup. Then re-evaluate Release Manifest M001 and Bootstrap Installers M001 dependencies from their roadmaps.
