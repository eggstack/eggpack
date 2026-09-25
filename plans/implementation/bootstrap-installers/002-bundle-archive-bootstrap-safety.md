# Bootstrap Installers Milestone 002 — Bundle/Archive Bootstrap Safety

Status: closed

Closure record: `plans/closure/bootstrap-installers/002-status.md`

Repository baseline: ca0537b5ec6f7996ee9f0a1eaa04256752515eaa

Source roadmap: plans/subsystems/bootstrap-installers-roadmap.md

Hard dependency closures:

- Bootstrap M001: plans/closure/bootstrap-installers/001-status.md
- Build/Qualification M004: plans/closure/build-qualification/004-status.md
- Release Manifest M002: plans/closure/release-manifest/002-status.md

Applicable ADRs:

- plans/adrs/ADR-0001-producer-consumer-release-boundary.md
- plans/adrs/ADR-0002-contract-plan-manifest-separation.md

Primary class: capability / consumer-facing generated artifact / filesystem safety

## 1. Objective

Extend the closed direct first-install generator so deterministic POSIX shell and PowerShell bootstrap installers can safely install exact bundle and archive releases described by DistributionContract + ReleaseManifest.

M002 must preserve the first-install-only boundary:

- exact release selected by caller;
- exact origin selected by caller;
- no latest/version discovery;
- no overwrite/update/rollback;
- no service-manager ownership;
- no privilege escalation;
- no Eggup receipt semantics;
- no public release lookup.

Bundle/archive installation must be all-or-nothing from the bootstrap installer's perspective.

## 2. Why this is ready

Bootstrap M001 already provides:

- strict contract/manifest agreement;
- exact target mapping;
- fixed HTTPS origin policy;
- loopback-only HTTP fixtures;
- deterministic POSIX/PowerShell rendering;
- size + SHA-256 verification;
- safe private temporary directory;
- no overwrite;
- no redirects;
- direct first-install placement.

Build M004 now provides the missing producer evidence:

- direct/bundle/archive finalization;
- exact final size/hash;
- deterministic tar-gzip archive construction;
- exact archive member paths/content;
- Manifest v1 member hashes tied to finalized archive members;
- mixed-release/source rejection.

The remaining bootstrap work is consumer-side transaction/extraction policy, not producer archive construction.

## 3. Ownership boundary

~~~text
DistributionContract + ReleaseManifest
                 |
                 v
        Bootstrap projection
                 |
                 +--> caller install-mode policy
                 +--> exact origin
                 |
                 v
 generated first-install script
                 |
                 +--> verify exact artifact bytes
                 +--> verify exact archive members if needed
                 +--> transactional placement
                 |
                 X no normal update / receipt / service / publication authority
~~~

Eggpack may generate bootstrap extraction logic because the generated script is itself a first-install consumer artifact.

This does not move ordinary update/extraction ownership out of Eggup/application code.

## 4. Invariants

- generated installer derives release names/relationships from contract + manifest only;
- caller policy may supply install behavior such as executable/data mode, but may not rename artifacts/install names;
- target set must match contract exactly;
- unsupported runtime platform fails before network transfer;
- fixed HTTPS origin only, except explicit loopback fixture mode;
- no redirects;
- every downloaded release artifact is checked for exact size and SHA-256 before use;
- archive extraction occurs only after archive verification;
- archive member set must exactly match contract/manifest expectations before installation;
- extracted members must be regular files, never symlinks/directories/devices;
- extracted member size + SHA-256 must match Manifest member evidence;
- destination names are contract install names only;
- all destination absence checks happen before placement;
- race-time destination conflicts fail closed;
- partial placement is rolled back only for files created by this invocation;
- pre-existing files are never removed or replaced;
- executable permission is explicit policy for bundle/archive entries, never guessed from filename;
- no shell command or arbitrary post-install hook in policy;
- no sudo/elevation;
- generated scripts remain reviewable.

## 5. Scope

### In scope

- bundle projection/rendering for POSIX and PowerShell;
- archive projection/rendering for POSIX and PowerShell;
- strict BootstrapInstallPolicyV1 for install modes/archive encoding;
- exact multi-artifact download/verification;
- exact tar-gzip archive verification/extraction;
- exact archive member inventory validation;
- member size/hash verification;
- transactional multi-file placement with rollback;
- executable/data mode handling;
- deterministic script generation;
- local HTTP multi-request fixture tests;
- shell/PowerShell parse and runtime tests;
- docs/MSRV/package/hosted CI.

