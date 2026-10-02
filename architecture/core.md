# `eggpack-core` — Deep Dive

Producer-side planning, qualification, local finalization, and manifest
aggregation. Four source files: `src/lib.rs` (1056 lines, planning + manifest
aggregation), `src/builder.rs` (1301 lines, build seam),
`src/qualification.rs` (2610 lines, qualification),
`src/finalization.rs` (860 lines, finalization).

## Pipeline

```text
DistributionContract + PackConfig + release_id + source_revision + selected
 -> PackConfig::resolve() -> ReleasePlan (sorted canonical triples)
 -> BuildBindingsV1::validate_for + execute_target_cancellable -> BuildAttempt
 -> QualificationBindingsV1::validate_for + qualify_target -> QualificationEvidence
 -> FinalizationRequest -> finalize_release -> build_manifest -> ReleaseManifest v1
```

Identity (`release_id`/`source_revision`/`target`/`strategy`) threads every stage;
mismatches fail closed. Note: `execute_target_cancellable` does **not** call
`BuildBindingsV1::validate_for` internally — callers must validate separately
(contrast `qualify_target_for_host_with_runner`, which validates both binding
sets internally).

## Key types / functions

**`src/lib.rs` — planning + aggregation:**

- `PackConfig{schema_version:1, targets:Vec<TargetPolicy>}` +
  `PackConfig::from_toml()`, `PackConfig::resolve(contract, release_id,
  source_revision, selected)`. Pure, deterministic, side-effect free.
- `TargetPolicy{target, strategy, host_os, host_arch, qualification_host?,
  toolchain, floor, qualification, support}`;
  `BuildStrategy::NativeCargo | CargoZigbuild`;
  `HostOs::Linux | Macos | Windows`; `HostArch::X86_64 | Aarch64 | Armv7`;
  `CompatibilityFloor::None | Glibc{major,minor} | Macos{major,minor}`;
  `Qualification::Native | DeferredNative | Emulated | Structural`;
  `SupportTier::Required | NonGating | Experimental`.
  `strategy` (how bytes are built) and `qualification` (how those exact bytes are
  proved) are independent axes: `validate_policy()` admits `Native` for either
  strategy exactly when the effective qualification host matches the target
  OS/arch, keeps the `NativeCargo` rejection of cross-tool versions, and keeps
  the floor applicability rules.
- `ReleasePlan{schema_version, release_id, source_revision, targets}` +
  `to_json()`; `PlannedTarget{target, policy, artifact_form}`;
  `PlannedAssetForm::Direct | Bundle | Archive`.
- `build_manifest(contract, input)`; `FinalizedReleaseInput{product_id,
  release_id, source_revision, targets, evidence_references}`;
  `FinalizedTargetInput{target, release_files, archive_members}`;
  `CoreError`, `digest_file()`, `validate_policy()`, `host_matches_target()`.

**`src/builder.rs` — bindings/command/execution:**

- `LogicalOutputSelector::Direct | BundleEntry{index} | ArchiveMember{source}`;
  `BuildBinding{selector, package, binary}`;
  `BuildBindingsV1{schema_version:1, targets}` + `from_toml()`,
  `validate_for(contract, plan)` (exact coverage, no extras). Internal
  `validate_shape()` is private (runs inside `from_toml`/`validate_for`).
  `BoundCommand` is defined but never constructed — flow uses `CommandSpec`
  directly via `cargo_command()`.
- `CommandSpec{executable, args, cwd, env, timeout, stdout/stderr limits}`;
  `CommandOutcome::Success | Failed | TimedOut | Cancelled |
  OutputLimitExceeded`; `ProcessEvidence` (byte counts, not contents);
  `BuildCancellation`.
- `CandidateArtifact{target, selector, package, binary, path, size}`;
  `BuildAttempt{release_id, source_revision, target, strategy, tool_summary,
  process, candidates}`.
- `cargo_command(...)` — `cargo +<rust> build|zigbuild --release --locked
  --target <triple[.glibc-floor]> --package --bin`, `CARGO_TARGET_DIR`-only env;
  `preflight()` (rustc/cargo/zigbuild/zig versions, substring match);
  `run_bounded_cancellable()`; `execute_target[_cancellable]`;
  `private_target_dir()` (sanitized via `safe_component()`, marker file,
  reuse rejected); `discover_candidate()` (exact
  `<target_dir>/<triple>/release/<bin>[.exe]`, regular non-empty non-symlink
  with parent-dir + containment checks). Stops at candidate bytes.

**`src/qualification.rs` — bindings/evidence/execution:**

