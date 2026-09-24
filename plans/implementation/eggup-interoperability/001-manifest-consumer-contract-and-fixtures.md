# Eggup Interoperability Milestone 001 — Manifest Consumer Contract and Cross-Repository Fixtures

Status: closed

Closure record: `plans/closure/eggup-interoperability/001-status.md`

Eggpack implementation baseline: `0b1c3b9795280dea610c2fbe9d1533591d610b3f` (Manifest M001/M001a closed)

External Eggup baseline: `eggstack/eggup@2cab1f97ef30fa347c2030da321462459672c521`

Relevant Eggup producer-authority retirement: `bc25885bd41b86bfdf2f32d1e42856e00829cd7a` with closure `plans/closure/distribution-bootstrap/004-status.md`

Source roadmap: `plans/subsystems/eggup-interoperability-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#16-eggup-interoperability`
- `plans/001-terminology-and-domain-model.md#11-release-manifest`
- `plans/001-terminology-and-domain-model.md#16-receipts`
- `plans/002-long-term-roadmap.md#phase-10--eggup-interoperability`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: interface / compatibility / cross-repository infrastructure

## 1. Objective

Freeze and prove the Eggpack side of the ReleaseManifest v1 → Eggup consumer seam without changing Eggup.

M001 documents exact translation rules, adds cross-repository direct/bundle/archive fixtures, and—only where generically useful—adds small non-wire helper APIs to `eggpack-manifest` for exact target lookup and validated SHA-256 byte conversion.

The milestone must demonstrate that an Eggup adapter can consume stable manifest facts without importing `eggpack-core`, build/CI machinery, release-selection policy, or producer authority.

## 2. Readiness and dependencies

Hard dependencies are satisfied:

- Contract M001/M002 is closed.
- Manifest M001/M001a is closed and its target/artifact/install/digest semantics are stable enough for an initial consumer contract.
- Eggup M004 retired `eggup-dist`; current Eggup runtime workspace contains only consumer-side core/acquisition/service crates.

Eggup adapter implementation is NOT part of this milestone. Any code change in `eggstack/eggup` requires a separately registered Eggup plan after this Eggpack-side contract/fixture milestone closes.

## 3. Current external interface evidence

At the pinned Eggup baseline:

- `eggup-core::ProductId` and `ReleaseId` are opaque validated consumer identities;
- `MemberId` is an opaque member identity;
- `ArtifactMember` takes an acquired local source path plus consumer-relative destination;
- `ArtifactSet` is the coherent multi-member mutation unit;
- `IntegrityRequirement::Sha256([u8; 32])` carries expected digest evidence;
- `InstallPlan` requires a caller-owned installation root and does not infer ownership;
- `eggup-acquisition::AcquisitionRequest` accepts an exact caller-selected HTTP(S) URL;
- `FetchLimits` carries caller byte/time bounds;
- Eggup does not own producer release selection or origin policy.

These APIs are evidence for the mapping contract only. Eggpack must not depend on them at runtime.

## 4. Invariants

- `eggup-core` remains independent of Eggpack.
- `eggpack-manifest` remains independent of Eggup.
- The ReleaseManifest never selects a release, origin, mirror, installation root, ownership policy, service policy, or fallback.
- Consumer/application policy resolves and authorizes the release and origin before artifact acquisition.
- Target selection is exact/canonical. No nearest-target guessing.
- Manifest ProductId/ReleaseId map as opaque identities; Eggup does not reinterpret version ordering.
- Manifest SHA-256 maps to Eggup integrity requirement only; it does not assert authenticity.
- Manifest exact size is an acquisition/post-fetch check; it does not become trust.
- Manifest install names are target-local flat names. Consumer policy owns the installation root and any explicitly permitted destination prefix.
- Direct and bundle forms can map to Eggup ArtifactSet semantics without producer build types.
- Archive form must preserve archive acquisition identity and required member mappings, but this milestone MUST NOT claim that current Eggup provides generic archive extraction. Archive fixture state must explicitly represent “extraction required before InstallPlan”.
- Eggup install receipts remain Eggup-owned and are not serialized by Eggpack.
- No URL is added to ReleaseManifest v1.

## 5. Scope

### In scope

- normative Eggpack-side interoperability mapping document;
- direct, bundle, and archive manifest fixtures derived from current representative contracts;
- expected consumer projection fixtures;
- tests proving mapping facts are unambiguous and bounded;
- exact canonical target lookup helper in `eggpack-manifest`, if useful;
- validated SHA-256 hex→`[u8; 32]` helper(s), if useful;
- cross-version/schema-negative fixtures;
- explicit mapping to current Eggup domain concepts without linking Eggup crates;
- provenance metadata recording the pinned Eggup baseline/API evidence.

