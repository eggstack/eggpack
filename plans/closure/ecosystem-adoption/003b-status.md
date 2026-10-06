# Ecosystem Adoption Milestone 003b — Eggsearch Seven-Target Eggpack Producer Cutover

Status: **conditionally closed** — one named operational condition remains: publication of the staged draft is a manual maintainer action.

Source plan: `plans/implementation/ecosystem-adoption/003b-eggsearch-seven-target-eggpack-producer-cutover.md`

Paired consumer plan (registered before any external edit, per planning process §9):

- `eggstack/eggsearch: plans/implementation/eggpack-release-adoption/001-seven-target-eggpack-producer-cutover.md` — registered at `eggstack/eggsearch@377f9e8`, registry `6767062`.

Predecessor evidence:

- `plans/closure/ecosystem-adoption/003a-status.md` (compatibility preflight, no producer prerequisite).

## 1. Baselines

| Role | Repository | Commit |
|---|---|---|
| Eggpack planning baseline | `eggstack/eggpack` | `6656752` (paired M003b plan registered) |
| Eggsearch reviewed baseline | `eggstack/eggsearch` | `4ccf35c` |
| Eggsearch implementation | `eggstack/eggsearch` | `eabbf80` (cutover), `28a0060` (baseline correction) |
| Eggsearch release candidate | tag | `v0.4.2` → `eabbf802449e32902110ae5adea4017a491c7161` |

A concurrent upstream commit (`eggstack/eggsearch@d444acc`, 26 application defect fixes) landed while the cutover was in flight. The two commits were rebased onto it and **the entire local gate was re-run on the rebased base** rather than on the pre-rebase tree. Gate results cited below are the post-rebase run.

## 2. What shipped

Eggsearch `release/eggpack/` now holds the complete, static, identity-free producer authority set:

| File | Owns |
|---|---|
| `distribution.toml` | seven targets, asset names, install names, checksum sidecars |
| `pack.toml` | build strategy, host, floor, qualification intent per target |
| `build-bindings.toml` | one direct Cargo output per target |
| `qualification-bindings.toml` | bounded core smoke per executing target |
| `consumer-validators.json` | required product-owned validator per target |
| `workflow-shape.json` | the static render seam, **derived** from the files above |
| `github-policy.json` | runners, action pins, release inputs, staging, cross-tool provisioning |
| `github-template.json` | draft release template |
| `install-policy.toml` | empty: every target is a direct artifact |
| `installer-presentation.json` | product wrappers are the public install surface |

`packaging/gen-release-workflow-shape.py` derives `workflow-shape.json` from the other nine files. Neither adopted reference repository (Eggsact, StegoEggo) does this — both hand-maintain the shape, which makes it a second copy of every producer fact. At seven targets that duplication is materially worse, so the generator was written for Eggsearch. The generated file is still drift-guarded by `eggpack ci check`, so the generator cannot silently diverge from what the renderer produces.

## 3. Generated-workflow evidence

`.github/workflows/release-eggpack.yml` is **generated** and was never hand-edited.

```text
eggpack ci generate --workflow-shape release/eggpack/workflow-shape.json \
  --contract release/eggpack/distribution.toml \
  --github-policy release/eggpack/github-policy.json \
  --output .github/workflows/release-eggpack.yml
  -> generated 75695 bytes                                    exit 0

eggpack ci check   … --workflow .github/workflows/release-eggpack.yml
  -> ci check: match (75695 bytes)                            exit 0

re-render determinism: cmp render-a.yml render-b.yml           byte-identical
working tree after regeneration                              unchanged
```

Hosted confirmation on the pushed commit, with the pinned tool read out of
`github-policy.json` at runtime (never duplicated into the workflow):

| Run | Workflow | Result |
|---|---|---|
| `37530846521` | Release drift guard | success — `ci check: match (75695 bytes)`, `eggpack 0.1.0` |

Rendered shape: **27 jobs** — `preflight`, `resolve`, 7 × `build_*`, 7 × `qualify_build_*`, 7 × `validate_build_*`, `required_gate`, `aggregate`, `stage`.

Static assertions over the rendered bytes:

| Assertion | Result |
|---|---|
| `--clobber` occurrences | 0 |
| `id-token` / `attestations: write` / `artifact-metadata` occurrences | 0 |
| `contents: write` | exactly 1 (the `stage` job) |
| `windows-11-arm` | 3 (build, qualify, consumer-validate for `aarch64-pc-windows-msvc`) |
| `push:` trigger | none; dispatch-only, so the generated writer cannot race a tag push |

## 4. Seven-target matrix, as rendered and as run

| # | Target | Strategy | Host | Runner | Floor | Qualification | Required consumer validator | Core smoke |
|---|---|---|---|---|---|---|---|---|
| 1 | `x86_64-unknown-linux-gnu` | CargoZigbuild | linux/x86_64 | `ubuntu-24.04` | glibc 2.17 | Native | yes | yes |
| 2 | `aarch64-unknown-linux-gnu` | CargoZigbuild | linux/aarch64 | `ubuntu-24.04-arm` | glibc 2.17 | Native | yes | yes |
| 3 | `armv7-unknown-linux-gnueabihf` | CargoZigbuild | linux/x86_64 | `ubuntu-24.04` | glibc 2.17 | **Structural** | **yes — carries the runtime proof** | **none** |
| 4 | `x86_64-apple-darwin` | NativeCargo | macos/x86_64 | `macos-15-intel` | none | Native | yes | yes |
| 5 | `aarch64-apple-darwin` | NativeCargo | macos/aarch64 | `macos-14` | none | Native | yes | yes |
| 6 | `x86_64-pc-windows-msvc` | NativeCargo | windows/x86_64 | `windows-latest` | none | Native | yes | yes |
| 7 | `aarch64-pc-windows-msvc` | NativeCargo | windows/aarch64 | `windows-11-arm` | none | Native | yes | yes |

All seven are `support = "required"`.

### Deliberate divergence from the M003a scratch

M003a's scratch used `floor = { kind = "none" }` for ARMv7. The implementation uses `floor = { kind = "glibc", major = 2, minor = 17 }`.

Reason: the predecessor workflow built `armv7-unknown-linux-gnueabihf.2.17`, and the plan mandates a glibc 2.17 ceiling proof for **all three** GNU targets. A `none` floor would render the build target without the `.2.17` sysroot suffix and would make the mandated ARMv7 ceiling proof a statement about a build that does not hold the floor. The stricter value reproduces predecessor build semantics exactly and makes the proof meaningful. M003a is not contradicted; its scratch value was superseded by a better-grounded choice.

### ARMv7 structural constraint

`Qualification::Structural` forbids a core smoke binding (`QualificationBindingsV1::validate_for` requires `smoke.is_some() == must_smoke`, and `must_smoke` excludes `Structural`). M003a recorded this as finding E-1 and the implementation honoured it: `qualification-bindings.toml` carries an **empty** `[targets."armv7-unknown-linux-gnueabihf"]` entry — required so the bindings cover exactly the ReleasePlan inventory — and no `smoke` block. A first render failed with the opaque `qualification bindings do not cover ReleasePlan` before this was corrected.

## 5. ARMv7 runtime proof

Core qualification executes nothing for ARMv7, so the proof moved entirely to the required product-owned consumer validator `packaging/validate-armv7-candidate.py`, which:

- asserts 32-bit ARM ELF (`EM_ARM == 40`, rejecting ARM64 and x86-64);
- asserts the GLIBC ceiling is ≤ 2.17, failing closed if `readelf` is unavailable or no GLIBC requirement is found at all;
- registers qemu-user binfmt from a **digest-pinned** `tonistiigi/binfmt`, failing closed rather than skipping;
- executes `--version` and `--help` under a **digest-pinned** `arm32v7/ubuntu` image;
- asserts the reported identity equals the workspace `Cargo.toml` version.

Local exercise against the real published `v0.4.1` ARMv7 binary, before any release run:

```text
PASS  assert_armv7_elf(eggsearch-armv7-unknown-linux-gnueabihf)      32-bit EM_ARM
PASS  glibc ceiling                                                 (2, 17)
PASS  negative control: x86-64 ELF rejected                        "candidate is not ELF32"
```

Hosted evidence, eggsearch run `37531104901`, artifact
`eggpack-consumer-evidence-armv7-unknown-linux-gnueabihf`:

