# Ecosystem Adoption Milestone 003a Closure — Eggsearch Seven-Target Compatibility Preflight and Migration Design

Status: closed

Source plan: `plans/implementation/ecosystem-adoption/003a-eggsearch-seven-target-compatibility-preflight-and-migration-design.md`

Roadmap: `plans/subsystems/ecosystem-adoption-roadmap.md`

Applicable ADRs: `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`, `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`, `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`, `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md`

Eggpack baseline: `c892990` (tip at review; plan implementation baseline `911da48c7c1c7967397a2d190730fc643c8c6dc3` plus the two closed milestone lines Contract M003 `43fa2d7`/`c892990`).

Reviewed external baseline: `eggstack/eggsearch@ec437cb87c2b97a835a13377838c34631acad63b` ("close CodeGG M005 0.4.1 release, reconcile M003/M004, record M002 evidence").

External drift check: `ec437cb` is an ancestor of `origin/main` (`33f508d`) with 4 commits between, and `git diff --stat ec437cb origin/main` touches only `README.md`, `AGENTS.md`, `architecture/*.md`, `docs/*.md`, and `skills/`. No workflow, source, configuration, or release file changed. The plan's §2 "re-review if the external baseline changes materially" trigger is therefore **not** tripped for any release-relevant fact. The 0.13.0 pin, the seven targets, the attestation job, and `--clobber` are all confirmed against `ec437cb` specifically.

Implementation commits: none. M003a is read-only/scratch by its own §8 and §10. Eggpack production code, every consumer repository, and every external release/tag/attestation are unchanged.

## Executive finding

**M003b is ready for plan authoring**, with a mirrored Eggsearch plan required first. All four gaps resolved with evidence, and **no new Eggpack producer capability is required**.

That last sentence is the important one, because the plan was written expecting one of the four to land on Option B and force a prerequisite milestone. None did:

- **G1 → Option A.** The Zig 0.13.0 archive-layout incompatibility is real and was confirmed against the live upstream index, but the pin is *not* product compatibility policy. `0.13.0` appears in exactly one file in the entire Eggsearch repository at `ec437cb` — `.github/workflows/release-binaries.yml`. It is absent from the README, `docs/release.md`, every closure record, and every architecture document. The externally meaningful guarantee is the **glibc 2.17 floor**, and that is asserted post-build on the produced bytes by an independent `readelf` scan, so it survives a toolchain bump and is re-provable inside adoption. Selecting Option B here would have preserved an unpublished implementation detail at the cost of a new producer capability milestone.
- **G2 → Option A.** `Qualification::Structural` plus a required Eggsearch-owned consumer validator is expressible today and renders a workflow where the ARMv7 runtime proof is still release-gating. Proven by rendering both variants.
- **G3 → Option A, design only.** A narrow Eggsearch-owned provenance workflow can attest exact already-staged draft bytes without recreating producer release authority. No live attestation was created, per §7.
- **G4 → accepted.** Eggpack's no-clobber staging is strictly stronger than `--clobber`, and operator recovery for an incomplete draft is a two-step procedure.

