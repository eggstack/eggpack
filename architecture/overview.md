# Eggpack Architecture Overview

Bird's-eye view of the Eggpack codebase: **producer-side** release construction
and distribution infrastructure for Eggstack. Eggpack centralizes portable
release contracts, target/build/qualification planning, artifact composition,
release manifests, bootstrap installers, generated release CI, and draft staging.

It deliberately does **not** replace Eggup: Eggup remains the consumer-side
verified installation/update/rollback layer, and the product repository owns
release/install policy. See [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md).

Code baseline `fa82486dab83d2369d330a3cef197a3651851c26` — the tree the line
counts and the `file:line` citations in this directory were measured and
verified against. That is the SHA the numbers below describe: this index changes
no Rust source, so the counts and citations stay true of the tree it commits onto.
Re-verify them against the workspace if you edit the overview again.
Milestone/evidence statuses are **not** restated here; `plans/registry.md` is the
authority for those, and [principles-roadmap.md](principles-roadmap.md) narrates
them.

### Citation-verification state

Citations in this directory are `file:line` references into
`crates/*/src/*.rs`, and they are the main reason the deep dives are worth
reading instead of the source. Keeping them true is a manual obligation, and it
has been missed before, so record what is actually true rather than what was
once true.

The last code commit, `61b2c03`, changed five source files
(`builder.rs`, `qualification.rs`, `ci/src/lib.rs`, `cli/src/main.rs`,
`github/src/lib.rs`). It also edited most of the deep dives, but **did not
re-base the line numbers** in six of them, so roughly ninety citations pointed at
the wrong code while still being in range. A later pass re-based them against
the current tree and verified each corrected anchor by name:

- `github.md` — 42 anchors, uniformly `+6`
- `ci.md` and `ci-rendering.md` — 62 anchors, `+2` above `ci/src/lib.rs:2200`
  and `+29` above `:4723`
- `process-execution.md` — the `eggpack-ci` row, `+2`
- `validation-model.md` — 23 anchors that cited a `#[derive]`/`#[serde]`
  attribute line rather than the item, plus the `ci` rows
- `cli.md` — the `git` site, which also described the pre-`61b2c03` design in
  which this crate spawned `git` itself
- line counts and one out-of-range range in `core-planning.md`

Two cautions learned the hard way. First, a file can be **partly** correct: in
several of these documents the anchors below a shift point were already right,
so a blanket re-base corrupts as much as it fixes — shift only the range the diff
actually moved, and confirm by symbol name, never by arithmetic alone. Second,
`determinism.md` and `testing-and-portability.md` were *reported* stale by an
automated pass and were in fact correct; the tell was that the cited line already
contained the named item.

`ci-consumer-seam.md` carried a known set of stale anchors into
`cli/main.rs` and `core/builder.rs`, both of which moved substantially in
`61b2c03`, and its line references were to be treated as indicative until
re-based by symbol name. **That gap is closed** — see the 2026-10-23 entry
below: every anchor in that file was re-based against the CI crate's module
tree, and the process-execution section it described was rewritten because CI
M003m had already made it false.

Contract M003 moved `cli/src/main.rs` again, and re-based `cli.md` for it. The
mechanical part was provable rather than assumed: the change is a pure `+254`
shift for every line at or below the rewritten dispatch block, verified
byte-for-byte on 397 anchors before being applied, with the 16 anchors inside
the rewritten block hand-written instead of shifted. That pass also surfaced
four citations in `cli.md` that were **already** wrong before M003 and are now
fixed: `reject_symlink_output` and `atomic_write` each pointed 7-10 lines past
its real anchor. A residual set of pre-existing drift in `cli.md` — the aggregate-sidecar,
`release-manifest.json` placement, path-absolutization call-site, and
end-to-end-orchestration (`1754`) sub-anchors — was left indicative rather than
re-based, because M003 could not confirm the intent of each anchor. **That gap
is closed**: the 2026-10-23 entry below resolved every one of them by symbol
name against the current tree. The rule from the `61b2c03` pass still holds:
re-base by reading the line and confirming it holds what the sentence claims,
never by arithmetic alone.