### Out of scope

- direct M001 semantic redesign;
- zip or other archive encodings;
- package managers;
- service installation;
- ownership/chown policy;
- ACL synthesis;
- normal updates;
- replacing existing installs;
- Eggup InstallReceipt creation;
- signature/authenticity verification;
- arbitrary extraction commands;
- product hook scripts;
- remote release/version discovery.

## 6. Required production model

### A. BootstrapInstallPolicyV1

Add a strict versioned caller-owned bootstrap install policy.

Purpose: supply consumer install behavior not represented by DistributionContract, without becoming naming/release authority.

Suggested shape:

- schema_version = 1;
- per canonical target policy;
- exact install-name -> InstallMode map;
- archive encoding when target form is Archive.

InstallMode is a finite enum:

- Executable;
- Data.

For direct targets, preserve M001 behavior: the single direct install remains executable unless/until a later explicit migration decides otherwise.

For bundle/archive targets:

- policy keys must exactly equal the expanded contract install-name set for that target;
- no missing/extra install names;
- policy cannot change release asset name, install name, member source, target, URL, size, hash, or ordering;
- unknown fields reject;
- bounded target/item counts;
- duplicate/case-colliding install names reject through existing contract invariants.

Archive encoding is initially only TarGzip and must agree with the M004-qualified archive convention. Do not infer additional encodings from suffix.

### B. Projection model

Replace the M001 direct-only internal Item with a bounded target projection capable of:

- Direct:
  - one artifact;
  - one install name;
  - exact size/hash;
  - Executable mode.

- Bundle:
  - ordered exact artifact records;
  - each exact install name;
  - exact size/hash;
  - caller mode.

- Archive:
  - one exact archive record;
  - exact archive size/hash;
  - explicit TarGzip encoding;
  - exact ordered member source/install names;
  - each member exact size/hash from manifest;
  - caller mode.

Projection must validate contract and manifest pairwise, including all relationships, before emitting script text.

Do not accept a manifest member merely because its basename matches.

### C. Bundle download/verification

Generated installer:

1. determines runtime target;
2. creates a private temp directory beneath destination;
3. downloads each bundle asset to a deterministic private temp filename;
4. checks exact size;
5. checks exact SHA-256;
6. does not place anything until every bundle member verifies.

Use one fixed origin plus manifest/contract release filenames.

No sidecar download is required for bootstrap correctness because ReleaseManifest already carries the expected digest; checksum sidecars remain release artifacts/evidence, not a second runtime authority.

### D. Bundle transactional placement

Before placement:

- every final destination must be absent;
- all parent/destination path operations remain beneath caller destination root;
- no install name may contain separators/traversal by contract validation.

POSIX:

- apply mode in temp first;
- prefer same-filesystem hard-link placement as M001;
- if any link fails, remove only final paths successfully created by this invocation;
- do not add copy fallback silently if it weakens race semantics.

PowerShell:

- create temp beneath destination so File.Move stays on the same volume;
- move sequentially only after all verification;
- maintain an invocation-owned created-path list;
- on any placement failure, remove only those created paths;
- never remove a pre-existing path.

After success, private temp cleanup must not remove final installed files.

### E. Archive verification

For Archive targets:

1. download archive;
2. verify exact archive size/hash;
3. list archive member names before extraction;
4. compare exact normalized member inventory to contract/manifest expected sources;
5. reject missing members;
6. reject extras;
7. reject absolute/traversal/empty names;
8. only then extract into private temp.

The generated installer must not extract directly into destination.

### F. Archive tool boundary

Initial supported archive is tar+gzip only.

POSIX requires a tar implementation capable of listing/extracting gzip tar archives.

PowerShell/Windows requires tar.exe or another fixed finite built-in/tool path explicitly supported by the renderer. Do not download an extractor.

Generated scripts must fail with a clear bounded message when the required extractor is unavailable.

Do not invoke arbitrary caller-supplied extraction command.

### G. Extracted member validation

After extraction, for every declared member source:

- use lstat-equivalent checks;
- reject symlink;
- require regular file;
- reject directory/device/pipe;
- verify exact byte size against manifest member record;
- verify exact SHA-256 against manifest member record;
- map source to contract install name only after verification;
- apply caller InstallMode.

