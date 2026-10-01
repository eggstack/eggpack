# CI and Release Orchestration Milestone 003h Closure — Live Status Reconciliation and Main Integration

Status: closed

Source plan: `plans/implementation/ci-release-orchestration/003h-live-qualification-status-reconciliation-and-main-integration.md`

Roadmaps: `plans/subsystems/ci-release-orchestration-roadmap.md`; `plans/subsystems/ecosystem-adoption-roadmap.md`

## Reviewed baselines and history preservation

- Current `main` before synchronization: `404f63ec2bae119f7fa1a41a56a34e91bc267b1a`.
- Candidate before synchronization: `a60a90346b9d781957283cfa9973a10db4611c24`.
- Actual pre-merge compare: candidate was 16 commits ahead and 3 behind `main`. The plan's earlier registration snapshot (`1b2c519`) was 13 ahead / 3 behind; subsequent candidate commits only rebaselined M003h and its registration.
- The three commits only on then-current `main` were planning-only and are preserved exactly:
  - `b9baa93a93264075801cdc6f5c0c60d5207e99cf` — distinguish Eggwork producer adoption from Eggup Interop M003;
  - `3d1cb67e3ac04d3064b4307d9b15540e8353fe37` — record external Eggwork producer adoption boundary;
  - `404f63ec2bae119f7fa1a41a56a34e91bc267b1a` — record Eggwork producer-only external adoption.
- Synchronization merge: `609d5fb` (`merge: preserve current main planning updates`), with `404f63ec` as first parent and `a60a903` as second parent. No rebase, squash, or force update was used.
- The final integrated candidate remains a strict descendant of `404f63ec`; hosted CI head, final candidate SHA, and non-forced `main` ref movement are recorded below after closeout.
- During final closeout, `main` advanced through two additional planning-only commits: `caed0c08c9e9f915fda85074c592e3d9aef3ba33` (Eggup M003 producer-gate reconciliation) and `8dafb919e0422e2c72e20e793102ebeb73fa5bca` (Eggup runtime handoff registry update). They change only `plans/subsystems/eggup-interoperability-roadmap.md` and `plans/registry.md` and are preserved by final synchronization merge `d050e2b77813c8db2947de648737fb199cb4eadf` (parents `d1ea63d` and `8dafb91`).

Qualified implementation and closure identities retained exactly:

| Milestone | Implementation commit(s) | Closure commit |
|---|---|---|
| CI M003e | `b9062d48498a5d3511f3651e385bb97df1ec0621` | `04cc95dd79b8c12fb2bc7e28d22ff62c3236ba82` |
| CI M003f | `c190e7740ac94421bd880c9d239b91b5c958b32b` | `8d1701a331bd40626ac33949b4da4b11a5794586` |
| CI M003g | `5acda73e126fd29dbf7ff47938f7bbb1abd5785f`, `4b28820efeaab308cd3722c969695a9f3d5e4992`, `5ac5b83a0f47c453212b09fc2c744065f3dfaf8e`, `e5c81f28bd328d4aea41c3f061a0ed9944306262` | `8507fbeebc6e6a0f8176965d8b21dfc818a03719` |
| Ecosystem M001 | consumer implementation evidence in eggsact at `v1.2.7` | `8507fbeebc6e6a0f8176965d8b21dfc818a03719` |

Closure bodies for M003e/M003f/M003g and Ecosystem M001 were not changed.

## Status contradiction matrix