- `CandidateSmokeBinding{selector, argv, timeout_ms, stdout/stderr limits}`;
  `TargetQualificationBinding{smoke?}`;
  `QualificationBindingsV1{schema_version:1, targets}` + `from_toml()`,
  `validate_for(plan, build_bindings)` (smoke iff Native/DeferredNative/Emulated;
  DeferredNative requires `qualification_host`). Internal `validate_shape()`
  is private.
- `QualificationRuntime{qemu_sysroot?}`;
  `QualificationMethod::Native | Deferred | DeferredNativeOnNativeHost |
  QemuUser | Structural`;
  `QualificationStatus::Passed | Deferred | Failed(QualificationFailure)`;
  `CandidateFormat::Elf | PeCoff | MachO`;
  `CandidateArchitecture::X86_64 | Aarch64 | Armv7`;
  `QualifiedCandidateEvidence{selector, package, binary, size, sha256, format,
  architecture}`; `QualificationEvidence{schema_version:1, release_id,
  source_revision, target, planned_classification, method, actual_host, support,
  status, candidates, smoke_selector?, processes}` + `validate_for(...)`.
- `QualificationRequest{contract, plan, target, attempt, build_bindings,
  qualification_bindings, runtime, cancellation}`;
  `qualify_target()`, `qualify_target_for_host[_with_runner]()`;
  `inspect_candidate()`, `parse_binary_header()`, `qemu_for_target()`
  (Linux/ELF-only: x86_64/aarch64/armv7 → `qemu-*`), `make_evidence()`.
  Hashes bytes before/after execution; build success ≠ qualification.

**`src/finalization.rs` — gating/copy/archive/manifest:**

- `ArchiveEncoding::TarGzip` only;
  `FinalizationTargetInput{target, attempt, qualification}`;
  `FinalizationRequest{product_id, release_id, source_revision, targets,
  evidence_references, archive_encoding?}`;
  `FinalizedRelease{root, manifest}`.
- `finalize_release(contract, plan, request, output_root)`: gates on
  identity-matched evidence (`Required` must be `Passed`;
  `NonGating|Experimental` may be `Deferred`), copies direct/bundle candidates
  under contract names, assembles `TarGzip` (normalized mtime/uid/gid/mode,
  contract member names), writes `{digest}  {asset}` sidecars from final bytes,
  aggregates `ReleaseManifest v1` (artifact digests = final bytes, member
  digests = archive-input bytes). Rejects mixed release/source, missing/extra
  inventory, `.tar.gz` suffix mismatch, pre-existing output root; failure removes
  only the owned root and returns no manifest. Unix `0700` output.

## Boundaries / safety

- Authority separation: contract owns names/layout; config owns policy; plan is
  intent; manifest owns final digests. Config cannot redefine artifact names.
- Finite/enumerated, `deny_unknown_fields`, `schema_version == 1`, bounded
  counts (targets/bindings ≤256, args ≤128, capture ≤256KiB), no shell, no
  generic command/plugin DSL, no network client (cross-tool provisioning is
  `ready`-check only, not install).
- Path/process safety: absolute repo root / pre-existing work root, symlink
  rejection, canonical-containment, per-invocation `target/` dirs +
  `.eggpack-owner`, pre/post hash + size equality, `env_clear()` + allowlists,
  `command-group` process-group kill, typed timeout/cancel/limit outcomes.
- Qualification gates: `Native` requires the effective qualification host
  (`qualification_host`, else the build host) to match the target OS/arch and is
  independent of `BuildStrategy`, so a `CargoZigbuild` candidate may be
  qualified natively on a matching host (including a separate AArch64
  qualification host for a cross-built candidate); it still requires a bounded
  smoke binding, and a non-matching host yields a failed `HostMismatch` record
  rather than a pass or a skip. Off-host `DeferredNative` → `Deferred` (never
  `Passed`); `Emulated` fixed `qemu-* --version` preflight; `Structural` never
  executes.
- Producer rule: `BuildStrategy` decides how candidate bytes are constructed;
  `Qualification` decides how those exact bytes are proved. A declared
  `CompatibilityFloor` is a build-policy input carried into the cross-tool build
  command — native execution on a matching host proves the candidate runs there,
  it is not independent proof that the produced binary honours the declared
  minimum glibc/macOS runtime.
- Producer-side file construction only: no extract/install/publish/authenticate.

## Dependencies / dependents

- Deps (`Cargo.toml:14-23`): `eggpack-contract`, `eggpack-manifest`, `serde`
  (derive), `serde_json`, `sha2`, `toml`, `command-group 5.0.1`, `tar`
  (no default features), `flate2`.
- Dependents: `eggpack-ci`, `eggpack-github`, `eggpack-cli`.
