# Bootstrap Installers Milestone 003 Closure — Two-Consumer Adoption and Receipt-Boundary Decision

Status: closed

Source plan: `plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md`

Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Applicable ADRs: `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`, `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Reviewed Eggpack baseline: `013e091de2ce96fe1e8c4c6ccae344029b725055` (tip at review; the plan's implementation baseline `911da48c7c1c7967397a2d190730fc643c8c6dc3` plus the three plan-registration commits, no production change).

Reviewed external baselines:

- `eggstack/eggsact` at `d4e6e5c5cd7e9665373fbe312f4e1fb122fb5729` (tip at review); published release `v1.2.7` -> `d8014cfe68503fceb2448838fd22a5da5a6cfa27`;
- `eggstack/stegoeggo` at `5662d56246f3bf71d328f76356cbc58f6819b6a2` (tip at review); published release `v0.5.0` -> `57ca94c910269e080b06aa8cb34c3767b3bf669d`.

Implementation commits: none. This milestone landed **zero production, configuration, or workflow changes in Eggpack and zero changes in either consumer repository**. The only repository delta is this closure record plus the roadmap/registry/agent-handoff reconciliation.

## Executive finding

M003 is closed with the outcome the plan predicted, and it closes as an evidence-and-ownership decision rather than a code milestone.

1. **The existing exact-installer generator generalizes across two real products without a new producer primitive.** Both consumers publicly ship Eggpack-generated `install-exact.sh` / `install-exact.ps1` whose embedded artifact names, install names, exact sizes, and SHA-256 digests are equal to their own published `release-manifest.json` values — verified here against the live public releases, not only against fixtures.
2. **Remaining wrapper logic is product policy, with one precisely-scoped exception that is consumer-side duplication rather than a producer gap.** The wrappers still reconstruct the asset name (`binary_name="eggsact-${target}"`, `asset="stegoeggo-$target"`) when they take the latest/exact wrapper path. That is duplication *inside the consumer*, eligible for a consumer-side cleanup plan once Contract M003 provides a bounded scalar projection; it is not a missing Eggpack capability and it is not authority the bootstrap installer holds.
3. **Wrapper-to-exact-installer delegation is not warranted and cannot be done safely today.** The generated installer deliberately fails closed on every acquisition and integrity error, and it exposes no stable exit-status protocol a wrapper could use to keep a 404-only Cargo fallback while still hard-failing 5xx/TLS/digest errors. Delegation is therefore a separate capability, not an increment.
4. **An Eggup receipt handoff is rejected.** Bootstrap owns first-install bytes; Eggup owns deployment transaction state. The distinction is already encoded in the codebase and this milestone confirms it.
5. **The subsystem completion definition holds.** Two independent consumers have removed duplicated mapping/checksum *authority* from bootstrap installers; effective authority, not the mere existence of wrapper code, is the test.

No unresolved medium-or-higher bootstrap defect was found. Nothing was weakened to reach this outcome.

## Requirement-to-evidence matrix

| Plan requirement | Evidence and result |
|---|---|
| §13.1 both independent public-consumer release/installer evidence sets reviewed | Live `gh release view` inventories for `eggstack/eggsact v1.2.7` (published 2026-09-30T13:06:59Z, non-draft, non-prerelease) and `eggstack/stegoeggo v0.5.0` (published 2026-10-04T06:34:19Z, non-draft, non-prerelease). Each release carries exactly 15 assets: 5 target binaries, 5 `.sha256` sidecars, `install.sh`, `install.ps1`, `install-exact.sh`, `install-exact.ps1`, `release-manifest.json`. |
| §13.2 current exact installers shown to derive producer facts from Eggpack | Independent parity check (below) confirms every embedded size/digest/artifact-name/install-name equals the published manifest, in both the POSIX and PowerShell installer, for all 5 targets of both consumers. Producer code path: `render_posix`/`render_powershell` (`crates/eggpack-bootstrap/src/lib.rs`) take `DistributionContract` + `ReleaseManifest` and project from `project()`, which rejects a contract/manifest target-set mismatch and any ambiguous runtime platform mapping. |
| §13.3 remaining wrapper duplication classified by ownership with concrete examples | See the two-consumer authority matrix. Two surviving classes: (a) frozen `PUBLIC_ASSETS` compatibility tables, and (b) asset-name reconstruction in the wrapper download path. Both are consumer-owned; (b) is real duplication to remove consumer-side. |
| §13.4 no unresolved common producer-side bootstrap capability needed for either live consumer | Both live consumers publish every target their contract declares, in both installer forms, with no unsupported published target and no target that only a producer feature could express. No M003a is required. |
| §13.5 generated exact installers retain fail-closed size/SHA and first-install boundaries | Confirmed against the live installers: `size mismatch`, `SHA-256 mismatch`, `unsupported platform`, and `destination already exists` are all present in both forms; the PowerShell installer removes invocation-created paths on failure; the POSIX installer places with `ln` and re-checks the destination immediately before placement; neither resolves `latest/download`; both pin the exact release origin. Producer test `generated_shell_installs_verified_bytes_and_refuses_existing_file` remains green. |
| §13.6 Eggup receipt decision explicit and ownership-correct | No receipt concept exists in `eggpack-bootstrap` at all; `crates/eggpack-bootstrap/README.md` line 12 already excludes "normal updates, Eggup receipts, service lifecycle, signing, and remote discovery". The one `*Receipt*` type in the workspace, `GitHubDraftReceiptV1` (`crates/eggpack-github/src/lib.rs:295-298`), is documented as "Bounded staging receipt (producer evidence, **not** an install receipt)". Decision recorded in §Receipt-handoff decision below. |
| §13.7 targeted Eggpack bootstrap tests green | `cargo test -p eggpack-bootstrap --all-targets --all-features --locked` -> 8 passed, 0 failed; `eggpack-github` 32 passed; `eggpack-cli` 12 passed; MSRV `1.89.0` bootstrap 8 passed; `cargo doc -p eggpack-bootstrap --no-deps` clean; `cargo fmt --all -- --check` clean; `git diff --check` clean. Full output in §Verification executed. |
| §13.8 no unresolved medium-or-higher bootstrap defect | See §Unresolved findings. The only new finding is low severity and consumer-side. |
| §13.9 no external repository or production source changed | `git status` in all three repositories is clean apart from the closure/roadmap/registry edits in Eggpack; evidence was gathered read-only from git objects and from public release URLs. No `gh` write call was made. |
| §13.10 closure record exists | This file. |

## Two-consumer authority matrix

| Fact | Canonical Eggpack authority | Generated exact installer | Remaining hand-written producer fact in the consumer | Ownership verdict |
|---|---|---|---|---|
| Published target set | `release/eggpack/distribution.toml` `[[targets]]`, projected through the generated workflow and `release-manifest.json` | One generated case per target; `*)` refuses anything else with exit 2 | Wrapper `case "${os}:${arch}"` host classification (eggsact maps `Linux:armv7l`, stegoeggo maps `Darwin:aarch64`) | Product policy. The wrapper must classify its own host in order to decide *whether to take a binary path at all*; Eggpack's generated case table is the canonical published set. Eggsact deliberately keeps `armv7-unknown-linux-gnueabihf` out of the contract and treats it as Cargo-fallback-only, and the contract comment says so explicitly. |
| Release asset name | `{product}-{target}` (plus the contract's `.exe` rule for `*-pc-windows-msvc`) | Literal per case, equal to the manifest `artifact.name` | `binary_name="eggsact-${target}"` (eggsact `packaging/install.sh`); `asset="stegoeggo-$target"` with a `*‑pc-windows-msvc` suffix append (stegoeggo `packaging/install.sh`) | **Duplicated producer fact, consumer-owned.** It is a reconstruction of the contract template, not new policy. It is *not* bootstrap authority and *not* a producer gap. It is now cleanable consumer-side through `eggpack contract expand --field asset` once Contract M003 closes — see §Downstream handoff. |
| Checksum digest values | `ReleaseManifest` final bytes; `install-exact.*` embeds the exact digest | Literal per case; `sha256sum`/`shasum`/`openssl` fallback, exit 7 if none, exit 8 on mismatch | **None.** Both wrappers download the producer-published `.sha256` sidecar and compare; neither hand-writes a digest | Producer authority. The wrappers *verify* producer facts, which is not duplication of authority. |
| Exact sizes | `ReleaseManifest` | Literal per case; exit 6 on mismatch | None | Producer authority. |
| Install name / destination | Contract `install` for the generated installer; `install-policy.toml` for modes | Literal `$file="$dest"/'eggsact'` / `Join-Path $dest 'eggsact'` | Wrapper `install_destination` (`/usr/local/bin` vs `$HOME/.local/bin`) and the eggsact `$EUID == 0` equivalent | Product policy under ADR-0001. The generated installer takes the destination as `$1`/`.`; the wrapper owns where a human's install goes. |
| Release selection (`latest` vs explicit version) | None — the generated installer is release-pinned | `origin` is pinned to the exact tag; no `latest/download` appears in either installer | `release_path="latest/download"` vs `download/v$requested_version`; `$STEGOEGGO_RELEASES_URL` override | Product policy by design. The plan's invariant "generated exact installers remain release-specific and do not select latest versions" is satisfied and was asserted in the parity check. |
| Cargo fallback | None | Absent by design | `cargo_fallback` in both wrappers | Product policy. |
| Candidate version check | None (bootstrap verifies bytes, not product identity) | Absent by design | `eggsact --version` / `stegoeggo version` identity assertions | Product policy. |
| Frozen public-name guard | Not authority | — | `PUBLIC_ASSETS` in both `scripts/check-release-contract.py` | Legitimate consumer-owned guard. It is compared *against* contract expansion (`if contracted != PUBLIC_ASSETS`) so a public rename fails CI instead of silently shipping. This is the case the plan §7/Q1 explicitly permits to remain. |

## Generated/public installer inventory (live, reviewed 2026-10-05)

`eggstack/eggsact v1.2.7`, `release-manifest.json` `release_id: v1.2.7`, `source_revision: d8014cfe68503fceb2448838fd22a5da5a6cfa27` — exact tag match:

| Target | Artifact | Size | SHA-256 (first 16) | Install name |
|---|---|---|---|---|
| `aarch64-apple-darwin` | `eggsact-aarch64-apple-darwin` | 11460784 | `756e6b3759107fdeb` | `eggsact` |
| `aarch64-unknown-linux-gnu` | `eggsact-aarch64-unknown-linux-gnu` | 11893680 | `6bf8cfcb5539a07e` | `eggsact` |
| `x86_64-apple-darwin` | `eggsact-x86_64-apple-darwin` | 12564384 | `892eb3569749ea5c` | `eggsact` |
| `x86_64-pc-windows-msvc` | `eggsact-x86_64-pc-windows-msvc.exe` | 16607232 | `1749c94aa589f555` | `eggsact.exe` |
| `x86_64-unknown-linux-gnu` | `eggsact-x86_64-unknown-linux-gnu` | 14161536 | `2d88a034988acfd2` | `eggsact` |

`eggstack/stegoeggo v0.5.0`, `release-manifest.json` `release_id: v0.5.0`, `source_revision: 57ca94c910269e080b06aa8cb34c3767b3bf669d` — exact tag match:

| Target | Artifact | Size | SHA-256 (first 16) | Install name |
|---|---|---|---|---|
| `aarch64-apple-darwin` | `stegoeggo-aarch64-apple-darwin` | 3554800 | see manifest | `stegoeggo` |
| `aarch64-unknown-linux-gnu` | `stegoeggo-aarch64-unknown-linux-gnu` | 3892200 | see manifest | `stegoeggo` |
| `x86_64-apple-darwin` | `stegoeggo-x86_64-apple-darwin` | 4063904 | see manifest | `stegoeggo` |
| `x86_64-pc-windows-msvc` | `stegoeggo-x86_64-pc-windows-msvc.exe` | 3899392 | see manifest | `stegoeggo.exe` |
| `x86_64-unknown-linux-gnu` | `stegoeggo-x86_64-unknown-linux-gnu` | 4427472 | see manifest | `stegoeggo` |

Live origin resolution, every artifact and sidecar for both releases, HTTP 200 with `Content-Length` equal to the manifest size (20 of 20 URLs):

```text
eggsact-aarch64-apple-darwin            200 11460784     stegoeggo-aarch64-apple-darwin       200 3554800
eggsact-aarch64-apple-darwin.sha256     200 95           stegoeggo-aarch64-apple-darwin.sha256 200 97
eggsact-aarch64-unknown-linux-gnu      200 11893680     stegoeggo-aarch64-unknown-linux-gnu  200 3892200
eggsact-aarch64-unknown-linux-gnu.sha256 200 100        stegoeggo-aarch64-unknown-linux-gnu.sha256 200 102
eggsact-x86_64-apple-darwin             200 12564384     stegoeggo-x86_64-apple-darwin       200 4063904
eggsact-x86_64-apple-darwin.sha256      200 94           stegoeggo-x86_64-apple-darwin.sha256 200 96
eggsact-x86_64-pc-windows-msvc.exe      200 16607232     stegoeggo-x86_64-pc-windows-msvc.exe 200 3899392
eggsact-x86_64-pc-windows-msvc.exe.sha256 200 101        stegoeggo-x86_64-pc-windows-msvc.exe.sha256 200 103
eggsact-x86_64-unknown-linux-gnu       200 14161536     stegoeggo-x86_64-unknown-linux-gnu 200 4427472
eggsact-x86_64-unknown-linux-gnu.sha256 200 99          stegoeggo-x86_64-unknown-linux-gnu.sha256 200 101
```

`install-exact.sh` and `install-exact.ps1` also resolve at both `releases/download/<tag>/` and `releases/latest/download/` (HTTP 200 in all four pairs).

Independent parity check executed for this closure (read-only, no repository mutation): a bounded Python check parsed each published `release-manifest.json`, extracted each generated installer's per-target artifact name, size, SHA-256 and install name, and required exact equality in both the POSIX and PowerShell installer; it also required a case for every published target, the fail-closed markers `size mismatch` / `SHA-256 mismatch` / `unsupported platform` / `destination already exists`, PowerShell partial-failure cleanup, absence of `latest/download`, and an origin pinned to the release id. Result: **passed for both consumers, 5 targets each, no discrepancy.**

## Wrapper policy comparison

| Behaviour | eggsact `packaging/install.{sh,ps1}` | stegoeggo `packaging/install.{sh,ps1}` | Class |
|---|---|---|---|
| Bash-only guard | Explicit `BASH_VERSION` + `"${BASH##*/}" = "sh"` check before `set -euo pipefail` | `BASH_VERSION` + POSIX-mode probe | Product policy |
| Explicit version selection | `--version X.Y.Z`, `download/v$requested_version` | `--version` and `--version=`, `download/v$requested_version` | Product policy |
| Latest resolution | `releases/latest/download` | `releases/latest/download`, overridable via `STEGOEGGO_RELEASES_URL` | Product policy |
| Host classification | 6 mappings incl. `Linux:armv7l` (Cargo-fallback-only) | 4 mappings; `aarch64` also spelled `arm64` | Product policy |
| Fallback trigger | HTTP 404 only (download returns 44); all other failures hard-fail | HTTP 404 only; all other failures hard-fail | Product policy |
| Fallback mechanism | `cargo install eggsact --locked --root <tmp>` then validate + install | `cargo install stegoeggo-cli --locked [--version]` (system install) | Product policy |
| Integrity | Producer sidecar + `sha256sum`/`shasum`; refuses install if the sidecar cannot be fetched | Producer sidecar + `sha256sum`/`shasum`; fails on sidecar non-2xx | Verification of producer facts, not duplicated authority |
| Candidate identity | `eggsact --version` must match the request or the `X.Y.Z` shape | `stegoeggo version` must match `stegoeggo X.Y.Z` and the request | Product policy |
| Install destination | `/usr/local/bin` as root else `$HOME/.local/bin`, PATH hint | `/usr/local/bin` as root else `$HOME/.local/bin`, PATH warning | Product policy |
| Frozen public-name guard | `scripts/check-release-contract.py` `PUBLIC_ASSETS` compared against contract expansion | same | Consumer-owned compatibility guard |
| Contract-parity guard | `scripts/check-release-contract.py` also asserts the wrapper still contains the host-mapping and target fragments | same | Consumer-owned guard on wrapper policy |

The two wrappers converge on the same policy shape without sharing an implementation, which is itself the evidence that this is policy rather than producer authority: neither one is required for Eggpack to be correct, and neither carries a fact Eggpack needs.

## Candidate-review question dispositions

### Q1 — Are target/artifact/checksum producer facts singular?

**Yes, with one consumer-side reconstruction called out.** Target set, artifact names, install names, sizes, and digests are singular and producer-owned: `distribution.toml` is the only canonical input, the generated workflow and `release-manifest.json` are its projections, and both generated exact installers embed the manifest values byte-for-byte. No consumer hand-writes a digest. Two non-authority artifacts remain consumer-owned by design: the frozen `PUBLIC_ASSETS` guard table (names only, compared against contract expansion) and the wrapper's asset-name reconstruction. The latter is genuine duplication of a producer *fact* and is recorded as low-severity finding F-1 with a consumer-side disposition; it is not bootstrap authority and does not require an Eggpack capability.

### Q2 — Do exact installers cover the required public target form?

**Yes.** Both consumers publish five direct targets and both generated installers carry exactly those five cases plus an `unsupported platform` refusal, in both POSIX and PowerShell form. No published target is missing from either installer, and neither consumer publishes a target that only a producer feature could express. Eggsact's `armv7-unknown-linux-gnueabihf` host recognition is a wrapper-side Cargo-fallback decision, and its contract states why the target is deliberately absent.

### Q3 — Is wrapper-to-exact-installer delegation now warranted?

**No, and it cannot be done safely without a new capability.** Delegation would have to preserve, simultaneously: latest/exact selection, 404-only fallback, unsupported-host fallback, hard failure on checksum/version/TLS/5xx errors, and install-destination behaviour. The generated installer offers no channel for the first four:

- POSIX documents exit codes 2 (unsupported platform), 3 (destination exists), 4 (temp dir), 5 (no `curl`), 6 (size mismatch), 7 (no SHA-256 tool), 8 (SHA-256 mismatch) — but an HTTP 404 is not one of them. `curl --fail` collapses every 4xx/5xx to curl's exit 22, verified here against a real missing-asset URL on a live release (`exit=22`, `curl: (22) The requested URL returned error: 404`). A wrapper therefore cannot tell "asset absent, fall back" from "server broken, hard fail".
- PowerShell emits no explicit exit codes at all (0 `exit` calls in the rendered file); every failure is a terminating `throw`, which is indistinguishable at the process boundary.

Making delegation safe requires a new stable machine-readable exit-status or result protocol — precisely the plan §7/Q3 and §14 stop condition. That is a separate capability and is **not** authorized here. Not warranted now, and it must not be smuggled in as a follow-on edit.

### Q4 — Is an Eggup receipt handoff warranted?

**No.** The plan's default holds, and the codebase already encodes the reason:

- `eggpack-bootstrap` contains no receipt type, no transaction state, no prior-state read, and no rollback record. The generated installers are single-shot: verify bytes, refuse an existing destination, place one file, clean up their own temp directory. They cannot distinguish "installed over a previous install" from "fresh install" because they never read previous state and never replace a file.
- The workspace's only receipt type, `GitHubDraftReceiptV1`, is documented in `crates/eggpack-github/src/lib.rs` as "Bounded staging receipt (producer evidence, not an install receipt)" and describes a GitHub draft and its asset accounting — a release-staging fact, categorically different from an installed-state fact.
- `crates/eggpack-bootstrap/README.md` already states that "normal updates, Eggup receipts, service lifecycle, signing, and remote discovery remain out of scope".

The two states the plan asks to be distinguished remain distinct: "bootstrap installed exact bytes" is what the generated installer does; "Eggup transaction committed / owns rollback state" is what Eggup owns. No live consumer currently needs the second from the first — both consumers' installers are a first-install convenience surface over the same producer artifacts, and eggsact's real update path is `eggfetch-core`-backed and self-contained. Adopting a receipt here would move transaction authority into producer first-install output, which ADR-0001 forbids and which the plan requires an architecture decision for. Rejected.

### Q5 — Does the Bootstrap subsystem completion definition hold now?

**Yes.** The roadmap definition is "at least two consumers have removed duplicated mapping/checksum authority from bootstrap installers", and the plan correctly says to test *effective authority*, not the mere existence of wrapper code. Two independent products publish Eggpack-generated exact installers whose mapping, sizing, and digest facts come solely from the producer contract and manifest, with no consumer-authored digest or target table acting as authority. The residual wrapper code is selection, fallback, destination, and identity policy — product-owned under ADR-0001 — plus one consumer-side name reconstruction (F-1). On that basis the bootstrap-installers subsystem completion definition is satisfied and no follow-on producer milestone is required.

## Production implementation evidence

None, by design and by the plan's stop conditions. Specifically **not** changed: generated script bytes, template logic, wrapper delegation, exit-code protocol, release selection, Cargo fallback, install destinations, any Eggup code or receipt semantics, any consumer repository, service lifecycle, signatures/attestations, package publication. `git diff --stat` for this milestone touches `plans/` and `AGENTS.md` only.

## Verification executed

Run against Eggpack `013e091`, Linux x86-64, non-mutating (`cargo test` and `cargo doc` only; no build, no release, no workflow dispatch):

```text
cargo fmt --all -- --check                                                             passed
cargo test -p eggpack-bootstrap --all-targets --all-features --locked                  passed (8 passed, 0 failed, 0 ignored)
cargo test -p eggpack-github --all-targets --all-features --locked                      passed (32 passed, 0 failed, 0 ignored)
cargo test -p eggpack-cli --all-targets --all-features --locked                         passed (12 passed, 0 failed, 0 ignored)
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked                          passed (8 passed, 0 failed, 0 ignored)
cargo doc -p eggpack-bootstrap --no-deps --locked                                      passed
git diff --check                                                                       passed
```

Read-only external evidence review, all non-mutating (no `gh` write, no release/tag/package operation):

```text
gh release view v1.2.7 --repo eggstack/eggsact --json tagName,isDraft,isPrerelease,publishedAt,targetCommitish,assets
gh release view v0.5.0 --repo eggstack/stegoeggo --json tagName,isDraft,isPrerelease,publishedAt,assets
git show / git log in eggstack/eggsact and eggstack/stegoeggo (object reads only)
20 HTTPS HEAD/GET origin checks for staged artifacts and sidecars (all 200, sizes match)
4 installer URL checks at /download/<tag>/ and /latest/download/ (all 200)
1 bounded curl exit-code probe for Q3 (404 -> exit 22)
1 bounded Python parity check for installer/manifest equality (passed for both consumers)
```

**Not performed, and therefore not claimed:** no hosted CI run was triggered by this milestone (there is no production change to qualify); no real `install-exact.sh`/`install-exact.ps1` installation was executed here — that evidence belongs to the two consumer closures already recorded (`plans/closure/ecosystem-adoption/001-status.md` §16 exact/latest installer smoke and `plans/closure/ecosystem-adoption/002-status.md` acceptance row 8); no macOS or Windows lane was exercised locally.

## Invariant review

| Invariant | Result |
|---|---|
| No runtime/product policy moves from consumer repositories into Eggpack | Held. Nothing moved; the wrapper policy classification in the authority matrix explicitly leaves it in the consumer. |
| No Eggup transaction/receipt semantics move into generated bootstrap installers | Held, and no receipt was added anywhere. Q4 rejected the handoff. |
| Generated exact installers remain release-specific and do not select latest versions | Held and asserted: both live installers pin `origin` to the exact release id and neither contains `latest/download`. |
| Product wrappers may continue selecting latest/exact/Cargo fallback paths | Held and unchanged; both wrappers keep their behavior. |
| Integrity remains distinct from authenticity | Held. Installers verify size and SHA-256 only; no signature or attestation claim appears in either generated installer, and this milestone added none. |
| Bootstrap does not become self-update | Held. No update, upgrade, or replace-existing-destination path exists; an existing destination is refused with exit 3 / `throw 'destination already exists'`. |
| Existing exact installer safety semantics are not weakened | Held. Zero production bytes changed; the producer test suite is green. |
| Historical M001/M002/M002a closures remain historical evidence | Held. `plans/closure/bootstrap-installers/001`, `002`, `002a` untouched. |
| M003 does not modify external repositories | Held. Read-only git object reads and public HTTP reads only. |
| A discovered feature gap gets a new plan rather than opportunistic code | Held. Q3's needed protocol is recorded as a non-authorized future capability; Q1's duplication is routed to a consumer-side plan. No code was written. |
| No independent target/asset table in the generated installer | Held. Cases are generated from the contract/manifest projection; `project()` fails closed if the contract and manifest target sets differ or if two targets map to one runtime platform pair. |

## Failure/recovery review

- This pass is non-mutating outside planning/docs, so there is nothing to roll back and no partial state to reconcile.
- The parity check was written to fail closed: any absent case, absent digest, absent size, absent install name, `latest/download` leak, un-pinned origin, or missing fail-closed marker is reported as a failure. It reported none. Its value is therefore negative evidence, and it is reproducible from public releases without credentials.
- The Q3 `curl --fail` probe is the only behavioral experiment. It is a property of `curl`, not of Eggpack, and it would change only if the generated installer stopped using `curl --fail` — which no current milestone proposes.
- If either consumer later re-publishes with different names, the existing consumer-owned `check-release-contract.py` guard fails CI *before* a release exists. That is the intended recovery path and it is already in place in both repositories.
- No contention or restart hazard: no cache, service, or shared state was introduced.

## Compatibility and migration review

No migration is authorized or performed. Neither consumer was modified; no wrapper behavior changed; no public asset, URL, or installer name changed. Both consumers' `eggpack_tool.revision` pins are untouched, so this closure does not move any consumer's Eggpack dependency. Per the plan §10, any later wrapper delegation must be planned in each consumer repository separately, because wrapper policy is product-owned.

## Security review

No new attack surface: no network, credential, privilege, archive, publication, or setup surface was introduced, and no generated byte changed.

Reviewed properties, confirmed on the live installers:

- bounded acquisition (`--connect-timeout 10 --max-time 120`; PowerShell `-TimeoutSec 120 -MaximumRedirection 0`);
- integrity checked **before** placement, with a re-check of the destination immediately before the atomic `ln` / `File.Move`;
- existing installations are preserved, never overwritten, and never removed;
- PowerShell removes only invocation-created paths on failure and always removes its temp directory;
- no shell string interpolation of downloaded content — the artifact name is a quoted literal, not evaluated;
- no privilege escalation: the generated installer installs where it is told (`$1` or `.`) and the wrapper's destination choice is product policy;
- partial failure is bounded and typed by distinct exit codes on POSIX;
- unsupported platforms fail closed with no network attempt;
- no secret, token, or environment value appears in generated text; the only repository-level receipt/credential logic stays in `eggpack-github`'s redaction path, which its own tests cover (`token_absent_rejects_before_io_and_redacts_diagnostics` green).

The one security-relevant observation from this pass is the reverse of a new risk: because the generated installers refuse an existing destination rather than replacing it, they cannot be used as a silent upgrade path. That is intentional, is documented, and is a reason Q4's receipt discussion is moot today.

## Documentation and operations evidence

Updated by this closure:

- `plans/subsystems/bootstrap-installers-roadmap.md` — §11 completion definition recorded as satisfied, §12 M003 row closed with this record, subsystem status noted;
- `plans/registry.md` — Bootstrap M003 row moved to `closed` with the closure pointer; the "Dependency-ready implementation work" preamble and the immediate execution graph updated; the live-evidence bullet refreshed with the installer-parity result;
- `AGENTS.md` — current-handoff paragraph updated;
- this closure record.

Not changed: canonical documents `plans/000`–`plans/002`, `plans/003-planning-process.md`, every ADR, and every historical closure record. No new ADR was required, because nothing in this milestone moved ownership, a public schema, publication authority, a trust model, or release finalization semantics.

Operations impact: none. No new command, no new artifact, no new required configuration, no release-operator procedure change.

## Unresolved findings

| Severity | Finding | Disposition |
|---|---|---|
| Low (F-1) | Both wrappers reconstruct the producer asset name instead of deriving it: eggsact `binary_name="eggsact-${target}"`, stegoeggo `asset="stegoeggo-$target"` plus a Windows `.exe` append. This is duplicated producer truth living consumer-side. | Accepted as consumer-owned. It is not bootstrap authority, does not affect either published release (the parity check proves the generated installers are correct), and does not block this subsystem. It becomes cleanly removable consumer-side once `eggpack contract expand --field asset` exists — Contract M003 is authorized and in progress. Requires a plan in the *consumer* repository; this Eggpack closure does not authorize it. |
| Low (F-2) | Stegoeggo's wrapper asset-name rule appends `.exe` from a target *substring* test (`*‑pc-windows-msvc`) rather than from the contract. Today it agrees with the contract; a future contract change that introduced a second Windows form could drift. | Consumer-owned. Its existing `check-release-contract.py` already compares wrapper fragments and contract expansion, and F-1's removal eliminates the class entirely. Same disposition as F-1. |
| Informational | The stegoeggo checkout available locally was stale (tip `a01a022`) relative to the reviewed release; `v0.5.0` and the `0.5.0` closure were not present locally at first review. | Resolved before adjudication: the repository was fetched read-only and all evidence was taken from `v0.5.0` (`57ca94c`) and current `origin/main` (`5662d56`). Recorded because a closure that claimed a baseline it did not actually read would be worthless. |
| Informational | No hosted CI run was triggered for this milestone and no generated byte changed, so there is no CI evidence to report. | Accepted. The plan's §11 for this milestone requires targeted local bootstrap verification plus read-only external evidence review, both executed above. Full-workspace hosted evidence belongs to the milestones that change production bytes. |
| Informational | The generated PowerShell installer exposes no exit-status channel at all (0 `exit` calls; failures are terminating throws). | Recorded as the concrete reason Q3 delegation is a separate capability, not as a defect of the current release surface. Changing it would alter a public artifact's contract and would need its own plan. |
| None | No unresolved medium-or-higher bootstrap defect. | M003 acceptance criteria satisfied; no M003a corrective is required. |

## Roadmap disposition

| Milestone | Status after M003 | Reason |
|---|---|---|
| Bootstrap M003 | closed | This record. Two-consumer evidence satisfied; no production change; no M003a. |
| Bootstrap subsystem (roadmap §11 completion definition) | satisfied | Two independent consumers have removed duplicated mapping/checksum authority from bootstrap installers; verified against live releases, not fixtures. |
| Contract M003 | unaffected, still `ready` | Bootstrap M003 reviewed bootstrap only. Contract M003's evidence is its own: three duplicated contract-expansion implementations in two consumer repositories. |
| Ecosystem M003a | unaffected, still `ready` | Reviewed Eggsearch seven-target shape and ARMv7 evidence strategy, not bootstrap authority. Its target coverage conclusion is consistent with Q2. |
| Bootstrap M002 / M002a | unchanged, closed (M002 historically, corrective closed) | Untouched; no closure rewritten. |

No milestone was unblocked by this closure, and none was newly blocked.

## Downstream handoff

Bootstrap has no remaining open work. The only concrete follow-up this closure creates is consumer-side and optional:

1. **Optional, product-owned:** eggsact and stegoeggo may each delete the wrapper asset-name reconstruction (F-1/F-2) in favour of `eggpack contract expand --field asset` once Contract M003 closes and each consumer pins an Eggpack revision that provides it. That is a change in the consumer's repository and requires a plan registered there. This Eggpack closure does not authorize it and does not claim it.
2. **Not authorized, explicitly:** a wrapper→exact-installer delegation capability. If it is ever wanted, it is a new milestone whose first requirement is a stable machine-readable failure/outcome protocol, because neither installer form exposes one today.
3. **Not authorized, explicitly:** any Eggup receipt handoff from bootstrap. The boundary stays where ADR-0001 put it.

## Registry updates

- Bootstrap M003 row: `ready` -> `closed`, closure pointer `plans/closure/bootstrap-installers/003-status.md`.
- Bootstrap subsystem roadmap: completion definition satisfied; M003 row closed.
- Live consumer evidence bullet: extended with the live installer/manifest parity result for both releases.
- Immediate execution graph: Bootstrap M003 node marked closed; the graph no longer shows it as a ready line.
- `AGENTS.md` current handoff: Bootstrap M003 removed from the ready set.

## Eggstack references

- Closed source plan: `plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md`
- Roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`
- Predecessor closures (historical, untouched): `plans/closure/bootstrap-installers/001-status.md`, `plans/closure/bootstrap-installers/002-status.md`, `plans/closure/bootstrap-installers/002a-status.md`
- Two-consumer adoption closures: `plans/closure/ecosystem-adoption/001-status.md` (eggsact `v1.2.7`, Eggpack run `36652731202`), `plans/closure/ecosystem-adoption/002-status.md` (stegoeggo `v0.5.0`, Eggpack run `37181914252`)
- Staging/installer-presentation ownership: `crates/eggpack-github/src/lib.rs` (`InstallerPresentationV1`, `GitHubDraftReceiptV1`), `crates/eggpack-bootstrap/src/lib.rs` (`render_posix`, `render_powershell`, `project`)
- External repositories were read only. No external commit, release, tag, or package state was created or changed by this milestone.