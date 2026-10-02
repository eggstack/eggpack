# Release Manifest Milestone 003 — `eggpack-manifest 0.1.0` Consumer Compatibility Baseline and Registry Publication

Status: ready

Repository authoring baseline: `ed1bef885eb3e396a945165d680ed3073decf2a5`

Primary roadmap: `plans/subsystems/release-manifest-roadmap.md`

Cross-repo dependencies / evidence:

- Eggup interoperability M003 closure: `eggstack/eggup@538e3e5605cf3c315c10e5be200c8896de7379b1`;
- qualified real consumer: `eggstack/eggsact@65c916b`;
- Eggup consumer-qualified Eggpack pin: `eggstack/eggpack@678bbf04f5a02827003a1d9ab83ba4f0e6360e41`;
- Eggup M004a package/API promotion preflight closure: `eggstack/eggup@e937c3f8b77c94c06e82c48c0a0da5f9b500a984`;
- Eggup M004a closure record: `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/004a-status.md`.

Primary class: compatibility / packaging / manual publication prerequisite

## 1. Why this milestone exists

Release Manifest M003 was originally gated on a real consumer. That gate is now
satisfied.

Eggup interoperability M003 closed after Eggsact adopted the bounded
`eggpack-manifest` parser/projection path in its real updater. Hosted consumer CI
and release-drift qualification are green. Eggup then ran M004a, a package/API
promotion readiness preflight, and proved that its next package-promotion step
cannot begin from registry-only dependencies because `eggpack-manifest 0.1.0`
does not yet exist on crates.io.

The required producer-side action is deliberately narrow. Eggup does not need a
new manifest schema, a producer runtime API, a new transport, or a workspace-wide
Eggpack release. It needs the already-qualified leaf schema crate to become
registry-resolvable at the exact version its adapter has already qualified:

`eggpack-manifest = "=0.1.0"`.

At this plan's authoring baseline, review of
`678bbf04f5a02827003a1d9ab83ba4f0e6360e41..ed1bef885eb3e396a945165d680ed3073decf2a5`
shows no path delta under `crates/eggpack-manifest/`. The consumer-qualified
schema/parser source is therefore still the current source. The workspace
version is `0.1.0`; the crate has normal crates.io metadata, no
`publish = false`, and only registry dependencies on `serde` and
`serde_json`.

This milestone converts that proven source identity into a clean package,
performs first-publication qualification, publishes only
`eggpack-manifest 0.1.0` through the existing manual publication boundary, and
records registry-only evidence that Eggup can consume next.

## 2. Why ready

All implementation dependencies are satisfied:

1. ReleaseManifest v1 M001/M001a are closed.
2. Manifest M002 final-artifact construction is closed.
3. Eggpack/Eggup interface fixtures and the projection corrective are closed.
4. Eggup's bounded manifest adapter path is implemented and qualified.
5. Eggup M003 real-consumer adoption is closed on Eggsact.
6. The exact consumer-qualified Git pin uses `eggpack-manifest 0.1.0`.
7. Eggup M004a proved that the missing registry package is a real downstream
   blocker rather than a speculative publication request.
8. No source delta under `crates/eggpack-manifest/` was found between the
   qualified pin and this authoring baseline.

The crates.io name/version must still be rechecked immediately before the real
publish. The 2026-10-01 Eggup M004a evidence observed the crate name as absent;
that observation is not a permanent reservation.

## 3. Objective

Publish exactly one package, `eggpack-manifest 0.1.0`, from a reviewed,
qualified Eggpack commit, then prove that a clean registry-only consumer can
resolve and compile the exact version without Git/path fallback.

The milestone closes only when all of the following are true:

- the packaged crate contents and public API correspond to the reviewed source;
- the consumer-qualified schema-v1 behavior remains intact;
- stable and MSRV qualification pass;
- `cargo package` and `cargo publish --dry-run` pass from a clean tree;
- the package name is still available immediately before first publication;
- a maintainer performs the explicit manual publish;
- crates.io resolves exactly `eggpack-manifest = "=0.1.0"`;
- a temporary registry-only smoke consumer builds/tests against that exact
  version;
