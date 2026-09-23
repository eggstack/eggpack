# Contract and Conformance Milestone 002 Closure — Release and Mapping Validators

Status: closed

Source plan: `plans/implementation/contract-conformance/002-release-and-installer-conformance-validators.md`

Roadmap: `plans/subsystems/contract-conformance-roadmap.md`

Reviewed baseline: `801535b9cb38ad1c3af11474ef53efb5c80cd09b` (plan baseline `e3452263225fa1ea262e03b557f40b395e6a52d8`; planning history advanced on `main` before implementation)

Implementation commits: `a74009ac5726fb775bbfef0926dc763d81a9a706`, `a36803a7c34cc5b273559520bf99cb2400cc183a`

Eggup predecessor implementation: `9941c58d7039410c728860f9e4e382881d4ccf54`

## Executive finding

Eggpack now owns the closed Eggup M003 conformance behavior as additive `eggpack-contract` API. Expected release files, bounded release/member inventories, extras policy, typed observations, exact mapping comparisons, and deterministic bounded reports are implemented and exercised over direct, CodeGG bundle, and Egress archive fixtures. The predecessor source differed from Eggpack M001 only by this additive conformance surface. The port retains that public surface and behavior matrix without changing schema-v1 parsing or expansion, and adds a regression check for bundle entry relationships that the predecessor's independent-set comparison did not detect.

The API is pure: callers supply release names, archive member paths, and observations. No network, filesystem discovery, archive opening/extraction, subprocess, installer generation, release selection, or Eggup dependency was added. The optional CLI is deferred because library integration tests and consumer-produced observation fixtures exercise the boundary directly.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Expected release-file derivation for direct, bundle, archive | `expected_release_files`; stable direct/bundle/archive labels; conformance integration tests |
| Bounded flat release inventory | `ReleaseInventory`; 256 entries, filename length bound, control/separator, duplicate and ASCII-case collision rejection; inventory test |
| Missing files and extras policy | `validate_release_inventory`; default `AllowExtras`, opt-in `Exact`; direct missing/extra and bundle completeness tests |
| Bounded archive member inventory | `ArchiveMemberInventory` reuses schema member validation; duplicate, traversal, absolute, drive, backslash and dot-component tests |
| Required archive members without opening archives | `validate_archive_member_inventory`; complete, missing, allowed-extra and exact-extra cases |
| Typed direct/bundle/archive mapping observation | Observation types; TOML round-trip and golden observations for all three forms |
| Canonical target, alias, and exact field comparison | `validate_observed_mapping`; canonical result retained after alias resolution; target/assets/sidecar/install/archive mapping drift cases; bundle asset/sidecar/install association is compared per entry |
| Structured bounded deterministic reporting | `FindingKind`, `ConformanceFinding`, `ConformanceReport`; sorting, bounds, finding cap, `is_conformant` and `into_result` |
| Predecessor behavior port | Additive source port from M003; imported API and behavior matrix plus retained M001 suites |
| Docs and observation boundary | Crate README API example explains caller-owned observation extraction and no source parsing |
| CLI decision | Deferred: the library and checked-in observation fixtures exercise the boundary without additional I/O surface |
| Dependency/runtime I/O guard | Dependency tree contains only serde/toml and transitive dependencies; source scan found no process/network/archive API or Eggup reference |

## Production implementation evidence

The public API is exposed from `crates/eggpack-contract/src/lib.rs`: `MAX_OBSERVED_ENTRIES`, `ExtrasPolicy`, `ExpectedReleaseFile`, `expected_release_files`, `ReleaseInventory`, `validate_release_inventory`, `ArchiveMemberInventory`, `validate_archive_member_inventory`, observation types, `FindingKind`, `ConformanceFinding`, `ConformanceReport`, and `validate_observed_mapping`. `DistError` adds typed release/member inventory construction errors.

Contract expansion remains the sole expected-state authority. Findings are structured and sorted. Alias resolution uses the existing exact resolver. Inputs are bounded and validators perform no hidden I/O.

### Predecessor-to-Eggpack API and behavior matrix

