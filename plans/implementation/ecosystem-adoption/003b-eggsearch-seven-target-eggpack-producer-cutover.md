# Ecosystem Adoption Milestone 003b — Eggsearch Seven-Target Eggpack Producer Cutover

Status: ready

Class: capability / cross-repository adoption / operational qualification

Roadmap: `plans/subsystems/ecosystem-adoption-roadmap.md`

Eggpack planning baseline: `8d880c4097e3d883ce3a5e7a5a0509f77852302e`

External consumer baseline: `eggstack/eggsearch@33f508d87b9623b785bd151f46779f01b2409eb3`

Predecessor closure:

- `plans/closure/ecosystem-adoption/003a-status.md`

Required mirrored consumer plan:

- `eggstack/eggsearch: plans/implementation/eggpack-release-adoption/001-seven-target-eggpack-producer-cutover.md`
- plan commit: `eggstack/eggsearch@377f9e8ae6c762e34dad77793f2c0c3069e9b41a`
- registry commit: `eggstack/eggsearch@6767062aa24c402308fa388a932bd3384d6a7e90`

Long-term references:

- `plans/000-long-term-specification.md#23-consumer-adoption-requirement`
- `plans/002-long-term-roadmap.md#phase-11--broader-native-adoption`
- `plans/003-planning-process.md`

Applicable ADRs:

- ADR-0001 producer/consumer release boundary
- ADR-0003 checked-in generated CI + explicit publication
- ADR-0004 first-party Cargo/cargo-zigbuild adapter
- ADR-0005 host-matched native qualification for cross-tool builds

## 1. Objective

Migrate Eggsearch's seven-target producer release authority from its hand-maintained release workflow to the already-qualified Eggpack contract/build/qualification/finalization/staging pipeline without reducing any existing release guarantee.

The cutover must preserve:

- seven public binary targets;
- glibc 2.17 compatibility for all three GNU Linux builds;
- ARMv7 runtime execution evidence;
- Windows ARM64 native coverage;
- public product wrapper behavior;
- binary-first updater compatibility;
- GitHub Artifact Attestation coverage;
- explicit human publication;
- immutable published releases.

The cutover intentionally replaces Eggsearch's current clobber-capable draft assembly with Eggpack's exact-reuse/fail-closed staging.

No Eggpack production-code change is expected or authorized by this plan. Eggpack is the producer authority being adopted; the implementation work is consumer-owned in the paired Eggsearch plan.

## 2. Readiness

M003a closed all architecture questions and found no producer prerequisite.

Proven by M003a:

- all seven targets resolve/render through current Eggpack;
- Windows ARM64 maps through `windows-11-arm`;
- Zig 0.13.0 is an implementation pin, not a public compatibility contract;
- Eggsearch may move to the already-qualified Zig 0.14.1 / cargo-zigbuild 0.23.3 pair;
- ARMv7 may use `Qualification::Structural` plus a required product-owned consumer validator;
- generated Eggpack jobs remain free of OIDC write permissions;
- provenance may remain in a separate Eggsearch-owned read-only workflow;
- Eggpack no-clobber staging supersedes `gh release upload --clobber`.

The paired Eggsearch plan is registered at `eggstack/eggsearch@377f9e8ae6c762e34dad77793f2c0c3069e9b41a` and its control-surface registration is `6767062aa24c402308fa388a932bd3384d6a7e90`. The process gate is therefore satisfied and M003b is `ready`.

## 3. Release-contract delta

The existing Eggsearch public release contract is exactly 16 files:

- 7 binaries;
- 7 checksum sidecars;
- `install.sh`;
- `install.ps1`.

Eggpack `ProductWrappers` staging additionally publishes:

- `release-manifest.json`;
- `install-exact.sh`;
- `install-exact.ps1`.

Therefore the post-cutover exact public inventory is **19 files**.

This is an intentional additive compatibility change, not accidental drift.

It is acceptable only if:

