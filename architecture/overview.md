# Eggpack Architecture Overview

Bird's-eye view of the Eggpack codebase: **producer-side** release construction
and distribution infrastructure for Eggstack. Eggpack centralizes portable
release contracts, target/build/qualification planning, artifact composition,
release manifests, bootstrap installers, generated release CI, and draft staging.

It deliberately does **not** replace Eggup: Eggup remains the consumer-side
verified installation/update/rollback layer, and the product repository owns
release/install policy. See [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md).

Code baseline `61b2c03` (the commit that bounds the `git` verification path
recorded in [process-execution.md](process-execution.md)). Line counts below are
measured against that baseline and were re-verified when this index was last
touched — re-verify them against the workspace if you edit the overview again.
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

`ci-consumer-seam.md` still carries a known set of stale anchors into
`cli/main.rs` and `core/builder.rs`, both of which moved substantially in
`61b2c03`. Treat its line references as indicative until they are re-based; its
prose and the constants it names are unaffected. Anything you re-base should be
verified the same way — by reading the line and confirming it is the thing the
sentence claims.

Contract M003 moved `cli/src/main.rs` again, and re-based `cli.md` for it. The
mechanical part was provable rather than assumed: the change is a pure `+254`
shift for every line at or below the rewritten dispatch block, verified
byte-for-byte on 397 anchors before being applied, with the 16 anchors inside
the rewritten block hand-written instead of shifted. That pass also surfaced
four citations in `cli.md` that were **already** wrong before M003 and are now
fixed: `reject_symlink_output` and `atomic_write` each pointed 7-10 lines past
its real anchor. A residual set of pre-existing drift remains in `cli.md` in
sections M003 did not touch — the aggregate-sidecar, `release-manifest.json`
placement, path-absolutization call-site, and end-to-end-orchestration
(`1754`) sub-anchors all resolve to unrelated lines in the pre-M003 file as
well, so M003 carried them forward faithfully rather than inventing new numbers
whose intent it could not confirm. Treat those as indicative until re-based by
symbol name. The rule from the `61b2c03` pass holds: re-base by reading the line
and confirming it holds what the sentence claims, never by arithmetic alone.

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
| `eggpack-contract` | 2302 (+405 tests) | [contract.md](contract.md) |
| `eggpack-manifest` | 1354 | [manifest.md](manifest.md) |
| `eggpack-core` | 5998 (4 files) | [core.md](core.md) |
| `eggpack-bootstrap` | 3436 | [bootstrap.md](bootstrap.md) |
| `eggpack-ci` | 10256 | [ci.md](ci.md) |
| `eggpack-github` | 4873 (3025 + 1848 tests) | [github.md](github.md) |
| `eggpack-cli` | 3803 src + 381 integration tests | [cli.md](cli.md) |

Counts are Rust source lines per crate. A count in parentheses is test code
included in the total; `contract` is the only crate whose production line count
is not its whole-crate count, because its conformance and fixture tests live in
`tests/` rather than a `#[cfg(test)]` module.

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

Four files. The only crate that writes release artifacts and the only one that
spawns toolchain processes (Cargo, `cargo zigbuild`, Zig, QEMU). It also owns the
single bounded runner that `eggpack-cli` uses for `git` revision verification:

| File | Lines | Stage | Deep dive |
|---|---|---|---|
| `src/lib.rs` | 1056 | `PackConfig` → `ReleasePlan`, policy types, `build_manifest` | [core-planning.md](core-planning.md) |
| `src/builder.rs` | 1461 | bindings, command specs, bounded execution, candidate discovery | [core-build.md](core-build.md) |
| `src/qualification.rs` | 2621 | qualification methods, host matching, evidence | [core-qualification.md](core-qualification.md) |
| `src/finalization.rs` | 860 | `finalize_release`, archive assembly, manifest aggregation | [core-finalization.md](core-finalization.md) |

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

Single `src/lib.rs` at 10256 lines, the largest component in the workspace.
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
**Deep dive:** [ci.md](ci.md) → [ci-rendering.md](ci-rendering.md),
[ci-consumer-seam.md](ci-consumer-seam.md).

### `eggpack-github` — staging payload and draft adapter

`src/lib.rs` (3025) + `src/tests.rs` (1848, a `#[cfg(test)]` module). Note that
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

Single `src/main.rs`, binary `eggpack`, no `clap`. Hand-rolled dispatch in
`dispatch` (`src/main.rs:21`) with two user-facing subcommands —
`eggpack ci generate` and `eggpack ci check` — plus nine narrow internal
`ci _*` runner commands (`_verify-source`, `_resolve-release`, `_capture-build`,
`_qualify-target`, `_validate-consumer`, `_evaluate-gate`, `_aggregate`,
`_prepare-stage`, `_stage-github-draft`) that the generated workflow invokes as
file-in/file-out steps. Exit code 0 on success, 1 with a single-line
`eggpack: <message>` on failure. Owns the atomic-write, bounded-read,
symlink-rejection, and relative-path-absolutization helpers that make the
generated steps safe.

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
  rejection on output paths. No production spawn site is unbounded: `core` and
  `cli` share one bounded runner, and `ci`'s independent reimplementation is the
  only site without process-group kill.

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
