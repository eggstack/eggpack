# Eggpack user guides

Start with the [quickstart](quickstart.md) to build a sample release and
render its GitHub Actions workflow. The walkthrough is designed for a Linux
x86_64 host because it builds and natively qualifies a Linux x86_64 target.

- [Quickstart](quickstart.md): create the sample inputs, build and qualify a
  release, derive its manifest, and check a generated workflow.
- [Release preparation](release-process.md): versioning, verification, and the
  release/publication boundary for Eggpack itself.
- [Release identity policy](release-identity-policy.md): optionally map
  `vMAJOR.MINOR.PATCH` tags to unprefixed manifest IDs.

## References

- [CLI command reference](../crates/eggpack-cli/README.md): command usage, flags, outputs, and diagnostics.
- [Architecture overview](../architecture/overview.md): system boundaries and links to the detailed design documents.
- Crate READMEs beside each crate document its Rust API and responsibilities.