The single genuinely new *constraint* discovered is a schema rule, not a gap: `Qualification::Structural` **forbids** a core smoke binding (`QualificationBindingsV1::validate_for` requires `smoke.is_some() == must_smoke`, where `must_smoke` is true only for `Native | DeferredNative | Emulated`). That is correct behaviour — a structural classification executes nothing, so a core smoke binding would be incoherent — but it is a sharp edge a migrating consumer will hit. Recorded as finding E-1.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §11.1 freeze the Eggsearch current release/config/install/provenance baseline | Frozen at `ec437cb`: seven targets in `.github/workflows/release-binaries.yml`; `ZIG_VERSION: 0.13.0` and `CARGO_ZIGBUILD_VERSION: 0.20.1` (`:31-32`); `mlugg/setup-zig@d1434d08867e3ee9daa34448df10607b98908d29 # v2.2.1` (`:159-161`, `:206-208`); glibc 2.17 enforced by `cargo zigbuild --target "${{ matrix.target }}.2.17"` plus a `readelf --version-info` ceiling check (`:171-180`); ARMv7 on `ubuntu-24.04` with `docker/setup-qemu-action@29109295f81e9208d7d86ff1c6c12d2833863392 # v3.6.0` and `platforms: arm` (`:196-203`); attestation job with `id-token: write`, `attestations: write`, `artifact-metadata: write` (`:331-336`) using `actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6 # v4.2.2` (`:360`); `gh release upload "$tag" dist/* --clobber` (`:395`). Live public `v0.4.1` confirmed: 7 binaries, 7 `.sha256` sidecars, `install.sh`, `install.ps1` = 16 assets. |
| §11.2 freeze the current Eggpack build/qualification/CI/staging interfaces | Frozen from source: `HostArch::{X86_64, Aarch64, Armv7}` (`eggpack-core/src/lib.rs:328-335`); `Qualification::{Native, DeferredNative, Emulated, Structural}` (`:339-355`); `Qualification::Structural => (QualificationMethod::Structural, false)` in `qualification.rs:682`; `BuildStrategy::CargoZigbuild` provisioning limited to Linux x86-64/AArch64 hosts (`ci/src/lib.rs:1506-1522`); `ZigOfficialArchiveV1` caller-supplied digests (`:481-496`); `zig_archive_name` constructing the new order (`:556-570`); runner labels free-form within `safe_runner_label` (`:1540-1545`); generated workflow rejects `id-token: write` (`:3772-3774`); staging reuses byte-identical assets and never clobbers. |
| §11.3 build the seven-target scratch contract/config | Built outside tracked files under `/tmp/eggpack-m003a-scratch`, ten files, then discarded. Inventory and digests in §Scratch inventory. **Not committed to Eggsearch** — `git status` in `eggsearch` is empty after the run. |
| §11.4 prove Windows ARM64 projection/render | Proven. `aarch64-pc-windows-msvc` renders three jobs, all on `runs-on: "windows-11-arm"`: build, qualify, and required consumer validation. Six runner mappings render, five distinct labels including `windows-11-arm`, `ubuntu-24.04-arm`, `macos-15-intel`, `macos-14`. |
| §11.5 adjudicate G1 with a rendered/tool-provisioning experiment | Adjudicated. Rendered with `zig = "0.13.0"`, `cargo_zigbuild = "0.20.1"`: Eggpack emits `https://ziglang.org/download/0.13.0/zig-x86_64-linux-0.13.0.tar.xz`, which is **HTTP 404**. Live probes: `zig-linux-x86_64-0.13.0.tar.xz` = 200, `zig-x86_64-linux-0.13.0.tar.xz` = 404, `zig-x86_64-linux-0.14.1.tar.xz` = 200. Gap real; Option A selected. |
| §11.6 adjudicate G2 with a bounded local/scratch consumer-validator experiment | Adjudicated by rendering both options. Option A renders with no `--qemu-sysroot` and with the ARMv7 consumer validator gating `required_gate` and `aggregate`. Option B renders and emits `--qemu-sysroot 'sysroots/linux-armv7'` but provisions neither QEMU nor the sysroot. |
| §11.7 adjudicate G3 with a no-mutation provenance workflow design/proof | Adjudicated at design level per §7. See §G3 adjudication. No attestation created; the live attestation API returned 404 for the available token scope, recorded honestly. |
| §11.8 confirm G4 no-clobber migration and operator recovery | Confirmed. The generated `stage` job requests only `contents: write`, contains no `--clobber`, and calls `_stage-github-draft`. Recovery procedure recorded in §G4 adjudication. |
| §11.9 produce a requirement-to-option matrix | See §G1-G4 option matrix. |
| §11.10 decide whether M003b can be authored now | **M003b can be authored now.** No prerequisite producer milestone is required. One prerequisite *process* step remains: the mirrored Eggsearch plan per planning process §9. |
| §11.11 write this closure and reconcile the roadmap/registry | This file; roadmap/registry/`AGENTS.md` reconciled in the same change. |
| §13 Eggpack local verification | `cargo fmt --all -- --check`, `cargo test -p eggpack-core` (46 passed / 4 ignored), `-p eggpack-ci` (60 / 3), `-p eggpack-cli` (22 + 8), `scripts/check-local.sh`, `git diff --check`. All green. |
| §13 scratch-specific assertions | All eight asserted; results in §Scratch render and check results. |
| §15.1 all seven targets render, or the exact unsupported interface identified | All seven render. No unsupported interface found. |
| §15.2 Windows ARM64 mapping proven or isolated as a blocker | Proven; not a blocker. |
| §15.3 one G1 toolchain strategy selected with compatibility rationale | Option A; rationale in §G1 adjudication. |
| §15.4 one G2 ARMv7 strategy selected with evidence-parity rationale | Option A; rationale in §G2 adjudication. |
| §15.5 one G3 strategy selected without weakening Eggpack's permission model | Option A; the permission model is untouched and asserted absent from the rendered output. |
| §15.6 G4 accepted with recovery semantics | Accepted; recovery in §G4 adjudication. |
| §15.7 scratch configuration removed after evidence capture | Removed. All five scratch trees and the temporary probe deleted after the inventory was recorded. |
| §15.8 no production/external state changed | Verified: `git status` clean in `eggsearch`; Eggpack's only diff is this closure and the planning/registry reconciliation; no `gh` write call; no attestation created. |
| §15.9 the closure states M003b readiness or exact prerequisites | Stated: M003b ready for plan authoring, mirrored Eggsearch plan required first, zero producer prerequisites. |
| §15.10 no unresolved architecture choice deferred into M003b implementation | Verified: G1-G4 all decided with evidence, and the one newly discovered constraint (E-1) is recorded here rather than deferred. |
| §16 no stop condition was met unaddressed | None of the seven stop conditions fired. See §Stop conditions. |
| §17 closure evidence contents | All twelve required items are present in this record. |

## Scratch inventory

Built under `/tmp/eggpack-m003a-scratch/release/eggpack/`, entirely outside any tracked consumer file, with `SCRATCH ONLY` in the header comment of each authored file. Digests recorded before removal:

| Scratch file | Bytes | sha256 (first 16) | Purpose |
|---|---|---|---|
| `distribution.toml` | 1599 | `730992d32acb5098` | Seven direct targets, `{product}-{target}` with the contract `.exe` rule, seven aliases |
| `pack.toml` | 1839 | `bfe74b9df528f224` | Build strategy, host, floor, and qualification intent per target; ARMv7 `structural` |
| `build-bindings.toml` | 874 | `c5b43e4210a4a8a8` | One direct Cargo output per target |
| `qualification-bindings.toml` | 1304 | `f4904846efe6f202` | Bounded `--version` smoke for the six executing targets; **none** for ARMv7 |
| `workflow-shape.json` | 8109 | `4a0b5c3bc4ddcb20` | Portable shape: seven targets, seven aliases, embedded bindings/validators, draft-staging intent |
| `github-policy.json` | 3106 | `9ed6e7c0d74be60f` | Six runner mappings including `windows-11-arm`; action pins; release inputs; staging policy; `cross_tools` |
| `consumer-validators.json` | 2022 | `8a7c2b878718be75` | Required Python3 validator per target |
| `github-template.json` | 387 | `3fcfa8a5a0ca31b1` | Draft template (copied from the proven stegoeggo set) |
| `install-policy.toml` | 297 | `fbef2160b2b97769` | Empty install policy (all direct artifacts) |
| `installer-presentation.json` | 278 | `0ad33c941b778a81` | `product_wrappers` presentation mode |
| rendered `release.yml` | 75723 | `e062cf8ee35fc375` | The rendered seven-target workflow (G2 Option A) |

Two additional variant trees were rendered and discarded: `eggpack-m003a-emulated` (G2 Option B, ARMv7 `emulated` plus `emulated_sysroots`) and `eggpack-m003a-g1` (Zig `0.13.0` / cargo-zigbuild `0.20.1`). A known-good render using the adopted stegoeggo inputs was run first to confirm the toolchain before trusting any failure.

## Seven-target matrix

Every row was rendered through the real resolver and renderer and asserted in the output.

| # | Target | Strategy | Host | Runner | Floor | Qualification | Required consumer validator | Smoke | Rendered |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `x86_64-unknown-linux-gnu` | CargoZigbuild | linux/x86_64 | `ubuntu-24.04` | glibc 2.17 | Native | yes | yes | yes |
| 2 | `aarch64-unknown-linux-gnu` | CargoZigbuild | linux/aarch64 | `ubuntu-24.04-arm` | glibc 2.17 | Native | yes | yes | yes |
| 3 | `armv7-unknown-linux-gnueabihf` | CargoZigbuild | linux/x86_64 | `ubuntu-24.04` | none | **Structural** (G2 Option A) | **yes — carries the runtime proof** | **none** | yes |
| 4 | `x86_64-apple-darwin` | NativeCargo | macos/x86_64 | `macos-15-intel` | none | Native | yes | yes | yes |
| 5 | `aarch64-apple-darwin` | NativeCargo | macos/aarch64 | `macos-14` | none | Native | yes | yes | yes |
| 6 | `x86_64-pc-windows-msvc` | NativeCargo | windows/x86_64 | `windows-latest` | none | Native | yes | yes | yes |
| 7 | `aarch64-pc-windows-msvc` | NativeCargo | windows/aarch64 | `windows-11-arm` | none | Native | yes | yes | yes |

All seven are `support = "required"`. The rendered workflow contains 27 jobs: preflight, resolve, 7 build, 7 qualify, 7 consumer-validate, `required_gate`, `aggregate`, `stage`.

The seven `selected_aliases` (`linux-x64`, `linux-arm64`, `linux-armv7`, `macos-x64`, `macos-arm64`, `windows-x64`, `windows-arm64`) resolve, and every expanded asset name was cross-checked against the published `v0.4.1` release:

```text
OK  x86_64-unknown-linux-gnu        -> eggsearch-x86_64-unknown-linux-gnu
OK  aarch64-unknown-linux-gnu       -> eggsearch-aarch64-unknown-linux-gnu
OK  armv7-unknown-linux-gnueabihf   -> eggsearch-armv7-unknown-linux-gnueabihf
OK  x86_64-apple-darwin              -> eggsearch-x86_64-apple-darwin
OK  aarch64-apple-darwin             -> eggsearch-aarch64-apple-darwin
OK  x86_64-pc-windows-msvc          -> eggsearch-x86_64-pc-windows-msvc.exe
OK  aarch64-pc-windows-msvc         -> eggsearch-aarch64-pc-windows-msvc.exe
```

That check used the Contract M003 command closed earlier in the same session (`eggpack contract expand --field asset`), which is incidental evidence that the two closed milestones compose: the preflight's authority question and the new CLI surface answer each other.

## Scratch render and check results

```text
eggpack ci generate --workflow-shape … --contract … --github-policy … --output release.yml
  generated 75723 bytes to release.yml                                  exit 0

eggpack ci check --workflow-shape … --contract … --github-policy … --workflow release.yml
  ci check: match (75723 bytes)                                         exit 0

re-render determinism: cmp release.yml release2.yml                    byte-identical
```

Acceptance assertions from §13, all run against the rendered output:

| Assertion | Result |
|---|---|
| `eggpack ci generate` for the seven-target shape | pass (75723 bytes) |
| `eggpack ci check` against the generated scratch workflow | pass (`match`) |
| Windows ARM64 build/qualification maps to the intended ARM64 runner | pass — `runs-on: "windows-11-arm"` on all three of its jobs |
| ARMv7 strategy represented exactly as the chosen G2 option | pass — `structural` classification, no `--qemu-sysroot`, required consumer validator gating `required_gate` and `aggregate` |
| No generated workflow requests `id-token: write` | pass — 0 occurrences |
| No generated workflow contains `--clobber` | pass — 0 occurrences |
| All seven target assets/checksum sidecars are projected | pass — 7 binaries + 7 sidecars + `install.sh` + `install.ps1` = 16, matching published `v0.4.1` exactly |

