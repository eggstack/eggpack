# `eggpack-core` — Planning stage (`src/lib.rs`) — Deep Dive

Owns producer **policy intake and pure planning**: parsing strict `PackConfig` v1
policy, resolving it against the `eggpack-contract` layout authority into a
canonically sorted `ReleasePlan` (intent, never evidence), and assembling a
schema-v1 `ReleaseManifest` from explicitly supplied finalized files. It is one
of four `eggpack-core` stage deep dives, alongside `core-build.md`,
`core-qualification.md`, and `core-finalization.md`.

Crate root: `crates/eggpack-core/src/lib.rs`, 1379 lines, `#![forbid(unsafe_code)]`
(`:1`), `#![deny(missing_docs)]` (`:2`). The file is the crate root and declares
the sibling stages, re-exported wholesale at `:10-14`
(`builder`, `digest`, `finalization`, `floor`, `qualification`). Lines 1–769 are
non-test code; `:771-1379` are unit tests.

## Inputs and outputs

| Entry point | Inputs | Output | I/O |
| --- | --- | --- | --- |
| `PackConfig::from_toml` (`:678`) | TOML string | `PackConfig` | none |
| `PackConfig::resolve` (`:709`) | `&PackConfig`, `&DistributionContract`, `release_id`, `source_revision`, `&[String]` selected | `ReleasePlan` | none |
| `ReleasePlan::to_json` (`:671`) | `&ReleasePlan` | compact JSON `String` | none |
| `build_manifest` (`:341`) | `&DistributionContract`, `&FinalizedReleaseInput` | `eggpack_manifest::ReleaseManifest` | reads supplied files |

Planning is pure. The only filesystem access in this file is the crate's single
file-digest authority, `crate::file_digest` (`src/digest.rs:102-114`), called
exclusively from `build_manifest` (`:372-373`, `:419-420`); it opens exactly the
paths the caller named in `FinalizedTargetInput`. `test_temp_dir` (`:31`) is
`#[cfg(test)]`-only.

## Policy vocabulary

All policy types are `serde(rename_all = "snake_case")`; every input struct
carries `deny_unknown_fields` (`HostRequirement :518`, `ToolchainRequirement
:533`, `TargetPolicy :567`, `PackConfig :596`), and the two input documents are
additionally version-gated. The two output types carry it as well
(`ReleasePlan :605`, `PlannedTarget :618`): `eggpack-ci` parses a plan back and
compares it with the canonical re-resolution, so an unknown field in a plan
document is refused rather than ignored.

| Type | Variants / fields | Load-bearing meaning |
| --- | --- | --- |
| `BuildStrategy` (`:456`) | `NativeCargo`, `CargoZigbuild` | Finite, explicit; planning never executes either (`:453`). Mutually exclusive cross-tool presence is enforced at `:231-246`. |
| `HostOs` (`:465`) | `Linux`, `Macos`, `Windows` | Build-host family. |
| `HostArch` (`:476`) | `X86_64`, `Aarch64`, `Armv7` | Build-host family; serialized as `x86_64` (`:476-486`). |
| `Qualification` (`:487`) | `Native`, `DeferredNative`, `Emulated`, `Structural` | Intent only, stored as intent (`:488`). Only `Native` triggers host matching (`:260-270`). Builder-independent by design: a `CargoZigbuild` candidate is admissible for `Native` (`:489-496`). |
| `SupportTier` (`:507`) | `Required`, `NonGating`, `Experimental` | Gating tier. Carried through planning; **not enforced anywhere in this file** (see Boundaries). |
| `HostRequirement` (`:518`) | `os`, `arch` | Provider-neutral host capability. |
| `ToolchainRequirement` (`:533`) | `rust: String`, `cargo_zigbuild: Option<String>`, `zig: Option<String>` | No version is inferred from the host (`:524`). `zig` is an additive optional field for schema-v1 back-compat (`:526-530`). |
| `CompatibilityFloor` (`:546`) | `None`, `Glibc{major,minor}`, `Macos{major,minor}` | Internally tagged `kind` (`:545`). A build-policy input, not a proof (`:581-587`). |
| `TargetPolicy` (`:567`) | target, strategy, host_os, host_arch, `qualification_host`, toolchain, floor, qualification, support | One target's full producer policy. |
| `PackConfig` (`:596`) | `schema_version: u32`, `targets: Vec<TargetPolicy>` | Strict v1 document. |
| `ReleasePlan` (`:605`) | `schema_version`, `release_id`, `source_revision`, `targets: Vec<PlannedTarget>` | Intent, canonically ordered. Not evidence. |
| `PlannedTarget` (`:618`) | `target`, `policy: TargetPolicy`, `artifact_form: PlannedAssetForm`, `floor_status: FloorVerificationStatus` | Canonical triple + resolved policy + contract-derived form + the declared floor's plan-time status (`:625-633`). |
| `PlannedAssetForm` (`:638`) | `Direct`, `Bundle`, `Archive` | Mirrors `eggpack_contract::AssetForm` (`:746-750`) but carries no names — identity is re-derived from the contract at build/manifest time. |
| `CoreError` (`:74`) | newtype over `String` | Opaque by design: paths and file contents are intentionally omitted (`:72`). Every failure funnels through `err()` (`:81-83`). |
| `FinalizedTargetInput` (`:48`) | `target`, `release_files`, `archive_members` | Explicit per-target filename → local path maps. |
| `FinalizedReleaseInput` (`:59`) | `product_id`, `release_id`, `source_revision`, `targets`, `evidence_references` | Caller-supplied release identity and inventory. |