```json
{
  "release_id": "v0.4.2",
  "source_revision": "eabbf802449e32902110ae5adea4017a491c7161",
  "target": "armv7-unknown-linux-gnueabihf",
  "interpreter": "python3",
  "outcome": "passed",
  "candidate_size": 17649944,
  "candidate_sha256": "a68890368507ecf5b205cbb4f7fc69648c54650174b557c3252c6cdd6fdae12f"
}
```

Because the validator is **required**, Eggpack gated `required_gate` and `aggregate` on it. `validate_build_armv7_unknown_linux_gnueabihf` completed success in the live run, and `required_gate` completed success only after it did.

## 6. Toolchain and glibc proof

Migrated from Zig 0.13.0 / cargo-zigbuild 0.20.1 to the Eggpack-qualified Zig 0.14.1 / cargo-zigbuild 0.23.3 pair, with the already-qualified official archive digests:

- Linux x86-64: `24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c`
- Linux AArch64: `f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b`

`mlugg/setup-zig` is no longer a build authority anywhere in Eggsearch; the generated workflow provisions Zig deterministically from those digests.

The glibc 2.17 ceiling is proved on the **exact staged bytes**, not on a build in the abstract: every natively-hosted target runs `packaging/validate-release-binary.py`, which reads `readelf --version-info`, collects `GLIBC_<n>` requirements, and fails above 2.17. Because staging only aggregates after every consumer validator passed, a binary carrying a higher ceiling can never reach a draft.

## 7. Public inventory: 16 → 19

Derived from the contract by `packaging/expected_release-assets` (module `packaging/expected_release_assets.py`), so the guard cannot drift from the producer:

| Count | Files |
|---|---|
| 7 | versionless executables, byte-identical public names to `v0.4.1` |
| 7 | `.sha256` sidecars |
| 2 | `install.sh`, `install.ps1` — product wrappers, copied unchanged |
| 1 | `release-manifest.json` — Eggpack final-bytes evidence |
| 2 | `install-exact.sh`, `install-exact.ps1` — Eggpack-rendered exact installers |

Every contract-expanded name was cross-checked against the published `v0.4.1` release through the closed Contract M003 surface, `eggpack contract expand --field asset|sidecar|install`, for all seven targets. All fourteen expanded names are unchanged.

**Wrapper byte-identity is proven by digest, not asserted.** The `v0.4.1` attestation statement (read via `gh attestation verify --format json`) records `install.sh` = `d860e7437de1c5b9…` and `install.ps1` = `27373018d6129ee5…`. The `v0.4.2` staging receipt records the same two digests. The public wrappers are byte-identical across the cutover.

Historical releases remain 16-asset releases. Only post-cutover releases carry 19; no history was rewritten.

## 8. Provenance seam

Artifact Attestation moved out of the release writer into `.github/workflows/release-provenance.yml`, so that **no Eggpack-generated job ever requests an OIDC write permission** (asserted statically at `packaging/check-contract.sh` and confirmed by 0 occurrences in the rendered workflow).

The provenance workflow is dispatch-only, takes an exact existing tag, and is read-only with respect to release state. It requires the checkout to be the tag commit with a clean tree and the GitHub Release to still be a draft, then `packaging/verify-staged-release.py` proves: the remote asset set is exactly the 19 expected names; each file's local digest and size equal what GitHub reports; all seven binary/checksum pairs self-verify; `release-manifest.json` declares the same release id, source revision, and target inventory with matching artifact digests and sizes; the public wrappers equal the checked-in wrapper bytes; both generated exact installers are present. Only then does it attest, and it verifies the result with `gh attestation verify`.

The verifier was additionally exercised **locally against the real staged `v0.4.2` draft**, not only by construction:

```text
$ python3 packaging/verify-staged-release.py --tag v0.4.2 --staged-dir /tmp/staged042
verify-staged-release: ok (v0.4.2, 19 assets, draft-only,
  digests/sizes/sidecars/manifest/wrappers verified)          exit 0
```

All 19 assets were downloaded from the draft and checked: exact set equality, per-asset digest and size against GitHub's own report, seven self-verifying binary/checksum pairs, manifest release-id/source-revision/target-inventory identity with matching digests and sizes, wrapper byte-equality, and both generated exact installers. The provenance workflow will therefore run on verified logic when a maintainer dispatches it.

