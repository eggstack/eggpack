# Build and Qualification Milestone 005 Closure — Deterministic Cross-Tool Provisioning

Status: closed

Source plan: `plans/implementation/build-qualification/005-deterministic-cross-tool-provisioning.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `f3b1f3e53ab1ddca7f3cee9f1bc885a965d660a7` (plan baseline; architecture-documentation-only successor to the reviewed `b8062dff` implementation baseline).

Implementation SHA: `7a206ba73e986352410f94b34c7c616c1ea26019` (`feat: implement Build M005 deterministic cross-tool provisioning`).

Hosted CI on that exact SHA: [36256831000](https://github.com/eggstack/eggpack/actions/runs/36256831000), completed successfully (push event, all jobs): `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, and `portability (windows-latest)` all success.

First proving consumer baseline (unchanged, not migrated by this closure): `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`. Consumer parity was additionally confirmed against live eggsact `main` during closure: `.github/workflows/release-binaries.yml` still pins Zig 0.14.1, cargo-zigbuild 0.23.3, the same official archive names/digests/origin, the `cargo install --locked` acquisition path, and the `.2.17` floor syntax encoded by this milestone.

## Executive finding

M005 is closed. CargoZigbuild release jobs no longer assume mutable ambient tools on hosted runners: the exact Zig version is producer policy (`ToolchainRequirement::zig`), a finite provider policy (`CrossToolProvisioningV1`) provisions the exact cargo-zigbuild plus the SHA-256-verified official Zig archive for Linux x86-64 and AArch64, and both exact versions are checked before any product build. ADR-0004's first-party Cargo/cargo-zigbuild boundary is preserved; no generic package manager, arbitrary URL, arbitrary setup command, or new build backend was introduced.

## Requirement-to-evidence matrix

| Requirement (plan §) | Evidence and result |
|---|---|
| Exact Zig version is producer policy (§4) | `ToolchainRequirement` gains additive optional `zig`; `CargoZigbuild` requires both `cargo_zigbuild` and `zig`, `NativeCargo` requires both absent; grammar is the existing safe-tool-version set (`zig_version_is_required_for_zigbuild_and_forbidden_for_native`). |
| Schema stays v1; historical docs parseable (§4/§15) | `PackConfig`, `ReleasePlan`, `CIPlan` remain schema version 1; `zig` and `cross_tools` are `#[serde(default, skip_serializing_if)]` additive fields. Historical schema-v1 CargoZigbuild input without Zig parses (`PackConfig::from_toml` ok) but fails resolution/validation and never selects ambient Zig (`m005_missing_tool_version_fails_closed_never_ambient`). Pre-M005 readers are not promised to understand newly emitted schema-v1 documents carrying the additive Zig field. |
| Provider provisioning policy is finite (§5) | `CrossToolProvisioningV1 { cargo_install_timeout_minutes, zig_connect_timeout_secs, zig_max_time_secs, zig: ZigOfficialArchiveV1 { linux_x86_64_sha256, linux_aarch64_sha256 } }`; Zig version comes from `TargetPolicy`, never the provider; origin fixed to `https://ziglang.org/download/<version>/...`; only Linux x86-64/AArch64 hosts accepted; wrong/empty/uppercase/zero digests and out-of-bound timeouts reject (`m005_provisioning_policy_rejects_bad_digests_and_bounds`). |
| cargo-zigbuild provisioning (§6) | Rendered `CARGO_INSTALL_ROOT="$install_root" cargo install cargo-zigbuild --version '0.23.3' --locked` into an invocation-private root, only its `bin` joins PATH via `$GITHUB_PATH`, step carries `timeout-minutes: 10`, and `test "$actual" = 'cargo-zigbuild 0.23.3'` gates success (`m005_provisioned_render_pins_exact_tools_without_ambient_dependency`, `m005_release_renderer_provisions_before_build_with_private_cache`). No ambient cargo-zigbuild is accepted: provisioned validation does not consult the runner booleans, which stay `false` in all new tests. |
| Zig provisioning (§7) | Finite archive mapping (`zig_archive_name`, `zig_download_url`), bounded HTTPS-only curl, checked-in SHA-256 verified before extraction, single-top-level layout validation, private extraction, regular non-symlink executable requirement, `CARGO_ZIGBUILD_ZIG_PATH` binding, invocation-private `CARGO_ZIGBUILD_CACHE_DIR`, exact `test "$("$zig_bin" version)" = '0.14.1'` equality. No apt/system Zig; PATH carries the verified directory for diagnostics only. |
| Download boundary (§8) | Rendered `curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location --proto-redir '=https' --connect-timeout 30 --max-time 600`; no credentials; configured digest is the authority (`sha256sum --check -` against the checked-in value); extraction destination is private. The release-workflow static guard now permits exactly this finite bootstrap form and still rejects any other `curl` invocation. |
| Runner model (§9) | `RunnerMapping` labels keep host OS/arch meaning. Absent `cross_tools` is `PreinstalledVerified` legacy mode: both exact versions are now checked (`actual_zig="$(zig version)"` + equality; goldens `m002-mixed.yml`, `native-direct-multitarget.yml` regenerated for exactly this two-line change). Present `cross_tools` is `Provisioned` mode used for eggsact. |
| Workflow ordering (§10) | Release renderer proves install → provision → build → capture ordering by step offsets (`m005_release_renderer_provisions_before_build_with_private_cache`); tool setup precedes the product build and the candidate handoff; no provisioning job has write permissions (`contents: read` only; stage remains the sole writer). |
| Cache policy (§11) | No cross-run cache. `CARGO_ZIGBUILD_CACHE_DIR` is invocation-private scratch under `runner.temp` with run/attempt identity; rendered value cannot resolve to an ambient/shared directory (asserted absent: `~/.cargo`, `$HOME/.cargo`; asserted present: `zigbuild-cache/${{ github.run_id }}-${{ github.run_attempt }}`). |
| No 0.23.4 upgrade / no prebuilt binaries (§6/§12) | Renderer output for the 0.23.3 policy contains `cargo-zigbuild 0.23.3` and no `0.23.4`, no `cargo-zigbuild-x86_64` prebuilt artifact, no `releases/download` acquisition. |
| glibc floor unchanged (§12) | All provisioned renders retain `x86_64-unknown-linux-gnu.2.17` / `aarch64-unknown-linux-gnu.2.17` cargo-zigbuild target syntax; `cargo_command` semantics untouched. |
| M003c invariants unchanged (§12) | Source-identity, least-privilege, and staging suites all green unmodified; provisioned release render passes `check_release_github` drift check deterministically. |

