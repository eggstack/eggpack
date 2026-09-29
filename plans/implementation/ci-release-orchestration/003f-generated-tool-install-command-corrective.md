# CI M003f Corrective — Generated Tool-Install Command Is Rejected by Cargo

Status: closed — see `plans/closure/ci-release-orchestration/003f-status.md` (implementation `c190e77`, hosted run 36632209736 green)

Source finding: Ecosystem M001 live-draft first dispatch (eggsact run `36630837644`, tag `v1.2.7`) failed in every job's first step: `cargo install --git <repo> --rev <rev> --locked -p <package>` exits 1 with `error: unexpected argument '-p' found`. `cargo install` has no `-p/--package` flag for git sources; the package is selected positionally. Cargo's own error prescribes the form: `cargo install --git <repo> --rev <rev> --locked <package>`.

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

## 1. Objective

Emit a tool-install command that Cargo accepts, so the generated workflow can provision its pinned tool on a real runner. Resolves finding F8. No other generated step changes.

## 2. Finding F8 — high: install snippet passes `-p` to `cargo install`

`tool_install_snippet` (`crates/eggpack-ci/src/lib.rs`) emits `-p {package}`. Modern Cargo rejects it before any network or build work, so every generated job fails closed at provisioning with no mutation (no artifacts, no draft, no release). Proven live: eggsact run `36630837644` failed at `Install pinned Eggpack tool` in `resolve`; all downstream jobs skipped; no `v1.2.7` release object exists.

Root cause of escape: M003e's execution proof used a locally built tool binary and never executed the literal install command; the M003e unit invariant asserted install-before-use ordering but not install-command validity.

## 3. Corrective boundaries

- Change exactly one emission site: `-p {package}` becomes positional `{package}`. Pin semantics (`--git`, `--rev`, `--locked`), timeout, and the `eggpack --version` confirmation are unchanged.
- No library validation change, no new action/trigger/privilege, no consumer-config change beyond the mechanical re-pin that M001 performs under its own plan.
- Out of scope: restructuring the producer workspace, adding a root binary target, or caching the install.

## 4. Fix

```rust
"cargo install --git {repo} --rev {rev} --locked {package}\n          eggpack --version\n"
```

Verified against real Cargo before landing: `cargo install --git file://<repo> --rev <sha> --locked eggpack-cli --root <dir>` installs `eggpack 0.1.0` and the binary runs. The `-p` form was re-executed to confirm it is the rejected form.

## 5. Tests

- T1: new renderer unit test asserting the install snippet contains `--locked {package}` positionally, contains `--rev`, and contains no `-p ` flag.
- T2: the seven goldens regenerate (every fixture contains the snippet) and stay green.
- T3: existing `generated_orchestration_cli_executes_capture_to_aggregate` suite unchanged.

## 6. Verification

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Plus a hosted CI run on the exact implementation SHA (all four lanes green), and execution of the literal install command against the implementation revision (file-URL install into a clean root plus binary `--version`).

## 7. Acceptance criteria

M003f closes only when:

- F8 is corrected per §4 with no other generated-output change except the snippet line in goldens;
- T1–T3 pass;
- full local verification passes;
- hosted CI passes on the implementation SHA;
- the literal install command is proven executable at the implementation revision;
- the M001 live re-dispatch (same tag `v1.2.7`, re-pinned workflow) provisions the tool and proceeds past install — recorded as downstream evidence, not as M003f scope;
- no unresolved medium-or-higher finding remains.

## 8. Stop conditions

Stop and re-plan if the install needs anything beyond the positional package (network egress beyond the pinned git fetch, extra privileges, or a workspace restructure).

## 9. Closure evidence

Create:

`plans/closure/ci-release-orchestration/003f-status.md`

Record:

- implementation SHA and hosted run id;
- F8 finding-to-fix entry;
- golden regeneration inventory (same seven fixtures);
- local verification results;
- literal install-command execution evidence.
