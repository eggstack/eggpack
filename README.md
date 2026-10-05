# eggpack

Producer-side release construction for Eggstack. Turns a versioned release
contract into a build/qualify/finalize plan, a deterministic GitHub Actions
workflow, and a draft-only GitHub release.

Eggpack does **not** replace Eggup — Eggup remains the consumer-side verified
install/update/rollback layer. It never publishes: SHA-256 checksums are
integrity facts only, never authenticity, and staging targets a draft on the
exact existing tag. Publication is a separate human action.

## Quickstart

Prerequisites: Rust stable (MSRV 1.89) and git.

```sh
cargo build -p eggpack-cli
EGGPACK=./target/debug/eggpack
$EGGPACK --version   # eggpack 0.1.0
```

A release is three pure file-in/file-out commands — no network, no token, no
real build. These need six input files: a contract, a pack config, build
bindings, qualification bindings, a draft template, and a GitHub policy.
[docs/quickstart.md](docs/quickstart.md) writes all six and runs the whole flow
end to end (~5 minutes, no network); every command and expected output there was
executed verbatim.

```sh
# 1. resolve intent -> release plan + executable CI graph + draft identity
$EGGPACK ci _resolve-release --contract contract.toml --pack-config pack-config.toml \
  --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
  --selected linux-x64 --tag 1.2.6 --source-revision "$REV" --template draft-template.json \
  --source-root . --output-plan out/release-plan.json \
  --output-ci-plan out/release-ci-plan.json --output-github-policy out/draft-policy.json
# checked-out source matches release plan
# resolved runtime release identity for tag 1.2.6

# 2. render the workflow — deterministic: same inputs, byte-identical output
$EGGPACK ci generate --ci-plan out/release-ci-plan.json \
  --github-policy github-policy.json --output out/release.yml
# generated 6831 bytes to out/release.yml

# 3. prove a checked-in workflow still matches its plan — never writes, exit 1 on drift
$EGGPACK ci check --ci-plan out/release-ci-plan.json \
  --github-policy github-policy.json --workflow out/release.yml
# ci check: match (6831 bytes)
```

The rendered workflow is read-only (`contents: read`) and chains
`preflight` → `build_x86_64_unknown_linux_gnu` →
`qualify_build_x86_64_unknown_linux_gnu` → `required_gate` → `aggregate`.

`generate` and `check` are the only user-facing commands. The nine `ci _*`
runners are the file-in/file-out steps the generated workflow replays inside
CI. A bare `eggpack` prints every subcommand.

## Verify

```sh
scripts/check-local.sh   # fmt, check, clippy, tests, doc, package, MSRV
```

## Docs

- [docs/quickstart.md](docs/quickstart.md) — the runnable walkthrough, in full
- [docs/](docs/) — user guides and the workspace layout
- [crates/eggpack-cli/README.md](crates/eggpack-cli/README.md) — command reference
- [architecture/overview.md](architecture/overview.md) — pipeline, crate boundaries, invariants
- [AGENTS.md](AGENTS.md) — operating contract, verification, planning conventions
