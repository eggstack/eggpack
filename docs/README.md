# docs

User guides for Eggpack. Design docs and crate internals live in
[../architecture/](../architecture/) (start at
[overview.md](../architecture/overview.md)); planning status in
[../plans/registry.md](../plans/registry.md); the operating contract for agents
and contributors in [../AGENTS.md](../AGENTS.md).

- [quickstart.md](quickstart.md) — resolve a one-target release and render its
  GitHub Actions workflow with the `eggpack` CLI (verified end-to-end, no
  network, ~5 minutes).

## Workspace layout

Seven crates, one direction of dependency: `contract`/`manifest` → `core` →
`bootstrap`/`github`/`ci` → `cli`.

| Crate | Role |
|---|---|
| [eggpack-contract](../crates/eggpack-contract/README.md) | Portable schema-v1 release layout authority + conformance validators |
| [eggpack-manifest](../crates/eggpack-manifest/README.md) | Bounded schema-v1 JSON evidence for finalized releases (the only published crate) |
| [eggpack-core](../crates/eggpack-core/README.md) | Planning, Cargo building, qualification, finalization |
| [eggpack-bootstrap](../crates/eggpack-bootstrap/README.md) | Deterministic first-install shell/PowerShell renderers |
| [eggpack-ci](../crates/eggpack-ci/README.md) | CI graph projection + deterministic workflow render and drift check |
| [eggpack-github](../crates/eggpack-github/README.md) | Staging payload + GitHub draft adapter (draft-only) |
| [eggpack-cli](../crates/eggpack-cli/README.md) | The `eggpack` binary wiring it together, and the full command reference |

Per-crate references are kept next to the code and are also rendered on
crates.io for the published manifest crate.
