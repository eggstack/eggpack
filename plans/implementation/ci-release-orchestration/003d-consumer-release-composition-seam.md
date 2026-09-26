# CI and Release Orchestration Milestone 003d — Consumer Release Composition Seam

Status: ready for handoff

Repository baseline: `8a451ba8f98cd058d861b3283af92c6412deec15`

Source roadmaps:

- `plans/subsystems/ci-release-orchestration-roadmap.md`
- `plans/subsystems/ecosystem-adoption-roadmap.md`

First proving consumer:

- `eggstack/eggsact@174764c5c71130ec98fee18c445fcecb3e35eb25`

Hard dependencies:

- CI M003c closed: `plans/closure/ci-release-orchestration/003c-status.md`
- Bootstrap M002a closed: `plans/closure/bootstrap-installers/002a-status.md`
- Build/Qualification M003/M004 closed
- ADR-0003 and ADR-0004 remain authoritative

Primary class: capability / consumer-composition / qualification extension / staging assets

## 1. Objective

Add the narrow composition seams required by the first real Eggpack consumer without absorbing product-owned release/install semantics into Eggpack.

M003d also separates static checked-in workflow shape from runtime release identity. The first consumer review proved that a checked-in CI graph cannot truthfully embed the Git SHA of the same revision that contains that graph.

The eggsact adoption review proves two requirements not represented by the current producer model:

1. eggsact publishes product-owned `install.sh` / `install.ps1` wrappers that intentionally implement latest/version selection and Cargo fallback, while Eggpack currently reserves those names for generated exact-release first-install scripts;
2. eggsact release qualification runs a bounded MCP stdin/stdout handshake against the exact staged candidate, while Eggpack core qualification currently supports only direct candidate execution with fixed argv.

M003d must support these requirements without creating a generic command DSL, arbitrary staging-asset surface, or product-specific logic inside Eggpack.

## 2. Current incompatibility evidence

### Installer composition

Current Eggpack staging always creates:

- `release-manifest.json`
- `install.sh`
- `install.ps1`

where the two installers are generated from `eggpack-bootstrap` and are exact-release, first-install-only scripts.

Current eggsact public installers at the reviewed baseline:

- accept `--version X.Y.Z` / `-Version X.Y.Z`;
- default to `releases/latest/download`;
- recognize an unsupported host or exact binary HTTP 404 and fall back to exact-version Cargo install;
- keep checksum/version/transport failures as hard errors;
- choose product-owned install destinations and PATH advice.

Replacing those public files with Eggpack's generated exact-release bootstrap would be a user-visible regression and violates the producer/product ownership boundary.

### Candidate qualification

Eggpack `CandidateSmokeBinding` invokes only the exact candidate executable plus fixed argv.

eggsact additionally runs:

```text
python3 scripts/smoke-mcp-binary.py <candidate>
```

The script performs a bounded MCP initialize -> initialized notification -> tools/list handshake, checks server identity and `math_eval`, then requires clean EOF shutdown.

The migration must preserve this exact-candidate protocol evidence.

### Reusable workflow identity

Current `CIPlan` / release orchestration graph copies `release_id` and `source_revision` from an exact `ReleasePlan`.

That is sufficient for fixed fixtures but not a reusable repository release workflow:

- the exact tag/source revision does not exist when a future workflow is generated;
- committing a ReleasePlan containing the commit's own SHA changes the SHA, so a self-consistent checked-in exact ReleasePlan is impossible;
- M003c now correctly compares checked-out HEAD to ReleasePlan.source_revision, making this contradiction fail rather than remain hidden;
- `GitHubDraftPolicyV1` likewise contains an exact tag/title and therefore needs a runtime resolved instance for future tags.

The first consumer must not solve this with placeholders or by disabling source verification.

## 3. Non-goals

M003d does not:

- add arbitrary supplemental release files;
- add arbitrary shell commands;
- add generic plugins/hooks;
- teach Eggpack eggsact MCP semantics;
- replace eggsact self-update policy;
- implement latest/version selection in Eggpack bootstrap;
- implement Cargo fallback in Eggpack bootstrap;
- publish releases;
- create/move tags;
- change DistributionContract, ReleaseManifest, or Eggup receipts;
- generalize from one consumer beyond the minimum reusable seam.

## 4. Static workflow shape vs runtime release identity

Introduce an explicit separation between reusable checked-in release workflow configuration and one invocation's exact release identity.

### A. Static checked-in workflow shape

Add a versioned static release-workflow model containing only identity-independent information needed to render the checked-in workflow:

