# `eggpack-github` — Deep Dive

`eggpack-github` is the staging payload materializer and draft-only GitHub
adapter — the only networked, credentialed component in the workspace. It has
two bounded jobs: turn a finalized release into a local, inspectable
`StagingPayloadV1`, and reconcile that payload into a **draft** release on an
exact pre-existing tag. It never publishes.

`crates/eggpack-github/src/lib.rs` (3025 lines), tests in `src/tests.rs`
(1848 lines). `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` at
`lib.rs:9-10`, so every public item carries a doc comment. All citations below
are to `crates/eggpack-github/src/lib.rs` unless another path is given.

## Responsibility and the publication boundary

This is the most safety-critical crate in the workspace: it is the only one
holding a credential and the only one opening a socket. The publication
boundary is enforced in code, not by convention.

| Boundary | Enforced where | Citation |
| --- | --- | --- |
| Draft-only | `create_release` body hardcodes `"draft": true`; no user input reaches it | `2483-2490` |
| Draft-only | created release re-asserted as a mutable draft before use | `2026-2028` |
| Draft-only | a pre-existing release must already be a mutable draft | `2032-2034` |
| Draft-only | receipt is always `draft: true, immutable: false`, and re-validated | `2204-2205`, `353-355` |
| Never "latest" | `"make_latest": "false"` sent as a string | `2489` |
| Exact existing tag | tag must peel to `payload.source_revision`, checked before **and** after staging | `1990-1997`, `2185-2192` |
| Exact existing tag | the trait has no tag create/update/delete method at all | `1674-1734` |
| No tag mutation | no `target_commitish` in the create body; the tag is never created or moved | `2483-2490` |
| No `--clobber` | the string does not occur in this crate; the only mutating calls are create, upload, and the narrow starter delete | `2602-2626` |
| No auto-publish | no `publish` endpoint and no `draft: false` field; `deny_unknown_fields` rejects such a document | `56`, `2487` |
| No immutable overwrite | `immutable` is read from the API and must be false on create and on reuse | `2380-2381`, `2026`, `2032` |

`--clobber` is a workspace-wide non-goal enforced statically in `eggpack-ci`
(`crates/eggpack-ci/src/lib.rs3777`, `:6901`), not by a flag validated here.
Publication remains a separate human action: nothing in this crate ever flips a
draft to published.

## Key types / functions (with `file:line`)

### Policy, payload, receipt

| Item | Line | Description |
| --- | --- | --- |
| `GithubError` | `40` | Newtype over a bounded, redacted message; `Display` `42-46`, `fail()` `50-52`. No response body is ever embedded. |
| `GitHubDraftPolicyV1` | `57` | Strict per-run policy: `schema_version`, `owner`, `repository`, `tag`, `title`, `body`, `prerelease`, `token_env`, `request_timeout_secs`, `max_metadata_bytes`, `max_list_pages`; `deny_unknown_fields` at `56`. |
| `.from_json` / `.to_json` | `106` / `117` | Parses within `MAX_POLICY_JSON` (256 KiB, `26`) and validates; serialization validates first. |
| `.validate` | `123` | `schema_version == 1`; owner/repo/tag/title/body bounds; `token_env` must be `GITHUB_TOKEN` (`132`); timeout 5–120 s; metadata ≤ 8 MB; pages 1–32. |
| `.download_origin` | `148` | `https://github.com/<owner>/<repo>/releases/download/<tag>` — the exact-tag installer origin. |
| `StagingAssetKind` | `166` | `FinalizedArtifact`, `ChecksumSidecar`, `ReleaseManifest`, `PosixInstaller`, `PowershellInstaller`, `ProductPosixWrapper`, `ProductPowershellWrapper`. The payload schema stays v1; a v1 reader that does not know the wrapper variants rejects such a payload (fail-closed) rather than misreading it — `158-163`. |
| `StagingAsset` | `188` | `name`, `path` (must equal `name`), non-zero `size`, lowercase `sha256`, `media_type`, `kind`. |
| `StagingPayloadV1` | `206` | The materialized plan: identity, owner/repo/tag, title, prerelease, body, and 1–1024 assets sorted by name. |
| `.from_json` / `.to_json` / `.validate` | `233` / `244` / `252` | Bounded at 1 MiB; validates identity bounds, flat names, `path == name`, non-zero size, digest shape, media-type bound, and case-insensitive uniqueness (`287`). |
| `GitHubDraftReceiptV1` | `298` | Producer evidence of the staged draft: `github_release_id`, `draft`, `immutable`, ordered assets, `created`, `uploaded`, `reused`. |
| `.from_json` / `.to_json` / `.validate` | `329` / `340` / `346` | Requires `draft && !immutable` and `uploaded + reused == assets.len()` (`359-365`). |
| `FIXED_MANIFEST_NAME` | `31` | `release-manifest.json` — a **private** constant, not public API. Companion privates: `FIXED_POSIX_NAME` `32`, `FIXED_POWERSHELL_NAME` `33`. |

