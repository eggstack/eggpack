# `eggpack-bootstrap` — Deep Dive

`eggpack-bootstrap` renders release-specific, non-interactive **first-install**
scripts (POSIX `sh` and PowerShell) for one exact release, from the contract
layout plus that release's finalized manifest. It selects no release, updates
nothing, and models no installed state; install, update, and rollback belong to
Eggup in a different repository. Everything lives in one file,
`crates/eggpack-bootstrap/src/lib.rs` (3436 lines, `#![forbid(unsafe_code)]` at
`:1`, `#![deny(missing_docs)]` at `:2`).

## Responsibility and the hard boundary

The crate is a pure text generator. It reads exactly four things:
`&DistributionContract` (`:4`), `&ReleaseManifest` (`:5`),
`&BootstrapSpec { origin, fixture_http }` (`:12-17`), and — for the policy
renderers only — `&BootstrapInstallPolicyV1`. No `ReleasePlan`, no
qualification record, no build evidence, no network, no filesystem, no clock,
no environment probing at render time.

`render_posix` (`:140-144`) and `render_powershell` (`:193-197`) take the first
three; `render_posix_with_policy` (`:657-662`) and
`render_powershell_with_policy` (`:872-877`) take all four.

The boundary is structural, not conventional. The origin is a single fixed base
URL (`:14`) validated as `https://`, or loopback `http://` when `fixture_http`
is set (`:43-55`, duplicated as `validate_origin` at `:340-355`); it rejects
newlines, quotes, backticks, `$`, `\`, `?`, `#`, spaces, `@`, a trailing `/`,
and a bare `https://`. Release identity is pinned by
`c.expand(&t.target, &m.release_id)` (`:70`, `:426`), so there is no "latest"
and no index to consult. The rendered scripts contain no discovery, update, or
signature logic at all.

## Key types / functions (`crates/eggpack-bootstrap/src/lib.rs`)

| Item | Line | Description |
| --- | --- | --- |
| `BootstrapSpec { origin, fixture_http }` | `:12-17` | Fixed base URL (no trailing `/`) plus loopback-fixture switch. |
| `BootstrapError(String)` | `:20-29` | Opaque, bounded, path-free failure; `Display` + `Error`, built by `fail()`. |
| `render_posix` | `:140-188` | Direct-only POSIX renderer (spec pair). |
| `render_powershell` | `:193-238` | Direct-only PowerShell renderer (spec pair). |
| `InstallMode { Executable, Data }` | `:250-255` | Per-entry mode: POSIX `0755`/`0644`; Windows inherits ACL. |
| `BundleArchiveEncoding { TarGzip }` | `:260-263` | The only archive encoding a generated installer accepts. |
| `TargetInstallPolicy { modes, archive_encoding }` | `:268-275` | Per-target mode map plus optional required archive encoding. |
| `BootstrapInstallPolicyV1` | `:280-286` | Strict versioned policy; `deny_unknown_fields`. |
| `empty` / `from_toml` / `from_json` / `validate_shape` | `:294-337` | Constructors, bounded parsers, shape validation. |
| `render_posix_with_policy` | `:657-860` | Direct + bundle + archive POSIX renderer (policy pair). |
| `render_powershell_with_policy` | `:872-1048` | Direct + bundle + archive PowerShell renderer (policy pair). |

Internal machinery: `Item` (`:30-37`), `project` (`:38-112`, direct-only),
`safe_segment` (`:113-117`), `runtime_pair` (`:118-138`), `shq` (`:189-191`),
`psq` (`:239-241`), `BundleEntryProjection` (`:358-364`),
`ArchiveMemberProjection` (`:367-373`), `TargetProjection` (`:376-398`),
`project_all` (`:400-622`), `posix_mode` (`:624-629`), `POSIX_SHA_SNIPPET`
(`:631`), `posix_verify_block` (`:633-645`), `posix_download_block`
(`:647-654`), `ps_verify_block` (`:862-869`).

## The two-layer render API

