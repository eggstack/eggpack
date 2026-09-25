# CI and Release Orchestration Milestone 003a — Local Staging Payload and GitHub Draft Adapter

Status: ready for handoff

Repository baseline: `49aa849c471c6927fc5fe3c899e418779b65a40b`

Source roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`

Hard/interface dependency closures:

- CI M002a: `plans/closure/ci-release-orchestration/002a-status.md`;
- Bootstrap M002a: `plans/closure/bootstrap-installers/002a-status.md`;
- Build/Qualification M004: `plans/closure/build-qualification/004-status.md`;
- ADR-0003: `plans/adrs/ADR-0003-checked-in-generated-ci-and-publication-gate.md`.

External implementation baseline:

- `eggstack/eggfetch@3a2e23f1953505bf052df6e9c79478bfcad49d78`;
- `eggfetch-core 0.2.0`, using a lean HTTPS client profile rather than default retry/redirect policy.

External platform evidence reviewed 2026-09-25:

- GitHub REST Releases API version `2026-03-10`;
- draft-first staging is recommended when immutable releases are enabled;
- published immutable releases lock tag/assets;
- upload asset names are unique and upload responses expose size/state/digest;
- a failed upload may leave a zero-byte `starter` asset after a 502;
- "get release by tag" is documented as published-release-only, so rerunnable draft reconciliation must not depend on that endpoint alone.

Primary class: capability / provider adapter / release staging / security-sensitive network I/O

## 1. Objective

Implement the provider boundary needed to assemble an exact, reviewable GitHub **draft** release from an Eggpack-qualified finalized release without introducing publication authority.

M003a owns two bounded capabilities:

1. materialize a complete local staging payload from M004 finalization + ReleaseManifest + deterministic bootstrap generators;
2. reconcile that exact payload into a GitHub draft release through a provider-specific adapter.

M003a does **not** wire the adapter into generated GitHub Actions yet. That is M003b.

## 2. Architecture decision review

No new ADR is required for this slice.

ADR-0003 already selects checked-in GitHub Actions as the initial provider surface, explicitly allows build/qualification/aggregation to stage a draft release, and keeps public publication as a maintainer action. The long-term roadmap explicitly names a GitHub Releases staging adapter for Phase 8.

Stop and require a new ADR if implementation needs:

- a non-GitHub provider protocol;
- automatic public publication;
- tag creation/movement authority;
- registry publication;
- signing/authenticity trust policy;
- a general remote-release backend abstraction beyond the narrow GitHub draft adapter.

## 3. Ownership boundary

```text
M004 finalized root + ReleaseManifest
            |
            v
local staging payload materializer
            |
            +--> release assets + checksum sidecars
            +--> release-manifest.json
            +--> install.sh
            +--> install.ps1
            |
            v
GitHub draft staging adapter
            |
            +--> exact existing tag/source preflight
            +--> create/reuse draft
            +--> exact asset reconciliation
            |
            X no tag creation/move
            X no publish endpoint
            X no crates.io/PyPI publication
```

Product policy continues to own:

- exact repository;
- exact release/tag name;
- exact release title/notes;
- prerelease intent;
- release/version selection;
- bootstrap origin/install policy.

Eggpack owns:

- validating finalized release evidence;
- deterministic staging payload construction;
- GitHub draft reconciliation and least-authority failure semantics.

## 4. Key invariants

- only an already-existing exact tag may be staged;
- tag must resolve to the exact `ReleaseManifest.source_revision`;
- annotated tags are peeled with a bounded loop to an exact commit;
- Eggpack never creates, moves, force-updates, or deletes tags;
- draft is always `true` for any created release;
- there is no API/CLI path in M003a that publishes a release;
- published or immutable releases are never mutated;
- unexpected remote assets fail closed;
- same-name remote assets are reused only when exact size + SHA-256 digest match;
- mismatched same-name assets are not clobbered or silently deleted;
- a zero-byte `starter` asset created by a failed upload is the only narrowly recoverable remote deletion case;
- credentials are accepted only from an environment variable and never serialized/logged;
- no token appears in argv, config, evidence, errors, or Debug output;
- HTTP retries are disabled;
- redirects are disabled for API mutation calls;
- bounded timeouts/body sizes apply to every request;
- local staging input/output paths reject symlinks/traversal;
- M004 release-root semantics remain unchanged.

## 5. Production structure

### A. Add provider-specific crate

Add `crates/eggpack-github`.

It owns only:

- GitHub release/tag REST models needed by staging;
- exact-tag source preflight;
- draft lookup/create;
- release asset listing/upload;
- narrow stale-`starter` cleanup;
- bounded staging receipt/status.

It must not own:

- build/qualification/finalization;
- bootstrap semantics;
- CI graph rendering;
- publication;
- tag mutation;
- GitHub Issues/PRs/general API.

Suggested dependencies:

- `eggfetch-core = 0.2.0` with explicit lean features:
  - `standard-http1`;
  - `tls-rustls`;
  - `tls-native-roots`;
  - `json`;
- `serde`, `serde_json`, `sha2`;
- `tokio` only as required to execute the async transport.

Do not use the default eggfetch feature set if it enables logical retry/redirect behavior not required by this adapter.

### B. Fixed production origins

Production endpoints are fixed to:

- `https://api.github.com`;
- the upload host returned by GitHub, constrained to `https://uploads.github.com`.

