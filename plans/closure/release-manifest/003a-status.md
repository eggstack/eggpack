# Release Manifest Milestone 003a Closure — Post-Publication Downstream Closure Reconciliation

Status: closed

Source plan: `plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md`

Roadmap: `plans/subsystems/release-manifest-roadmap.md`

Predecessor closure: `plans/closure/release-manifest/003-status.md` (M003, remains closed)

Plan authoring baseline: `64cc8453a572651f8f935add42a91d426d035f06`

Reconciliation commit: `a0cd5ebb4c167c27f1a967be3e342cfc6bcab0b5`

Reconciliation-plan status commit: this record's own commit (plan header advanced `ready` -> `closed` with a `Closure record:` pointer, matching the M001/M001a/M002/M003 plan convention)

No production source, package, dependency, lockfile, workflow, release, tag, or registry change was made. This milestone is planning reconciliation only. `eggpack-manifest 0.1.0` remains exactly as M003 published it.

## Executive finding

Eggup consumed the M003 handoff and completed the registry promotion that was waiting on it. Eggpack's active planning surfaces now say so, and the M003 closure carries an appended, separately identified downstream-receipt addendum. The producer/consumer package seam is closed end to end, and no Eggpack work remains on it.

The drift this milestone corrected was real but documentation-only: `plans/subsystems/eggup-interoperability-roadmap.md` still asserted in the present tense that `eggup-acquisition` and `eggup-eggfetch` were "still at `0.1.1`", that `eggup-eggpack` was "still unpublished", and that "that chain is demonstrably unfinished". All three became false when Eggup published on 2026-10-03. They were accurate when written and were preserved as history in the M003 closure; only the *active* surfaces were corrected.

## External evidence revalidated (WP1)

Revalidated read-only before editing, per planning process §2 and §9. Eggpack did not modify any external repository.