### Entry points, remote types, transport

| Item | Line | Description |
| --- | --- | --- |
| `read_token` | `493` | Reads the credential from the environment; the env name must be exactly `GITHUB_TOKEN`. |
| `prepare_staging_payload` | `559` | Materializes the default staging payload from contract + manifest + finalized root + policy. |
| `prepare_staging_payload_with_presentation` | `910` | Same, with an explicit presentation; `GeneratedDefault` delegates byte-identically (`922-929`). |
| `MAX_WRAPPER_BYTES` | `950` | `1 MiB` per product wrapper source. |
| `InstallerPresentationV1` | `966` | `schema_version` + `mode`; `generated_default()` `999`, `validate()` `1024`. |
| `InstallerPresentationModeV1` | `976` | `GeneratedDefault` or `ProductWrappers(ProductWrapperSourcesV1)`. |
| `ProductWrapperSourcesV1` | `986` | Wrapper source paths plus the staged names for the generated exact installers. |
| `GitHubDraftTemplateV1` | `1113` | Identity-independent draft template; must not carry release id, revision, tag, GitHub id, or digests (`1103-1110`). |
| `.resolve(tag)` | `1194` | Validates template and tag, appends the tag to the title prefix, returns a `GitHubDraftPolicyV1`. No templating language. |
| `verify_tag_source` | `1778` | Bounded annotated-tag peel (≤ 8, `25`) down to the exact source revision. |
| `stage_with_bytes` | `1949` | Public reconciliation entry point over in-memory bytes. |
| `stage_with_dir` | `2214` | Public reconciliation entry point over a staging directory. |
| `RefTarget` | `1617` | `{ sha, kind }` for `refs/tags/<tag>`. |
| `TagObject` | `1626` | One annotated-tag peel step. |
| `RemoteRelease` | `1635` | Release summary including `draft`, `immutable`, `upload_url`. |
| `RemoteAsset` | `1656` | `{ id, name, size, state, digest }`; `digest` is `Option<String>` shaped `sha256:<hex>`. |
| `trait GithubApi` | `1674` | The provider seam (`1669-1734`). |
| `EggfetchTransport` | `2264` | Production transport over `eggfetch-core 0.2.0`; `impl GithubApi` at `2395-2627`. |
| `FixtureGithub` | `2661` | In-memory double; `impl GithubApi` at `2824-3022`. |
| `fixture_release` / `fixture_asset` | `2786` / `2808` | Constructors for fixture records. |

## The `GithubApi` seam

`trait GithubApi: Send + Sync` (`1674-1734`) has seven methods: `get_ref`,
`get_tag`, `list_releases`, `create_release`, `list_assets`, `upload_asset`,
`delete_asset`. Futures use return-position `impl Future`, so there is no
`async_trait` boxing.

The trait is why the whole reconciliation flow is testable without a network.
`verify_tag_source` (`1778`), `stage_with_bytes` (`1949`) and `stage_with_dir`
(`2214`) all take `&impl GithubApi`; the transport is the only injected
dependency, so the full sequence — tag resolve, draft lookup, create, upload,
verify, receipt — runs against an in-memory double.

What the trait asks of an implementation:

- Return the typed records, so validation lives in one place in the core rather
  than being re-derived per transport.
- Surface `502` and `422` as errors whose message **starts with** `http_502` /
  `http_422_duplicate` (documented `1714-1715`, classified `1753-1759`). This is
  a message-prefix protocol, not a typed error, and it is how the core applies
  its narrow recovery.
- Treat `delete_asset` as reserved: "only the narrow starter recovery may call
  this" (`1727`).

The public `upload_asset` boundary takes `Box<dyn Read + Send>` (`1723`), while
the internal core holds a seekable handle through the private `ReadSeek` trait
(`1736-1751`) so it can size-check and hash a file before streaming it. Two
implementations exist: `EggfetchTransport` (`2395-2627`) and `FixtureGithub`
(`2824-3022`).