- canonical selected target set;
- target build/host/toolchain/floor/support policy from PackConfig;
- build bindings;
- core qualification bindings;
- consumer validator configuration;
- staging intent/provider;
- installer-presentation mode;
- GitHub runner/action/tool pins;
- checked-in config paths.

It MUST NOT contain release_id, source_revision, an exact future tag, a GitHub release id, or artifact digests/sizes.

The exact type may be `ReleaseWorkflowPlanV2`, `ReleaseShapeV1`, or an equivalent bounded schema. Preserve historical `CIPlan` v1 / `ReleaseCIPlanV1` evidence rather than silently reinterpreting it.

### B. Runtime identity resolution

Add a finite internal CLI command such as `eggpack ci _resolve-release`.

The command receives explicit contract, PackConfig, selected targets, exact selected tag, checked-out source revision, static GitHub draft template, and output paths.

It must:

1. validate the exact existing tag selected by the workflow event/input;
2. verify/consume checked-out HEAD as source revision;
3. resolve PackConfig against DistributionContract through existing `PackConfig::resolve`;
4. use the exact tag as the opaque `release_id` for this first adoption rather than inferring/stripping product syntax;
5. emit the invocation-local `ReleasePlan`;
6. resolve the static GitHub draft template into exact `GitHubDraftPolicyV1`;
7. write both only into invocation-private workflow storage, never back into the repository.

Using the exact tag as release_id avoids implicit product-specific semver transformation. Eggsact's published asset names are versionless, so this does not alter them.

### C. Static GitHub draft template

Add a bounded static template for fields known before the future tag:

- owner/repository;
- fixed title prefix such as `eggsact `;
- bounded fixed body/notes;
- prerelease flag;
- token environment;
- timeout/body/page bounds.

Runtime resolution appends the exact validated tag to the title prefix and sets the exact tag. Do not add arbitrary string templating.

### D. Generated workflow preflight

For a staging-enabled reusable workflow:

1. checkout the event-selected exact tag per M003c;
2. derive/verify HEAD commit;
3. resolve invocation-local ReleasePlan and GitHub draft policy;
4. upload those runtime documents as an internal preflight artifact;
5. every build/qualify/gate/aggregate/stage job downloads and consumes those exact documents;
6. `_verify-source` compares every source checkout against the runtime ReleasePlan.

No job may consume a checked-in release-specific source SHA.

### E. Deterministic drift model

`eggpack ci generate/check` operates only on static workflow shape/template.

Tests must prove:

- the same static config renders byte-identical workflow bytes independent of future tag;
- two runtime tags produce distinct ReleasePlan/GitHubDraftPolicy instances while using identical checked-in workflow bytes;
- runtime source mismatch still fails M003c verification.

## 5. Installer presentation policy

Add a strict versioned staging installer policy owned by the provider/staging layer.

Suggested model:

```text
InstallerPresentationV1
  GeneratedDefault
  ProductWrappers {
    posix_source
    powershell_source
    generated_posix_name
    generated_powershell_name
  }
```

Exact naming is implementation-defined.

### GeneratedDefault

Preserve current behavior byte-for-byte:

- generated POSIX script staged as `install.sh`;
- generated PowerShell staged as `install.ps1`;
- no product wrapper source files.

### ProductWrappers

Stage four installer assets:

- product-owned source file -> public `install.sh`;
- product-owned source file -> public `install.ps1`;
- Eggpack-generated exact POSIX installer -> configured non-colliding name;
- Eggpack-generated exact PowerShell installer -> configured non-colliding name.

For eggsact the planned exact-installer asset names are:

- `install-exact.sh`;
- `install-exact.ps1`.

The product wrappers remain the public `latest/download/install.*` compatibility surface.

The generated exact installers remain available as evidence that Eggpack's qualified bootstrap projection still matches the same manifest.

## 6. Product wrapper source rules

Wrapper source paths are explicit repository-relative paths.

Requirements:

- non-empty and bounded;
- no absolute path;
- no `..`, backslash ambiguity, NUL, or control characters;
- exact checked-out source tree only;
- source file must be regular and non-symlink;
- bounded size, recommended <= 1 MiB per wrapper;
- bytes copied exactly; no template interpolation;
- no executable evaluation during payload construction;
- product wrapper asset names are fixed to `install.sh` and `install.ps1`;
- generated exact-installer names are safe flat names and may not collide case-insensitively with artifacts, sidecars, manifest, or public wrappers.

The stage job already verifies checked-out source identity through M003c. Product-wrapper bytes therefore come from the same exact source revision as the ReleasePlan/manifest.