One deliberate negative control: the renderer was first driven with the *adopted* stegoeggo inputs and produced 55963 bytes successfully, so the toolchain was known-good before the scratch was trusted. A temporary probe test was then used once to surface the real library error behind the CLI's deliberately masked `reusable release rendering failed`, and **deleted** in the same pass; it is not part of this change.

## G1-G4 option matrix

| Gap | Option A | Option B | Selected | Deciding evidence |
|---|---|---|---|---|
| G1 Zig archive layout | Migrate to Eggpack's qualified pair (Zig `0.14.1`, cargo-zigbuild `0.23.3`), re-prove Linux output, glibc floor, and ARMv7 behaviour before cutover | Add bounded legacy official-Zig archive-layout support to Eggpack | **A** | `0.13.0` exists in exactly one Eggsearch file and in no public document; the guaranteed artefact is the glibc 2.17 floor, re-asserted post-build by `readelf`; Option B's digest is already caller-supplied, so it is *safe* but adds a producer milestone for an unpublished detail |
| G2 ARMv7 runtime | `Structural` core qualification + required Eggsearch-owned emulated consumer validator, pinned Docker/binfmt + ARMv7 image | New Eggpack emulation-runtime provisioning capability | **A** | Both render; Option B emits `--qemu-sysroot` while provisioning **nothing**, so it would fail at runtime; Option A keeps aggregation gated on the validator |
| G3 attestations | Eggpack workflow owns build/qualify/finalize/stage; a narrow Eggsearch workflow attests only the already-staged bytes | Block M003b on Eggpack Phase 12 provenance | **A** | Downloading staged draft assets is read-only and preserves exact subject identity, so it does not recreate producer release authority; no new Eggpack capability and no permission-model weakening |
| G4 clobber | Migrate to Eggpack's exact-reuse/refusal staging | Keep `--clobber` for parity | **A** | Eggpack's semantics are strictly stronger; parity is not a goal for a safety property |

## G1 adjudication — toolchain strategy

The gap is real and was measured rather than assumed. Eggpack's provisioner constructs the archive name as `zig-{arch}-linux-{version}.tar.xz` (`eggpack-ci/src/lib.rs:556-570`) and the rendered 0.13.0 workflow pointed at:

```text
https://ziglang.org/download/0.13.0/zig-x86_64-linux-0.13.0.tar.xz   -> 404
https://ziglang.org/download/0.13.0/zig-aarch64-linux-0.13.0.tar.xz  -> 404
```

Live probes against ziglang.org establish the actual layout change:

| URL | Status |
|---|---|
| `…/0.13.0/zig-linux-x86_64-0.13.0.tar.xz` | 200 |
| `…/0.13.0/zig-x86_64-linux-0.13.0.tar.xz` | **404** |
| `…/0.13.0/zig-linux-aarch64-0.13.0.tar.xz` | 200 |
| `…/0.13.0/zig-aarch64-linux-0.13.0.tar.xz` | **404** |
| `…/0.14.1/zig-x86_64-linux-0.14.1.tar.xz` | 200 |
| `…/0.14.1/zig-linux-x86_64-0.14.1.tar.xz` | **404** |

So "preserve 0.13.0 unchanged under current Eggpack provisioned mode" is indeed not representable, exactly as the plan predicted.

**Why Option A, and why this is not "silently selecting the easier option":** the plan required deciding whether the pin is product compatibility policy or implementation detail, and the evidence is unambiguous. `git grep -in "0.13.0" ec437cb` returns **one** file: `.github/workflows/release-binaries.yml`. It is not in `README.md`, not in `docs/release.md` (which promises only "a glibc 2.17 floor for Linux GNU artifacts"), not in any `plans/closure/**`, and not in any `architecture/*.md`. No user, installer, updater, or published document can observe which Zig produced the bytes.

What *is* observable is the glibc ceiling, and Eggsearch asserts it on the produced artefact, independently of the compiler: `readelf --version-info` scanned for any `GLIBC_` symbol above 2.17, failing the build otherwise. That assertion survives a toolchain change and is exactly what makes a bump requalifiable. The externally meaningful policy is the floor; the Zig version is the mechanism.

Two supporting facts that further weaken Option B's case. First, Eggsearch does not even manage this pin itself today — it is an input to the third-party `mlugg/setup-zig` action. Second, Option B is *safer* to implement than the plan assumed: `CrossToolProvisioningV1.zig` already requires caller-supplied SHA-256 digests (`ZigOfficialArchiveV1`), so a finite second official-layout mapping cannot weaken integrity and needs no arbitrary URL or template language. It is rejected because it buys an unpublished implementation detail at the price of a new producer milestone, not because it is unsafe or hard.

**Consequence for M003b, which must not be skipped:** before cutover, re-prove for both floored Linux targets and ARMv7 that the glibc 2.17 ceiling still holds and the ARMv7 artefact still executes, using the `readelf` ceiling assertion and the ARMv7 runtime proof from G2. That re-proof is work inside M003b, not a producer capability.

## G2 adjudication — ARMv7 evidence strategy