**Historical subject parity (M003b §17.13) is established, not deferred.** The `v0.4.1` attestation covers exactly 16 subjects — 7 binaries plus their 7 sidecars plus `install.sh` and `install.ps1`. The new workflow attests a strict superset: those 16 plus `install-exact.sh` and `install-exact.ps1`. No previously attested subject is dropped. The readback that proved this used the same token that reads the repository, so no evidence gap remains.

## 9. Hosted release evidence

| Step | Evidence |
|---|---|
| `make check` on the rebased base | `CHECK_EXIT=0` |
| `make release-check` on the clean tree | `RELEASE_EXIT=0` (includes `cargo publish --dry-run --locked`) |
| producer drift, local | `ci check: match (75695 bytes)`, re-render byte-identical, working tree unchanged |
| hosted drift guard | run `37530846521` success |
| crate publication | `eggsearch 0.4.2` live on crates.io, not yanked |
| tag | `v0.4.2` → `eabbf802449e32902110ae5adea4017a491c7161` |
| release run | `37531104901` attempt 1, `workflow_dispatch`, `release_tag=v0.4.2`, **27/27 jobs success** |
| staging receipt | draft `405157598`, `draft: true`, `immutable: false`, **19 assets** |
| rerun | `37531104901` attempt 2, same tag/source, **failed closed** — see §9a |

Live per-job outcome: 7 build + 7 qualify + 7 consumer-validate all success; `required_gate` success; `aggregate` success; `stage` success.

## 9a. Rerun: refused, not reused — a real and unresolved finding

Attempt 2 of run `37531104901` (same `release_tag=v0.4.2`, same source revision) rebuilt all seven targets, passed all 21 per-target jobs, passed `required_gate` and `aggregate`, and then **failed at `stage`**:

```text
prepare-stage: 19 assets
eggpack: same-name remote asset digest mismatch
```

This is the no-clobber invariant working exactly as designed: an already-staged asset whose bytes differ from the newly built bytes is a **refusal**, not an overwrite. The draft was left untouched and still holds attempt 1's complete 19 assets.

But the plan's §16 step 6 asked for a rerun proving **byte-identical reuse**. That was **not** achieved, and the requirement is recorded as unmet rather than reinterpreted. Two facts are established; one is not:

Established:

- Every non-binomial pipeline stage re-derived identically enough to reach staging (`prepare-stage: 19 assets`).
- The difference is in at least one **asset payload**, not in the asset set, the names, or the release identity.
- The public wrappers are *not* implicated: attempt 1's staged `install.sh`/`install.ps1` digests are byte-identical to the `v0.4.1` attested digests, and those two files are copied from the checkout rather than built.

Not established — and worth naming precisely:

- **Which** asset differed. Eggpack's staging receipt is written only on success; a refusal produces no receipt, and the generated workflow uploads only the receipt, so the attempt-2 payload is not recoverable from the run.
- **Why**. Two plausible causes were considered and the obvious one was **disproved**: `CARGO_TARGET_DIR` is set per attempt (`${{ runner.temp }}/eggpack/${{ run_id }}-${{ run_attempt }}`), which would normally embed differently on each attempt. It does not — the staged binary contains zero occurrences of `runner.temp/eggpack`, and its embedded registry paths are the stable `index.crates.io-<hash>` form. Remaining candidates are a floated hosted toolchain/image between 21:00 and 21:30, or a non-deterministic link step; neither was isolated with the evidence available.

Two consequences for future plans:

1. Eggpack has no machine-readable evidence of *which* asset a no-clobber refusal concerns. That is a genuine usability gap for any consumer debugging a failed rerun, and is a candidate corrective, not a defect: refusal is the safe behaviour.
2. Eggsearch's release build is **not** demonstrated byte-reproducible across attempts. Until that is either fixed or accepted, an Eggsearch rerun against an existing draft will refuse rather than reuse, and recovery means the documented operator path (delete the draft deliberately, rerun the same exact tag/source, get a fresh 19-asset draft).

**The staged draft needs no recovery.** Attempt 1's draft is complete, internally consistent, and built from the tagged source with every per-target validation green. It is the artifact a maintainer should publish. The refusal is the reason that artifact is still intact.

## 10. Deviation from the plan's stated ordering — recorded, not hidden