| Closed Eggup M003 surface | Eggpack M002 surface | Equivalence evidence |
|---|---|---|
| `MAX_OBSERVED_ENTRIES` and 256-entry bound | Same public constant and checks in both inventory constructors and observation validation | Boundary tests reject 257 items |
| `ExpectedReleaseFile { label, file_name }` and `expected_release_files` | Same type/function; direct, each bundle asset/sidecar, archive/sidecar labels | Complete expected sets asserted for all layouts |
| `ExtrasPolicy::{AllowExtras, Exact}` (default permits extras) | Same variants/default for release and archive validation | Extra acceptance/rejection tests |
| `ReleaseInventory::new`, `files` | Same sorted flat-name inventory; max length 256 bytes; rejects empty, control, separators, exact and ASCII-case duplicates | Invalid-name, duplicate, case-collision, count and stable ordering tests |
| `validate_release_inventory` | Same missing required and Exact unexpected-file finding behavior | Direct missing asset/sidecar, bundle missing entry, archive asset+sidecar tests |
| `ArchiveMemberInventory::new`, `members` | Same sorted bounded inventory using schema-v1 member path validation | Nested valid, traversal, absolute, drive, backslash, dot component, duplicate and case tests |
| `validate_archive_member_inventory` | Same required-member and Exact extra comparison; non-archive input returns typed `DistError` | Complete, missing, AllowExtras, Exact cases; caller supplies names, no archive I/O |
| `ObservedDirectMapping`, `ObservedArchiveMapping`, `ObservedTargetAssets`, `ObservedTargetMapping` | Same Serde data shape and fields for direct/bundle/archive observations | Predecessor TOML observations copied and parsed; TOML round-trip test |
| `validate_observed_mapping` | Same exact contract expansion comparison; alias resolution retains canonical target; no fallback | Alias/canonical success and wrong target/asset/sidecar/install/member mismatch cases |
| `FindingKind`, `ConformanceFinding`, `ConformanceReport` | Same finding categories/fields/order, 512 finding cap, `is_conformant`, and `into_result` | Stable finding-order test and typed drift assertions |
| M003 direct, CodeGG bundle, Egress archive fixtures/matrix | Predecessor observation goldens added beside Eggpack's existing contract fixtures; 10 conformance integration tests | `golden_observation_fixtures_conform_for_all_layouts` and positive/negative matrices |

The predecessor's private bounds and validation helpers remain private. `MAX_CONFORMANCE_FINDINGS` remains an internal 512-finding cap; report fields are bounded to 512 bytes. All intended predecessor behavior is preserved; a regression test also closes the predecessor's untested false acceptance of bundle entries whose asset/sidecar/install associations were crossed.

## Exact verification executed

All local required checks passed on implementation commit `a36803a7c34cc5b273559520bf99cb2400cc183a` (with the implementation first landed in `a74009ac5726fb775bbfef0926dc763d81a9a706`):

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test -p eggpack-contract --all-targets --all-features --locked` (39 tests across 3 suites)
- `cargo test --workspace --all-targets --all-features --locked` (39 tests across 3 suites)
- `cargo doc --workspace --no-deps --locked`
- `cargo tree -p eggpack-contract --locked`
- `cargo package -p eggpack-contract --locked --allow-dirty`
- `cargo +1.89.0 check --workspace --all-targets --locked`
- `cargo +1.89.0 test -p eggpack-contract --all-targets --locked` (39 tests across 3 suites)
- `./scripts/check-local.sh`
- `git diff --check`
- Static source scan for process/network/archive APIs and Eggup dependency (no matches)

Hosted CI run `35811838730` for implementation commit `a36803a7c34cc5b273559520bf99cb2400cc183a` passed all jobs: Linux stable, Linux Rust 1.89, macOS stable, and Windows stable. Linux stable passed format, check, workspace tests, Clippy, and docs; the other jobs passed their configured check/test steps.

## Invariant, failure, and recovery review

- Contract expansion remains the expected-state authority; aliases resolve only through exact target resolution.
- Release names are flat; archive paths share schema-v1 traversal rules. Entries and observation fields are bounded.
- Duplicate names reject at inventory construction; ASCII case collisions use ASCII lowercase semantics.
- Findings are sorted by kind, label, expected, then observed; report output is bounded deterministically.
- Validation is repeatable, structured, and non-mutating. No restart or contention behavior applies.

## Compatibility, security, and documentation review

Schema-v1 behavior is unchanged. The M003 API is additive and preserves the predecessor's `AllowExtras` default and exact-set option. Validators compare names and mappings only; they do not compute digests or claim authenticity. No new dependency or execution authority was introduced. README and rustdoc explain that consumers produce observations from their own code/tests, which Eggpack does not parse.

## Unresolved findings

None. No medium-or-higher conformance ambiguity or corrective work was identified.

## Roadmap disposition and dependency transitions

- Contract and Conformance M002 is closed.
- Release Manifest M001 is unblocked and ready: expected-file derivation is stable and available; a registered implementation handoff is added with this closure.
- Eggup distribution M004 retirement is unblocked by the proven Eggpack equivalence. This authorizes the already-registered Eggup plan; this closure does not claim Eggup files changed or that retirement executed.
- Build/qualification M001, Bootstrap Installers M001, CI orchestration, Eggup interoperability, and adoption remain blocked by Release Manifest M001 or their other dependencies.
- No corrective plan is required.

Registry and affected roadmaps were updated with these transitions. The plan is formally closed.
