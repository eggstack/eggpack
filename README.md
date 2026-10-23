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

`generate` and `check` are the only user-facing commands in the `ci` family. The nine `ci _*`
runners are the file-in/file-out steps the generated workflow replays inside
CI. A bare `eggpack` prints every subcommand.

There are two other user-facing families.

## The operator family — build a release locally

Six commands drive the producer on a workstation: `check` → `plan` → `package` →
`qualify` → `collect` → `manifest`. Unlike the `ci` rendering path, this family
**performs a real Cargo build** — `package` invokes `cargo +<toolchain>` on
every planned target — but it still performs no network access and needs no
GitHub token. The six commands share one implementation with the `ci _*`
runners, so both surfaces derive one plan from one rule set.

| Command | Class | What it does |
|---|---|---|
| `check` | validation | Resolves the whole planning input set and writes nothing. |
| `plan` | validation | Writes the `ReleasePlan` — the release identity, the selected targets, and their policies. |
| `package` | build | Builds every planned target's candidates under an output root. |
| `qualify` | build | Qualifies the built candidates and writes qualification evidence. |
| `collect` | release aggregation | Aggregates the qualified candidates into a finalized release tree. |
| `manifest` | final-bytes evidence | Derives `release-manifest.json` from the finalized release tree — and checks it against the finalized tree it describes. |

Three optional flags are available on the family: `--qemu-sysroot` (the QEMU
sysroot for an emulated target), `--identity-mode` (a caller-chosen
release-identity mode), and `--consumer-validators` (a checked-in consumer
validator map). The producer policy and both bindings documents may be given as
**TOML or JSON**. `qualify` refuses `--output-dir == --artifacts-dir`, because
staging the qualification artifacts into the directory the candidates were read
from would rewrite the very bytes the qualification is about; and `manifest`
reads every archive member back out of the finalized `.tar.gz` and compares it
against the collected manifest, so a manifest that disagrees with its archive is
refused rather than emitted.

```sh
# 1. validate the planning inputs — writes nothing
$EGGPACK check --contract contract.toml --pack-config pack-config.toml \
  --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
  --selected linux-x64 --release-id 1.2.6 --source-revision "$REV"

# 2. resolve the release identity — writes the plan
$EGGPACK plan --contract contract.toml --pack-config pack-config.toml \
  --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
  --selected linux-x64 --release-id 1.2.6 --source-revision "$REV" \
  --output out/release-plan.json

# 3. build every planned target — a real Cargo build
$EGGPACK package --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml --source-root demo --output-root out/build

# 4. qualify the candidates
$EGGPACK qualify --contract contract.toml --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
  --artifacts-dir out/build --output-dir out/qualification

# 5. aggregate the qualified candidates into a finalized release tree
$EGGPACK collect --contract contract.toml --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
  --artifacts-dir out/qualification --output-root out/release/finalized

# 6. derive the final-bytes evidence — the manifest sits beside the finalized root
$EGGPACK manifest --contract contract.toml --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml --qualification-bindings qualification-bindings.toml \
  --finalized-root out/release/finalized --output out/manifest/release-manifest.json
```

[docs/quickstart.md](docs/quickstart.md) runs this path end to end with the
exact inputs and the quoted output of every step.

## The `contract` family

`contract expand` is a bounded, local-only projection that reads one producer
contract and prints one scalar, so a consumer repository can read a producer
fact instead of re-implementing TOML parsing and template expansion.