The plan (§16) sequences the work as: qualify the exact candidate through Eggpack, *then* retire the old writer. That ordering is **unsatisfiable** for this consumer and was not followed literally.

The generated workflow is `workflow_dispatch`-only and requires an **exact existing tag**. The legacy writer is triggered by `v*` tag pushes. Therefore pushing `v0.4.2` — the very act needed to obtain Eggpack qualification evidence — would have fired the legacy writer, which uploads with `--clobber`. Two writers would then be able to mutate the same draft.

The plan's own stronger invariant, "at no point may two workflows independently mutate the same release draft," was chosen over its weaker sequencing guidance. The legacy `release-binaries.yml` was deleted **in the same commit** as the cutover, before the tag was pushed, so the tag push had exactly one writer.

Consequence, stated plainly: **there is no live staging evidence from a period during which both writers existed**, because there was no such period. The live evidence is from a state in which the generated writer was already the sole writer. This is a deviation in ordering only; no invariant was weakened and no `--clobber` path was ever reachable from the generated workflow.

## 11. Baseline correction made in passing

`src/core/error_query.rs:351` used `.filter(..).next_back()`, which newer clippy denies as `clippy::filter_next`, so `make check` could not compile with `-D warnings`. This was pre-existing and unrelated to the cutover. The compiler's own suggested `.rfind(..)` was applied in a **separate commit** (`28a0060`) so the release diff stays reviewable. Semantically identical.

A second local-only obstruction is worth recording: this sandbox replaces `rm` with a trash wrapper that exits non-zero when the target is already absent, which breaks `set -euo pipefail` scripts such as `packaging/test-install.sh`. It was confirmed to fail identically on a clean tree before any change. Verification runs used a local PATH shim restoring real `rm` semantics. This is an environment artifact, not a repository defect, and it is not present in CI.

## 12. Acceptance criteria

| # | Criterion | Status |
|---|---|---|
| 1 | paired plans exist and were followed | met — both registered before any external edit |
| 2 | Eggsearch producer authority checked in under `release/eggpack/` | met — 10 files |
| 3 | generated workflow is drift-checked | met — local + hosted run `37530846521` |
| 4 | all seven target names remain unchanged | met — verified against `v0.4.1` via `contract expand` |
| 5 | 16 → 19 transition documented/guarded and additive only | met — derived inventory guard |
| 6 | glibc ≤ 2.17 on all three GNU targets after the bump | met — per-target proof on staged bytes |
| 7 | ARMv7 runtime proof through the required consumer validator | met — run `37531104901`, outcome `passed` |
| 8 | Windows ARM64 remains required and passes | met — `windows-11-arm`, 3 jobs green |
| 9 | product wrappers retain fallback/update semantics | met — untouched; digests identical to `v0.4.1` |
| 10 | generated installers and manifest staged as additive producer evidence | met — 19-asset receipt |
| 11 | staging draft-only, no-clobber, exact-reuse, exact tag/source | **partial** — draft-only, no-clobber, and exact tag/source are met (receipt `draft: true`; zero `--clobber`; rerun resolved the same tag/source). Exact **reuse** on rerun is not met: attempt 2 refused with `same-name remote asset digest mismatch` because the rebuild is not byte-reproducible across attempts. See §9a |
| 12 | generated jobs request no OIDC write permission | met — 0 occurrences, asserted by guard |
| 13 | provenance subject parity recorded, or closure conditional on a named read gap | met — parity **established** from the `v0.4.1` attestation |
| 14 | provenance workflow mutates no release/tag/asset state | met — asserted by guard; workflow is read-only |
| 15 | local gates and required hosted qualification green | met — `CHECK_EXIT=0`, `RELEASE_EXIT=0`, 27/27 jobs |
| 16 | no medium-or-higher regression remains | met — one Low finding recorded in §13 |
| 17 | old hand-maintained release workflow no longer owns build/stage authority | met — deleted in the cutover commit |
| 18 | publication remains human-controlled | met — no workflow publishes; the draft is unpublished |
| 19 | closure records exact commits/run IDs/release evidence | met — this record |