## 7. Staging payload/evidence model

Do not weaken exact-set reconciliation.

All four installer files become ordinary expected staging assets with:

- exact asset name;
- exact relative staged path;
- exact size;
- lowercase SHA-256;
- media type;
- semantic kind/source classification.

Prefer additive staging kinds such as:

- generated POSIX installer;
- generated PowerShell installer;
- product POSIX wrapper;
- product PowerShell wrapper.

If changing the serialized enum can break existing v1 payload readers, either:

- add a backward-compatible source field with defaults; or
- bump the staging payload schema explicitly.

Document and test the compatibility choice. Do not silently reinterpret old payload JSON.

The GitHub adapter continues to reconcile the complete exact remote asset set.

## 8. CLI/stage preparation changes

Extend the finite `_prepare-stage` input contract with an explicit installer-presentation policy and source root.

Suggested inputs:

```text
--installer-presentation <path>
--source-root <checked-out repository root>
```

Rules:

- source root itself must be a real non-symlink directory;
- resolved wrapper paths must remain beneath it;
- source root is used only for explicitly named wrapper files;
- no directory scanning/globbing;
- generated exact installers still use the existing contract/manifest/bootstrap policy/origin.

Generated CI passes the checked-out repository root explicitly.

## 9. Consumer-owned exact-candidate validator

Add one narrow post-core-qualification verifier type to the executable CI graph.

This is intentionally not part of `QualificationEvidence`; M003 remains the provider-neutral binary qualification authority.

Suggested model:

```text
ConsumerValidatorV1 {
  selector
  interpreter: Python3
  script
  timeout_ms
  stdout_limit
  stderr_limit
}
```

Initial supported interpreter is exactly `python3`.

No arbitrary executable field, shell, environment map, or caller-supplied argument vector.

Invocation is fixed:

```text
python3 <validated-script-path> <exact-candidate-path>
```

This is sufficient for the proven eggsact MCP smoke while remaining materially narrower than a command DSL.

## 10. Validator source/security rules

Validator script:

- explicit repository-relative path;
- regular non-symlink file;
- source checkout already verified by M003c;
- bounded size;
- no traversal;
- no PATH-selected script;
- interpreter string is not configurable beyond the finite enum.

Process:

- shell-free;
- candidate path appended by Eggpack;
- bounded timeout/output;
- cancellation integrated with existing build/qualification cancellation semantics where practical;
- environment cleared or reduced to a documented allowlist;
- working directory is explicit;
- no network authority granted by Eggpack;
- process outcome captured without stdout/stderr contents in durable evidence.

If Python is unavailable on a runner, required validation fails rather than silently skipping.

## 11. Consumer-validator graph semantics

For each target with a consumer validator:

```text
build
  -> core qualify
  -> consumer validate exact candidate
  -> required gate
  -> aggregate
```

The consumer validation job consumes the canonical qualification handoff, including exact candidate bytes already validated by Eggpack.

It must not rebuild the binary.

Required target validator failure prevents aggregation.

Non-gating target validator behavior follows the same release-suppression semantics as existing optional qualification: no partial release.

Add bounded `ConsumerValidationEvidenceV1` containing only:

- schema version;
- release/source/target identity;
- selector;
- validator kind;
- process outcome summary;
- candidate size/SHA identity.

No script output contents.

## 12. Local executable harness

Extend the local orchestration harness so consumer validation can be executed from the same typed runner-command model used by GitHub rendering.

Required fixture:

- a temporary candidate process;
- a temporary checked-in-style Python validator;
- success;
- non-zero;
- timeout;
- output-limit;
- missing Python;
- wrong candidate identity;
- validator symlink/path escape.

This proves the renderer is not the sole source of truth.

## 13. eggsact proving configuration

After M003d implementation, the Eggpack-side eggsact adoption plan will configure:

### Product wrappers

- `packaging/install.sh` -> `install.sh`;
- `packaging/install.ps1` -> `install.ps1`;
- generated exact scripts:
  - `install-exact.sh`;
  - `install-exact.ps1`.

### Consumer validator

For every target whose candidate runs natively:

- interpreter: Python3;
- script: `scripts/smoke-mcp-binary.py`;
- candidate selector: direct output;
- timeout >= current script's bounded 15-second request/shutdown windows plus process overhead;
- output bounds small and explicit.

The core candidate smoke remains responsible for direct candidate CLI validation such as `--version` or `--help`.

The consumer validator owns only the MCP handshake.

## 14. Generated workflow requirements

