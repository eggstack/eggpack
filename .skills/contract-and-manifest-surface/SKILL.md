---
name: contract-and-manifest-surface
description: Change Eggpack's schema-v1 distribution contract or release manifest, or add a target, asset kind, bundle entry, archive member, install name, or collision rule. Use when touching eggpack-contract, eggpack-manifest, PackConfig target entries, or ExtrasPolicy. Encodes the single-authority rule, the deny_unknown_fields compatibility trap, and which of the two documents is published.
---

# Contract and manifest surface

These two documents are the ones other repositories depend on. `eggpack-contract`
is the layout authority; `eggpack-manifest` is a **published** crate. Treat edits
to either as interface changes, not as local refactors.

## One authority per name

```text
DistributionContract  owns every name: assets, sidecars, bundle entries,
                      archive members, install names
        |
        +--> PackConfig / ReleasePlan   references those names, never redefines them
        +--> ReleaseManifest            records the names that were actually produced
```

If a name appears in `core`, `bootstrap`, `ci`, or `github` as a literal rather
than a reference back to `contract`, that is a bug regardless of whether the
string is currently correct. PackConfig selects and configures; it does not
introduce.

## The `deny_unknown_fields` compatibility trap

This is the single most important fact when editing either schema. Both documents
are `deny_unknown_fields` throughout — `eggpack-manifest` has it on every
deserialized struct. That means:

**Adding an optional field to a schema-v1 document is a breaking change for every
older parser that reads it.** There is no forward compatibility. An
`eggpack-manifest 0.1.0` consumer handed a document containing a field added
later does not ignore the field — it fails closed.

So "it's optional, so it's safe" is false here. A shape change to the published
manifest is a compatibility event, not a convenience. It needs a plan, an ADR if
it establishes a public compatibility contract, and downstream coordination.

`eggpack-manifest 0.1.0` is live on crates.io and consumed by `eggup-eggpack`
downstream. The published `src/lib.rs` is byte-identical to the
consumer-qualified pin the consumer was tested against — that equality is a
deliberate, recorded property, not an accident. The other six workspace crates
are unpublished; do not publish one as a side effect of unrelated work.

## The two documents say different things

| | Contract | Manifest |
|---|---|---|
| Answers | what *should* exist | what *does* exist |
| Contains | intent, expected layout | final bytes, digests, sizes |
| Must never contain | — | intent, policy, installed state, authenticity claims |

`ReleasePlan` sits between them and is likewise **intent, not evidence**. The
four-object split (Contract / Plan / Manifest / Eggup `InstallReceipt`) is
normative in `principles-roadmap.md` §1. If a change makes one carry another's
job, that is an architecture change.

## Bounds and extras

Validation is fail-closed: `schema_version == 1`, bounded counts and sizes
(`MAX_OBSERVED_ENTRIES`, `MAX_TARGETS`, `MAX_RECORDS`, `MAX_EVIDENCE_REFERENCES`,
`MAX_DOCUMENT_BYTES`), and exact-match resolution where identity matters — no
guessing a target or alias.

`ExtrasPolicy` has two settings, and the default matters: `AllowExtras` is the
default and tolerates observed entries the contract does not name; `Exact` is
opt-in and reports them. When you add a field or a new artifact form, decide
explicitly whether an older or newer producer's output should be accepted — do
not change the default casually, because `AllowExtras` → `Exact` is a
fail-closed tightening that can reject releases that previously staged.

## Collision rules, which are not symmetric

- **Install-name collisions are target-local** — two different targets may both
  install an executable of the same name.
- **Release-artifact filename collisions are manifest-global** — two artifacts
  with the same release filename are a conflict.

Do not "harmonize" these. They were separated deliberately to fix a real
conformance defect, and the registry records the reasoning.

## Integrity is not authenticity

SHA-256 digests and sizes are **integrity facts only**. No signature, no
attestation, no provenance claim is made anywhere in this workspace, and
`ByteEvidence` is named to keep that boundary visible. Do not add language to
docs, error messages, or code that implies otherwise. Adding a real trust model
requires a dedicated ADR — external artifact-attestation support elsewhere in
the ecosystem is prior art, not a selection.

## Adding a target or artifact form

1. Define the name in `eggpack-contract` only.
2. Confirm the resolution and expansion behavior in `contract.md` §"Target and
   alias resolution" and §"The `expand` projection" — exact-match, no guessing.
3. Check both inventory validators: `expected_release_files` /
   `validate_release_inventory` and `ArchiveMemberInventory` /
   `validate_archive_member_inventory`. New artifact forms usually need both.
4. Archives are `TarGzip` only, and the output filename must end `.tar.gz`, with
   deterministic timestamps and ownership.
5. Update the fixtures under `crates/eggpack-contract/tests/` and the
   conformance tests; then run the `pre-submit-gate` skill.
6. If the change affects what a consumer must understand, check the Eggup
   mapping in `eggup-manifest-consumer-v1.md` and say so in the plan.

## Reference

- [architecture/contract.md](../../architecture/contract.md) — authority, schema-v1, validators
- [architecture/manifest.md](../../architecture/manifest.md) — evidence-only, bounds, collisions
- [architecture/validation-model.md](../../architecture/validation-model.md) — the fail-closed model
- [architecture/principles-roadmap.md](../../architecture/principles-roadmap.md) — the four-object split
- [architecture/eggup-manifest-consumer-v1.md](../../architecture/eggup-manifest-consumer-v1.md) — consumer mapping
