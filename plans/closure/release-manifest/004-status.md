# Release Manifest Milestone 004 — `eggpack-manifest 0.1.1` Publication and Compatibility Event Record

Status: closed

Source plan: `plans/implementation/release-manifest/004-eggpack-manifest-0.1.1-publication-and-compatibility-event.md`

Primary roadmap: `plans/subsystems/release-manifest-roadmap.md`

Governing decision: `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md`
(Status: **accepted**, Option A)

Reviewed baseline: `d4d5e4399255b8b5a2998e41f084896c45f5dd44`

Publication source commit: `4ce4fb850ffa0e141f1a807b60c150173507a92b`

## Published identity

| Property | Value |
|---|---|
| Crate | `eggpack-manifest` |
| Version | `0.1.1` |
| Checksum | `352c3e327159615b763298e911e239f1decc4a2c9e28c28be8ddd28e50515e37` |
| Yanked | no |
| Published | 2026-10-26T03:30:26Z |
| Publication source commit | `4ce4fb850ffa0e141f1a807b60c150173507a92b` |
| Tag | `eggpack-manifest-v0.1.1` (annotated, pushed to `origin`) |
| Hosted CI on the publication source commit | run `40263953007`, green on all four lanes |
| Predecessor | `0.1.0`, checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`, still published and not yanked |

## Executive finding

ADR-0006 §5.2 and §9 obliged the published crate to take a `0.1.1` patch
release carrying the narrowed digest acceptance set, and §5.4 required that
narrowing to be recorded as a compatibility event in a **release-manifest**
closure record. Neither obligation was tracked. The crate had carried
`0.1.1` in-tree since `9cf0bf8` with its sources unchanged, but crates.io
exposed only `0.1.0`, no plan or closure record owned the publication, and
the release-manifest roadmap asserted that no further Release Manifest
implementation was implied.

This milestone discharged both obligations, and it found a second gap while
doing so: the `0.1.1` package carries **two** narrowings of the published
crate's public acceptance set, not one.

The second narrowing came from commit `3ef806f`, not from ADR-0006.
`ReleaseManifest::validate` now ends with `check_document_bound`, which
measures the canonical encoding and refuses it past `MAX_DOCUMENT_BYTES`.
Under `0.1.0` a manifest satisfying every *structural* bound but encoding
past 1 MiB was accepted by `validate` and serialized by `to_json` — so
`0.1.0` could emit a document its own `from_json` would refuse to read back.
Under `0.1.1` the same manifest is refused by both, with `TooLarge`. That
narrowing had been recorded only as an implementation note in
`architecture/manifest.md`, so it would have shipped unannounced. Both are
now recorded as compatibility events, on both surfaces ADR-0006 §5.4 names.

## Compatibility event 1 — the ADR-0006 digest narrowing

**What changed.** The one SHA-256 hex acceptance rule in the workspace is
`validate_sha256_hex` in `eggpack-manifest`. It requires exactly 64 lowercase
hex characters **and** refuses the all-zeros digest. `ArtifactRecord::validate`,
`ByteEvidence::validate`, and `decode_sha256` delegate to it, and
`eggpack-core` and `eggpack-github` call the same rule. The refusal is typed:
`Sha256Error::AllZeros`, whose `reason()` names the actual cause.

**Who is affected.** `eggup-eggpack`, the only known consumer of the
published crate. It consumes registry `eggpack-manifest =0.1.0`.

**What a consumer observes.** A document whose digest is the all-zeros
placeholder is now refused rather than accepted. For that consumer the
change turns a silent placeholder into a fail-closed error, which is the
intended direction.

**Migration.** None is required for a consumer that only reads real producer
output: no real producer emits an all-zeros digest, and the bootstrap
fixtures that used it as a stand-in were replaced by the ADR-0006 corrective
(M003a). A consumer that *constructed* fixtures with the all-zeros sentinel
must change them.

**Schema impact.** None. `ReleaseManifest` stays at `schema_version: 1`, no
field is added or removed, and the wire shape is unchanged.

## Compatibility event 2 — the document-bound narrowing

**What changed.** `ReleaseManifest::validate` now ends with
`check_document_bound`, which measures the canonical encoding and returns
`ManifestError::TooLarge` past `MAX_DOCUMENT_BYTES`. `to_json` validates, so
it cannot emit such a document either.

**Who is affected.** The same consumer, `eggup-eggpack`.

**What a consumer observes.** A consumer that only *reads* real producer
output is unaffected: no document a real producer emits approaches 1 MiB, and
`from_json` already refused such documents in `0.1.0`, so the refused input
was never one that could have been read back either way. A consumer that
*constructs* a manifest programmatically and serializes it now receives
`TooLarge` where it previously received a document its own reader would have
rejected.

**Measured, not assumed.** Against identical fixtures on both versions (256
targets × 256 members): the canonical length is 25,689,453 bytes. `0.1.0`
`validate()` returns `Ok(())` and `to_json()` returns `Ok(25,689,453)`;
`0.1.1` `validate()` returns `Err("release manifest exceeds the 1 MiB limit")`
and `to_json()` returns `Err(TooLarge)`. The rule is not over-tightened: a
canonical length of 999,768 bytes still validates and serializes under
`0.1.1`.

**Schema impact.** None. `TooLarge` already existed and was already
reachable through `from_json`. No field, no schema version, and no error
variant changed.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| The version is published exactly once | crates.io exposes `0.1.1`, not yanked; `num_versions: 2`; the pre-publish availability gate confirmed `0.1.1` was absent |
| The publication source is recorded | `4ce4fb850ffa0e141f1a807b60c150173507a92b`, tagged `eggpack-manifest-v0.1.1` |
| The package checksum is recorded | `352c3e32…15e37`, computed locally at the publication source commit and confirmed by the registry and by an independent registry-only resolution |
| Hosted qualification | run `40263953007` green on `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)` |
| Registry-only consumer proof | an external crate with `eggpack-manifest = "=0.1.1"` and no Git or path source resolved from the registry with the exact checksum, built, and ran the smoke test green |
| Narrowing 1 is recorded | the ADR-0006 digest narrowing, above, naming `eggup-eggpack` |
| Narrowing 2 is recorded | the document-bound narrowing, above, naming `eggup-eggpack` |
| Both surfaces are satisfied | this record **and** `architecture/manifest.md` carry both events |
| The schema is unchanged | `schema_version: 1`, same field set, same wire shape, verified against the published `0.1.0` |
| No public removal | no `pub` item removed or re-signatured; the only public additions are `Sha256Error` and `validate_sha256_hex`, both additive |
| The publication is bounded | only `eggpack-manifest` was published; no automated publication workflow was added; the `ci` family is unchanged |
| The roadmap is corrected | the "no further implementation is implied" claim is retired and replaced with the published identity |
| The downstream truth is reconciled | the consumer notification and the consumer-side obligation are recorded below |
| Closure traceability | source commit, package checksum, registry evidence, tag, commands, CI run, and unresolved findings are all recorded here |

## Exact verification executed and results

### Pre-publish qualification

```text
$ cargo package -p eggpack-manifest --locked
    Packaged 6 files, 67.7KiB (13.2KiB compressed)