- the closure record captures package identity, publication evidence, source
  commit, and downstream handoff;
- Eggup's active planning is updated to show that its Eggpack-owned prerequisite
  is satisfied, without claiming Eggup's own publication chain is complete.

## 4. Authoritative package and compatibility identity

The implementation must preserve these separate identities:

| Identity | Required value / rule |
|---|---|
| Cargo package | `eggpack-manifest` |
| Cargo package version | `0.1.0` |
| Manifest wire schema | `schema_version = 1` |
| Rust edition | workspace 2021 |
| MSRV | 1.89 |
| License | MIT |
| Source repository | `https://github.com/eggstack/eggpack` |
| Consumer-qualified source baseline | `678bbf04f5a02827003a1d9ab83ba4f0e6360e41` |
| Publication source | final reviewed implementation/closure candidate commit; must be recorded exactly |
| Runtime dependency boundary | `serde` + `serde_json` only, subject to transitive registry dependencies |
| I/O/trust boundary | parser/serializer only; no filesystem/network/trust authority |

Cargo package version and ReleaseManifest wire schema version are independent.
Publishing Cargo `0.1.0` does not create a new manifest schema.

If any production source or public API under `crates/eggpack-manifest/` has
changed since the consumer-qualified pin by execution time, stop the
"no-semantic-delta" path and perform an explicit compatibility review before
publication. Do not assume that a later `0.1.x` source is interchangeable with
the exact source Eggup qualified.

## 5. Invariants

- Publish only `eggpack-manifest`; do not implicitly publish
  `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-ci`,
  `eggpack-github`, or `eggpack-cli`.
- Do not change schema-v1 wire semantics merely to make packaging succeed.
- Do not weaken unknown-field, unsupported-version, size, count, name/path, or
  digest validation.
- Do not add network, filesystem, release-selection, installation, replacement,
  authenticity, or credential authority to the manifest crate.
- Keep `eggpack-manifest` a lightweight consumer leaf crate.
- Do not add Git/path runtime dependencies to the package.
- Do not vendor Eggup or consumer code into Eggpack.
- Do not publish automatically from CI as part of this milestone.
- Never commit a crates.io token or other publication credential.
- Treat first publication as an explicit maintainer action after dry-run
  qualification.
- Treat a successful crates.io upload as immutable for this version. Never plan
  to overwrite `0.1.0`; any post-publication defect requires normal versioning
  and downstream requalification.
- Do not claim that publishing this crate completes Eggup M004. It removes only
  the Eggpack-owned prerequisite. Eggup still owns its proven
  `eggup-acquisition 0.1.2 -> eggup-eggfetch 0.1.2 -> eggup-eggpack 0.1.2`
  publication sequence and its separately authorized M004 implementation.
- Do not make formal JSON Schema generation a prerequisite for this publication.
  Existing strict schema-v1 parsing, fixtures, projection tests, and real
  consumer evidence are the compatibility baseline. A formal schema export may
  remain later polish if useful.

## 6. In scope

### 6.1 Reconcile M003 planning truth

Update Eggpack planning so it records two distinct facts:

- Eggup interoperability M003 is already closed in Eggup/Eggsact and is not a
  ready-to-resume consumer task anymore.
- Release Manifest M003's former "real consumer" blocker is satisfied and this
  publication plan is dependency-ready.

The registry, release-manifest roadmap, and Eggup-interoperability roadmap must
agree on those facts.

### 6.2 Freeze and audit the crate source

Before any packaging or publication action:

1. re-fetch current `main`;
2. record the exact candidate commit;
3. run
   `git diff 678bbf04f5a02827003a1d9ab83ba4f0e6360e41..<candidate> -- crates/eggpack-manifest`;
4. if the diff is empty, record that the consumer-qualified source is still
   exact;
