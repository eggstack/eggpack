# Eggpack Planning Process

Status: normative planning process

This document governs how Eggpack's canonical direction becomes implementation work.

## 1. Planning layers

Eggpack uses:

1. canonical specification and terminology;
2. ADRs;
3. master long-term roadmap;
4. subsystem roadmaps;
5. milestone implementation plans;
6. implementation;
7. closure records;
8. archive.

A lower layer may refine implementation detail but MUST NOT silently contradict a higher layer.

Every release/distribution plan MUST first classify ownership. Producer release contracts, conformance, build/qualification, packaging, final release evidence, bootstrap generation, generated release CI, staging/publication, and producer provenance belong to Eggpack. Consumer-machine acquisition, verification, local install/update/rollback, service lifecycle, and install receipts belong to Eggup. Application release/version/origin/fallback policy remains product-owned.

## 2. Baseline rule

Every implementation plan MUST name a concrete Eggpack repository baseline.

Cross-repository migration/adoption plans MUST also name the external source baseline being relied upon, for example an Eggup SHA and an `eggup-dist` closure/implementation SHA.

If the external baseline changes materially before implementation, the plan MUST be re-reviewed rather than applied mechanically.

## 3. Dependency classes

Roadmaps and plans classify dependencies as:

- **hard** — work cannot safely begin until closed;
- **interface** — work may proceed against a stable documented contract;
- **soft** — improves quality but does not block correctness;
- **operational** — environment/runner/credential evidence needed for final closure.

Only hard/interface-ready milestones may be marked `ready`.

## 4. Architecture decision threshold

An ADR is required for decisions that:

- change the Eggpack/Eggup ownership boundary;
- establish a public schema/manifest compatibility contract;
- select a durable build/release backend;
- change publication authority;
- select an authenticity/signing trust model;
- introduce a network publication protocol;
- materially change release finalization semantics;
- create a general workflow/configuration language.

Routine implementation choices that preserve these contracts do not require ADRs.

## 5. External-tool evaluation rule

External tooling MUST be evaluated with representative Eggstack fixtures before adoption.

A spike MUST record:

- exact tool/version;
- supported/unsupported target forms;
- manifest/installer/CI behavior;
- dependency/runtime impact;
- security/update ownership implications;
- integration cost;
- exit/disposition.

A spike does not authorize production dependency adoption. An ADR or follow-on implementation plan does.

## 6. Migration rule for eggup-dist

The unpublished `eggup-dist` crate is frozen qualified predecessor evidence. Eggup distribution M003 is the terminal producer-side predecessor and is already closed.

Migration proceeds as:

```text
Eggup M003 closed implementation/closure
      |
      v
Eggpack Contract M001 schema import [closed]
      |
      v
Eggpack Contract M002 conformance fidelity port
      |
      v
close M002 with predecessor behavior matrix
      |
      +--> unblock Eggpack ReleaseManifest planning
      |
      `--> authorize already-registered Eggup M004 retirement
                  |
                  v
           remove eggup-dist
```

Do not create a period where neither repository contains the qualified contract. Do not remove the Eggup copy merely because files were copied. Contract M002 closure MUST explicitly state whether Eggup M004 retirement is unblocked. After cutover, producer contract evolution occurs only in Eggpack.

## 7. Implementation plan requirements

Each plan MUST state:

- objective;
- why ready;
- current evidence;
- invariants;
- in/out scope;
- required production changes;
- ordered work packages;
- failure/restart/contention semantics as applicable;
- compatibility/migration;
- tests;
- exact verification commands expected;
- docs;
- acceptance criteria;
- stop conditions;
- closure evidence.

Plans SHOULD specify behavior and authority rather than brittle line-by-line edits.

## 8. Closure requirements

Closure records MUST distinguish:

- implementation evidence;
- tests actually run;
- hosted CI actually observed;
- environmental gaps;
- unresolved findings by severity;
- dependency transitions unlocked.

Compilation alone does not close a capability milestone.

Historical closure records are not rewritten to hide defects. Correctives get new plans/closure records.

## 9. Cross-repository work

Eggpack will frequently affect Eggup and consumer repositories.

The Eggpack repository MAY define the expected cross-repo interface, but changes to another repository require a plan registered in that repository when its planning system exists.

A single Eggpack closure MUST NOT claim another repository was migrated unless its actual commit/evidence is reviewed.

## 10. Consumer adoption evidence

A reusable abstraction is not mature until proven by independent consumers.

Adoption plans MUST capture:

- before/after duplicated authority;
- exact contract/config consumed;
- release artifacts produced;
- installer/runtime mapping evidence;
- qualification results;
- remaining product-specific policy;
- rollback path for migration.

## 11. Security-sensitive planning

Plans involving download, hooks, archive extraction, publication, signing, or CI permissions MUST explicitly cover:

- bounds/timeouts;
- path handling;
- credential/redaction rules;
- privilege scope;
- immutable/public release handling;
- third-party tool/action pinning;
- partial failure;
- reproducible evidence.

## 12. Registry discipline

`plans/registry.md` is the compact active control surface.

It SHOULD contain:

- canonical documents;
- accepted ADRs;
- active subsystem roadmaps;
- dependency-ready implementation plans;
- blocked/planned work;
- immediate execution graph;
- current baselines;
- next handoff.

Detailed requirements remain in source plans/roadmaps.

## 13. Corrective work

When post-implementation review finds a medium-or-higher defect:

1. preserve the prior closure as historical evidence;
2. write a new corrective plan;
3. enumerate every affected invariant;
4. add regression evidence that would have detected the defect;
5. update registry/roadmap status;
6. re-close before unblocking dependent work.

## 14. Canonical-document change control

The canonical specification, terminology, and roadmap may evolve, but not as incidental edits during feature implementation.

Material changes require:

- explicit rationale;
- an ADR when ownership/public contracts change;
- reconciliation of affected roadmaps;
- registry update.

## 15. Initial planning baseline

The project starts empty. The first code milestone is therefore repository/bootstrap plus faithful DistributionContract import, not a greenfield redesign of packaging semantics.