| Surface | Contradiction found in the reviewed candidate | Corrected current state |
|---|---|---|
| `plans/registry.md` active subsystem summary | M003g listed active; M003h called ready; Ecosystem M001 listed ready/unblocked | M003g closed; M003h active during closeout; Ecosystem M001 closed |
| Registry dependency-ready work | M003g still described F9/F10 as unresolved | M003g row points to its closure and live 15-asset evidence |
| Registry planned/blocked work | M003b live draft shown outstanding; M003e asks for a future M001 re-pin; Ecosystem M002 absent/blocked pending M001 | M003b's sole outstanding condition is exact rerun reuse; M003e's re-pin is historical; Ecosystem M002 is ready to plan |
| Registry evidence narrative / execution graph | M001 described as still ready; first live draft not represented | M001 closed from eggsact `v1.2.7`; graph marks M001 closed and isolates M003b rerun reuse under eggsact M005a |
| Registry next handoff / downstream disposition | M001 said not closed and M002 blocked solely on M001 closure | M001 closed; M002 ready to plan; Bootstrap M003 and Eggup Interop M003 retain their independent dispositions |
| CI roadmap narrative | M003b live draft still outstanding behind M001 implementation/tag | Live `v1.2.7` draft, inventory, draft-only behavior, and refusal evidence recorded; only exact rerun reuse remains |
| CI roadmap table | M003b condition and M003h status stale | M003b condition is consumer Windows byte reproducibility; M003h is active while integrating |
| Ecosystem roadmap dependency/current-state narrative | M001 returned to ready pending re-review/re-pin and live tag | Historical stop/restart remains traceable; current M001 is closed on live `v1.2.7` evidence |
| Ecosystem roadmap table | M002 blocked on M001 closure | M002 is ready to plan from first-consumer evidence |
| Registry and Eggup Interoperability roadmap | M003 described as ready to plan with consumer selection outstanding | Eggsact is already selected and its Eggup-owned plan is registered; M003 is ready to resume after the producer gate closed. Runtime adoption remains Eggup/Eggsact-owned and open. |
| M003b implementation-plan annotation | no current explanation of the changed live evidence | Dated 2026-10-01 annotation records live draft success and exact rerun condition while preserving the original 2026-09-25 historical close |

## Live consumer evidence and dispositions

Eggsact `v1.2.7` tag at consumer commit `d8014cf`; generated live run `36652731202`; attempt 1 created draft `eggsact v1.2.7` with all 15 assets (receipt `RE_kwDOTGg0Mc4X0gk6`), later published by the maintainer. Attempt 2 reused the draft and matching assets but correctly failed closed on the different Windows asset digest. That byte-reproducibility issue is owned by eggsact M005a. The complete live history and inventory are retained in `plans/closure/ci-release-orchestration/003g-status.md` and `plans/closure/ecosystem-adoption/001-status.md`.

| Milestone / condition | Disposition |
|---|---|
| Build/Qualification M001-M006 | closed |
| CI M003e/M003f/M003g | closed; existing closure evidence retained |
| Ecosystem M001 | closed; consumer M005 closure retained |
| CI M003b | conditionally closed; exact rerun reuse is the sole outstanding condition |
| Phase 8 | not exited; blocked only on exact rerun reuse / eggsact M005a Windows byte reproducibility |
| Ecosystem M002 stegoeggo | ready to plan |
| Bootstrap M003 | remains blocked on independent adoption evidence/candidate review |
| Eggup Interoperability M003 | independently ready to resume in Eggup/Eggsact under its registered consumer plan; producer gate is met, runtime adoption remains open |
| Build M006 glibc compatibility floor | no new claim; independent glibc floor proof remains outside M006 |

## M003h changes and scope

M003h changes are documentation-only:

- `plans/registry.md`;
- `plans/subsystems/ci-release-orchestration-roadmap.md`;
- `plans/subsystems/ecosystem-adoption-roadmap.md`;
- `plans/implementation/ci-release-orchestration/003b-generated-draft-staging-job-and-operational-qualification.md` (dated current-condition annotation);
- this closure record.

The synchronization merges also carry only planning changes from the two sets of reviewed current-main commits. No Rust source, Cargo manifest/lockfile, workflow, fixture/golden, or script is changed by M003h.

## Verification

Required repository verification on the integrated candidate:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `cargo test --workspace --all-targets --all-features --locked` | passed: 219 passed, 7 ignored |
| `cargo doc --workspace --no-deps --locked` | passed |
| `cargo +1.89.0 check --workspace --all-targets --locked` | passed |
| `./scripts/check-local.sh` | passed (exit 0; all workspace package and test checks completed) |
| `git diff --check` | passed after final history synchronization and closure updates |
| stale-state phrase scan | passed on current-state registry and adoption roadmap summaries; historic descriptions are explicitly historical |

