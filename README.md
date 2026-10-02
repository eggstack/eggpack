# eggpack

Producer-side release construction and distribution for Eggstack: portable
release contracts, build/qualification planning, artifact finalization,
bootstrap installers, generated release CI, and GitHub draft staging.

Eggpack does **not** replace Eggup — Eggup remains the consumer-side verified
installation/update/rollback layer. SHA-256 checksums are integrity facts only,
never authenticity; staging targets drafts on the exact existing tag, and
publication stays a separate human action.

## Quickstart

Prerequisites: Rust stable (MSRV 1.89) and git.

```sh
cargo build -p eggpack-cli
./target/debug/eggpack --version   # eggpack 0.1.0
```

Worked example — resolve a one-target release and render its GitHub workflow
(~5 minutes, no network): [docs/quickstart.md](docs/quickstart.md). It covers
`ci _resolve-release` → `ci generate` → `ci check` (including drift detection)
with exact input files and expected output.

## Layout

| Crate | Role | Details |
|---|---|---|
| `eggpack-contract` | Portable schema-v1 release layout authority + validators | [README](crates/eggpack-contract/README.md) |
| `eggpack-manifest` | Bounded schema-v1 JSON evidence for finalized releases | [README](crates/eggpack-manifest/README.md) |
| `eggpack-core` | Planning, Cargo building, qualification, finalization | [README](crates/eggpack-core/README.md) |
| `eggpack-bootstrap` | Deterministic first-install shell/PowerShell renderers | [README](crates/eggpack-bootstrap/README.md) |
| `eggpack-ci` | CI graph projection + deterministic workflow render/drift | [README](crates/eggpack-ci/README.md) |
| `eggpack-github` | Staging payload + GitHub draft adapter (draft-only) | [README](crates/eggpack-github/README.md) |
| `eggpack-cli` | Deterministic `eggpack` binary wiring it all together | [README](crates/eggpack-cli/README.md) |

## Verify

```sh
scripts/check-local.sh   # full gate: fmt, check, clippy, tests, doc, package, MSRV
```

Faster loops and single-test invocation: [AGENTS.md](AGENTS.md#verify-trust-these-over-docs).

## Docs

- [docs/](docs/) — user guides (start with [quickstart](docs/quickstart.md)).
- [architecture/](architecture/) — crate deep dives, domain model, ADRs.
- [plans/registry.md](plans/registry.md) — status, blockers, next handoff
  (currently: Release Manifest M003, manual `eggpack-manifest 0.1.0` publication).
