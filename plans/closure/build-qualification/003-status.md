# Build and Qualification Milestone 003 Closure — Qualification Execution and Evidence

Status: closed

Source plan: `plans/implementation/build-qualification/003-qualification-execution-and-evidence.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `25a6f185f865510be0ab26a51a819d949701674c`.

Implementation SHA: `8f00822294c5d7a85c914080241a3064273bedf0`.

Hosted CI on that exact SHA: [36095915717](https://github.com/eggstack/eggpack/actions/runs/36095915717), completed successfully (push event, all jobs).

## Implementation evidence

- `BuildAttempt` now carries release and source identity from `ReleasePlan` through successful and failed build outcomes.
- `QualificationBindingsV1` is strict, bounded, and limited to the exact selected candidate smoke binding. It does not accept arbitrary hooks, shell commands, or environment overrides.
- Qualification verifies exact target, release, source, build binding, candidate selector, package/binary identity, and candidate inventory. Evidence is typed and bounded, records no local paths or captured process output, and revalidates candidate bytes before and after execution.
- Candidate structural inspection supports ELF, PE/COFF, and thin Mach-O headers and checks architecture against the canonical target independently of the current host.
- Native qualification executes only the exact candidate on a matching actual host. Deferred-native remains pending off its declared host and can execute when that exact host is available. Emulated qualification uses a finite target-to-QEMU mapping on Linux ELF only, with fixed preflight and optional explicit sysroot. Structural qualification reports no execution claim.
- Existing bounded process controls are reused with a separate runtime executable allowlist. Timeouts, cancellation, nonzero exits, output limits, and unavailable QEMU are classified as typed failures/evidence.

## Local verification evidence

The following completed successfully on the implementation tree:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-core --locked
cargo package -p eggpack-core --locked --allow-dirty --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --all-features --locked -- --test-threads=1
./scripts/check-local.sh
git diff --check
```

The final hosted retry additionally confirms the Windows digest regression fixture after it was corrected to mutate bytes outside the PE COFF architecture field. A first hosted run on the preceding SHA failed only this test because it mutated the last byte of the minimal PE fixture; implementation behavior was not implicated. The corrected test passes in the successful final run.

## Hosted evidence

Run 36095915717 passed Linux stable, Linux Rust 1.89, macOS, and Windows. The Windows lane passed MSVC environment verification, the real local Cargo candidate smoke, process-group timeout/cancellation, serialized `eggpack-core` tests, `eggpack-ci` tests, workspace check, and workspace tests. The macOS lane passed workspace check and tests. Linux stable passed formatting, workspace check/tests, clippy, and docs; Rust 1.89 passed workspace check/tests.

These jobs establish platform compilation/test compatibility and matching-host test execution. They do not establish live QEMU emulation: no `qemu-x86_64`, `qemu-aarch64`, or `qemu-arm` executable was available in the local environment. QEMU dispatch, fixed command construction, success, and unavailable-preflight behavior are covered by injected runner tests. This environmental limit is recorded; the finite emulation path remains implemented, and no real emulation result is claimed.

## Findings and downstream transitions

No unresolved medium-or-higher qualification, identity, path-safety, process-boundary, or evidence-integrity finding remains. The initial Windows-only fixture failure was corrected and the same required hosted lane passed on the final implementation SHA.

| Milestone | Status after M003 | Reason |
|---|---|---|
| Build/Qualification M003 | closed | Implementation, local verification, and hosted matrix on `8f00822` satisfy the plan; live QEMU availability gap is explicit and does not invalidate injected-path coverage. |
| Build/Qualification M004 | active | Both hard dependencies are closed: this M003 closure and Release Manifest M002. See `plans/implementation/build-qualification/004-finalization-and-local-aggregation.md`. |
| CI Orchestration M002 | blocked | Requires M004 finalization/aggregation interface and evidence in addition to CI M001. |
| Bootstrap Installers M002 | blocked | Requires M004 finalized bundle/archive evidence and consumer-boundary review. |
| CI Orchestration M003 | blocked | Still requires CI M002 and a separately planned staging adapter. |

M004 is the only direct producer dependency newly unblocked by this closure. CI M002 and Bootstrap M002 are not unblocked by qualification alone.