The two pairs differ in capability. The **spec-driven pair** projects only
`ExpandedAssets::Direct` against `ArtifactForm::Direct`; any bundle or archive
target is rejected with `M001 requires matching direct artifact targets`
(`:83`). It emits one artifact per platform, one install name, no mode
selection, no extraction. It is retained as the direct-release regression
surface.

The **policy-driven pair** supersedes it and handles all three asset forms. The
policy layer adds multi-entry placement with rollback, per-entry POSIX mode from
`InstallMode`, archive extraction with per-member verification, and a strict
cross-check of policy against contract. Policy is mandatory for bundle and
archive targets (`:466`, `:527`) and optional for direct targets. Callers outside
this crate use only the policy pair (`crates/eggpack-github/src/lib.rs:802`,
`:808`, `:1471`, `:1477`); the spec pair has no caller outside its own tests.

## Install modes and the shell/PowerShell matrix

`InstallMode` is keyed by install name, not by target. `posix_mode` (`:624-629`)
maps `Executable` to `755` and `Data` to `644`. The PowerShell renderers never
consult the mode: they move verified bytes with `[IO.File]::Move` (`:942`,
`:1015`), preserving content and inheriting the destination directory ACL, as
the `InstallMode` doc comments state (`:251`, `:253`). Policy is still required
and validated for Windows targets, so a policy valid for POSIX is valid for
PowerShell. `BundleArchiveEncoding` has one variant, and the archive filename
must end in `.tar.gz` (`:533-535`).

| Form | POSIX (`render_posix_with_policy`) | PowerShell (`render_powershell_with_policy`) |
| --- | --- | --- |
| Direct | Download to `"$tmp/payload"`, size + SHA-256, `chmod 755`, `ln` (`:676-684`). | `Invoke-WebRequest -OutFile`, `Get-FileHash`, `[IO.File]::Move` (`:890-900`). No chmod. |
| Bundle | All destinations pre-checked absent; per-entry download + size + SHA-256; re-check; per-entry `chmod 755`/`644`; sequential `ln` with `rm -f` rollback of prior links (`:687-744`). | Same pre-check / verify / re-check order; sequential `[IO.File]::Move` appending to `$created` (`:903-947`); rollback via the shared `catch` (`:1040-1042`). |
| Archive | Requires `tar` (`:768`); outer verify (`:770-776`); expected-inventory `printf` (`:778-783`); `tar -tzf` (`:784`); member-path guard (`:785`); `LC_ALL=C sort` + `cmp -s` exact inventory (`:786-788`); extract to `"$tmp/extracted"` (`:789-790`); per-member symlink + regular-file check (`:793`), size + SHA-256 (`:796-803`), `chmod` (`:804-807`); re-check; `ln` with rollback (`:815-834`). | Requires `tar.exe` (`:971`); outer verify (`:976-981`); `$expected` vs `tar.exe -tzf` with `$missing`/`$extra` (`:982-993`); extract to `$extractDir` (`:994`); per-member `LinkType` and `-PathType Leaf` (`:997`) plus size + SHA-256 (`:1000-1005`); `[IO.File]::Move` with `$created` (`:1013-1019`). |

Archive members declare a `source` inside the archive and an `install` name at
the destination. Nested sources flatten: the test fixture uses
`bin/egress-helper` installing as `egress-helper` (`:1914-1921`), and `bin/` is
never created in the destination (`:818`, `:1015`; asserted at `:3178`). A
missing member fails the inventory comparison (`archive member inventory
mismatch`, `exit 6` at `:788`; `throw` at `:992-993`); a member that is present
but not a regular file is rejected after extraction (`symlink member rejected` /
`member is not a regular file`, `:793`, `:997`).

## `BootstrapInstallPolicyV1` and per-target policy

The policy is a caller-owned, strictly parsed document. `from_toml` (`:302`) and
`from_json` (`:312`) bound input at `MAX_POLICY_JSON = 256 KiB` (`:288`) before
parsing, then run `validate_shape` (`:322-337`): `schema_version == 1`, at most
`MAX_POLICY_TARGETS = 256` targets, at most `MAX_POLICY_ENTRIES = 256` mode
entries per target, non-empty target keys of at most 128 characters, and every
mode key passing `safe_segment` (`[A-Za-z0-9._+-]+`, `:113-117`).
`deny_unknown_fields` (`:267`, `:279`) rejects extras.