`ReleasePlan` is deliberately a lossless projection: `PlannedTarget.policy`
retains the whole `TargetPolicy` (`:622`), so a plan can be re-serialized and
re-resolved without consulting the original config. The added
`PlannedTarget.floor_status` (`:625-633`) is the one field that is *derived*
rather than retained: it is a pure function of the retained policy
(`planned_floor_status`, `src/floor.rs:105-132`), so it cannot be chosen by a
caller and cannot disagree with the declaration it summarises.

Release identity mapping is owned by `eggpack-ci`, not by `PackConfig` or
`ReleasePlan`. The default mode passes the exact tag as `release_id`. A checked-in
CI policy may opt into the finite `vMAJOR.MINOR.PATCH` → `MAJOR.MINOR.PATCH`
mapping while the exact source tag and 40-hex revision remain separate runtime
facts. CI calls this same pure `PackConfig::resolve` with the mapped manifest ID
and source revision. The core schema and ReleaseManifest v1 wire format do not
change.

## `PackConfig` -> `ReleasePlan`

`from_toml` (`:678-707`) is the **weak** gate: `toml::from_str` under
`deny_unknown_fields`, then `schema_version == 1`, non-empty `targets`
(`:680-682`), a bounded target count (`:687-689`), and a per-target scan
rejecting empty targets, duplicate `target` strings, empty/over-64-byte `rust`,
and the `CargoZigbuild` ⇔ `cargo_zigbuild.is_some()` coupling (`:697-706`).

`validate_policy` (`:208-272`) is the **strict** gate, and runs only inside
`resolve` (`:724`) — not in `from_toml`. A parsed-but-unresolved config can
therefore still carry an invalid policy. The leniency is intentional and
commented: the additive `zig` field's presence rules are deferred to
resolution, "which fails closed rather than selecting an ambient Zig"
(`:691-696`).

`resolve` (`:709-769`) runs in four ordered steps:

1. **Version gate.** `schema_version != 1` → error (`:717-719`). The
   *contract's* own schema version is not re-checked here; it is guaranteed by
   contract parsing.
2. **Policy ingest and validation.** Every declared policy is resolved through
   `contract.resolve` (exact-match triple or alias, `:721-723`), validated,
   and keyed by **canonical** triple into a `BTreeMap` (`:719`, `:725`). Two
   distinct aliases collapsing to one triple is rejected as
   "duplicate canonical policy target" (`:725-727`). Note this iterates *all*
   declared policies, not just selected ones: a bad policy on an unselected
   target still fails resolution.
3. **Selection and coverage.** Each `selected` entry resolves through the
   contract (`:732-734`); duplicates are rejected (`:735-737`), and every
   selected target must have a resolved policy or the call fails with
   "selected target has no policy" (`:738-742`). `policy.target` is
   **rewritten to the canonical triple** (`:743`), and `artifact_form` is
   derived solely from the contract's `AssetForm` (`:744-748`). The floor status
   is derived from the resolved policy (`:749`). Empty selection is an error
   (`:757-759`).
4. **Canonical sort and assembly.** `targets.sort_by(|a, b| a.target.cmp(&b.target))`
   (`:760`) — byte-order `Ord` over canonical triples, which is what makes the
   plan deterministic regardless of `selected` order or policy declaration order.
   The `BTreeMap` at `:719` gives the same ordering guarantee on the policy side.

`to_json` (`:671-674`) first runs `ReleasePlan::validate` (`:653-669`), which
re-checks `schema_version == 1`, non-empty `release_id`/`source_revision`, and
non-empty `targets` (`:654-660`), and which also requires every target's
`floor_status` to equal the status its policy implies (`:661-667`). Those two
string bounds are *only* enforced here; `resolve` itself does not reject empty
identity strings (`:761-766`).

