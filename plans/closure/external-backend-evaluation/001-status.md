# External Backend Evaluation M001 — Closure

Status: closed — disposition C (design prior art with selected output normalization)

## Source plan and roadmap

- Plan: `plans/implementation/external-backend-evaluation/001-dist-0.33-capability-and-interoperability-spike.md`
- Roadmap: `plans/subsystems/external-backend-evaluation-roadmap.md`
- Reviewed Eggpack baseline: `cbe22eb3cbffc3ac5710837d0036bcba9290da02` (contract M001 closed)
- Evaluation activation/status commit: `383f3e8`
- Eggpack production implementation commit: none; this was an evidence-only spike.
- Exact upstream tag/commit: `axodotdev/cargo-dist` `v0.33.0`, `a1c829ffacb7ce98fb617b7acd7ffdd2282b45c2`
- Upstream release page: <https://github.com/axodotdev/cargo-dist/releases/tag/v0.33.0>
- Upstream config reference at the pinned commit: <https://github.com/axodotdev/cargo-dist/blob/a1c829ffacb7ce98fb617b7acd7ffdd2282b45c2/book/src/reference/config.md>
- Upstream manifest schema at the pinned commit: <https://github.com/axodotdev/cargo-dist/blob/a1c829ffacb7ce98fb617b7acd7ffdd2282b45c2/cargo-dist-schema/src/lib.rs>

## Executive finding and disposition

**Disposition C.** Treat `dist` 0.33 as design prior art for target planning, archive assembly, checksum generation, generated installers, and build-manifest organization. Do not adopt it as Eggpack's production backend or canonical artifact authority in this milestone.

The exact tool builds useful ordinary Rust archives, including a two-binary Egress-shaped archive, and generates a target-aware workflow and installer pair. The direct fixture produces archives rather than Eggpack's raw direct asset. The Gregg-shaped multi-package fixture produces two separately identified app releases and two archives, not one Eggpack sibling-bundle artifact group. Its manifest lacks byte sizes, source revision, qualification status/evidence, and an Eggpack schema contract. Cross/native build tasks are represented, but qualification truthfulness remains a CI concern. Its generated tag workflow publishes automatically; `create-release = false` still has the workflow undraft a pre-existing draft. Several generated jobs inherit write permission. An adapter would still need Eggpack-owned contract validation, inventory/hash finalization, qualification evidence, and publication gating.

Selected normalization direction for any separately approved future experiment: treat dist output only as backend observations; map its app/version, target, archive, asset path, and post-build checksum data through the Eggpack contract; independently validate final files and calculate Eggpack size/digest fields; attach source revision and qualification from Eggpack-owned evidence. Never import dist types as canonical Eggpack API and never use its generated publishing workflow as Eggpack's publication authority. This closure does not authorize that adapter or a production dependency.

## Pinning and environment

- Release published 2026-09-11; the immutable tag resolves to the planned commit.
- Environment: Linux x86_64, `rustc 1.98.1`, `cargo 1.98.1`.
- Downloaded asset: `cargo-dist-x86_64-unknown-linux-gnu.tar.xz` (6.1 MiB); the downloaded sidecar and local `sha256sum` both reported `4b3f0a5f0ebbdb798f6db649d01b32ba1518376b6f7a0502b7d92b75cc2c8293`.
- Executable reported `cargo-dist 0.33.0`.
- Source checkout command: `git clone --depth 1 --branch v0.33.0 https://github.com/axodotdev/cargo-dist.git`; checked-out `HEAD` was `a1c829ffacb7ce98fb617b7acd7ffdd2282b45c2`.
- All projects used dummy `eggstack/eggpack-dist-*-fixture` repository URLs. No matching remote was created or contacted for release activity.

## Fixtures and commands actually run

Disposable fixtures are retained under `plans/closure/external-backend-evaluation/fixtures/`:

- `direct/`: eggsact 1.2.6, shell + PowerShell installers, direct-like single binary, target edge settings, glibc override, and attestation enabled.
- `sibling-bundle/`: gregg + greggd as separate Cargo packages with one shared version/tag.
- `archive-pair/`: egress package with `eggress` + `pproxy` binaries in one Unix archive.

For each fixture, a temporary worktree was used so generated GitHub workflow files stayed outside Eggpack. Commands run with the exact binary:

```text
dist init --yes --skip-generate
dist generate --mode ci
dist plan --output-format json --no-local-paths --tag <fixture-version>
dist generate --mode ci --check
```

All three plans succeeded and all three `--check` drift checks succeeded. Additional local-only builds:

```text
dist build --artifacts all --target x86_64-unknown-linux-gnu --tag 1.2.6
dist build --artifacts global --tag 1.2.6
dist build --artifacts all --target x86_64-unknown-linux-gnu --tag 0.9.0
dist build --artifacts all --target x86_64-unknown-linux-gnu --tag 2.1.0
dist build --artifacts local --target x86_64-unknown-linux-gnu --tag 2.1.0 --output-format json --no-local-paths
```

The final command produced the bounded captured excerpt in `samples/archive-build-manifest.json`. Archive listing confirmed `egress-x86_64-unknown-linux-gnu/eggress` and `.../pproxy` members. No public release, remote build, tag push, registry upload, or attestation was performed.

## 1. Feature/capability matrix

| Capability | Result | Evidence / limitation |
|---|---|---|
| Control package/target archive asset names | Supported with conventions | Names are package + target + platform extension (`eggsact-x86_64-unknown-linux-gnu.tar.xz`); checksum sidecar is derived. Arbitrary raw asset names require `extra-artifacts` build commands. |
| Eggpack `direct` raw executable asset | Not represented directly | Default artifact is an `executable-zip` tar/zip containing the executable. `extra-artifacts` can expose custom files but uses a build command and does not supply Eggpack's normal target/member contract. |
| One archive with multiple binaries | Supported | `archive-pair` produced one tarball containing `eggress` and `pproxy`; configurable `unix-archive` selected `.tar.gz`. Tar paths have an archive-name root directory; zip members are at root. |
| Multiple package releases on one tag | Supported as separate apps | Bundle fixture has two `releases[]` entries, two archives, two installers, and one shared tag/hosting path. This is not one logical Eggpack sibling-bundle group. |
| Installer generation | Supported | Shell and PowerShell use manifest-derived platform mapping; default install path is configurable. Generated shell checksum verification may skip if the required local checksum utility is absent. |
| Checksums | Supported | SHA-256 sidecars and `sha256.sum` were generated; post-build JSON includes the SHA-256 value. These remain integrity data. |
| Build and archive generation | Supported for Cargo apps | Direct and archive host-target fixture builds passed. Cross-target matrix generation is supported; we did not build those remote targets. |
| Build hooks/custom workflow jobs | Supported with custom glue | `extra-artifacts`, `github-build-setup`, and reusable jobs are commands/workflows supplied by the user; they are not bounded product smoke/qualification types. |
| CI generation and drift check | Supported | `dist generate --mode ci` writes a checked-in release workflow; rerun `--check` passed for every fixture. |
| Human-controlled publication gate | Not supplied by generated default | Tag pushes trigger automated release creation. `create-release = false` assumes a draft exists and automatically undrafts it after upload. A separate protected/manual gate would be Eggpack/host policy and would require a custom seam. |
| External updater | Optional, default off | `install-updater = false` produced no updater artifact. Enabling it adds dist's standalone `*-update` command; normal Eggup update ownership can remain intact only if this stays disabled. |
| GitHub Artifact Attestations | Supported, optional | Enabling the flag generated `actions/attest@v4` against per-target files under `target/distrib`; no attestation was generated in this local spike. |

## 2. Target matrix

| Target/constraint | Result | Planned runner/build evidence |
|---|---|---|
| Linux GNU x86_64 | Supported directly | `ubuntu-22.04`, native host match. |
| Linux GNU AArch64 | Supported directly | `ubuntu-22.04-arm`, native host match in generated matrix. |
| Linux ARMv7 GNU hard-float | Supported via cross tool | Plan installed/used `cargo-zigbuild` on x86_64; no ARMv7 execution evidence. |
| Linux musl x86_64 | Supported via system setup | Plan selected `musl-tools`; build was not run for musl. |
| macOS x86_64/AArch64 | Supported directly | `macos-15-intel` / `macos-14` runners. |
| Windows x86_64 | Supported directly | `windows-2022`, native host match. |
| Windows AArch64 | Supported via cross tool | Plan used Linux + `cargo-xwin`; no Windows ARM64 execution evidence. |
| glibc 2.17 floor | Configurable | `min-glibc-version` accepts per-target or `"*"` override. Our fixture set `"*" = "2.17"`; no cross-built artifact proved the actual floor, and the manifest does not expose the configured floor. |
| macOS deployment floor | Not represented directly | No corresponding v0.33 config setting was found. A custom build setup/environment hook may be needed; artifact manifest does not attest a floor. |

The exact fixture plan matrix was inspected for target, runner, host, and package-install command. The source target catalogue at the pinned commit also includes ARMv7 and Windows ARM64. Cross-build support is not counted as target execution or qualification.

## 3. Qualification matrix