## Production implementation evidence

- `crates/eggpack-core/src/lib.rs`: additive `ToolchainRequirement::zig`, shared `valid_tool_version` grammar, strategy-coupled presence validation (`validate_policy`), lenient `from_toml` shape check (historical Zig-less documents parse; resolution fails closed), `NativeCargo` rejects either cross-tool version.
- `crates/eggpack-core/src/builder.rs`: local preflight now requires exact `zig version` equality with the configured Zig version (previously presence-only); `tool_summary` records `rust + cargo-zigbuild + zig`. `cargo_command`/floor rendering unchanged.
- `crates/eggpack-ci/src/lib.rs`: `ZigOfficialArchiveV1`, `CrossToolProvisioningV1` (+`validate`), `zig_archive_name`, `zig_download_url`, `zig_expected_digest`, `validate_sha256_hex`, `validate_zig_archive_listing`, `validate_provisioned_zig_dir`, `GitHubPolicy::cross_tools` (additive optional), updated `CIPlan` toolchain validation, updated `GitHubPolicy::validate` (provisioned host allowlist vs legacy boolean requirement), shared `cross_tools_section`/`provision_cross_tools_steps`/`verified_cross_tools_step` renderers wired into all three workflow renderers (`render_github`, exact `render_release_github`, reusable `render_reusable_release_github` via the shared inner renderer), provisioned `CARGO_ZIGBUILD_ZIG_PATH`/`CARGO_ZIGBUILD_CACHE_DIR` build env, and the narrowed curl static guard.
- `crates/eggpack-cli/src/main.rs`: test-harness construction updated for the additive fields; no CLI surface change.
- Goldens: `m002-mixed.yml` and `native-direct-multitarget.yml` regenerated for the two-line `PreinstalledVerified` exact-Zig check only; all other goldens byte-identical.

## Rendered provisioning snippets (x86-64 and AArch64)

Representative provisioned job section (exact versions from producer policy, digests from provider policy):

