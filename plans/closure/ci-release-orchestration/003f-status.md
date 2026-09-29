# CI and Release Orchestration Milestone 003f Closure — Generated Tool-Install Command Corrective

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003f-generated-tool-install-command-corrective.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed baseline: implementation parent `1888af7` on branch `m003e-execution-wiring` (clean tree at closed M003e plus M001 pin-pointing docs).

Implementation commit: `c190e77` on branch `m003f-tool-install-command`.

## Executive finding

M003f is closed. The generated `Install pinned Eggpack tool` step now selects the package positionally (`cargo install --git <repo> --rev <rev> --locked <package>`), which is the form Cargo accepts for git sources. The change is exactly one emission site plus its command-text coverage; all seven goldens regenerate with only that line changed.

No mutation was ever made by the defective command: it fails closed before network or build work, proven live by eggsact run `36630837644` (failed at install in `resolve`, all downstream jobs skipped, no `v1.2.7` release object exists).

## Root-cause matrix

| Finding | Correction | Evidence |
|---|---|---|
| F8 — install snippet passed `-p` to `cargo install`, which has no such flag for git sources | `tool_install_snippet` emits the package positionally; pin semantics (`--git`, `--rev`, `--locked`) and the version confirmation unchanged | `m003f_tool_install_uses_positional_package`; corrected M002 pinned-tooling assertion; seven regenerated goldens |
| Escape cause — M003e proved ordering but never executed the literal install command | Plan §6 now requires literal install-command execution evidence at the implementation revision | `cargo install --git file://<repo> --rev c190e77... --locked eggpack-cli` into a clean root installs `eggpack 0.1.0` and the binary runs; the `-p` form re-executed to confirm rejection |

## Requirement-to-evidence matrix

```text
cargo fmt --all -- --check                                             passed
cargo check --workspace --all-targets --locked                         passed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings  passed
cargo test -p eggpack-ci --all-targets --all-features --locked         passed (58 passed, 3 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked        passed (10 tests)
cargo test --workspace --all-targets --all-features --locked           passed (216 passed, 7 ignored)
cargo doc --workspace --no-deps --locked                               passed
cargo tree (all seven crates)                                          passed
cargo package (all seven crates)                                       passed with workspace path patches
cargo +1.89.0 check --workspace --all-targets --locked                 passed
cargo +1.89.0 test (all seven crates)                                  passed
./scripts/check-local.sh                                               passed (exit 0 on final SHA)
git diff --check                                                       passed
```

The path patches for package verification are the repository's established `scripts/check-local.sh` mechanism for unpublished sibling crates.

Hosted CI run `36632209736` validates implementation commit `c190e77`; Linux stable, Linux Rust 1.89, macOS, and Windows lanes passed. The workflow run is linked at [GitHub Actions run 36632209736](https://github.com/eggstack/eggpack/actions/runs/36632209736).

Golden regeneration inventory (snippet line only, `-p eggpack-cli` to positional):

- `crates/eggpack-ci/tests/fixtures/m002-direct.yml`
- `crates/eggpack-ci/tests/fixtures/m002-bundle.yml`
- `crates/eggpack-ci/tests/fixtures/m002-archive.yml`
- `crates/eggpack-ci/tests/fixtures/m002-mixed.yml`
- `crates/eggpack-ci/tests/fixtures/m003b-direct-staging.yml`
- `crates/eggpack-ci/tests/fixtures/m003b-bundle-staging.yml`
- `crates/eggpack-ci/tests/fixtures/m003b-archive-staging.yml`

## Invariant, recovery, compatibility, and security review

- Pin semantics are unchanged: exact git URL, exact 40-hex revision, `--locked`, immutable (never floating `main`).
- The install still precedes every Eggpack invocation and still confirms with `eggpack --version`; timeout handling unchanged.
- No library validation change, no new action/trigger/privilege/egress beyond the pinned git fetch the design already required.
- Downstream consumer re-pin (Ecosystem M001 live re-dispatch of tag `v1.2.7`) is recorded under M001/M005, not as M003f scope.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| None | No unresolved medium-or-higher finding. | M003f acceptance criteria satisfied. |

## Roadmap disposition and dependency transitions

| Milestone | Status after M003f | Reason |
|---|---|---|
| CI M003f | closed | This record and hosted run `36632209736`. |
| Ecosystem adoption M001 (eggsact) | ready to re-dispatch | Corrective dependency is closed; it re-pins to `c190e77` and re-dispatches the same tag `v1.2.7` (no new version; the failed attempt made no mutation). |
| CI M003b live draft qualification | still blocked | Runs inside Ecosystem M001; unchanged. |

Registry and the CI roadmap now identify M003f as closed and Ecosystem M001 as ready to re-dispatch.