What a caller may override per target is deliberately narrow: the mode of each
install name and, for archive targets only, the required encoding. It cannot
override the install location (fixed to the single positional argument, `:848` /
`:1031`), the download URL (fixed by contract + manifest + origin), the artifact
names, or the digests. Policy cannot contradict the contract; every
contradiction is a hard error before any text is emitted:

| Attempt | Rejection |
| --- | --- |
| Mode for an install name the contract does not declare, or a missing one | `:497-499`, `:511-515`, `:572-574`, `:586-590` |
| `archive_encoding` on a direct or bundle target | `:445-447`, `:467-469` |
| Direct target policy that is not exactly one `Executable` mode named by `install` | `:448-454` |
| Archive target without explicit `TarGzip` | `:528-532` |
| Archive filename not ending in `.tar.gz` | `:533-535` |
| Policy naming a target absent from the manifest | `:608-612` |
| Manifest archive member whose `install` differs from the contract's `install_name` | `:564-566` |

Independently of policy, `project_all` enforces `m.validate()` (`:408`),
contract/manifest product identity (`:412-414`), canonical unique targets
(`:422-424`), unambiguous runtime OS/arch mapping (`:429-433`), exact target-set
equality (`:605-607`), and exact contract/manifest entry or member
correspondence for bundles and archives (`:470-472`, `:544-546`).
`DistributionContract` is `Deserialize` and this crate calls `c.resolve` and
`c.expand` (`:419-427`) but never a contract validation entry point, so it
relies on the caller having obtained a validated contract — in practice via
`parse_toml_str`, which validates.

## What a generated installer does, step by step

POSIX policy script (emitted at `:841-856`):

1. `set -eu` (`:842`).
2. Platform detection from `uname -s` / `uname -m`, normalized to
   `linux|macos|windows` × `x86_64|aarch64|armv7` (`:843-846`).
3. `origin` assignment, `dest=${1:-.}`, `mkdir -p "$dest"` (`:847-849`).
4. `tmp=$(mktemp -d "$dest/.eggpack.XXXXXX")` with `EXIT` and
   `HUP/INT/TERM` traps (`:850-852`) — the temp dir is a sibling of the
   destination, so final placement is a same-filesystem operation.
5. `case "$os:$arch"` dispatch (`:853-855`); unmatched platforms exit 2.
6. Per branch: destination pre-checks; download via
   `curl --fail --silent --show-error --connect-timeout 10 --max-time 120` with
   no `-L` (`:649`); size check (exit 6) and SHA-256 check (exit 8)
   (`:633-645`); destination re-check; `chmod`; atomic placement via `ln`
   (`:677`, `:725-742`, `:815-834`). Archive branches additionally run the
   listing, path, and inventory gates below before extraction.
7. `exit 0` per branch (`:743`, `:835`), so the `case` is terminal.

PowerShell policy script (emitted at `:1026-1043`): `$ErrorActionPreference =
'Stop'`; OS/arch from `$env:OS`, `$IsMacOS`, and
`[Runtime.InteropServices.RuntimeInformation]::OSArchitecture` (`:1027-1029`);
`$origin`, `$dest = $args[0]`, `New-Item -Force` (`:1030-1032`); GUID temp dir
under `$dest` (`:1033-1034`); `$created = @()` (`:1035`); `try { switch ... }`
(`:1036-1039`); `catch` removes only `$created` paths and rethrows
(`:1040-1042`); `finally` removes the temp dir (`:1043`).

There is no PATH wiring in either renderer: no profile editing, no symlink into
a `bin` directory, no shell rc modification. The installer places files and
exits.

The scripts explicitly do not: fetch or parse a release index; check for a newer
version; install over an existing file; remove a pre-existing file; escalate
privilege (`sudo`, `RunAs`, `Start-Process` absent, asserted at `:3122-3123`);
follow redirects (`-MaximumRedirection 0` at `:228`, `:891`, `:919`, `:973`);
or verify a signature.

## Integrity, not authenticity