When the consumer seam is configured:

- all existing M003c source verification stays intact;
- wrapper source files are read only in stage preparation;
- consumer validator runs after core qualification and before aggregate;
- validator receives exact handoff candidate bytes;
- stage still remains the only `contents: write` job;
- no new secret scope;
- no product script runs in the write-authorized stage job unless it is only copied as bytes;
- `ci check` detects removal/reordering of the consumer validator and wrapper configuration.

Staging-disabled and consumer-extension-disabled workflows remain byte-compatible with current M003c goldens.

## 15. Tests

### Installer presentation

- GeneratedDefault exactly preserves existing payload behavior;
- ProductWrappers produces four installer assets;
- wrapper bytes copied exactly;
- wrapper symlink rejects;
- wrapper path traversal rejects;
- wrapper size bound rejects;
- generated-name collision rejects;
- contract artifact collision rejects;
- same input produces identical payload/digests;
- GitHub exact-set reconciliation includes all four installers;
- existing M003a direct/bundle/archive generated-installer fixtures remain green.

### Consumer validator

- successful Python validator gates through;
- nonzero fails;
- timeout fails;
- output-limit fails;
- missing interpreter fails;
- script symlink/traversal rejects;
- exact candidate identity mismatch rejects;
- required failure blocks aggregate;
- optional failure suppresses release rather than producing partial output;
- durable evidence contains no script stdout/stderr.

### Regression

- M003c tag-source/source-verifier behavior unchanged;
- only stage has write permission;
- no publication/tag mutation;
- old no-extension golden workflows byte-compatible.

## 16. Documentation

Update:

- root README;
- `eggpack-ci` README;
- `eggpack-github` README;
- `eggpack-bootstrap` README if installer naming/presentation needs explanation;
- ecosystem adoption roadmap;
- CI/release orchestration roadmap;
- registry.

Document the ownership boundary explicitly:

- generated exact installer = Eggpack first-install evidence;
- product wrapper = consumer-owned selection/fallback/install UX;
- consumer validator = bounded consumer-owned release evidence, not Eggpack qualification semantics.

## 17. Verification

At minimum:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted Linux stable, Rust 1.89, macOS, and Windows must pass.

No real eggsact draft is required for M003d closure. That is M003b/Ecosystem M001 operational evidence.

## 18. Acceptance criteria

M003d closes only when:

- reusable checked-in workflow configuration contains no future release_id/source_revision/tag;
- runtime preflight resolves exact ReleasePlan + GitHub draft policy from the selected tag and checked-out source;
- the same checked-in workflow bytes work for distinct future tags while M003c exact-source verification remains enforced;
- product-owned public wrappers can coexist with generated exact installers without filename collision;
- wrapper bytes are exact source-revision bytes with bounded path/type/size validation;
- exact remote staging inventory includes wrappers and generated installers;
- eggsact's MCP script can run as a bounded exact-candidate validator without arbitrary command support;
- validator failure gates/suppresses release correctly;
- validator evidence is bounded and identity-linked;
- existing M003c safety invariants remain;
- extension-disabled workflows/payloads preserve compatibility;
- all local/hosted checks pass;
- no unresolved medium-or-higher consumer-composition issue remains.

On closure, Eggpack Ecosystem M001 and the mirrored eggsact M005 adoption plan become dependency-ready for implementation.

## 19. Stop conditions

Stop and re-plan if:

- static/dynamic identity separation requires weakening M003c source verification;
- reusable workflow generation still requires a checked-in future source SHA or exact tag;
- product wrappers require arbitrary staging-directory scanning;
- validator support requires arbitrary executables/shell/env;
- Python-only finite validation cannot express eggsact's existing release smoke;
- generated exact installers cannot coexist without changing DistributionContract semantics;
- staging schema compatibility cannot be preserved or explicitly versioned;
- consumer validation must mutate product source or candidate bytes;
- live publication authority becomes necessary.

## 20. Closure evidence

Create:

`plans/closure/ci-release-orchestration/003d-status.md`

Record:

- implementation SHA;
- static workflow schema and compatibility decision;
- two-tag runtime identity resolution evidence;
- proof no checked-in release-specific source SHA/tag is required;
- installer-presentation schema/compatibility decision;
- eggsact wrapper fixture evidence;
- generated exact installer names and digests;
- validator process/evidence matrix;
- exact candidate identity linkage;
- required/optional gate behavior;
- direct/bundle/archive regression results;
- M003c invariant review;
- hosted matrix;
- unresolved findings;
- explicit eggsact M001/M005 readiness disposition.
