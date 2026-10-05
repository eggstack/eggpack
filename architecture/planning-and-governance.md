# Planning and Governance — Deep Dive

How this repository is planned, how a milestone is allowed to be called closed,
and where the authority for "what is the current state" lives. This is a
cross-cutting concern about the *process*, not about any one crate.

Code baseline `61b2c03`. Planning state is a moving target: `plans/registry.md`
is the authority for current status, and this document deliberately does **not**
restate milestone statuses, because they change. Read the registry for those.

This file cites no `file:line` references into `crates/`, so it does not share the
re-verification burden described in `overview.md`. The one count table below is
the exception: re-measure it with `find plans -name '*.md' | wc -l` before editing
this document again.

## What `plans/` is for

`plans/README.md` states the split directly: the directory separates **durable
architectural direction** from **temporary execution planning**. Two kinds of
document live here, and the distinction is enforced by rule, not convention:

- **Long-term documents** state *what Eggpack is becoming and what must remain
  true*. They are normative and stable.
- **Interim documents** state *what an implementation agent should do next
  against a specific repository baseline*. They are disposable once superseded.

The four canonical long-term documents are
`plans/000-long-term-specification.md` (normative end-state specification and
invariants), `plans/001-terminology-and-domain-model.md` (normative language and
identity model), `plans/002-long-term-roadmap.md` (dependency-ordered capability
roadmap), and `plans/003-planning-process.md` (rules for deriving interim plans).
`plans/README.md` is explicit that the first three **MUST NOT be edited as part
of ordinary implementation work** — changing them requires an explicit long-term
architecture decision. Interim plans must *reference* them rather than silently
revise their requirements. That is the mechanism that stops scope creep from
quietly redefining the product.

## Hierarchy and directory roles

```text
Long-term specification and terminology   000, 001
        |
        v
Architecture decision records            adrs/
        |
        v
Master long-term roadmap                 002
        |
        v
Subsystem roadmaps                       subsystems/
        |
        v
Milestone implementation plans           implementation/<subsystem>/NNN-*.md
        |
        v
Implementation and verification           (code + hosted CI)
        |
        v
Closure records                          closure/<subsystem>/NNN-status.md
        |
        v
Archive                                  archive/
```

| Directory | Holds | Count at baseline |
|---|---|---|
| `plans/` | the four canonical documents, `README.md`, `registry.md` | 6 |
| `plans/adrs/` | durable architecture decisions (ADR-0001…0005) | 5 ADRs |
| `plans/subsystems/` | per-subsystem specification and roadmap | 9 roadmaps |
| `plans/implementation/` | bounded milestone plans for implementation agents | 36 documents |
| `plans/closure/` | verification and evidence records | 35 documents |
| `plans/archive/` | superseded planning retained for traceability | README only at baseline |

`registry.md` sits outside the hierarchy deliberately: it is the **compact
control surface**, not a document of record. Its own header says detailed
requirements live in the canonical documents, roadmaps, plans, closure records,
and Git history. Use it to find things, not to settle them.

## Required classification

Every subsystem roadmap and implementation plan must distinguish four
categories, and this is the single most useful rule in the system for a reviewer
(`plans/README.md`):

| Category | Meaning |
|---|---|
| **Invariant** | a property that must always remain true |
| **Capability** | developer- or release-operator-visible behavior |
| **Infrastructure** | internal machinery required by capabilities |
| **Polish** | ergonomics, diagnostics, performance, cleanup, documentation |

The load-bearing sentence: **infrastructure and polish must not be presented as
completed capability without the corresponding contract-level acceptance
evidence.** This is the rule that distinguishes a real capability from a
mechanism that merely exists, and it is the reason "the code is there" is never
sufficient in this repository. See `core-qualification.md` for a worked example —
the qualification machinery is infrastructure; host-matched native
qualification for cross-tool builds is the capability, and the two were closed
separately.

## Milestone lifecycle

`plans/README.md` defines nine steps:

1. Identify the relevant long-term specification sections and invariants.
2. Record unresolved architectural decisions in `adrs/`.
3. Create or update a subsystem roadmap in `subsystems/`.
4. Select one dependency-ready milestone.
5. Write a bounded handoff plan under `implementation/`.
6. Implement and verify.
7. Write a closure record under `closure/`.
8. Update `registry.md` and the subsystem roadmap.
9. Archive completed or superseded interim planning.

Note step 4: **one** dependency-ready milestone at a time. That is what keeps a
handoff bounded, and it is why the registry's "dependency-ready implementation
work" section is normally a short list.

## Closure discipline

This is where most of the process's real weight sits.

**Compilation is not closure.** `plans/README.md`: "No milestone is complete
merely because code landed. Completion requires the closure evidence defined by
its implementation plan and roadmap."

`plans/closure/README.md` defines the evidence gate. A closure record at
`closure/<subsystem>/NNN-status.md` (same milestone number as its plan) must
include: status; source plan and roadmap; **reviewed baseline**; **implementation
commits**; executive finding; a requirement-to-evidence matrix; production
implementation evidence; exact verification executed and results; invariant
review; failure/recovery review; compatibility/migration review; security review;
documentation/operations evidence; unresolved findings with severity; roadmap
disposition; and registry updates.

Two rules from that file deserve emphasis for anyone reviewing the process:

- **Unknown or unavailable evidence must be recorded honestly.** A closure record
  is allowed to say "this was not verified"; it is not allowed to imply coverage
  that does not exist.
- **Prior closure records are not rewritten to conceal later defects.** History
  is append-only.

That second rule has a direct architectural consequence, recorded in this
repository's `AGENTS.md`: when a later milestone discovers a defect in an
already-closed one, the correction is a **new numbered plan** (`NNNa`), never an
edit to the original plan or its closure record. This is why the milestone
numbering has corrective suffixes — `002a`, `003a` through `003h` — and why
several closure records are marked "closed historically; corrective closed".
Superseded material moves to `plans/archive/` under its original relative
structure; it is not deleted.

A good closure record is also a *diff* record. `plans/closure/build-qualification/006-status.md`
lists exactly three production edits in a table, names what was deliberately left
unchanged, and states explicitly that nothing was weakened to achieve the
milestone — which is the kind of negative evidence a reviewer actually needs.

## Evidence discipline

Closure records are expected to be pinned to immutable identifiers rather than
prose descriptions:

- **Reviewed baseline** — a full 40-character commit SHA of the code state the
  milestone was planned against, with a note on what had and had not advanced
  since.
- **Implementation commits** — full SHAs with the conventional-commit subject,
  distinguishing production changes from test-only or plans-only commits.
- **Hosted CI run IDs** — linked, with the lane matrix spelled out
  (`linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`,
  `portability (windows-latest)`) and the attempt number.

This is a real strength of the repository: a claim like "M006 is closed" is
traceable to `398cd43` and hosted run `36484758546` in one step. A reviewer does
not have to take the prose on trust.

The corollary is that evidence is per-milestone and can go stale in either
direction. A closure record's run ID proves *that* milestone's lanes were green;
it does not prove the current tip is green. `testing-and-portability.md` covers
how to check the current state yourself.

## Status vocabulary

`plans/registry.md` defines a closed vocabulary, and the distinctions are
deliberately fine-grained:

| Status | Meaning |
|---|---|
| proposed | roadmap/plan exists, not approved for execution |
| ready | hard dependencies and interfaces are satisfied |
| active | implementation or closure work in progress |
| blocked | a named dependency or piece of evidence prevents progress |
| closing | implementation landed, closure evidence being gathered |
| closed | closure record accepted |
| conditionally closed | substantially complete, with a **named** evidence condition outstanding |
| superseded | replaced by another document |
| archived | retained only for traceability |

"Conditionally closed" is the one to watch. It means a specific, named condition
is outstanding — not that the work is vaguely incomplete. A reviewer should
always be able to name that condition; if they cannot, the status is being
misused.