## `FixtureGithub` and the fault injectors

**`FixtureGithub` is not test-gated. It is compiled into the shipped library
and is part of this crate's public API.** The only two conditional attributes in
the crate are `#[cfg(unix)]` at `544` and `#[cfg(test)] mod tests;` at
`3024-3025`. `FixtureGithub` (`2661`), its inherent `impl` (`2665-2783`),
`fixture_release` (`2786`), `fixture_asset` (`2808`) and
`impl GithubApi for FixtureGithub` (`2824-3022`) are all unconditional — even
though the section header above them reads "Deterministic fixture transport for
tests" (`2629-2631`). `stage_with_bytes` (`1949`) is likewise public, and its
only in-repo caller is the test suite.

State lives in `FixtureInner` under a `Mutex` (`2638-2663`), seeded with a
lightweight tag pointing at a commit, ids from 100 / 1000, and
`upload_fail_502_create_starter: true` (`2681`).

| Injector / observer | Line | What it proves |
| --- | --- | --- |
| `with_tag` | `2667` | A lightweight tag resolving straight to the source revision is accepted. |
| `set_ref` | `2692` | Tag movement between the pre- and post-stage verify is caught by the TOCTOU guard (`2185`); an unresolvable tag (`None`) fails. |
| `add_tag_object` | `2699` | Annotated tags peel to the exact commit; a chain deeper than `MAX_TAG_PEEL_DEPTH` (8) fails (`1813`). |
| `seed_release` | `2708` | A pre-seeded published or immutable release makes staging reject without mutation (`2032`). |
| `set_asset_page` | `2719` | Pagination edges: a short final page terminates successfully, a full page on the last allowed page fails closed (`1903-1910`); a duplicate or unexpected asset on a later page is still caught. |
| `fail_upload_502_next` | `2728` | On 502 the core re-lists, deletes a single `state == "starter" && size == 0` record for that name, and still returns an error (`2120-2136`). `create_starter = false` proves the same path with no starter present. |
| `fail_upload_422_next` | `2735` | A duplicate-name 422 fails closed immediately with no recovery (`2117-2119`). |
| `rename_next_upload` | `2741` | The server's echoed name is authoritative — a renamed upload response is rejected (`2098-2100`). |
| `digest_mismatch_next` | `2747` | A wrong `sha256` in the upload response is rejected (`2107-2111`) and in the final set (`2176-2181`). |
| `size_mismatch_next` | `2753` | A wrong size in the upload response is rejected (`2104-2106`). |
| `create_calls` | `2759` | Proves no release was created on a pre-flight failure (tag mismatch, absent token). |
| `upload_calls` | `2764` | Proves no upload was attempted before a local digest failure. |
| `delete_calls` | `2769` | Proves the delete path fires **only** in the narrow 502 starter recovery, never for a pre-existing uploaded asset. |
| `assets_for` | `2774` | Reads back the exact post-stage remote asset set. |

These injectors are the evidence for draft-only, no-clobber, and
verify-after-upload: the only way to observe a remote mutation is a counter the
test itself inspects, and each counter is asserted against a fault that should
have prevented the call.

Two honest limitations of the double: `FixtureGithub::list_releases` returns an
empty vector for any page other than 1 (`2863-2865`), so it does not model
multi-page release listing; and `create_release` always returns
`draft: true, immutable: false` (`2892-2893`), so the "server returned a
non-draft creation" rejection at `2026` is unreachable through it.

## Staging payload materialization

`prepare_staging_payload` (`559-903`) turns a finalized root plus contract,
manifest and policy into an inspectable directory and payload:

1. Validate policy and manifest, require `contract.product.id ==
   manifest.product_id` (`567-573`), reject a symlinked finalized root (`574`).
2. Build the expected inventory from `contract.expand` per manifest target and
   cross-check it against the manifest form — Direct (`582-605`), Bundle (entry
   count and name set, `606-642`), Archive (`643-665`); any other pairing fails
   (`666`).
3. Reject any release asset colliding case-insensitively with
   `release-manifest.json`, `install.sh` or `install.ps1` (`670-679`).
4. Require the finalized root to hold only regular non-symlink files with flat
   names (`685-698`) and to match the expected inventory by **exact set
   equality** — missing and extra both fail (`699-703`).
