# docs

User guides for Eggpack. Design docs and crate internals live in
[../architecture/](../architecture/) (start at
[overview.md](../architecture/overview.md)); planning status in
[../plans/registry.md](../plans/registry.md); the operating contract for agents
and contributors in [../AGENTS.md](../AGENTS.md).

- [quickstart.md](quickstart.md) — resolve a one-target release and render its
  GitHub Actions workflow with the `eggpack` CLI (verified end-to-end, no
  network, ~5 minutes).

Per-crate references (kept next to the code, also rendered on crates.io):

- [../crates/eggpack-contract/README.md](../crates/eggpack-contract/README.md) — contract schema + conformance checks
- [../crates/eggpack-manifest/README.md](../crates/eggpack-manifest/README.md) — manifest format + consumer integration
- [../crates/eggpack-core/README.md](../crates/eggpack-core/README.md) — planning, building, qualification, finalization
- [../crates/eggpack-bootstrap/README.md](../crates/eggpack-bootstrap/README.md) — installer generation
- [../crates/eggpack-ci/README.md](../crates/eggpack-ci/README.md) — CI graph, rendering, drift, staging, consumer seam
- [../crates/eggpack-github/README.md](../crates/eggpack-github/README.md) — draft staging adapter
- [../crates/eggpack-cli/README.md](../crates/eggpack-cli/README.md) — CLI command reference
