---
name: planning-and-closure
description: Open, plan, correct, and close an Eggpack milestone. Use when writing a plan under plans/implementation/, writing or reviewing a closure record under plans/closure/, updating plans/registry.md, or correcting something a closed milestone got wrong. Encodes the corrective NNNa pattern, the evidence a closure must carry, and the status vocabulary.
---

# Planning and closure

Eggpack has the strictest planning discipline in the eggstack repositories. The
reason is that "the code is there" is not accepted as completion — capability
requires contract-level acceptance evidence. `plans/003-planning-process.md` is
**normative**; this skill only routes you into it.

## Before you open anything

1. Read `plans/registry.md` §"Next handoff". As of the last review **no
   implementation milestone is `ready`** — the next three lines (Contract M003,
   Bootstrap M003, Ecosystem M003 eggsearch) are *research/plan decisions*, not
   execution. If your idea is one of them, the deliverable is a plan, not code.
2. Check the subsystem roadmap (`plans/subsystems/<subsystem>-roadmap.md`). A
   plan may already exist under a name you did not guess.
3. Only then write anything.

## The rule that catches people

The first three canonical documents — `000-long-term-specification.md`,
`001-terminology-and-domain-model.md`, `002-long-term-roadmap.md` — **MUST NOT be
edited as ordinary implementation work.** A material change requires explicit
rationale, an ADR if ownership or a public contract moves, roadmap
reconciliation, and a registry update (`003` §14). Interim plans reference them;
they never silently revise them. That is the mechanism that stops scope creep from
quietly redefining the product.

## Opening a milestone

`plans/README.md` defines a nine-step lifecycle; `003` §3 classifies dependencies
as hard / interface / soft / operational, and **only hard- or interface-ready
milestones may be marked `ready`**. One milestone at a time — that is what keeps
a handoff bounded.

Every roadmap and plan must classify its content as **invariant**, **capability**,
**infrastructure**, or **polish**. The load-bearing sentence: *infrastructure and
polish must not be presented as completed capability without the corresponding
contract-level acceptance evidence.* The worked example is Build/Qualification
M003 (the machinery) versus M006 (host-matched native qualification, the
capability) — closed separately, on purpose.

An ADR is required when a decision changes the Eggpack/Eggup boundary, establishes
a public schema/manifest compatibility contract, selects a durable build/release
backend, changes publication authority, selects an authenticity trust model,
introduces a network publication protocol, materially changes release
finalization semantics, or creates a general workflow/configuration language
(`003` §4). Routine choices that preserve those contracts do not need one.

## Writing the plan

Path `implementation/<subsystem>/NNN-short-title.md`. `003` §7 requires it to
state: objective; why ready; current evidence; invariants; in/out scope; required
production changes; ordered work packages; failure/restart/contention semantics;
compatibility/migration; tests; **exact verification commands**; docs; acceptance
criteria; stop conditions; closure evidence.

Specify behavior and authority, not brittle line-by-line edits. Name a concrete
40-character repository baseline; for cross-repo work, name the external baseline
too.

## Closing it

Path `closure/<subsystem>/NNN-status.md`, same milestone number as the plan.
`plans/closure/README.md` lists the required sections — status; source plan and
roadmap; reviewed baseline; implementation commits; executive finding;
requirement-to-evidence matrix; production implementation evidence; exact
verification executed and results; invariant review; failure/recovery review;
compatibility/migration review; security review; documentation/operations
evidence; unresolved findings with severity; roadmap disposition; registry
updates.

Two rules do the real work:

- **Unknown or unavailable evidence must be recorded honestly.** "This was not
  verified" is allowed. Implying coverage that does not exist is not.
- **Prior closure records are not rewritten.** History is append-only.

A good closure record is also a *diff* record: what changed, what was
deliberately left unchanged, and an explicit statement that nothing was weakened
to reach the milestone. `closure/build-qualification/006-status.md` is the model.

Pin evidence to immutable identifiers — full SHAs, hosted run IDs, and the lane
matrix spelled out. A closure's run ID proves *that* milestone's lanes were
green; it does not prove the current tip is green. The `pre-submit-gate` skill
is how you check the current tip.

## Correcting a closed milestone

Do not edit the original plan or its closure record. Instead:

1. preserve the prior closure as historical evidence;
2. write a new corrective plan `NNNa`;
3. enumerate every affected invariant;
4. add regression evidence that would have caught the defect;
5. update registry and roadmap status;
6. re-close before unblocking dependent work.

That is why the milestone numbers carry corrective suffixes — `002a`, `003a`
through `003h` — and why several registry rows read "closed historically;
corrective closed". Those `NNNa` files are not clutter; they are the audit trail
proving a defect was found, bounded, fixed, and independently evidenced.

## The registry

`plans/registry.md` is a **compact control surface, not a document of record**.
Use it to find things, not to settle them. Its §"Current evidence baselines" ages
worst: treat every SHA and run ID there as a claim with a date.

The status vocabulary is closed and deliberately fine-grained — proposed, ready,
active, blocked, closing, closed, conditionally closed, superseded, archived.
**"Conditionally closed" means a specific named condition is outstanding.** If a
reviewer cannot name that condition, the status is being misused.

A derived rule from the last reconciliation pass: do not pin "current main@<sha>"
in the registry. It rots silently. Point at a closure record instead — that is
what Planning Hygiene M001 did when it replaced a moving `current main` snapshot.

## Cross-repository scope

Eggpack's planning spans three repositories: Eggpack (producer authority),
Eggup (consumer deployment), and product repositories such as Eggsact and
StegoEggo (release/install policy, first real manifest consumers).

- The Eggpack repository **MAY** define the expected cross-repo interface, but
  changes to another repository require a plan registered in *that* repository.
- A single Eggpack closure **MUST NOT** claim another repository was migrated
  unless its actual commit and evidence were reviewed.
- When a milestone is cross-repo, name the producer view and the consumer view
  separately and record both closure states. That is why the registry carries a
  dedicated "Cross-repo M003 mapping" paragraph.
- A convenience that would make Eggpack perform live machine deployment, or make
  Eggup a release-production/build system, is an architecture change requiring an
  ADR. That sentence is the test.

## Reference

- [plans/003-planning-process.md](../../plans/003-planning-process.md) — normative
- [plans/README.md](../../plans/README.md) — hierarchy, naming, lifecycle
- [plans/closure/README.md](../../plans/closure/README.md) — the evidence gate
- [plans/registry.md](../../plans/registry.md) — the control surface
- [architecture/planning-and-governance.md](../../architecture/planning-and-governance.md)
  — how and why the process works
