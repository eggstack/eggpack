# Build/Qualification M009a Corrective — Local Producer Operator Defects

Status: **closed**.

Subsystem: Build and qualification.

Predecessor: `plans/implementation/build-qualification/009-local-producer-operator-commands.md` (M009, implementation `eb77d19` and `8950eb9`, closure `plans/closure/build-qualification/009-status.md`).

Triggered by: the 2026-10-23 post-implementation review of the M009 chain, which found ten defects in the newly written operator command family — one of which destroyed the operator's own input bytes.

M009's closure record remains historical and is not rewritten. This corrective is a new `NNNa` plan, per the repository's closure discipline.

## 1. Objective

Make the six local producer operator commands safe to run and faithful to the CI path they stand in for.

The milestone's design was right: the operator family is thin wiring over the same core surfaces the CI path uses, and the byte-identity property it claimed was real. The defects were in the wiring, and they are the kind a command surface acquires when it is written quickly and exercised only on the happy path.

## 2. The defects

### 2.1 D1 (HIGH) — `qualify` destroys its own input candidates

`stage_qualification_artifacts` staged each candidate with a hard link and, when that failed, a plain `std::fs::copy`. When the caller's `--output-dir` equals the `--artifacts-dir`, the source and the destination are the same path. The hard link fails with `EEXIST`; the copy fallback opens the destination with `O_TRUNC`, truncating the shared inode, and then copies zero bytes.

The command then fails with `staged qualification artifact validation failed` — but the input candidate bytes are **already gone**.

Observed:

```text
BEFORE: 436496 bytes  sha256 20c5bdf73a7cad7992f3d68762b9578c5d9082719f99fc006d8ad90855319925
$ eggpack qualify --artifacts-dir out/build --output-dir out/build
eggpack: staged qualification artifact validation failed   (exit 1)
AFTER:        0 bytes  sha256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

This is a data-destruction defect on the command's own input, reachable by a plausible operator mistake. The audit found that the same `hard_link`-then-`copy` pattern existed on two CI paths that could also collide (`ci _qualify-target` and `ci _capture-build`).

### 2.2 D2–D5 (MEDIUM) — the operator family was not equivalent to the CI path

- **D2.** `qualify` hardcoded `qemu_sysroot: None`. `Qualification::Emulated` maps to `QualificationMethod::QemuUser`, and `qemu_args` prepends `-L <sysroot>` only when the sysroot is present. An emulated target qualified through the operator therefore ran QEMU **without** the sysroot — silently weaker evidence than the CI path, with no way to supply one.
- **D3.** The operator family had no `--identity-mode` and no `--consumer-validators`. `collect` hard-failed with "consumer evidence present without a configured validator" whenever consumer evidence existed, so a consumer-validated or v-prefixed-semver release could not be planned or aggregated locally at all — while the README claimed that `collect` "projects the same release graph the CI path projects".
- **D4.** The operator's input readers were TOML-only, while the CI path accepts the same documents as TOML **or** JSON.
- **D5.** For an archive target, `manifest` resolved each member's bytes from the caller-supplied `--artifacts-dir` and hashed them, with nothing cross-checking those bytes against the archive actually in the finalized tree. A stale or tampered `--artifacts-dir` produced a manifest whose `archive_members` digests did not describe the finalized release — and the command reported `manifest: ok`.

### 2.3 D6–D10 (LOW)

`collect` discarded the aggregation failure reason; `absolutize_cli_path` rejected `..` in relative paths but accepted it in absolute paths; `parse_selected` had an unreachable branch that made `--selected ","` fail with a confusing downstream message; `manifest` hardcoded `evidence_references` instead of forwarding the projected graph's; and the README's "prints exactly one line on stderr" claim was false for the top-level dispatch, which prints the usage block.

## 3. Invariants

- **No command may destroy, truncate, or overwrite its own input.** Staging a candidate MUST refuse a destination that is the source, and MUST NOT open the source for writing.
- The operator family MUST be able to reproduce every qualification and aggregation the CI path can, or MUST refuse explicitly. A silent weakening is not acceptable.
- The operator family and the CI path MUST accept the same input encodings and the same identity and consumer-validator surface.
- A derived manifest MUST describe the finalized bytes. Where the operator cannot prove that, it MUST fail closed rather than report success.
- A command's diagnostic MUST name the actual cause.
- No panic on untrusted input; no `unwrap`/`expect` on user input in production paths.
- The byte-identity property with the CI path MUST be preserved.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-cli/**`, `docs/quickstart.md`, `architecture/cli.md`.

**Out of scope:**

- The core surfaces the operator calls. Every needed function was already public; the operator is wiring.
- The CI crate's own implementations. The CI-path staging sites that shared the destruction pattern are fixed in place, but the CI crate's rules and renderers are unchanged.
- The `eggpack-manifest`, `eggpack-contract`, `eggpack-github`, and `eggpack-bootstrap` crates.
- Any change to `ReleaseManifest` v1 or `schema_version`.

## 5. Required production changes

1. A shared staging helper refuses a destination that canonicalizes to the source, prefers a hard link, and falls back to copy-into-a-fresh-temp-then-rename. Every staging site — the operator's `qualify` and `package`, the CI's `_qualify-target` and `_capture-build` — routes through it. The operator's `qualify` additionally refuses `--output-dir == --artifacts-dir` before any write.

2. `--qemu-sysroot` is added to the operator's `qualify`, validated as a real directory, and threaded into `QualificationRuntime`.

3. `--identity-mode` and `--consumer-validators` are added to the operator family. `collect` uses the consumer-aware projection and finalization. The "consumer evidence present without a configured validator" refusal applies only when no validator is configured.

4. The operator's input readers accept TOML or JSON, normalizing the JSON document into canonical document shape and re-reading it with the same strict core parser — one validator for both encodings.

5. `manifest` re-projects the release graph and is self-checking: it reads each archive member back out of the finalized `.tar.gz` and compares size and digest with the candidate bytes, and compares the derived document with the collected `release-manifest.json` when that file is present beside the finalized root. Any mismatch fails closed and writes nothing.

6. The four LOW defects are corrected as described in §2.3.

## 6. Required tests

1. `qualify` with `--output-dir == --artifacts-dir` leaves the candidate bytes **unchanged** and fails closed.
2. The CI `_qualify-target` and `_capture-build` paths refuse a same-file staging source.
3. The operator's `qualify` accepts `--qemu-sysroot` and produces the same result as the CI runner.
4. `plan` produces the same bytes as `ci resolve-release` in v-prefixed-semver identity mode.
5. An unknown identity mode is refused.
6. A consumer-validated release plans and aggregates locally.
7. The operator reads JSON input documents.
8. `manifest` refuses a tampered `--artifacts-dir`.
9. `manifest` refuses a collected manifest that differs from the derived one.
10. `collect` names the aggregation failure reason.
11. An empty `--selected` element is refused with a clear diagnostic.
12. The top-level dispatch prints the usage block.

## 7. Verification

`cargo fmt --all -- --check`, `cargo clippy -p eggpack-cli --all-targets --all-features --locked -- -D warnings`, and `cargo test -p eggpack-cli --all-targets --all-features --locked`.

The workspace gate is not the acceptance criterion for this corrective; it runs once for the whole review line.

## 8. Compatibility note

`manifest` now requires `--build-bindings` and `--qualification-bindings`, because it must re-project the same graph `collect` projects. The command family is unpublished at the time of this correction, so this is a surface change rather than a published break.

## 9. Closure

Closed at `plans/closure/build-qualification/009a-status.md`.