5. if non-empty, classify every source/API/test/metadata change and re-run the
   compatibility review before proceeding;
6. inspect `Cargo.toml`, README, public rustdoc surface, packaged file list, and
   dependency tree;
7. confirm package version remains exactly `0.1.0` and no `publish = false`
   or non-registry runtime dependency was introduced.

Metadata-only improvements are allowed only if they do not alter the qualified
runtime/API/wire behavior and the package identity remains `0.1.0`.

### 6.3 Compatibility baseline

Treat the existing schema-v1 and cross-repo evidence as the compatibility
harness for publication:

- all in-crate schema-v1 round-trip/determinism/negative tests;
- corrected Eggup direct/bundle/archive projection fixtures;
- target mismatch and unknown-schema rejection;
- exact size/digest propagation;
- direct/bundle/archive relationship preservation;
- consumer-qualified Eggsact M003 evidence.

If implementation adds a small package-specific regression, keep it focused on
published-surface compatibility. Do not expand this milestone into schema-v2,
signing, or a general schema-generation project.

### 6.4 Package qualification

Run from a clean tree and record exact outputs:

```text
cargo fmt --all -- --check
cargo check -p eggpack-manifest --all-targets --locked
cargo clippy -p eggpack-manifest --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo doc -p eggpack-manifest --no-deps --locked
cargo tree -p eggpack-manifest --locked
cargo +1.89.0 check -p eggpack-manifest --all-targets --locked
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked
cargo package -p eggpack-manifest --locked --list
cargo package -p eggpack-manifest --locked
cargo publish -p eggpack-manifest --locked --dry-run
git diff --check
git status --short
```

Requirements:

- no `--allow-dirty` for the final package/publish-dry-run evidence;
- inspect the packaged file list for accidental repository/planning/secrets
  inclusion;
- confirm the packaged crate builds from the generated package artifact, not
  only from workspace paths;
- record the generated `.crate` SHA-256 (or equivalent package checksum
  evidence) in the closure record;
- hosted CI on the exact candidate commit must be green before the real publish.

A full workspace regression run is strongly preferred because the crate is a
workspace member and is consumed by Eggpack itself:

```text
cargo test --workspace --all-targets --all-features --locked
```

If the full workspace has an unrelated known failure, stop and classify it
rather than silently narrowing evidence.

### 6.5 First-publication availability gate

Immediately before the actual publish:

- query crates.io for the exact package name;
- confirm `eggpack-manifest` still has no conflicting owner/package;
- confirm `0.1.0` is not already present;
- confirm the maintainer account/token is the intended Eggstack publication
  authority;
- if the name has been claimed by an unrelated party, stop. Do not silently
  rename the crate or change Eggup's qualified dependency identity inside this
  milestone.

The publication credential remains local/secret and must not appear in Git,
logs copied into closure records, or CI configuration.

### 6.6 Manual publication

After all prior gates pass, a maintainer may run:

```text
cargo publish -p eggpack-manifest --locked
```

This is the only irreversible external action in the milestone.

Do not publish any other workspace package in the same command/session merely
because it shares workspace version `0.1.0`.

Use a crate-specific source tag after successful publication if the repository
does not already have a stronger package-tag convention. Preferred default:

`eggpack-manifest-v0.1.0`

Do not use a generic workspace `v0.1.0` tag if that would falsely imply the
whole Eggpack workspace was published. If an established Eggpack tag convention
exists by execution time, follow it and record the exact tag instead.

### 6.7 Post-publication registry proof

After crates.io reports the version:

1. verify exact version presence and non-yanked state;
2. verify package metadata points to the expected repository/docs/readme;
3. create a temporary project outside the repository with only a registry
   dependency:
   `eggpack-manifest = "=0.1.0"`;
4. run `cargo generate-lockfile`, `cargo check`, and a minimal parse/round-trip
   smoke using schema-v1 data;