`sha2` is a **dev-dependency** (`crates/eggpack-bootstrap/Cargo.toml:21-24`)
used only by the test helper `sha_hex` (`:1403-1409`). The renderer never
computes a digest: it copies the manifest's `sha256` string into the script
(`:80`, `:461`, `:504`, `:579`, `:596`) and emits a comparison (`:639-643`,
`:864`). Likewise `tar` and `flate2` are dev-dependencies used to *build*
deterministic fixture archives in tests (`:1757-1780`, `:2758-2780`), not to
unpack anything.

What the emitted check proves: the bytes the script received are the bytes the
finalized manifest recorded — integrity against corruption, truncation, and a
mismatched or substituted artifact. The size check (`:634-638`) fires before the
hash check, and both fire before placement.

What it does not prove: nothing about who published the artifact. The digest
travels to the consumer in the same trusted channel as the artifact, so a party
who can replace the artifact can replace the digest. There is no signature, no
certificate, no transparency log, and no key distribution in this crate or in
the generated script, and the emitted text makes no authenticity claim. This
limitation is deliberate and load-bearing: `ReleaseManifest` is final-bytes
evidence, not a signed attestation, and consumers needing authenticity must
verify it outside this seam. See `eggup-manifest-consumer-v1.md`.

## Determinism of rendered scripts

The same contract, manifest, spec, and policy render byte-identical output.
Nothing in the rendered text is sampled from the host: no timestamp, no
hostname, no user name, no random identifier, no locale-dependent formatting.

- `project_all` sorts targets by `(os, arch)` before emission (`:613-620`), so
  emission order does not follow manifest order.
- `BTreeSet` guards target and platform uniqueness (`:415-416`); `BTreeMap`
  pairs contract and manifest entries (`:474`, `:547`); policy mode maps are
  `BTreeMap` (`:271`), so lookup and serialization are ordered.
- Randomness exists only at run time: `mktemp -d` (`:850`) and
  `[Guid]::NewGuid()` (`:1033`) are fixed source text in the script and resolve
  when the script runs.
- The legacy `project` (`:38-112`) deliberately preserves manifest order. That
  is input-derived and stable, not host-derived.
- Asserted directly: `render_posix` equality at `:1122-1123`,
  `render_powershell_with_policy` equality at `:3116-3120`.

Cross-reference `determinism.md`.

## Safety properties

**Fail-closed exits.** Size mismatch exits 6 (`:635`, `:178`); SHA-256 mismatch
exits 8 (`:641`, `:180`); missing tooling exits 5 (`curl is required` `:649` /
`:176`, `tar is required` `:768`, `tar.exe is required` `:971`); missing
SHA-256 tooling exits 7 (`:631`); every branch ends in `exit 0` (`:743`, `:835`)
so execution cannot fall through. PowerShell throws instead, under
`$ErrorActionPreference = 'Stop'` (`:1026`).

**Unwritable targets.** Destinations are checked absent before any download and
re-checked immediately before placement (`:696-699` + `:712-717`; `:965-970` +
`:1007-1012`). Final placement is the authoritative guard: POSIX `ln` (`:677`,
`:728`, `:737`, `:818`, `:828`) and PowerShell `[IO.File]::Move` (`:232`,
`:942`, `:1015`) both refuse to overwrite, reporting `destination already exists
or filesystem does not support safe placement`. A failure after the first
successful placement rolls back only files this invocation created — `rm -f` of
prior links (`:737`, `:828`) or the `$created` list (`:1040-1042`) — and never
deletes anything pre-existing. Because the temp dir lives under the destination
(`:850`, `:1033`), placement is a same-filesystem link or same-volume move by
construction; a cross-device layout surfaces as a placement failure, not a
silent copy.

