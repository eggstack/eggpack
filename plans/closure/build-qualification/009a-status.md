# Build/Qualification Milestone 009a Closure — Local Producer Operator Defects

Status: closed

Source plan: `plans/implementation/build-qualification/009a-local-producer-operator-defects.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md` (registered there)

Triggered by: the 2026-10-23 post-implementation review of the M009 chain.

Reviewed baseline: `e0e1510` (the tip of the review chain), on top of M009's implementations `eb77d19` and `8950eb9`.

## Executive finding

Build/Qualification 009a is closed. The six local producer operator commands no longer destroy their own input, are equivalent to the CI path they stand in for, and report `manifest: ok` only when the manifest they write describes the finalized bytes.

The milestone's central claim was sound and was independently confirmed during this review: the operator family is thin wiring over the same core surfaces the CI path uses, and its output is byte-identical to the CI path's. Every defect found was in the wiring, not in the design.

## D1 (HIGH) — `qualify` destroyed its own input candidates

`stage_qualification_artifacts` staged each candidate with a hard link and, when that failed, a plain `std::fs::copy`. With `--output-dir` equal to `--artifacts-dir`, the source and destination were the same path: the hard link failed with `EEXIST`, and the copy fallback opened the destination with `O_TRUNC`, truncating the shared inode before copying zero bytes.

The command reported failure — but the input was already destroyed:

```text
BEFORE: 436496 bytes  sha256 20c5bdf73a7cad7992f3d68762b9578c5d9082719f99fc006d8ad90855319925
$ eggpack qualify --artifacts-dir out/build --output-dir out/build
eggpack: staged qualification artifact validation failed   (exit 1)
AFTER:        0 bytes  sha256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

The audit found the same `hard_link`-then-`copy` pattern on two CI paths that could also collide: `ci _qualify-target` (`--candidate-dir X` with `X/candidates` present, against `--output-dir X`) and `ci _capture-build` (`--candidate-dir X/candidates` against `--output-dir X`).

### Production implementation evidence

- A shared `copy_candidate_bytes` helper refuses a destination that canonicalizes to the source, prefers a hard link, and falls back to copy-into-a-fresh-temp-then-rename. The Windows replace path mirrors the existing `atomic_write` move-aside/restore.
- All four staging sites route through it: `stage_qualification_artifacts`, `ci_capture_build`, `write_build_artifacts`, and the operator's `qualify`.
- The operator's `qualify` refuses `--output-dir == --artifacts-dir` **before any write**, so the candidate bytes are untouched.

### Verified after the correction

```text
BEFORE: size=436496 sha=20c5bdf73a7cad7992f3d68762b9578c5d9082719f99fc006d8ad90855319925
$ eggpack qualify --artifacts-dir out/build --output-dir out/build
eggpack: output dir must differ from artifacts dir        (exit 1)
AFTER:  size=436496 sha=20c5bdf73a7cad7992f3d68762b9578c5d9082719f99fc006d8ad90855319925
```

## D2–D5 (MEDIUM) — the operator family was not equivalent to the CI path

- **D2 — `--qemu-sysroot`.** Added to the operator's `qualify`, validated as a real directory that is not a symlink, and threaded into `QualificationRuntime`. The core's existing rule — the sysroot is valid only for `Emulated` qualification — applies identically to both surfaces. An emulated target no longer runs QEMU without a sysroot by omission.
- **D3 — `--identity-mode` and `--consumer-validators`.** Added to the operator family. `collect` now uses the consumer-aware projection and `aggregate_finalize_with_consumer`, so a consumer-validated or v-prefixed-semver release can be planned and aggregated locally. The "consumer evidence present without a configured validator" refusal now applies only when no validator is configured, which is the correct scope for it.
- **D4 — JSON input.** `read_strict_document` reads TOML first and then the JSON document, normalizing the JSON into canonical document shape and re-reading it with the **same strict core parser**. One validator now covers both encodings, so the two paths cannot drift.
- **D5 — the derived manifest is self-checking.** `manifest` re-projects the release graph and reads each archive member back out of the finalized `.tar.gz`, comparing its size and digest with the candidate bytes; it also compares the derived document with the collected `release-manifest.json` when that file is present beside the finalized root. Any mismatch fails closed and writes nothing.

## D6–D10 (LOW)

- **D6.** `collect` now reports `aggregation failed: {cause}` instead of discarding the reason. It was the only operator error path not carrying a diagnostic.
- **D7.** `absolutize_cli_path` applies one rule to both path forms. `/abs/../x` is now refused like `../x`; the previous behaviour accepted the absolute form.
- **D8.** `parse_selected` refuses empty elements with `selected target aliases must not be empty`, and the unreachable branch is gone.
- **D9.** `manifest` forwards `graph.finalization.evidence_references` instead of hardcoding an empty vector, and it re-projects the same graph `collect` projects — so the two commands cannot emit different documents.
- **D10.** The README's "prints exactly one line on stderr" claim now states that it holds for the operator commands and the `ci` runners, and that the top-level dispatch prints the usage block. The prior claim was false: `eggpack` with no args emits 15 stderr lines and `eggpack bogus` emits 16.

## Test evidence

`crates/eggpack-cli/tests/operator_commands.rs` gained twelve tests, including:

- `qualify_refuses_to_stage_into_its_own_artifacts_directory` — asserts the candidate bytes are **unchanged** and that no evidence was written.
- `ci_qualify_target_refuses_to_stage_into_its_own_candidate_directory`.
- `qualify_accepts_a_qemu_sysroot_and_matches_the_ci_runner`.
- `plan_writes_the_same_bytes_as_ci_resolve_release_in_v_prefixed_mode`.
- `consumer_validated_release_plans_and_aggregates_locally`.
- `operator_reads_json_input_documents`.
- `manifest_refuses_a_tampered_artifacts_directory`.
- `manifest_refuses_a_collected_manifest_that_differs`.
- `collect_names_the_aggregation_failure_reason`.
- `check_rejects_an_empty_selected_alias`.
- `the_top_level_dispatch_prints_the_usage_block`.

## Verification

```text
$ cargo fmt --all -- --check
(clean)

