# `eggpack-core` — Deep Dive

Crate-level orientation for `eggpack-core`: the producer pipeline. This file
explains how the four source files divide the work and what flows between them.
It does not restate the per-file detail — each stage has its own deep dive.

`eggpack-core` is the only crate that writes **release artifacts** (finalized
output roots, archives, sidecars) and the only one that spawns **toolchain**
processes as part of the release pipeline — Cargo, `cargo zigbuild`, Zig, and
QEMU. Other crates touch the filesystem or spawn processes in narrower ways:
`eggpack-cli` writes command outputs and shells out to `git` for source-revision
verification, and `eggpack-ci` runs a caller-supplied external validator. `core`
is the only crate that produces the bytes a release is made of.
`contract` and `manifest` are pure leaves,
`bootstrap` and `ci` derive documents from core's outputs, `github` consumes
core's finalized release, and `cli` wires all of it.

Code baseline `fc072af`; 5827 lines across four files.

## File map

| File | Lines | Owns | Deep dive |
|---|---|---|---|
| `src/lib.rs` | 1056 | policy vocabulary, `PackConfig` -> `ReleasePlan`, `build_manifest` | [core-planning.md](core-planning.md) |
| `src/builder.rs` | 1301 | build bindings, command construction, bounded execution, candidate discovery | [core-build.md](core-build.md) |
| `src/qualification.rs` | 2610 | qualification bindings, method taxonomy, host matching, evidence | [core-qualification.md](core-qualification.md) |
| `src/finalization.rs` | 860 | qualification gate, output root, archives, sidecars, manifest | [core-finalization.md](core-finalization.md) |

Read them in pipeline order. The rough size split is itself informative:
qualification is the most intricate part of the producer, and finalization is
the smallest because most of its work is delegated to `eggpack-manifest` and
`eggpack-contract`.

## Stage-to-stage data flow

The types below are the seams between stages. Each is defined in exactly one
file and consumed by the next; no stage redefines another's vocabulary.

```text
DistributionContract          (eggpack-contract: layout authority)
        |
        |  PackConfig + contract
        v
ReleasePlan                   (lib.rs: intent, canonically sorted)
        |
        |  BuildBindingsV1           (builder.rs: logical output -> package/bin)
        |  execute_target[_cancellable]
        v
BuildAttempt                  (builder.rs: candidate bytes, identity-bound to
        |                        release + source revision)
        |  QualificationBindingsV1  (qualification.rs: smoke binding per target)
        |  qualify_target
        v
QualificationEvidence         (qualification.rs: host-matched proof, or failure)
        |
        |  finalize_release(FinalizationRequest)
        v
FinalizedRelease              (finalization.rs: bytes under contract names)
        |
        |  build_manifest
        v
ReleaseManifest               (eggpack-manifest: evidence over FINAL bytes)
```

Two separations are load-bearing and worth stating explicitly, because they are
the reason the stages cannot be collapsed:

1. **`BuildAttempt` is not evidence.** It is candidate bytes plus the identity
   of what produced them. Nothing may be published from a `BuildAttempt`; the
   qualification gate in finalization is what turns bytes into a releasable
   release.
2. **The manifest is computed at finalization, not at build time.** Digests in
   `ReleaseManifest` describe the final on-disk bytes after renaming, archiving,
   and sidecar generation — not the candidate bytes the builder produced.

## Cross-file contracts

- **Identity binding.** Release identity and source revision are carried from
  the plan through the build attempt into qualification evidence, so evidence
  cannot be silently attributed to a different release or a different commit.
- **Exact coverage.** Build bindings and qualification bindings are validated
  against the plan for *exact* coverage: a missing target and an extra target
  are both errors. Partial coverage would let a release ship with an unplanned
  artifact.
- **Host matching.** `Qualification::Native` is independent of the builder.
  A `cargo zigbuild` candidate may be legitimately qualified natively when the
  qualification host matches the target OS/architecture; a mismatch is a
  `QualificationFailure::HostMismatch` recorded as a failure, never a pass and
  never a skip. This is the rule that unblocked cross-tool builds. The two
  stages enforce it differently and both must hold: the planning stage rejects
  an inadmissible combination up front as a fail-closed `CoreError`
  (`src/lib.rs:157-160`), and the qualification stage records the observed
  `HostMismatch` at run time (`src/qualification.rs:216`). The planning gate
  runs only inside `PackConfig::resolve`, not on plain parsing.
- **Fail-closed everywhere.** A missing input, an unknown field, an out-of-bounds
  count, or an unmatched identity is an error. There is no tolerant or
  best-effort path in this crate.

## Boundaries

- No network access. `core` never talks to a registry or a release host; that
  is `eggpack-github`'s job.
- No publication and no tag mutation. `core` produces files; staging a draft and
  any publication remain separate, explicit, human-gated steps.
- No shell interpretation. Commands are constructed as argument vectors and
  executed directly; generated inputs are never passed through a shell.
- No authenticity or provenance claims. Digests and sizes produced here are
  integrity facts. See [determinism.md](determinism.md) and
  [manifest.md](manifest.md).
- No generic command DSL. The build adapter surface is restricted to the
  first-party Cargo and `cargo zigbuild` paths (ADR-0004), not an arbitrary
  user-defined command escape hatch.
- No installed-state modeling. What a user actually has on disk is Eggup's
  receipt, not anything core records.

## Dependencies / dependents

Dependencies: `eggpack-contract`, `eggpack-manifest`, `serde`, `serde_json`,
`sha2`, `toml`, `command-group` 5.0.1, `tar`, `flate2`.

Dependents: `eggpack-ci` (projects the pipeline into a CI graph and encodes
evidence handoffs), `eggpack-github` (consumes `FinalizedRelease` and the
manifest to build a staging payload), `eggpack-cli` (invokes the stages as
file-in/file-out runner commands).

## Related deep dives

- [overview.md](overview.md) — workspace-level view
- [contract.md](contract.md) — the layout authority core resolves against
- [manifest.md](manifest.md) — the schema-v1 document finalization produces
- [core-planning.md](core-planning.md), [core-build.md](core-build.md),
  [core-qualification.md](core-qualification.md),
  [core-finalization.md](core-finalization.md) — per-stage detail
- [determinism.md](determinism.md) — sorting and byte-identical output rules
- [validation-model.md](validation-model.md) — the fail-closed model
- [process-execution.md](process-execution.md) — bounded process execution and
  environment handling
- [testing-and-portability.md](testing-and-portability.md) — how this crate is
  tested, including the Windows-only lanes
- [principles-roadmap.md](principles-roadmap.md) — ADRs, in particular ADR-0004
  (build adapter) and ADR-0005 (native qualification for cross-tool builds)