Eggpack core supports `Qualification::Emulated`, runs `qemu-arm` for ARMv7, accepts an explicit `-L <sysroot>`, and emits structured QEMU evidence — but it **provisions neither** QEMU nor the sysroot. Confirmed in the source (`qemu-arm` selection at `eggpack-core/src/qualification.rs:1046`; `--qemu-sysroot` threading at `eggpack-ci/src/lib.rs:3377`; the CLI merely validates that the given path "is a real directory", `eggpack-cli/src/main.rs:821`).

Both options were rendered to compare.

**Option B (`emulated` + `emulated_sysroots`)** renders 75763 bytes and emits:

```text
'eggpack' 'ci' '_qualify-target' … '--target' 'armv7-unknown-linux-gnueabihf' … '--qemu-sysroot' 'sysroots/linux-armv7'
```

and provisions **nothing**: 0 occurrences of `setup-qemu`, `apt-get install … qemu`, or any sysroot fetch in the rendered workflow. The renderer correctly *requires* an explicit runtime policy (`ci/src/lib.rs:3113-3123`, "never silently invoke M003 with an empty runtime") but a declaration is not a provisioning. Option B would therefore produce a workflow that fails at runtime unless the runner image happens to carry `qemu-arm` and the repository happens to contain that sysroot path. That is Option B as a prerequisite producer milestone.

**Option A (`structural` + required consumer validator)** renders 75723 bytes with:

- no `--qemu-sysroot` anywhere (structural classification executes nothing, so none is needed);
- a `validate_build_armv7_unknown_linux_gnueabihf` job running the required Python3 consumer validator;
- `required_gate` and `aggregate` both downloading ARMv7's qualification **and** consumer evidence, so a failed runtime proof fails the gate and blocks finalization.

That is the same release-gate strength Eggsearch has today: the runtime proof still gates the release, it simply moves from a workflow job body into the consumer-validator seam that Eggpack already gates on. No new Eggpack capability, no generalizing one consumer's emulation runtime prematurely.

**The constraint a migrating consumer will hit (E-1).** `QualificationBindingsV1::validate_for` (`eggpack-core/src/qualification.rs:105-112`) requires `smoke.is_some() == must_smoke`, where `must_smoke` is true only for `Native | DeferredNative | Emulated`. A `structural` target must therefore have **no** smoke binding. This is correct — a structural classification runs nothing, so a core smoke binding is incoherent — but the first scratch render failed with `qualification bindings do not cover ReleasePlan` because of it. M003b must omit the ARMv7 smoke entry; this is recorded here rather than deferred into implementation.

## G3 adjudication — attestation preservation

Current Eggsearch mixes producer and provenance authority in one job: the assemble job holds `contents: write`, `id-token: write`, `attestations: write`, and `artifact-metadata: write` together, attests `dist/eggsearch-*` plus `dist/install.sh` and `dist/install.ps1`, then creates/updates the draft and uploads with `--clobber`. Generated Eggpack workflows reject `id-token: write` outright (`ci/src/lib.rs:3772-3774`), and the rendered scratch workflow contains zero occurrences — asserted.

**Selected: Option A, at design level.** After cutover the Eggpack-generated workflow owns build, qualification, finalization, and draft staging; a separate narrow Eggsearch-owned workflow would:

1. accept an exact existing tag as an explicit dispatch input, never creating or moving a tag;
2. verify tag → source identity against the same release plan the producer used;
3. `gh release download "$tag"` the **exact already-staged draft assets** — a read-only operation that uploads nothing and replaces nothing;
4. verify each downloaded byte against the staged `release-manifest.json` digests before attesting, so a drifted asset cannot be attested;
5. run the pinned `actions/attest` with only `id-token`/`attestations`/`artifact-metadata` write;
6. `gh attestation verify` the result;
7. perform no upload, no replacement, and no publication.

**Subject identity is preserved**, which is the condition Option B exists to protect. `actions/attest` records the subject as the file basename plus its digest; `gh release download` writes each asset under its exact public asset name; and those names are already fixed by the producer contract. The attested digest therefore equals the staged digest equals the published digest. Reading an asset is not producer authority: it mutates nothing, and Eggpack's no-clobber staging remains the only writer.

**Evidence limits, stated honestly.** Per §7 this was a design pass and **no attestation was created**. The existing `v0.4.1` public release was confirmed as the prior-evidence anchor (16 assets, seven targets). The live attestation API returned HTTP 404 for the token available to this session, so the published attestation set could not be read back directly; that is recorded as finding E-2 rather than papered over. M003a does not authorize new attestations, and none were attempted.

## G4 adjudication — no-clobber migration

Confirmed by the rendered output rather than by reading documentation: the `stage` job requests only `contents: write`, contains no `--clobber`, and calls `ci _prepare-stage` then `ci _stage-github-draft`, which reuses byte-identical assets, fails closed on a digest mismatch, and never clobbers a differing public or draft asset.

**Accepted as a required safety improvement, not a parity item.** The operator recovery for an incomplete draft is short and does not require weakening anything:

1. the run fails closed at staging and leaves the draft as-is — nothing partial is published, and the finalized release artifact plus staging receipt remain available in the workflow run for diagnosis;
2. the operator deletes the incomplete draft release (a draft is not public state);
3. the operator re-runs the same workflow dispatch with the same exact tag. Because the tag and source revision are unchanged and Eggpack renders deterministically, the regenerated bytes are identical, so every asset is reused rather than replaced.

