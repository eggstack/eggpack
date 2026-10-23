# ADR-0006: Release Evidence Digest Acceptance Set

Status: **accepted** (Option A)

Date: 2026-10-23 (accepted 2026-10-23)

Decision owners: project maintainers

Related specification sections:

- plans/000-long-term-specification.md#11-artifact-finalization-and-checksums
- plans/000-long-term-specification.md#12-concrete-release-manifest
- plans/000-long-term-specification.md#19-security-model
- plans/000-long-term-specification.md#20-determinism-and-reproducibility

Related decisions:

- ADR-0002 (contract / plan / manifest separation);
- ADR-0005 (native qualification is independent of build strategy).

Affected subsystem roadmaps:

- build and qualification;
- release manifest;
- CI/release orchestration.

## 1. Status note

This record is **accepted** (Option A), decided by the project maintainers on
2026-10-23. It exists because a post-implementation review found that Eggpack
carries **three different SHA-256 acceptance rules** for the same concept, and
because the choice between them is a public compatibility decision rather than
an internal refactor.

Accepting this ADR authorizes
`plans/implementation/ci-release-orchestration/003l-producer-rule-authority-convergence-corrective.md`
to implement the selected rule set. The digest-validator work package in M003l
is therefore **unblocked**. Every other milestone in this review chain stays
independently implementable and is unaffected by this decision.

## 2. Context

`ReleaseManifest v1` records SHA-256 digests of final bytes. Three independent
validators exist in the workspace, and they do not agree:

| Validator | Location | Rule |
|---|---|---|
| `ByteEvidence::validate` | `crates/eggpack-manifest/src/lib.rs:307-322` | exactly 64 lowercase hex characters |
| `validate_sha256` | `crates/eggpack-github/src/lib.rs:567-576` | exactly 64 lowercase hex characters |
| `validate_sha256_hex` | `crates/eggpack-ci/src/lib.rs:632-642` | exactly 64 lowercase hex characters **and not the all-zeros digest** |

The locations above and the behaviour below are as of this ADR's date
(2026-10-23); the CI crate's validator has since moved with the M003n module
split to `crates/eggpack-ci/src/policy.rs:161`, and `ByteEvidence::validate` to
`crates/eggpack-manifest/src/lib.rs:365`, so the table records the divergence
that motivated the decision rather than the current tree.

The divergence is not cosmetic. The all-zeros string `"0" * 64` is a
syntactically valid 64-character lowercase hex digest that no real file can
produce, because SHA-256 of any byte sequence is astronomically unlikely to be
the all-zero value. It is therefore a natural sentinel for "digest not computed",
"digest placeholder", or "digest defaulted". At this ADR's date:

- the CI crate **rejects** it;
- the GitHub staging crate, which is the component that actually computes and
  compares digests over staged bytes, **accepts** it;
- the manifest crate, whose documents are published, **accepts** it.

Two distinct problems follow.

### 2.1 The producer's own evidence rules disagree

A rule that is enforced in one crate and not in another is not a rule. The
crate that stages bytes to a draft release (`eggpack-github`) is the one place
where an all-zeros digest could be persisted into a published asset record, and
it is the one place that does not currently reject the sentinel. This is an
internal consistency defect, and it is fixable without any public contract
change.

### 2.2 The published manifest's acceptance set is a public compatibility contract

`eggpack-manifest 0.1.0` is published on crates.io and is consumed downstream by
`eggup-eggpack`. Tightening what `ReleaseManifest v1` accepts — even for a
value that no legitimate producer emits — changes the acceptance set of a
published schema-v1 document. `plans/003-planning-process.md` §4 names exactly
this as an ADR trigger: *"establish a public schema/manifest compatibility
contract"*.

Therefore the choice cannot be made silently inside a refactor. It must be made
explicitly, with a compatibility handling decision, and recorded.

## 3. Decision drivers

- **Fail-closed validation.** The workspace's stated validation model
  (`architecture/validation-model.md`, `plans/000-long-term-specification.md`
  §19) requires that invalid evidence be refused rather than normalized or
  tolerated. An all-zeros digest is a value that can only mean "no digest".
- **No in-tree producer emits it.** Every in-tree manifest is produced by
  Eggpack's own finalization path, which computes digests from real bytes. The
  sentinel has no legitimate producer.
- **Downstream compatibility must not break silently.** `eggup-eggpack` parses
  published manifests. If the acceptance set tightens, that consumer must be
  told, and the compatibility window must be explicit.
- **Schema version discipline.** `ReleaseManifest` is `schema_version: 1`.
  Tightening a v1 acceptance rule without a version marker would make two
  documents with the same `schema_version` mean different things.

## 4. Considered options

### Option A — reject the all-zeros digest everywhere, keep `ReleaseManifest v1` at `schema_version: 1` (recommended)

Every SHA-256 validator in the workspace — producer-internal and published —
refuses the all-zeros sentinel. The published acceptance set tightens.

Strengths:

- one rule, one meaning, across the whole workspace;
- the published manifest stops accepting a value no legitimate producer emits;
- no new schema version, no new field, no wire change; the document shape is
  identical and the previously-accepted subset remains valid;
- the "digest was never computed" sentinel can no longer reach a published
  asset record.

Costs and risks:

- it **is** a narrowing of a published crate's acceptance set, so it is a
  compatibility event that must be announced, not a pure refactor;
- a downstream consumer that currently persists an all-zeros digest — for
  example as a placeholder during a two-phase upload — would now fail closed.
  This is the desired behavior, but it must be surfaced rather than assumed
  away.

### Option B — reject the all-zeros digest in producer-internal paths only; leave the published manifest permissive