Hosted CI on integrated candidate `3ed946e4779f00f437d751ef567954e63291452a` passed all repository lanes in run `36865702333`: Linux stable, Linux 1.89.0, macOS, and Windows. Main CI run `36866156168` also passed on that SHA after the initial fast-forward. Closure-evidence candidate `57c150f34ddffea34dac03b5fc0a9a0c956b2865` passed candidate-tip CI `36866761493` and post-fast-forward main CI `36867295046`, each with all four lanes green. After `main` advanced with the two planning-only Eggup handoff commits, candidate merge `d050e2b77813c8db2947de648737fb199cb4eadf` preserved them. The fully synchronized candidate `f237d8d4d4e5bfa77fb95005118daf6785c38e60` then passed candidate CI `36869038147` and main CI `36869503841`; all four lanes passed on both runs.

## Integration and ref-move evidence

- Final integrated code/status candidate after the later planning-only main advance: `f237d8d4d4e5bfa77fb95005118daf6785c38e60`; hosted run `36869038147` passed all repository lanes.
- Before updating `main`, `origin/main` was `404f63ec2bae119f7fa1a41a56a34e91bc267b1a`, candidate was `3ed946e4779f00f437d751ef567954e63291452a`, candidate was a strict descendant, and the compare was `0` commits behind / `18` ahead.
- Non-forced fast-forward push: `404f63e..3ed946e main -> main`.
- Post-update fetch: `origin/main` and `origin/m003g-live-qualification` both resolved to `3ed946e4779f00f437d751ef567954e63291452a`; compare `0/0`.
- Main CI run `36866156168` on that exact SHA: passed, all four lanes green.
- Closure-evidence commit `57c150f34ddffea34dac03b5fc0a9a0c956b2865` was separately checked: candidate run `36866761493` passed all four lanes; after the final non-forced fast-forward, main run `36867295046` passed all four lanes on that exact SHA.
- Before the final integration ref move after re-review, `origin/main` was `8dafb919e0422e2c72e20e793102ebeb73fa5bca`, candidate was `f237d8d4d4e5bfa77fb95005118daf6785c38e60`, the candidate was a strict descendant, and the compare was `0` behind / `3` ahead. Non-forced fast-forward: `8dafb91..f237d8d main -> main`. Post-update fetch showed both refs at `f237d8d4d4e5bfa77fb95005118daf6785c38e60`, compare `0/0`; main run `36869503841` passed all four lanes.
- The final closure record follow-up is documentation-only and remains a descendant of that verified integration tip.
- Eggup interoperability M003's status was re-reviewed on those commits: Eggsact is already selected and the Eggup-owned plan is registered; the producer gate is now met. M003 is ready to resume in Eggup/Eggsact after baseline refresh. No Eggpack implementation plan is newly unblocked by this consumer-owned runtime work. Ecosystem M002 remains ready to plan as the next ecosystem handoff.

## Invariant, recovery, compatibility, and security review

- M003e/M003f/M003g producer commits and closure records retain their exact identities; no qualified producer history was rewritten.
- The three current-main Eggwork planning commits remain individually addressable. The ordinary merge preserves both parents.
- No production behavior, schema, publication authority, or ownership boundary changed.
- Draft staging remains draft-only; same-name/different-digest assets remain a hard refusal. No Windows reproducibility exception was introduced.
- Only staging retains release write permission. The M003g diagnostic and credential-redaction conclusions remain unchanged.
- No unresolved Eggpack production finding was discovered by this planning reconciliation. The Windows mismatch remains consumer-owned by eggsact M005a; Phase 8 stays open.

## Roadmap and registry disposition

M003h is closed. M003b remains conditionally closed and Phase 8 remains open on exact rerun reuse. Ecosystem M002 (stegoeggo) is ready to plan; separately, Eggup M003 is ready to resume under its already-registered Eggup-owned plan now that its producer gate is met. Bootstrap M003 remains blocked on independent adoption evidence/candidate review.