Tests may inject a loopback fixture endpoint through a test-only transport/configuration seam.

Production config must not accept arbitrary API base URLs.

### C. GitHub API headers/auth

Every production request uses:

- `Accept: application/vnd.github+json`;
- `X-GitHub-Api-Version: 2026-03-10`;
- bounded `User-Agent`;
- `Authorization: Bearer ...` sourced only from a named environment variable, initially `GITHUB_TOKEN`.

Token absence fails before network I/O.

Token text must be redacted from all diagnostics.

## 6. Local staging payload

### A. Standalone manifest file

Do not change M004's finalized release-root inventory.

Additively update CI aggregation output so the internal `eggpack-finalized/` handoff contains:

```text
root/                     # unchanged M004 contract artifacts + sidecars only
summary.json
release-manifest.json     # deterministic ReleaseManifest::to_json()
```

`release-manifest.json` is CI/staging handoff material, not a new file inside the M004 release root.

The aggregate CLI must verify that the bytes written to `release-manifest.json` decode back to the exact returned M004 manifest.

### B. Staging payload model

Add a bounded deterministic `StagingPayloadV1` model, preferably in `eggpack-github` or another clearly provider-facing module.

At minimum:

- schema version;
- product id;
- release id;
- source revision;
- owner/repository;
- exact existing tag;
- draft title;
- prerelease boolean;
- bounded release notes;
- ordered asset records.

Each asset record carries:

- safe flat asset name;
- local relative path under payload root;
- byte size;
- lowercase SHA-256;
- media type;
- semantic kind:
  - finalized artifact;
  - checksum sidecar;
  - release manifest;
  - POSIX installer;
  - PowerShell installer.

No absolute filesystem paths in the serialized payload.

### C. Materialize complete asset set

Add a narrow CLI command such as:

```text
eggpack ci _prepare-stage
```

Exact spelling is implementation-defined.

Inputs are explicit:

- contract;
- `release-manifest.json`;
- finalized root;
- GitHub staging policy;
- BootstrapSpec / BootstrapInstallPolicyV1 as needed;
- output staging directory;
- output payload JSON.

The command:

1. parses/validates contract + manifest;
2. revalidates every finalized release asset/sidecar against the manifest/contract;
3. rejects missing/extra finalized-root files;
4. copies/hard-links exact finalized assets into a private staging payload directory;
5. writes fixed-name `release-manifest.json`;
6. generates deterministic `install.sh` and `install.ps1` through `eggpack-bootstrap`;
7. validates all staging asset names are globally unique;
8. computes exact size/SHA-256 for every staged file;
9. emits deterministic `StagingPayloadV1`.

Recommended fixed auxiliary asset names:

- `release-manifest.json`;
- `install.sh`;
- `install.ps1`.

Reject a contract whose release asset names collide with these fixed staging names.

### D. Bootstrap origin

Generated installers use the exact version/tag origin:

```text
https://github.com/<owner>/<repo>/releases/download/<tag>
```

Do not generate a `latest/download` origin in M003a.

Product-specific latest/version-selection/fallback wrappers remain consumer-owned.

## 7. GitHub staging policy

Add strict `GitHubDraftPolicyV1`.

At minimum:

- schema version;
- owner;
- repository;
- exact tag;
- release title;
- bounded notes/body;
- prerelease boolean;
- token environment variable name fixed/allowlisted (default `GITHUB_TOKEN`);
- request timeout;
- maximum metadata response bytes;
- maximum bounded release-list pages.

Validation:

- owner/repo/tag/title contain no control/path/query injection;
- tag is not inferred from ReleasePlan;
- body has a documented upper bound;
- no `draft=false` / publish flag exists;
- no "latest" or tag-creation option exists.

## 8. Exact tag/source preflight

Before creating/reusing a draft:

1. fetch exact `refs/tags/<tag>`;
2. if lightweight tag points directly to commit, require its SHA equals `source_revision`;
3. if annotated tag, peel tag objects with a bounded maximum depth (e.g. 8);
4. require terminal object type = commit;
5. require exact commit SHA = `source_revision`;
6. re-run this verification after asset reconciliation before reporting staged success.

Missing/moved/mismatched tag fails closed.

No tag write endpoints are implemented.

## 9. Draft lookup semantics

The GitHub "get release by tag" endpoint is documented for **published** releases, so M003a must not use it as the sole draft lookup.

For rerunnable draft staging:

- list releases with authenticated bounded pagination;
- find exact matching `tag_name`;
- reject duplicate same-tag release records if observed;
- if none exists, create a new draft release;
- if one exists:
  - require `draft == true`;
  - require `immutable == false`;
  - require exact tag;
  - require acceptable title/prerelease/body policy;
  - otherwise fail without mutation.

Do not patch an existing release from published to draft or vice versa.

## 10. Draft creation

Create release using the exact existing tag.

Request must set:

- `tag_name` exact;
- `name` exact;
- `body` exact;
- `draft: true`;
- `prerelease` exact;
- `make_latest: false`.

Do not supply a branch-style `target_commitish` as authority for creating a tag. The tag must already exist and be preflighted.

The response must still match:

- requested tag;
- `draft == true`;
- `immutable == false`.

## 11. Exact asset reconciliation

Before uploading:

- list all current release assets;
- require remote asset names are within the exact expected payload set, except for a narrowly recognized recoverable `starter` record;
- for each expected asset:
  - if no remote record: upload;
  - if one remote `uploaded` record exists: require exact size and `digest == sha256:<local digest>`; reuse;
  - if same name exists with mismatched size/digest: fail;
  - if duplicate same-name records exist: fail.

After each upload:

- require HTTP 201;
- require response name exactly equals expected safe name;
- require state `uploaded`;
- require exact size;
- require exact SHA-256 digest when provided;
- if GitHub renames the asset, fail.

At completion, re-list and require exact expected set and exact evidence.

## 12. 502 / starter recovery

GitHub documents that a failed asset upload may return 502 and leave an empty asset with state `starter`.

Recovery is intentionally narrow:

- after 502, list release assets;
- if exactly one asset with the expected name is `starter` and zero bytes, delete that asset;
- report the staging attempt as failed;
- do not automatically retry the upload in the same invocation;
- next explicit invocation may resume.

No other remote asset deletion is authorized.

## 13. Staging receipt

Return/write bounded `GitHubDraftReceiptV1`:

- schema version;
- owner/repository;
- release id;
- tag;
- source revision;
- GitHub numeric release id;
- draft state;
- immutable state;
- ordered asset names/sizes/digests;
- whether release was created or reused;
- count of uploaded/reused assets.

No token, Authorization header, raw API URL containing query credentials, or unbounded response body.

Receipt is producer staging evidence, not an Eggup InstallReceipt.

## 14. CLI

Add a narrow staging command, for example:

```text
eggpack ci _stage-github-draft
```

Inputs:

- staging payload JSON;
- GitHub draft policy;
- output receipt path.

Credential:

- environment only.

The CLI may create a current-thread Tokio runtime for this command only.

Do not add a public `publish` subcommand.

## 15. Failure/restart semantics

The adapter must be safely rerunnable.

A rerun:

- re-verifies tag/source;
- reuses an exact draft;
- reuses exact uploaded assets;
- uploads only missing assets;
- fails on mismatched/unexpected assets;
- never clobbers;
- never publishes.

If failure occurs after draft creation but before completion, the incomplete draft remains visible for maintainer inspection and a later explicit rerun.