5. inspect `cargo tree` and prove there is no Git/path Eggpack source;
6. remove the temporary project;
7. record the registry/version/source/package checksum evidence in the closure
   record.

A docs.rs build is useful follow-up evidence but is not required to unblock
Eggup if crates.io resolution and the registry-only compile/smoke are already
green.

### 6.8 Downstream handoff

After publication is verified:

- mark Release Manifest M003 closed with a closure record;
- update Eggpack registry/roadmap status;
- update Eggup active planning to show the external
  `eggpack-manifest 0.1.0` prerequisite is satisfied;
- leave Eggup's own publication sequence under Eggup ownership;
- do not author or execute Eggup M004 from Eggpack.

Eggup's proven next dependency sequence is:

```text
eggpack-manifest 0.1.0        [Eggpack; this milestone]
        |
        v
eggup-acquisition 0.1.2       [Eggup]
        |
        v
eggup-eggfetch 0.1.2          [Eggup]
        |
        v
eggup-eggpack 0.1.2           [Eggup]
        |
        v
Eggup M004 registry promotion / consumer migration
```

## 7. Out of scope

- publishing any other Eggpack crate;
- publishing any Eggup crate;
- editing Eggsact to use crates.io;
- authoring/implementing Eggup M004;
- new release-selection or install policy;
- schema v2;
- authenticity/signature design;
- canonical signing JSON;
- changing digest algorithms;
- adding I/O to `eggpack-manifest`;
- automating crates.io publication in GitHub Actions;
- generating a formal JSON Schema unless a concrete compatibility defect makes
  it necessary for this publication;
- workspace-wide versioning/release policy beyond what is required to publish
  this single leaf crate.

## 8. Required work packages

### WP1 — Planning and source-identity reconciliation

- reconcile Eggpack Eggup-interoperability M003 to downstream-closed truth;
- move Release Manifest M003 from planned/blocked to ready;
- register this plan;
- record the exact candidate commit and compare the manifest subtree with the
  consumer-qualified `678bbf04` pin.

Exit: planning surfaces agree and no unreviewed manifest source delta exists.

### WP2 — Package/API/contents audit

- inspect package metadata and README/rustdoc;
- inspect public API and schema-v1 constants;
- inspect dependency tree;
- inspect `cargo package --list`;
- classify any change since the qualified pin.

Exit: package identity is exactly `eggpack-manifest 0.1.0`, contents are
intentional, and runtime dependencies remain registry-only/leaf-sized.

### WP3 — Local + hosted publication qualification

- run stable/MSRV crate checks/tests/docs;
- run package and publish dry-run from a clean tree;
- run full workspace regression;
- require hosted CI green on the exact candidate.

Exit: clean package/dry-run and hosted qualification are recorded.

### WP4 — First-publication gate and manual publish

- recheck crates.io name/version availability;
- confirm intended publisher authority;
- perform only the explicit `cargo publish -p eggpack-manifest --locked`.

Exit: crates.io accepts `eggpack-manifest 0.1.0`.

### WP5 — Registry-only consumer proof

- resolve exact `=0.1.0` in an external temporary project;
- compile and smoke schema-v1 parse/round-trip;
- prove registry source only;
- capture registry/package checksum identity.

Exit: downstream can resolve the exact crate without Git/path fallback.

### WP6 — Closure and cross-repo handoff

- write `plans/closure/release-manifest/003-status.md`;
- update Release Manifest roadmap and Eggpack registry;
- update Eggup active planning with the satisfied external prerequisite;
- leave Eggup's own package publication chain blocked/ready according to its
  actual registry state.

Exit: Eggpack publication work is closed and Eggup has an unambiguous next
action.

## 9. Acceptance criteria