- existing 16 names remain byte/semantic compatible;
- updater and public wrapper resolution remain unchanged;
- packaging guards/docs are updated from exact 16 to exact 19;
- no consumer assumes unknown extra release assets are forbidden;
- the new exact installers and manifest are documented as producer evidence, not replacement public entry points.

If preserving the existing 16-file contract becomes a hard consumer requirement, stop. Current Eggpack wrapper presentation cannot silently omit the three producer-evidence files and must not be changed opportunistically inside adoption.

## 4. Producer/consumer ownership

### Eggpack owns

- `DistributionContract`;
- target/asset/checksum naming;
- PackConfig + build bindings;
- qualification graph;
- final-byte `ReleaseManifest`;
- deterministic cross-tool provisioning;
- generated release workflow;
- exact generated installers;
- draft staging and exact reuse/refusal semantics;
- staging receipt.

### Eggsearch retains

- product wrapper policy;
- crates.io-first publication sequencing;
- version/tag selection;
- Cargo fallback policy;
- self-update semantics;
- service/startup behavior;
- ARMv7 runtime validation script;
- provenance/attestation workflow;
- manual draft publication.

No fact may have two active writer authorities after cutover.

## 5. Exact toolchain migration

Linux cross builds migrate from:

- Zig 0.13.0;
- cargo-zigbuild 0.20.1;

to the Eggpack-qualified pair:

- Zig 0.14.1;
- cargo-zigbuild 0.23.3.

Use the already-qualified official Zig archive digests:

- Linux x86-64: `24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c`;
- Linux AArch64: `f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b`.

Eggsearch MUST not keep `mlugg/setup-zig` as a parallel build authority after the generated workflow cutover.

## 6. GNU Linux compatibility proof

Before deleting the old workflow, the real generated build graph must prove the glibc 2.17 ceiling for:

- `x86_64-unknown-linux-gnu`;
- `aarch64-unknown-linux-gnu`;
- `armv7-unknown-linux-gnueabihf`.

For each produced binary:

1. run `readelf --version-info`;
2. collect `GLIBC_<n>` requirements;
3. fail if any requirement exceeds 2.17;
4. record the exact binary SHA-256 tied to the proof.

This proof is mandatory after the toolchain bump.

If any target exceeds the 2.17 ceiling, stop M003b and return to M003a rather than weakening the documented compatibility floor.

## 7. ARMv7 qualification

ARMv7 producer configuration:

- build: `CargoZigbuild`;
- core qualification: `Structural`;
- **no core smoke binding**;
- required product-owned Python3 consumer validator.

The no-smoke rule is mandatory because `Qualification::Structural` correctly rejects a declared core smoke that would never execute.

The consumer validator must:

- receive only the exact Eggpack-qualified candidate path;
- assert 32-bit ARM ELF identity;
- assert glibc <= 2.17;
- execute `--version` and `--help` under the pinned ARMv7 runtime;
- use a pinned/immutable runtime-image identity rather than a floating tag where practical;
- remain within Eggpack validator timeout/output bounds;
- fail closed if Docker/QEMU/runtime prerequisites are unavailable.

If the validator cannot retain the existing ARMv7 runtime guarantee without adding a generic setup-command hook to Eggpack, stop.

## 8. Windows ARM64

Use current Eggpack host/runner policy:

- target `aarch64-pc-windows-msvc`;
- build/qualification host Windows/AArch64;
- runner `windows-11-arm`;
- native smoke/consumer validation as applicable.

The cutover must not reduce Windows ARM64 from a required release target.

## 9. Provenance/Artifact Attestation seam

Eggpack-generated jobs retain their current least-privilege model and MUST NOT request:

- `id-token: write`;
- `attestations: write`;
- `artifact-metadata: write`.

Eggsearch owns a separate provenance workflow with only the permissions needed to attest already-staged draft bytes.

Required shape:

1. explicit exact tag input or a provenance-specific trigger tied to the completed staging run;
2. checkout/verify exact tag commit;
3. require the GitHub Release to exist and still be a draft;
4. download the exact staged inventory read-only;
5. verify each downloaded file against the GitHub Release asset's reported SHA-256 digest;
6. verify the seven binary/checksum pairs;
7. verify `release-manifest.json` release/source/target identity and its artifact digests;
8. verify public wrapper bytes equal the checked-in wrapper sources;
9. attest at minimum the existing provenance subject set: seven binaries + `install.sh` + `install.ps1`;
10. SHOULD also attest the two newly public generated exact installers;
11. verify resulting attestations with `gh attestation verify`;
12. mutate no tag, release metadata, or release asset.

The provenance workflow is an authenticity layer over already-staged bytes. It does not become a second staging/build authority.

### Subject parity gate

Before cutover closure, use credentials that can read the existing `v0.4.1` attestations and record the subject-name/digest set.

The new flow must cover every previously attested public subject. Additive attestation of `install-exact.sh` / `install-exact.ps1` is allowed.

If existing subject parity cannot be established because of GitHub token/API limitations, M003b cannot fully close; record a named operational condition rather than claiming parity.

## 10. Draft staging and recovery

Generated staging replaces `gh release upload --clobber`.

Required properties:

- exact existing tag only;
- draft-only;
- exact source revision;
- no auto-publish;
- absent asset -> upload;
- identical asset -> reuse;
- differing asset -> fail;
- unexpected remote asset -> fail;
- published/immutable release -> fail.

Operator recovery from a mismatched/incomplete draft:

1. inspect the refusal and draft inventory;
2. delete the stale draft only when intentionally restarting the unpublished release candidate;
3. rerun the same exact tag/source after confirming the draft is absent.

Never add `--clobber` to Eggpack.

## 11. Consumer configuration set

The paired Eggsearch implementation must check in a complete `release/eggpack/` authority set, using the same closed interface families proven by Eggsact/StegoEggo and M003a scratch work.

At minimum:

- distribution contract;
- pack config;
- build bindings;
- qualification bindings;
- consumer validators map;
- workflow shape;
- GitHub provider policy/runner mappings;
- deterministic cross-tool provisioning policy;
- install policy;
- installer presentation using product wrappers;
- draft template/staging policy;
- generated checked-in release workflow.

All checked-in producer facts must drift-check through `eggpack ci check`.

## 12. Existing consumer paths to preserve

The migration MUST preserve behavior of:

- `packaging/install.sh`;
- `packaging/install.ps1`;
- `src/platform.rs` host mapping;
- `src/update.rs` binary-first update;
- crates.io version authority;
- exact asset 404/unsupported-host Cargo fallback;
- service restart semantics;
- default-feature single-SKU release;
- macOS unsigned status;
- current seven target public names.

Product wrappers remain the public friendly install surface.

## 13. Ordered work packages

1. Revalidate both repository baselines and paired-plan registration.
2. Land Eggsearch `release/eggpack/` producer configuration.
3. Generate the checked-in Eggpack release workflow and make `eggpack ci check` a drift guard.
4. Implement ARMv7 consumer validator.
5. Implement separate provenance workflow.
6. Update Eggsearch packaging contract from exact 16 to exact 19 assets.
7. Update packaging/docs/architecture/static guards for the new authority and inventory.
8. Run local Eggsearch gate.
9. Run an Eggpack-generated **qualification** dispatch on the exact candidate, with no release mutation.
10. Prove glibc 2.17 on all three GNU targets and ARMv7 runtime execution.
11. Prove Windows ARM64 and remaining native targets.
12. Exercise draft staging on a maintainer-authorized exact candidate/tag only when release prerequisites are satisfied.
13. Exercise provenance on the exact staged bytes; prove subject parity.
14. Prove rerun exact reuse/no-clobber.
15. Keep publication manual.
16. Write closures in both repositories and reconcile registries.

## 14. Local verification

Eggpack:

```bash
scripts/check-local.sh
git diff --check
```

Eggsearch:

```bash
make check
make release-check
git diff --check
```