`eggpack-core`, `eggpack-ci`, and `eggpack-github` refuse the sentinel. The
published `ReleaseManifest v1` acceptance set is unchanged, and the divergence
is recorded as a known, documented manifest-side gap.

Strengths: zero downstream compatibility impact; the defect that can actually
persist bad evidence (the staging crate) is closed.

Costs: the published schema continues to accept a value that no producer can
emit; the workspace keeps two acceptance sets; the gap survives and must be
re-discovered later. This trades a documented, cheap-to-explain asymmetry for
the disappearance of an asymmetry.

### Option C — keep all three validators as they are and document the divergence

Strengths: zero code change, zero compatibility impact.

Costs: the rule that governs producer evidence remains unenforced in the crate
that publishes it. This is not acceptable under the fail-closed validation model
and is not recommended.

### Option D — reject the all-zeros digest everywhere and bump `ReleaseManifest` to `schema_version: 2`

Strengths: the acceptance change is machine-visible rather than implicit.

Costs: `eggup-eggpack` consumes `schema_version: 1`. A version bump is a
hard breaking change for that consumer and requires coordinated work in the
Eggup repository, which this ADR cannot authorize. The narrowing here does not
justify that cost: every manifest a real producer emits remains valid, so the
version-1 documents in the field continue to parse under a tightened rule.

## 5. Decision

**Accepted: Option A — reject the all-zeros digest everywhere, keeping
`ReleaseManifest` at `schema_version: 1`.**

Option B is rejected: it would leave two behaviours for one rule, which is the
class of duplication this review chain exists to remove. Option C is rejected.
Option D is rejected as disproportionate.

Concretely, on acceptance:

1. One shared SHA-256 hex validator is defined in `eggpack-manifest`, which owns
   the evidence schema, and is consumed by `eggpack-core`, `eggpack-ci`, and
   `eggpack-github`. It accepts exactly 64 lowercase hex characters and rejects
   the all-zeros digest.
2. `eggpack-manifest` publishes a new patch release (`0.1.1`) carrying that
   rule. The `ReleaseManifest` document shape, field set, and `schema_version`
   are unchanged; only the acceptance set tightens.
3. `eggpack-github` and `eggpack-ci` stop carrying private copies of the rule
   and delegate to the shared validator.
4. The narrowing is recorded as a compatibility event in the release-manifest
   closure record and in `architecture/manifest.md`, naming `eggup-eggpack` as
   the affected downstream consumer.
5. `eggup-eggpack` is informed and given the opportunity to pin the new
   `eggpack-manifest` patch release. Any change inside the Eggup repository is
   authorized by a plan registered **there**, not by this ADR.

If the maintainers prefer zero downstream compatibility impact, Option B is the
acceptable fallback and is recorded as a known residual gap with a named
follow-on. Option C is rejected. Option D is rejected as disproportionate.

## 6. Consequences

- The workspace gains exactly one SHA-256 hex acceptance rule, owned by the
  crate that owns the evidence schema, and consumed by every crate that
  validates or records a digest.
- `eggpack-github` can no longer persist an all-zeros digest into a staged
  asset record.
- The published `ReleaseManifest v1` acceptance set narrows. No document shape,
  field, or `schema_version` changes, so no consumer that only reads real
  producer output is affected.
- `eggpack-manifest` gains a patch release obligation. It is the only published
  crate in the workspace, so this ADR creates the first crate-release event
  driven by a producer-internal correctness finding.
- This ADR resolves **only** the digest acceptance set. It does not authorize
  any change to the digest *computation* path, the file-digest implementations,
  or the release manifest's field set.

## 7. Compatibility and migration

- No `ReleaseManifest` document produced by any Eggpack version is affected:
  the tightened rule rejects a value no producer emits.
- `eggup-eggpack` is the only known downstream consumer of
  `eggpack-manifest 0.1.0`. It must be notified and given the new patch
  version; its own repository's planning system authorizes any change there.
- `eggpack-manifest 0.1.0` remains published and resolvable. The change ships
  as `0.1.1`. `Cargo.lock` in this workspace is updated by the implementing
  milestone, not by this ADR.
- Existing generated workflows are unaffected: no workflow bytes change, so
  `eggpack ci check` drift status is unchanged.

## 8. Security and reliability implications

- The change strictly narrows what is accepted. It cannot cause a previously
  valid real-world document to be rejected, because the rejected value is not a
  digest of any byte sequence.
- The residual risk is a consumer that used the all-zeros value as a
  placeholder. For that consumer the change turns a silent placeholder into a
  fail-closed error, which is the intended direction. The affected consumer is
  named in §7 so the transition is announced rather than discovered.
- No new network, credential, archive, publication, or privilege surface is
  introduced. No signature, attestation, or provenance claim is made or implied.

## 9. Verification required for implementation closure

- a shared validator rejects the all-zeros digest, uppercase hex, wrong length,
  non-hex characters, and an empty string;
- the shared validator accepts a real SHA-256 digest of known bytes;
- `eggpack-manifest` rejects an all-zeros digest in `ByteEvidence::validate`;
- `eggpack-github` rejects an all-zeros digest on a staged asset path;
- `eggpack-ci` continues to reject an all-zeros digest;
- every previously valid in-tree manifest fixture still validates;
- `eggpack-manifest` packages and publishes as a patch release;
- full workspace `cargo fmt`, `cargo clippy -- -D warnings`,
  `cargo test --workspace --all-targets --all-features --locked`,
  `cargo doc --workspace --no-deps --locked`, and `./scripts/check-local.sh`
  green;
- hosted CI green on all lanes.

## 10. Supersession

Superseded only by a later accepted ADR.