Nested archive sources are allowed only as already validated by DistributionContract. Final installed names remain flat contract install names.

### H. Archive extraction containment

Extraction root is a private temp subdirectory.

Even though the verified M004 archive is expected to be safe, generated scripts must still validate the exact listing and resulting file types before placement.

No archive member may escape the extraction root.

Reject archive entries with platform-specific alternate separators or names that do not exactly match the contract source strings.

### I. Install modes

POSIX:

- Executable => mode 0755;
- Data => mode 0644.

PowerShell/Windows:

- do not synthesize POSIX modes;
- preserve file bytes and normal inherited ACL;
- policy is still required and validated so the same release policy is explicit across renderers.

Do not chown/chgrp.

### J. Existing installation semantics

M002 remains first-install only.

If any target destination already exists:

- fail before placement where observable;
- still defend against races during placement;
- never replace;
- never merge an archive into an existing installation;
- never use existing files as evidence.

### K. Rollback journal

Generated scripts must maintain enough invocation-local state to roll back partial placement.

The journal need not be persisted after the process exits.

Rollback rules:

- only remove paths successfully created by this invocation;
- never remove a path that existed before the placement attempt;
- cleanup failure is reported but must not trigger broader deletion;
- private temp directory cleanup remains scoped.

### L. Origin/download policy

Preserve M001 network behavior:

- HTTPS production origin;
- loopback HTTP only for explicit fixtures;
- exact artifact URL = origin + contract/manifest filename;
- no redirects;
- connect timeout and total timeout;
- no credentials embedded in generated text;
- no latest/version API.

For multiple bundle downloads, apply the same bounds independently.

### M. Determinism

Same contract + manifest + BootstrapSpec + install policy must produce byte-identical script output.

No timestamps/random IDs in generated text. Runtime temp names may be generated by the script.

## 7. Ordered work packages

1. Add BootstrapInstallPolicyV1 and validation.
2. Refactor projection into Direct/Bundle/Archive internal forms.
3. Preserve direct golden output/behavior unless required for shared helpers.
4. Add bundle POSIX rendering.
5. Add bundle PowerShell rendering.
6. Add archive tar-gzip POSIX rendering.
7. Add archive tar-gzip PowerShell rendering.
8. Add exact archive member inventory and file-type validation.
9. Add member size/hash validation.
10. Add transactional multi-file placement/rollback.
11. Add multi-request local HTTP fixture server.
12. Add bundle/archive negative runtime matrix.
13. Add shell/PowerShell parser/static-tool checks.
14. Update docs/roadmap/registry and write closure.

## 8. Failure/restart/contention semantics

Any verification failure before placement leaves destination unchanged.

Any placement failure:

- rolls back only invocation-created files;
- leaves pre-existing files untouched;
- produces no success result.

A rerun after a successful install fails because destination files exist.

Concurrent installers targeting the same destination race on final creation; at most one may succeed for a given install name, and a loser rolls back only its own created paths.

No lockfile is required in M002 if atomic/no-overwrite placement plus rollback preserves safety. If implementation needs a durable lock, stop and specify ownership/staleness semantics rather than improvising one.

## 9. Compatibility

Direct M001 remains supported.

DistributionContract v1 and ReleaseManifest v1 do not change.

BootstrapInstallPolicyV1 is producer/generator input, not a portable release identity schema.

eggpack-bootstrap should remain independent of eggpack-core if contract+manifest evidence is sufficient; do not add a core dependency merely to inspect M004 local paths.

No Eggup dependency.

## 10. Security review

Explicitly audit:

- exact contract/manifest pairwise relationships;
- URL/origin escaping;
- shell/PowerShell quoting;
- path traversal;
- tar listing/extraction semantics;
- symlink/hardlink archive entries;
- extra archive members;
- destination race behavior;
- rollback scope;
- pre-existing path preservation;
- executable/data mode correctness;
- checksum tool fallbacks;
- temp directory permissions;
- redirect behavior;
- no embedded secrets/elevation/update behavior.

Archive integrity is not authenticity. Document that distinction.

## 11. Required tests

### Projection/policy

