# eggpack

Eggpack is producer-side release tooling for Eggstack. It turns a versioned
distribution contract and producer policy into build candidates, qualification
evidence, finalized assets, and a Release Manifest. It can also render GitHub
Actions workflows and, when configured, stage a draft release.

Eggup handles consumer-side verified installs, updates, and rollbacks. Eggpack
does not install or update applications. Asset sizes and SHA-256 digests
describe integrity, not authenticity. Draft staging is optional, targets an
existing exact tag, and never publishes a release or changes tags.

## Start here

You need Rust 1.89 or newer and Git. Build the CLI with:

```sh
cargo build -p eggpack-cli
```

The [quickstart](docs/quickstart.md) creates a sample project, builds and
qualifies a Linux x86_64 release, and renders and checks its workflow. Run the
full local build and native qualification on a matching Linux x86_64 host.

## Main workflows

| Workflow | Commands | Purpose |
|---|---|---|
| Local release | `check`, `plan`, `package`, `qualify`, `collect`, `manifest` | Validate inputs, build and qualify candidates, finalize release files, and derive the manifest. |
| Workflow generation | `ci generate`, `ci check` | Render a deterministic GitHub Actions workflow and check a checked-in workflow for drift. |
| Contract lookup | `contract expand` | Print one bounded value expanded from a distribution contract. |

The local release commands use local files and do not call GitHub. `package`
runs Cargo with the configured toolchain, so Cargo may need network access to
download project dependencies. The `ci _*` commands are workflow runners, not a
general scripting interface. The quickstart uses `ci _resolve-release` to
resolve a sample release against its local Git checkout.

Generated workflow permissions depend on its policy. Draft staging is an
optional workflow step that requires write access and a GitHub token;
publication remains a separate human action.

## Documentation

- [Quickstart](docs/quickstart.md): complete sample release and workflow walkthrough.
- [Release preparation](docs/release-process.md): versioning, verification, and the release/publication boundary.
- [Release identity policy](docs/release-identity-policy.md): map `vMAJOR.MINOR.PATCH` tags to unprefixed manifest IDs.
- [CLI command reference](crates/eggpack-cli/README.md): commands, flags, inputs, outputs, and exit behavior.
- [Guide index](docs/README.md): user guides and reference links.
- [Architecture overview](architecture/overview.md): design boundaries and links to the detailed architecture documents.
