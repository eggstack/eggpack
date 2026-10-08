# Bootstrap Installers M002b Closure — Windows POSIX Archive Test Portability

Status: closed

Source plan: `plans/implementation/bootstrap-installers/002b-windows-posix-archive-test-portability-corrective.md`

Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Historical records retained: `plans/closure/bootstrap-installers/002-status.md` and `002a-status.md`.

Reviewed baseline: `3ef806fedc9d7683948e5678ec0d3c7b78c06f0e`.

Implementation commit and immutable SHA: `88ddf2e796ac3674b306b51726c65ed7cc9bd29a` (`fix: gate POSIX archive runtime test on Unix`).

Hosted CI: [run 37806244313](https://github.com/eggstack/eggpack/actions/runs/37806244313), push, attempt 1, conclusion `success`, `head_sha` exactly `88ddf2e796ac3674b306b51726c65ed7cc9bd29a`.

## Executive finding

M002b is closed. The Windows compile failure was caused by `m002_archive_posix_runtime_matrix` using Unix-only `PermissionsExt` APIs without a Unix test boundary. Adding `#[cfg(unix)]` to that test function fixed the failure. The existing POSIX bundle/archive runtime matrices continue to execute on Linux and macOS. The Windows PowerShell archive runtime test was not gated and executed successfully with `pwsh 7` and `tar.exe`. This corrective changes test compilation only; no production behavior, generated installer, policy, schema, dependency, or CI permission changed.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| POSIX-only test code is bounded to Unix | `m002_archive_posix_runtime_matrix` is annotated `#[cfg(unix)]`; its entire function, including `PermissionsExt`, Unix symlink, `sh`, and Unix filesystem behavior, is excluded from non-Unix test builds. |
| Shared and platform-neutral bootstrap tests remain available | Linux/macOS and Windows workspace test suites compile and pass. Only the inherently POSIX archive runtime function is gated; shared fixtures and render/validation tests remain outside it. |
| POSIX runtime behavior remains exercised | Local bootstrap suite: 11 passed, including `m002_bundle_posix_runtime_matrix` and `m002_archive_posix_runtime_matrix`. Hosted Linux stable, Linux 1.89, and macOS workspace tests all passed. |
| Windows PowerShell archive runtime remains required and executes | Hosted Windows verified `pwsh -Version` and `tar.exe --version`; focused `m002a_powershell_archive_runtime` passed, followed by Windows workspace tests. |
| No production installer output or behavior changed | The implementation diff is one cfg attribute in the test module. `git diff 3ef806f..88ddf2e -- crates/eggpack-bootstrap/src/lib.rs` shows only that test annotation; no renderer or generated output changes. |

## Production implementation evidence

The single source change is a Unix cfg on `m002_archive_posix_runtime_matrix` in `crates/eggpack-bootstrap/src/lib.rs`. The function uses Unix permission modes and POSIX shell/runtime tooling and belongs in Unix lanes. The adjacent PowerShell runtime matrix remains unconditionally compiled and retains its Windows prerequisite assertions. No production source, installer template, build dependency, workflow permission, or generated artifact changed.

## Exact verification executed

Local verification at implementation SHA `88ddf2e796ac3674b306b51726c65ed7cc9bd29a`:

```text
cargo fmt --all -- --check                                           passed
cargo check --workspace --all-targets --locked                       passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-bootstrap --all-targets --all-features --locked 11 passed
cargo test --workspace --all-targets --all-features --locked          257 passed, 9 ignored
cargo +1.89.0 check --workspace --all-targets --locked               passed
./scripts/check-local.sh                                              passed (exit 0)
git diff --check                                                      passed
```

Hosted run 37806244313 on the implementation SHA:

| Lane | Result and relevant coverage |
|---|---|
| `linux (stable)` | Passed format, check, workspace tests, Clippy, and docs; POSIX runtime tests included. |
| `linux (1.89.0)` | Passed workspace check and tests; POSIX runtime tests included. |
| `portability (macos-latest)` | Passed workspace check and tests; POSIX runtime tests included. |
| `portability (windows-latest)` | Passed MSVC environment verification, focused builder/process tests, CI tests, explicit `pwsh 7` + `tar.exe` prerequisite step, focused PowerShell archive runtime, focused orchestration tests, workspace check, and workspace tests. |

The prior failing run was `37649332862` on baseline `3ef806fedc9d7683948e5678ec0d3c7b78c06f0e`; the new Windows lane passed on the exact implementation SHA. No required runtime test was skipped to obtain green status.

## Invariant review

POSIX archive behavior remains tested on Unix. Windows PowerShell archive behavior remains tested on Windows. The change does not alter deterministic rendering, contract-owned names, validation, extraction, integrity checks, install placement, rollback, first-install-only semantics, or draft/publication boundaries. Nothing was weakened to reach closure.

## Failure and recovery review

The failure occurred at test compilation before the Windows PowerShell runtime step. The cfg boundary makes the Unix API dependency explicit while allowing Windows compilation to proceed to the required PowerShell tests. If the Windows runtime test fails in a future run, it still fails the lane; it has no Windows skip path. The local and hosted lanes showed no additional failure requiring a corrective plan.

## Compatibility and migration review

No public API, serialized format, dependency, command, installer bytes, or consumer behavior changed. No migration is required. M002, M002a, and M003 historical closure records remain unchanged.

## Security review

No runtime security boundary or production code changed. Archive integrity, inventory, path/type rejection, exact installation, no-overwrite, and rollback behavior remain as covered by the prior closures and the unchanged runtime tests. SHA-256 remains an integrity check and is not represented as authenticity evidence.

## Documentation and operations evidence

The closure and roadmap now record the exact implementation SHA, hosted run, platform coverage, and dependency transition. No user-facing documentation change was needed for this test-only portability fix.

## Unresolved findings

None. Linux stable/MSRV, macOS, and Windows hosted lanes all passed. The focused Windows PowerShell archive runtime actually executed successfully.

## Roadmap disposition and dependency transitions

| Milestone | Disposition |
|---|---|
| Bootstrap M002 | Closed historically; unaffected. |
| Bootstrap M002a | Closed; historical runtime evidence retained. |
| Bootstrap M002b | Closed on `88ddf2e`; hosted run `37806244313` passed all four lanes. |
| Bootstrap M003 | Remains closed as the prior two-consumer adoption/ownership decision; no follow-up is unblocked by this test corrective. |
| CI M003i | Its hard Bootstrap M002b dependency is satisfied. M003i is now **ready** and is the next eligible plan. |
| wg-basic M003 | Remains a separate downstream consumer plan, blocked pending M003i's qualified immutable producer revision and consumer-owned work. |

## Registry updates

`plans/registry.md` records Bootstrap M002b as closed with this record and moves CI M003i from blocked to ready. The Bootstrap and CI subsystem roadmaps reflect the same transition. The source plan status is closed. Earlier closure records remain unchanged.
