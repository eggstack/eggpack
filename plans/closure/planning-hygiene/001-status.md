# Planning Hygiene Milestone 001 Closure — Current Evidence Baseline and Blocker Reconciliation

Status: closed

Source plan: `plans/implementation/planning-hygiene/001-current-evidence-baseline-and-blocker-reconciliation.md`

Source roadmap: `plans/subsystems/planning-hygiene-roadmap.md`

Eggpack authoring baseline: `cfb32de60fb678d6ad05c388ab178335f16360cf`

## 1. Executive finding

M001 is closed. All ten acceptance criteria are met with an empty production/package/workflow diff.

M001 was a docs/evidence reconciliation pass. It replaced snapshot-era "current evidence" prose with terminal closure references, discharged CI M003b's obsolete external condition, closed Ecosystem M002, and gave Contract M003, Bootstrap M003, and Ecosystem M003 exact current dispositions. Three milestones changed to `closed` and one ordering gate was cleared. No Rust, Cargo, generated workflow, package, release, tag, or external-repository state was modified.

**One §11 stop condition fired during execution and was absorbed rather than re-planned.** See §2. That decision is recorded here in full because it is the single most consequential judgment in this closure, and a later reader should be able to disagree with it.

## 2. The fired stop condition, recorded explicitly

The plan's §11 directed: "Stop and re-plan if StegoEggo `0.4.3` has already published and changes Ecosystem M002 from pending to closable during execution."

During WP1 evidence verification, the following was found first-hand:

- StegoEggo **did not** publish `0.4.3`. It stopped that release pre-publication at `eggstack/stegoeggo@0e5235d3` — "plans: block 0.4.3 release on root API semver findings" — because two stable public enum changes were not semver-compatible.
- It selected `0.5.0` as the next `0.x` minor boundary and **published it on 2026-10-04T06:34:19Z**, from source `57ca94c910269e080b06aa8cb34c3767b3bf669d`.
- That release is a genuine Eggpack-produced ordinary stable release carrying the exact 15-asset inventory, `release-manifest.json`, both Eggpack-generated exact installers, manual publication after inspection, and a real public `0.4.2 -> 0.5.0` updater transition.

So the event the plan named had already occurred, under a different version number. The condition fired in substance. The plan's §4 disposition for F4 — "keep M002 conditionally closed, cite StegoEggo Release-Distribution M004 `0.4.3` as the owner" — was void: the milestone it pointed at was superseded by the consumer's own M005.

**Disposition taken:** rather than closing M001 as stopped and authoring a corrective `M001a`, the maintainer directed that the 0.5.0 reality be absorbed into M001 and closed as a single pass. Recorded consequences:

- M001's closure is therefore a single record covering both the originally-planned reconciliation and the post-0.5.0 reconciliation.
- The originally-planned F4 outcome ("M002 remains conditionally closed pending a future release") was **not** implemented, because it was no longer true. It is superseded here, not silently dropped.
- The two documents a strict reading of §11 would have produced (a stopped-M001 closure plus an `M001a` plan and closure) do not exist. The audit trail is carried by this section instead.
- A reader who prefers the two-document form should treat this as a deviation from planning process §13, accepted knowingly, not an oversight.

No other §11 stop condition fired. Specifically: the eggsact rehearsal did satisfy M003b's exact rerun requirement (§4), consumer evidence revealed no medium-or-higher Eggpack runtime defect (§5), and the reconciliation required no code, workflow, package, or external-repository change (§7).

## 3. Evidence matrix — WP1

All read-only verification performed 2026-10-04. Eggpack evidence is attributed to Eggpack; consumer evidence to the consumer repository.

### 3.1 Eggsact — CI M003b's final condition (F3)

| Fact | Value | Verified by |
|---|---|---|
| M005a implementation | `eggstack/eggsact@f1352101dab748066c788e65e21e1bf303cfe995` | `git log -1` |
| M005a closure | `eggstack/eggsact@685fa3739562a4c3c670c28e05ee9e09b07cdbd2` | `git log -1` |
| Closure record | `eggstack/eggsact: plans/closure/distribution-update-release/005a-status.md` | read in full |
| Independent double build | run `36880110434`, `workflow_dispatch`, head `f135210…`, conclusion `success` | `gh run view` |
| Double-build digest | `dc1eda1f0f6927806db2d25e06360161776869bcbea6eadc2487ad3d09a52175`, 16,606,720 B, from two distinct target dirs | closure record |
| Eggpack pipeline rehearsal | run `36886042696`, `workflow_dispatch`, head `f135210…`, conclusion `success` | `gh run view` |
| Attempt 1 | draft `401132612`, `created: true, uploaded: 15, reused: 0` | closure record |
| Attempt 2 | `created: false, uploaded: 0, reused: 15`, all 15 digests identical, zero refusals | closure record |
| Drift guard | run `36880067370` green, byte match at pin `e5c81f2` | `gh run view` |
| Consumer standard CI | run `36880067269` green | closure record |
| Producer delta | none: no change under `release/eggpack/`, the generated workflow, or `packaging/` | `git show --stat f135210` |