The one operational difference worth flagging to the M003b operator: `--clobber` would silently overwrite a stale asset, whereas Eggpack refuses. That refusal is the intended behaviour — it is exactly what prevents a rerun from mutating a release that already carries different bytes — and step 2/3 above is its whole cost.

## Production implementation evidence

None. M003a's §8 and §10 forbid it and this change honours both:

- `git status` in `eggsearch` is empty; no file was created, modified, or deleted in any consumer repository;
- the only Eggpack file changes are this closure, the plan header, and the roadmap/registry/`AGENTS.md` reconciliation — all under `plans/` and `AGENTS.md`;
- the temporary probe test used to unmask one library error was created and deleted within a single pass and is not part of the tree;
- no `gh` write call of any kind; every GitHub interaction was `gh release view` or `gh api` (read), and the ziglang.org interaction was HTTP HEAD.

## Verification executed

Eggpack local gate (`scripts/check-local.sh`), **exit 0**, plus the §13 set:

```text
cargo fmt --all -- --check                                              passed
cargo test -p eggpack-core --all-targets --all-features --locked        passed (46 passed, 4 ignored)
cargo test -p eggpack-ci   --all-targets --all-features --locked        passed (60 passed, 3 ignored)
cargo test -p eggpack-cli  --all-targets --all-features --locked        passed (22 + 8 passed)
./scripts/check-local.sh                                                exit 0
git diff --check                                                        passed
```

Scratch-specific verification is listed in §Scratch render and check results. External verification was read-only: `git show`/`git grep` at `ec437cb`; `git diff --stat ec437cb origin/main`; `gh release view v0.4.1 --repo eggstack/eggsearch`; `gh api repos/eggstack/eggsearch/attestations` (404); six `curl -I` probes against `ziglang.org`.

**Not performed, and therefore not claimed:** no hosted CI dispatch — §13 states none is required and no production change exists to qualify; no real build, qualification, consumer validation, or staging run for the seven-target shape; no attestation created or verified live; no consumer script was written (the ARMv7 validator is *named* in the scratch config but its content is Eggsearch-owned work that belongs to M003b); no macOS or Windows lane was exercised locally.

## Invariant review

| Invariant | Result |
|---|---|
| No Eggpack or Eggsearch production file changes in M003a | Held — planning/registry only. |
| No release/tag/package/attestation mutation | Held — no write call; attestation API read returned 404 and nothing was attempted. |
| No reduction from seven published targets | Held — all seven rendered, all `required`. |
| No loss of glibc 2.17 ceilings for the two floored Linux targets | Held — both floored targets carry `floor = { kind = "glibc", major = 2, minor = 17 }` in the scratch. **Re-proof after the G1 bump is an explicit M003b prerequisite**, recorded in §G1. |
| No loss of ARMv7 runtime execution evidence | Held — execution moves to a required consumer validator that gates `required_gate` and `aggregate`. |
| No loss of Windows ARM64 | Held — rendered on `windows-11-arm` for build, qualify, and validate. |
| No loss of current public installer/update semantics | Held — `installer-presentation.json` `product_wrappers` and `install-policy.toml` carried through; the rendered stage job prepares and stages the wrapper set. |
| No loss of existing Artifact Attestation coverage | Held by design — §G3 preserves it without new Eggpack capability; live re-attestation is explicitly unauthorized here. |
| No `--clobber` added to Eggpack | Held — asserted absent from the rendered workflow. |
| No generic arbitrary workflow/setup hook language | Held — every scratch input uses existing closed schemas; nothing new was invented. |
| No automatic publication | Held — the stage job creates/reuses a draft; publication remains a human action. |
| Product self-update/startup/service policy remains Eggsearch-owned | Held — untouched; no Eggpack surface reaches it. |
| Eggpack-owned target/artifact/build/finalization/staging authority not duplicated after cutover | Held by design — the G3 seam is read-only and attests; it never writes, uploads, or stages. |
| No runtime/product policy moved from the consumer into Eggpack | Held — the ARMv7 runtime proof and the provenance seam are both product-owned by construction. |
| Scratch files never committed to Eggsearch | Held — built under `/tmp`, discarded, `eggsearch` status empty. |
| A discovered gap gets a plan, not opportunistic code | Held — E-1 is recorded as a constraint for M003b; no code was written for it. |

## Failure/recovery review

- **A failed scratch render was evidence, not permission to weaken anything** (§12). The first render failed; the CLI's diagnostic is deliberately coarse (`reusable release rendering failed`), so a throwaway probe surfaced the real cause, `qualification bindings do not cover ReleasePlan`, which led to the E-1 schema rule. The scratch was corrected to match the schema, not the schema relaxed.
- **A negative control preceded the experiment.** The renderer was driven with adopted stegoeggo inputs first and produced 55963 bytes, so a later failure could be attributed to the scratch rather than the toolchain.
- **No external state to roll back.** Every interaction was a read or a HEAD. The scratch trees and probe were deleted after the inventory was captured; nothing survives outside this record.
- **No contention.** The seven-target scratch used only local files and local renders; no hosted run, no shared cache, no service.
- **Recovery for the eventual M003b cutover** is captured in §G4 and does not require weakening the no-clobber invariant.

## Compatibility and migration review