| Qualification path | Result | Eggpack consequence |
|---|---|---|
| Native build | Supported as a build job on matching runners | The manifest records build system/environment; successful compilation is not a smoke result. |
| Cross-build then later native smoke | Build/runner work can be arranged | No first-class deferred-qualification state or evidence reference in `dist-manifest`; Eggpack must own the later test and evidence. |
| QEMU/container smoke | Custom workflow possible | Not a dist target qualification mode; custom workflow and explicit evidence normalization required. |
| Structural-only artifact verification | Build/archive generation covers some structure | No explicit structural qualification record or completeness claim in manifest. |
| Product-owned bounded smoke | Custom `host-jobs` / reusable workflow possible | No command bounds or qualification category in the dist domain model; release barrier can depend on job success, but structured evidence is external. |
| Failed sibling/target work | Release host job waits on successful plan/build jobs | The workflow can prevent final host/release when a required build job fails; logical bundle membership itself is not a first-class contract. |

## 4. Manifest-field normalization matrix

| Eggpack field | dist representation | Classification |
|---|---|---|
| Product/release identity | `releases[].app_name`, `app_version`, top-level `announcement_tag` | Derivable for simple one-app mapping; semantics are dist app/tag identity, not Eggpack ProductId/ReleaseId authority. |
| Target identity | `artifacts[].target_triples`, `assets[].target_triples` | Direct data, but still must resolve/validate against Eggpack contract. |
| Asset filenames and kind | `artifacts` map keys/names and `kind` | Direct for dist artifacts; default archive kind is `executable-zip`, not Eggpack direct/bundle/archive enum. |
| Archive member/install identity | `assets[].name` and `assets[].path` | Direct for packaged executable names/paths; custom arbitrary source-to-install mapping is not represented. |
| Byte size | No field on `Artifact` | Missing; Eggpack must stat final bytes. |
| SHA-256 | Sidecar reference before final build; `checksums.sha256` after build | Direct after build, but Eggpack must recompute/verify before finalizing its own manifest. |
| Source revision | Announcement tag; generated host workflow uses `github.sha` separately when creating a release | Missing from `DistManifest`; tag is not an immutable source revision field. |
| Qualification status/evidence | `systems` includes toolchain/build environment and linkage | Semantically insufficient; no native/deferred/QEMU/smoke status or evidence reference. |
| Provenance reference | `github_attestations` flag/config; GitHub stores attestation externally | Missing as a manifest reference. Attestation subject can be selected final artifacts; source/workflow identity is in external GitHub provenance. |
| Explicit schema compatibility version | `dist_version` identifies generator version | No independent Eggpack-style manifest schema-version contract was found in `DistManifest`; parser/normalizer must be version-pinned. |

The captured post-build sample is `samples/archive-build-manifest.json`. Its size, source revision, and qualification omissions are visible in the typed output and upstream schema.

## 5. Installer/updater ownership matrix

| Concern | Finding |
|---|---|
| Target mapping | Generated from dist's manifest/planned triples and embedded into shell/PowerShell installer; no Eggpack contract input. |
| Integrity | Installer selects declared checksum data and verifies when a supported checksum utility is present; shell code may warn and skip when unavailable. |
| Source fallback | Explicit installer URL/base override variables exist. Homebrew/npm are separate selected installer types, not implicit Eggup behavior. |
| Install behavior | Installs binaries into configured/default path; this is bootstrap behavior, not normal update/rollback authority. |
| Updater | `install-updater=false` omits updater artifact. The optional feature would compete with Eggup as normal self-update authority, so it must remain disabled for any future integration. |
| Eggup boundary | Eggup can remain the normal updater if Eggpack uses only first-install installer output and translates artifacts through its own manifest. |

## 6. CI/publication control matrix

| Control | Finding |
|---|---|
| Checked-in workflow | Yes; `dist generate --mode ci --check` verifies generated-file drift. `allow-dirty = ["ci"]` suppresses drift but gives up automatic template updates. |
| Default trigger | Pull requests plan; matching tag pushes drive create/build/host/release workflow. |
| Build/write separation | Some build jobs have `contents: read`; local artifact build with attestations has `contents: read`, `attestations: write`, and `id-token: write`. Generated workflow has workflow-level `contents: write`, and other built-in jobs without overrides inherit it. Not a least-privilege boundary Eggpack should adopt as-is. |
| Human release gate | No generated manual approval step. `create-release=false` still ends by undrafting a draft automatically. A protected GitHub environment/custom host gate would be external or require owned customization. |
| Tool/action pinning | `cargo-dist-version = "0.33.0"` pins the tool version. Generated actions use version tags (`checkout@v6`, `upload-artifact@v7`, `download-artifact@v8`, optional `attest@v4`); `github-action-commits` can pin action commits by configuration. |
| Hidden remote policy | Generated workflow installs the pinned dist release and contains the upstream workflow template. It is checked in and reviewable, but template behavior remains owned by upstream version and regeneration. |