## `validate_policy` and native qualification host matching

`validate_policy` (`:208-272`) enforces, in order:

- **Toolchain bounds** (`:209-230`): `rust` non-empty, ≤64 bytes, alphanumeric
  or `.-+_`; optional `cargo_zigbuild` and `zig` additionally pass
  `valid_tool_version` (`:112-127`) — a **complete `MAJOR.MINOR.PATCH`**, with an
  optional SemVer pre-release/build suffix. A partial pin is rejected: `0` and
  `0.14` would satisfy the preflight comparison against almost any reported
  version, leaving a declared requirement effectively unset while every check
  still reported success. Failure message is the single shared "toolchain
  requirement is empty, overlong, or not a complete version".
- **Strategy/tool coupling** (`:231-246`): `NativeCargo` must declare neither
  cross-tool field; `CargoZigbuild` must declare **both**. Fails closed — there
  is no ambient-tool fallback.
- **Floor/target coherence** (`:247-259`): `Glibc` requires `-linux-gnu` in the
  triple; `Macos` requires the `-apple-darwin` suffix. This is the *triple* half
  of the canonical floor rule (`floor_applies_to_triple`, `:144-165`); the
  *strategy* half (`floor_applies_to_strategy`, `:166-187`) is enforced later,
  at build-command time in `cargo_command`, not here.
- **Native host matching** (`:260-270`).

For `Qualification::Native`, the effective host is
`policy.qualification_host` if present, else synthesized from `host_os` +
`host_arch` (`:261-264`). `host_matches_target` (`:92-111`) then requires **both**
dimensions to agree, using prefix/substring matching rather than a full triple
parse:

| Host dimension | Test on the triple |
| --- | --- |
| `X86_64` / `Aarch64` / `Armv7` | `starts_with("x86_64-")` / `"aarch64-"` / `"armv7-"` (`:93-97`) |
| `Linux` / `Macos` / `Windows` | `contains("-linux-")` / `ends_with("-apple-darwin")` / `contains("-windows-")` (`:98-102`) |

A mismatch returns a `CoreError` (`:265-269`), so a `Native` policy that
cannot be host-matched can never reach a plan at all — planning fails closed
before any build or qualification begins.

The `HostMismatch` **record** is not produced here. It is
`QualificationFailure::HostMismatch` (`qualification.rs:210-236`), emitted as
`QualificationStatus::Failed(QualificationFailure::HostMismatch)` for native,
deferred-native, and emulated paths (`qualification.rs:736-746`, `:768-781`,
`:806-816`).
A non-matching host is a failed record — never a pass, never a skip, and never
an executed smoke (asserted at `qualification.rs:2013-2053`). See
`core-qualification.md`.

**A native run does not prove a declared deployment floor.** `floor` is a
build-policy input carried into the cross-tool build command; the doc comment
states the boundary directly (`:581-587`): native evidence proves that the
exact candidate executes on the qualifying host, "it is not independent proof
that the produced binary honours this declared minimum runtime." `validate_policy`
checks only that the floor is *coherent with the triple* (`:247-259`) — never
that the binary meets it.

**Planning records the floor; it never verifies it.** The plan carries the
declared floor (`TargetPolicy::floor`) and the status that declaration implies
(`PlannedTarget.floor_status`, `:625-633`). `planned_floor_status`
(`src/floor.rs:105-131`) is a pure function of the policy:
`CompatibilityFloor::None` → `NotDeclared`; a glibc floor under
`CargoZigbuild` → `ToolchainBound` (the floor is bound into the toolchain by
the triple rewrite); a glibc floor under `NativeCargo` →
`Unverified(PendingInspection)` (the bytes have not been inspected yet); a
macOS floor → `Unverified(FloorKindNotVerifiable)` (Mach-O deployment-target
verification is a named follow-on). The byte-level verification itself is
never done here — it belongs to qualification, and it is recorded on the
evidence (`core-qualification.md`).

## `build_manifest`

`build_manifest` (`:341-451`) converts caller-supplied finalized files into an
`eggpack_manifest::ReleaseManifest`. It is the only place this file touches the
filesystem or hashes bytes.

1. **Identity and bounds** (`:345-350`): `contract.product.id` must equal
   `input.product_id`; target count must be 1–256.
2. **Per target** (`:353-438`): expand via `contract.expand(target, release_id)`
   (`:354-356`), rejecting duplicate canonical triples (`:357-359`).
