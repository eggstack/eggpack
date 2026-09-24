# Bootstrap Installers Milestone 001 — Direct Installer Model and Deterministic POSIX/PowerShell Fixtures

Status: ready for handoff

Repository implementation baseline: `0b1c3b9795280dea610c2fbe9d1533591d610b3f` (Contract M001/M002 and Manifest M001/M001a closed)

Source roadmap: `plans/subsystems/bootstrap-installers-roadmap.md`

Long-term references:

- `plans/000-long-term-specification.md#13-bootstrap-installers`
- `plans/001-terminology-and-domain-model.md#13-bootstrap-installer`
- `plans/002-long-term-roadmap.md#phase-6--bootstrap-installer-generationconformance`

Applicable ADRs:

- `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`
- `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`

Primary class: capability / infrastructure / security-sensitive generation

## 1. Objective

Implement the first bounded bootstrap-installer generator for exact direct-artifact releases.

M001 creates a deterministic producer-side installer model plus readable POSIX shell and PowerShell renderers that derive target/artifact/digest identity from DistributionContract + ReleaseManifest rather than maintaining independent platform/asset/checksum tables. Generated installers perform first-install acquisition and integrity verification only; normal update remains application/Eggup-owned.

## 2. Readiness and dependencies

Hard dependencies are satisfied:

- Contract conformance M002 is closed.
- Release Manifest M001/M001a is closed and provides exact canonical target, file, install-name, size, and SHA-256 evidence.
- Build engine/finalizer is not required because local fake-release fixtures can provide valid manifest/artifact inputs.

This plan may execute independently of Manifest M002 and Build/Qualification M001.

## 3. Current evidence

Eggstack consumers currently contain repeated shell/PowerShell release logic, including target mapping, asset naming, checksum verification, temporary download staging, and installation.

The reusable authority now exists in:

- `eggpack-contract` for supported target/asset/install naming;
- `eggpack-manifest` for exact finalized direct artifact size/digest evidence.

M001 deliberately starts with direct artifacts. Bundle/archive installation safety remains Bootstrap M002.

## 4. Invariants

- Generated installers contain no independently maintained target/artifact mapping; all mappings are projections of validated contract + manifest input.
- The generator accepts one exact resolved ReleaseManifest and therefore does not decide “latest”, compare versions, or select a release.
- Release origin is explicit product/caller policy supplied to the generator; it is not discovered.
- Generated installers use exact URLs formed from the supplied fixed origin plus validated flat artifact names.
- Production origin policy accepts HTTPS only. Tests MAY use loopback HTTP under an explicit fixture-only mode.
- Unsupported OS/architecture combinations fail closed before download.
- Redirect behavior is disabled in M001 unless same-origin enforcement can be proved portably; do not silently follow arbitrary origins.
- Network operations have explicit connect/total timeouts.
- Bytes are downloaded to a private temporary area, never directly to the live destination.
- Exact expected size and SHA-256 are verified before installation.
- Integrity is not authenticity.
- Existing destination files are not overwritten in M001. Bootstrap M001 is first-install semantics; an existing destination fails safely and users should use the application/Eggup update path.
- No implicit sudo/admin elevation.
- The generated script must be readable, deterministic, and testable.
- Installer generation must not become a general shell/PowerShell template or command DSL.
- Generated script behavior must not create Eggup receipt semantics.

## 5. Scope

### In scope

- dedicated renderer crate `crates/eggpack-bootstrap` or equivalent isolated package consistent with the canonical layer model;
- bounded `BootstrapSpec`/policy model;
- exact ReleaseManifest + DistributionContract validation before rendering;
- direct-artifact-only target projection;
- deterministic POSIX shell renderer;
- deterministic PowerShell renderer;
- explicit install-directory argument/default policy seam;
- exact size + SHA-256 checks;
- safe temp staging and cleanup;
- first-install refusal on existing destination;
- exact platform mapping for currently represented native target families;
- generation/check API suitable for later CLI wiring;
- deterministic golden fixtures;
- local fake HTTP release integration tests where practical.

### Out of scope

- bundle/archive installation;
- self-update;
- service lifecycle;
- overwrite/rollback of existing installations;
- ownership inference;
- package-manager/source-build fallback;
- automatic release selection/latest lookup;
- GitHub API coupling;
- redirect chains;
- signatures/authenticity;
- arbitrary candidate validation commands;
- installer receipt emission;
- publication/upload.