```text
- name: Install exact cargo-zigbuild
  timeout-minutes: 10
  run: |
    set -euo pipefail
    install_root="${{ runner.temp }}/eggpack/cargo-install/${{ github.run_id }}-${{ github.run_attempt }}"
    mkdir -p "$install_root"
    CARGO_INSTALL_ROOT="$install_root" cargo install cargo-zigbuild --version '0.23.3' --locked
    echo "CARGO_INSTALL_ROOT=$install_root" >> "$GITHUB_ENV"
    echo "$install_root/bin" >> "$GITHUB_PATH"
    actual="$(cargo zigbuild --version)"
    test "$actual" = 'cargo-zigbuild 0.23.3'
- name: Provision verified Zig 0.14.1
  run: |
    set -euo pipefail
    zig_version='0.14.1'
    zig_archive='zig-x86_64-linux-0.14.1.tar.xz'
    zig_url='https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz'
    zig_sha='24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c'
    ...
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location --proto-redir '=https' --connect-timeout 30 --max-time 600 -o "$archive" "$zig_url"
    printf '%s  %s\n' "$zig_sha" "$archive" | sha256sum --check -
    ... single-top-level layout validation, private extraction ...
    test ! -L "$work/extract/$top/zig"
    test -f "$work/extract/$top/zig"
    test -s "$work/extract/$top/zig"
    test -x "$work/extract/$top/zig"
    zig_bin="$work/extract/$top/zig"
    test "$("$zig_bin" version)" = '0.14.1'
    ...
    echo "CARGO_ZIGBUILD_ZIG_PATH=$zig_bin" >> "$GITHUB_ENV"
    echo "CARGO_ZIGBUILD_CACHE_DIR=$cache" >> "$GITHUB_ENV"
```

AArch64 renders the same shape with `zig-aarch64-linux-0.14.1.tar.xz` and digest `f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b` (proven by `m005_provisioned_aarch64_uses_arch_digest_and_name`).

Build steps in provisioned jobs carry:

```text
env:
  CARGO_TARGET_DIR: "${{ runner.temp }}/eggpack/${{ github.run_id }}-${{ github.run_attempt }}"
  CARGO_ZIGBUILD_ZIG_PATH: ${{ env.CARGO_ZIGBUILD_ZIG_PATH }}
  CARGO_ZIGBUILD_CACHE_DIR: ${{ env.CARGO_ZIGBUILD_CACHE_DIR }}
```

## Verification executed

All verification below ran against implementation commit `7a206ba` (closure edits touch plans only; no production code changed after the hosted run):

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test --workspace --all-targets --all-features --locked           passed (206 passed, 7 ignored)
cargo doc --workspace --no-deps --locked                               passed
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                 passed (52 passed, 3 ignored)
./scripts/check-local.sh                                               passed (exit 0, includes packaging)
git diff --check                                                       passed
```

Hosted CI run 36256831000 on the exact implementation SHA passed all four lanes on first attempt: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher toolchain reproducibility finding. | M005 acceptance criteria satisfied. |
| Operational boundary | Eggpack hosted CI has no Linux AArch64 runner, so the AArch64 provisioning path is proven by unit/fixture tests (archive mapping, digest selection, render) but has never executed on a hosted runner from this repository. | Recorded per plan §13; the first real Linux x86-64/AArch64 provisioning executions are recorded in eggsact M001 (mirrored eggsact M005), not fabricated here. |
| Operational condition | No real Zig archive was downloaded during Eggpack verification (no network in unit tests by design). | SHA/layout/path assumptions are covered by local fixtures (wrong digest, extra top-level member, traversal/absolute entries, symlinked/non-executable/empty `zig` rejection). Live download proof belongs to the consumer adoption runs. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M005 | Reason |
|---|---|---|
| Build/Qualification M005 | closed | This record; implementation `7a206ba`; hosted run 36256831000 green. |
| Ecosystem adoption M001 (eggsact) | ready (unblocked) | M003d and Build M005 are both closed; the sole remaining Eggpack-side prerequisite is satisfied. The mirrored eggsact M005 plan stays registered; M001 itself performs the outstanding M003b live-draft/rerun proof. No eggsact repository change is claimed by this closure. |
| CI M003b live draft qualification | ready to resume | M003d corrective dependency was already closed; the Build M005 prerequisite is now closed. The live proof still runs inside eggsact M001, not in this repository. |
| Bootstrap M003 | blocked (unchanged) | Independent adoption evidence/candidate review remains outstanding. |
| Eggup Interoperability M003 | ready to plan (unchanged) | Independent of this seam. |

Historical closure records are preserved. Registry, the build-qualification roadmap, the ecosystem-adoption roadmap, and the CI orchestration roadmap now identify M005 as closed and Ecosystem M001 as the unblocked handoff. Newly generated schema-v1 documents carrying the additive Zig field are a deliberate pre-1.0 forward-compatibility break for pre-M005 Eggpack binaries.

## Eggsact M001 readiness disposition

Ready. Eggpack now provisions the exact Zig 0.14.1 / cargo-zigbuild 0.23.3 pair with the exact official digests the consumer already uses, binds the build to the verified executable via `CARGO_ZIGBUILD_ZIG_PATH`, isolates `CARGO_ZIGBUILD_CACHE_DIR`, and preserves the `.2.17` target-floor syntax. No deferred M005 follow-up (§18: 0.23.4 evaluation, prebuilt-binary acquisition, ELF symbol-floor auditing) is required before first-consumer adoption; each is an independent future review only if post-M005 evidence justifies it.