## 7. Maintenance/dependency assessment

- Developer/CI integration is a standalone prebuilt Rust executable; the tested Linux release archive was 6.1 MiB. No Eggpack Cargo dependency or runtime updater dependency is needed to invoke it as a build tool.
- A production use would make Eggpack own selecting/updating the dist version, validating its output schema, handling custom artifacts/jobs, reconciling generated workflow drift, and tracking upstream target/installer/security changes.
- The output schema has a substantial generic model and supports custom jobs/extra artifact commands, but normalizing Eggstack's direct/bundle/archive, qualification, source revision, and publication policy remains Eggpack work. Wrapper and policy cost erodes the value of treating dist as the release authority.
- Generated installers are substantial shell/PowerShell artifacts whose behavior and action templates follow the selected dist version. Upstream changes therefore require fixture/regression review on version bumps.
- Default action refs are version tags; commit pin overrides are available. The dist installer command itself uses a release-tagged upstream script in generated CI.
- GitHub attestations are optional and externally verified by GitHub CLI; they do not impose a runtime consumer dependency, but their policy and manifest linkage remain Eggpack concerns.
- Upstream's immutable v0.33 release page lists the built tool artifacts, checksum sidecars, and GitHub Artifact Attestations: <https://github.com/axodotdev/cargo-dist/releases/tag/v0.33.0>.

## Exact verification and hosted evidence

- Release/tag/commit verified by `git ls-remote`, GitHub release API, and a depth-1 source checkout.
- Downloaded release sidecar SHA-256 matched `sha256sum`.
- `dist --version` returned `cargo-dist 0.33.0`.
- All three fixture `dist plan --output-format json --no-local-paths --tag ...` commands passed.
- `dist generate --mode ci --check` passed for all three generated fixtures.
- Direct-like local archive build and global shell/PowerShell installer generation passed for the x86_64 Linux build path.
- Gregg sibling fixture build passed and emitted separately named `gregg-*` / `greggd-*` archives and sidecars.
- Egress archive fixture build passed; tar listing contained both executable members beneath the archive-name root directory.
- The post-build JSON manifest sample was captured after output generation. No cross-architecture build, PowerShell execution, GitHub-hosted fixture CI, actual attestation creation, or public hosting was run.
- No hosted CI applies to these disposable fixtures; no credentials were used.
- `git diff --check` is required before committing this closure and fixture evidence.

## Invariant, failure, compatibility, and security review

- No public release, tag movement, registry upload, real consumer modification, production dependency, or runtime updater adoption occurred.
- All builds were local-only and used synthetic binaries/repository URLs. Build failures remained command failures; the only initial failure was a fixture manifest typo (the per-target `binaries` override required a map), corrected before accepted runs.
- No compatibility promise is made for dist's manifest or configs. The future Eggpack backend must be replaceable without DistributionContract/ReleaseManifest schema changes.
- Updater remained disabled in fixtures. Checksums were treated only as integrity.
- Generated CI enables remote write authority for release jobs and some inherited jobs. It triggers publication from a tag without a built-in human approval gate; this is incompatible with Eggpack's explicit final publication gate unless wrapped/overridden under a separate approved design.
- No secrets were available to the fixture. Attestation support was inspected in generated source only; no external write action executed.

## Unresolved findings and evidence limits

No unresolved issue prevents a prior-art-only disposition. Remaining low-severity operational unknowns: actual remote cargo-xwin Windows ARM64 build behavior; actual ARMv7 native smoke; real artifact enforcement of configured glibc 2.17; and a product-specific macOS deployment floor hook. These require target runner/build evidence in a future plan, not assumptions from the generated matrix.

## Roadmap disposition and dependency transitions

- External Backend Evaluation M001: **closed, disposition C**.
- External Backend Evaluation M002 Backend Selection ADR: **not required** for disposition C; create only under a new plan if future evidence recommends A/B production adoption.
- Build/Qualification M001 remains blocked on Contract M002 and Release Manifest M001. External-tool disposition is satisfied; it no longer blocks those milestones, and the eventual build plan should specify a replaceable native/backend seam without adopting dist types.
- Release Manifest M001 remains blocked on the Contract M002 expected-file/conformance interface.
- Bootstrap Installers, CI Orchestration, Eggup Interoperability, and Adoption remain blocked by their contract/manifest/build roadmap dependencies.
- Contract/Conformance M002 is ready and is the next dependency-ready implementation plan; no unforeseen issue blocks it.

## Registry updates

Record the spike closed with C, remove the conditional backend ADR from active blocked work, retain Contract M002 as the next ready handoff, and show Build/Qualification gated by Contract M002 + Manifest M001 rather than the now-closed dist disposition. No production backend is selected.