- **Consumer-visible surface is unchanged.** M003a changed nothing, so there is nothing to migrate for.
- **The migration M003b will perform, and what it costs.** Seven targets, a glibc-floor re-proof after the toolchain bump, an ARMv7 validator rewrite into the consumer-validator seam (with the E-1 smoke omission), a draft-assembly split out of the assemble job, and a no-clobber staging cutover. Every one of those is consumer-side work plus the existing producer pipeline.
- **No dual authority.** The decisive requirement for the G3 seam is that it be read-only; it is, so cutover leaves exactly one writer per fact.
- **Rollback shape.** Because nothing changed, rollback is the absence of a change. Once M003b exists, its rollback is the same as M001/M002: Eggsearch keeps its hand-maintained workflow as the pre-cutover state, and publication stays a human action throughout.

## Security review

No new attack surface: no code, no workflow shipped, no permission granted, no credential used, no token read beyond the already-authenticated `gh` session's read paths.

Reviewed properties of the *proposed* M003b shape, since that is what this milestone authorizes:

- **Least privilege improves.** Today one Eggsearch job holds `contents: write` + `id-token: write` + `attestations: write` + `artifact-metadata: write`. After cutover the staging job holds only `contents: write`, and provenance permissions move to a separate job that never writes release state.
- **Provenance cannot bless a mutated asset.** The narrow workflow must verify each downloaded byte against the staged `release-manifest.json` before attesting, so attestation is downstream of a digest check rather than a substitute for one.
- **Integrity remains distinct from authenticity.** Eggpack asserts SHA-256 and size only. The attestation seam adds provenance *about* bytes that are already digest-pinned; it does not convert a digest into an authenticity claim, and no ADR was needed because this milestone changes no trust model.
- **No supply-chain expansion.** The G1 decision *reduces* third-party dependency exposure: today the build consumes `mlugg/setup-zig`, whereas Eggpack's provisioned mode fetches official `ziglang.org` archives by pinned SHA-256 with bounded timeouts.
- **No arbitrary URL, mirror, or setup-command surface** is introduced by the selected options.
- **Publication remains a human action** under both the old and new flows.

## Documentation and operations evidence

Updated by this closure:

- `plans/subsystems/ecosystem-adoption-roadmap.md` — M003a closed; §11 completion and the M003b readiness/blocker state updated; the four decisions recorded as settled rather than open;
- `plans/registry.md` — the M003a row, the ecosystem execution graph, the planned/blocked paragraph, and the remaining-ready-plan handoff;
- `AGENTS.md` — current-handoff paragraph;
- this closure record.

Not changed: canonical long-term documents `plans/000`-`plans/002`, `plans/003-planning-process.md`, every ADR, every architecture deep dive, and every historical closure record. **No long-term direction was contradicted by anything discovered**, which is why §14 required no canonical edit: all four gaps were resolved with existing producer capability plus product-owned seams.

The `architecture/` deep dives were not touched because M003a changed no source file, so no `file:line` citation could have rotted.

Operations impact: none. No command, artifact, configuration, or operator procedure changed.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| Medium (E-1) | `Qualification::Structural` **forbids** a core smoke binding: `validate_for` requires `smoke.is_some() == must_smoke` and `must_smoke` excludes `Structural` (`eggpack-core/src/qualification.rs:105-112`). A migrating consumer that copies its per-target smoke block verbatim will fail with the opaque `qualification bindings do not cover ReleasePlan`. | Not a defect — the rule is correct, since a structural classification executes nothing. Recorded here as an explicit M003b requirement: the ARMv7 entry in `qualification-bindings.toml` must carry no `smoke` block, and the ARMv7 runtime proof belongs entirely in the required consumer validator. Worth a migration note in the M003b plan; **not** worth an Eggpack code change, because changing the rule would let a consumer declare a smoke that never runs. |
| Low (E-2) | The live attestation API returned HTTP 404 for the token available to this session, so the published `v0.4.1` attestation set could not be read back and compared subject-by-subject. | Recorded rather than worked around. §7 already forbids creating new attestations in M003a, so the missing read does not block any decision here. The G3 design rests on the existing workflow's own contract (`actions/attest` `subject-path` semantics) plus the confirmed `v0.4.1` asset set, not on a live attestation read. If M003b's cutover needs subject-level parity proof, it must obtain a token with attestation read scope; that is a verification step for M003b, not a gap in this decision. |
| Low (E-3) | The G1 decision to bump Zig `0.13.0` → `0.14.1` and cargo-zigbuild `0.20.1` → `0.23.3` has **not** been validated against Eggsearch's actual build graph. Whether 0.14.1 produces a 2.17-floor artefact for both GNU targets and an ARMv7 artefact that still runs is unproven. | This is expected: M003a forbids implementing or running Eggsearch builds. It is recorded as a hard M003b prerequisite rather than a residual question, with the exact re-proof obligation named in §G1. If the bump *does* break the floor, M003b must stop and return here — that is precisely the plan's §16 stop condition, and this closure keeps it armed rather than assuming it away. |
| Low (E-4) | The scratch `consumer-validators.json` and shape *name* an ARMv7 validator script (`scripts/release-smoke.py`) that was never written; the rendered workflow references a path that does not exist in the scratch. | Expected and harmless for a shape-level preflight — `ci generate`/`ci check` validate the *declared* seam, not the script's existence, which is checked at run time by `_validate-consumer`. The validator's content is Eggsearch-owned work for M003b. Noted so nobody reads the render as a claim that a validator exists. |
| Informational | The plan's external baseline `ec437cb` is four documentation commits behind `origin/main` `33f508d`. | Not material: the drift touches only `README.md`, `AGENTS.md`, `architecture/`, `docs/`, and `skills/`. All release-relevant facts were verified at `ec437cb` itself, not at the tip. |
| Informational | Option B for G1 was found to be *safer to implement* than the plan assumed, because the Zig archive digest is already caller-supplied. | Recorded so the rejection is understood as a scope judgement rather than a technical one. If Eggsearch later treats a specific Zig version as a compatibility requirement, Option B becomes the right answer and belongs in a CI-subsystem plan, not in the adoption roadmap. |
| None | No unresolved medium-or-higher defect in Eggpack. | The only Medium finding is a consumer-migration constraint with a documented answer, not a producer gap. |