**Quoting and escaping.** Every interpolated value passes through `shq`
(`:189-191`, single quotes with `'\''` escaping) or `psq` (`:239-241`, single
quotes with `''` doubling). Values reaching unquoted positions are additionally
constrained by `safe_segment` (`:113-117`) to `[A-Za-z0-9._+-]`, and the origin
is rejected if it contains any character that could escape quoting (`:47-48`,
`:345-346`). Install names, artifact names, and archive sources all go through
the quoting helpers (`:677`, `:690`, `:757`, `:794`, `:818`, `:891`, `:906`,
`:960`, `:998`, `:1017`). One coupling worth recording: `ps_verify_block`
formats the expected digest with `shq` and lowercases it (`:867`), which is safe
only because `m.validate()` enforces 64 lowercase hex characters
(`crates/eggpack-manifest/src/lib.rs:268-275`); a digest containing a quote
would be mis-escaped for PowerShell. Real dependency, currently unreachable.

**No `eval`-style constructs.** No `eval`, `Invoke-Expression`, `iex`,
`bash -c`, `sh -c`, or `Start-Process` appears in either renderer, and absence
is asserted (`:1125`, `:3122-3123`). Branching uses `case` / `switch` over
literal arms.

**Archive path traversal.** The guard is emitted into the script, not delegated
to `tar`, and it runs *before* extraction. The POSIX listing gate (`:785`)
rejects, per listed member: the empty string, a leading `/`, any backslash, any
colon, and any occurrence of `..`. The PowerShell gate (`:991`) is the same five
checks. Extraction then happens into a fresh private temp directory (`:789-790`,
`:994`), and only afterwards is each expected member resolved, checked for
symlink and regular-file type (`:793`, `:997`), verified by size and SHA-256
(`:796-803`, `:1000-1005`), and linked or moved to its install name. No
archive-derived path is ever used as a write target inside the destination, so
even a guard bypass could not place a file outside `$dest`. Independently, the
exact-inventory comparison (`:786-788` via `sort` + `cmp -s`; `:992-993` via
`$missing`/`$extra`) rejects any member not named by the contract and manifest,
which subsumes traversal as a special case. The `..` test is substring-based
(`*'..'*` / `-match '\.\.'`) and so also rejects legitimate names containing
`..` — conservative in the safe direction. On the Rust side, `install` is
`safe_segment`-checked (`:567`) but `source` is not re-validated here;
traversal-shaped `source` values are rejected upstream by
`validate_member_source` in the contract
(`crates/eggpack-contract/src/lib.rs:707-751`, which rejects control
characters, braces, backslashes, leading `/`, `:`, and any `.` or `..`
component). Tests build raw ustar fixtures the `tar` crate would refuse to
construct, specifically to exercise the consumer guard (`:1780-1850`;
traversal `:2594-2609`, absolute `:2611-2627`, backslash `:2629-2646`).

## Windows and PowerShell specifics

The PowerShell renderers need a host that provides a PowerShell exposing
`$IsMacOS` and `[Runtime.InteropServices.RuntimeInformation]` (`:1027-1029`) —
hence the pwsh 7 requirement — and, for archive targets, `tar.exe` (`:971`).
POSIX archive targets need `tar` (`:768`).

`cargo test -p eggpack-bootstrap` refuses to skip on the Windows lane: under
`cfg!(windows)` the test fails unless `pwsh_available()` and
`tar_exe_available()` both hold (`:3089-3094`, helpers `:1862-1905`); other
platforms print a skip note and return (`:3095-3098`). The Windows CI lane
independently asserts `pwsh -Version` and `tar.exe --version`
(`.github/workflows/ci.yml:70-75`) and then runs
`m002a_powershell_archive_runtime` as a focused step
(`.github/workflows/ci.yml:76-77`).

`m002a_powershell_archive_runtime` (`:3084-3436`) is real runtime evidence: it
serves fixture bytes over loopback HTTP, renders the archive installer,
executes it with `pwsh -NoProfile -NonInteractive -File`, and asserts exact
installed bytes (`:3176-3177`), nested-source flattening (`:3178`), no-overwrite
on re-run (`:3180-3183`), outer size and digest rejection (`:3186-3202`), the six
inner guards (`:3311-3349`), member-evidence rejection (`:3352-3371`), the
`tar.exe` tool boundary (`:3373-3399`), rollback of an invocation-created file
while preserving a pre-existing sentinel (`:3401-3421`), and preservation of a
pre-existing first destination (`:3423-3432`).