### Out of scope

- modifying Eggup;
- adding `eggup-eggpack` crate;
- performing acquisition;
- constructing live Eggup InstallPlan values in Eggpack;
- installing/updating files;
- ownership/rollback/service behavior;
- archive extraction;
- release/version selection;
- origin URL storage in Manifest;
- authenticity/signature policy;
- receipt generation.

## 6. Required interface contract

Create a durable document such as `architecture/eggup-manifest-consumer-v1.md` defining the following translation.

### A. Release identity

```text
ReleaseManifest.product_id -> Eggup ProductId
ReleaseManifest.release_id -> Eggup ReleaseId
```

Both are opaque. Conversion may fail only if the consumer type has stricter generic input bounds; it must never reorder or reinterpret versions.

`source_revision` is producer evidence and does not become Eggup release-selection policy.

### B. Target resolution

The caller supplies/derives one exact canonical platform target according to application policy/platform detection. The adapter selects exactly one matching `TargetRecord`.

No alias fallback or fuzzy target matching belongs in the manifest adapter unless a later Eggup plan explicitly consumes DistributionContract as an optional separate input. M001 uses manifest canonical triples only.

If zero or multiple matches occur, fail.

### C. Direct form

For one direct target:

```text
artifact.name   -> caller-origin + exact asset filename
artifact.size   -> acquisition maximum + exact post-fetch size check
artifact.sha256 -> IntegrityRequirement::Sha256
install         -> MemberId default + relative destination basename
```

The origin/base URL and installation root are caller-owned.

A future adapter may use the flat install name as the default MemberId because M001a guarantees target-local uniqueness. If a consumer needs a distinct member-id policy, that remains adapter/application policy and must not alter Manifest v1.

### D. Bundle form

Each bundle entry becomes one independently acquired artifact member, but the entire set maps to one Eggup `ArtifactSet`/transaction generation.

The adapter must preserve each artifact↔install relationship exactly and reject partial/mixed release input.

Size/digest handling is per entry. Install names remain target-local unique.

### E. Archive form

The archive artifact maps to one acquisition unit with exact size/digest.

Each manifest member preserves:

- archive `source` path;
- target-local install name;
- member size/digest.

Current Eggup core accepts local member files, not a generic archive extraction policy. Therefore the interface fixture MUST distinguish:

```text
archive acquisition evidence
        |
        v
consumer-owned/qualified extraction seam [NOT DEFINED HERE]
        |
        v
member local files
        |
        v
Eggup ArtifactSet / InstallPlan
```

M001 must not fabricate an InstallPlan directly from the archive file.

### F. Acquisition size semantics

Manifest `size` is exact. Eggup acquisition currently exposes a maximum byte limit rather than an exact-size contract.

A future adapter should:

1. set `FetchLimits.max_artifact_bytes = Some(manifest_size)` where representable;
2. fetch to a transaction-owned local path;
3. require returned/written byte count or local metadata to equal manifest size exactly;
4. only then pass the file onward for digest verification/staging.

M001 fixtures must encode this distinction.

### G. Destination/ownership semantics

Manifest `install` is a flat install identity, not an absolute destination and not proof of ownership.

The future adapter receives installation root/destination-prefix/ownership policy from the application/Eggup caller. Eggpack fixtures must never imply that a manifest grants replacement authority.

## 7. Eggpack production/test changes

### A. Manifest consumer helpers

Only if justified by the fixture implementation, add small non-serialized helpers such as:

- exact `ReleaseManifest::target(canonical_triple)`;
- `ArtifactRecord::sha256_bytes()`;
- `ByteEvidence::sha256_bytes()`.

These helpers MUST NOT alter JSON wire shape and MUST remain generic, not named for Eggup.

Do not add acquisition/install-plan projection types to `eggpack-manifest` unless review proves they are consumer-neutral enough to belong to the manifest domain. Prefer fixtures/documentation over premature abstraction.

### B. Interoperability fixture set

Add stable fixtures covering at minimum:

- direct multi-target Eggsact-style manifest;
- CodeGG-style bundle manifest;
- Egress-style archive manifest;
- expected consumer projection for direct;
- expected consumer projection for bundle;
- expected archive acquisition/member mapping with explicit extraction-required state;
- wrong target;
- malformed/unknown manifest schema;
- digest/size corruption examples.

Fixtures should be deterministic JSON and small enough to copy/pin into a later Eggup adapter plan.

### C. Fixture/projection schema

The expected projection fixture may use a test-only/documentation schema. It must distinguish:

- consumer-selected target;
- acquisition units;
- exact expected size/digest;
- member/install identity;
- transaction grouping;
- archive extraction requirement.