## Stop conditions

None of §16's seven conditions fired:

| Stop condition | Status |
|---|---|
| Preserving Zig 0.13.0 requires arbitrary URL/template injection | Not reached — Option B was not needed, and even it would require only a finite validated layout distinction over an already caller-supplied digest. |
| Changing to 0.14.1/0.23.3 changes required compatibility evidence in a way not safely requalifiable **inside adoption** | **Not yet determined, and deliberately left armed.** The change is requalifiable in principle because the guarantee is a post-build artefact assertion, but it is unproven against Eggsearch's real build graph (E-3). M003b must perform that re-proof and stop if it fails. |
| ARMv7 runtime proof cannot be retained as a bounded product-owned validator | Not reached — retained, via the required consumer validator. |
| Preserving attestation evidence requires generated build/stage jobs to request OIDC permissions | Not reached — asserted zero `id-token: write` in the rendered output; the seam is a separate read-only job. |
| Windows ARM64 cannot map through current host/runner policy | Not reached — proven on `windows-11-arm`. |
| Adoption would require keeping two active build/staging authorities | Not reached — one writer per fact. |
| Any existing seven-target release guarantee would be dropped | Not reached. |

## Roadmap disposition

| Milestone | Status after M003a | Reason |
|---|---|---|
| Ecosystem M003a | closed | This record. G1-G4 decided with evidence; seven targets rendered; no producer prerequisite required. |
| Ecosystem M003b (Eggsearch adoption) | **ready for plan authoring** | No producer blocker remains. Blocked on one *process* step only: a mirrored implementation plan registered in `eggstack/eggsearch` before any external edit, per planning process §9. |
| Contract M003 / Bootstrap M003 | unaffected, closed | Neither depends on this milestone. Contract M003's CLI was incidentally exercised to cross-check asset names (§Seven-target matrix). |
| Ecosystem M001 / M002, CI M003b | unchanged, closed | Untouched; no closure rewritten. |

## Downstream handoff

M003b may now be authored. Before any external edit, the following must exist:

1. **A mirrored implementation plan in `eggstack/eggsearch`**, registered in that repository, covering the consumer-side work: adopting the producer contract and config set, moving the ARMv7 runtime proof into a required consumer validator (with the E-1 smoke omission), splitting provenance out of the assemble job, and switching draft assembly to no-clobber staging.
2. **The glibc 2.17 re-proof after the G1 toolchain bump**, for both floored Linux targets and ARMv7, using the `readelf` ceiling assertion. If it fails, M003b stops and returns to M003a (E-3).
3. **An attestation-subject parity check** at cutover, with a token that can read attestations (E-2).

One option is explicitly **not** authorized and is recorded so it is not smuggled in: generalized producer provenance. If the narrow product-owned seam proves insufficient in practice, that becomes Eggpack Phase 12 work with its own plan — not an adoption detail.

## Registry updates

- Ecosystem M003a row: `ready` -> `closed`, closure pointer `plans/closure/ecosystem-adoption/003a-status.md`.
- Ecosystem subsystem roadmap: M003a closed; the four decisions recorded as settled; M003b readiness and its single remaining process blocker recorded.
- `plans/registry.md`: subsystem table row, the M003a row in the milestone index, the planned/blocked paragraph, the execution graph, and the "one plan is ready" handoff.
- `AGENTS.md`: current-handoff paragraph.

## Eggstack references

- Closed source plan: `plans/implementation/ecosystem-adoption/003a-eggsearch-seven-target-compatibility-preflight-and-migration-design.md`
- Roadmap: `plans/subsystems/ecosystem-adoption-roadmap.md`
- Predecessor closures (historical, untouched): `plans/closure/ecosystem-adoption/001-status.md` (eggsact `v1.2.7`), `plans/closure/ecosystem-adoption/002-status.md` (stegoeggo `v0.5.0`)
- Milestones closed earlier in the same session and composed with here: `plans/closure/bootstrap-installers/003-status.md`, `plans/closure/contract-conformance/003-status.md`
- Eggpack capability anchors: `crates/eggpack-core/src/qualification.rs` (`validate_for`, `Structural`, `Emulated`, `qemu-arm`), `crates/eggpack-ci/src/lib.rs` (`zig_archive_name`, `ZigOfficialArchiveV1`, `safe_runner_label`, id-token rejection, emulated-sysroot requirement, staging job), `crates/eggpack-cli/src/main.rs` (`_qualify-target` sysroot validation)
- External repository read only: `eggstack/eggsearch@ec437cb` and published release `v0.4.1`. No external commit, branch, release, tag, or attestation state was created or changed.