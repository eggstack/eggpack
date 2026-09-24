# Release Manifest Milestone 002 Closure — Final Artifact Manifest Builder

Status: closed

Source plan: `plans/implementation/release-manifest/002-final-artifact-manifest-builder.md`

Roadmap: `plans/subsystems/release-manifest-roadmap.md`

Reviewed baseline: `154d4a2ebd6b2703c9f368c108ccd5e75b2cf388` (current planning baseline; code baseline is the previously registered `0b1c3b9795280dea610c2fbe9d1533591d610b3f`).

Implementation commits: `4eba6ec9422ab01a675ef34cdfc2b9d6f9c0944d`, `fae149ebab0023e1b9fd765d6429718b612ab383`, `f1f06387157966a0288e00c04aa455daa50699e9`.

Hosted CI: [run 35955038613](https://github.com/eggstack/eggpack/actions/runs/35955038613), all Linux stable, Linux Rust 1.89, macOS, and Windows jobs passed. The initial run 35954770550 exposed a macOS test-fixture target-selection error; the fixture was corrected to mutate the selected host target and the complete hosted matrix passed on the next run.

## Executive finding

`eggpack-core::build_manifest` now builds a validated Manifest v1 value from one explicit release identity and explicitly named finalized files. Contract expansion and exact release inventory validation precede file reads. Direct, bundle, and archive relationships come from the contract; archive member evidence is read from explicit caller-provided finalized member files. The builder streams SHA-256 and derives size from the same bytes. PackConfig/ReleasePlan is implemented in the same core crate under the next milestone; its closure is recorded separately below.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| New producer-only `eggpack-core`; manifest remains leaf | `crates/eggpack-core`; dependency tree shows only contract, manifest, Serde/TOML/JSON, and SHA-256 support. `eggpack-manifest` has no dependency on core. |
| Explicit ProductId, ReleaseId, source revision, and target inputs | `FinalizedReleaseInput` and `FinalizedTargetInput`; no discovery or directory scanning API. |
| Contract/conformance gate including sidecars and exact inventory | `expected_release_files`, `ReleaseInventory`, and `ExtrasPolicy::Exact` run before artifact reads; missing and extra inventory regression. |
| Direct, bundle, and archive output preserves contract identity | Eggsact direct, CodeGG bundle, and Egress archive builder tests. Archive members are separately named file inputs and keep source/install relationships. |
| Exact non-zero byte evidence; non-regular input rejection | `digest_file` rejects symlinks/non-regular files, streams bytes through SHA-256, and rejects empty files. Artifact size is counted from the hashed stream. |
| Deterministic, validated manifest output | `ReleaseManifest::validate` gates return; `to_json` remains the canonical serializer. Existing deterministic manifest goldens pass. |
| No build/network/process/archive/Eggup authority | No such dependencies or calls in `eggpack-core`; archive bytes are not opened or extracted. |

## Production implementation evidence

- Added workspace package `crates/eggpack-core` and the initial dependency direction.
- Used explicit filename→path maps; expected logical names are derived from contract expansion and are never derived from a directory scan.
- Applied exact inventory conformance before hashing and returned no partial result on failure.
- Recorded the known boundary: member bytes are independently evidenced, but M002 does not prove that a supplied archive contains those member bytes. Archive assembly/finalization remains later Build/Qualification work.

## Verification executed

Passed locally:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty [with local path patches for unpublished workspace dependencies]
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --all-features --locked
./scripts/check-local.sh
git diff --check
```

The complete workspace suite passed 60 tests. Core tests cover direct, bundle, and archive construction, deterministic alias/canonical identity, and missing/extra contract inventory. Package verification used Cargo `patch.crates-io` path overrides for the two unpublished in-workspace dependencies; `scripts/check-local.sh` records the exact reproducible invocation.

## Invariant, failure, compatibility, and security review

The manifest crate remains filesystem/network/build independent. Inputs are caller-named and are never modified. Contract, inventory, metadata, read, and schema errors fail without returning a manifest. Symlinks and non-regular objects are rejected before reads; diagnostics omit local paths and file contents. SHA-256 is integrity evidence only. There is no lock or immutable snapshot guarantee against a concurrent producer changing an input path; stronger owner-private finalization remains later work.

Manifest v1 fields and wire shape are unchanged. New producer construction code is additive. No unresolved medium-or-higher finding remains for the M002 capability; the archive/member-content relationship and concurrent input mutation limits are explicitly carried forward.

## Documentation and roadmap disposition

The core crate README records producer/consumer authority and file-input boundaries. The Release Manifest roadmap and registry now mark M002 closed. The manifest schema-export M003 remains planned because the milestone did not produce a real runtime consumer.

Downstream transitions: Build/Qualification M001 can extend the established `eggpack-core` and remains ready; Bootstrap M001 and Eggpack-side Eggup Interoperability M001 remain independently ready. No later milestone is claimed closed here.
