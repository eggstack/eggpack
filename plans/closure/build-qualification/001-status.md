# Build and Qualification Milestone 001 Closure — PackConfig and ReleasePlan

Status: closed

Source plan: `plans/implementation/build-qualification/001-pack-config-and-release-plan.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `7e63ca0209943a03f338fc4e9b722325842bc4e6` (Manifest M002 closure; core crate established).

Implementation commits: `4eba6ec9422ab01a675ef34cdfc2b9d6f9c0944d`, `fae149ebab0023e1b9fd765d6429718b612ab383`, `f1f06387157966a0288e00c04aa455daa50699e9`.

Hosted CI: [run 35955038613](https://github.com/eggstack/eggpack/actions/runs/35955038613), all four hosted jobs passed.

## Executive finding

`eggpack-core` now provides strict TOML PackConfig v1 parsing and a pure resolver to deterministic ReleasePlan data. Configured and selected targets resolve through DistributionContract; output stores canonical triples and derives direct/bundle/archive form from contract data. Build strategy, provider-neutral build and qualification hosts, toolchain versions, compatibility floors, qualification class, and support tier are bounded values. The planner does not execute build or qualification work and does not adopt the evaluated `dist` backend.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Versioned strict PackConfig v1 | `PackConfig::from_toml`; unknown-field and unsupported-version regressions. |
| Finite build, host, and toolchain model | Enumerated `BuildStrategy`, `HostOs`, `HostArch`, `HostRequirement`, and `ToolchainRequirement` types. cargo-zigbuild requires an explicit version. |
| Compatibility floor and qualification intent | `CompatibilityFloor` (none, glibc, macOS), `Qualification` (native, deferred-native, emulated, structural), and `SupportTier`. |
| Contract owns target/artifact identity | `PackConfig::resolve` rejects unknown/duplicate canonical policies and selected targets without a policy; aliases resolve to canonical triple. `PlannedAssetForm` is derived from the resolved contract target. |
| Cross-build cannot claim native qualification | Native classification requires a matching target host and is rejected for cargo-zigbuild. Regression covers host mismatch. |
| Deterministic, side-effect-free plan output | Resolver sorts canonical targets; `ReleasePlan::to_json` emits compact structured JSON; alias resolution regression. |
| No process, network, Git, build, or CI authority | Core dependency/source review; no backend/action/command configuration fields exist. |

## Production implementation evidence

PackConfig includes a Rust toolchain, optional cargo-zigbuild version, provider-neutral build host, optional distinct qualification host, target compatibility floor, classification, and release support tier. Unsupported target policies fail even when unselected. Invocation identities are opaque inputs. The resulting plan states intent and contract-derived form, not that artifacts or qualification evidence exist.

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

The workspace suite passed 60 tests. PackConfig tests cover strict parsing, schema version, aliases, canonical output, unknown target, and native host mismatch. Hosted Linux stable and Rust 1.89 passed workspace checks/tests, Clippy, and docs; hosted macOS and Windows passed workspace checks/tests.

## Invariant and compatibility review

DistributionContract remains the only target and artifact naming authority. Release identity and source revision are explicit and opaque. No path or external effect is part of plan resolution. No CI provider syntax or generic command language is introduced. No schema v1 contract or manifest change was needed. No unresolved medium-or-higher finding remains.

## Roadmap disposition and dependency transitions

Build/Qualification M001 is closed. M002 native/cross builder work remains blocked pending a backend/adapter implementation-boundary decision; M003 qualification execution and M004 finalization/aggregation remain blocked on their named upstream interfaces. CI Orchestration M001 is now ready for implementation-plan authoring because the ReleasePlan interface is fixed; it is not approved for implementation until a bounded plan is registered. Bootstrap Installers M001 and Eggpack-side Eggup Interoperability M001 remain ready. Ecosystem adoption remains blocked until the needed generated CI and consumer adoption evidence exist.
