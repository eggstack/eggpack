# Bootstrap Installers Roadmap

Status: active

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

M003 two-consumer adoption and optional Eggup receipt handoff design.

## 8. Cross-cutting requirements

ShellCheck/PSScriptAnalyzer where available, no secrets in generated text, safe temp dirs, download timeouts, checksum tool fallbacks explicitly tested.

## 9. Verification strategy

Local HTTP fixture server, checksum mismatch, 404, timeout, unsupported arch, wrong candidate version, existing install preservation, shell parse tests, Windows parse/tests.

## 10. Risks and decision points

Complete generation may become opaque. Prefer small deterministic templates/fragments if that remains more reviewable.

## 11. Completion definition

At least two consumers have removed duplicated mapping/checksum authority from bootstrap installers.

## 12. Milestone status

M001 generator model and direct installer fixtures is closed and qualified. M002 is historical implementation/closure evidence with its M002a corrective closed.

M003 is now **unblocked for candidate review**. Its completion definition — real two-consumer adoption evidence — is satisfied as of 2026-10-04 (Planning Hygiene M001). Both Eggsact and StegoEggo adopted Eggpack-generated exact installers in their producer cutovers, and the second consumer's previously-outstanding operational condition has cleared: StegoEggo's ordinary stable `v0.5.0` (Eggpack run `37181914252`, published 2026-10-04) is a live public release carrying both Eggpack-generated exact installers (`install-exact.sh`, `install-exact.ps1`) alongside the product wrappers, and the consumer asset audit plus public installer smoke passed.

Its earlier blocker chain — `StegoEggo Release-Distribution M004 public 0.4.3 live receipt -> Ecosystem M002 full closure -> Bootstrap M003 candidate review` — is fully discharged, with `0.4.3` superseded by `0.5.0` on consumer-side semver grounds. No Bootstrap M003 implementation is authorized by that evidence; the milestone now needs its own candidate review and implementation plan.

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 direct generator + fixtures | closed | `plans/implementation/bootstrap-installers/001-direct-installer-generator.md` | `plans/closure/bootstrap-installers/001-status.md` | direct first-install semantics qualified; archive-content/finalization evidence remains later work |
| M002 bundle/archive bootstrap safety | closed historically; corrective closed | `plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md` | `plans/closure/bootstrap-installers/002-status.md` | Post-closure PowerShell archive runtime evidence gap corrected and qualified by M002a |
| M002a PowerShell archive runtime evidence | closed | `plans/implementation/bootstrap-installers/002a-powershell-archive-runtime-evidence-corrective.md` | `plans/closure/bootstrap-installers/002a-status.md` | Closed on implementation `4d2270a` + hosted run 36154905956 (attempt 1, all lanes green) via shared closeout pass `plans/implementation/ci-release-orchestration/002a-ci-bootstrap-closure-registry-pass.md` |
| M003 two-consumer adoption/receipt decision | unblocked for candidate review | — | — | Two-consumer evidence satisfied: eggsact `v1.2.7` and stegoeggo `v0.5.0` (run `37181914252`) both ship Eggpack-generated exact installers publicly. Candidate review and a new implementation plan are the remaining steps; no implementation authorized yet |