| Requirement | Acceptance evidence |
|---|---|
| M003 consumer gate is satisfied | Eggup M003 closure + Eggsact `65c916b` hosted evidence referenced |
| Qualified source identity is preserved | exact `git diff 678bbf04..<publish-candidate> -- crates/eggpack-manifest` result recorded; empty or every delta explicitly requalified |
| Package identity is exact | package metadata shows `eggpack-manifest 0.1.0`, MSRV 1.89, MIT, expected repository/docs/readme |
| No hidden producer dependency | `cargo tree -p eggpack-manifest --locked` contains only intended schema dependencies/transitives |
| Clean package | `cargo package -p eggpack-manifest --locked` passes from clean tree; file list reviewed |
| Clean dry-run | `cargo publish -p eggpack-manifest --locked --dry-run` passes |
| Stable/MSRV behavior | required checks/tests/docs pass, plus hosted CI on exact candidate |
| First-publication conflict ruled out | immediate pre-publish crates.io query confirms name/version availability |
| Exact version published | crates.io exposes non-yanked `0.1.0` |
| Registry-only usability | external temp project resolves `=0.1.0`, checks, and runs minimal schema-v1 smoke with no Git/path source |
| Publication remains bounded | no other Eggpack package published; no automatic publication workflow added |
| Downstream truth reconciled | Eggup active planning points to satisfied Eggpack prerequisite while retaining its own remaining publication chain |
| Closure traceability | closure records source commit, package checksum, registry evidence, tag (if created), commands, CI run, and unresolved findings |

## 10. Stop conditions

Stop before publication and write a corrective/replan if any of the following is
found:

- `crates/eggpack-manifest/` has an unreviewed semantic/public-API delta from
  the consumer-qualified source;
- publication requires schema-v1 wire/API weakening;
- a non-registry runtime dependency is required;
- another Eggpack workspace package unexpectedly becomes a publication
  prerequisite;
- the exact package name is no longer available;
- the package version is no longer `0.1.0` or that version already exists with
  different bytes/source;
- package/dry-run fails for a reason requiring code/API changes;
- stable/MSRV/hosted qualification fails;
- packaged contents contain credentials, unintended generated artifacts, or
  unrelated repository content;
- registry-only resolution selects a Git/path source or cannot resolve exact
  `0.1.0`;
- publication credentials/ownership cannot be established safely;
- a downstream request would require Eggpack to take over Eggup release,
  install, or package-publication policy.

Do not "fix forward" through an irreversible first publish without a reviewed
corrective if one of these conditions triggers.

## 11. Security and maintenance review

The package is intentionally low-authority: bounded JSON parsing/serialization
only. Publication must not accidentally turn manifest evidence into trust or
install authority.

Specific checks:

- no secrets in package contents or diagnostics;
- no token in repository/CI;
- unknown fields/schema versions continue to fail closed;
- all untrusted input bounds remain enforced before allocation/iteration grows
  without limit;
- SHA-256 remains described as integrity evidence, not authenticity;
- package dependency footprint remains narrow;
- source/repository metadata allows downstream audit;
- closure records immutable registry/package identities for later incident or
  compatibility review.

## 12. Documentation / registry updates required at closure

At minimum:

- `plans/closure/release-manifest/003-status.md`;
- `plans/subsystems/release-manifest-roadmap.md`;
- `plans/registry.md`;
- Eggup `plans/subsystems/eggpack-manifest-interoperability-roadmap.md`;
- Eggup `plans/registry.md`.

Update `crates/eggpack-manifest/README.md` only if publication-facing wording
is materially missing or inaccurate. Do not add release/publish claims before
the registry proof exists.

## 13. Handoff after closure

Once `eggpack-manifest 0.1.0` is registry-resolvable, Eggpack has completed its
M004a-proven external prerequisite for Eggup.

The next actions belong to Eggup and must follow its own M004a evidence and
authorization rules. Eggpack should not preemptively publish Eggup crates or
change Eggsact's dependency source.

If Eggup discovers that the registry-published bytes/API differ from the
consumer-qualified Git source despite this milestone's evidence, treat that as
a compatibility incident: stop Eggup promotion, preserve the published
`0.1.0` record, and version/requalify normally rather than attempting to
overwrite the published version.
