# eggpack-cli

Minimal deterministic `eggpack` binary for CI generation and release orchestration.

- `eggpack ci generate --ci-plan <ReleaseCIPlanV1 JSON> --github-policy <GitHubPolicy JSON> --output <workflow>` renders deterministically, creates or atomically replaces only the explicit output path, rejects symlink outputs, performs no network or repository discovery, and prints a bounded summary.
- `eggpack ci check --ci-plan <ReleaseCIPlanV1 JSON> --github-policy <GitHubPolicy JSON> --workflow <existing>` renders in memory, compares with drift semantics (CRLF-to-LF only), exits 0 on exact match, nonzero on drift or invalid input, never modifies files, and prints a bounded diagnostic.
- Release-resolution runners. `ci _resolve-release --contract --pack-config --build-bindings --qualification-bindings --selected --tag --source-revision --template --source-root --output-plan --output-ci-plan --output-github-policy [--consumer-validators]` requires the checked-out `HEAD` under `--source-root` to equal `--source-revision`, then writes the `ReleasePlan` (intent), the `ReleaseCIPlanV1` (executable graph), and the resolved draft policy. `ci _verify-source --release-plan` re-checks that a plan's `source_revision` is 40 lowercase hex characters and that the inherited working directory matches it, so every later job in a generated workflow runs against the exact resolved source.
- Internal runner commands (`ci _capture-build`, `ci _qualify-target`, `ci _evaluate-gate`, `ci _aggregate`) are deterministic file-in/file-out wrappers over `eggpack-core` qualification/finalization APIs. They are not a generic scripting interface. `_capture-build` accepts `--cargo-target-dir` plus `--output-dir` to derive Cargo outputs and stage the canonical `build-handoff.json` + `candidates/` directory; `_qualify-target` copies the validated handoff and candidate bytes next to `evidence.json`; `_evaluate-gate` accepts `--inputs-dir` with the canonical per-target layout; `_qualify-target` accepts an optional `--qemu-sysroot` for Emulated targets. `_aggregate` additionally writes a standalone deterministic `release-manifest.json` beside `summary.json` (never inside the M004 root) and verifies it decodes back to the exact manifest.
- `ci _validate-consumer --consumer-validators --target --candidate-dir --build-handoff --evidence --source-root --output` runs the configured post-qualification consumer check for one target. It refuses fast unless the qualification evidence is `Passed` and its identity matches the handoff, so a failed candidate is never executed against.
- Draft staging commands (`ci _prepare-stage`, `ci _stage-github-draft`) are draft-only with no publication path. `_prepare-stage` materializes a complete deterministic staging payload (finalized assets, checksum sidecars, `release-manifest.json`, `install.sh`, `install.ps1`) with exact-tag origins. `_stage-github-draft` reconciles that payload into a GitHub draft via environment-only `GITHUB_TOKEN`, reusing exact drafts/assets and failing closed on mismatch; incomplete drafts may remain for maintainer inspection and later rerun.

Internal artifacts are not public releases. Optional failures suppress release output. Generated staging is implemented (M003b): the rendered `stage` job calls `_prepare-stage` then `_stage-github-draft` with the exact aggregate handoff; reruns reconcile exact state without clobber, and publication stays manual. Tool provisioning in generated CI is pinned to the official Eggpack repository at an exact revision. `ci check` is non-mutating.

## `eggpack contract expand` (Contract M003)

A bounded, local-only projection over existing `eggpack-contract` schema-v1
semantics. It lets a consumer repository read a producer fact from the contract
instead of re-implementing TOML parsing and template expansion.

```sh
eggpack contract expand \
  --contract distribution.toml \
  --release-id v1.2.7 \
  --target linux-x64 \
  --field asset
# eggsact-x86_64-unknown-linux-gnu
```

- **Local file input only.** One contract path, read through the CLI's bounded
  non-symlink reader with a 1 MiB limit. No git discovery, current-directory
  search, environment fallback, network, GitHub API, or release lookup.
- **Exactly one scalar.** `--field` selects one of `canonical-target`,
  `asset`, `sidecar`, or `install`. On success stdout is that value plus a
  single `\n` and stderr is empty; every failure exits nonzero, prints nothing
  to stdout, and writes a bounded diagnostic to stderr. Repeated runs over
  equal bytes and arguments are byte-identical.
- **`--release-id` is opaque.** It is an expansion input, not a release
  selector: Eggpack applies no SemVer ordering and chooses no release.
  Ordinary prerelease and build punctuation (`v1.2.3-rc.1+build.5`) is
  accepted; the existing contract path-safety rules still reject path
  separators and empty values.
- **Direct-only fields.** `asset`, `sidecar`, and `install` are valid only when
  the resolved target expands to a direct artifact. Bundle and archive targets
  fail closed with `selected --field <name> requires a direct artifact target`
  rather than choosing a primary entry or archive member.
  `canonical-target` is valid for every asset form.
- **Closed argument surface.** `--contract`, `--release-id`, `--target`, and
  `--field` are each required exactly once and accept `--flag value` or
  `--flag=value`. Duplicate flags, unknown options, empty values, positional
  arguments, and unsupported field names are rejected.
- **Read-only.** The contract and its directory are never modified.
- **Consumer policy stays consumer-owned.** Frozen public-name tables,
  latest/exact selection, Cargo fallback, install destinations, updater
  policy, and compatibility-floor checks are not provided here and must not be
  inferred from this output.

`eggpack-contract` remains the sole semantic authority: the command calls
`DistributionContract::parse_toml_str` and `DistributionContract::expand` and
adds no template grammar, target/alias resolution, or serialized expansion
document of its own.

The example above is executable and guarded by
`crates/eggpack-cli/tests/contract_expand.rs::the_documented_readme_example_runs_verbatim`,
which runs it with these exact relative paths from a temporary working
directory.
