# Contract and Conformance M001 — Closure

Status: closed

## Source plan and roadmap

- Plan: `plans/implementation/contract-conformance/001-workspace-and-distribution-contract-v1-import.md`
- Roadmap: `plans/subsystems/contract-conformance-roadmap.md`
- Reviewed Eggpack baseline: `23a7f312589327088482a0158be611b472c80dc9`
- Implementation commit: `3283634` (`feat: import distribution contract v1 into eggpack`)
- CI setup correction: `82a4ac8` (`ci: scope formatting check to stable toolchain`)

## Executive finding

Eggpack now owns a Rust 2021 workspace with the Rust 1.89 MSRV and the portable `eggpack-contract` crate. Its implementation and fixtures were imported from Eggup's qualified unpublished `eggup-dist` at the exact M002 corrective baseline. Schema-v1 semantics remain unchanged. Eggup was not modified; its predecessor implementation remains intact. M001 is closed and the pure conformance-validator follow-on is ready.

## Requirement-to-evidence matrix

| Requirement / invariant | Evidence |
|---|---|
| Explicit schema v1; unknown majors typed; unknown fields rejected | `unknown_schema_version_fails_typed`, `unknown_fields_are_rejected` |
| Canonical target/alias uniqueness and no guessing | `duplicate_target_is_rejected`, `duplicate_alias_is_rejected`, `alias_colliding_with_triple_is_rejected`, `unsupported_target_lookup_fails_without_guessing`, `missing_alias_input_fails` |
| Strict, non-executable template grammar and context rules | `unknown_placeholder_is_rejected`, `malformed_template_grammar_is_rejected_during_parse`, `repeated_known_placeholders_remain_valid` |
| Exact/ASCII-case release, sidecar, and install collision checks | collision tests for direct, bundle, and archive expansions, including asset/sidecar cross-collisions |
| Literal, normalized, safe archive-member sources | `archive_member_absolute_path_is_rejected`, `archive_member_traversal_is_rejected`, `duplicate_normalized_archive_members_rejected` |
| Opaque but filesystem-safe versions; deterministic expansion/round-trip | `product_version_is_opaque_but_filesystem_safe`, `deterministic_round_trip_golden`, fixture round-trip test |
| Direct, sibling-bundle, and archive layouts | `simple_direct_fixture_expands`, `codegg_bundle_fixture_expands_three_members`, `egress_archive_fixture_expands_members` |
| Rust 1.89 workspace and leaf dependency boundary | stable/MSRV check and test; `cargo tree -p eggpack-contract --locked` lists only serde and toml runtime dependencies |
| No process/network/archive/updater implementation | crate API/source inspection; dependency tree contains no such stack |
| Eggup predecessor retained | no Eggup repository changes; source copied from the pinned Eggup commit |

## Production implementation evidence

- Root workspace has one member: `crates/eggpack-contract`.
- Package/library name: `eggpack-contract`; public model/error API and schema remain the predecessor's.
- Source blob imported byte-for-byte: Eggup `5a8d7bc7c3bd134b691f8e1c7b2f6de640a55477`.
- Package metadata blob: `41d153536e724b379f4b78a5072ffe412fe240b8`.
- Fixture harness source blob: `4ef2e0704603ba00128f2a715e8b35d5422a41fa`; crate identifiers were renamed.
- Direct, CodeGG bundle, and Egress archive fixtures were copied unchanged from blobs `499c2192bd2eebdb543df38021b3586ec69f6fe4`, `6f54543e49488d5b6363f28b52f13dc819f3d52c`, and `e1106397459bfe8ade40922f6d8f74287e372a7e`.
- Workspace policy denies unsafe code. The CI workflow covers Linux stable and 1.89, plus stable check/test on macOS and Windows. There are no release or publication workflows.
- Public API inventory is unchanged from the predecessor: schema constant; typed contract, product, target, asset-form and expansion models; parser/serializer; deterministic target resolution/expansion; and typed errors.

## Verification actually run

Local `./scripts/check-local.sh` passed on 2026-09-23. It ran formatting, workspace check, Clippy with warnings denied, 24 unit tests and 4 integration tests, workspace tests, rustdoc, dependency tree, package verification, and Rust 1.89 check/tests. `git diff --check` passed.

`cargo package -p eggpack-contract --locked --allow-dirty` packaged and verified six publishable files. Cargo's notice that integration tests are excluded is expected: the crate manifest excludes `tests/`.

Hosted GitHub Actions run [35805026227](https://github.com/eggstack/eggpack/actions/runs/35805026227) passed on Linux stable, Linux Rust 1.89, macOS stable, and Windows stable. An earlier run failed only because rustfmt was not installed on the MSRV lane; the workflow was corrected to run formatting on stable, and the corrected run passed. CI emitted upstream deprecation/runner-image notices only.

## Invariant, recovery, compatibility, and security review

- Contract parsing and expansion remain pure and synchronous. Invalid input returns typed errors without mutation; there is no partial expansion return.
- No restart/contention behavior applies. No global mutable state was added.
- The import preserves the unpublished predecessor's schema-v1 behavior. This is fidelity evidence, not a crates.io compatibility promise.
- Dependency tree is limited to serde/toml and their transitive parser/derive dependencies. No Eggup dependency is present.
- No network access, subprocess execution, archive extraction, release publication, signing, live installation/update, or service management was added.
- Bounded inputs, placeholder grammar, flat-name validation, collision detection, and archive path safety remain in the imported implementation and tests.

## Documentation and operations

Root README now describes the implemented crate; crate README and rustdoc document schema-v1, artifact forms, naming/path rules, integrity-vs-authenticity, boundaries, and provenance. `scripts/check-local.sh` is the repeatable local qualification entrypoint. CI is checked in. No release workflow or credentials are involved.

## Unresolved findings

No unresolved medium-or-higher findings. The generated Cargo lockfile currently resolves TOML 0.8.23 under the declared 0.8 range; dependency updates remain ordinary maintenance.

## Roadmap disposition and dependency transitions

- Contract/conformance M001: **closed**.
- Contract/conformance M002: **ready**. At M001 closure, the Eggup M003 validator handoff had been reviewed from Eggup main `cf5b3d3819c168eb2dbf841daa8332f3eb28c915` (plan blob `29c1f95d3f3bc3280241d9e4ae49de32efc50dca`). Eggup M003 was subsequently implemented and closed at implementation `9941c58d7039410c728860f9e4e382881d4ccf54` / closure `4169c8021b447fe73c8ee3ea71a80a535c940f54`; the current Eggpack M002 plan is rebaselined as a fidelity port of that terminal predecessor behavior.
- Release Manifest M001 remains blocked on Contract M002's expected-file/conformance interface; M001 alone does not satisfy that interface.
- Bootstrap Installers, Build/Qualification, Eggup Interoperability, and Adoption remain blocked by their roadmap dependencies.
- External Backend Evaluation M001 was independent of this closure and has since closed with disposition C; it does not authorize a production backend.

## Registry updates

The current registry records Contract M001 closed and Contract M002 as the sole dependency-ready producer-contract handoff. M002 now depends on the closed Eggup M003 predecessor evidence and, when closed, authorizes the already-registered Eggup M004 retirement. The external-backend spike is closed with disposition C. Eggup's retained predecessor and source baselines remain recorded until retirement executes.