**2026-10-23 — post-implementation review baseline reconciliation (Planning
Hygiene M003).** Baseline: tree SHA
`fa82486dab83d2369d330a3cef197a3651851c26` — the SHA the line counts and the
`file:line` citations in this directory were measured and verified against. That
is the SHA the numbers describe; this pass changed no Rust source, so they stay
true of the tree it lands on.

Method: every `file:line` anchor in every document in `architecture/` was
extracted, the cited construct was located **by symbol name** in the tree at
that SHA, the cited line was read, and the sentence's claim was checked against
it. Where the claim could not be pinned to one line, the anchor became a
symbol-name reference. No anchor was shifted arithmetically and no number was
invented.

- Anchors checked by that pass: **2510** citation tokens. That is the pass's own
  count of every numeric line-reference token in `architecture/`, bare `:N`
  references included — it is **not** a count of distinct `file:line` citations.
  Measured on the current tree, `architecture/` holds **1144** `file:line`
  citation tokens across 21 documents (**1086** of them into `.rs` files), and
  `plans/` holds **181** more, so **1325** across `architecture/` and `plans/`.
  Measured with
  `grep -ohE '[A-Za-z0-9_./-]+\.[A-Za-z0-9]+:[0-9]+' architecture/*.md | wc -l`
  and the same expression over `architecture/ plans/`.
- Re-pointed at a verified line (the old line no longer held what the sentence
  claimed, or the construct had moved): **596**.
- Replaced by a symbol-name reference because no single line could carry the
  claim: **12**.