| Item | Value | Revalidation result |
|---|---|---|
| Eggpack M003 closure on `main` | `48ed13c53140fd2d88e3b3b8977ce8371d92d2f8` | ancestor of `main` — present |
| Eggpack M003 handoff record on `main` | `64cc8453a572651f8f935add42a91d426d035f06` (plan authoring baseline) | ancestor of `main` — present |
| Eggup M004 closure | `eggstack/eggup@ea1f1c5e29302e5feca4599db9342d3a5ac93915` — "plans: close M004 with the 0.1.2 registry publication and promotion evidence" (2026-10-03T02:43:53Z) | reachable on Eggup `main`; `main` is 1 commit ahead of it, 0 behind |
| Eggup publication source | `eggstack/eggup@02a1d32931be29cc3d8980833643b2cd822f2d28` (2026-10-03T02:36:31Z) | present |
| Eggup formal status reconciliation | `eggstack/eggup@3b82d5397e728649a666690868a1e2d0fe42460d` — "plans: formally close M004 plan status and reconcile stale blocked-state prose" (2026-10-03T02:58:18Z) | **is** Eggup `main`'s current tip; no Eggup commit exists after it |
| Eggup hosted qualification | run [`37090397398`](https://github.com/eggstack/eggup/actions/runs/37090397398) on `02a1d32` | `conclusion: success`; `Stable checks`, `MSRV check`, `macOS tests`, and `Windows archive, acquisition, and service tests and check` all passed |
| Eggup M004 closure record | `eggstack/eggup: plans/closure/eggpack-manifest-interoperability/004-status.md` at `3b82d53` | present (reviewed; 302 lines) |
| `eggup-acquisition 0.1.2` | checksum `0b01deb80e7b67a73348eb8fe9d8f55db4188e3fd10435b1828f98b96f6ef170`, published 2026-10-03T02:38:50Z, `yanked = false` | confirmed on crates.io; supersedes `0.1.1` (`2bf3853d…`), which stays non-yanked |
| `eggup-eggfetch 0.1.2` | checksum `2e48315263c6d2419e35d201007e420f6414f72e42c4e0233e834c973d8f3528`, published 2026-10-03T02:39:10Z, `yanked = false` | confirmed on crates.io; supersedes `0.1.1` (`0ea6c111…`) |
| `eggup-eggpack 0.1.2` | checksum `9dbfdfb74b4faa88d6badaa6f24e71dada3c7322dd0e9877cebe8b0168b13fef`, published 2026-10-03T02:39:29Z, `yanked = false` | confirmed on crates.io; **first** publication — the crate previously had no version at all |
| `eggpack-manifest 0.1.0` (M003's own identity) | checksum `2a08f24b05e9652878dd49145cdc3cbd38c7a76032d7b01a5fe1535d9446b629`, published 2026-10-02T21:13:36Z, `yanked = false`, only version published | re-confirmed unchanged and non-yanked; the M003 identity still holds |

Eggup has not reopened or materially revised M004 after `3b82d53` — that commit is the branch tip — so plan §8 WP1's stop condition was not reached and this plan was applied as written.

### Independent confirmation of the producer edge

Rather than relying on Eggup's own claim, the published `eggup-eggpack 0.1.2` `.crate` was fetched from crates.io and its normalized manifest inspected directly:

```text
sha256sum eggup-eggpack-0.1.2.crate
# 9dbfdfb74b4faa88d6badaa6f24e71dada3c7322dd0e9877cebe8b0168b13fef  (equals the registry checksum)

[dependencies.eggpack-manifest]
version = "=0.1.0"
```

No `git` or `path` source is present, confirming Eggup's M004a compatibility-incident branch was not triggered and that Eggpack's M003 byte-identity finding held in practice. The temporary download was made outside the repository and is not part of the workspace.

## Stale-state inventory and disposition (WP2/WP5)

Swept with the plan's §10 pattern plus a broader present-tense sweep over every active planning surface (`plans/registry.md`, `plans/000`–`003`, `plans/README.md`, `plans/subsystems/*.md`, `README.md`, `AGENTS.md`, `docs/`, `architecture/`). Historical closure and implementation-plan prose was read but not rewritten, per plan §8 WP4 and planning process §8.

| # | Location | Stale claim as found | Disposition |
|---|---|---|---|
| 1 | `plans/subsystems/eggup-interoperability-roadmap.md:57` | "`eggup-acquisition` and `eggup-eggfetch` are still at `0.1.1` and `eggup-eggpack` is still unpublished, so that chain is demonstrably unfinished" | **Rewritten.** Now records the completed chain, the publication order, the full Eggup commits, run `37090397398`, the closure-record path, and that the work is Eggup's. |
| 2 | `plans/subsystems/eggup-interoperability-roadmap.md:117` | "Eggup can execute its registry-only promotion sequence under its own authorization rules" (present-tense future) | **Rewritten.** Records that Eggup has already executed and closed it; seam closed end to end; no Eggup milestone is blocked on Eggpack; Eggsact migration is downstream and not an Eggpack item. |
| 3 | `plans/subsystems/eggup-interoperability-roadmap.md:95, 97` | "M004 bootstrap receipt compatibility only if justified. Eggup M004 registry promotion remains Eggup-owned." — omitted that Eggup M004 had closed, and conflated the Eggpack placeholder with the downstream milestone | **Split into two paragraphs.** Eggup M004 is now recorded as closed downstream with its evidence; Eggpack's own bootstrap-receipt placeholder is preserved as `planned only if justified` and explicitly not repurposed (plan §5.3). |
| 4 | `plans/subsystems/eggup-interoperability-roadmap.md:125` (M004 table row) | "Eggup-owned; not Eggpack's to close … remaining gate is Eggup's own `0.1.2` publication chain"; also pointed at the `004a-*` preflight plan/closure rather than the M004 plan/closure that actually closed | **Rewritten.** Now cites the real `004-registry-package-api-promotion-and-publication.md` plan and `004-status.md` closure, with the closure commit, publication source, status cleanup, and run; blockers column reads "none remaining". |
| 5 | `plans/subsystems/release-manifest-roadmap.md:72` | "Eggup M004 closed … Eggpack still contains pre-M004 present-tense planning text, so M003a is registered…" | **Rewritten** as the confirmed downstream consumption record, and to state that M003a is closed and no further Release Manifest implementation is implied. |
| 6 | `plans/subsystems/release-manifest-roadmap.md` (dependency graph, M003a §7, status table) | M003a `READY; DOCS ONLY` / `ready` / `—` closure | **Closed** with the closure-record pointer. |
| 7 | `plans/registry.md:80, 84, 97, 224, 249–251, 270` | M003a `ready`/`registered to perform`; short SHAs `ea1f1c5`/`3b82d53`; no publication-source/run; execution graph `READY` | **Rewritten** to closed, with full 40-char external SHAs, publication source, and run. |
| 8 | `plans/registry.md:87` | "That Eggup-owned chain subsequently completed … Eggpack M003a is registered solely to reconcile…" | **Rewritten** to state the seam is closed end to end, that no implementation work is unlocked, and that Eggsact's migration is the next downstream action and is not Eggpack-owned. |
| 9 | `plans/registry.md:91` | "Current dependency-ready implementation work:" above a table whose only open row was M003a | **Replaced** with an explicit "No Eggpack milestone is currently dependency-ready" statement, so the heading cannot be read as an open work queue. |
| 10 | `plans/registry.md:268–270` (downstream unblock disposition) | Eggup M004 row used short SHAs and still read as a residual gate; M003a row read `Ready`; no row existed for Eggsact's migration | **Rewritten/added.** M004 row is the closed-downstream fact with full evidence; a new row records Eggsact's migration as not-Eggpack-owned; M003a row is closed. |
| 11 | `plans/closure/release-manifest/003-status.md:98, 235, 294, 299–315` | "Eggup's own `0.1.2` publication chain remains Eggup-owned and unfinished"; "`eggup-eggpack` is not yet published by Eggup"; "currently 0.1.1"; the `unpublished — crate absent` diagram | **Preserved unchanged.** These were true at M003 closure. A dated, separately headed addendum was appended recording the later completion and the chronology distinction. |
| 12 | `AGENTS.md:51` | "Next handoff (per registry): none pending … Release Manifest M003 is closed" — accurate but did not mention that Eggup had since consumed the handoff or that M003a closed | **Updated** to record the downstream consumption, `ea1f1c5`/`3b82d53`, and M003a's closure. The "no Eggpack work" conclusion is unchanged. |

### Matches reviewed and deliberately not changed

The plan's §10 search pattern still matches six lines after the sweep. Each was reviewed individually; none asserts a false present-tense state.

- `plans/registry.md:52`, `:110`, `:139` — the "remains blocked" alternative fires on **CI M003b exact rerun reuse** and eggsact M005a Windows PE/PDB byte nondeterminism. That condition is real, still open, and unrelated to Eggup M004. Untouched.
- `plans/subsystems/eggup-interoperability-roadmap.md:57`, `:117` — the `remains .*blocked` alternative greedily spans from "No Eggpack work **remains**…" to the word "**blocked**" appearing later inside a *correct* sentence ("reconciled Eggup's own stale blocked-state prose"; "no Eggup milestone is blocked on Eggpack"). Confirmed by extracting the matched span. Regex false positives, not stale claims.

Not stale and not in scope:

- `plans/subsystems/contract-conformance-roadmap.md:159` and `plans/002-long-term-roadmap.md:85` — "Eggup M004 retirement is unblocked" refers to **Eggup distribution M004** (`eggup-dist` retirement), a different and already-closed milestone. Untouched.
- `architecture/eggup-manifest-consumer-v1.md` — an interface note pinned to `eggstack/eggup@2cab1f9`; it makes no publication-status claim. Untouched.
- `plans/README.md:94` — the same distribution-M004 retirement history, already accurate. Untouched.
- `docs/quickstart.md`, `README.md` — no publication or M004 status claims. Untouched.

## Files changed

```text
plans/registry.md                                                              | 17 ++++-------
plans/subsystems/release-manifest-roadmap.md                                   | 16 ++++-----
plans/subsystems/eggup-interoperability-roadmap.md                             |  6 ++++--
plans/closure/release-manifest/003-status.md                                   | 51 ++++++++++++++++++++
plans/implementation/release-manifest/003a-...-reconciliation.md              |  2 +-
AGENTS.md                                                                      |  1 +-
```

All files are Markdown under `plans/` plus the agent-guidance pointer in `AGENTS.md`. No file outside documentation changed.

## Proof that no production/package/workflow content changed

Executed at `a0cd5eb`:

```text
git diff --check 64cc8453a572651f8f935add42a91d426d035f06..a0cd5eb
# no output, exit 0

git diff 64cc8453a572651f8f935add42a91d426d035f06..a0cd5eb -- crates/ Cargo.toml Cargo.lock .github/
# EMPTY  (confirmed with `git diff --quiet`)

git diff --name-only 64cc8453a572651f8f935add42a91d426d035f06..a0cd5eb
# plans/closure/release-manifest/003-status.md
# plans/implementation/release-manifest/003a-post-publication-downstream-closure-reconciliation.md
# plans/registry.md
# plans/subsystems/eggup-interoperability-roadmap.md
# plans/subsystems/release-manifest-roadmap.md

git status --short
# only the intended planning files
```

Commits in `64cc845..a0cd5eb`: `2dba62e`, `ceadfdc`, `71ae297` (plan registration, planning-only) and `a0cd5eb` (the reconciliation). All are documentation-only; the range is clean against every production path.

Publication immutability re-checked rather than assumed: the only tag is still `eggpack-manifest-v0.1.0` (annotated, pointing at `8d661e4`), and `eggpack-manifest 0.1.0` is still the only published version, with checksum `2a08f24b…b629` and `yanked = false`.

## Hosted CI

No hosted runtime qualification was required for this delta. Per plan §10, Rust tests are not required for a docs/planning-only pass because no Rust, Cargo, fixture, script, workflow, or package content changed; the verification target was repository truth and scope containment, both proven above.

Hosted CI did run, because the commits were pushed. It is recorded here as **observed, incidental** — it confirms nothing broke, and is not treated as evidence for any runtime or compatibility claim:

| Run | Commit | Result |
|---|---|---|
| [`37096952348`](https://github.com/eggstack/eggpack/actions/runs/37096952348) | `0b8e1f4` (closure commit) | `success`; `linux (stable)`, `linux (1.89.0)`, `portability (macos-latest)`, `portability (windows-latest)` all passed |
| `37096121155` | `71ae297` (plan registration) | `success` |
| `37096095399` | `ceadfdc` | `success` |
| `37096069510` | `2dba62e` | `success` |

The `Stable`/`MSRV`/`macOS`/`Windows` evidence cited throughout this record is **Eggup's** run `37090397398` on `eggstack/eggup`, reviewed as an external commit. It is attributed to Eggup and is not claimed as Eggpack execution. Eggpack's own last green run remains the M003 candidate run `37064833069` on `8d661e4`.

## Invariant review

- **M003 remains closed.** Untouched by this milestone; its historical body and findings are byte-identical, with an addendum appended below them.
- **History not rewritten.** The `unfinished` / `0.1.1` / `crate absent` statements inside M003 were correct on 2026-10-02 and are still there. The addendum's first table states the closure-time and later states side by side so a reader cannot mistake one for the other.
- **ReleaseManifest = evidence only.** No schema, API, wire-format, version, digest, or validation change. This milestone altered only descriptions of already-completed cross-repository state.
- **Integrity ≠ authenticity.** No signature, attestation, or provenance claim was added anywhere, including in the new addendum.
- **Publication authority unchanged.** No crate was published, yanked, retagged, re-versioned, or modified. `0.1.0` is still the sole version, still non-yanked, still at checksum `2a08f24b…b629`.
- **No automatic publication introduced.** Nothing in `.github/` changed.
- **Staging/release invariants untouched.** No tag, GitHub Release, or draft-staging state was created or altered.
- **External repositories untouched.** Eggup and Eggsact were read through the GitHub and crates.io APIs only. No write, no push, no issue, no file changed in either.
- **Ownership boundary preserved.** Everything written describes Eggup's and Eggsact's work as *downstream evidence reviewed by Eggpack*, never as work Eggpack performed, authorized, or closed. Eggsact's migration is explicitly recorded as Eggsact-owned.
- **Deferral preserved.** The low-severity published-README dangling-link finding stays deferred to a future crate version; `crates/eggpack-manifest/README.md` was not edited, so repository source still matches the immutable published bytes.

## Tests actually run

None. No Rust, Cargo, fixture, script, workflow, or package content changed, so no local `cargo` gate was run and none is claimed. Running the full workspace suite would have produced evidence about code that did not change, which would misrepresent the milestone. Scope-containment evidence is the diff proof above.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| low | The published `eggpack-manifest 0.1.0` README still points at the repository-only `architecture/eggup-manifest-consumer-v1.md`, so the reference dangles on crates.io/docs.rs. | **Unchanged and still deferred** to a future `0.1.1`/`0.2` metadata polish, per M003's disposition. Plan §4 forbids editing the published source representation for cosmetic alignment, and plan §6 excludes the fix. Repository source remains identical to the published bytes. |
| informational | The `eggup-eggfetch 0.1.1` and `eggup-acquisition 0.1.1` predecessors remain published and non-yanked alongside the `0.1.2` releases. | Not a defect. Supersession by version is normal semver registry practice; those versions are Eggup's to manage under its own policy. Recorded only so the sweep's finding is not silently dropped. |
| informational | Eggpack's own `AGENTS.md` next-handoff line and `plans/registry.md` had no live Eggpack consumer blocked on `eggpack-manifest 0.1.0`. | Resolved by this milestone; the surfaces now state it. |
| informational | docs.rs build status for the published crates was not re-queried. | Not a closure gate for a docs-only pass. M003 already recorded that an incomplete docs.rs build is a docs-infra signal, not an evidence or integrity defect. |

No medium-or-higher finding exists. No M003 finding was upgraded or reopened, and nothing here reopens M003.

## Dependency transitions

Unlocked by this closure: **none.** That is the finding, not an omission. M003a is a hygiene milestone, so per planning process §13 a correctives-first posture has nothing to hand off.

- **No Eggpack milestone becomes dependency-ready.** Every Release Manifest and Eggup interoperability milestone is now closed. Any new Eggpack milestone requires new evidence and a new plan registered in `plans/registry.md`.
- **No Eggup milestone was unblocked by Eggpack.** Eggup M004 was already closed in Eggup before this milestone; Eggpack only recorded it. Per planning process §9, no other repository's milestone is claimed as migrated.
- **Eggsact's Git-to-registry migration** is the next downstream action. It is Eggsact-owned, separately authorized in `eggstack/eggsact`, and explicitly not claimed by Eggup's M004 closure either. It is not an Eggpack blocker, handoff, or dependency-ready item, and it gates nothing here.
- **Future `eggpack-manifest` package polish** (the README link; any additional `0.1.x` metadata) remains a separate future version with its own plan. This milestone did not evaluate or authorize it.
- **Other Eggpack package publication remains unauthorized.** `eggpack-contract`, `eggpack-core`, `eggpack-bootstrap`, `eggpack-ci`, `eggpack-github`, and `eggpack-cli` are still unpublished, and no plan may assume otherwise.
- **Unrelated open conditions are unaffected:** Ecosystem M002's ordinary StegoEggo stable B > 0.4.2 live evidence, CI M003b's exact byte-identical rerun reuse (eggsact M005a Windows determinism), and Bootstrap M003's adoption evidence. None of these depended on Eggup M004, and none was touched.

## Final handoff

**No Eggpack work remains on the M003 -> Eggup M004 registry seam.** The Eggpack-owned publication prerequisite is closed and was consumed; the Eggup-owned registry promotion is closed and was recorded. Release Manifest M003a is closed, and the next Eggpack action is not a handoff but a blank control surface: new Eggpack work needs new evidence and a new plan.