The fix was entirely product-side: target-scoped MSVC `/BREPRO` + `/DEBUG:NONE` in a checked-in `.cargo/config.toml`. **Eggpack implemented nothing and is not credited with the fix.** M003b closes because the producer path behaved correctly against a reproducible consumer.

### 3.2 StegoEggo — Ecosystem M002's live condition (F4)

| Fact | Value | Verified by |
|---|---|---|
| `0.4.3` stop record | `eggstack/stegoeggo@0e5235d3`, blocked on root API semver findings | `git log -1` / commit list |
| M004 plan / registration / handoff | `7b9c72c3`, `8f0503c6`, `a01a0222` | `git log -1` |
| Superseding consumer closure | Release-Distribution M005, `eggstack/stegoeggo@7bde933b`, record `plans/closure/release-distribution/005-status.md` | read in full |
| Release | `v0.5.0`, non-draft, non-prerelease, published 2026-10-04T06:34:19Z | `gh release view` |
| Tag | `v0.5.0` lightweight -> `57ca94c910269e080b06aa8cb34c3767b3bf669d` | `gh api …/git/ref/tags/v0.5.0` |
| Published manifest | `schema_version: 1`, `product_id: stegoeggo`, `release_id: v0.5.0`, `source_revision: 57ca94c9…` — exact tag match | downloaded asset |
| Asset count | 15: 5 binaries, 5 `.sha256`, 2 product wrappers, 2 Eggpack exact installers, 1 `release-manifest.json` | `gh release view --json assets` |
| Eggpack run | `37181914252`, `workflow_dispatch`, head_sha `57ca94c9…`, conclusion `success`, attempt 1 | `gh run view` |
| Staging receipt | `release_id=v0.5.0`, source SHA matched, `uploaded=15, reused=0` | consumer closure |
| crates.io | `stegoeggo-stego`, `stegoeggo`, `stegoeggo-cli` all at `0.5.0` | crates.io API readback |
| Public updater A->B | public macOS x86-64 `0.4.2` binary -> `0.5.0` via production endpoints; no overrides, no Cargo fallback, curl-independent | consumer closure |
| Not exercised | rerun reuse (attempt 1 only); Windows A->B transition | `gh run view`, consumer closure |

### 3.3 Consumer contract-check duplication (F5)

| Repository | File | Duplicated step |
|---|---|---|
| `eggstack/eggsact` | `scripts/check-release-contract.py` (163 lines) | `tomllib.load(distribution.toml)`, then `{product}`/`{target}` expansion compared to a frozen `PUBLIC_ASSETS` table |
| `eggstack/stegoeggo` | `scripts/check-release-contract.py` | same parse + same expansion + same frozen-table comparison |
| `eggstack/stegoeggo` | `scripts/release-check-assets.sh` (Python block, ~line 50) | an independent second expansion of the same contract |

Eggpack already owns the semantics in `DistributionContract::parse_toml_str` and its expansion, but the public CLI exposes only the `eggpack ci ...` surface. So consumers cannot reuse them and re-implement instead. That is the specific, bounded duplication Contract M003 must now decide about.

### 3.4 Eggsearch preflight (F7)

Verified from the current release workflow:

- 7 release targets: Linux x86-64, Linux AArch64, Linux ARMv7, macOS x86-64, macOS AArch64, Windows x86-64, Windows ARM64;
- Linux ARMv7 qualifies through a QEMU runtime step (`armv7_run`), i.e. `Qualification::Emulated`-shaped work;
- glibc `2.17` floors applied as `.2.17` target suffixes on Linux x86-64/AArch64;
- toolchain predates current pins: `mlugg/setup-zig` v2.2.1, `CARGO_ZIGBUILD_VERSION` older than Eggpack's 0.23.3;
- draft assembly uses `gh release upload "$tag" dist/* --clobber` — a direct conflict with Eggpack's fail-closed no-clobber policy;
- installers and the target matrix remain hand-maintained;
- immutable `v0.4.1` release evidence exists.

## 4. Finding-by-finding disposition (F1-F7)

