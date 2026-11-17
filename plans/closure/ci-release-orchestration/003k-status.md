# CI Release Orchestration M003k — Closure

Status: **closed**

Source plan: `plans/implementation/ci-release-orchestration/003k-precheckout-tag-resolution-and-stage-environment.md`

Roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Reviewed Eggpack baseline: `559d940af0fe6a2951eb17de1fcbecbf9e0bb6ce` (CI M003j).

Implementation commits: `c4c4bb8649fac812b0ea5d27411dd89c1951c991` (resolve tags before checkout), `23d9f0347d89fd6ecd916692c145152e39de687f` (require tags to peel to commits), and `fb81aa1400e1f333f6500075f78d944a9242b673` (final qualification record). PR `eggstack/eggpack#1` merged at `1103a1affc62a210f1102a769457db80d9188bc8`.

Downstream consumer baseline: `dbowm91/wg-basic@bd7c085d191ccf147e0f4dab72fde7cc8818f3fd`. Downstream strict closure: `dbowm91/wg-basic@1e034d089ca62d9f4c22a562a3067227ad8beb9d`, Release Readiness R001.

Hosted producer CI: [run 38027470046](https://github.com/eggstack/eggpack/actions/runs/38027470046), pull request, attempt 1, exact final implementation head `fb81aa1400e1f333f6500075f78d944a9242b673`, conclusion `success`. All four lanes passed: Linux stable, Linux 1.89.0, macOS, and Windows.

## Executive finding

CI M003k is closed. Generated workflows validate and resolve the selected tag to a commit before checking out release source, use the frozen commit for later source checkouts, and verify checked-out `HEAD` before producer or product commands run. The generated GitHub Environment is optional and is limited to the write-authorized `stage` job. The downstream wg-basic consumer repinned M003k, regenerated its checked-in workflow with an exact `eggpack ci check` match, configured its maintainer-controlled stage environment, and closed R001 with exact-head hosted CI and release-contract evidence.

The live tagged-stage preflight was not exercised because wg-basic had no release tags at closure time. Its strict R001 closure explicitly deferred that operation to the separately gated R004 release. No release was staged, signed, published, or mutated as part of M003k.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Validate and resolve the selected tag before any source checkout | M003k implementation commits `c4c4bb8` and `23d9f03`; hosted run `38027470046`; downstream generated-workflow security tests at wg-basic R001 closure | Pass |
| Resolve only an existing tag that peels to a commit, then bind checkouts to that immutable OID | `23d9f0347d89fd6ecd916692c145152e39de687f`; consumer R001 closure at `1e034d0` records tag-object validation for annotated/lightweight tags, changed/non-commit objects, malformed input, and injection rejection | Pass |
| Recheck checkout identity before Eggpack or product release commands | Frozen-SHA checkout and HEAD verification are present in the M003k-generated workflow; consumer exact-head workflow drift check and hosted CI passed | Pass |
| Keep the optional stage environment scoped to the stage job | Producer renderer policy change; downstream R001 records the sole `contents: write` grant on `stage` and the `release-staging` environment with maintainer reviewer `dbowm91` | Pass |
| Preserve compatibility when no environment is configured and keep release semantics unchanged | Producer test suite and four hosted lanes passed; no `ReleaseManifest` or runtime handoff schema change; draft-only staging and no-publication boundary preserved | Pass |
| Qualify downstream release-contract, native candidate, and systemd lifecycle lanes | wg-basic hosted Distribution Foundation run `38108270368` at exact consumer head `1e034d0`; both GNU native candidates and both systemd install-lifecycle lanes passed | Pass |

## Production implementation evidence

- The reusable workflow resolves and validates the tag without first checking out the selected source. Later source-consuming jobs check out the resolved OID and verify `HEAD` before invoking Eggpack or product commands.
- The provider policy accepts an optional staging environment name and renders it only on `stage`; Eggpack does not create the environment or choose its reviewers.
- PR `eggstack/eggpack#1` contains the implementation and qualification commits and merged to `main` at `1103a1affc62a210f1102a769457db80d9188bc8`.
- wg-basic pinned the producer, regenerated `.github/workflows/release-eggpack.yml`, and recorded `eggpack ci check: match` at 40,658 bytes. Its strict R001 closure records read-only GitHub settings evidence and the named stage reviewer configuration.
- The public manifest schema, producer/consumer authority boundary, exact-tag draft staging, and separate publication decision did not change.

## Exact verification executed

| Command or evidence | Result |
|---|---|
| `scripts/check-local.sh` | Recorded as passing on 2026-10-10 in the M003k implementation plan and final PR qualification commit |
| Hosted Eggpack CI run `38027143275` | Passed at intermediate head `23d9f0347d89fd6ecd916692c145152e39de687f` |
| Hosted Eggpack CI run `38027470046` | Passed on final PR head `fb81aa1400e1f333f6500075f78d944a9242b673` across Linux stable, Linux 1.89.0, macOS, and Windows |
| PR `eggstack/eggpack#1` | Merged at `1103a1affc62a210f1102a769457db80d9188bc8` |
| wg-basic regenerated-workflow `eggpack ci check` | Exact match at 40,658 bytes at the downstream M003k pin |
| wg-basic hosted CI run `38108270370` | Exact-head consumer CI success at `1e034d089ca62d9f4c22a562a3067227ad8beb9d`; 21 checks passed |
| wg-basic hosted Distribution Foundation run `38108270368` | Exact-head release-contract, GNU native-candidate, and systemd install-lifecycle jobs passed |
| wg-basic R001 closure | Closed at `1e034d089ca62d9f4c22a562a3067227ad8beb9d`; [closure record](https://github.com/dbowm91/wg-basic/blob/1e034d089ca62d9f4c22a562a3067227ad8beb9d/plans/closure/release-readiness/001-status.md) |

The M003k PR has no formal GitHub review object recorded. The merge, exact-head producer CI, downstream generation, repository-policy evidence, and exact-head consumer CI are recorded above; no separate reviewer approval is claimed.

## Invariant review

- Dispatch input remains bounded data, not shell source; tag validation precedes source checkout.
- Every source-consuming job uses the resolved immutable commit and checks its checkout identity before release commands.
- The optional environment reference appears only on the stage job. Reviewer identity and GitHub Environment configuration remain consumer-owned.
- Build jobs remain read-only, staging remains draft-only on the exact existing tag, and publication remains a separate human action.
- `ReleasePlan`, `ReleaseManifest`, and Eggup receipts retain their distinct authority; no public schema changed.

Nothing was weakened to reach the milestone.

## Failure, recovery, compatibility, and security review

Missing, malformed, ambiguous, moved, or non-commit refs fail before release work or draft mutation. A checkout mismatch fails before Eggpack or product commands execute. If the configured reviewer gate is not approved, the stage job does not run and cannot mutate a draft. Omitting the environment preserves the old policy form.

The consumer R001 strict closure records the configured `release-staging` environment and its single maintainer reviewer. The same record states that no live `v*` tag existed, so the live tagged-stage preflight was deferred to R004. That release operation is outside M003k and remains separately gated; M003k does not claim a live stage, signing, or publication.

## Documentation and operations evidence

The CI renderer roadmap and registry now link this closure and identify the exact producer merge, final hosted run, and downstream R001 closure. The downstream plan and closure remain in wg-basic; no external repository was edited in this Eggpack closeout.

## Unresolved findings

No unresolved Eggpack implementation finding remains for M003k. The live tagged-stage preflight, production signing, and publication were not exercised and remain downstream R004 operations. No independent GitHub review object was recorded; this evidence limitation is stated rather than inferred away.

## Roadmap disposition and registry updates

- CI M003k: **closed**. Pre-checkout tag resolution and stage-environment rendering are qualified at the merged producer revision.
- wg-basic Release Readiness R001: **closed** at its own repository baseline with the configured reviewer environment and exact-head CI evidence; the first live tagged-stage preflight remains assigned to R004.
- No additional Eggpack milestone became ready as a result of M003k. The remaining registry entries are blocked or planned, not eligible for implementation.