$ cargo clippy -p eggpack-cli --all-targets --all-features --locked -- -D warnings
(clean)

$ cargo test -p eggpack-cli --all-targets --all-features --locked
65 passed; 0 failed   (27 unit + 8 contract_expand + 30 operator_commands)

$ cargo check --workspace --all-targets --locked
(clean)
```

Both byte-identity tests still pass: `plan_writes_the_same_bytes_as_ci_resolve_release` and `package_qualify_collect_manifest_matches_ci_aggregate`. The latter's `manifest` invocation gained the two newly required binding flags; its byte assertions are unchanged.

The quickstart's steps 1–7 were re-run verbatim in a scratch repository: `check/plan/package/qualify/collect/manifest: ok`, and `diff out/release/release-manifest.json out/manifest/release-manifest.json` reports the two documents identical.

## Invariants re-checked

- No command destroys, truncates, or overwrites its own input. Staging refuses a same-file destination and never opens the source for writing.
- The operator family reproduces every qualification and aggregation the CI path can, and refuses explicitly where it cannot. No silent weakening remains.
- The operator family and the CI path accept the same input encodings and the same identity and consumer-validator surface.
- A derived manifest describes the finalized bytes, or the command fails closed. It never reports `manifest: ok` for a manifest it cannot prove.
- Diagnostics name the actual cause.
- The byte-identity property with the CI path is preserved.
- CLI commands still distinguish validation, build, staging, and publication. No operator command stages or publishes; staging and publication remain separately authorized actions.
- No panic on untrusted input.

## Known limits of this correction

- The archive cross-check adds tar/gzip reading to the CLI crate. It is read-only and streaming, but it is new logic in a crate that is otherwise thin wiring.
- The end-to-end emulated qualification path is not covered end to end here, because no emulated host or QEMU is available on the review machine. The sysroot is validated and threaded; the test asserts parity with the CI runner's construction rather than a real emulated run.
- `manifest` now requires `--build-bindings` and `--qualification-bindings`, because it must re-project the same graph `collect` projects. The command family is unpublished, so this is a surface change rather than a published break.

## Historical record

M009's own closure record (`plans/closure/build-qualification/009-status.md`) remains as written. The defects it did not cover are recorded here and in the M009a plan; no history was rewritten.
