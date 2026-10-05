# Contract and Conformance Milestone 003 — Bounded Direct-Contract Expansion CLI

Status: ready

Repository implementation baseline: `911da48c7c1c7967397a2d190730fc643c8c6dc3`

Source roadmap: `plans/subsystems/contract-conformance-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md#phase-2--contract-conformance-engine`
- `plans/003-planning-process.md`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: polish / developer-facing capability

## 1. Objective

Add the smallest bounded local CLI surface that lets consumer repositories reuse Eggpack's existing `DistributionContract` parse/resolve/expand semantics instead of re-implementing producer-owned TOML parsing and template expansion.

The concrete M003 surface is:

```text
eggpack contract expand \
  --contract <distribution.toml> \
  --release-id <opaque-safe-release-id> \
  --target <canonical-triple-or-alias> \
  --field <canonical-target|asset|sidecar|install>
```

The command reads one local contract, expands one target using the existing schema-v1 implementation, and writes exactly one scalar value plus a trailing newline to stdout.

M003 is intentionally direct-artifact-only for `asset`, `sidecar`, and `install`, because the evidence that justified this milestone comes from two independent direct-binary consumers. Bundle/archive CLI projection remains evidence-gated future work.

This milestone does not add another release schema, another source of artifact identity, repository discovery, network access, release selection, or a general CLI framework.

## 2. Why this is ready

The original M003 gate was first-consumer evidence. Planning Hygiene M001 closed that gate.

Observed duplication now exists across two adopted consumers and three independently maintained implementations:

- `eggstack/eggsact: scripts/check-release-contract.py`;
- `eggstack/stegoeggo: scripts/check-release-contract.py`;
- `eggstack/stegoeggo: scripts/release-check-assets.sh`'s Python contract-expansion block.

All independently:

1. read `release/eggpack/distribution.toml`;
2. parse it outside Eggpack;
3. reproduce `{product}` / `{target}` direct-asset expansion;
4. compare the result with product-owned frozen names or release facts.

Eggpack already owns the relevant semantics:

- `DistributionContract::parse_toml_str`;
- target/alias resolution;
- `DistributionContract::expand`;
- `expected_release_files`.

The CLI already depends on `eggpack-contract` directly. No new crate dependency or architecture decision is required.

## 3. Current evidence and design decision

### 3.1 Why a scalar query rather than a new JSON document

The goal is to expose existing contract semantics, not establish a second durable machine-readable schema.

A new serialized `ExpandedTargetV1` document would become a compatibility surface of its own and risks crossing the planning-process ADR threshold for a new public schema contract.

M003 instead exposes one field at a time. That keeps the interface narrow and lets shell/Python consumers compare producer facts without decoding another Eggpack document.

### 3.2 Supported fields

`canonical-target`

- valid for any successfully resolved target/alias;
- prints the canonical target triple.

`asset`, `sidecar`, `install`

- valid only when the resolved target's `ExpandedAssets` is `Direct`;
- return the existing `ExpandedDirect` values;
- fail closed for bundle/archive forms with a bounded diagnostic.

The direct-only restriction is deliberate. Do not invent list syntax for bundle entries or archive members without real consumer evidence.

### 3.3 Release identity

`--release-id` is the existing opaque expansion input currently called `version` inside the contract library. The CLI does not apply SemVer ordering or decide which release should be used.

Use the existing contract validation/path-safety rules. Do not add a second release-id grammar unless an actual mismatch is found.

## 4. Invariants

- `eggpack-contract` remains the sole semantic authority for schema-v1 parsing and expansion.
- The CLI MUST call existing contract APIs; it MUST NOT duplicate template grammar or target/alias resolution.
- Local file input only. No Git discovery, current-directory search, environment fallback, network, GitHub API, or release lookup.
- Contract input is bounded before allocation and symlink-refused using the CLI's existing bounded-input pattern.
- Unknown schema versions/fields/targets/aliases fail closed.
- Direct `asset` / `sidecar` / `install` queries fail for bundle/archive targets rather than guessing a primary member.
- stdout contains only the selected scalar plus `\n` on success. Diagnostics go to stderr.
- Output is deterministic for equal bytes + arguments.
- The command never mutates the contract or any repository file.
- Product-owned frozen names, fallback policy, install destinations, updater policy, and GLIBC checks remain outside Eggpack.
- This milestone does not authorize consumer-repository changes; those require their own plans if/when consumers adopt the command.
- No general subcommand framework or plugin mechanism is introduced.

## 5. In scope

### Production

- extend the top-level `eggpack` dispatch with a `contract` command family;
- implement only `contract expand`;
- reuse `read_bounded` (or a shared equivalent) for at most 1 MiB contract input;
- parse through `DistributionContract::parse_toml_str`;
- resolve/expand through the existing contract API;
- project one scalar field;
- bounded deterministic CLI diagnostics;
- help/usage text.

### Tests

- CLI unit/integration tests for the new command;
- direct simple fixture coverage;
- alias resolution;
- opaque safe release id containing normal prerelease punctuation;
- all four fields;
- invalid contract/schema/unknown target;
- bundle/archive rejection for direct-only fields;
- oversized/symlink input rejection;
- argument-count/unknown-field rejection;
- deterministic repeated output;
- consumer-shaped comparison fixtures using the current Eggsact/StegoEggo contract shape.

### Documentation

