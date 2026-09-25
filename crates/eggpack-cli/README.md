# eggpack-cli

Minimal deterministic `eggpack` binary for CI generation and release orchestration.

- `eggpack ci generate --ci-plan <ReleaseCIPlanV1 JSON> --github-policy <GitHubPolicy JSON> --output <workflow>` renders deterministically, creates or atomically replaces only the explicit output path, rejects symlink outputs, performs no network or repository discovery, and prints a bounded summary.
- `eggpack ci check --ci-plan <ReleaseCIPlanV1 JSON> --github-policy <GitHubPolicy JSON> --workflow <existing>` renders in memory, compares with drift semantics (CRLF-to-LF only), exits 0 on exact match, nonzero on drift or invalid input, never modifies files, and prints a bounded diagnostic.
- Internal runner commands (`ci _capture-build`, `ci _qualify-target`, `ci _evaluate-gate`, `ci _aggregate`) are deterministic file-in/file-out wrappers over `eggpack-core` qualification/finalization APIs. They are not a generic scripting interface.

Internal artifacts are not public releases. Optional failures suppress release output. Staging remains separate. Tool provisioning in generated CI is pinned to the official Eggpack repository at an exact revision. `ci check` is non-mutating.