3. **Exact inventory** (`:360-366`): derives the expected file set with
   `expected_release_files`, builds a `ReleaseInventory` from the caller's keys,
   and requires `validate_release_inventory(&expected, &inventory,
   ExtrasPolicy::Exact).is_conformant()`. This is **stricter than the contract's
   default** `AllowExtras` — a missing sidecar and an unexpected extra file are
   both fatal.
4. **Form-specific records** (`:380-433`): `Direct` and `Bundle` reject any
   `archive_members` input (`:382-384`, `:391-393`). `Archive` requires
   `archive_members.len() == members.len()` (`:408-410`) and then looks each
   member up by `source`. Note this is a length check plus per-source lookup,
   **not** `validate_archive_member_inventory`, so a same-length map with wrong
   keys fails as "required archive member path missing" rather than as a
   conformance report.
5. **Evidence and self-check** (`:434-450`): assembles `TargetRecord`s in caller
   order, builds the manifest with `schema_version: 1`, and finishes with
   `manifest.validate()` (`:447-449`) so the constructed document must satisfy
   `eggpack-manifest` bounds — `evidence_references` and `source_revision` are
   bounded there, not here.

`crate::file_digest` (`src/digest.rs:102-114`) is the sole byte reader: it
resolves the path with `symlink_metadata` and requires a regular non-symlink
file, rejects an empty file before reading it, streams through a fixed 64 KiB
buffer (`src/digest.rs:20-24`), accumulates size with overflow checking
(`src/digest.rs:138-141`), and re-inspects the path after the read so a path
that changed length or type is refused (`src/digest.rs:143-155`). SHA-256 plus
size are **integrity facts only** — no signature or provenance claim is made or
implied.

## Boundaries / non-goals

- **Not evidence.** `ReleasePlan` and `PlannedTarget` are declared intent;
  `ReleaseManifest` is the only final-bytes evidence. `PlannedAssetForm` carries
  no filenames, so a plan cannot stand in for a manifest.
- **Never executes a strategy.** `BuildStrategy` is a value, not a switch
  (`:453`).
- **`SupportTier` is inert here.** `resolve` does not require that every
  `Required` target be selected, and no tier comparison exists in this file.
  Selection coverage is caller-driven.
- **No ambient inference.** Toolchain versions are never read from the host,
  and no tool lookup, version probe, or fallback happens at planning.
- **No floor verification.** Planning records the declared floor and the status it
  implies; the byte-level verification is qualification's job (see above).
- **No ordering, publishing, or network.** No semver comparison, no process
  execution, no HTTP, no archive extraction. Archive/tar handling lives in
  `finalization`, not here.
- **Errors are opaque.** `CoreError` carries a fixed message string; no path,
  filename, or file content is ever placed in an error (`:72`).
- `file_digest` rejects symlinks and empty files; it does **not** verify that a
  file sits inside a finalized output root — that containment is
  `finalization`'s invariant.

## Dependencies / dependents

Own dependencies (`crates/eggpack-core/Cargo.toml:17-26`):

| Crate | Used here for |
| --- | --- |
| `eggpack-contract` | `DistributionContract::{resolve, expand}`, `AssetForm`, `ExpandedAssets`, `expected_release_files`, `validate_release_inventory`, `ExtrasPolicy`, `ReleaseInventory` (`Cargo.toml:15`) |
| `eggpack-manifest` | `ReleaseManifest`, `TargetRecord`, `ArtifactForm`, `ArtifactRecord`, `BundleRecord`, `ArchiveMemberRecord`, `ByteEvidence`, `validate_sha256_hex` (`Cargo.toml:16`) |
| `serde` / `serde_json` | derive + strict `deny_unknown_fields`; compact plan JSON (`Cargo.toml:17-18`, `:671-674`) |
| `sha2` | `Sha256` streaming digest, used only by the digest authority (`Cargo.toml:19`, `src/digest.rs`) |
| `toml` | `PackConfig::from_toml` (`Cargo.toml:20`, `:678`) |

`command-group`, `tar`, and `flate2` (`Cargo.toml:21-23`) are declared for the
sibling `finalization` stage, not for this file.

Dependents (path, `0.1.0`): `eggpack-cli`, `eggpack-ci`, `eggpack-github`. No
`eggpack-*` crate depends on this module's planning types other than through
`eggpack-core` itself.

## Sibling deep dives

- [Planning — this file](core-planning.md)
- [Build stage](core-build.md)
- [Qualification stage](core-qualification.md)
- [Finalization stage](core-finalization.md)
- [`eggpack-core` crate overview](core.md)
- [Workspace overview](overview.md)