Do not delete the release automatically.

## 16. Security review

Explicitly review:

- token redaction;
- fixed production hosts;
- HTTPS/TLS;
- no redirects/retries;
- API response body bounds;
- annotated tag peeling bounds;
- tag/source TOCTOU (verify before and after staging);
- asset name injection/query encoding;
- local symlink/traversal handling;
- draft/public/immutable state transitions;
- only-stage-job write permission in the future M003b renderer;
- 502 starter cleanup authorization;
- malicious/unexpected remote asset behavior.

## 17. Required tests

### Local payload

- direct finalized root -> exact payload;
- bundle finalized root -> exact payload;
- archive finalized root -> exact payload;
- standalone manifest exactly matches M004 manifest;
- missing/extra finalized file rejects;
- tampered finalized file rejects;
- symlink rejects;
- auxiliary asset collision rejects;
- generated installers deterministic;
- installer origin exact-tag only;
- staging payload deterministic.

### GitHub adapter fixture server

- lightweight exact tag accepted;
- annotated tag peeling accepted;
- tag mismatch rejects before release mutation;
- excessive tag-object depth rejects;
- no release -> draft created;
- existing exact draft reused;
- published release rejects;
- immutable release rejects;
- wrong draft tag/title/prerelease policy rejects;
- existing exact assets reused;
- missing asset uploaded;
- mismatched same-name asset rejects;
- unexpected asset rejects;
- renamed upload response rejects;
- 422 duplicate-name response fails closed;
- 502 + exact zero-byte starter -> starter deleted, invocation fails, next invocation can resume;
- 502 without exact starter does not delete anything;
- upload digest/size mismatch rejects;
- post-stage tag movement rejects;
- token absent rejects before I/O;
- diagnostics redact token.

## 18. Verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-cli --all-targets --all-features --locked
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-github --locked
cargo package -p eggpack-github --locked --allow-dirty
cargo package -p eggpack-cli --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-github --all-targets --locked
./scripts/check-local.sh
git diff --check
```

Hosted Linux stable, Rust 1.89, macOS, and Windows must remain green.

No real GitHub draft is required to close M003a; network semantics are qualified against a deterministic loopback GitHub API fixture. Live draft evidence is reserved for M003b.

## 19. Documentation

Update:

- root README;
- `crates/eggpack-github/README.md`;
- `crates/eggpack-cli/README.md`;
- CI/release orchestration roadmap;
- registry;
- closure `plans/closure/ci-release-orchestration/003a-status.md`.

Document clearly:

- draft only;
- exact existing tag only;
- no publication;
- credentials environment-only;
- incomplete drafts may remain after failure;
- exact rerun semantics.

## 20. Acceptance criteria

M003a closes only when:

- complete direct/bundle/archive staging payloads are deterministic;
- standalone manifest + both bootstrap installers are included;
- M004 root semantics remain unchanged;
- GitHub adapter verifies exact existing tag/source;
- draft creation/reuse is bounded/idempotent;
- published/immutable releases reject;
- asset reconciliation is exact and non-clobbering;
- 502 starter cleanup is narrow and tested;
- token secrecy/redaction is proven;
- no publication/tag-write endpoint exists;
- all local/hosted verification passes;
- no unresolved medium-or-higher staging safety issue remains.

M003b becomes dependency-ready only after this closure.

## 21. Stop conditions

Stop and re-plan if:

- GitHub draft staging cannot be implemented without tag creation/movement;
- exact asset reconciliation requires general remote deletion/clobber;
- a publish endpoint becomes necessary;
- GitHub API behavior requires arbitrary provider URLs;
- eggfetch-core cannot perform bounded raw uploads without weakening the transport policy;
- staging payload requires changing M004 final release identity semantics;
- installer generation requires product-specific latest/fallback policy inside Eggpack.

## 22. Closure evidence

Record:

- implementation SHA;
- eggfetch external baseline/version/features;
- staging payload examples for direct/bundle/archive;
- exact standalone manifest/installers evidence;
- fake GitHub API transcript matrix;
- tag-peeling cases;
- exact draft create/reuse behavior;
- asset reconciliation matrix;
- 502 starter cleanup matrix;
- token-redaction tests;
- dependency/package/MSRV results;
- hosted matrix;
- unresolved findings;
- explicit M003b readiness disposition.