Do not present this test schema as a new production wire format.

## 8. Ordered work packages

1. Record pinned Eggup API/baseline evidence in the interoperability document.
2. Write exact ReleaseManifest→Eggup concept mapping and authority exclusions.
3. Add direct/bundle/archive manifest fixtures.
4. Add expected consumer projection fixtures.
5. Add generic manifest helper APIs only where fixture code proves value.
6. Add negative target/schema/digest/size projection tests.
7. Verify no Eggup runtime/dev dependency enters Eggpack.
8. Qualify Manifest package/MSRV/docs/hosted CI.
9. Write closure and determine whether an Eggup-side M002 adapter plan is now ready to author/register.

## 9. Failure, restart, and contention semantics

M001 is pure parsing/projection/fixture work.

Failures are typed/fixture-test failures with no network, filesystem mutation, install, or retry behavior.

A manifest mismatch or unsupported archive pathway fails closed rather than producing a partially valid consumer projection.

## 10. Compatibility and migration

No ReleaseManifest wire-format change is expected.

The mapping contract is additive documentation/test evidence. Any helper API is additive and non-serialized.

Eggup remains fully usable without Eggpack. A future `eggup-eggpack` adapter should be optional and must not force `eggup-core` to depend on producer/build crates.

If the pinned Eggup baseline changes materially before its adapter plan is authored, re-review the mapping against current Eggup APIs.

## 11. Required tests

At minimum:

- exact direct target projection;
- exact bundle transaction grouping;
- archive projection retains archive/member identities and marks extraction required;
- wrong canonical target fails;
- no target fallback;
- product/release opaque mapping retained;
- install name does not become absolute destination/root policy;
- SHA-256 conversion round-trip if helper added;
- invalid digest cannot produce digest bytes;
- size remains exact in projection;
- bundle partial/mixed projection rejected;
- Manifest M001a repeated install names across different targets remain valid;
- unknown schema/field behavior remains fail-closed;
- fixture JSON deterministic;
- no Eggup dependency in `cargo tree -p eggpack-manifest`;
- unchanged manifest/contract suites.

## 12. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-manifest --locked
cargo package -p eggpack-manifest --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Also re-read the pinned Eggup baseline API files and record the exact external files/commit reviewed. Do not run or modify Eggup from this plan.

## 13. Documentation updates

Add/update:

- `architecture/eggup-manifest-consumer-v1.md` or equivalent durable interface document;
- manifest crate README consumer section;
- Eggup interoperability roadmap;
- registry;
- closure `plans/closure/eggup-interoperability/001-status.md`.

The closure must explicitly say that no Eggup repository change occurred.

## 14. Acceptance criteria

M001 closes when:

- direct/bundle/archive ReleaseManifest→consumer mappings are explicit and fixture-backed;
- current Eggup ProductId/ReleaseId/ArtifactSet/Integrity/acquisition concepts can consume the direct/bundle facts without producer build types;
- archive limitations are explicit rather than papered over;
- release selection, origin, install root, ownership, rollback, services, and receipts remain consumer/application-owned;
- no Eggup dependency is introduced into Eggpack;
- Manifest v1 wire shape remains unchanged;
- package/MSRV/docs/hosted CI pass;
- no unresolved medium-or-higher interface ambiguity remains.

Closure should state whether a concrete Eggup-side adapter plan is now safe to author in `eggstack/eggup`.

## 15. Stop conditions

Stop for ADR/replanning if:

- Eggup consumption requires `eggpack-core` or build/CI crates;
- a URL/origin must be added to Manifest v1;
- manifest data would decide update/release authorization;
- archive handling requires Eggpack to define Eggup extraction/ownership semantics;
- an Eggup receipt must become an Eggpack schema;
- current Eggup APIs cannot preserve direct/bundle integrity/transaction identity without a material ownership-boundary change.

## 16. Closure evidence required

Record:

- Eggpack implementation SHA;
- pinned Eggup baseline and exact API files reviewed;
- direct/bundle/archive mapping matrix;
- fixture inventory/blob identities;
- helper API additions, if any;
- no-wire-change evidence;
- no-dependency-leak evidence;
- package/MSRV/docs/hosted CI;
- unresolved findings;
- explicit decision on authoring the subsequent Eggup-side adapter plan.

## 17. Handoff notes

This is Eggpack-side interface work only. Do not modify `eggstack/eggup` under this plan.

If M001 closes cleanly, the next cross-repository step is to author/register an Eggup plan for an optional lightweight `eggup-eggpack` adapter against the exact closed fixture/interface baseline. That later adapter should initially prioritize direct/bundle releases; archive installation remains subject to Eggup's separate extraction/update transaction boundary.
