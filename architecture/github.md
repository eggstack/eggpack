# `eggpack-github` — Deep Dive

GitHub draft release staging adapter + local staging payload materializer
(CI M003a). `crates/eggpack-github/src/lib.rs` (3000+ lines), tests in
`src/tests.rs` (~1850 lines).

Two bounded capabilities:

1. **Staging payload** — materialize `StagingPayloadV1` from the M004 finalized
   root + `ReleaseManifest` + `eggpack-bootstrap` generators: copy exact
   artifacts/sidecars, write standalone `release-manifest.json` (round-trip
   verified), generate deterministic `install.sh`/`install.ps1` with exact-tag
   origin. M004 root semantics unchanged.
2. **Draft adapter** — reconcile the exact payload into a GitHub draft via the
   provider-specific `GithubApi` seam. Actions wiring lives in `eggpack-ci`
   (M003b), not here.

## Key types / functions (`src/lib.rs`)

**Local payload:**

- `GithubError` + `fail()` (`:38-52`) — redacted bounded errors.
- `GitHubDraftPolicyV1` (`:54-154`): `schema_version == 1`, `owner`,
  `repository`, `tag`, `title`, `body`, `prerelease`, `token_env
  (=GITHUB_TOKEN)`, `request_timeout_secs` (5–120), `max_metadata_bytes` (≤8M),
  `max_list_pages` (≤32). `from_json/to_json/validate/download_origin()`
  (`https://github.com/<owner>/<repo>/releases/download/<tag>`).
- `StagingAssetKind` (`:166-183`): `FinalizedArtifact, ChecksumSidecar,
  ReleaseManifest, PosixInstaller, PowershellInstaller` (+ M003d
  `ProductPosixWrapper, ProductPowershellWrapper`; schema stays v1, old readers
  fail closed).
- `StagingAsset{name, path == name, size != 0, sha256, media_type, kind}`
  (`:188-201`).
- `StagingPayloadV1` (`:206-293`): `product_id, release_id, source_revision,
  owner, repository, tag, title, prerelease, body, assets[1-1024]` sorted by
  name; `from_json` (≤1MiB) / `to_json` / `validate`.
- `GitHubDraftReceiptV1` (`:298-368`): producer evidence — `draft == true`,
  `immutable == false`, `github_release_id`, `created`,
  `uploaded + reused == len(assets)`.
- `prepare_staging_payload(contract, manifest, finalized_root, policy,
  install_policy, output_dir)` (`:559-903`): contract-expansion vs manifest
  revalidation, exact finalized inventory match, digest/sidecar checks, absent
  private `0700` output, manifest canonical round-trip, exact-tag installer
  guard (`latest/download` rejected).
- M003d: `InstallerPresentationV1 / InstallerPresentationModeV1
  {GeneratedDefault, ProductWrappers} / ProductWrapperSourcesV1` (`:966-1077`,
  `MAX_WRAPPER_BYTES = 1MiB`); `prepare_staging_payload_with_presentation()`
  (`:910-943`, inner `:1216-1609`); `GitHubDraftTemplateV1{owner, repository,
  title_prefix, body, prerelease, …} + resolve(tag)` (`:1113-1213`, appends
  validated tag, no templating).

**Draft adapter:**

- `RefTarget / TagObject / RemoteRelease / RemoteAsset` (`:1615-1667`).
- `trait GithubApi: Send + Sync` (`:1674-1734`): `get_ref, get_tag,
  list_releases, create_release, list_assets, upload_asset(file: Box<dyn
  Read+Send>, length, content_type), delete_asset`.
- `verify_tag_source()` (`:1778-1814`): lightweight tag == `source_revision`;
  annotated tags peeled ≤8 (`MAX_TAG_PEEL_DEPTH`).
- `stage_with_bytes / stage_with_source / stage_with_dir` (`:1949-2251`):
  policy/payload agreement, pre+post tag verify (TOCTOU guard), paginated draft
  lookup, exact asset reconcile, narrow 502 recovery, final exact-set verify,
  receipt emit. Production `stage_with_dir` streams in 64KiB chunks
  (`UPLOAD_CHUNK_BYTES`).
- `EggfetchTransport` (`:2264-2621`): fixed `https://api.github.com` /
  `https://uploads.github.com`, `API_VERSION = 2026-03-10`,
  `USER_AGENT = eggpack-github/0.1.0`; per-request `Accept /
  X-GitHub-Api-Version / Bearer`; redacted `Debug`; lean `eggfetch-core`
  (retries/redirects disabled, 3xx fail closed); `draft:true,
  make_latest:false`; upload `201` + exact `name/size/state == uploaded /
  digest == sha256:<hex>` checks; parsed exact upload origin + query-pair
  encoding. 404/422/502 mapped; narrow 502 recovery only (delete exactly one
  `starter + size == 0`, still fail; next explicit rerun may resume).
- `FixtureGithub` (`:2653-2816`): deterministic in-memory seam (tag/ref/seeded
  releases, asset pages, 502/422/rename/digest/size faults); `ASSET_PAGE_SIZE =
  100`, `list_all_assets` default 16 / max 32 pages, incomplete final page fails
  closed.
- `read_token(GITHUB_TOKEN)` (`:493-502`) — env-only credentials.

## Safety properties

- **Draft-only:** created/existing releases must satisfy `draft == true &&
  immutable == false`; receipt validates the same. Title/prerelease/body/tag
  mismatch fails without mutation.
- **Exact existing tag only:** `refs/tags/<tag>` must equal
  `payload.source_revision`, verified before *and* after staging; never
  create/move/force-update/delete tags; no `target_commitish`.
- **Exact-set reconciliation:** pre+post `list_all_assets` across all bounded
  pages (`per_page = 100`); duplicate same-name, unexpected, or size/digest
  mismatch (`digest == sha256:<hex>` required for reuse) fails closed without
  clobber; upload rename/size/digest mismatch fails; `422_duplicate` fails.
- **Credentials:** env-only, absent-before-I/O, never in argv/config/evidence/
  errors/`Debug` (`<redacted>`).
- **Local:** symlink rejection, flat `name == path` (no traversal/`..`/
  absolute/`\`), auxiliary collision guard (`release-manifest.json/install.sh/
  install.ps1` + generated names, case-insensitive), deterministic sorted
  payload, exact-tag installer origin only.

## Boundaries (no publication)

Must not own build/qualification/finalization, bootstrap semantics (only calls
`eggpack-bootstrap::render_*_with_policy`), CI graph rendering, publication,
tag mutation, registries, Issues/PRs, signing trust. No API/CLI path publishes:
no `publish` endpoint/subcommand, no `draft:false` flag (`deny_unknown_fields`),
no `gh release --clobber`, no `curl`, no tag create/move/delete. Stage job
copies product wrappers as bytes only, never executes.

## Dependencies / dependents

- Deps (`Cargo.toml:14-26`): `eggpack-contract`, `eggpack-manifest`,
  `eggpack-core`, `eggpack-bootstrap` + `serde/serde_json/sha2`,
  `tokio[rt,time,macros,net,io-util]`, `eggfetch-core 0.2.0`
  (`standard-http1,tls-rustls,tls-native-roots,json`), `bytes`,
  `futures-util`, `url`. Dev: `tokio[rt-multi-thread]`.
- Dependents: `eggpack-cli` (hard dep — implements `_prepare-stage /
  _stage-github-draft / _aggregate / _resolve-release / _validate-consumer`);
  `eggpack-ci` dev-dep only (production renderer stays provider-neutral).
