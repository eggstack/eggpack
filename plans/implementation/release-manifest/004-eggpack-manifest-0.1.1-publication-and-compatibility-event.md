# Release Manifest Milestone 004 — `eggpack-manifest 0.1.1` Publication and Compatibility Event Record

Status: active

Closure record: `plans/closure/release-manifest/004-status.md`

Repository authoring baseline: `c908996ceebc3429d88b0b3480dbe5b84042c9b0`

Crate version carried by this milestone: `eggpack-manifest 0.1.1` (already set in
the tree since `9cf0bf8`)

Published identity to be produced: crates.io `eggpack-manifest 0.1.1`, tag
`eggpack-manifest-v0.1.1`, published from the publication source commit named in
the closure record

Primary roadmap: `plans/subsystems/release-manifest-roadmap.md`

Governing decision: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md`
(Status: **accepted**, Option A)

Cross-repo dependency / evidence:

- the published `eggpack-manifest 0.1.0`: crates.io, checksum
  `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`, tag
  `eggpack-manifest-v0.1.0`, published from
  `8d661e4eb9da1806e5d7c7806939d24e9aceb2c0`, hosted run `37064833069` green;
- the affected downstream consumer: `eggup-eggpack`, which consumes registry
  `eggpack-manifest =0.1.0` (`eggstack/eggup@648b1db`, branch `main`).

Primary class: compatibility / packaging / manual publication prerequisite

## 1. Why this milestone exists

The 2026-10-23 post-implementation review accepted ADR-0006 (Option A) and
implemented its digest-acceptance work package in CI/Release M003l. ADR-0006
§5.2 and §9 created an obligation that the review chain never discharged:

> `eggpack-manifest` publishes a new patch release (`0.1.1`) carrying that rule.

The crate was bumped to `0.1.1` in-tree by `9cf0bf8`, and the tree has carried
`0.1.1` unchanged since. Nothing was published. As of this plan's authoring
baseline the crates.io API reports `num_versions: 1`, `max_version: "0.1.0"`,
`newest_version: "0.1.0"` for `eggpack-manifest` — the `0.1.1` version does
not exist on the registry.

Three separate gaps follow from that, and this milestone closes all three.

**Gap 1 — an unregistered obligation.** No plan, closure record, registry row,
or roadmap entry tracked the `0.1.1` publication. `plans/registry.md` listed
CI M003l as closed and stated that the published crate "takes a `0.1.1` patch
release", while
`plans/subsystems/release-manifest-roadmap.md` asserted that "No further
Release Manifest implementation is implied." Both statements were true about
the *code* and false about the *obligation*. A milestone that was accepted by
ADR and never planned is exactly the state the planning process exists to
prevent, and it would have survived indefinitely because no code change would
ever fail a test.

**Gap 2 — ADR-0006 §5.4 is only half satisfied.** The ADR requires the
narrowing to be recorded as a compatibility event "in the release-manifest
closure record **and** in `architecture/manifest.md`". The second surface
exists (`architecture/manifest.md` §"The ADR-0006 compatibility event"). The
first does not: no file under `plans/closure/release-manifest/` mentions
ADR-0006, the all-zeros digest, or the `0.1.1` publication. The event lived
only in a CI/Release closure record and an architecture document — a
cross-subsystem substitute for the record the ADR named.

**Gap 3 — a second narrowing ships in the same package and was recorded
nowhere.** The `0.1.1` package carries **two** narrowings of this crate's
public acceptance set, not one.

- *The digest narrowing* is ADR-0006's. `validate_sha256_hex` (`:98`) refuses
  the all-zeros sentinel, and `ArtifactRecord::validate`, `ByteEvidence::validate`,
  and `decode_sha256` delegate to it.
- *The document-bound narrowing* arrived by a different route: commit `3ef806f`
  ("close 31 audited defects across all seven crates"). `ReleaseManifest::validate`
  now ends with `check_document_bound` (`:304`, `:309-320`), which measures the
  canonical encoding and returns `ManifestError::TooLarge` past
  `MAX_DOCUMENT_BYTES`.

The second was recorded only as an implementation note inside
`architecture/manifest.md` §"Canonical ordering". It appears in no ADR, in no
closure record, and in no compatibility-event surface. A consumer notified only
from the compatibility-event surfaces would learn about the digest rule and not
about the new document-bound refusal — a compatibility change shipping
unannounced.

This milestone publishes the patch release and writes the record ADR-0006 §5.4
requires, covering **both** narrowings.

## 2. Why ready

- The crate is already at `0.1.1` in the tree and has been unchanged since
  `9cf0bf8`. `git diff 9cf0bf8..HEAD -- crates/eggpack-manifest/` is empty.
- The `0.1.1` version is unoccupied on crates.io, confirmed immediately before
  publication.
- The crate is a leaf: its only dependencies are registry `serde` and
  `serde_json`. It has ordinary crates.io metadata and no `publish = false`.
- The workspace version stays `0.1.0` for the other six crates, so the patch
  release does not imply a workspace-wide release.
- `^0.1.0` dependents resolve `0.1.1` unchanged, so no in-tree dependency
  edge needs to move.
- The local gate is green at the authoring baseline, and the package and
  publish dry-run both pass (§6.4).

## 3. Objective

1. Publish `eggpack-manifest 0.1.1` to crates.io through the existing manual
   publication boundary, as the ADR-0006 patch release.
2. Prove the published artifact resolves from the registry and that an
   external registry-only consumer can build against the exact version.
3. Record **both** narrowings the package carries as compatibility events, in a
   release-manifest closure record — the record ADR-0006 §5.4 names — and in
   `architecture/manifest.md`.
4. Record the notification to `eggup-eggpack` and the consumer-side work it
   implies, without changing the Eggup repository.
5. Correct the release-manifest roadmap's now-false "no further implementation
   is implied" claim, and register this milestone in `plans/registry.md`.

## 4. Authoritative package and compatibility identity

| Property | Value |
|---|---|
| Crate | `eggpack-manifest` |
| Version | `0.1.1` |
| Predecessor on crates.io | `0.1.0` (checksum `2a08f24b…b629`, non-yanked, tag `eggpack-manifest-v0.1.0`) |
| Schema version | `1` — unchanged |
| Public items removed | none |
| Public items added | `Sha256Error`, `validate_sha256_hex` (additive) |
| Document shape / field set | unchanged |
| Narrowing 1 | the all-zeros SHA-256 sentinel is refused (ADR-0006, Option A) |
| Narrowing 2 | a manifest whose canonical encoding exceeds `MAX_DOCUMENT_BYTES` is refused by `validate()` and `to_json()` (`3ef806f`) |
| Publication boundary | manual, explicit, single crate, no automated publication workflow |

**The package checksum is a function of the publication source commit.** The
generated `.cargo_vcs_info.json` embeds the commit SHA, so the `.crate` bytes
differ per commit even when the crate sources are identical. The checksum
recorded in the closure record must be computed at the publication source
commit, not at this plan's authoring baseline. The baseline build produced
`98d78391b050fd619615eef25d644ff374d1b9a15c46d12d74abab459bb23c8d` at
`c908996ceebc3429d88b0b3480dbe5b84042c9b0`; that value is **not** the
publication checksum and is recorded here only to show the build is
reproducible from a clean tree.

## 5. Invariants

- Only `eggpack-manifest` is published. No other workspace crate is published,
  and no workspace-wide version policy is implied or implied-by.
- Publication happens from an explicit `cargo publish -p eggpack-manifest
  --locked` on a clean tree. Never a bare `cargo publish`, never `--allow-dirty`
  for the publication evidence.
- No automatic publication workflow is added. The generated CI is unchanged and
  the `ci` family stays byte-identical.
- The package's dependency set stays registry-only and leaf-sized.
- The published source representation is not edited after publication. A
  cosmetic edit after the fact is a compatibility incident, not a fix.
- Every narrowing the package carries is recorded as a compatibility event in a
  release-manifest closure record and in `architecture/manifest.md`.
- The affected downstream consumer is named in the compatibility event. A
  narrowing is announced, not discovered.
- `ReleaseManifest` stays at `schema_version: 1`. No field is added or removed.
- Checksums remain integrity facts. No signature, attestation, or provenance
  claim is made or implied.
- The Eggup repository is not modified. A change there requires a plan
  registered in that repository.
- Prior closure records are not rewritten. M003 and M003a's deferred-finding
  wording stays as written; this milestone records that the deferred item is
  now discharged.

## 6. In scope

### 6.1 Reconcile M003 planning truth

The release-manifest roadmap's claim that no further Release Manifest
implementation is implied is false from this milestone's baseline. Correct it in
this milestone's own plan/closure cycle rather than editing the closed M003 or
M003a records. The M003 closure recorded the published README's dangling
relative reference as *deferred to a future `0.1.1`/`0.2` metadata polish*; the
`0.1.1` README discharges that item by using an absolute repository URL, and
this milestone records that discharge.

### 6.2 Freeze and audit the crate source

Confirm that `crates/eggpack-manifest/` is unchanged from `9cf0bf8`, that the
version is exactly `0.1.1`, that there is no `publish = false`, and that the
dependency set is registry-only. Record the exact source delta against the
published `0.1.0` source (`8d661e4`) and classify every difference as
widening, narrowing, or neutral.

### 6.3 Compatibility baseline

Enumerate the public surface of `0.1.1` against `0.1.0`: items added, items
removed, signatures changed, and behaviour changed. Confirm empirically — not
only by reading — that the two narrowings behave as claimed, and that neither
is over-tightened.

### 6.4 Package qualification

```text
$ cargo package -p eggpack-manifest --locked
$ cargo publish -p eggpack-manifest --locked --dry-run
```

Both from a clean tree at the publication source commit. No `--allow-dirty`.
Record the packaged file list and the `.crate` checksum computed at that commit.

### 6.5 First-publication availability gate

Immediately before the real publish, confirm on the crates.io API that
`eggpack-manifest 0.1.1` is absent. If the version exists, stop: the
precondition for a first publication is violated and the milestone must not
proceed to a publish attempt.

### 6.6 Manual publication

```text
$ cargo publish -p eggpack-manifest --locked
```

Exactly one crate, from the publication source commit, with a clean tree. This
is the manual publication boundary the workspace rules require; it is not
performed by CI and no automated publication workflow is added.

After a successful publish, create the crate-scoped source tag
`eggpack-manifest-v0.1.1` at the publication source commit. A generic
workspace `v0.1.1` tag is not used: it would falsely imply the whole Eggpack
workspace was published.

### 6.7 Post-publication registry proof

- The crates.io API exposes `eggpack-manifest 0.1.1`, non-yanked, with the
  recorded checksum.
- A temporary, external, registry-only consumer crate depends on
  `eggpack-manifest = "=0.1.1"` with no Git or path source, builds, and runs a
  smoke test: parse a schema-v1 document, round-trip it, and confirm the two
  narrowings are observed through the published artifact.

### 6.8 Compatibility-event record

Write `plans/closure/release-manifest/004-status.md` recording **both**
narrowings as compatibility events:

- the ADR-0006 digest narrowing, naming `eggup-eggpack` as the affected
  downstream consumer; and
- the document-bound narrowing, naming the same consumer and stating that it
  changes what `validate()` and `to_json()` accept.

`architecture/manifest.md` already carries both; the ADR requires both
surfaces, and the closure record is the one that was missing.

### 6.9 Downstream handoff

Record the notification to `eggup-eggpack` and the consumer-side work it
implies. `eggup-eggpack` pins `eggpack-manifest = "=0.1.0"` — an exact
requirement — so publishing `0.1.1` does **not** move the consumer
automatically; the consumer must bump the pin, update its asserted pin
constant, and re-qualify under a plan registered in the Eggup repository.
This milestone records the obligation and the evidence that the narrowing
breaks nothing in the consumer today. It does **not** change the Eggup
repository, and this closure does **not** claim the consumer was migrated.

## 7. Out of scope

- Publishing any other Eggpack crate.
- Changing `ReleaseManifest`'s schema version, field set, or wire shape.
- Changing the digest *computation* path or the file-digest implementations.
- Any signature, attestation, or provenance work.
- Any change in the Eggup repository, or any claim that the Eggup consumer was
  migrated.
- A workspace-wide version or release policy.
- Formal JSON Schema export.
- Any change to the generated CI, the `ci` family, or the workflow bytes.
- Yanking, amending, or replacing the published `0.1.0`. It stays published and
  resolvable.
- Editing the published source representation after publication.

## 8. Required work packages

### WP1 — Planning and source-identity reconciliation

Freeze and audit the crate source against `9cf0bf8` and the published
`8d661e4`; classify every public-surface difference.

Exit: the source delta and its classification are recorded.

### WP2 — Package/API/contents audit

Enumerate the packaged file list; check every link and path in the published
README against it; confirm the version, the `publish = false` absence, and the
registry-only dependency set.

Exit: the package contents and their compatibility surface are recorded.

### WP3 — Local publication qualification

Run the package and publish dry-run from a clean tree; run the crate's test
lane and the full local gate; require hosted CI green on the exact candidate
commit before the real publish.

Exit: clean package/dry-run and hosted qualification are recorded.

### WP4 — First-publication gate and manual publish

Confirm the intended publisher authority; confirm `0.1.1` is absent on the
registry; perform only the explicit
`cargo publish -p eggpack-manifest --locked`; capture the registry, checksum,
and source-commit identity; create the crate-scoped tag.

Exit: the published identity and tag are recorded.

### WP5 — Registry-only consumer proof

Build and smoke an external registry-only consumer against `=0.1.1`.

Exit: the registry-only proof is recorded.

### WP6 — Compatibility-event and closure record

Write the release-manifest closure record covering both narrowings, the
consumer notification, and the roadmap/registry reconciliation. Prove an empty
production diff against the publication source commit apart from the version
bump already carried by `9cf0bf8`.

Exit: the closure record is accepted.

## 9. Acceptance criteria

| Criterion | Evidence |
|---|---|
| Version published exactly once | crates.io exposes non-yanked `0.1.1` |
| Publication source recorded | full SHA, tag `eggpack-manifest-v0.1.1`, `.crate` checksum at that commit |
| Hosted qualification | hosted CI green on the publication source commit, all lanes |
| Registry-only consumer proof | an external crate with `=0.1.1` and no Git/path source builds and smokes green |
| Narrowing 1 recorded | the all-zeros refusal is a compatibility event naming `eggup-eggpack` |
| Narrowing 2 recorded | the document-bound refusal is a compatibility event in the same record |
| Both surfaces satisfied | the closure record **and** `architecture/manifest.md` carry both events |
| Schema unchanged | `schema_version: 1`, same field set, same wire shape, verified against the published `0.1.0` |
| No public removal | no `pub` item removed or re-signatured |
| Publication bounded | only `eggpack-manifest` published; no automatic publication workflow added |
| Roadmap corrected | the "no further implementation is implied" claim is retired |
| Registry truth reconciled | the registry's published-version statement matches crates.io |
| Downstream truth reconciled | the consumer notification and the consumer-side obligation are recorded |
| Closure traceability | the closure record carries source commit, package checksum, registry evidence, tag, commands, CI run, and unresolved findings |

## 10. Stop conditions

Stop, and do not publish, if any of these hold:

- the working tree is not clean at the publication source commit;
- `cargo package` or `cargo publish --dry-run` fails;
- hosted CI is not green on the exact publication source commit;
- `eggpack-manifest 0.1.1` already exists on crates.io;
- the crate source differs from the audited source delta in any way;
- the full local gate is not green;
- the crates.io credentials are absent or the publish is refused.

A publish attempt that fails after the version is created on the registry is
**not** retryable by republishing the same version. Stop, record the failure
honestly, and treat the version as occupied.

## 11. Security and maintenance review

- The change narrows what is accepted; it cannot cause a previously valid
  real-world document to be rejected, because the rejected values are the
  all-zeros sentinel and a document that exceeds the bound the crate's own
  reader already refused.
- No new network, credential, archive, publication, or privilege surface is
  introduced beyond the publication itself.
- The publication is a credentialed action performed once, explicitly, for one
  crate, from a tagged commit.
- The residual risk for the consumer is a document carrying the all-zeros
  placeholder. For that consumer the change turns a silent placeholder into a
  fail-closed error, which is the intended direction, and the consumer is named
  so the transition is announced rather than discovered.

## 12. Documentation / registry updates required at closure

- `plans/closure/release-manifest/004-status.md` — the closure record, with
  both compatibility events.
- `plans/registry.md` — a milestone row, an execution-graph node, and the
  handoff paragraph.
- `plans/subsystems/release-manifest-roadmap.md` — an M004 section and table
  row; retire the "no further implementation is implied" claim.
- `architecture/manifest.md` — already carries both events; re-verify the
  citations against the published tree and correct them if they moved.
- `AGENTS.md` — the current handoff, rewritten from the registry.
- `plans/adrs/ADR-0006-…md` — a dated note recording that the publication
  occurred, with a forward pointer to the closure record. The ADR's decision
  body is not rewritten.

## 13. Handoff after closure

The published `eggpack-manifest 0.1.1` becomes the current registry version.
`eggup-eggpack` is given the patch release and the narrowed acceptance set,
and the consumer-side bump is queued under a plan registered in the Eggup
repository. The release-manifest subsystem has no open work after this
milestone closes.