- direct M001 projection unchanged;
- bundle exact asset/install relationship;
- archive exact archive/member/install relationship;
- unknown target rejects;
- target-set mismatch rejects;
- missing/extra install policy rejects;
- invalid policy version/unknown field rejects;
- archive target without TarGzip policy rejects;
- archive policy on non-archive target rejects;
- mode map cannot rename an install.

### Bundle runtime

- all members download/verify/install;
- one missing asset => no installed files;
- one wrong size => no installed files;
- one wrong SHA => no installed files;
- existing first destination => no placement;
- existing later destination => no placement;
- race/failure after first placement rolls back created file;
- executable mode 0755 on POSIX;
- data mode 0644 on POSIX;
- PowerShell installs exact bytes;
- no overwrite.

### Archive runtime

- exact M004-style tar.gz installs declared members;
- archive size mismatch rejects;
- archive SHA mismatch rejects;
- missing member rejects;
- extra member rejects;
- traversal member rejects;
- absolute member rejects;
- symlink member rejects;
- directory in place of file rejects;
- member size mismatch rejects;
- member SHA mismatch rejects;
- nested source flattens only to contract install name;
- unavailable tar rejects before placement;
- partial placement rolls back;
- pre-existing destination preserved.

### Determinism/static checks

- identical inputs => identical scripts;
- shell sh -n succeeds;
- ShellCheck where available;
- PowerShell parser succeeds;
- PSScriptAnalyzer where available;
- no sudo/Start-Process RunAs/elevation;
- no update/latest/version API;
- no redirect-following option;
- no arbitrary extraction command.

## 12. Verification

~~~bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-bootstrap --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-bootstrap --locked
cargo package -p eggpack-bootstrap --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked
./scripts/check-local.sh
git diff --check
~~~

Also run, when installed:

~~~text
shellcheck <generated POSIX fixtures>
PSScriptAnalyzer against generated PowerShell fixtures
~~~

Hosted Linux stable, Linux Rust 1.89, macOS, and Windows must pass.

Runtime fixture coverage must execute POSIX bundle/archive paths on Unix and PowerShell bundle/archive paths on a Windows-capable lane when PowerShell/tar are available. Required platform behavior may not be silently skipped and still counted as closure evidence.

## 13. Documentation updates

Update:

- root README;
- crates/eggpack-bootstrap/README.md;
- Bootstrap Installers roadmap;
- plans/registry.md;
- closure plans/closure/bootstrap-installers/002-status.md.

Document:

- first-install only;
- bundle/archive all-or-nothing semantics;
- caller-owned install modes;
- tar-gzip-only archive support;
- no overwrite/update;
- integrity vs authenticity;
- required local tar/tooling;
- normal update remains Eggup/application-owned.

## 14. Acceptance criteria

M002 closes only when:

- bundle POSIX and PowerShell generation is deterministic and runtime-qualified;
- archive POSIX and PowerShell generation is deterministic and runtime-qualified for TarGzip;
- exact archive member inventory is checked before placement;
- every extracted member is regular/non-symlink and size/hash verified;
- install modes are explicit caller policy rather than guessed;
- multi-file placement is all-or-nothing with bounded rollback;
- existing installs are preserved;
- no direct M001 regression;
- no contract/manifest schema change;
- stable/MSRV/macOS/Windows CI passes;
- no unresolved medium-or-higher bootstrap extraction/placement finding remains.

Closure must state whether Bootstrap M003 two-consumer adoption is ready to plan. Do not author M003 before this closure and real adoption candidate review.

## 15. Stop conditions

Stop and re-plan if:

- archive extraction requires a general package/extractor framework;
- zip/another encoding becomes required;
- safe transactional placement needs persistent lock semantics not specified here;
- contract/manifest must gain install permission fields;
- consumer update/rollback behavior becomes necessary;
- Eggup receipt semantics are required;
- platform ACL/ownership mutation becomes necessary;
- archive member verification cannot be implemented safely in generated shell/PowerShell.

## 16. Closure evidence

Record implementation SHA, BootstrapInstallPolicyV1 examples, bundle/archive projection matrix, generated fixture hashes, runtime local-server bundle/archive evidence, archive listing/type/hash negative matrix, rollback/pre-existing preservation evidence, shell/PowerShell static parse/lint, package/dependency/MSRV/docs results, hosted matrix, unresolved findings, and explicit Bootstrap M003 readiness disposition.