$ cargo publish -p eggpack-manifest --locked --dry-run
    Uploading eggpack-manifest v0.1.1
warning: aborting upload due to dry run
```

Both from a clean tree at the publication source commit. No `--allow-dirty`.

### Availability gate, immediately before the publish

```text
$ curl -s -H "User-Agent: eggpack-release-check/1.0" \
    https://crates.io/api/v1/crates/eggpack-manifest
0.1.1 present: False
versions: ['0.1.0']
```

### Publication

```text
$ cargo publish -p eggpack-manifest --locked
    Uploading eggpack-manifest v0.1.1 to registry `crates-io`
    Uploaded eggpack-manifest v0.1.1 to registry `crates-io`
    Published eggpack-manifest v0.1.1 at registry `crates-io`
```

One crate, one version, one explicit invocation, from a clean tree at the
publication source commit.

### Post-publication registry proof

```text
num_versions: 2 | max_version: 0.1.1 | newest: 0.1.1
  0.1.1    yanked=False  checksum=352c3e327159615b763298e911e239f1decc4a2c9e28c28be8ddd28e50515e37
  0.1.0    yanked=False  checksum=2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629
```

`0.1.0` is untouched: still published, not yanked, same checksum.

### Registry-only consumer proof

An external crate outside the workspace, with `eggpack-manifest = "=0.1.1"`
and no Git or path source. Its lock resolved the dependency from the
registry with the exact published checksum:

```text
name = "eggpack-manifest"
version = "0.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "352c3e327159615b763298e911e239f1decc4a2c9e28c28be8ddd28e50515e37"
```

and the smoke test, run against the published artifact, printed:

```text
round-trip: ok (343 bytes)
narrowing 1a (all-zeros digest, serialize): refused — invalid release manifest: sha256 must not be the all-zeros digest
narrowing 1b (all-zeros digest, parse): refused — invalid release manifest: sha256 must not be the all-zeros digest
narrowing 2 (document bound): refused — release manifest exceeds the 1 MiB limit
in-bound document: accepted
registry-only proof: PASS
```

This is the end-to-end evidence for both compatibility events: the narrowings
are observed **through the published artifact**, not through the workspace
source.

### Local gate

`./scripts/check-local.sh` at the publication source commit — exit 0: `fmt
--check`, `check`, `clippy -D warnings`, the per-crate test lanes, the
workspace test lane, `doc`, `tree`, seven `cargo package` invocations, and the
MSRV check and test lanes. **942 passed, 0 failed.**

### Hosted CI on the publication source commit

Run `40263953007` on `4ce4fb850ffa0e141f1a807b60c150173507a92b` — green on
all four lanes.

## Invariant review

| Invariant | Result |
|---|---|
| Only `eggpack-manifest` is published | held — one explicit `cargo publish -p eggpack-manifest --locked`; no other crate was published |
| No workspace-wide version policy | held — the workspace version stays `0.1.0` for the other six crates |
| Publication is explicit and single-crate | held — no `--allow-dirty`, no bare `cargo publish`, no automated workflow |
| The published source representation is not edited afterwards | held — no source change followed the publish |
| Every narrowing the package carries is recorded | held — both events are recorded here and in `architecture/manifest.md` |
| The affected consumer is named | held — `eggup-eggpack`, in both events |
| `ReleaseManifest` stays at `schema_version: 1` | held — verified against the published `0.1.0` |
| Checksums remain integrity facts | held — no signature, attestation, or provenance claim is made |
| The Eggup repository is not modified | held — no file in `eggstack/eggup` was changed |
| Prior closure records are not rewritten | held — M003 and M003a are untouched; the deferred README item is discharged by this record rather than by editing them |
| Determinism and stable serialization | held — the published package is byte-identical to the workspace source at the publication source commit |

## Failure and recovery review

- A publish attempt that fails *after* the version is created on the registry
  is not retryable by republishing the same version; the version is occupied
  and would have to be treated as such. This attempt did not reach that
  state: the upload and the registry's acceptance both succeeded on the
  first attempt.
- The availability gate immediately before the publish confirmed `0.1.1` was
  absent, so the publish could not have collided with an existing version.
- The registry-only consumer proof resolves the *published* artifact, so a
  packaging error that the local `cargo package` would not have caught would
  still have been caught here.

## Compatibility and migration review

Covered in full above under the two compatibility events. In summary: the
schema is unchanged, no public item was removed, and both narrowings are
announced with the consumer named and the migration consequence stated.

## Security review

- The change narrows what is accepted; it cannot cause a previously valid
  real-world document to be rejected, because the refused values are the
  all-zeros placeholder and a document that exceeds the bound the crate's own
  reader already refused.
- No new network, credential, archive, publication, or privilege surface was
  introduced beyond the publication itself, which is a single credentialed
  action for one crate from a tagged commit.
- The residual risk for the consumer is a document carrying the all-zeros
  placeholder. For that consumer the change turns a silent placeholder into a
  fail-closed error, which is the intended direction, and the consumer is
  named so the transition is announced rather than discovered.

## Documentation and operations evidence

- `architecture/manifest.md` — the publication status, the two compatibility
  events, and the corrected citations.
- `architecture/overview.md` — the crate's published state.
- `plans/adrs/ADR-0006-release-evidence-digest-acceptance-set.md` — a dated
  note recording that the publication obligation was discharged and that the
  consumer obligation was not. The ADR's decision body is not rewritten.
- `.skills/contract-and-manifest-surface/SKILL.md` — the published version
  and the two narrowings.
- `plans/registry.md`, `plans/subsystems/release-manifest-roadmap.md`, and
  `AGENTS.md` — M004 registered as closed, with the published identity.

## Unresolved findings

- **The consumer-side obligation is not discharged, and this milestone does
  not discharge it.** `eggup-eggpack` pins `eggpack-manifest = "=0.1.0"` — an
  exact requirement — so publishing `0.1.1` does not move it. The consumer
  must bump the pin, update its asserted pin constant, and re-qualify, under
  a plan registered in the Eggup repository. ADR-0006 explicitly does not
  authorize a change inside Eggup, and this closure does **not** claim the
  consumer was migrated. The narrowing breaks nothing in the consumer today:
  its only all-zeros construct is an assertion that the digest **must be
  refused** (`crates/eggup-core/tests/current_executable.rs:597`).
- **The crate's published README now points at an absolute repository URL**
  rather than the repository-only relative path that M003 recorded as
  dangling on crates.io. That deferred item is discharged by this
  publication: `architecture/eggup-manifest-consumer-v1.md` exists on
  `main`, so the link resolves for a crates.io reader. M003 and M003a's
  records are left as written.
- **Informational — the emulated qualification path is still not covered end
  to end**, for the same environmental reason M009a recorded.
- **Informational — the publication branch was not merged.** The branch
  `feat/producer-convergence-correctives` is pushed and green, and the
  crate-scoped tag `eggpack-manifest-v0.1.1` points at the publication
  source commit. Merging the branch to `main` is a separate human action
  and is not part of this milestone.

## Roadmap disposition

Release Manifest M004 is **closed**. ADR-0006's §5.2 and §9 publication
obligation is discharged, and its §5.4 compatibility record exists on both
required surfaces. ADR-0006's §5.5 consumer obligation remains open and is
owned by the Eggup repository, not by Eggpack.

The release-manifest subsystem has no open Eggpack work after this
milestone. The next Eggpack line is whatever the registry lists as `ready`;
if that table is empty, the next action is plan authoring, not
implementation.

## Registry updates

- `plans/registry.md` — M004 row in the milestone index (closed, with the
  published identity), the active-roadmap row, the execution-graph node, and
  the handoff paragraph.
- `plans/subsystems/release-manifest-roadmap.md` — M004 section and table
  row, and the retired "no further implementation is implied" claim.
- `AGENTS.md` — the current handoff.
