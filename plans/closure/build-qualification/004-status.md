# Build and Qualification Milestone 004 Closure — Finalization and Local Aggregation

Status: closed

Source plan: `plans/implementation/build-qualification/004-finalization-and-local-aggregation.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md`

Reviewed baseline: `25a6f185f865510be0ab26a51a819d949701674c`.

Implementation SHA: `a35cc5f48d5c67704859a8b10854f5f58065c042`.

Hosted CI on that exact SHA: [36098072913](https://github.com/eggstack/eggpack/actions/runs/36098072913), completed successfully (push event, all jobs).

## Implementation evidence

- `finalize_release` joins a `ReleasePlan`, each exact `BuildAttempt`, and its M003 qualification evidence. It verifies release/source/target/strategy identity, complete planned-target coverage, exact contract logical selectors, the byte digest from qualification, and finalization policy before returning an aggregate.
- Required targets must have passing qualification. Non-gating or experimental targets may remain deferred, with their typed result preserved in the caller's evidence; failed or incomplete qualification is rejected.
- Direct and bundle candidates are copied to contract-derived names. SHA-256 sidecars are generated after final bytes are written. A final exact inventory is passed to the existing Manifest M002 builder.
- Archive output uses the explicit producer input `ArchiveEncoding::TarGzip`; it is never guessed. The contract-derived filename must end in `.tar.gz`; other encodings fail closed. Contract member paths are used as tar paths. Gzip time and tar ownership/time metadata are normalized. Members are copied into an invocation-private staging directory, hashed against qualification evidence, archived from those copies, and used as the source for Manifest v1 member hashes. Staging is removed before success is returned.
- The output root must be an absent absolute path under a real parent. Traversal components, symlink parents, and existing destinations fail. Unix roots are restricted to mode 0700; on Windows the output inherits its caller-secured parent ACL. Failures return no manifest and remove only the root created by this invocation. Concurrent distinct roots do not share staging or outputs.
- Contract v1 and Manifest v1 were unchanged. The only new production dependencies are `tar` and `flate2`; tar's optional default features are disabled, and gzip uses the Rust backend.

The archive-format decision is scoped to this producer capability: the caller supplies `TarGzip`, and the contract's declared asset filename carries the `.tar.gz` suffix. No general archive autodetection or new schema claim was added. A future consumer requiring format semantics independent of that suffix, or another encoding, must return to contract/ADR review before support is expanded.

## Local verification evidence

`./scripts/check-local.sh` completed successfully on the final code tree. It runs formatting, workspace check, strict Clippy, contract and CI tests, workspace tests, docs, dependency tree/package checks, and Rust 1.89 workspace check and crate tests. Exact focused/full results included:

```text
cargo fmt --all -- --check                                      passed
cargo check --workspace --all-targets --locked                  passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-core --all-targets --all-features --locked  40 passed, 4 ignored
cargo test --workspace --all-targets --all-features --locked     116 passed, 4 ignored
cargo doc --workspace --no-deps --locked                         passed
cargo package -p eggpack-core --locked --allow-dirty --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'  passed
cargo +1.89.0 check --workspace --all-targets --locked            passed
cargo +1.89.0 test -p eggpack-core --all-targets --locked         passed
./scripts/check-local.sh                                         passed
git diff --check                                                 passed after closure edits
```

Finalizer-specific tests cover direct and bundle naming, checksum generation, final-byte hashes, deterministic archive bytes and exact member paths/content, qualification gating, mixed source identity, extra candidate slots, candidate tampering, preexisting-root preservation, cleanup after failed finalization, and concurrent invocations with distinct roots.

## Hosted evidence

Run 36098072913 passed Linux stable, Linux Rust 1.89, macOS, and Windows. Windows passed MSVC environment verification, both named builder smoke/process-group checks, serialized core tests (including the Windows finalizer tests), CI crate tests, workspace check, and workspace tests. macOS passed workspace check/tests. Linux stable passed formatting, workspace check/tests, strict Clippy, and docs. Rust 1.89 passed workspace check/tests.

The earlier push run [36097749824](https://github.com/eggstack/eggpack/actions/runs/36097749824) found a Windows-only lexical-path comparison bug because `canonicalize` adds the Windows extended path prefix. The finalizer now resolves the requested final component under the canonical parent and rejects traversal components; the same Windows matrix then passed on `a35cc5f`.

## Findings and downstream transitions

No unresolved medium-or-higher integrity, path-safety, qualification-gate, sidecar, archive-member, or mixed-identity finding remains. Caller responsibility for a private Windows parent ACL is explicit in the API contract. No live QEMU execution was available locally; M003's injected finite-runner coverage and recorded environmental limit remain in `plans/closure/build-qualification/003-status.md`.

| Milestone | Status after M004 | Reason |
|---|---|---|
| Build/Qualification M003 | closed | Prior closure `plans/closure/build-qualification/003-status.md` remains valid. |
| Build/Qualification M004 | closed | Local and hosted acceptance evidence is recorded above. |
| CI Orchestration M002 | ready to plan | CI M001 and the Build M003/M004 qualification/finalization interfaces are closed; the CI milestone may now define its gates and drift CLI against those interfaces. |
| Bootstrap Installers M002 | ready to plan | Bootstrap M001 and Build M004 are closed; its plan can define bundle/archive handling while preserving the consumer authority boundary. |
| CI Orchestration M003 | blocked | Still depends on CI M002 and a separately reviewed staging adapter plan. |
| Bootstrap Installers M003 | blocked | Still depends on Bootstrap M002 and adoption evidence. |

Only CI M002 and Bootstrap M002 became ready to plan. No downstream implementation plan or consumer migration is claimed by this closure.
