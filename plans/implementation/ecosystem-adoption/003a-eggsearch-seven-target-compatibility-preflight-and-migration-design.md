# Ecosystem Adoption Milestone 003a — Eggsearch Seven-Target Compatibility Preflight and Migration Design

Status: ready

Eggpack repository implementation baseline: `911da48c7c1c7967397a2d190730fc643c8c6dc3`

External consumer baseline: `eggstack/eggsearch@ec437cb87c2b97a835a13377838c34631acad63b`

Source roadmap: `plans/subsystems/ecosystem-adoption-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#23-consumer-adoption-requirement`
- `plans/002-long-term-roadmap.md#phase-11--broader-native-adoption`
- `plans/003-planning-process.md`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`
- `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`
- `plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md`

Primary class: planning / compatibility evidence / migration design

## 1. Objective

Resolve the remaining architecture and compatibility questions for Ecosystem M003 before any Eggsearch migration is authorized.

Eggsearch is the first intended consumer that materially extends the proven direct-binary shape:

- seven release targets rather than five;
- Linux ARMv7 with emulated runtime evidence;
- Windows ARM64;
- an older cross-tool pair than Eggpack's currently qualified provisioning example;
- existing GitHub Artifact Attestation evidence;
- a hand-maintained draft workflow that currently uses clobber-on-rerun.

M003a must prove which parts map directly onto existing Eggpack capabilities, which product-owned compatibility seams can remain in Eggsearch, and whether any new Eggpack producer primitive is actually required.

M003a does not modify Eggsearch, does not change Eggpack production code, and does not authorize Ecosystem M003b adoption.

Its output is a closure-backed migration design precise enough to either:

1. authorize a bounded M003b Eggsearch adoption plan plus a mirrored Eggsearch plan; or
2. register the exact prerequisite Eggpack capability milestone(s) and leave M003b blocked.

## 2. Why this is ready

The ordering dependency is closed:

- Ecosystem M001 Eggsact closed on public `v1.2.7`;
- Ecosystem M002 StegoEggo closed on public `v0.5.0`;
- CI M003b and Phase 8 are closed;
- current direct build/finalization/staging interfaces are qualified.

Eggsearch also has stable public release evidence to compare against:

- package/release version `0.4.1`;
- seven-target release workflow;
- immutable public release and provenance evidence;
- exact public installers;
- published SLSA/GitHub Artifact Attestation evidence recorded by its own closure.

The remaining uncertainty is architectural fit, not missing predecessor evidence.

## 3. Current Eggsearch release shape

At the external baseline, the hand-maintained release workflow covers:

1. `x86_64-unknown-linux-gnu`;
2. `aarch64-unknown-linux-gnu`;
3. `armv7-unknown-linux-gnueabihf`;
4. `x86_64-apple-darwin`;
5. `aarch64-apple-darwin`;
6. `x86_64-pc-windows-msvc`;
7. `aarch64-pc-windows-msvc`.

Current evidence/policy includes:

- glibc 2.17 floor on Linux x86-64/AArch64;
- Zig `0.13.0`;
- cargo-zigbuild `0.20.1`;
- ARMv7 built with cargo-zigbuild and executed under a Docker/QEMU path;
- Windows ARM64 native hosted runner;
- `--version` / `--help` smoke;
- exact checksum sidecars;
- product installers and Cargo fallback;
- GitHub Artifact Attestations generated and verified before publication;
- draft release assembly;
- current draft rerun behavior uses `gh release upload ... --clobber`.

M003b may not weaken any of those externally meaningful guarantees merely to fit Eggpack.

## 4. Confirmed existing Eggpack capability

### 4.1 Seven-target identity

Current types already represent:

- `HostArch::Armv7`;
- `HostArch::Aarch64`;
- PE/COFF AArch64 candidate identity;
- ARMv7 ELF identity;
- Windows/AArch64 host requirements and arbitrary safe GitHub runner labels.

Windows ARM64 is therefore not, by itself, a new architecture requirement.

### 4.2 Cross-tool builds

`CargoZigbuild` is already supported with explicit glibc floors and deterministic provisioning.

The provisioner currently has qualified evidence around the Eggpack pair Zig `0.14.1` + cargo-zigbuild `0.23.3`, but policy carries explicit version/digest data rather than a product-global hard-coded target matrix.

### 4.3 Emulated qualification primitives

Core qualification already supports:

- `Qualification::Emulated`;
- ARMv7 -> `qemu-arm`;
- explicit `-L <sysroot>`;
- bounded preflight and candidate smoke;
- structured QEMU evidence.

The GitHub renderer requires an explicit repository-relative `emulated_sysroots` entry and passes it to the CLI.

It does **not** provision QEMU or a sysroot. That distinction is central to this preflight.

### 4.4 Consumer validation

A required per-target Python3 consumer validator runs after core qualification and gates aggregation.

The validator:

- receives the exact candidate only after qualification evidence is `Passed`;
- executes shell-free as `python3 <script> <candidate>`;
- may contain product-owned subprocess logic;
- is bounded by timeout/output limits;
- records exact candidate identity and pass/fail evidence.

This may provide a migration-compatible place for Eggsearch-specific ARMv7 Docker/QEMU evidence if using core `Qualification::Structural` plus a required consumer validator is proven equivalent or stronger at the release-gate level.

### 4.5 Staging

Eggpack staging already:

- creates/reuses an exact draft;
- reuses byte-identical assets;
- fails closed on digest mismatch;
- never clobbers a differing public/draft asset;
- never publishes automatically.

Migration MUST prefer this stronger no-clobber behavior over Eggsearch's current `--clobber` implementation.

## 5. Confirmed compatibility gaps/questions

### G1 — Zig 0.13.0 archive layout does not match the current deterministic provisioner

Eggsearch currently pins Zig `0.13.0`.

The official Zig 0.13.0 Linux archives use the historical filename order:

```text
zig-linux-x86_64-0.13.0.tar.xz
zig-linux-aarch64-0.13.0.tar.xz
```

Current Eggpack provisioning constructs the newer order:

```text
zig-x86_64-linux-<version>.tar.xz
zig-aarch64-linux-<version>.tar.xz
```

Therefore "preserve Eggsearch 0.13.0 unchanged under current Eggpack provisioned mode" is not yet proven representable.

M003a MUST compare two bounded choices:

**Option A — migrate Eggsearch to Eggpack's currently qualified pair**

- Zig `0.14.1`;
- cargo-zigbuild `0.23.3`;
- re-prove all Linux target output/GLIBC-floor/ARMv7 behavior before cutover.

**Option B — add bounded legacy official-Zig archive-layout support in Eggpack**

- preserve `0.13.0` / `0.20.1`;
- no arbitrary download URL/template language;
- support only a finite, validated official archive-layout distinction;
- requires its own producer capability plan before M003b.

Do not silently select Option A merely because it is easier. Record whether the toolchain pin itself is product compatibility policy or simply current implementation detail.

### G2 — generated CI does not provision the ARMv7 QEMU runtime

Core `Qualification::Emulated` is executable only when the runtime already provides:

- `qemu-arm`;
- the declared sysroot directory.

The generated GitHub job currently does not install either.

M003a MUST compare:

**Option A — product-owned required consumer validator**

- classify ARMv7 core qualification as `Structural`;
- retain the current emulated execution as a required Eggsearch-owned Python validator;
- validator uses an immutable/pinned Docker/binfmt + ARMv7 runtime image strategy;
- aggregation remains gated on the validator;
- closure proves evidence is equal or stronger than the current workflow.

This avoids generalizing one consumer's emulation runtime prematurely.

**Option B — new Eggpack emulation-runtime provisioning capability**

- keep ARMv7 as core `Qualification::Emulated`;
- add a finite provider policy for deterministic QEMU/runtime provisioning;
- must not become arbitrary setup-command injection;
- requires a separate Eggpack plan before M003b.

If Option A cannot preserve the current runtime proof without unsafe privilege/network ambiguity, choose Option B and stop M003b.

### G3 — GitHub Artifact Attestations are current release evidence but Eggpack forbids OIDC write permission

Eggsearch's current release workflow requests:

- `id-token: write`;
- `attestations: write`;
- `artifact-metadata: write`;

and generates/verifies Artifact Attestations.

Current Eggpack generated release workflows intentionally reject `id-token: write`.

M003a MUST not solve this by weakening the Eggpack permission invariant inside adoption.

Compare:

**Option A — transitional Eggsearch-owned provenance workflow**

- Eggpack-generated workflow owns build/qualification/finalization/draft staging;
- a separate narrow Eggsearch workflow owns only attesting the exact already-staged asset bytes;
- no build, target-matrix, checksum, tag mutation, upload replacement, or publication authority;
- exact tag/source/digest linkage must be proven;
- maintainer publishes only after provenance success.

This preserves existing evidence while leaving generalized producer provenance for Phase 12.

**Option B — block M003b on Eggpack Phase 12 provenance**

Use only if the narrow product-owned compatibility seam cannot preserve exact subject identity without recreating producer release authority.

Do not add generic attestation support during M003a.

### G4 — current clobber semantics

Eggsearch currently updates draft assets with `--clobber`.

Eggpack's exact reuse/refusal semantics are already stronger and proven.

M003a should treat migration to no-clobber as a required safety improvement, not a parity blocker, provided release operator recovery remains clear for an incomplete draft.

## 6. Required scratch projection

M003a MUST build a complete seven-target **scratch** Eggpack configuration outside tracked consumer files and drive it through the real current Eggpack resolver/renderer.

The scratch must cover at least:

- distribution contract;
- pack config;
- build bindings;
- qualification bindings;
- workflow shape;
- GitHub runner policy;
- cross-tool policy;
- consumer-validator map;
- installer presentation;
- draft template/staging policy.

No scratch file may be committed to Eggsearch.

### Target intent to test

Linux x86-64:

- CargoZigbuild;
- glibc 2.17;
- native qualification.

Linux AArch64:

- CargoZigbuild;
- glibc 2.17;
- matching AArch64 native qualification.

Linux ARMv7:

- CargoZigbuild;
- current release does not declare a glibc floor separate from its target/runtime proof;
- test the selected G2 strategy;
- if Option A, Structural core qualification + required emulated consumer validation must gate aggregation.

macOS x86-64/AArch64:

- NativeCargo;
- native qualification.

Windows x86-64/AArch64:

- NativeCargo;
- native qualification;
- explicit x86-64 and ARM64 runner mappings.

## 7. Attestation compatibility experiment

Without mutating a release, determine whether a narrow Eggsearch-owned provenance workflow can:

1. accept an exact existing tag;
2. verify tag -> source identity;
3. obtain exact staged draft assets without replacing them;
4. verify their checksums/manifest identity;
5. invoke the pinned GitHub attestation action with only provenance-specific permissions;
6. verify one or more resulting attestations;
7. perform no GitHub Release publish/upload replacement action.

If live attestation would create durable external state, stop at static/workflow design and use the existing `v0.4.1` attestation evidence plus GitHub action contract as prior evidence. M003a does not authorize new attestations.

## 8. Invariants

- No Eggpack or Eggsearch production file changes in M003a.
- No release/tag/package/attestation mutation.
- No reduction from seven published targets.
- No loss of GLIBC 2.17 ceilings for the two currently floored Linux targets.
- No loss of ARMv7 runtime execution evidence.
- No loss of Windows ARM64.
- No loss of current public installer/update semantics.
- No loss of existing Artifact Attestation coverage unless a later explicit architecture decision authorizes it.
- No `--clobber` added to Eggpack.
- No generic arbitrary workflow/setup hook language.
- No automatic publication.
- Product self-update/startup/service policy remains Eggsearch-owned.
- Eggpack-owned target/artifact/build/finalization/staging authority must not be duplicated after M003b cutover.

## 9. In scope

- read-only Eggpack/Eggsearch source and closure review;
- local scratch config/render;
- local renderer/validation tests;
- exact option analysis G1-G4;
- architecture/ownership decision record in the M003a closure;
- roadmap/registry status updates;
- authoring follow-on plans only after the decisions are evidence-backed.

## 10. Out of scope

- editing Eggsearch;
- editing Eggpack production code;
- dispatching a live Eggsearch release;
- publishing packages/releases;
- generating new attestations;
- accepting an authenticity trust model;
- implementing legacy Zig provisioning;
- implementing QEMU runtime provisioning;
- performing M003b adoption.

## 11. Ordered work packages

1. Freeze Eggsearch current release/config/install/provenance baseline.
2. Freeze current Eggpack build/qualification/CI/staging interfaces.
3. Build the seven-target scratch contract/config.
4. Prove Windows ARM64 projection/render.
5. Adjudicate G1 with a rendered/tool-provisioning experiment.
6. Adjudicate G2 with a bounded local/scratch consumer-validator experiment where feasible.
7. Adjudicate G3 with a no-mutation provenance workflow design/proof.
8. Confirm G4 no-clobber migration and operator recovery semantics.
9. Produce a requirement-to-option matrix.
10. Decide whether M003b can be authored now or which prerequisite producer plan(s) are required.
11. Write `plans/closure/ecosystem-adoption/003a-status.md` and reconcile the roadmap/registry.

If M003b becomes ready, the closure MUST require a mirrored implementation plan in `eggstack/eggsearch` before external edits, per planning process §9.

## 12. Failure/restart semantics

M003a is read-only/scratch.

A failed scratch render or unsafe compatibility option is evidence, not permission to weaken an invariant.

Discard scratch files after recording bounded results.

Do not leave a half-authored external migration branch.

## 13. Verification

Eggpack local:

```bash
cargo fmt --all -- --check
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
./scripts/check-local.sh
git diff --check
```

Scratch-specific:

- `eggpack ci generate` for the seven-target shape;
- `eggpack ci check` against the generated scratch workflow;
- assert Windows ARM64 build/qualification job maps to the intended ARM64 runner;
- assert ARMv7 strategy is represented exactly as the chosen G2 option;
- assert no generated workflow requests `id-token: write`;
- assert no generated workflow contains `--clobber`;
- assert all seven target assets/checksum sidecars are projected.

No hosted dispatch is required for M003a closure unless an explicitly non-mutating qualification workflow already exists and is separately authorized.

## 14. Documentation

Update only planning/architecture status needed to record the decision.

Do not change canonical long-term direction unless a discovered requirement truly conflicts with it.

If G1/G2 requires a new producer feature, create the owning subsystem plan and reference it from Ecosystem M003b's blocker instead of burying requirements in the adoption roadmap.

## 15. Acceptance criteria

M003a closes when:

1. all seven targets render or the exact unsupported interface is identified;
2. Windows ARM64 mapping is proven or isolated as a concrete blocker;
3. one G1 toolchain strategy is selected with compatibility rationale;
4. one G2 ARMv7 qualification strategy is selected with evidence-parity rationale;
5. one G3 attestation-preservation strategy is selected without weakening Eggpack's permission model;
6. G4 no-clobber migration is accepted with recovery semantics;
7. scratch configuration is removed after evidence capture;
8. no production/external state changed;
9. the closure states either:
   - M003b ready for plan authoring, plus mirrored Eggsearch plan required; or
   - exact prerequisite Eggpack milestone(s) required before M003b;
10. no unresolved architecture choice is deferred into M003b implementation.

## 16. Stop conditions

Stop and register a prerequisite plan if:

- preserving Zig 0.13.0 requires arbitrary URL/template injection;
- changing to Zig 0.14.1/cargo-zigbuild 0.23.3 changes required compatibility evidence in a way not safely requalifiable inside adoption;
- ARMv7 runtime proof cannot be retained as a bounded product-owned validator;
- preserving Artifact Attestation evidence requires generated Eggpack build/stage jobs to request OIDC permissions;
- Windows ARM64 cannot map through current host/runner policy;
- adoption would require keeping two active build/staging authorities;
- any existing seven-target release guarantee would be dropped.

## 17. Closure evidence

Create `plans/closure/ecosystem-adoption/003a-status.md` recording:

- exact Eggpack and Eggsearch baselines;
- seven-target matrix;
- scratch config inventory;
- render/check results;
- G1-G4 option matrix and selected decisions;
- toolchain/provisioning findings;
- ARMv7 evidence strategy;
- Windows ARM64 evidence;
- attestation preservation strategy;
- no-clobber migration result;
- whether any producer prerequisite plan was registered;
- M003b readiness/blocker;
- external mirrored-plan requirement;
- unresolved findings by severity.