## 13. Findings

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| E-1 | Medium (M003a) | `Qualification::Structural` forbids a core smoke binding; a consumer copying per-target smoke blocks verbatim fails with the opaque `qualification bindings do not cover ReleasePlan` | Correct behaviour. Honoured by omitting the ARMv7 smoke. Not an Eggpack defect: relaxing the rule would let a consumer declare a smoke that never runs. |
| E-6 | Medium (found here) | Eggsearch's release build is not byte-reproducible across workflow attempts, so a rerun against an existing draft refuses instead of reusing. The differing asset and its cause could not be identified: the receipt is written only on success, so a refusal leaves no machine-readable evidence. The obvious suspect (per-attempt `CARGO_TARGET_DIR`) was disproved by inspecting the staged binary | Consumer-owned reproducibility work, plus an Eggpack usability gap: a no-clobber refusal should identify the asset concerned. Both are candidate correctives, not defects — refusing is the safe behaviour and the draft stayed intact | §9a |
| E-2 | Medium (found here) | The plan's §16 ordering (qualify, then retire the old writer) is unsatisfiable for a `workflow_dispatch`-only, tag-requiring generated writer, because the tag push is itself the trigger for the legacy writer | Ordering deviation, recorded in §10. The stronger single-writer invariant was preserved instead. No production change. |
| E-3 | Low (found here) | The plan's §3 ARMv7 floor treatment in M003a scratch (`none`) would have rendered a build that does not hold the documented 2.17 floor, making the mandated ARMv7 ceiling proof vacuous | Corrected in implementation to `glibc 2.17`; recorded in §4. |
| E-4 | Low (found here) | `packaging/test-install.sh` deletes a file that may not exist; under an environment whose `rm` wrapper exits non-zero for a missing path, the script aborts under `set -euo pipefail` | Environment artifact, confirmed on a clean tree before any change. Not a repository defect; recorded so a future reader does not mistake it for a regression. |
| E-5 | Low (found here) | `src/core/error_query.rs` `.filter(..).next_back()` is denied by current clippy, blocking the local gate | Corrected in a separate commit (`28a0060`). |

## 14. Stop conditions

None of the §18 stop conditions fired. Specifically:

- the toolchain bump did **not** violate glibc 2.17 — proved per target on the staged bytes;
- ARMv7 runtime proof **did** run through the bounded consumer-validator seam, with no new Eggpack primitive and no generic setup-command hook (the validator registers binfmt and pins its own images);
- Windows ARM64 **did** execute on current runner mapping;
- the exact 19-file additive inventory **was** acceptable and is guarded;
- provenance **did not** require OIDC in any generated job;
- provenance **did not** become a second release writer;
- generated staging needed **no** clobber or overwrite path;
- consumer updater/install semantics **did not** regress;
- **no new Eggpack production primitive became necessary.** The Eggpack diff for this milestone is empty — the entire capability was already available.

## 15. Not performed, and therefore not claimed

- The staged `v0.4.2` draft is **not published**. Publication is a separate maintainer action and was not authorized here.
- No external Unix/Windows installer smoke was run against the *published* `v0.4.2`, because it is not published.
- The plan's §17.13 subject-parity readback was performed against the `v0.4.1` attestation; the `v0.4.2` attestation set is asserted by design and verified by the provenance workflow when it is dispatched, which requires a staged draft. No `v0.4.2` attestation has been created, so none is claimed.
- No Eggpack production source, Cargo metadata, generated workflow, or published crate changed in this milestone.

## 16. Named remaining condition

**Publication of eggsearch `v0.4.2` (draft `405157598`) is a manual maintainer action.** Until a maintainer publishes it:

- the seven public binaries, their sidecars, and the two public wrappers for `v0.4.2` are not downloadable from GitHub Releases;
- `releases/latest/download/install.sh` still resolves the `v0.4.1` asset, which is correct and unchanged — the wrappers are byte-identical;
- the `Release provenance` workflow has not been dispatched against `v0.4.2`, because it requires a staged draft and is the last step before publication.

Discharging the condition is a maintainer action in `eggstack/eggsearch`, not Eggpack work. Once published, this closure can be promoted from *conditionally closed* to *closed* by appending the release evidence.

## 17. Next handoff

- **Eggpack**: no open work. The ecosystem-adoption subsystem's third open line is closed or conditionally closed; `plans/registry.md` is the control surface.
- **Eggsearch**: publish `v0.4.2`, then dispatch `Release provenance` for `v0.4.2`, then verify public installers/updater against the published assets.