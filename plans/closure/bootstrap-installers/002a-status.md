# Bootstrap Installers Milestone 002a Closure — PowerShell Archive Runtime Evidence

Status: closed

Source plan: `plans/implementation/bootstrap-installers/002a-powershell-archive-runtime-evidence-corrective.md`

Coordinating closeout pass: `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md`

Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Historical M002 closure: `plans/closure/bootstrap-installers/002-status.md`

Reviewed baseline: `4d2270afe7de10bdff92563ed0f51a41ba807a04` (M002a corrective implementation; no production-code commit supersedes it — all later commits to `abd125140c763970f0658610b419b248cf41e91f` are planning-only).

Implementation commits: `4d2270a` (`feat: implement bootstrap M002a archive runtime evidence and CI M002a execution wiring`). One commit carries both M002a correctives; Bootstrap-owned paths are `crates/eggpack-bootstrap/src/lib.rs` (769 changed lines per `git show --stat 4d2270a`), `crates/eggpack-bootstrap/README.md`, and the Windows qualification steps in `.github/workflows/ci.yml`.

Implementation SHA: `4d2270afe7de10bdff92563ed0f51a41ba807a04`.

Hosted CI: [run 36154905956](https://github.com/eggstack/eggpack/actions/runs/36154905956), event `push`, attempt 1, conclusion `success`, `head_sha` exactly `4d2270afe7de10bdff92563ed0f51a41ba807a04`. All four lanes passed: `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)`.

## Executive finding

M002a is closed. The historical M002 evidence gap — PowerShell archive installers were rendered and parser-checked but never runtime-executed, and several malicious-archive tests failed at outer archive integrity before reaching their intended inner member defenses — is corrected and qualified on the Windows lane. At `4d2270a` the Windows job explicitly verifies `pwsh` 7 and `tar.exe`, then executes the focused `m002a_powershell_archive_runtime` test, which installs exact expected bytes with nested-source flattening, proves no-overwrite and pre-existing preservation, rejects wrong outer size/SHA before extraction, drives all seven inner member cases past outer digest verification into their intended inventory/path/type guards (missing, extra, traversal, absolute, backslash, symlink, directory), exercises member size/SHA evidence failures, asserts the bounded `tar.exe is required` tool boundary, and proves archive placement-conflict rollback of invocation-created files while pre-existing paths stay untouched. The POSIX malicious-member matrix was corrected under the same valid-outer-digest principle. No production renderer change was required: runtime evidence exposed no generator defect, so M002a is a test/evidence corrective with no production delta beyond the added qualification itself.

## Root-cause/corrective finding

Historical evidence gap (recorded in the M002 post-closure annotation, preserved intact):

- PowerShell archive output was previously parsed but not runtime-qualified (POSIX bundle/archive and PowerShell bundle paths executed; PowerShell archive installation never executed end-to-end, short of the M002 acceptance criterion requiring archive POSIX and PowerShell runtime qualification for TarGzip);
- malicious archive fixtures mutated archive bytes while retaining the original outer archive digest, so those cases could reject at archive-integrity verification before exercising the intended member inventory/path/type defenses.

M002a correction, as implemented at `4d2270a`:

- Windows hard requirement for `pwsh` 7 + `tar.exe` (CI prerequisite step fails the lane if either is absent; the focused test asserts both on Windows and may not silently skip there — on non-Windows lanes it runs as a regression guard when both tools exist and skips otherwise);
- focused PowerShell archive runtime test `m002a_powershell_archive_runtime` (positive install, flattening, no-overwrite, pre-existing preservation, outer integrity negatives, seven inner negatives, member evidence negatives, tool boundary, rollback/pre-existing cases);
- raw test-only tar fixture support (`build_raw_tar_gz` with `RawTarEntry` name/kind/linkname/data) for unsafe names/types the Rust tar builder refuses to emit — producer validation is not weakened to create malicious fixtures;
- outer manifest digest recomputation (`manifest_with_archive_bytes`) for every tampered archive so outer verification succeeds and the intended inner guard is actually reached, with stderr assertions on the expected guard category;
- intended inner guard assertions per case (`archive member inventory mismatch`, `unsafe archive member`, `symlink member rejected`, `member is not a regular file`) plus proof outer digest passed (no `size mismatch` without the expected guard) and zero installed files;
- member evidence negative tests (wrong member size / wrong member SHA against the valid-inventory archive, failing after extraction/member discovery but before placement);
- no-overwrite/pre-existing preservation/rollback evidence via the shared `$created` rollback primitive (first-move-succeeds/second-conflicts fixture rolls back the invocation-created file; pre-existing sentinels byte-identical afterwards);
- deterministic PowerShell rendering/parser checks retained alongside runtime execution.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| Positive PowerShell TarGzip install | Host-specific archive contract/manifest/policy plus local HTTP fixture; generated installer executed via `pwsh -NoProfile -NonInteractive -File`; exit success. Hosted Windows step `Focused PowerShell archive runtime (Bootstrap M002a)` passed. |
| Exact installed bytes | `dest/hostbin` and `dest/host-helper` byte-compared against fixture member bodies. |
| Nested source flattening | Members sourced under nested paths install to flat contract install names; `dest/bin` asserted absent. |
| Repeat/no-overwrite failure | Second run against the populated destination fails; installed bytes unchanged; file count stays 2. |
| Pre-existing destination preservation | Pre-existing `hostbin` sentinel run fails with sentinel bytes intact and no additional install. |
| Wrong outer size | Mutated size manifest fails before member listing/extraction; zero installed files. |
| Wrong outer SHA | Zeroed SHA manifest fails before member listing/extraction; zero installed files. |
| Missing member after valid outer digest | `manifest_with_archive_bytes` outer digest matches tampered bytes; stderr reaches `archive member inventory mismatch`; nothing installed. |
| Extra member after valid outer digest | Same outer-digest principle; `archive member inventory mismatch`; nothing installed. |
| Traversal member after valid outer digest | Raw tar entry `../evil`; `unsafe archive member`; nothing installed. |
| Absolute member after valid outer digest | Raw tar entry `/abs`; `unsafe archive member`; nothing installed. |
| Backslash member after valid outer digest | Raw tar entry `bin\evil`; `unsafe archive member`; nothing installed. |
| Symlink member after valid outer digest | Raw tar entry kind `2` with linkname; `symlink member rejected`; nothing installed. |
| Directory member after valid outer digest | Raw tar entry kind `5`; `member is not a regular file`; nothing installed. |
| Wrong member size | Valid-inventory archive with mutated member size fails after extraction/member discovery, before placement; nothing installed. |
| Wrong member SHA | Valid-inventory archive with mutated member SHA fails likewise; nothing installed. |
| Missing `tar.exe` bounded failure | `PATH=''` probe asserts `tar.exe is required`, nonzero exit, no extraction, no installed files (with a documented static-boundary fallback when the lane still resolves `tar.exe` via app-alias paths). |
| Rollback/pre-existing-path evidence | Placement-conflict fixture: first move succeeds, second fails on a pre-existing sentinel; invocation-created `hostbin` rolled back (asserted absent), sentinel bytes intact. Archive-specific execution of the shared `$created` rollback primitive — no test hook added to generated installers. |
| Corrected POSIX inner-defense matrix | `m002_archive_posix_runtime_matrix` (and bundle matrix unchanged) rebuilt on the matching-outer-digest principle: missing/extra/traversal/absolute/symlink/type cases reach the intended inventory/path/type guard with no installed files; unavailable-tar, pre-existing, and member size/SHA negatives retained. |
| Deterministic PowerShell rendering/parser checks | Same input renders byte-identical script; script contains `tar.exe`, no `sudo`/`Start-Process`, `-MaximumRedirection 0`; PowerShell parser coverage retained; no elevation/update/latest/redirect/arbitrary-extractor strings. |

## Production implementation evidence

- `crates/eggpack-bootstrap/src/lib.rs` (769 changed lines): test-only helpers `build_deterministic_tar_gz`, `build_raw_tar_gz`/`RawTarEntry`, `manifest_with_archive_bytes`, local multi-request HTTP server `serve_map`, `pwsh_available`/`tar_exe_available` prerequisite probes; the `m002a_powershell_archive_runtime` qualification test described above; corrected POSIX archive malicious-member fixtures. No renderer, policy, contract/manifest, encoding, or placement-authority production change — runtime evidence exposed no generator defect.
- `crates/eggpack-bootstrap/README.md` (2 changed lines): documents the M002a Windows-lane runtime qualification and the valid-outer-digest test principle.
- `.github/workflows/ci.yml` (16 changed lines across both correctives): Windows lane verifies `pwsh -Version` plus `tar.exe --version` in `Verify PowerShell archive prerequisites (pwsh 7 + tar.exe)`, then runs `Focused PowerShell archive runtime (Bootstrap M002a)` (`cargo test -p eggpack-bootstrap --locked m002a_powershell_archive_runtime`) before the workspace suite. The focused test fails — never skips — when a required Windows prerequisite is absent.
- `eggpack-bootstrap` dependency set unchanged (contract, manifest, serde, serde_json, toml; no network, process, GitHub, or Eggup dependencies).

## Hosted evidence

Run `36154905956` (push, attempt 1, `head_sha` = `4d2270a`, conclusion `success`):

- `linux (stable)` — passed (fmt, workspace check, workspace tests incl. POSIX bootstrap matrices, Clippy, docs).
- `linux (1.89.0)` — passed (workspace check, workspace tests).
- `portability (macos-latest)` — passed (workspace check, workspace tests incl. POSIX bootstrap matrices).
- `portability (windows-latest)` — passed, including in order:
  1. `Verify PowerShell archive prerequisites (pwsh 7 + tar.exe)` — success;
  2. `Focused PowerShell archive runtime (Bootstrap M002a)` — success;
  3. `Focused generated-orchestration execution (CI M002a)` — success;
  4. `Focused orchestration CLI harness (CI M002a)` — success;
  5. `cargo check --workspace --all-targets --locked` and `cargo test --workspace --all-targets --all-features --locked` — success.

The Windows lane explicitly verified `pwsh` 7 and `tar.exe` before running the focused M002a PowerShell archive runtime test, and the focused test passed. Windows qualification evidence is claimed from these hosted steps: on the closeout Linux workstation (`pwsh` 7.6.6 present, `tar.exe` absent) the focused test correctly exercises its non-Windows skip guard, so local execution there is a regression guard, not qualification evidence.

## Verification executed

Local verification at closeout commit `abd1251` (planning-only delta over `4d2270a`; no production diff):

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-bootstrap --all-targets --all-features --locked  8 passed
cargo test -p eggpack-ci --all-targets --all-features --locked         21 passed, 1 ignored
cargo test -p eggpack-cli --all-targets --all-features --locked        2 passed
cargo test --workspace --all-targets --all-features --locked           136 passed, 5 ignored (pre-existing ignores)
cargo doc --workspace --no-deps --locked                               passed
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked         8 passed
cargo +1.89.0 test -p eggpack-ci --all-targets --locked                21 passed
cargo +1.89.0 test -p eggpack-cli --all-targets --locked               2 passed
./scripts/check-local.sh                                               passed (exit 0)
git diff --check                                                       passed
```

ShellCheck/PSScriptAnalyzer were not installed on the closeout workstation; they remain supplementary — runtime execution plus parser checks provide the closure evidence. No medium-or-higher finding emerged during local verification. `git diff 4d2270a..HEAD -- crates/ .github/` is empty, so the hosted evidence at `4d2270a` still describes the production code being closed.

## Invariant review

First-install-only semantics, no independent target/asset table, unsupported-target closed failure, bounded fixed-origin download policy (exact URLs, no redirects, timeouts, no credentials), integrity-before-execution (outer size/hash, then inventory, then member size/hash), no implicit privilege escalation, existing-installation preservation on every verification failure, rollback limited to invocation-created paths, readable deterministic installer text, and TarGzip-only archive policy all hold. No overwrite, update, receipt, service, elevation, or discovery authority introduced.

## Failure/recovery review

Every matrix case (outer integrity, seven inner member guards, member evidence, tool boundary, placement conflict, pre-existing destination) fails closed with bounded diagnostics and installs nothing except the proven rollback case, which removes only the invocation-created file and leaves sentinels byte-identical. Fault-injected `ln` failure (historical POSIX evidence) and the new archive placement-conflict fixture together prove rollback removes partial placement without touching pre-existing state.

## Compatibility/migration review

DistributionContract v1 and ReleaseManifest v1 unchanged. Direct M001 renderers untouched. `BootstrapInstallPolicyV1` remains generator input, not portable release identity. Malicious-archive construction is test-only (`build_raw_tar_gz`); production M004 remains the sole producer finalization path. No Eggup dependency, no persistent lockfile, no consumer migration required or claimed.

## Security review

HTTPS production origin with loopback-only fixture mode; exact member inventory checked before extraction into a private temp; traversal/absolute/backslash/symlink/directory/device members rejected; extracted regular files revalidated by exact size and SHA-256; atomic no-overwrite placement under explicit caller-owned modes; no downloaded extractor or caller-supplied command (`tar.exe` fixed boundary with bounded failure); no secrets in generated text; no elevation/update/latest/version/redirect-following/arbitrary-extraction behavior. SHA-256 remains integrity, not authenticity. No unresolved medium-or-higher extraction/placement finding remains.

## Documentation/operations evidence

`crates/eggpack-bootstrap/README.md` documents the M002a runtime qualification matrix and the valid-outer-digest principle; root README first-install/transactional language remains accurate and needed no change. The Windows prerequisite verification plus focused-test steps are checked into `.github/workflows/ci.yml`; `scripts/check-local.sh` covers the crate.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher archive runtime/safety finding. | M002a acceptance criteria pass. |
| Informational | The `tar.exe`-absent probe has a documented static-boundary fallback when a lane resolves `tar.exe` outside `PATH` (Windows app-alias path). | The generated script's `tar.exe is required` guard is asserted either way; recorded in source, not a product gap. |
| Environmental | ShellCheck/PSScriptAnalyzer not installed on the closeout workstation; local Linux execution of the focused test exercises the skip guard only. | Supplementary linters only; qualification evidence is the hosted Windows lane above. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M002a | Reason |
|---|---|---|
| Bootstrap M001 | closed | Unchanged; direct behavior preserved. |
| Bootstrap M002 | closed historically; corrective satisfied | Original generator evidence remains intact; the PowerShell archive runtime gap is now qualified. |
| Bootstrap M002a | closed | Implementation `4d2270a`; hosted run 36154905956 (attempt 1, all lanes green including the focused PowerShell archive runtime step after explicit `pwsh`/`tar.exe` verification). |
| Bootstrap M003 two-consumer adoption | blocked | M002a precondition now satisfied; still requires real adoption evidence and candidate review. M002a closure alone does not authorize M003 and no M003 plan is authored here. |
| CI M002a | closed separately | See `plans/closure/ci-release-orchestration/002a-status.md`; same SHA/run. |
| Ecosystem adoption | blocked | Still requires core pieces plus real-consumer selection. |

Bootstrap M003 is therefore **not ready to plan**: the archive-safety precondition is now satisfied, but real two-consumer adoption evidence and receipt-handoff review are still outstanding.

## Registry updates

`plans/registry.md` reconciled: Bootstrap M002a moved to closed with this closure path; M002 annotated as historical with corrective satisfied; Bootstrap M003 blocker narrowed to real adoption evidence and candidate review; narrative, execution graph, and next-handoff paragraphs updated to the post-close state. Bootstrap roadmap `plans/subsystems/bootstrap-installers-roadmap.md` updated to match.
