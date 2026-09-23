# Release Manifest Milestone 001a Closure — Cross-Target Install Namespace Corrective

Status: closed

Source plan: `plans/implementation/release-manifest/001a-cross-target-install-namespace-corrective.md`

Roadmap: `plans/subsystems/release-manifest-roadmap.md`

Reviewed baseline: `8bdccb325c056ed8364d7ed77a5187b45ba7142f` (M001a registered as the sole dependency-ready corrective handoff).

Implementation commit: `20a3084bbfbc7fac03b70e0c397f9059186169c8`.

Hosted CI run: [CI run 35868707685](https://github.com/eggstack/eggpack/actions/runs/35868707685), triggered by the implementation commit.

## Executive finding

ReleaseManifest schema-v1 validation now treats each canonical target as an independent installation namespace. Exact and ASCII-case-colliding install names remain invalid within one target, while the same install identities may appear under mutually exclusive targets. Release artifact filenames continue to be checked in one manifest-global flat namespace. The correction changes validation scope only; schema-v1 fields, enum tags, serialization ordering, and deterministic JSON bytes are unchanged.

The pre-fix defect was reproduced with the checked-in Eggsact direct contract shape: Linux x86_64 and macOS arm64 each install `eggsact`, using distinct release artifact filenames. With the original validator, the regression failed because the second target collided in the manifest-global install set. After the correction, it passes.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Install namespace is target-local | `ReleaseManifest::validate()` initializes install-name tracking inside the target loop; `direct_install_names_are_target_local_like_eggsact_contract` accepts repeated `eggsact` across Linux and macOS. |
| Archive target-local namespace | `archive_install_names_are_target_local_like_egress_contract` accepts repeated `egress` / `egress-helper` across Linux and macOS with distinct archive files; values mirror `crates/eggpack-contract/tests/fixtures/egress-archive.toml`. |
| Bundle target-local namespace | `bundle_install_names_are_target_local` accepts repeated bundle install identities across targets. |
| Within-target exact and ASCII-case install collisions still fail | `install_collisions_remain_rejected_within_target` covers bundle and archive member installs. A direct record has exactly one install identity by schema shape. |
| Release filename namespace remains global | `release_artifact_collisions_remain_manifest_global` rejects both exact and ASCII-case collisions across canonical targets. |
| Existing direct/bundle/archive behavior and deterministic JSON | Existing round-trip tests remain; `direct_round_trip_stable` asserts the unchanged byte-exact JSON golden and canonical ordering tests pass. |
| DistributionContract representative behavior | Regression comments identify the direct and archive source fixtures; the contract fixture suite passes unchanged. |
| No dependency or authority expansion | `cargo tree -p eggpack-manifest --locked` lists only Serde, serde_json, and their transitive dependencies. |

## Production implementation evidence

- Kept the `release_names` set at manifest scope.
- Moved only the install-name set into each `TargetRecord` iteration.
- Added five multi-target/namespace regressions plus within-target exact/case negatives for bundle and archive forms.
- No production dependency on `eggpack-contract` was added.
- No ADR was required because the correction preserves the schema and authority boundary.

## Verification executed

All required verification passed for the implementation. Local commands included:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-manifest --locked
cargo package -p eggpack-manifest --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked
cargo +1.89.0 test -p eggpack-contract --all-targets --locked
./scripts/check-local.sh
git diff --check
```

The targeted manifest suite passed 11 tests. Contract tests passed (39 across the three contract test binaries). The workspace suite and `check-local.sh` passed. Package verification succeeded. Manifest docs and the Rust 1.89 checks/tests passed.

Hosted CI run 35868707685 passed all four required lanes: Linux stable (including fmt, workspace check/test, Clippy, and docs), Linux Rust 1.89 (workspace check/test), macOS stable (workspace check/test), and Windows stable (workspace check/test). GitHub emitted informational runner/action deprecation notices; no job failed.

## Invariant review

- Canonical target records remain independent installation namespaces.
- Exact and ASCII-case install collisions fail inside one target.
- Identical installs across different canonical targets are accepted.
- Exact and ASCII-case release artifact filename collisions fail across the whole manifest.
- Archive member source uniqueness remains target-local.
- Bundle relationships and archive source/install/member-byte relationships are unchanged.
- Deterministic serialization and schema-v1 wire shape are unchanged; the existing JSON golden is byte-identical.
- Identity bounds, sizes/digests, target uniqueness, version/unknown-field behavior, and evidence-reference semantics are untouched.
- No build, network, process, archive extraction, install, publication, or trust authority was introduced.

## Failure/recovery review

Invalid exact/case-colliding install names in one target and invalid global release filename collisions still return validation errors. Distinct targets can now repeat installs without changing parsing, serialization, or recovery behavior. The failed pre-fix regression demonstrates the false rejection and the same test passes on the corrected implementation.

## Compatibility and migration review

This is a schema-v1 validation-correctness fix. Serialized fields, enum tags, ordering rules, digest encoding, size semantics, and version number are unchanged. Manifests valid under the DistributionContract target-local namespace model become accepted; no previously valid within-target collision becomes valid. No migration is required.

## Security review and unresolved findings

The leaf dependency tree remains limited to Serde/serde_json and transitives. Inputs remain bounded, filenames and member paths retain their existing validation, and the manifest still carries integrity facts rather than trust claims. No unresolved medium-or-higher finding remains in the manifest domain for this corrective.

## Roadmap disposition and dependency transitions

M001a is closed. The following Eggpack work is unblocked and marked ready:

- Release Manifest M002 final-artifact builder;
- Build/Qualification M001 PackConfig/ReleasePlan;
- Bootstrap Installers M001 generator model and direct fixtures;
- Eggup interoperability M001 Eggpack-side interface/fixture work.

These milestones were recorded as blocked while M001a was open. Their dependency status was re-evaluated after local qualification and hosted CI passed for the corrective implementation.

Eggup-side implementation still requires a registered Eggup plan. CI orchestration M001 remains blocked on the Build/Qualification M001 interface. Ecosystem adoption remains blocked on the generator/CI capabilities. Eggup M004 retirement remains independently authorized by Contract M002. No other downstream work is claimed closed by this record.

Registry and subsystem roadmaps now reflect these transitions. The original M001 closure remains historical evidence, with only a short addendum pointing to this corrective closure.
