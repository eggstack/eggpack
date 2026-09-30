# Ecosystem Adoption Milestone 001 Closure — eggsact Direct Release Adoption and Live Draft Qualification

Status: closed

Source plan: `plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md`

Roadmap: `plans/subsystems/ecosystem-adoption-roadmap.md`

Mirrored consumer milestone: `eggstack/eggsact: plans/closure/distribution-update-release/005-status.md` (Distribution M005, closed conditionally on eggsact M005a).

## Executive finding

Ecosystem M001 is closed. eggsact is the first real consumer of Eggpack as
producer authority. A maintainer-authorized release (`v1.2.7`) ran the
generated pipeline end to end on real runners and produced a complete
15-asset draft: five cross/native builds, five qualifications, five consumer
MCP validations, the required gate, aggregation, and draft staging, with one
write-authorized job and no publication, tag, or clobber authority anywhere in
the generated workflow. The maintainer then published the draft, and the
public installer, `releases/latest/download`, and `eggsact update` checks all
pass against the published release.

Getting there required four upstream correctives, each found by the real
pipeline rather than by inspection, each closed with hosted cross-platform
evidence:

| Corrective | Finding | Implementation |
|---|---|---|
| CI M003e | no generated workflow could execute: uninstalled tool, four missing output directories, relative aggregate root and qualify directory | `b9062d4` |
| CI M003f | `cargo install --git` rejects `-p`; the package must be positional | `c190e77` |
| CI M003g | cross-tool PATH scoped to a later step; an invalid `cargo zigbuild --version` check; exec bits stripped by artifact transfer; consumer validation executed against a failed candidate; stage-draft arity guard rejected the renderer's call; staging error swallowed; `make_latest` sent as a boolean | `5acda73`, `4b28820`, `5ac5b83`, `e5c81f2` |

The final tool pin is `e5c81f28bd328d4aea41c3f061a0ed9944306262`.

## Plan requirements versus outcome

| Requirement (§) | Outcome |
|---|---|
| §5 DistributionContract | Checked in at `release/eggpack/distribution.toml`; asset expansion byte-exact against the five public names, including the `.exe` convention |
| §6 PackConfig / toolchain policy | Checked in; Zig 0.14.1 and cargo-zigbuild 0.23.3 pinned, both Linux targets cross-built with the glibc 2.17 floor, three targets native |
| §7 Build/qualification bindings | Checked in; five per-target smokes plus the bounded consumer-validator map |
| §8 Installer presentation | Checked in; four installers staged (`install.sh`, `install.ps1`, `install-exact.sh`, `install-exact.ps1`) |
| §9 Trigger policy | `workflow_dispatch` only, with the exact existing `release_tag`; no tag-push trigger |
| §10 Runtime release identity | M003d runtime resolver consumed by every job; all jobs verify checked-out source against the resolved plan |
| §11 Tool pin | Exact 40-hex revision, re-pinned to the M003g implementation; no floating branch, tag, or version |
| §12 Legacy cutover | `.github/workflows/release-binaries.yml` replaced by the generated workflow; no second active release workflow; predecessor recoverable from history at `d8014cf^` |
| §13 Script migration | `scripts/check-release-contract.py` now derives the target/asset matrix from the Eggpack configuration and retains only eggsact-owned installer/updater invariants |
| §14 Drift guard | `.github/workflows/release-drift.yml`: pinned tool install, `eggpack ci check`, contract script; green on every push |
| §15 Live draft qualification | Run 36652731202 attempt 1, 15/15 assets, draft preserved; rerun attempt 2 reused the draft and every asset except the Windows one (see Unresolved) |
| §16 Publication evidence | Maintainer published; exact-tag and `latest/download` installer URLs 200, `install.sh` smoke installed `eggsact 1.2.7`, `eggsact update` reports it current |
| §17 Self-update boundary | Unchanged; new contract-parity test `published_targets_match_eggpack_contract` guards the updater's published-target table against the contract without migrating self-update |
| §20 Stop condition | Resolved by Build M006; M001 resumed and completed |
| §23/§24 Baseline re-review | Consumer re-reviewed at `8a582a4`; no material release-surface drift since the originally recorded baseline |

## Local verification

```text
./scripts/check-local.sh (eggpack)                                     passed (exit 0 on e5c81f2)
cargo test --workspace --all-targets --all-features --locked           passed (218 passed, 7 ignored)
hosted CI: linux stable, linux 1.89.0, macos, windows                  all green on every implementation commit
hosted CI (eggsact): ci.yml, release-drift.yml, maintenance lanes      green
python3 scripts/check-release-contract.py (eggsact)                    passed
scripts/release-check.sh (eggsact, on the release-prep commit)         passed, no publication
```

## Recorded parity deltas (accepted, all pre-registered in the mirrored plan)

- No `Swatinem/rust-cache` step: `GitHubPolicy` pins a fixed action set. Build-time caching only, not release evidence.
- No `windows-installer-check` job: Windows installer/source behavior is covered by the per-target consumer validation and the contract script.
- One `timeout_minutes` (60) for every job, where the predecessor used 10/45/10/15. Every live job completed well inside the bound.
- Initial trigger is manual dispatch with the exact tag rather than tag-push, per §9.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None in Eggpack | No unresolved medium-or-higher producer defect. | M001 acceptance criteria satisfied. |
| Consumer-side, medium | The Windows candidate is not byte-reproducible (PE timestamp plus random PDB GUID, 24 bytes, proven byte-level). A rerun therefore reuses the draft and the other four assets but correctly refuses to clobber the Windows one. | Owned by eggsact M005a; Eggpack's refusal is the correct fail-closed behavior. |
| External, operational | ziglang.org can exceed the bounded 600s download timeout on a cross-build job. | Bounded, fail-closed, resolved by job retry; not a configuration defect. |
| Cosmetic | `upload-artifact` / `download-artifact` warn about the unsupported `if-no-files-found` input the renderer emits. | Third-party annotation only; no effect on evidence. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M001 | Reason |
|---|---|---|
| Ecosystem M001 (eggsact) | closed | This record plus `eggstack/eggsact: plans/closure/distribution-update-release/005-status.md`. |
| CI M003b live draft qualification | satisfied except rerun-reuse | Real draft, exact assets, draft-only, no-clobber all recorded; byte-identical rerun reuse is blocked on eggsact M005a. |
| Phase 8 exit | blocked | Requires a byte-identical rerun-reuse receipt, which needs eggsact M005a. |
| Ecosystem M002 (stegoeggo) | ready to plan | First-consumer evidence now exists; second-consumer evidence remains the anti-overfitting control. |
| Build/Qualification, Bootstrap M003 | unchanged | Bootstrap M003 still needs independent adoption evidence/candidate review. |

Registry, the ecosystem roadmap, and the CI roadmap now identify M001, M003e,
M003f, and M003g as closed and Phase 8 as blocked only on the consumer's
Windows artifact determinism.