Cross-reference `testing-and-portability.md`.

## Tests and coverage shape

This crate has 8 `#[test]` functions (`:1077`, `:1160`, `:1185`, `:1991`,
`:2201`, `:2440`, `:2964`, `:3084`) — the lowest test count in the workspace
against 3436 source lines, and the test module spans `:1050-3436`, so roughly
70% of the file is tests. The dev-dependencies make fixtures real: `sha2`
computes fixture digests (`:1403-1409`); `tar` plus `flate2` build deterministic
`.tar.gz` archives with `mtime(0)`, `uid/gid 0`, and
`tar::HeaderMode::Deterministic` (`:1757-1780`). A hand-written ustar writer
(`:1780-1850`) produces archives with names the `tar` crate rejects, so the
traversal and type guards run against bytes an attacker could actually serve.

Honest assessment. The strong part: the direct-only renderers execute for real
against a loopback fixture server (`:1185`), the POSIX bundle and archive
renderers run full positive and negative matrices end to end (`:2201`, `:2440`),
and the PowerShell archive lane runs actual `pwsh`. The weak part: most of this
crate is emitted text and several assertions are substring checks
(`assert!(a.contains("--connect-timeout 10 --max-time 120"))` at `:1124`;
`assert!(script.contains("tar.exe"))` at `:3121`). A substring assertion can
pass while unrelated parts of the same script are wrong. POSIX policy coverage
is behavioural, strong but only for the platform and tool combinations that ran;
the spec pair is covered by substring, parse, and one runtime direct test.
PowerShell non-archive forms have static coverage only (`:2964`), making
`m002_powershell_static_and_runtime` the weakest M002 test by construction.
Nothing asserts the full text of a rendered script, and nothing exercises
`render_posix` / `render_powershell` output on Windows.

## Boundaries / non-goals

This crate never: selects a release or resolves `latest`; checks for or applies
an update; rolls back a prior release; writes or reads an Eggup receipt; models
installed state; fetches a release index; follows a redirect; verifies a
signature or provenance claim; elevates privilege; overwrites, merges, or
replaces an existing file; edits `PATH` or any shell profile; falls back to a
system package manager; calls the GitHub API; publishes anything.

Normal update and service lifecycle stay with Eggup and the application.

## Dependencies / dependents

- Runtime (`crates/eggpack-bootstrap/Cargo.toml:14-19`): `eggpack-contract`,
  `eggpack-manifest`, `serde`, `serde_json`, `toml`. No `eggpack-core`, no HTTP
  client, no async runtime, no build backend, no process execution.
- Dev (`crates/eggpack-bootstrap/Cargo.toml:21-24`): `sha2`, `tar`, `flate2`,
  used only by tests.
- Dependents: `eggpack-github` (regular; `crates/eggpack-github/Cargo.toml:18`,
  renders the installers it stages), `eggpack-cli` (regular;
  `crates/eggpack-cli/Cargo.toml:23`, parses a policy file at
  `crates/eggpack-cli/src/main.rs:1017-1024`), `eggpack-ci` (dev;
  `crates/eggpack-ci/Cargo.toml:24`, renderer tests only).

## Related deep dives

- [`overview.md`](overview.md) — module map and producer pipeline.
- [`contract.md`](contract.md) — layout authority, `ExpandedAssets`,
  `validate_member_source`.
- [`manifest.md`](manifest.md) — `ReleaseManifest`, `ArtifactForm`, and the
  integrity facts the emitted digests come from.
- [`github.md`](github.md) — staging of the rendered installers.
- [`cli.md`](cli.md) — the CLI surface that reads a policy file and triggers
  rendering.
- [`determinism.md`](determinism.md) — workspace determinism rules implemented
  here.
- [`validation-model.md`](validation-model.md) — fail-closed conventions used by
  `project` and `project_all`.
- [`testing-and-portability.md`](testing-and-portability.md) — the Windows
  qualification lane and the M002a runtime evidence.
- [`eggup-manifest-consumer-v1.md`](eggup-manifest-consumer-v1.md) — the
  consumer-side boundary and why installed state lives there.