Additionally:

- `eggpack ci generate`;
- `eggpack ci check`;
- repeated generation byte-identical;
- exact target/asset inventory comparison against the seven-row Eggsearch contract;
- static assertion that generated workflow contains no `--clobber` and no OIDC write permission;
- packaging guard updated for exact 19-file post-cutover inventory.

## 15. Hosted qualification requirements

Before cutover closure, an exact candidate must show:

- seven required build targets green;
- seven core qualification outcomes green;
- ARMv7 required consumer validation green;
- required gate green;
- aggregate/finalization green;
- exact 19-file staging payload;
- no automatic publication.

A live draft-staging run is required before deletion of the old release writer.

Publication itself remains a separate maintainer action and is not required merely to prove the implementation diff, but full Ecosystem M003b closure requires one real maintainer-authorized release or an explicitly named operational condition if the user chooses not to publish yet.

## 16. Compatibility/migration sequence

Do not delete the old workflow first.

Safe sequence:

1. add Eggpack config + generated workflow alongside old workflow;
2. qualify the exact candidate through Eggpack;
3. qualify ARMv7/provenance seams;
4. compare target names, binary/checksum bytes, installers, and updater mapping;
5. only after evidence parity, retire the old workflow as a release writer;
6. retain product-owned provenance workflow;
7. stage/publish the first Eggpack-produced Eggsearch release;
8. verify public installers/updater/attestations;
9. close M003b.

At no point may two workflows independently mutate the same release draft.

## 17. Acceptance criteria

M003b closes only when:

1. paired plans exist and were followed;
2. Eggsearch producer authority is checked in under `release/eggpack/`;
3. generated workflow is drift-checked;
4. all seven target names remain unchanged;
5. public inventory transition 16 -> 19 is documented/guarded and only additive;
6. Zig 0.14.1 / cargo-zigbuild 0.23.3 builds satisfy glibc <= 2.17 for all three GNU targets;
7. ARMv7 runtime proof passes through the required consumer validator;
8. Windows ARM64 remains required and passes;
9. product wrappers retain current fallback/update semantics;
10. exact generated installers and release manifest are staged as the three additive producer-evidence files;
11. staging is draft-only, no-clobber, exact-reuse, exact-tag/source;
12. generated jobs request no OIDC write permission;
13. provenance subject parity with `v0.4.1` is recorded, or closure is explicitly conditional on a named read-scope evidence gap;
14. provenance workflow mutates no release/tag/asset state;
15. local gates and required hosted qualification are green;
16. no medium-or-higher regression remains;
17. old hand-maintained release workflow no longer owns build/stage authority after cutover;
18. publication remains human-controlled;
19. closure records exact commits/run IDs/release evidence in both repositories.

## 18. Stop conditions

Stop and return to planning if:

- the paired Eggsearch plan materially disagrees with this plan;
- the toolchain bump violates glibc 2.17;
- ARMv7 runtime proof cannot run through the bounded consumer-validator seam;
- Windows ARM64 cannot execute under current runner mapping;
- exact 19-file additive inventory is not acceptable to Eggsearch;
- provenance requires OIDC permissions in Eggpack-generated jobs;
- provenance cannot prove subject identity without becoming a second release writer;
- generated staging needs clobber/overwrite;
- consumer updater/install semantics regress;
- a new Eggpack production primitive becomes necessary.

## 19. Closure evidence

Create `plans/closure/ecosystem-adoption/003b-status.md` recording:

- Eggpack implementation/planning baseline;
- paired Eggsearch plan/implementation commits;
- exact 7-target matrix;
- 16 -> 19 inventory evidence;
- toolchain pins/digests;
- glibc proof per GNU target;
- ARMv7 validator evidence;
- Windows ARM64 evidence;
- generated-workflow drift proof;
- staging receipt/rerun proof;
- provenance subject-parity evidence;
- installer/update smoke;
- publication state;
- old-writer retirement evidence;
- unresolved findings and final disposition.