- root README command summary;
- `crates/eggpack-cli/README.md`;
- `architecture/cli.md`;
- `architecture/contract.md` if needed to point from library semantics to the thin CLI projection;
- roadmap/registry/closure.

## 6. Out of scope

- JSON/TOML expansion documents;
- `contract generate`, `contract edit`, or schema migration commands;
- bundle/archive list output;
- observed-mapping conformance CLI;
- parsing consumer source code;
- rewriting Eggsact or StegoEggo scripts;
- release-manifest inspection;
- package publication;
- network/GitHub access;
- latest/version selection;
- changing `DistributionContract` schema v1;
- a new ADR.

If implementation discovers that replacing the observed consumer duplication requires a durable multi-record output schema or generic query language, stop and re-plan rather than broadening M003.

## 7. Required production changes

### A. Top-level dispatch

Extend usage to include:

```text
eggpack contract expand ...
eggpack ci ...
```

Keep the current direct dispatch style; do not introduce a CLI framework dependency solely for this command.

### B. Argument contract

Required flags:

- `--contract`;
- `--release-id`;
- `--target`;
- `--field`.

Reject:

- duplicates if current flag helper semantics cannot unambiguously handle them;
- unknown extra positional/flag arguments;
- missing/empty field selection;
- unsupported field names.

If the existing generic flag helper silently tolerates duplicate flags, M003 SHOULD add a command-local exact argument parser rather than changing all historical CI commands incidentally.

### C. Field projection

Implementation MUST pattern-match the existing `ExpandedAssets` result.

For `Direct`:

- `asset` -> `asset_file`;
- `sidecar` -> `sidecar_file`;
- `install` -> `install_name`.

For all forms:

- `canonical-target` -> `ExpandedTarget::triple`.

For `Bundle` / `Archive` plus a direct field, return a bounded error such as `selected field requires a direct artifact target`.

Do not choose the first bundle entry or archive member.

### D. No new serialized model

Do not derive/introduce serialization solely for `ExpandedTarget` in this milestone.

## 8. Ordered work packages

1. Add command-local argument parsing/usage tests.
2. Add bounded local contract read + existing parser call.
3. Add target expansion and scalar field projection.
4. Add positive/negative CLI tests including bundle/archive refusal.
5. Add consumer-shaped parity tests proving the command returns the same producer facts currently reconstructed manually.
6. Update user/developer docs and architecture.
7. Run stable/MSRV/package/workspace qualification.
8. Write `plans/closure/contract-conformance/003-status.md` and reconcile roadmap/registry.

## 9. Failure and restart semantics

The command is read-only and side-effect free.

Any parse, bounds, target, field, or expansion failure:

- exits nonzero;
- prints no partial scalar result to stdout;
- emits a bounded diagnostic to stderr;
- leaves every file unchanged.

A rerun with the same inputs is deterministic and safe.

## 10. Compatibility and migration

This is additive CLI behavior.

- Existing `eggpack ci` commands remain byte/behavior compatible.
- No existing contract/library API changes are required.
- No consumer must migrate as part of M003 closure.
- Later consumer plans may replace only their duplicated producer-fact parsing with this command; product-owned assertions remain local.
- A consumer must pin the Eggpack revision/version providing the command rather than invoke mutable `main`.

## 11. Required tests and verification

At minimum:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-cli --locked
cargo package -p eggpack-cli --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-cli --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Manual/scratch proof before closure:

- run the command against current Eggsact `release/eggpack/distribution.toml` for every published target and compare `asset` to its product-owned frozen table;
- run the same proof against StegoEggo;
- do not modify either external repository.

Record exact external baselines used.

## 12. Documentation requirements

Document:

- local-only behavior;
- direct-only field scope;
- release-id opacity;
- stdout/stderr contract;
- no release selection;
- no network/repository discovery;
- bundle/archive refusal;
- consumer policy remains consumer-owned.

The CLI README example MUST be executable and guarded by a test or a documented verification command so it does not repeat the prior unguarded-example drift finding.

## 13. Acceptance criteria

M003 closes when:

1. `eggpack contract expand` exists with the exact bounded flags above;
2. all successful queries delegate to existing `eggpack-contract` semantics;
3. direct target aliases and canonical targets return the expected scalar facts;
4. bundle/archive direct-field queries fail closed;
5. no new serialized expansion schema exists;
6. no existing `eggpack ci` behavior regresses;
7. consumer-shaped scratch comparisons match current Eggsact and StegoEggo producer facts;
8. stable, MSRV, package, docs, and hosted CI evidence are green;
9. no unresolved medium-or-higher correctness/security finding remains;
10. closure records whether consumer cleanup is now eligible but does not claim external migration.

## 14. Stop conditions

Stop and re-plan if:

- the command needs repository discovery or network access;
- direct consumer duplication cannot be removed without a multi-record public schema;
- bundle/archive support becomes necessary for this milestone;
- existing schema-v1 semantics are insufficient or must change;
- implementation requires a new CLI framework/general query language;
- a medium-or-higher defect is found in existing contract expansion rather than this projection.

## 15. Closure evidence

Create `plans/closure/contract-conformance/003-status.md` recording:

- implementation SHA;
- command/field contract;
- requirement-to-test matrix;
- Eggsact/StegoEggo scratch parity baselines/results;
- stable/MSRV/package/hosted CI;
- dependency tree;
- changed files;
- unresolved findings;
- consumer cleanup disposition;
- next contract-conformance handoff.
