# Bootstrap Installers Milestone 002a — PowerShell Archive Runtime Evidence Corrective

Status: ready for handoff

Repository baseline: 86321e1d7485e5e5eed923cbbff9e52140b3c16b

Historical M002 plan:

- plans/implementation/bootstrap-installers/002-bundle-archive-bootstrap-safety.md

Historical M002 closure:

- plans/closure/bootstrap-installers/002-status.md

Source roadmap:

- plans/subsystems/bootstrap-installers-roadmap.md

Applicable ADRs:

- plans/adrs/ADR-0001-producer-consumer-release-boundary.md
- plans/adrs/ADR-0002-contract-plan-manifest-separation.md

Primary class: corrective / runtime qualification / archive safety evidence

## 1. Objective

Complete the runtime qualification evidence required by Bootstrap M002 for PowerShell archive installers and strengthen archive negative tests so they exercise member-inventory/type/path defenses rather than failing earlier at outer archive digest verification.

M002a is primarily an evidence/test corrective. Production bootstrap rendering should change only if actual PowerShell archive execution exposes a defect.

Bootstrap M003 remains blocked until M002a closes and the separate real-consumer adoption evidence/candidate review requirement is satisfied.

## 2. Post-closure evidence gap

At repository baseline 388a8b056b05fb324122b1ac618e20bcb2963f77:

- POSIX bundle runtime is executed;
- POSIX archive runtime is executed;
- PowerShell bundle runtime is executed under pwsh 7;
- PowerShell archive output is rendered and parser-checked;
- PowerShell archive installation is not executed end-to-end.

This falls short of the M002 acceptance criterion requiring archive POSIX and PowerShell runtime qualification for TarGzip.

A second evidence issue exists in the POSIX malicious-archive matrix: several test archives mutate archive contents while retaining the original manifest artifact digest. Those cases can fail at outer archive SHA-256 verification before reaching the intended member inventory/path/type checks.

The generated guards exist, but the runtime tests do not prove all intended branches.

## 3. Boundaries to preserve

Do not redesign BootstrapInstallPolicyV1, DistributionContract, ReleaseManifest, TarGzip-only archive policy, first-install-only semantics, no-overwrite placement, caller-owned executable/data modes, Eggup update ownership, or origin/download policy.

No new archive encoding, package manager, elevation, service setup, or receipt semantics.

## 4. Corrective invariants

- PowerShell archive runtime must actually execute on the qualified Windows lane;
- required PowerShell/tar prerequisites may not silently skip and still count as closure evidence;
- positive archive runtime must install exact expected bytes;
- no-overwrite must be exercised;
- malicious archive tests must pass outer archive size/hash before testing inner member defenses;
- each negative case must identify the defense layer it reached;
- destination remains unchanged on every verification failure;
- rollback removes only invocation-created files;
- parser/static checks remain in addition to runtime execution;
- production changes are allowed only if runtime evidence exposes a real defect.

## 5. Required PowerShell archive runtime matrix

On a Windows-capable hosted lane with pwsh 7 and tar.exe:

### Positive

Generate a host-specific archive contract/manifest/policy and local HTTP fixture, then execute the generated PowerShell archive installer.

Assert archive download, outer size/hash, exact member inventory, private-temp extraction, nested-source flattening, exact installed bytes, no-overwrite, and pre-existing-file preservation.

The test must fail if pwsh 7 or tar.exe is unavailable on the Windows qualification lane. It may skip on platforms not designated to qualify PowerShell runtime.

### Archive integrity negatives

Execute wrong archive size and wrong archive SHA. These fail before member listing/extraction and install nothing.

### Inner archive negatives

For each inner negative, construct archive bytes first, then update the manifest artifact size/SHA to match those exact tampered bytes so outer verification succeeds.

The expected contract/member manifest remains authoritative for member relationships.

Required inner cases:

- missing member;
- extra member;
- traversal member;
- absolute member;
- alternate-separator/backslash member;
- symlink member;
- directory in place of expected regular file.

Each case must prove outer archive verification passed, the intended listing/path/type guard rejected the archive, and no final installed files exist.

If the Rust tar builder refuses unsafe traversal/absolute names, use a test-only minimal tar-byte helper capable of constructing the raw header needed to exercise the generated consumer guard. Do not weaken producer validation to create malicious fixtures.

### Member evidence negatives

Build an archive with the expected member inventory and valid outer archive digest, then mutate only manifest member evidence: wrong member size and wrong member SHA.

Assert failure after extraction/member discovery but before placement.

### Tool-boundary negative

Execute with tar.exe unavailable from the PowerShell runtime environment.

Assert bounded tar-required error, no extraction, and no installed files.

## 6. PowerShell transactional placement/rollback evidence

M002 claims all-or-nothing placement for bundle/archive.

Add a deterministic archive-specific PowerShell placement failure test.

Preferred fixture:

1. allow verification/extraction to complete;
2. arrange for first final move to succeed;
3. cause a later final move to fail without pre-existing state being owned by the installer;
4. assert the first invocation-created destination is rolled back;
5. assert unrelated/pre-existing paths remain untouched.

Use a test harness race/fault mechanism outside production policy. Do not add a general test hook or arbitrary command to generated installers.