5. Re-verify artifact size and SHA-256 (`706-716`), and verify each sidecar's
   `sha256  filename` content against the artifact it names, bounded at 4 KiB
   and required to end `.sha256` (`717-757`).
6. Create the output directory only if absent, require a real non-symlink parent,
   and set `0700` on unix (`759`, `533-551`). Every later failure calls the
   `cleanup` closure, which removes the whole output directory (`760-762`).
7. Copy the finalized files exactly (`765-771`); write `release-manifest.json`
   and prove it round-trips to identical bytes (`774-793`).
8. Render deterministic installers with `BootstrapSpec { origin, fixture_http:
   false }` (`796-812`) and reject any `latest/download` origin (`814-817`).
9. Assign media types and kinds, digest each staged file, sort by name
   (`840-883`); finalize and validate the payload (`885-902`).

The payload is deliberately a **separate materialized artifact** rather than an
implicit side effect of uploading. It is a bounded, strictly validated JSON
document, and the adapter re-validates it and re-verifies every local file's
size and digest against it before touching the network (`verify_file`,
`1915-1946`). Preparation and staging are separate CLI subcommands, so the exact
byte set, ordering, digests and kinds are reviewable before any remote object
exists. Determinism is preserved by sorting assets by name in both construction
(`883`) and serialization (`247`).

## Reconciliation: the remote state machine

`stage_with_source` (`1965-2211`) is the whole remote state machine;
`stage_with_bytes` (`1949-1963`) and `stage_with_dir` (`2214-2251`) are thin
adapters over it.

1. Validate the token shape **before any I/O** (`1975-1977`).
2. Validate payload and policy; require exact agreement on owner, repository,
   tag, title, prerelease and body (`1978-1988`).
3. `verify_tag_source` — the tag must already exist and peel to
   `payload.source_revision` (`1990-1997`, implementation `1778-1814`).
4. Paginated release lookup over pages `1..=max_list_pages` (`1999-2012`), then
   `find_exact_release`, which fails on more than one same-tag record
   (`1816-1830`).
5. If absent, `create_release`, then assert the tag matches and the release is a
   mutable draft (`2016-2030`). If present, assert it is a mutable draft
   (`2032`), the tag matches (`2035`), and title, prerelease and body all match
   (`2038-2043`).
6. `check_upload_url` (`2047-2052`) pins the release's `upload_url` to `https`,
   host `uploads.github.com`, the exact
   `/repos/<owner>/<repo>/releases/<id>/assets` path, and no userinfo,
   fragment, or query, port 443 or unset (`1832-1852`).
7. Pre-flight `list_all_assets` (`2061`): reject a case-insensitive duplicate
   remote name (`2065`) and any asset not in the payload (`2068-2069`).
8. For each payload asset in name order (`2080`): absent remotely →
   `verify_file` the local bytes against the payload, then `upload_asset`
   (`2083-2095`), rejecting a response whose `name`, `state`, `size` or `digest`
   disagrees (`2098-2111`); a `422` fails with no recovery (`2117-2119`); a `502`
   re-lists, deletes a single zero-byte `starter` for that name if exactly one
   exists, and still returns an error (`2120-2136`); present remotely requires
   `state == "uploaded"`, exact size, and `digest == "sha256:<hex>"`
   (`2141-2156`).
9. Final `list_all_assets` (`2160`): no duplicates, count must equal the
   expected count, and every expected name must match on state, size and digest
   (`2161-2183`).
10. `verify_tag_source` again (`2185-2192`) — the TOCTOU guard, since the tag
    could have moved mid-run.
11. Emit the receipt with `draft: true`, `immutable: false`, the `created` flag,
    `uploaded` / `reused` counts, and assets sorted by name (`2194-2210`).

Asset pagination (`list_all_assets`, `1893-1913`) fetches pages
`1..=policy.max_list_pages`; a short page (fewer than `ASSET_PAGE_SIZE` = 100,
`36`) terminates successfully, while a full page on the last permitted page
fails closed with "asset pagination bound exhausted". Default 16 pages, maximum
32 (`100-102`, `141-143`).

Bounded transfer: `streamed_upload_body` (`1869-1891`) streams the asset through
`futures_util::stream::try_unfold` in `UPLOAD_CHUNK_BYTES` (64 KiB, `35`) chunks
with an explicit declared length, converting a length that overflows the
platform `usize` into an error. The declared length is checked against the real
file size before the request is built (`1873-1874`, `verify_file` at `1917`).