- Dropped where the claim was folded into a neighbouring citation: **11**.
- Added where a sentence gained a verified anchor: **50**.
- **Left indicative with a reason: 0** — but that count covers *single-line*
  anchors only, and the claim it made ("every anchor now names a line that was
  read, or names the construct symbolically") was too strong. The directory
  still carries **418** range-style anchors (`file.rs:N-M`), spread over **391**
  lines of **16** documents. A range is neither a single line nor a symbol-name
  reference: each was checked as a range, not as one read line. The per-document
  range-anchor line counts are `ci-consumer-seam.md` 81,
  `process-execution.md` 65, `ci-rendering.md` 58, `determinism.md` 54,
  `cli.md` 29, `validation-model.md` 27, `core-finalization.md` 19,
  `core-qualification.md` 13, `testing-and-portability.md` 11,
  `core-planning.md` 10, `github.md` 9, `contract.md` 9, `core-build.md` 2,
  `bootstrap.md` 2, `manifest.md` 1, `core.md` 1. Measured with
  `grep -ohE '\.rs:[0-9]+-[0-9]+' architecture/*.md | wc -l` for occurrences and
  `grep -lE '\.rs:[0-9]+-[0-9]+' architecture/*.md | xargs grep -cE '\.rs:[0-9]+-[0-9]+'`
  for lines.
- Documents whose prose changed because the code changed underneath them:
  `ci-consumer-seam.md` (the CI crate's module tree, and the private process
  runner that CI M003m replaced with core's shared bounded runner),
  `testing-and-portability.md` (the CI crate's test topology, the ignored-test
  count, and the CLI crate's integration targets), `determinism.md`,
  `validation-model.md`, `core-finalization.md`, `manifest.md`, `bootstrap.md`,
  `contract.md`, `cli.md`, `github.md`, and `core.md`.
- Documents whose cited files were unchanged since their last update, and whose
  anchors were therefore confirmed rather than re-pointed: `core-planning.md`
  and `core-qualification.md`. `ci.md` and `ci-rendering.md` were in this set in
  the 2026-10-23 pass, but that pass did not catch their prose errors: both
  described the CI crate as a thirteen-file module tree, and `ci-rendering.md`
  counted the crate's test functions wrongly. Both are corrected, so they
  belong with the re-pointed documents rather than the confirmed ones.
- The two standing gap notes above — `ci-consumer-seam.md`'s stale anchors and
  `cli.md`'s residual drift — are closed by this pass.
- The per-crate line counts and the pinned baseline were re-measured in the same
  pass: the CI crate is now a 14-file module tree, and the crate table records
  one row per crate with that crate's breakdown in its own card.

## How to read this document

This file has two jobs, deliberately:

1. **A general overview per component** — one card per crate and per
   cross-cutting concern, describing what that component owns, its entry
   points, and the invariants it is responsible for enforcing.
2. **An index for deep dives** — every card links to a dedicated
   `architecture/*.md` file covering that component in depth, with `file:line`
   references into the source.

Start here, then follow exactly one link into the area you are reviewing.

If you are an agent looking for task-shaped guidance rather than a subsystem
description, start at [`AGENTS.md`](../AGENTS.md) and then the
[skills index](../.skills/README.md). Skills are **derived** — they route you to
the documents below and carry no invariant of their own.

## Pipeline map

```text
  DistributionContract (contract)            portable expected layout — the single authority
          |
          v
  PackConfig --> ReleasePlan (core-planning)  invocation intent — pure, sorted, deterministic
          |
          +--> BuildBindingsV1 --> cargo / cargo-zigbuild --> BuildAttempt     (core-build)
          |         identity-bound to release + source revision; candidate bytes only
          |
          +--> QualificationBindingsV1 --> qualify_target --> Evidence          (core-qualification)
          |         host-matched proof; independent of which builder ran
          |
          v
  finalize_release --> FinalizedRelease + ReleaseManifest v1                   (core-finalization)
          |         gates on qualification, archives, sidecars, manifest over final bytes
          |
          +--> render_posix / render_powershell                                (bootstrap)
          |         first-install scripts from contract + manifest
          |
          +--> StagingPayloadV1 --> GitHub draft                               (github)
          |         draft-only, exact existing tag, never published
          |
          v
  CIPlan --> ReleaseCIPlanV1 --> generated release.yml                         (ci)
          |         checked in, drift-checked, replays the steps above in CI
          |
          v
  eggpack CLI                                                                    (cli)
                    deterministic wiring only — hand-rolled args, no clap
```

`eggpack-manifest` sits beside `core`: the leaf crate that owns the schema-v1
evidence document itself, consumed by `core` (which writes it), `bootstrap` and
`github` (which read it), and the published `eggup-eggpack` adapter downstream.

## Crate components

Dependency DAG (arrows point at dependencies):

```text
contract   manifest
    |         |
    +----+----+
         |
        core -------- bootstrap
         |  \           /
         |   \         /
         |     github
         |    /
         |   ci
         |   |
         +---+--> cli   (cli depends on all six)
```

| Crate | Lines | Deep dive |
|---|---|---|
| `eggpack-contract` | 2937 (2449 + 488 tests) | [contract.md](contract.md) |
| `eggpack-manifest` | 1536 | [manifest.md](manifest.md) |
| `eggpack-core` | 8935 (6 files) | [core.md](core.md) |
| `eggpack-bootstrap` | 3587 | [bootstrap.md](bootstrap.md) |
| `eggpack-ci` | 12945 (14 files) | [ci.md](ci.md) |
| `eggpack-github` | 5498 (3439 + 2059 tests) | [github.md](github.md) |
| `eggpack-cli` | 8523 (5711 + 2812 tests) | [cli.md](cli.md) |

Counts are Rust source lines per crate, measured against the baseline named
above: every `.rs` file under `crates/<crate>/`, production and test code alike.
A count in parentheses is the split inside the total — `production + tests`. The
test code lives in a `#[cfg(test)]` module in `src/` for every crate except
`contract` (whose conformance and fixture tests are separate `tests/` targets)
and `ci` and `github` (whose test modules are separate out-of-line files).

### `eggpack-contract` — layout authority and validators

Single `src/lib.rs`. Pure, sync, side-effect free: `serde` + `toml` only, no
I/O, no process execution, no network. Owns `DistributionContract` (schema-v1
expected layout: product identity, targets and aliases, asset names, sidecars,
bundle entries, archive members, install names) and the conformance validators
that compare a contract against an observed release. Key surface:
`SCHEMA_V1`, `MAX_OBSERVED_ENTRIES`, `DistributionContract::{parse_toml_str,
to_toml_string, resolve, expand}`, the `Expanded*` projection types,
`ExtrasPolicy`, `expected_release_files`, `ReleaseInventory`,
`validate_release_inventory`, `ArchiveMemberInventory`,
`validate_archive_member_inventory`, the `Observed*Mapping` types,
`ConformanceReport`, and `validate_observed_mapping`. Nothing downstream may
redefine a layout name; downstream crates reference these types instead.
**Deep dive:** [contract.md](contract.md).

### `eggpack-manifest` — final-bytes evidence document

Single `src/lib.rs`. Leaf parser/serializer with no I/O (`serde` + `serde_json`
only), and the only crate here that is published for third-party consumption
(`eggpack-manifest 0.1.0` on crates.io, consumed by `eggup-eggpack`). Owns
`ReleaseManifest` plus `TargetRecord`, `ArtifactForm`, `ArtifactRecord`,
`BundleRecord`, `ArchiveMemberRecord`, and `ByteEvidence`, with hard bounds
`MAX_DOCUMENT_BYTES`, `MAX_TARGETS`, `MAX_RECORDS`, and
`MAX_EVIDENCE_REFERENCES`. It describes **final bytes only** — never intent,
never installed state.
**Deep dive:** [manifest.md](manifest.md).

### `eggpack-core` — the producer pipeline

Six files. The only crate that writes release artifacts and the only one that
spawns toolchain processes (Cargo, `cargo zigbuild`, Zig, QEMU). It also owns the
single bounded runner that `eggpack-cli` and `eggpack-ci` use — for `git`
revision verification and for the consumer validator:

| File | Lines | Stage | Deep dive |
|---|---|---|---|
| `src/lib.rs` | 1384 | `PackConfig` → `ReleasePlan`, policy types, `build_manifest` | [core-planning.md](core-planning.md) |
| `src/builder.rs` | 1902 | bindings, command specs, bounded execution, candidate discovery | [core-build.md](core-build.md) |
| `src/qualification.rs` | 2982 | qualification methods, host matching, evidence | [core-qualification.md](core-qualification.md) |
| `src/finalization.rs` | 928 | `finalize_release`, archive assembly, manifest aggregation | [core-finalization.md](core-finalization.md) |
| `src/digest.rs` | 327 | the one bounded file-digest authority | [core-finalization.md](core-finalization.md) |
| `src/floor.rs` | 1412 | bounded ELF compatibility-floor inspection | [core-qualification.md](core-qualification.md) |

Crate-level orientation, dependency direction, and the cross-file data
contracts live in [core.md](core.md).
**Deep dive:** [core.md](core.md) → the four stage files above.

### `eggpack-bootstrap` — first-install script generation

Single `src/lib.rs`, depends on `contract` + `manifest` only. Renders
release-specific, non-interactive first-install scripts from the contract layout
and the finalized manifest: `render_posix` / `render_powershell` (spec-driven)
and `render_posix_with_policy` / `render_powershell_with_policy` (policy-driven,
via `BootstrapInstallPolicyV1`, `TargetInstallPolicy`, `InstallMode`,
`BundleArchiveEncoding`). It performs **no release selection and no updates** —
that is Eggup's job. Emitted SHA-256 values are integrity facts, never
authenticity or provenance claims.
**Deep dive:** [bootstrap.md](bootstrap.md).

### `eggpack-ci` — release CI planning and workflow rendering

A 14-file module tree at 12885 lines, the largest component in the workspace.
Depends on `contract` + `core`. Projects a release into a provider-neutral
graph (`project_ci_plan` → `CIPlan` → `TargetJob`; `project_release_plan` →
`ReleaseCIPlanV1` with qualification, gate, aggregate/finalize, and staging
jobs) and renders deterministic GitHub Actions YAML from caller-supplied
runner and action-pin policy (`render_github`, `render_release_github`,
`render_reusable_release_github`), with byte-level drift comparison
(`check_github`, `check_release_github`, `check_reusable_release_github`) and
`DriftReport`. Also owns the Zig/cargo-zigbuild provisioning policy
(`CrossToolProvisioningV1`, `zig_download_url`, `zig_expected_digest`),
build/qualification artifact handoff formats, the external consumer validator
(`run_consumer_validator`, `ConsumerValidatorV1`), and the reusable-workflow
runtime identity contract (`ReleaseWorkflowShapeV1`,
`resolve_runtime_release_plan`).

| File | Lines | Concern |
|---|---|---|
| `src/lib.rs` | 59 | crate root: module declarations and re-exports |
| `src/error.rs` | 19 | `CiError` |
| `src/validate.rs` | 60 | shared string rules (`safe_target`, `safe_identifier`, `safe_metadata`, `safe_tool_version`) |
| `src/planning.rs` | 370 | `CIPlan` projection and validation |
| `src/policy.rs` | 739 | policy types, Zig/cross-tool provisioning, shared path constants |
| `src/runner.rs` | 475 | `RunnerCommand`, the finite runner-command enum |
| `src/render_github.rs` | 256 | `render_github` and the YAML scalar quoter |
| `src/drift.rs` | 106 | byte-level drift comparison |
| `src/handoff.rs` | 390 | build/qualification handoff documents |
| `src/release_plan.rs` | 655 | `ReleaseCIPlanV1` and the release-plan projection |
| `src/render_release.rs` | 1403 | release and reusable-workflow rendering |
| `src/consumer_validator.rs` | 560 | the external consumer validator |
| `src/identity.rs` | 215 | reusable-workflow runtime identity |
| `src/tests.rs` | 7638 | the crate's test module, declared out of line |

**Deep dive:** [ci.md](ci.md) → [ci-rendering.md](ci-rendering.md),
[ci-consumer-seam.md](ci-consumer-seam.md).

### `eggpack-github` — staging payload and draft adapter

`src/lib.rs` (3439) + `src/tests.rs` (2059, a `#[cfg(test)]` module). Note that
the in-memory GitHub double `FixtureGithub` and its fault injectors live in
`src/lib.rs`, **not** in `tests.rs`, and are not test-gated — they ship in the
crate's public API. Depends on `contract`,
`manifest`, `core`, and `bootstrap`; adds `tokio` + `eggfetch-core 0.2.0` for
HTTP. Materializes a local `StagingPayloadV1` from a finalized release
(`prepare_staging_payload`, `prepare_staging_payload_with_presentation`) and
reconciles it into a **draft** through the `GithubApi` trait
(`EggfetchTransport` in production, `FixtureGithub` for tests), producing a
`GitHubDraftReceiptV1`. Presentation/wrapper policy
(`InstallerPresentationV1`, `ProductWrapperSourcesV1`, `GitHubDraftTemplateV1`)
is generated by default rather than hand-written.
**Deep dive:** [github.md](github.md).

### `eggpack-cli` — deterministic wiring

`src/main.rs` (4617) plus `src/operator.rs` (1094), binary `eggpack`, no
`clap`. Hand-rolled dispatch in `dispatch` (`src/main.rs:24`) with the
`eggpack ci` family and six local producer operator commands (`check`, `plan`,
`package`, `qualify`, `collect`, `manifest`) — plus nine narrow internal
`ci _*` runner commands (`_verify-source`, `_resolve-release`, `_capture-build`,
`_qualify-target`, `_validate-consumer`, `_evaluate-gate`, `_aggregate`,
`_prepare-stage`, `_stage-github-draft`) that the generated workflow invokes as
file-in/file-out steps. Exit code 0 on success, 1 with a single-line
`eggpack: <message>` on failure. Owns the atomic-write, bounded-read,
symlink-rejection, and relative-path-absolutization helpers that make the
generated steps safe.

| File | Lines | Concern |
|---|---|---|
| `src/main.rs` | 4617 | dispatch, the `ci` family, `contract expand`, file-safety helpers |
| `src/operator.rs` | 1094 | the six local producer operator commands |
| `tests/contract_expand.rs` | 381 | `eggpack contract expand` integration tests (8 tests) |
| `tests/operator_commands.rs` | 2431 | operator-family integration tests (30 tests) |

The operator family gained three optional flags and three tightened behaviours
in the 2026-10-23 review chain; `cli.md` documents each in full, and this card
records only the shape. The flags are `--qemu-sysroot` (the caller's QEMU
sysroot for an emulated target, exactly as the `ci _qualify-target` runner takes
it), `--identity-mode` (a caller-chosen release-identity mode rather than a
hardcoded constant), and `--consumer-validators` (the checked-in validator map
path). The producer policy and both bindings documents are read as **TOML or
JSON** through one strict reader (`src/operator.rs:458-470`), so the bounds,
unknown-field rejection, and strictness do not diverge from the CI path's.
`qualify` refuses `--output-dir == --artifacts-dir` before writing anything
(`src/operator.rs:693-695`): staging the qualification artifacts into the
directory the candidates were read from would rewrite the very bytes the
qualification is about. The derived manifest is **self-checking** — it reads
every archive member back out of the finalized `.tar.gz` and compares it
against the collected manifest (`src/operator.rs:973-991`,
`digest_archive_member` `:1056`), so a manifest that disagrees with the archive
it describes is refused rather than emitted. Staging never truncates a
candidate.

It also owns `eggpack contract expand` (Contract M003), the one consumer-facing
scalar projection: read one local contract, print exactly one expanded name.
It delegates to `eggpack-contract` and adds no template grammar, no target
resolution, and no serialized expansion document. It is deliberately local-only —
no repository discovery, no network, no release selection — and it never implies
consumer policy such as latest/exact selection or Cargo fallback.
**Deep dive:** [cli.md](cli.md).

## Cross-cutting components

These are the review units that cut across crates. Each has a dedicated file.

| Component | What it governs | Deep dive |
|---|---|---|
| Determinism & serialization | canonical sort, stable field order, sorted inventories/findings, byte-identical renders and archives | [determinism.md](determinism.md) |
| Fail-closed validation model | `deny_unknown_fields`, `schema_version == 1`, bounded counts/sizes, exact-match resolution, `AllowExtras`/`Exact` | [validation-model.md](validation-model.md) |
| Process execution & environment | bounded cancellable process groups, cleared env with allowlist, MSVC init, no output leakage | [process-execution.md](process-execution.md) |
| Test topology & portability | inline unit tests, fixture corpora, Windows/macOS lanes, MSRV, `scripts/check-local.sh` | [testing-and-portability.md](testing-and-portability.md) |
| Planning & governance | `plans/` layout, closure discipline, ADRs, registry as control surface | [planning-and-governance.md](planning-and-governance.md) |
| Domain model, ADRs, roadmap phases | four-object split (Contract / Plan / Manifest / Receipt), ADR-0001…0005, `dist` disposition C | [principles-roadmap.md](principles-roadmap.md) |
| Eggup consumer mapping | how a `ReleaseManifest v1` maps onto Eggup install receipts | [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md) |

## Data flow (producer pipeline)

| # | Stage | Owner | Output |
|---|---|---|---|
| 1 | Author the distribution contract (TOML, schema-v1) | [contract](contract.md) | `DistributionContract` |
| 2 | Resolve policy against the contract | [core-planning](core-planning.md) | `ReleasePlan` (canonically sorted) |
| 3 | Declare build and qualification bindings, validated for exact plan coverage | [core-build](core-build.md), [core-qualification](core-qualification.md) | `BuildBindingsV1`, `QualificationBindingsV1` |
| 4 | Build via first-party `cargo` / `cargo zigbuild` | [core-build](core-build.md) | `BuildAttempt` (candidate bytes) |
| 5 | Qualify each target on a host-matched runtime | [core-qualification](core-qualification.md) | `QualificationEvidence` |
| 6 | Finalize: gate, copy under contract names, assemble `.tar.gz`, write sidecars, aggregate the manifest | [core-finalization](core-finalization.md) | `FinalizedRelease` + `ReleaseManifest v1` |
| 7 | Render installers and stage a draft | [bootstrap](bootstrap.md), [github](github.md) | scripts, `StagingPayloadV1`, `GitHubDraftReceiptV1` |
| 8 | Project the whole flow into checked-in CI and check for drift | [ci](ci.md), [cli](cli.md) | `release.yml` |

## Key invariants (apply everywhere)

- **Authority separation:** Contract owns layout/names; `PackConfig` owns policy;
  `ReleasePlan` is intent, not evidence; `ReleaseManifest` describes final bytes
  only; Eggup receipts describe installed state. No layer redefines another's
  names.
- **Determinism:** canonical sort, stable serialization, sorted inventories and
  findings, byte-identical renders for identical inputs. See
  [determinism.md](determinism.md).
- **Fail-closed validation:** `deny_unknown_fields`, `schema_version == 1`,
  bounded counts/sizes, exact-match resolution (no guessing where identity
  matters), `AllowExtras` default with opt-in `Exact`. See
  [validation-model.md](validation-model.md).
- **Integrity ≠ authenticity:** SHA-256 digests and sizes are integrity facts
  only. No signature, attestation, or provenance claim is made anywhere.
- **Publication is gated:** staging targets drafts only, on the exact existing
  tag. No `--clobber`, no tag mutation, no auto-publish, no immutable
  overwrite. Publication is a separate human action.
- **Safety:** `#![forbid(unsafe_code)]` workspace-wide (`unsafe_code = "deny"`),
  no shell interpretation of generated inputs, bounded process execution with
  timeouts/output limits/cancellation, environment allowlists, symlink
  rejection on output paths. No production spawn site is unbounded: `core`,
  `cli`, and `ci` share one bounded runner, and every supervised process is
  terminated as a process group or job object.

## Review paths

Pick the path that matches the question, then read only the linked files.

- **"What does this repo do and how do the pieces fit?"** — this file, then
  [principles-roadmap.md](principles-roadmap.md) §1 for the domain model.
- **"Where is a name defined?"** — [contract.md](contract.md). Contract is the
  only authority; everything else references it.
- **"How do artifacts and evidence get produced?"** — [core.md](core.md) and
  its four stage files, in pipeline order.
- **"What ends up in the release?"** — [core-finalization.md](core-finalization.md)
  then [manifest.md](manifest.md).
- **"What does a user run first?"** — [bootstrap.md](bootstrap.md).
- **"What runs in CI, and is the checked-in workflow still correct?"** —
  [ci.md](ci.md) and [ci-rendering.md](ci-rendering.md).
- **"How does anything reach a human?"** — [github.md](github.md) (draft only)
  and [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md).
- **"Is this deterministic / validated / safe to run?"** —
  [determinism.md](determinism.md), [validation-model.md](validation-model.md),
  [process-execution.md](process-execution.md).
- **"Can I trust the tests?"** — [testing-and-portability.md](testing-and-portability.md).
- **"What is done, blocked, or next?"** — `plans/registry.md`, narrated in
  [planning-and-governance.md](planning-and-governance.md).
- **"What is the process for opening or closing a milestone?"** — the
  `planning-and-closure` skill, which routes to
  [planning-and-governance.md](planning-and-governance.md) and
  `plans/003-planning-process.md`.
- **"Am I allowed to publish or overwrite something?"** —
  [github.md](github.md) draft-only boundary, and the
  `generated-ci-and-draft-staging` skill.
