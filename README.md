# eggpack

Developer-side release construction and distribution infrastructure for Eggstack.

Eggpack is intended to centralize portable release contracts, target/build/qualification planning, artifact composition, release manifests, bootstrap installers, and generated release CI. It deliberately does **not** replace Eggup: Eggup remains the consumer-side verified installation/update/rollback layer.

The repository is currently planning-first. Start with:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/registry.md`

The first implementation milestone is a faithful migration of the already-qualified unpublished `eggup-dist` schema-v1 contract into `eggpack-contract`, followed by conformance validators and a concrete release-manifest boundary.