## 6. Required production changes

### A. Isolated bootstrap renderer package

Add `eggpack-bootstrap` as a small producer-side renderer package depending on:

- `eggpack-contract`;
- `eggpack-manifest`;
- only minimal serialization/support crates if needed.

It MUST NOT depend on Eggup, HTTP client libraries, async runtimes, shell execution libraries, service managers, or build backends.

The library generates text. Network/process behavior occurs only when a generated script is deliberately executed in tests or by a user.

### B. BootstrapSpec / product policy seam

Define a bounded generator input containing only policy not already owned by contract/manifest, such as:

- exact fixed release origin/base URL;
- optional explicit product display label;
- installation-directory policy/default;
- whether generated file is POSIX or PowerShell;
- fixture-only allowance for loopback HTTP.

Do not include target filenames, digests, install names, version ordering, or alternate mirrors in this policy object.

If an install directory default is supported, it must be explicit caller policy and safely quoted. The script MUST also support an explicit install-directory override suitable for fixture testing. Do not inject arbitrary shell source fragments.

### C. Contract/manifest consistency gate

Before rendering:

1. require matching product identity;
2. require one exact release identity/version used for contract expansion;
3. validate every manifest target against the contract;
4. require direct artifact form for all generated target cases in M001;
5. require manifest artifact name/install identity to equal contract expansion;
6. reject targets present only in one side;
7. reject duplicate/unsupported platform projections.

The generator must fail before emitting a script if contract and manifest drift.

### D. Platform projection

Implement a bounded, provider-independent mapping from runtime OS/architecture observations to canonical manifest targets.

Initial runtime observations SHOULD cover, where present in the input manifest:

- Linux x86_64;
- Linux aarch64;
- Linux ARMv7 if explicitly represented;
- macOS x86_64;
- macOS arm64;
- Windows x86_64;
- Windows arm64.

Do not guess libc/ABI variants. If multiple canonical targets would map from the same shell-observable OS/arch and cannot be disambiguated safely, fail generation and require later policy/interface work.

The generated script contains only projections derived from its exact input manifest.

### E. POSIX shell renderer

Generate portable, readable shell with at least:

- strict error handling appropriate to POSIX/sh scope;
- deterministic OS/arch detection;
- explicit unsupported-platform error;
- exact artifact URL;
- bounded download using a supported tool policy;
- private temp directory and cleanup trap;
- exact Content-Length-independent local byte-size check;
- SHA-256 verification with a bounded fallback set of standard local tools (for example `sha256sum`, `shasum -a 256`, or `openssl dgst -sha256`);
- refusal if no supported hash tool is available;
- destination nonexistence check before final placement;
- no `sudo`;
- final rename/move only after verification.

If both `curl` and `wget` are supported, their timeout/redirect/error semantics must be explicitly equivalent enough for this contract. Otherwise choose one initial downloader and fail clearly when unavailable rather than creating complex fallback policy.

### F. PowerShell renderer

Generate equivalent first-install semantics using PowerShell-native primitives:

- deterministic architecture/OS handling;
- exact URL;
- private temp path;
- explicit request timeout/redirect policy where supported;
- exact size check;
- `Get-FileHash -Algorithm SHA256`;
- destination nonexistence check;
- no elevation request;
- cleanup on success/failure.

### G. Generate/check API

Expose deterministic renderer functions and a byte/string comparison helper suitable for later:

```text
eggpack installer generate
eggpack installer check
```

M001 does not add the CLI itself.

For equal validated inputs, generated text MUST be byte-identical.

## 7. Ordered work packages

1. Add `eggpack-bootstrap` package and authority/dependency documentation.
2. Define bounded BootstrapSpec and renderer error model.
3. Implement contract/manifest direct-layout consistency projection.
4. Implement deterministic platform mapping.
5. Implement POSIX renderer.
6. Implement PowerShell renderer.
7. Add golden generation/check tests.
8. Add static safety scans for forbidden elevation/release-selection/hidden mapping patterns.
9. Add local fake-release execution tests for supported host scripts where feasible.
10. Qualify package/MSRV/docs/dependencies/hosted CI.
11. Write closure and re-evaluate Bootstrap M002/adoption readiness.

## 8. Failure, restart, and contention semantics

Generation is pure and has no runtime side effects.

Generated installer execution is first-install-only:

- any download/hash/size/platform failure leaves the destination absent/unchanged;
- existing destination fails before replacement;
- temporary files are cleaned on handled exit;
- re-running after a pre-install failure is safe;
- simultaneous installation into the same destination is not made transactional in M001; an atomic no-clobber final-create primitive SHOULD be used where practical, otherwise document the residual race and stop if it cannot fail safely.

Do not add a broad lock/service/rollback mechanism; those belong to Eggup for normal updates.

## 9. Compatibility and migration

M001 does not require consumers to replace their current installers. It produces qualified generated fixtures first.

No existing consumer installer should be deleted until an adoption milestone compares behavior and closes.

The generated installer is tied to one exact manifest/release. Product policy remains responsible for deciding which release-specific installer/origin a user receives.

## 10. Required tests

At minimum:

- direct Eggsact-like multi-target generation;
- deterministic shell golden;
- deterministic PowerShell golden;
- contract/manifest product mismatch rejects;
- release/target/artifact/install mismatch rejects;
- bundle/archive manifest rejected by M001 renderer;
- unsupported runtime platform exits before download;
- fixed origin validation; production HTTP rejected;
- loopback HTTP accepted only in fixture mode;
- URL/path/shell metacharacter injection inputs rejected or safely quoted;
- timeout flags/options rendered;
- redirect following absent/disabled;
- successful local direct install verifies size and SHA then installs;
- checksum mismatch leaves destination unchanged;
- size mismatch leaves destination unchanged;
- HTTP 404/failure leaves destination unchanged;
- existing destination is preserved and installer fails;
- missing digest tool fails before install on POSIX fixture where testable;
- temp cleanup on failure;
- no `sudo`, package-manager fallback, release discovery/latest API, or Eggup receipt logic in generated text;
- shell syntax parse (`sh -n`) on supported CI;
- PowerShell parse/AST execution on Windows CI;
- unchanged contract/manifest suites.

## 11. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test -p eggpack-manifest --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-bootstrap --locked
cargo package -p eggpack-bootstrap --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Additionally execute generated shell/PowerShell fixture validation on the relevant hosted operating systems and record exact commands/tools available. ShellCheck/PSScriptAnalyzer are supplementary when available; their absence alone is not a closure blocker if syntax and behavioral tests cover the generated scripts.

## 12. Documentation updates

Update:

- root README;
- `crates/eggpack-bootstrap/README.md`;
- bootstrap installers roadmap;
- registry;
- closure `plans/closure/bootstrap-installers/001-status.md`.

Document prominently:

- first-install-only semantics;
- no release selection;
- no overwrite/update ownership;
- integrity vs authenticity;
- exact release-specific generation;
- Build engine is not required for generation tests.

## 13. Acceptance criteria

M001 closes when:

- direct release mappings/digests are derived from contract+manifest only;
- deterministic readable POSIX and PowerShell outputs exist;
- unsupported platforms fail closed;
- exact size/SHA are verified before installation;
- production origins are fixed/validated and network operations bounded;
- existing installations are never overwritten;
- no elevation/update/service/release-selection policy is introduced;
- local fake-release positive/negative behavior is qualified;
- package/MSRV/docs/dependency/hosted CI pass;
- no unresolved medium-or-higher installer safety finding remains.

## 14. Stop conditions

Stop for ADR/replanning if:

- supporting ordinary direct releases requires a generic shell/PowerShell DSL;
- target mapping cannot be derived unambiguously from canonical manifest/contract data;
- safe first-install requires Eggup-style ownership/rollback machinery;
- release selection/latest lookup becomes necessary inside Eggpack;
- redirects or multiple origins are required without a narrow enforceable trust policy;
- generated scripts must parse arbitrary application semantics.

## 15. Closure evidence required

Record:

- exact implementation/review baselines;
- renderer package/dependency inventory;
- contract+manifest→platform mapping matrix;
- byte-exact shell/PowerShell goldens;
- local fake-release success and failure matrix;
- timeout/redirect/origin behavior;
- no-overwrite/no-elevation evidence;
- syntax/analyzer results by platform;
- package/MSRV/docs/hosted CI;
- unresolved findings;
- whether Bootstrap M002 and simple adoption become ready.

## 16. Handoff notes

This milestone deliberately proves only direct first-install generation. Do not fold bundle/archive atomicity, Eggup receipts, service lifecycle, package fallback, or general self-update into M001.

It can be implemented in parallel with Manifest M002 because it consumes the already-closed Contract/Manifest schema and lives in an isolated renderer package.
