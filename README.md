# eggpack

Developer-side release construction and distribution infrastructure for Eggstack.

Eggpack is intended to centralize portable release contracts, target/build/qualification planning, artifact composition, release manifests, bootstrap installers, and generated release CI. It deliberately does **not** replace Eggup: Eggup remains the consumer-side verified installation/update/rollback layer.

The `eggpack-contract` crate implements the portable
DistributionContract schema-v1 authority used to describe target and artifact
names across Eggpack tooling. It is synchronous, side-effect free, and does not
build or publish releases. See [the crate README](crates/eggpack-contract/README.md)
for the schema and boundaries.

The `eggpack-manifest` crate defines bounded schema-v1 JSON evidence for
finalized release artifacts and members. It is a synchronous leaf parser and
serializer; see [its README](crates/eggpack-manifest/README.md) for the format
and consumer boundary.

Planning and architecture references:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/registry.md`

The contract implementation preserves the qualified unpublished `eggup-dist`
schema-v1 predecessor. Contract conformance validators are closed, and the
ReleaseManifest v1 domain is implemented.

`eggpack-core` owns pure PackConfig/ReleasePlan resolution, explicit Cargo
build bindings and bounded candidate production, qualification evidence for
native/deferred/QEMU/structural paths, and producer-side manifest construction
from explicitly named finalized files. BuildAttempt identity is tied to the
release and source revision; qualification verifies candidate format and
architecture, hashes candidate bytes, and runs only a selected candidate with
bounded fixed arguments. Qualification evidence is distinct from finalized
artifact and manifest evidence.
`finalize_release` gates required targets on qualification, assigns exact contract
filenames, writes checksum sidecars, assembles explicitly selected `.tar.gz`
archives, and returns a manifest over the final bytes from a new output root
under a caller-secured parent. It does not extract, install, publish, or authenticate
artifacts.
`eggpack-bootstrap` renders release-specific direct first-install shell and
PowerShell scripts from the contract and manifest; it does not select releases
or update existing installations. SHA-256 checks establish integrity, not
authenticity. Bundle releases install all verified members atomically, and
archive releases support tar+gzip only with exact member inventory and
size/hash validation before transactional placement under caller-owned
executable/data modes.

`eggpack-ci` projects resolved release plans and explicit M002 Cargo bindings
into a provider-neutral CI graph, renders read-only deterministic GitHub
Actions workflows from caller-supplied runner/action-pin policy, and checks
workflow drift without writing files. M002 adds executable qualification jobs,
required-evidence gates, and aggregate/finalize nodes invoking M003/M004 through
the `eggpack` CLI with pinned tooling; completed finalized releases are uploaded
as internal workflow artifacts only, alongside a standalone deterministic
`release-manifest.json` that decodes back to the exact M004 manifest without
changing the finalized root. M003a adds the local staging payload materializer
and GitHub draft adapter (draft-only, exact existing tag, no publication).
M003b wires that adapter into one least-privilege generated `stage` job
(`contents: write` isolated to stage, every prior job `contents: read`, no
`id-token: write`, token via environment only): checkout the exact tag,
install the pinned Eggpack CLI, download the exact aggregate artifact, run
`_prepare-stage`, then `_stage-github-draft`, and upload the bounded staging
receipt. Reruns reconcile exact draft/asset state without clobber; public
publication remains a separate human action. Live draft qualification remains
outstanding (maintainer-authorized fixture required). M003d adds the consumer
composition seam: reusable checked-in workflows carry static shape only
(`ReleaseWorkflowShapeV1`, no future tag or source SHA) with a runtime
`resolve` job materializing invocation-local ReleasePlan/ReleaseCIPlan/draft
policy per event-selected tag; product-owned `install.sh`/`install.ps1`
wrappers coexist with generated exact installers (`install-exact.*`) as
copied bytes; and a bounded Python3 consumer validator runs the exact
candidate after core qualification with identity-linked evidence and no
arbitrary command support. Ownership boundary: generated exact installers are
Eggpack first-install evidence, product wrappers are consumer-owned
selection/fallback/install UX, and the consumer validator is bounded
consumer-owned release evidence rather than Eggpack qualification semantics.

`eggpack-cli` provides deterministic `eggpack ci generate` and `eggpack ci check`
(exact `--ci-plan` mode plus reusable `--workflow-shape` mode) with narrow
internal runner commands wrapping source verification, runtime identity
resolution, core qualification, consumer validation, finalization, and draft
staging.