| Finding | Disposition |
|---|---|
| F1 stale Eggsact baseline prose | Fixed. The "blocked / planned … flagged for re-review before implementation" paragraph and the "release authority still duplicated" list are replaced with terminal evidence. Historical SHAs retained explicitly as traceability, marked as discharged. |
| F2 stale `current main@404f63e` / M003h-as-future | Fixed. Replaced with closure-record references (M003h's record captures merge `609d5fb` and the non-forced fast-forward). No new ephemeral `current main` SHA was introduced, deliberately, so this does not re-rot. |
| F3 CI M003b / Phase 8 obsolete blocker | **Discharged and closed.** All eleven §13 criteria mapped and met. Append-only §6 addendum added to the M003b closure record; Phase 8 recorded satisfied. |
| F4 Ecosystem M002 operational blocker | **Closed outright** on the `v0.5.0` evidence. `0.4.3` was superseded by the consumer. See §5 for the item-6 substitution. |
| F5 Contract M003 consumer evidence | **Satisfied.** Roadmap moved from "planned pending consumer evidence" to "research/decision ready", with the duplication identified precisely (2 repos, 3 implementations). No implementation authorized; general CLI framework explicitly not authorized. |
| F6 Bootstrap M003 narrower blocker | **Blocker cleared entirely, not merely narrowed.** The plan anticipated the blocker `StegoEggo M004 0.4.3 live receipt -> M002 full closure -> M003 candidate review`; the live receipt arrived (as `0.5.0`) and M002 closed, so the whole chain is discharged. M003 is unblocked for candidate review. No implementation authored. |
| F7 Ecosystem M003 Eggsearch | **Ordering gate cleared** for research/plan. Preflight facts recorded. Implementation remains unauthorized and must reconcile toolchain pins, Windows ARM64 host mapping, QEMU qualification semantics, and the current `--clobber` behavior. |

## 5. Ecosystem M002 closure and the item-6 substitution

M002's closure record required nine live-evidence items. Eight are met by `v0.5.0` directly. Full mapping is in `plans/closure/ecosystem-adoption/002-status.md` §9.2.

Item 6 — "rerun exact reuse **or** correctly owned fail-closed nondeterminism finding" — was met by **neither** branch: the release was a clean first attempt, so no rerun was needed and no nondeterminism arose.

It is recorded as **substituted, not met**. Item 6's purpose is to prove the producer's rerun-reuse and digest-refusal path works against a real consumer's artifacts, not to force a rerun on a release that did not need one. That path is proven on real consumer bytes by eggsact M005a run `36886042696` attempt 2: same generated code path, same fail-closed refusal, same receipt schema, different consumer's binaries. That evidence independently discharges CI M003b and Phase 8.

Had the rerun path not been independently proven, item 6 would have kept M002 conditionally closed regardless of how clean the `0.5.0` release was. This is called out so the substitution is reviewable rather than buried.

## 6. Files changed

| File | Change |
|---|---|
| `plans/closure/planning-hygiene/001-status.md` | new — this record |
| `plans/subsystems/planning-hygiene-roadmap.md` | M001 `ready` -> `closed`; current state rewritten to terminal evidence |
| `plans/closure/ci-release-orchestration/003b-status.md` | status -> `closed`; append-only §6 condition-discharge addendum (original sections preserved verbatim) |
| `plans/implementation/ci-release-orchestration/003b-...md` | status -> `closed`; new dated annotation prepended, 2026-10-01 and 2026-09-25 annotations preserved |
| `plans/closure/ecosystem-adoption/002-status.md` | status -> `closed`; §9 live evidence + item-6 substitution (original §1-§8 preserved verbatim) |
| `plans/implementation/ecosystem-adoption/002-...md` | status -> `closed`; dated annotation added |
| `plans/subsystems/ci-release-orchestration-roadmap.md` | M003b -> `closed`; Phase 8 -> satisfied; M003c sequencing note updated |
| `plans/subsystems/ecosystem-adoption-roadmap.md` | dependency graph updated; M002 -> `closed`; M003 -> unblocked for research/plan with preflight facts; M004-M007 dispositions made explicit; stale pre-adoption eggsact prose replaced |
| `plans/subsystems/contract-conformance-roadmap.md` | M003 -> research/decision ready with identified duplication; completion definition and status table updated |
| `plans/subsystems/bootstrap-installers-roadmap.md` | M003 -> unblocked for candidate review with two-consumer evidence |
| `plans/registry.md` | baselines refreshed; new "Consumer live release evidence" section; subsystem table, dependency-ready table, blocked/planned table, execution graph, next handoff, and unblock disposition table all reconciled |
| `AGENTS.md` | next handoff updated (its previous summary became false once M001 closed) |

Deliberately **not** changed:

- `plans/000-long-term-specification.md`, `plans/001-terminology-and-domain-model.md`, `plans/002-long-term-roadmap.md`. Phase 8 and Phase 9 exit criteria are now objectively satisfied in fact, but those documents state requirements rather than live status, so nothing in them became false. Editing them is outside M001's declared §6 scope and would require the §14 canonical-document change control. Status lives in the registry.

## 7. Verification

Scope proof — the production/package/workflow diff is empty:

```text
git diff --check                                                    clean
git status --short                                                  docs-only paths
git diff <implementation-parent>..HEAD -- crates/ Cargo.toml Cargo.lock .github/   (empty)
```

Consistency sweep (WP5), run against active planning after the edits:

| Stale-text check | Result |
|---|---|
| Eggsact M005a described as open | no remaining active-planning instance |
| CI M003b / Phase 8 described as blocked on Windows nondeterminism | no remaining instance; both now record discharge |
| Ecosystem M002 without its concrete live owner | no remaining instance; owner is now the closed `v0.5.0` evidence |
| Contract M003 described as lacking first-consumer evidence | no remaining instance |
| Bootstrap M003 presented as generally blocked | no remaining instance |
| `current main@404f63e` or `blocked / planned` eggsact snapshot text | no remaining instance |

Hosted CI is incidental for a docs-only pass. It is not treated as runtime qualification evidence and no qualification claim rests on it. No production artifact changed, so no re-qualification was required or performed.

## 8. Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| Medium (process) | §11 stop condition fired and was absorbed into a single M001 pass rather than re-planned into a corrective | Accepted by explicit maintainer decision. Recorded in §2 with its consequences so the choice is auditable. A reader preferring the two-document form can treat this as a known deviation from planning process §13. |
| Info | M002 item 6 was not exercised and is substituted by eggsact evidence | Recorded explicitly in both closure records; not presented as a pass. |
| Info | StegoEggo's `v0.5.0` line sits on `release/0.5.0` (`7bde933b`), 8 commits ahead of consumer `main` (`a01a022`), unmerged | Consumer-side housekeeping, not an Eggpack blocker. Tag, manifest `source_revision`, and run `head_sha` all agree on `57ca94c9…`. |
| Info | The eggsact rehearsal draft `401132612` and tag `m005a-rehearsal-1` were deleted after the rehearsal | Correct consumer hygiene. The remote objects are no longer inspectable; the evidence of record is the run logs plus the consumer closure record. |
| Info | Windows A->B updater transition never performed on either consumer | Accepted; the requirement was one supported target. No Windows transition is claimed. |
| None | No medium-or-higher Eggpack producer defect was revealed by any consumer evidence | Confirmed. |

## 9. Acceptance criteria

| # | Criterion | Status |
|---|---|---|
| 1 | current evidence baselines no longer present pre-M001/pre-M003h snapshots as current | met — F1, F2 |
| 2 | CI M003b's external condition formally discharged from exact eggsact evidence | met — F3, §3.1 |
| 3 | Phase 8 status agrees with M003b | met — both recorded satisfied |
| 4 | Ecosystem M002 points at a concrete live owner and remains accurate about the public event | met, with the outcome changed by evidence: it is closed on `0.5.0`, not pending on a future `0.4.3` |
| 5 | Contract M003 records consumer evidence as satisfied plus a bounded research/decision disposition | met — F5 |
| 6 | Bootstrap M003 retains only the outstanding-evidence blocker | met — F6; the blocker is fully cleared rather than narrowed |
| 7 | Ecosystem M003 records Eggsearch as the next ordered adoption with implementation still blocked | met — F7, unblocked for research/plan only |
| 8 | later adoption/provenance/package-adapter phases remain sequenced and unauthorized | met — M004-M007, Phase 12/13/14 all explicitly retained and unauthorized |
| 9 | no production/package/workflow/external-repository mutation | met — §7 empty diff |
| 10 | this record states exact evidence and remaining work | met — §3, §8 |

## 10. Next handoff

No Eggpack implementation milestone is `ready`. The next three items are research/plan decisions, in this order:

1. **Contract M003** — smallest and least blocked. Author a bounded plan for a local contract parse/expand/inspection surface, justified by duplication across 2 repositories and 3 implementations. Do not generalize into a CLI framework.
2. **Bootstrap M003** — two-consumer evidence satisfied; candidate review, then a plan if warranted.
3. **Ecosystem M003 eggsearch** — preflight research now, implementation later. Reconcile older Zig/cargo-zigbuild pins, Windows ARM64 runner/host mapping, QEMU qualification semantics, and the current `--clobber` draft-rerun behavior against Eggpack's fail-closed staging policy. Do not weaken the no-clobber policy to mirror Eggsearch.

Longer-horizon: Ecosystem M004-M007 behind Eggsearch; Phase 12 provenance behind a dedicated trust ADR (Eggsearch's Artifact Attestation release is prior art, not a trust-model selection); Phase 13 package adapters behind broader native adoption; Phase 14 stabilization premature.

No new planning-hygiene milestone is registered. This line is complete and reopens on new drift or new cross-repository evidence.