Source identity: the tag must exist beforehand. The `GithubApi` trait has no
create/update/delete-ref method (`1674-1734`) and the create request omits
`target_commitish` (`2483-2490`), so no code path in this crate can create or
move a tag.

## Token handling and redaction

`read_token(env_name)` (`493-502`) requires `env_name == "GITHUB_TOKEN"` (`494`),
reads `std::env::var` (`497`), and rejects an empty value, one over 4096 bytes,
or one containing control characters (`498`). Every failure returns the fixed
message `"github token is absent"` — the value is never echoed and the
underlying `VarError` is discarded. The same rules are re-checked at every point
of use: the first statement of `stage_with_source` (`1975-1977`) and in
`EggfetchTransport::new` (`2289-2291`). `token_env` is additionally allowlisted
to `GITHUB_TOKEN` in both the policy (`132-134`) and the template (`1174-1176`).

Redaction, as verified:

- `Debug for EggfetchTransport` writes the token field as the literal
  `"<redacted>"` and uses `finish_non_exhaustive`, so no other field is printed
  either (`2271-2278`).
- The token appears in no serializable type: neither `StagingPayloadV1`
  (`206-229`) nor `GitHubDraftReceiptV1` (`298-325`) has a token field, and
  `to_json` validates before encoding.
- No error message in the crate interpolates the token or a response body; all
  failures are fixed strings via `fail()` (`50-52`).
- The test `token_absent_rejects_before_io_and_redacts_diagnostics`
  (`src/tests.rs:1303-1328`) asserts `create_calls() == 0` and
  `upload_calls() == 0` for an empty token, and that a distinctive secret
  appears in neither the error string nor the transport's `Debug` output.

The token is only ever sent to the GitHub API host. Every request URL is built
from `API_BASE` (`https://api.github.com`, `21`) or `UPLOAD_BASE`
(`https://uploads.github.com`, `22`) via `upload_url` (`1854-1867`); the
credential is attached per-request as a bearer `AuthScheme` (`2311-2314`) and
never placed in a URL, query string, or body. `check_upload_url` (`1832-1852`)
additionally rejects a server-supplied `upload_url` pointing anywhere else. The
only non-GitHub host anywhere in the crate is the loopback listener in
`src/tests.rs:1357`, a bare `eggfetch-core` client with no credential.

## Presentation and product wrappers

A **product wrapper** is a thin, hand-written, consumer-owned script that this
crate treats as opaque bytes: it is read from the verified source tree and staged
verbatim as the public `install.sh` / `install.ps1`, while the
Eggpack-generated *exact* installer is staged beside it under a distinct name
(`install-exact.sh` / `install-exact.ps1` in the fixture,
`src/tests.rs:1402-1407`). Per the comment at `1487-1488`, the wrapper is the
friendly public surface while the generated exact installer remains available as
qualified-bootstrap evidence. The crate never parses, interpolates into, or
executes a wrapper — `read_wrapper_source` (`1084-1101`) returns raw bytes,
written unchanged at `1514-1525`.

The mode is opt-in. `generated_default()` (`999-1004`) selects
`GeneratedDefault`, and `prepare_staging_payload_with_presentation` delegates
that mode to `prepare_staging_payload` with no behavioural difference
(`922-929`); `generated_default_preserves_exact_m003a_behavior`
(`src/tests.rs:1418-1458`) asserts byte-identical payloads and identical file
inventories. Only an explicit `ProductWrappers` policy changes the public
surface, and even then the exact generated installers are still staged and still
carry `PosixInstaller` / `PowershellInstaller` kinds (`1551-1560`) while the
wrappers carry `ProductPosixWrapper` / `ProductPowershellWrapper`
(`1561-1570`). Generating the exact installer is therefore the default and stays
available under the wrapper mode; hand-maintained wrapper bytes are the narrow
deviation, not the norm.

Bounds and validation:

- `MAX_WRAPPER_BYTES = 1 MiB` (`950`), enforced in `read_wrapper_source`
  (`1093-1095`) with a re-stat after read (`1097-1099`).
