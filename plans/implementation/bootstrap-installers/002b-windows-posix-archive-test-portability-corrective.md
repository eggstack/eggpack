# Bootstrap M002b — Windows POSIX Archive Test Portability Corrective

Status: **closed** — `plans/closure/bootstrap-installers/002b-status.md`.
Repository baseline: `eggstack/eggpack@3ef806fedc9d7683948e5678ec0d3c7b78c06f0e`.
Source roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`.
Historical closure records: `plans/closure/bootstrap-installers/002-status.md` and `002a-status.md` (do not rewrite).
Failure: hosted run `37649332862`, Windows job `112888229492` on 2026-10-07.
Class: **corrective / invariant / test infrastructure**, not a new installer feature.

## 1. Problem and objective

The latest Eggpack main commit closes 31 audited defects; its Linux stable, Linux MSRV and macOS hosted lanes passed but the Windows lane fails at the `Focused PowerShell archive runtime (Bootstrap M002a)` step. Compiling `eggpack-bootstrap` test code emits seven errors (`E0433`/`E0599`) from unconditional `std::os::unix::fs::PermissionsExt` imports and `mode`/`set_mode` uses in POSIX archive runtime tests (reported near `crates/eggpack-bootstrap/src/lib.rs` lines 2579, 2635, 2968, 2993, 3050). This is an observed test-portability defect, not evidence of a PowerShell installer runtime defect.

**Objective:** restore full hosted cross-platform verification without weakening Unix runtime tests or Windows PowerShell archive runtime evidence. All previously closed installer capabilities remain historically closed.

## 2. Scope and invariants

In: test-side cfg boundaries, genuinely portable test helpers, regression assertions and documentation of coverage.

Out: producer CLI/manifest schema, generated POSIX or PowerShell script behavior, install and archive policy, network permissions, publication, dependencies, CI lane removal, broad test rewrites.

- Gate POSIX-only *test cases/helpers* using `#[cfg(unix)]` where they inherently need Unix APIs. Do not suppress or remove Windows-targeted PowerShell tests; do not gate an entire shared test module.
- Preserve POSIX archive extraction, symlink, modes, rollback and failure tests on both Linux and macOS; verify they still actually execute.
- Preserve the PowerShell archive runtime matrix on Windows (`pwsh` + `tar.exe`), including case-sensitive inventory and rollback/error scenarios.
- Do not alter production installer bytes or relax fail-closed validation. If another Windows failure emerges once compilation proceeds, investigate honestly rather than marking green.

## 3. Ordered work

1. Inventory all `std::os::unix` and Unix command/permission operations within `eggpack-bootstrap` tests, the current cfgs, and the intended platform matrix; pin the pre-fix failure in the closure.
2. Add narrow cfg guards on the Unix-specific test function(s) and, only if required, Unix-specific helper definitions. Retain shared fixtures and platform-neutral manifest/render/validation tests on Windows.
3. Explicitly verify that the focused Windows PowerShell archive test builds *and runs*; verify the POSIX runtime tests still run on Linux/macOS.
4. Execute the local and hosted gates; record tests actually executed and all Windows failures discovered, not merely compile success.

## 4. Required gates

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo +1.89.0 check --workspace --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Require hosted first-party jobs on the *implementation SHA*: Linux stable, Linux 1.89, macOS and Windows. The Windows `Focused PowerShell archive runtime (Bootstrap M002a)` job must execute successfully; test inventory must show no loss of required platform coverage.

## 5. Stop / acceptance / closure

Stop and register a new scoped plan if changes to actual generated installer behavior, public schema, Windows CI permissions, or the PowerShell runtime contract are necessary. Do not pass by skipping an existing required Windows test.

Close only with an unchanged production/render diff, full local and hosted green matrix, Unix POSIX tests still executed on Unix, Windows runtime tests actually executed, and a `plans/closure/bootstrap-installers/002b-status.md` record containing implementation SHA, observed run/job IDs, error-to-guard matrix, test inventory and genuine outcomes. Reconcile the bootstrap roadmap/registry. Strict M002b closure enables CI M003i's hosted qualification but does not itself implement the identity policy.