If Windows filesystem semantics make this exact injection infeasible, document the reason and prove the shared rollback primitive through bundle runtime plus an archive-specific no-overwrite/pre-existing race case. Do not claim archive rollback execution without evidence.

## 7. Strengthen POSIX inner-defense tests

Correct the existing POSIX archive malicious-member matrix using the same outer-digest principle:

- tampered archive bytes must have matching manifest artifact size/hash;
- expected member records remain unchanged;
- test assertions should verify failure reaches inventory/path/type checking.

Where practical, capture bounded stderr and assert the expected guard category, not merely nonzero exit.

Retain unavailable tar, pre-existing destination, member size/SHA negatives, and symlink/type negatives.

## 8. Test fixture architecture

Add reusable test-only helpers for exact host archive cases, deterministic safe TarGzip, tampered TarGzip with outer manifest evidence updated, raw unsafe-name tar fixtures where needed, local multi-request HTTP server, and PowerShell runtime prerequisite verification.

Keep malicious archive creation test-only. Production M004 remains the sole producer finalization path.

## 9. CI qualification requirements

Windows hosted CI must explicitly verify pwsh -Version and tar.exe availability, then execute a focused PowerShell archive runtime test by name before or in addition to the full workspace suite.

The focused test must not return early when a required Windows prerequisite is absent.

Retain PowerShell 5.1 parser coverage where useful, but parser success is not runtime evidence.

Linux/macOS continue qualifying POSIX archive runtime as regression guards.

## 10. Production-code policy

Start with test/evidence corrections.

If PowerShell archive positive or negative runtime exposes a renderer defect, add the failing regression first, make the smallest renderer correction, preserve M002 architecture and first-install boundary, and record the defect and production delta in closure.

Do not refactor the whole bootstrap generator unless required by demonstrated failure.

## 11. Required tests

At minimum:

### PowerShell archive runtime

- positive exact install;
- nested source flattening;
- no overwrite;
- pre-existing destination preservation;
- wrong archive size;
- wrong archive SHA;
- missing member after valid outer hash;
- extra member after valid outer hash;
- traversal member after valid outer hash;
- absolute member after valid outer hash;
- backslash/alternate separator after valid outer hash;
- symlink member after valid outer hash;
- directory member after valid outer hash;
- wrong member size;
- wrong member SHA;
- missing tar.exe;
- archive placement failure/rollback evidence or documented shared-primitive substitute.

### POSIX correction

- missing/extra/traversal/absolute/symlink/type cases use matching outer archive evidence;
- intended inner guard is reached;
- no installed files.

### Static/determinism

- same input renders same PowerShell archive script;
- PowerShell parser still passes;
- no elevation/update/latest/redirect/arbitrary extractor behavior introduced.

## 12. Verification

Run cargo fmt, workspace check, strict Clippy, eggpack-bootstrap tests, workspace tests, docs, Rust 1.89 check/tests, scripts/check-local.sh, and git diff --check.

Hosted qualification must include Linux stable, Linux Rust 1.89, macOS, Windows, and an explicit focused Windows PowerShell archive runtime test.

Record whether ShellCheck/PSScriptAnalyzer are present; they remain supplementary rather than substitutes for runtime evidence.

## 13. Planning/closure reconciliation

On registration:

- preserve Bootstrap M002 as historical implementation/closure evidence;
- annotate plans/closure/bootstrap-installers/002-status.md with the post-closure runtime-evidence gap;
- mark Bootstrap M002a active/ready;
- keep Bootstrap M003 blocked on both M002a closure and real adoption evidence/candidate review.

Do not alter CI M002 status from this corrective; CI has its separate M002a plan.

## 14. Acceptance criteria

M002a closes only when:

- generated PowerShell archive installer executes successfully on Windows with pwsh 7 + tar.exe;
- exact bytes/member flattening/no-overwrite are proven;
- all required inner malicious archive cases pass outer digest verification and then fail at the intended inner defense;
- member size/hash negatives reach the intended member evidence checks;
- missing tar.exe fails before placement;
- POSIX inner-defense tests are corrected similarly;
- rollback/pre-existing preservation evidence is truthful and explicit;
- stable/MSRV/macOS/Windows CI passes;
- no unresolved medium-or-higher archive runtime/safety finding remains.

Only then may Bootstrap M003 be considered for planning, and only after its independent real-consumer adoption evidence requirement is satisfied.

## 15. Stop conditions

Stop and re-plan if safe PowerShell TarGzip extraction cannot be demonstrated with the fixed tar.exe boundary, Windows archive semantics require a new extractor dependency, contract/manifest needs new archive metadata, transactional placement requires persistent locks or ACL ownership policy, or a fix would introduce update/receipt/service authority.

## 16. Closure evidence

Record corrective implementation SHA, production-code delta or explicit none, focused Windows PowerShell archive runtime output, pwsh/tar prerequisite evidence, positive archive install bytes, corrected malicious archive matrix with proof outer digest passed, member evidence negative matrix, rollback/pre-existing evidence, POSIX corrected matrix, hosted CI, unresolved findings, and explicit Bootstrap M003 readiness disposition.