- Wrapper source paths (`validate_wrapper_source_path`, `1050-1066`): bounded,
  relative, no `\`, no NUL, no leading `/`, no `:`, no empty / `.` / `..`
  segments, segments ≤ 128 bytes, no control characters.
- Wrapper sources must be distinct files (`1033-1035`).
- Generated installer names must be flat, must not collide with each other
  case-insensitively, and must not collide with the three reserved staging names
  (`1036-1043`, `1068-1077`).
- The source root must be a real non-symlink directory and the resolved file a
  regular non-symlink file beneath it (`1085`, `1090-1092`).

`GitHubDraftTemplateV1` (`1113-1140`) exists so a repository can hold a static,
identity-independent draft template resolved against a tag at run time. It
deliberately carries no release id, source revision, exact tag, GitHub release
id, or digest (`1103-1110`), and `resolve` (`1194-1212`) only validates the tag
with the same `validate_tag` used by the policy and concatenates it onto the
fixed title prefix. `draft_template_resolves_distinct_tags_from_identical_bytes`
(`src/tests.rs:1799`) pins that identical template bytes resolve to distinct
per-tag policies.

## Safety properties

| Property | Enforced where | Citation |
| --- | --- | --- |
| Created release must be a mutable draft | post-create assertion | `2026-2028` |
| Published / immutable release is never adopted | draft + `!immutable` gate | `2032-2034` |
| Tag must already exist and match the source revision | bounded peel, pre and post | `1778-1814`, `1990-1997`, `2185-2192` |
| No tag creation or mutation | no such trait method; no `target_commitish` | `1674-1734`, `2483-2490` |
| Exact-tag origin only, no `latest/download` | installer content guard | `814-817`, `1483-1486` |
| Local bytes match the payload before upload | size + streamed digest | `1915-1946`, `2084` |
| Upload response name must match | post-upload check | `2098-2100` |
| Upload response state must be `uploaded` | post-upload check | `2101-2103` |
| Fail closed on upload size mismatch | post-upload check | `2104-2106` |
| Fail closed on upload digest mismatch | `sha256:<hex>` comparison | `2107-2111` |
| Reuse requires matching state, size and digest | reuse branch | `2141-2156` |
| Duplicate-name 422 fails with no recovery | error branch | `2117-2119` |
| Unexpected remote asset fails closed | pre-flight inventory | `2068-2069` |
| Duplicate remote name (case-insensitive) fails closed | pre-flight inventory | `2065-2067` |
| Final exact-set verification after upload | post-upload inventory | `2160-2183` |
| Two same-tag release records fail closed | exact-release lookup | `1826-1828` |
| Title / prerelease / body drift fails without mutation | existing-draft comparison | `2038-2043` |
| Upload host pinned to `uploads.github.com` over exact https | URL check | `1832-1852` |
| Asset pagination bound fails closed on a full last page | pagination loop | `1903-1910` |
| 3xx fails closed; no redirect to any host | status gate, read and upload | `2325-2327`, `2586-2588` |
| No retries; bounded per-request time and body size | client built with timeout only, no retry configuration | `2261-2263`, `2299-2302` |
| No deletion of a pre-existing uploaded asset | delete only for `state == "starter" && size == 0`, exactly one match | `2120-2134` |
| No clobber / overwrite of an existing draft | reuse-only path; no replace or overwrite endpoint | `2141-2156` |
| Token absent before any I/O | first statement of the core | `1975-1977` |
| Token redacted in diagnostics | literal `"<redacted>"` in `Debug` | `2271-2278` |
| Symlinks, traversal, non-flat names rejected | validators plus `symlink_metadata` checks | `370-490`, `504-531`, `690-694`, `1085-1092` |
| Output dir created absent-only, `0700`, cleaned on failure | private-dir helper plus `cleanup` | `533-551`, `759-762` |
| Payload / receipt schemas strict and bounded | `deny_unknown_fields`, size and count bounds | `56`, `205`, `297`, `252-292`, `346-367` |

Deliberately **not** enforced here, listed honestly:

- **A cap on total uploaded bytes.** There is a per-request response bound and a
  platform-`usize` conversion (`1873-1874`), but no maximum asset size. A payload
  is bounded by asset count (≤ 1024, `270`) rather than total size.
- **Release-list pagination does not fail closed on bound exhaustion.** The loop
  at `1999-2012` breaks on an empty or short page and otherwise simply stops
  after `max_list_pages`. A target release beyond the bound would be treated as
  absent and the subsequent `create_release` rejected by GitHub; that path
  returns a plain error and applies no narrow recovery, so it still fails closed
  — but as a confusing 422 rather than a clear bound violation.
- **`immutable` defaults to `false` when the API omits it**
  (`#[serde(default)]`, `2386-2387`). This is the one place where a missing
  field resolves to a permissive value, and it is deliberate: making the field
  mandatory would fail *every* release read against an API that does not return
  it, which is a strictly worse failure than the one it prevents. The gate at
  `2032` therefore sees `immutable == false` without having observed it. In
  practice the outcome is still fail-closed — a genuinely immutable release
  rejects the asset upload server-side — but the rejection arrives as a transport
  error rather than a clear "not a mutable draft", and the field is not evidence
  the gate can rely on. Source now carries a comment recording the reasoning.
