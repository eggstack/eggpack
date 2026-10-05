# Fail-Closed Validation Model — Deep Dive

The workspace treats every externally supplied document as untrusted and parses it
into a typed value or rejects it; no tolerant or best-effort parse path exists
anywhere in the seven crates. This deep dive covers the mechanisms that make that
guarantee hold — unknown-field denial, schema-version equality, bounded counts and
sizes, exact-match resolution, and the `AllowExtras`/`Exact` policy — and is
cross-cutting rather than owned by any one crate.

Everything below is cited to source. Where enforcement is thinner than the stated
invariant, the gap is named in [Where the model is weaker than
stated](#where-the-model-is-weaker-than-stated) rather than smoothed over.

## The model

Operationally, "fail-closed" means four things in this workspace:

1. **Parse or reject, never salvage.** Every entry point returns
   `Result<T, E>`; there is no tolerant or defaulted variant.
   `DistributionContract::parse_toml_str`
   (`crates/eggpack-contract/src/lib.rs:261`), `PackConfig::from_toml`
   (`crates/eggpack-core/src/lib.rs:502`), `ReleaseManifest::from_json`
   (`crates/eggpack-manifest/src/lib.rs:163`), `CIPlan::from_json`
   (`crates/eggpack-ci/src/lib.rs:242`) and
   `BootstrapInstallPolicyV1::from_toml`
   (`crates/eggpack-bootstrap/src/lib.rs:302`) all propagate the underlying
   deserializer error or a fixed generic string and construct nothing.
2. **A partially-valid document does not become a value.** Constructors that
   build owned values (`ReleaseInventory::new`,
   `crates/eggpack-contract/src/lib.rs:1085`; `ArchiveMemberInventory::new`,
   `:1206`) return `Err` on the first invalid entry rather than skipping it.
3. **Identity is never guessed.** Where a name could plausibly match more than
   one artifact, the code compares for equality and errors otherwise; see
   [Exact-match resolution](#exact-match-resolution).
4. **Ambiguity is not resolved by an ambient default.** `CargoZigbuild` requires
   both `cargo_zigbuild` and `zig` present and `NativeCargo` requires both absent
   (`crates/eggpack-core/src/lib.rs:126-141`), with the comment that this "fails
   closed rather than selecting an ambient Zig" (`:508-512`).

Validation splits between the deserializer (`deny_unknown_fields`, document size
bound, version equality) and a hand-written `validate*` pass for cross-field
relationships the schema cannot express. Both run before a value is usable —
with one documented exception, below.

## Unknown fields

`deny_unknown_fields` is applied per struct or per tagged enum, and covers the
document types of every crate that parses external input. Unless a crate path is
given, line numbers are in that crate's `src/lib.rs`.

| Crate | Types carrying `deny_unknown_fields` |
| --- | --- |
| contract | `ProductIdentity` `:133`, `ChecksumSpec` `:144`, `BundleEntry` `:153`, `ArchiveMember` `:163`, raw shapes `RawContract` `:222`, `RawTarget` `:230`, `RawAsset` `:240` |
| manifest | `ReleaseManifest` `:59`, `TargetRecord` `:78`, `ArtifactRecord` `:113`, `BundleRecord` `:125`, `ArchiveMemberRecord` `:135`, `ByteEvidence` `:148`, tagged `ArtifactForm` `:88` |
| core | `HostRequirement` `lib.rs:370`, `ToolchainRequirement` `lib.rs:385`, `TargetPolicy` `lib.rs:419`, `PackConfig` `lib.rs:448`, `BuildBinding` `builder.rs71`, `BuildBindingsV1` `builder.rs83`, tagged `LogicalOutputSelector` `builder.rs53`, `CandidateSmokeBinding` `qualification.rs:44`, `TargetQualificationBinding` `qualification.rs:61`, `QualificationBindingsV1` `qualification.rs:70`, `QualifiedCandidateEvidence` `qualification.rs:258`, `QualificationProcessEvidence` `qualification.rs:278`, `QualificationEvidence` `qualification.rs:290` |
| bootstrap | `:267`, `:279` |
| ci | `:41`, `:57`, `:73`, `:93`, `:452`, `:467`, `:486`, `:502`, `:727`, `:780`, `:815`, `:850`, `:870`, `:1843`, `:1861`, `:2209`, `:2233`, `:2259`, `:2292`, `:2310`, `:3874`, `:3969`, `:4453`, `:4473` |
| github | `:56`, `:187`, `:205`, `:297`, `:975`, `:985`, `:1112` |

Contract parsing attaches its strictness to private `Raw*` mirror types
(`crates/eggpack-contract/src/lib.rs:208-251`) rather than to the public
`DistributionContract` (`:210`). The public type does not derive `Deserialize`;
the raw shapes do the work and `from_raw` (`:374`) converts them.

**The audit answer is yes, there are types that lack it.** The gap is confined
to types that are not part of an untrusted document's own surface:

- `ProcessEvidence` (`crates/eggpack-core/src/builder.rs183`) derives
  `Serialize, Deserialize` with no `deny_unknown_fields`, and is nested inside
  `QualificationProcessEvidence`
  (`crates/eggpack-core/src/qualification.rs:278`), which does carry the
  attribute. Serde's attribute is not inherited by nested structs, so an extra
  key inside a `process` object is accepted.
- `ReleasePlan` (`crates/eggpack-core/src/lib.rs:456`) and `PlannedTarget`
  (`:467`) derive both traits with no `deny_unknown_fields`. I found no
  non-test call site that deserializes a `ReleasePlan` from a string; they are
  constructed in code (`crates/eggpack-core/src/lib.rs:576`,
  `crates/eggpack-ci/src/lib.rs:116`, `:4551`).
- Unit-variant enums such as `QualificationMethod`
  (`crates/eggpack-core/src/qualification.rs:180`) and `QualificationStatus`
  (`:196`) carry no struct body, so there is nothing for the attribute to
  restrict; `QualificationStatus::Failed(QualificationFailure)` (`:203`) takes a
  payload, but a unit enum admits no unknown keys.

## Schema version

`SCHEMA_V1` is declared twice, independently:

- `crates/eggpack-contract/src/lib.rs:15` — the distribution contract document.
  Enforced in `from_raw` at `:375-379`, which returns
  `DistError::UnsupportedVersion { found }` and constructs nothing. The value is
  not carried through from the raw document; `from_raw` writes
  `schema_version: SCHEMA_V1` (`:431`), so a parsed contract always reports 1.
- `crates/eggpack-manifest/src/lib.rs:10` — the release manifest document.
  Enforced in `ReleaseManifest::validate` at `:176-178`, returning
  `ManifestError::UnsupportedVersion`.

The duplication implies two documents with separate version lifecycles. Neither
constant is shared, so a contract at v1 and a manifest at v1 have nothing tying
their versions together; the relationship is expressed structurally instead
(`product_id` and `release_id` must agree, checked at
`crates/eggpack-core/src/lib.rs:170-172`). Bumping either constant alone would
be a breaking change to that document only.

- The remaining four crates do not reuse either constant and compare against the
  literal `1` inline, each in its own `validate` or `validate_shape`. Sites
  abbreviated to `<crate>:<line>`:

| Crate | Enforcing sites |
| --- | --- |
| core | `lib.rs:490`, `lib.rs:504`, `lib.rs:533`, `builder.rs238`, `qualification.rs:138`, `qualification.rs:325`, `qualification.rs:574` |
| bootstrap | `lib.rs:323`, `lib.rs:409` |
| ci | `lib.rs:209`, `:253`, `:1897`, `:2011`, `:2353`, `:3912`, `:4011`, `:4514`, `:4613` |
| github | `lib.rs:124`, `:253`, `:347`, `:1025`, `:1162` |

Version checking is therefore duplicated roughly twenty times, and the
consistency is enforced by tests rather than by a shared constant: for example
`crates/eggpack-manifest/src/lib.rs:446` and
`crates/eggpack-bootstrap/src/lib.rs:2041` assert that a document with
`schema_version` set to 2 is rejected.

## Bounds

Bounded parsing matters because every one of these documents can originate from
a repository, a CI artifact, or a provider response rather than from the
producer. An unbounded recursive or collection-shaped document is a memory
exhaustion vector, and an unbounded diagnostic string is a log-flooding vector.
Both are closed here.

| Constant | Value | Enforcing crate | What it bounds |
| --- | --- | --- | --- |
| `MAX_OBSERVED_ENTRIES` | 256 | contract `:23` | Entries in caller-supplied release and archive inventories; `ReleaseInventory::new` `:1093`, expected-set size `:1141`, `ArchiveMemberInventory::new` `:1214`, expanded member count `:1254` |
| `MAX_CONFORMANCE_FINDINGS` | 512 | contract `:24` | Findings retained in a `ConformanceReport`; truncation sets `truncated: true` at `:1417-1418` |
| `MAX_ID_LEN` / `MAX_NAME_LEN` | 64 / 128 | contract `:18-19` | Product id, display name, target triple, alias length |
| `MAX_TEMPLATE_LEN` | 256 | contract `:20` | Name templates and observed flat filenames (`validate_observed_flat_name` `:1120`) |
| `MAX_DETAIL_LEN` | 512 | contract `:21` | Every `DistError` string, via `bound()` `:123-128` |
| `MAX_DOCUMENT_BYTES` | 1 MiB | manifest `:12` | Encoded manifest JSON, checked before deserialization at `:164-166` |
| `MAX_TARGETS` / `MAX_RECORDS` | 256 / 256 | manifest `:14`, `:16` | Target records `:184`; bundle entries `:198` and archive members `:210` |
| `MAX_EVIDENCE_REFERENCES` | 64 | manifest `:18` | Evidence reference count `:186` |
| `MAX_ID` / `MAX_NAME` / `MAX_REVISION` / `MAX_EVIDENCE` | 128 / 255 / 128 / 256 | manifest `:19-22` | Product id, release id, target, artifact filename, source revision, evidence reference |
| `MAX_TARGETS` | 256 | core qualification `:22` | Qualification plan targets `:138`; evidence candidates (inline `256`) `:336` |
| `MAX_ARGS` / `MAX_ARG_LEN` | 128 / 4096 | core qualification `:23-24` | Process record count as `1 + MAX_ARGS` `:337`; per-argument length |
| `MAX_CAPTURE` / `MAX_BINARY_HEADER` | 256 KiB / 4096 | core qualification `:25-26` | Recorded stdout/stderr byte counts per process `:375-376`; bytes read for structural header detection |
| `MAX_OUTPUT` | 256 KiB | core builder `:18` | Per-stream capture limit, default `:362-363` and ceiling `:411-412` |
| `MAX_CI_PLAN_JSON`, `MAX_RELEASE_PLAN_JSON`, `MAX_HANDOFF_JSON`, `MAX_EVIDENCE_JSON`, `MAX_SHAPE_JSON` | 1 MB each | ci `:22`, `:1837-1839`, `:4439` | Encoded graph documents, checked before parse (e.g. `:243`, `:2340`) |
| `MAX_OUTPUTS` / `MAX_WORKFLOW_BYTES` | 4096 / 8 MB | ci `:23-24` | Rendered workflow output count `:325`; rendered workflow size |
| `MAX_VALIDATOR_JSON`, `MAX_CONSUMER_EVIDENCE_JSON`, `MAX_VALIDATOR_SCRIPT_BYTES` | 64 KiB, 64 KiB, 1 MiB | ci `:3846-3849` | Validator and consumer-evidence documents; validator script size |
| `MAX_POLICY_JSON` / `MAX_POLICY_TARGETS` / `MAX_POLICY_ENTRIES` | 256 KiB / 256 / 256 | bootstrap `:288-290` | Install policy document, target count `:323`, per-target entry count |
| `MAX_TAG_PEEL_DEPTH` | 8 | github `:25` | Git tag peel traversal depth |
| `MAX_POLICY_JSON` / `MAX_PAYLOAD_JSON` | 256 KiB / 1 MiB | github `:26-27` | Staging policy document; GitHub request payload |
| `MAX_BODY_NOTES` / `MAX_TITLE` / `MAX_NAME_SEGMENT` | 64 KiB / 256 / 128 | github `:28-30` | Release body, title, asset name segments |
| `MAX_WRAPPER_BYTES` | 1 MiB | github `:950` | Installer wrapper source size |
| `MAX_PRESENTATION_JSON` / `MAX_TEMPLATE_JSON` / `MAX_TITLE_PREFIX` | 64 KiB / 64 KiB / 128 | github `:951-953` | Presentation and template documents, title prefix |

Structural string validation is separate from length bounds and equally
consistent: `validate_triple` (`crates/eggpack-contract/src/lib.rs:623`) rejects
control characters, path and URI separators, spaces, and any character outside
`[A-Za-z0-9-_.]`, and requires at least one `-`. `validate_observed_flat_name`
(`:1119`) rejects control characters and `/` or `\`, so a reported name can
never be a path.

## Exact-match resolution

Where a name selects an artifact, the comparison is string equality and nothing
else.

`DistributionContract::resolve` (`crates/eggpack-contract/src/lib.rs:280-290`)
scans entries testing `t.triple == target_or_alias` and
`t.aliases.iter().any(|a| a == target_or_alias)`, returning
`DistError::UnknownTarget` otherwise. The doc comment states the intent
directly: "unknown inputs fail with `DistError::UnknownTarget` instead of
guessing a nearby architecture" (`:279`). `ReleaseManifest::target`
(`crates/eggpack-manifest/src/lib.rs:156-163`) is stricter still — it searches
`t.target == canonical_triple` only, and the comment records that "this never
resolves aliases or guesses" (`:156`).

**Aliases are allowed in exactly one place and that is safe.** `resolve` accepts
an alias, but only because the contract guarantees alias unambiguity at parse
time. `from_raw` (`crates/eggpack-contract/src/lib.rs:398-417`) rejects an
alias that collides with any triple — including its own target's triple — and
rejects one alias mapping to different targets. Unambiguity is established once,
at construction, and the lookup itself is a linear equality scan. An
`Exact` consumer such as the manifest never reaches this path at all.

**Set equality for plan coverage.** Two rules require the supplied bindings to
cover the plan exactly, where a missing entry and an extra entry are both errors:

- Build side — `BuildBindingsV1::validate_for`
  (`crates/eggpack-core/src/builder.rs269-309`) computes the expected selector
  set from the contract expansion (`:273-283`), converts the supplied bindings to
  a set (`:284`), and rejects any difference (`:285-289`, "bindings do not
  exactly cover contract Cargo slots"). A separate check at `:291-297` rejects a
  target present in bindings but absent from the plan.
- Qualification side — `QualificationBindingsV1::validate_for`
  (`crates/eggpack-core/src/qualification.rs:86-99`) compares
  `self.targets.len() != plan.targets.len()` and rejects any key not in the plan
  (`:91-98`, "qualification target inventory differs from ReleasePlan"), with
  equality re-established by the per-target lookup at `:103-105`.
  `BuildBindingsV1::validate_shape` (`crates/eggpack-core/src/builder.rs237-265`)
  separately rejects duplicate selectors within a target and validates archive
  member source paths.

`QualificationEvidence::validate_for`
(`crates/eggpack-core/src/qualification.rs:319-394`) applies the same rule to
evidence: candidate count must equal the attempt's count unless the status is
`Failed` (`:334-335`), each record must resolve to a known candidate by exact
selector with every field matching (`:352-359`), candidates must be strictly
ascending by selector (`:369-372`), and the classification-to-method pairing is a
closed allowlist (`:383-393`). `project_ci_plan`
(`crates/eggpack-ci/src/lib.rs:110-141`) closes the loop from the other
direction: it reconstructs a `PackConfig` from the plan, re-resolves it, and
rejects the plan if the resolution differs (`:137-140`).

## The extras policy

`ExtrasPolicy` (`crates/eggpack-contract/src/lib.rs:1011-1018`) is a
caller-supplied choice with `AllowExtras` as the `#[default]` variant:

- `AllowExtras` — report only missing contract-declared names.
- `Exact` — additionally report every observed name the contract does not
  declare.

The choice is a parameter, not a stored field, and is threaded into both
inventory validators: `validate_release_inventory`
(`crates/eggpack-contract/src/lib.rs:1136`) applies it at `:1180-1191`, and
`validate_archive_member_inventory` (`:1244`) at `:1280`. Neither function
branches on anything else, so the caller fully determines strictness.

**The trade-off is real and the default is the looser side.** With
`AllowExtras`, the contract is an allowlist-with-tolerance, not an exact match:
a release containing an unexpected file conforms. That is the right default for
a function whose job is to report what a provider published, where unrelated
files such as signatures or attestations are legitimate and out of the
contract's scope. It is the wrong default for a producer asserting that a
release contains exactly what it declared.

Production selection is stricter than the default. `core::build_manifest`
passes `ExtrasPolicy::Exact` at `crates/eggpack-core/src/lib.rs:189` and rejects
a non-conformant report at `:190` ("release inventory is incomplete or contains
extras"). The archive branch is even tighter: it compares lengths directly and
rejects any difference (`:232-233`, "archive member inventory incomplete or has
extras"). Searching the workspace, the only call sites of the two validators
outside their own definitions are in `core`; nothing in the workspace currently
passes `AllowExtras` explicitly, and nothing passes the default through. The
loose default therefore exists as API surface for a consumer-side caller, not as
a live path in the producer pipeline.

## Two-tier validation and its implication

In `crates/eggpack-core/src/lib.rs`, `PackConfig` validation is genuinely
two-tier, and the tiers are not equivalent.

The first tier, `from_toml` (`:502-524`), is a weak gate. It deserializes with
`deny_unknown_fields` on both `PackConfig` (`:447`) and `TargetPolicy` (`:418`),
checks `schema_version == 1` and non-empty targets (`:504-506`), then applies a
single `.any()` pass (`:513-522`) that checks each target has a non-empty name,
is not a duplicate, has a non-empty toolchain string of at most 64 characters,
and satisfies one parity condition: `CargoZigbuild` iff `cargo_zigbuild` is
present.

The second tier, `validate_policy` (`:105-163`), runs only inside
`PackConfig::resolve` (`:526-545`), at `:541`. It is strictly stronger and
checks things the parse tier does not: character-class validation of the Rust
and cross-tool version strings via `valid_tool_version` (`:97-103`, `:106-125`);
that `NativeCargo` declares neither cross tool and `CargoZigbuild` declares both
(`:126-141`); that a glibc floor applies only to `-linux-gnu` triples and a
macOS floor only to `-apple-darwin` (`:142-150`); and that a `Native`
qualification's host actually matches the target (`:151-161`).

**A parsed-but-unresolved `PackConfig` can therefore carry an invalid policy.**
Concretely, a document that parses successfully can still contain a toolchain
version with illegal characters, a `CargoZigbuild` target with `cargo_zigbuild`
set but `zig` absent, a glibc floor on a Windows triple, or a `Native`
qualification whose host does not match. The parse-tier parity condition at
`:518-519` is deliberately written to admit that last case.

**Practical implication: consumers must call `resolve`, not merely parse.**
`PackConfig::from_toml` is a shape check, not an admission decision. Every
production path in the workspace treats it that way — `resolve_runtime_release_plan`
(`crates/eggpack-ci/src/lib.rs4613-4623`) checks the version, calls `resolve`,
and then independently re-checks that the plan's release id and revision equal
the exact tag and revision, and `project_ci_plan` (`:128-140`) re-resolves and
compares. I found no non-test call site of `PackConfig::from_toml`; its only
uses are in tests (`crates/eggpack-core/src/lib.rs:665`, `:852`;
`crates/eggpack-ci/src/lib.rs9929`).

The Zig rule is deferred on purpose, and the reason is written down at
`crates/eggpack-core/src/lib.rs:508-512`: `zig` is an additive optional field so
that historical schema-v1 documents without it stay parseable, and the strict
presence rules are enforced at resolution, "which fails closed rather than
selecting an ambient Zig". The same rationale is repeated on the type at
`:378-382`. This is a deliberate schema-version-1 compatibility decision, not an
oversight.

## Error surface

Errors are typed per crate and never cross a crate boundary as a generic string.

- Two crates use enums. `DistError` (`crates/eggpack-contract/src/lib.rs:47-80`)
  is `#[non_exhaustive]` with a distinct variant per failure class, including
  `UnsupportedVersion { found }`, `UnknownTarget`, and `NameCollision`, which
  carries only bounded logical field labels and never an arbitrary filename
  (`:68-75`). `ManifestError` (`crates/eggpack-manifest/src/lib.rs:27`) is
  likewise `#[non_exhaustive]`.
- Four crates use single-field newtypes: `CoreError`
  (`crates/eggpack-core/src/lib.rs:74`), `BuildError`
  (`crates/eggpack-core/src/builder.rs92`), `QualificationError`
  (`crates/eggpack-core/src/qualification.rs:30`), `CiError`
  (`crates/eggpack-ci/src/lib.rs:28`), `GithubError`
  (`crates/eggpack-github/src/lib.rs:40`), and `BootstrapError`
  (`crates/eggpack-bootstrap/src/lib.rs:20`) — six types across four crates.
  These are opaque at the type level; only the message string varies, and the
  messages are fixed literals or bounded fields.

Two rules govern message content.

**Diagnostics never echo captured process output.** The process runner drains
both streams into bounded buffers (`crates/eggpack-core/src/builder.rs542-552`),
collapses them to a typed `CommandOutcome` (`:542-556`), and returns a
`ProcessEvidence` (`:174-181`) carrying only the outcome plus byte *counts*. No
error in the workspace embeds stdout or stderr text. Environment values are
excluded by construction: `BuildAttempt::tool_summary` (`:211-212`) is
documented as a "tool summary, without environment values", and
`QualificationProcessEvidence`
(`crates/eggpack-core/src/qualification.rs:276`) as carrying no output contents
or environment values. The expected-stdout substring check at
`crates/eggpack-core/src/builder.rs:541-543` is the only read of captured bytes
and it yields a `Failed(-1)` outcome, not a message.

**Diagnostics are length-bounded.** `bound()`
(`crates/eggpack-contract/src/lib.rs:123-128`) truncates every `DistError` detail
to `MAX_DETAIL_LEN` (512), applied on every construction path including
`UnknownTarget` (`:289`) and the inventory errors (`:1094`, `:1100`).

## Where the model is weaker than stated

Four findings, all verified above, none of them a claim about intent.

1. **`deny_unknown_fields` does not reach nested structs.**
   `ProcessEvidence` (`crates/eggpack-core/src/builder.rs183`) has no
   attribute, so unknown keys inside a `QualificationProcessEvidence.process`
   object are accepted, even though the containing type
   (`crates/eggpack-core/src/qualification.rs:277-285`) is strict. `ReleasePlan`
   (`crates/eggpack-core/src/lib.rs:456`) and `PlannedTarget` (`:467`) are also
   unannotated. In practice the first is reachable from a serialized evidence
   document; the latter two are constructed in code today.
2. **The two-tier `PackConfig` gap is a real hole in the invariant.** A
   successful `PackConfig::from_toml` (`:502`) proves only shape. Every rule in
   `validate_policy` (`:105-163`) is unevaluated until `resolve` (`:541`) is
   called. The workspace's own callers comply, but the type does not make that
   obligation visible to a future consumer.
3. **The parse-tier predicate short-circuits.** The `.any()` at
   `crates/eggpack-core/src/lib.rs:513-522` stops at the first failing target, so
   a document with several problems reports one generic message
   ("invalid or duplicate PackConfig target", `:521`), and the `seen` set is
   only partially populated. Rejection is still fail-closed — the effect is
   diagnostic quality, not acceptance.
4. **Host matching is string-prefix matching, not parsed-triple matching.**
   `host_matches_target` (`crates/eggpack-core/src/lib.rs:84-96`) tests
   `triple.starts_with("x86_64-")` for architecture and then
   `triple.contains("-linux-")`, `ends_with("-apple-darwin")`, or
   `contains("-windows-")` for OS. `contains` is a substring test, not a
   component test, and the triple is never parsed into components. The
   independent copy in `eggpack-ci` (`crates/eggpack-ci/src/lib.rs:393-405`) has
   the same property. The nearby floor checks at
   `crates/eggpack-core/src/lib.rs:143` and `:146` use the same substring/prefix
   idiom, so floor applicability inherits the same weakness.

One further observation, offered as a bound rather than a defect: the
distribution contract has no upper bound on the number of targets it declares.
`from_raw` rejects an empty target list (`crates/eggpack-contract/src/lib.rs:384`)
and rejects duplicates, but I found no `MAX_TARGETS`-style ceiling in that crate,
while `manifest` (`:14`), `core` (`crates/eggpack-core/src/lib.rs:173`),
`ci` (`:256`), and `bootstrap` (`crates/eggpack-bootstrap/src/lib.rs:323`) each
cap their own target count at 256. The contract is producer-authored rather
than remotely supplied, so this is a consistency gap in the bounds table rather
than an untrusted-input exposure.

## Related deep dives

- [overview.md](overview.md) — module map and producer pipeline; start here.
- [determinism.md](determinism.md) — canonical sort and byte-identical render.
- [contract.md](contract.md) — the layout authority behind `resolve`, `expand`, and the inventory validators.
- [manifest.md](manifest.md) — schema-v1 evidence types and the second `SCHEMA_V1`.
- [core-planning.md](core-planning.md) — `PackConfig::from_toml`, `resolve`, `ReleasePlan`.
- [core-build.md](core-build.md) — build bindings, exact plan coverage, `ProcessEvidence`.
- [core-qualification.md](core-qualification.md) — evidence identity and the method allowlist.
- [process-execution.md](process-execution.md) — process groups, bounded capture, cleared env.
- [testing-and-portability.md](testing-and-portability.md) — how the rejection paths are exercised.