## The registry as control surface

`plans/registry.md` is the file to open first when asking "what is the current
state". It is organized as: canonical direction; status vocabulary; accepted
architecture decisions (a table of the five ADRs and one-line summaries); current
evidence baselines (including cross-repository baselines for Eggup, Eggsact,
and StegoEggo, and the external-backend evaluation); active subsystem roadmaps
(a table of subsystem, status, roadmap, current milestone, and
dependencies/blockers); dependency-ready implementation work; and
dependency-blocked work.

The "current evidence baselines" section is the part that ages worst. It records
commit SHAs, hosted run IDs, and cross-repository positions that were true at
writing time, and the registry's own next-handoff milestone exists specifically
to reconcile that section against downstream reality. Treat any specific SHA or
run ID you read there as a claim with a date, not as a live fact.

## Cross-repository scope

Eggpack's planning spans three repositories, and the boundary is load-bearing
(ADR-0001, see `principles-roadmap.md`):

- **Eggpack** — producer-side distribution authority, and the source of truth for
  producer semantics.
- **Eggup** — consumer-side deployment: acquisition, verification, local
  replacement, rollback, recovery, service lifecycle.
- **Product repositories** (Eggsact, StegoEggo) — release/install policy, and the
  first real consumers of a published manifest.

`plans/README.md` states the governing rule: a convenience that causes Eggpack to
perform live machine deployment, or causes Eggup to become a
release-production/build system, is an **architecture change requiring an ADR** —
not a detail to be settled in a milestone plan. When reviewing a plan that seems
to reach across that line, that sentence is the test.

Milestones may also be *cross-repo*, in which case the registry names the
producer view and the consumer view separately and records both closure states.
This is why the registry has a dedicated "Cross-repo M003 mapping" paragraph: a
single logical capability was closed in two repositories, and the record has to
say so explicitly or the seam looks unfinished.

## How this relates to the rest of the documentation

| Question | Authority |
|---|---|
| What does the code do, and how is it structured? | `architecture/overview.md` and the per-component deep dives |
| What is true right now, and what is blocked? | `plans/registry.md` |
| Why is it built this way? | `plans/adrs/` and `architecture/principles-roadmap.md` |
| What did a specific milestone actually do, and how was it verified? | `plans/closure/<subsystem>/NNN-status.md` |
| What must remain true? | `plans/000-…` and `plans/001-…` |
| How is new work shaped? | `plans/003-planning-process.md` and this document |
| What are the commands, and what must I not weaken? | `AGENTS.md` |
| What is the task-shaped starting point for this kind of work? | `.skills/<name>/SKILL.md` — **derived**, never authoritative |

`AGENTS.md` and `.skills/` are the agent-facing layer over the same process. They
exist so an agent does not have to reconstruct the hierarchy to know what to do
next, and they are the two files most likely to rot: the registry's status block
and this document's own status restatements are the recurring failure. When a
skill and this document disagree, this document is right and the skill is what
gets fixed. A skill that needs a new rule does not get one — the rule goes into
`AGENTS.md` or the owning deep dive, and the skill points at it.

`plans/` and `architecture/` have different jobs and should not be merged.
`architecture/` describes the system as it is. `plans/` describes how the system
is intended to change, and what evidence was demanded before calling a change
done. A planning document that describes current behavior without a code citation
belongs in the second category; a deep dive that argues for a future direction
belongs in neither.

## Related deep dives

- [overview.md](overview.md) — workspace-level view
- [principles-roadmap.md](principles-roadmap.md) — the domain model, ADR
  summaries, and roadmap phases that this process maintains
- [testing-and-portability.md](testing-and-portability.md) — how to produce the
  verification evidence a closure record demands
- [determinism.md](determinism.md) and [validation-model.md](validation-model.md) —
  two of the workspace invariants that closure records are required to review
- [eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md) — the consumer
  side of the cross-repository boundary
- `AGENTS.md` — the agent-facing summary of these conventions, including the
  closure discipline and the current next handoff