- **No log-sink redaction**, because the crate has no logging dependency; there
  is no `tracing`/`log` call in `lib.rs`.
- **`--clobber` is not a flag in this crate at all**; it is a workspace-level
  non-goal guarded statically in `eggpack-ci`
  (`crates/eggpack-ci/src/lib.rs3777`, `:6901`).
- **No authenticity, signature, or provenance claim.** Digests in the payload,
  the upload responses and the receipt are SHA-256 integrity facts only
  (`1915-1946`, `2107-2111`, `2176-2181`).

## Boundaries / non-goals

Not owned or not done by this crate: publication (no `publish` endpoint, no
`draft: false` path), tag creation or mutation, `--clobber`, auto-publish,
immutable overwrite, release selection, update, and rollback. It does not own
build, qualification or finalization (`eggpack-core`); it does not own installer
semantics, only calling `eggpack_bootstrap::render_posix_with_policy` /
`render_powershell_with_policy` (`802`, `808`); it does not render CI
workflows; and it does not touch registries, Issues, or PRs. The draft is left
for a human to publish.

## Dependencies / dependents

Dependencies (`crates/eggpack-github/Cargo.toml:14-26`): `eggpack-contract`
(layout authority — `DistributionContract`, `ExpandedAssets`),
`eggpack-manifest` (`ReleaseManifest`), `eggpack-core` (finalized-release
types), `eggpack-bootstrap` (installer rendering), plus `serde` / `serde_json`,
`sha2`, `tokio` (`rt,time,macros,net,io-util`), `eggfetch-core 0.2.0` with
`default-features = false` and features `standard-http1`, `tls-rustls`,
`tls-native-roots`, `json`, `bytes`, `futures-util`, and `url`. Dev-only:
`tokio` with `rt-multi-thread`.

The in-house HTTP stack is a deliberate choice over a heavyweight client: the
lean `eggfetch-core` profile gives explicit control over the security-relevant
knobs — no retry policy, no redirect following, bounded timeouts, bounded
decoded body size — instead of inheriting a vendor default. `tls-rustls` with
`tls-native-roots` means certificates are validated by rustls against the host
OS trust store rather than a bundled or pinned root set, so trust follows each
platform's CA configuration; combined with the fixed `api.github.com` /
`uploads.github.com` bases, the transport has no host negotiation to get wrong.

Dependents: `eggpack-cli` is a hard dependency
(`crates/eggpack-cli/Cargo.toml:24`) and the only production caller.
`_prepare-stage` uses `GitHubDraftTemplateV1::from_json`
(`crates/eggpack-cli/src/main.rs145`), `InstallerPresentationV1::from_json`
(`:1038`) and `prepare_staging_payload_with_presentation` /
`prepare_staging_payload` (`:1076`, `:1088`); `_stage-github-draft` uses
`StagingPayloadV1::from_json` (`:1118`), `read_token` (`:1124`),
`EggfetchTransport::new` (`:1126`) and `stage_with_dir` (`:1144`).
`eggpack-ci` depends on this crate **for dev only**
(`crates/eggpack-ci/Cargo.toml:25`, under `[dev-dependencies]`), to assert the
rendered workflow invokes exactly those two runner commands and never publishes
or clobbers (`crates/eggpack-ci/src/lib.rs:1038`, `:1059`, `:1242`, `:1278`,
`:3763-3775`).

## Related deep dives

- [Workspace module map and producer pipeline](overview.md)
- [Finalization and the finalized root](core-finalization.md)
- [Installer rendering](bootstrap.md)
- [Manifest parsing and canonical serialization](manifest.md)
- [CLI wiring](cli.md)
- [CI planning and workflow rendering](ci.md)
- [Validation model](validation-model.md)
- [Determinism rules](determinism.md)
- [Testing and portability lanes](testing-and-portability.md)
