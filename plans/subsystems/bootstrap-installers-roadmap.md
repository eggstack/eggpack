# Bootstrap Installers Roadmap

Status: active; M002b corrective closed

Long-term references:

- `plans/000-long-term-specification.md#13-bootstrap-installers`
- `plans/002-long-term-roadmap.md#phase-6--bootstrap-installer-generationconformance`

Related ADRs:

- ADR-0001;
- ADR-0002.

## 1. Purpose and ownership boundary

Own generation or deterministic validation of first-install POSIX shell and PowerShell installers from Eggpack release authority.

Bootstrap code is a consumer-facing artifact, but its release mapping originates on the producer side. Normal self-update remains Eggup/application-owned.

## 2. Work classification

### Invariants

- no independent target/asset table in generated installer;
- unsupported targets fail closed;
- bounded/fixed-origin download policy;
- integrity before execution/installation;
- no implicit privilege escalation;
- existing installation preserved on verification failure;
- installer remains readable.

### Capabilities

- generate/check shell installer;
- generate/check PowerShell installer;
- test against local fake releases;
- support direct then bundle/archive layouts.

### Infrastructure

- installer model/templates;
- OS/arch mapping projection;
- product policy injection points.

## 3. Non-goals

- full shell templating language;
- runtime self-update;
- service-manager ownership;
- package-manager fallback unless supplied explicitly by product policy.

## 4. Current state

eggsact, stegoeggo, eggsearch, Gregg, and Egress maintain separate shell/PowerShell installers with overlapping target mapping/checksum/download logic.

## 5. Target architecture

DistributionContract + ReleaseManifest-derived names feed generators. Product-specific origin/install destination/fallback messages remain explicit inputs.

## 6. Dependency graph

Hard dependencies: contract conformance M002 + manifest M001. Build engine is not required for local fake-release generator tests.

## 7. Milestones

M001 direct generator and installer fixtures — closed. Deterministic shell/PowerShell scripts derive their mapping and integrity values from validated contract+manifest inputs.

M002 bundle/archive bootstrap safety.

Implementation plan: `plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md`.

M003 two-consumer adoption and optional Eggup receipt handoff decision.

Implementation plan: `plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md`.

## 8. Cross-cutting requirements

ShellCheck/PSScriptAnalyzer where available, no secrets in generated text, safe temp dirs, download timeouts, checksum tool fallbacks explicitly tested.

## 9. Verification strategy

Local HTTP fixture server, checksum mismatch, 404, timeout, unsupported arch, wrong candidate version, existing install preservation, shell parse tests, Windows parse/tests.

## 10. Risks and decision points

Complete generation may become opaque. Prefer small deterministic templates/fragments if that remains more reviewable.

## 11. Completion definition

At least two consumers have removed duplicated mapping/checksum authority from bootstrap installers.

**Satisfied as of 2026-10-05** at `plans/closure/bootstrap-installers/003-status.md`. Eggsact `v1.2.7` and StegoEggo `v0.5.0` each publicly ship Eggpack-generated `install-exact.sh` / `install-exact.ps1` whose artifact names, install names, exact sizes, and SHA-256 digests were verified equal to their own published `release-manifest.json` values for all five direct targets, in both installer forms. No consumer hand-writes a digest. The residual wrapper code is latest/exact selection, Cargo fallback, install destination, and candidate-identity policy — product-owned under ADR-0001 — plus one consumer-side asset-name reconstruction recorded as low-severity finding F-1 with a consumer-side disposition.

## 12. Milestone status

M001 generator model and direct installer fixtures is closed and qualified. M002 is historical implementation/closure evidence with its M002a corrective closed.

M003 is **closed** as an evidence/ownership decision pass (`plans/closure/bootstrap-installers/003-status.md`, reviewed against Eggpack `013e091`, eggsact `d4e6e5c`/`v1.2.7` -> `d8014cfe`, stegoeggo `v0.5.0` -> `57ca94c9`). Its two-consumer evidence gate cleared 2026-10-04 (Planning Hygiene M001) and the candidate review closed with zero production change. Three decisions are now settled rather than open:

- the existing exact-installer generator generalizes across two real products without a new producer primitive, so **no M003a corrective was required**;
- wrapper-to-exact-installer delegation is **not warranted** and cannot be done safely without a new stable machine-readable failure/outcome protocol — a separate capability that is explicitly not authorized;
- an **Eggup receipt handoff is rejected**: bootstrap installs exact bytes, Eggup owns transaction and rollback state, and the workspace's only receipt type (`GitHubDraftReceiptV1`) is already documented as producer staging evidence, not an install receipt.

Bootstrap has no remaining open work. The only follow-up created is optional and consumer-owned: each consumer may delete its wrapper asset-name reconstruction once Contract M003 provides `eggpack contract expand --field asset`, under a plan registered in that consumer's own repository.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 direct generator + fixtures | closed | `plans/implementation/bootstrap-installers/001-direct-installer-generator.md` | `plans/closure/bootstrap-installers/001-status.md` | direct first-install semantics qualified; archive-content/finalization evidence remains later work |
| M002 bundle/archive bootstrap safety | closed historically; corrective closed | `plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md` | `plans/closure/bootstrap-installers/002-status.md` | Post-closure PowerShell archive runtime evidence gap corrected and qualified by M002a |
| M002a PowerShell archive runtime evidence | closed | `plans/implementation/bootstrap-installers/002a-powershell-archive-runtime-evidence-corrective.md` | `plans/closure/bootstrap-installers/002a-status.md` | Closed on implementation `4d2270a` + hosted run 36154905956 (attempt 1, all lanes green) via shared closeout pass `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md` |
| M003 two-consumer adoption/receipt decision | closed | `plans/implementation/bootstrap-installers/003-two-consumer-adoption-and-receipt-boundary-decision.md` | `plans/closure/bootstrap-installers/003-status.md` | Closed evidence/ownership pass, zero production delta; live installer/manifest parity proven for both releases; delegation and Eggup receipt explicitly declined; no M003a required; optional consumer-side cleanup only |

## 13. 2026-10-08 corrective addendum — M002b

**M002b ready:** `plans/implementation/bootstrap-installers/002b-windows-posix-archive-test-portability-corrective.md`. At `3ef806fedc9d7683948e5678ec0d3c7b78c06f0e`, hosted Windows run 37649332862 fails to compile POSIX-only test uses of `std::os::unix::fs::PermissionsExt` in the PowerShell archive test lane; Linux stable/MSRV and macOS passed. This is test portability, not a reversal of M002/M002a/M003 historical closures, nor a new installer feature. Keep POSIX archive runtime coverage on Unix and PowerShell runtime coverage on Windows. Full four-lane strict closure at `plans/closure/bootstrap-installers/002b-status.md` is a hard gate for CI M003i. This addendum supersedes prior 'no open bootstrap work' statements as to current corrective work.

**M002b closed** on implementation `88ddf2e796ac3674b306b51726c65ed7cc9bd29a`; hosted run `37806244313` passed Linux stable, Linux 1.89, macOS, and Windows. The POSIX archive runtime test is Unix-gated and still executes on both Unix lanes; the Windows `pwsh 7` + `tar.exe` PowerShell runtime test remains enabled and passed. Formal evidence: `plans/closure/bootstrap-installers/002b-status.md`. This strict closure unblocks CI M003i.
