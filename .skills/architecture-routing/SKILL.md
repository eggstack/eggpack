---
name: architecture-routing
description: Index into Eggpack's architecture/ deep dives - which file answers which question, which crate owns which boundary, and how to tell a real defect from documentation drift. Use before reading source, when you need to know where a behavior is implemented or which doc is normative, or before deciding a suspected bug is a code fix or a doc fix.
---

# Architecture routing

`architecture/overview.md` is the bird's-eye view *and* the index. The other
twenty documents are per-component deep dives, each written to be read *instead
of* the source when you need to understand a boundary. Read the index, then
follow exactly one link. Do not grep the whole tree.

## Which doc answers which question

| Question | Read |
|---|---|
| What is the product, and what is it deliberately not? | [README](../../README.md), [overview.md](../../architecture/overview.md) |
| Where is a release asset/bundle/archive name defined? | [contract.md](../../architecture/contract.md) — the only authority |
| What does the manifest document, and what may it never contain? | [manifest.md](../../architecture/manifest.md) |
| How do the four `core` files pass data to each other? | [core.md](../../architecture/core.md) |
| How does policy become a sorted release plan? | [core-planning.md](../../architecture/core-planning.md) |
| How are Cargo/zigbuild commands built, bounded, and run? | [core-build.md](../../architecture/core-build.md) |
| How is a target qualified, and what does native qualification *not* prove? | [core-qualification.md](../../architecture/core-qualification.md) |
| Where do archives, sidecars, and the manifest get written? | [core-finalization.md](../../architecture/core-finalization.md) |
| What does the first-install script actually do? | [bootstrap.md](../../architecture/bootstrap.md) |
| How is a release projected into a job graph? | [ci.md](../../architecture/ci.md) |
| How is YAML rendered deterministically, and how is drift detected? | [ci-rendering.md](../../architecture/ci-rendering.md) |
| What are the handoff file names, the consumer validator, and runtime identity? | [ci-consumer-seam.md](../../architecture/ci-consumer-seam.md) |
| How does anything reach a human without being published? | [github.md](../../architecture/github.md) |
| What are the CLI's commands, exit discipline, and path-safety rules? | [cli.md](../../architecture/cli.md) |
| Is this output byte-stable, and where is stability *not* claimed? | [determinism.md](../../architecture/determinism.md) |
| Why does validation fail closed, and what are the bounds? | [validation-model.md](../../architecture/validation-model.md) |
| How are child processes spawned, bounded, and cancelled? | [process-execution.md](../../architecture/process-execution.md) |
| What do the tests actually cover, and on which platforms? | [testing-and-portability.md](../../architecture/testing-and-portability.md) |
| How is work planned, and what does "closed" require? | [planning-and-governance.md](../../architecture/planning-and-governance.md) |
| What is the domain model, the ADRs, and the `dist` disposition? | [principles-roadmap.md](../../architecture/principles-roadmap.md) |
| How does a manifest map onto Eggup install receipts? | [eggup-manifest-consumer-v1.md](../../architecture/eggup-manifest-consumer-v1.md) |

`overview.md` §Review paths is the same routing, organized by question instead
of by file.

## Code that owns each boundary

| Boundary | Owner | Notes |
|---|---|---|
| Layout and names: assets, sidecars, bundles, archive members, install names | `eggpack-contract` | single authority; nothing downstream may redefine a name |
| Final-bytes evidence document | `eggpack-manifest` | leaf, no I/O; the only published crate |
| Policy → plan → build → qualify → finalize | `eggpack-core` | only crate that writes release artifacts and spawns toolchain processes |
| First-install script rendering | `eggpack-bootstrap` | no release selection, no updates |
| Job-graph projection, workflow render, drift check | `eggpack-ci` | plans and renders; executes nothing, has no network |
| Staging payload and GitHub draft | `eggpack-github` | the only crate with network/credentials |
| Deterministic wiring, file-safety helpers, exit codes | `eggpack-cli` | hand-rolled args, no `clap` |

Dependencies are one-way: `contract`/`manifest` → `core` → `bootstrap`/
`github`/`ci` → `cli`. `eggpack-ci` deliberately does **not** share core's
process-execution contract; the consumer validator has its own runner. That gap
is documented, not accidental.

## The four-object split — the thing most often collapsed

```text
DistributionContract  expected layout, authority      (contract)
ReleasePlan           intent, not evidence            (core-planning)
ReleaseManifest       final bytes, evidence only      (manifest)
InstallReceipt        installed state                 (Eggup, not here)
```

If a change makes one of these carry another's job, that is an architecture
change, not an implementation detail — it needs an ADR. `principles-roadmap.md` §1
is the normative statement.

## Citations are re-verified, so trust them but re-check after you edit

`overview.md` states that every `file:line` citation in `architecture/` was
verified against the working tree (2502 at the time of writing), and that
citations into a touched file were confirmed to still point at the same code,
not merely the same line number.

That is a real, maintained property — but it is invalidated the moment you change
a cited file. If you touch a `.rs` file, either re-verify the citations that
point into it or state in your commit message that they need a pass. A citation
that is merely *in range* is the failure mode this repository is explicitly
trying to avoid.

## Before you call something a bug

1. **Code contradicts a deep dive.** The deep dives are written to be accurate.
   Check the deep dive first. If it agrees with the code, the page-level prose
   (`overview.md`, `AGENTS.md`, `README.md`, a skill) is what drifted — fix the
   prose.
2. **A real defect.** Write it up with a `file:line` citation and a concrete
   failure, then check `plans/registry.md` *before* opening work: a plan may
   already name it, and Eggpack's closure discipline means a fix for an
   already-closed milestone is a new `NNNa` plan, never an edit to the old one.

Prefer narrowing a drifting invariant statement over deleting a real guard. An
absolute-sounding invariant that the code legitimately violates is worse than no
invariant: a future agent auditing against it reaches a confidently wrong
conclusion. Note that `overview.md` has no divergences table — unlike some
sibling repositories — so there is no shortcut here; the deep dives themselves
carry the caveats, and they say where determinism and qualification claims stop.

## Reference

- [overview.md](../../architecture/overview.md) — pipeline, crate cards, invariants
- [AGENTS.md](../../AGENTS.md) — the workspace-wide invariant list
- [plans/registry.md](../../plans/registry.md) — check before opening work
