# Bootstrap Installers Roadmap

Status: proposed

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

M001 generator model + direct installer fixtures.

M002 bundle/archive bootstrap safety.

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

Blocked on Release Manifest M001; contract conformance M002 is closed.
