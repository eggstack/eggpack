# `eggpack-core` — Planning stage (`src/lib.rs`) — Deep Dive

Owns producer **policy intake and pure planning**: parsing strict `PackConfig` v1
policy, resolving it against the `eggpack-contract` layout authority into a
canonically sorted `ReleasePlan` (intent, never evidence), and assembling a
schema-v1 `ReleaseManifest` from explicitly supplied finalized files. It is one
of four `eggpack-core` stage deep dives, alongside `core-build.md`,
`core-qualification.md`, and `core-finalization.md`.

Crate root: `crates/eggpack-core/src/lib.rs`, 1056 lines, `#![forbid(unsafe_code)]`
(`:1`), `#![deny(missing_docs)]` (`:2`). The file is the crate root and declares
the three sibling stages, re-exported wholesale at `:5-10`
(`builder`, `finalization`, `qualification`). Lines 1–583 are non-test code;
`:585-1056` are unit tests.

## Inputs and outputs

| Entry point | Inputs | Output | I/O |
| --- | --- | --- | --- |
| `PackConfig::from_toml` (`:502`) | TOML string | `PackConfig` | none |
| `PackConfig::resolve` (`:526`) | `&PackConfig`, `&DistributionContract`, `release_id`, `source_revision`, `&[String]` selected | `ReleasePlan` | none |
| `ReleasePlan::to_json` (`:489`) | `&ReleasePlan` | compact JSON `String` | none |
| `build_manifest` (`:166`) | `&DistributionContract`, `&FinalizedReleaseInput` | `eggpack_manifest::ReleaseManifest` | reads supplied files |

Planning is pure. The only filesystem access in this file is `digest_file`
(`:276`), called exclusively from `build_manifest`; it opens exactly the paths
the caller named in `FinalizedTargetInput`. `test_temp_dir` (`:31`) is
`#[cfg(test)]`-only.

## Policy vocabulary

All policy types are `serde(rename_all = "snake_case")`; every input struct
carries `deny_unknown_fields` (`HostRequirement :369`, `ToolchainRequirement
:384`, `TargetPolicy :418`, `PackConfig :447`), and the two input documents are
additionally version-gated. Output-only types (`ReleasePlan :456`,
`PlannedTarget :468`) do not, since they are never parsed from untrusted input.

| Type | Variants / fields | Load-bearing meaning |
| --- | --- | --- |
| `BuildStrategy` (`:308`) | `NativeCargo`, `CargoZigbuild` | Finite, explicit; planning never executes either (`:305`). Mutually exclusive cross-tool presence is enforced at `:126-141`. |
| `HostOs` (`:317`) | `Linux`, `Macos`, `Windows` | Build-host family. |
| `HostArch` (`:328`) | `X86_64`, `Aarch64`, `Armv7` | Build-host family; serialized as `x86_64` (`:328-334`). |
| `Qualification` (`:339`) | `Native`, `DeferredNative`, `Emulated`, `Structural` | Intent only, stored as intent (`:336`). Only `Native` triggers host matching (`:151`). Builder-independent by design: a `CargoZigbuild` candidate is admissible for `Native` (`:342-347`). |
| `SupportTier` (`:359`) | `Required`, `NonGating`, `Experimental` | Gating tier. Carried through planning; **not enforced anywhere in this file** (see Boundaries). |
| `HostRequirement` (`:370`) | `os`, `arch` | Provider-neutral host capability. |
| `ToolchainRequirement` (`:385`) | `rust: String`, `cargo_zigbuild: Option<String>`, `zig: Option<String>` | No version is inferred from the host (`:376`). `zig` is an additive optional field for schema-v1 back-compat (`:377-382`). |
| `CompatibilityFloor` (`:398`) | `None`, `Glibc{major,minor}`, `Macos{major,minor}` | Internally tagged `kind` (`:397`). A build-policy input, not a proof (`:433-439`). |
| `TargetPolicy` (`:419`) | target, strategy, host_os, host_arch, `qualification_host`, toolchain, floor, qualification, support | One target's full producer policy. |
| `PackConfig` (`:448`) | `schema_version: u32`, `targets: Vec<TargetPolicy>` | Strict v1 document. |
| `ReleasePlan` (`:456`) | `schema_version`, `release_id`, `source_revision`, `targets: Vec<PlannedTarget>` | Intent, canonically ordered. Not evidence. |
| `PlannedTarget` (`:468`) | `target`, `policy: TargetPolicy`, `artifact_form: PlannedAssetForm` | Canonical triple + resolved policy + contract-derived form. |
| `PlannedAssetForm` (`:479`) | `Direct`, `Bundle`, `Archive` | Mirrors `eggpack_contract::AssetForm` (`:561-565`) but carries no names — identity is re-derived from the contract at build/manifest time. |
| `CoreError` (`:74`) | newtype over `String` | Opaque by design: paths and file contents are intentionally omitted (`:72`). Every failure funnels through `err()` (`:81`). |
| `FinalizedTargetInput` (`:48`) | `target`, `release_files`, `archive_members` | Explicit per-target filename → local path maps. |
| `FinalizedReleaseInput` (`:59`) | `product_id`, `release_id`, `source_revision`, `targets`, `evidence_references` | Caller-supplied release identity and inventory. |

`ReleasePlan` is deliberately a lossless projection: `PlannedTarget.policy`
retains the whole `TargetPolicy` (`:472`), so a plan can be re-serialized and
re-resolved without consulting the original config.

## `PackConfig` -> `ReleasePlan`

`from_toml` (`:502-524`) is the **weak** gate: `toml::from_str` under
`deny_unknown_fields`, then `schema_version == 1`, non-empty `targets`
(`:504-506`), and a per-target scan rejecting empty targets, duplicate
`target` strings, empty/over-64-byte `rust`, and the
`CargoZigbuild` ⇔ `cargo_zigbuild.is_some()` coupling (`:513-522`).

`validate_policy` (`:105-163`) is the **strict** gate, and runs only inside
`resolve` (`:541`) — not in `from_toml`. A parsed-but-unresolved config can
therefore still carry an invalid policy. The leniency is intentional and
commented: the additive `zig` field's presence rules are deferred to
resolution, "which fails closed rather than selecting an ambient Zig"
(`:508-512`).

`resolve` (`:526-582`) runs in four ordered steps:

1. **Version gate.** `schema_version != 1` → error (`:533-535`). The
   *contract's* own schema version is not re-checked here; it is guaranteed by
   contract parsing.
2. **Policy ingest and validation.** Every declared policy is resolved through
   `contract.resolve` (exact-match triple or alias, `:538-540`), validated,
   and keyed by **canonical** triple into a `BTreeMap` (`:536, :542`). Two
   distinct aliases collapsing to one triple is rejected as
   "duplicate canonical policy target" (`:542-544`). Note this iterates *all*
   declared policies, not just selected ones: a bad policy on an unselected
   target still fails resolution.
3. **Selection and coverage.** Each `selected` entry resolves through the
   contract (`:549-551`); duplicates are rejected (`:552-554`), and every
   selected target must have a resolved policy or the call fails with
   "selected target has no policy" (`:555-559`). `policy.target` is
   **rewritten to the canonical triple** (`:560`), and `artifact_form` is
   derived solely from the contract's `AssetForm` (`:561-565`). Empty selection
   is an error (`:572-574`).
4. **Canonical sort and assembly.** `targets.sort_by(|a, b| a.target.cmp(&b.target))`
   (`:575`) — byte-order `Ord` over canonical triples, which is what makes the
   plan deterministic regardless of `selected` order or policy declaration order.
   The `BTreeMap` at `:536` gives the same ordering guarantee on the policy side.

`to_json` (`:489-498`) re-checks `schema_version == 1`, non-empty
`release_id`/`source_revision`, and non-empty `targets` (`:490-496`) before
emitting compact `serde_json`. Those two string bounds are *only* enforced here;
`resolve` itself does not reject empty identity strings (`:576-581`).

## `validate_policy` and native qualification host matching

`validate_policy` (`:105-163`) enforces, in order:

- **Toolchain bounds** (`:106-125`): `rust` non-empty, ≤64 bytes, alphanumeric
  or `.-+_`; optional `cargo_zigbuild` and `zig` additionally pass
  `valid_tool_version` (`:97-103`) — a **complete `MAJOR.MINOR.PATCH`**, with an
  optional SemVer pre-release/build suffix. A partial pin is rejected: `0` and
  `0.14` would satisfy the preflight comparison against almost any reported
  version, leaving a declared requirement effectively unset while every check
  still reported success. Failure message is the single shared "toolchain
  requirement is empty, overlong, or not a complete version".
- **Strategy/tool coupling** (`:126-141`): `NativeCargo` must declare neither
  cross-tool field; `CargoZigbuild` must declare **both**. Fails closed — there
  is no ambient-tool fallback.
- **Floor/target coherence** (`:142-150`): `Glibc` requires `-linux-gnu` in the
  triple; `Macos` requires the `-apple-darwin` suffix.
- **Native host matching** (`:151-161`).

For `Qualification::Native`, the effective host is
`policy.qualification_host` if present, else synthesized from `host_os` +
`host_arch` (`:152-155`). `host_matches_target` (`:84-96`) then requires **both**
dimensions to agree, using prefix/substring matching rather than a full triple
parse:

| Host dimension | Test on the triple |
| --- | --- |
| `X86_64` / `Aarch64` / `Armv7` | `starts_with("x86_64-")` / `"aarch64-"` / `"armv7-"` (`:86-88`) |
| `Linux` / `Macos` / `Windows` | `contains("-linux-")` / `ends_with("-apple-darwin")` / `contains("-windows-")` (`:91-93`) |

A mismatch returns a `CoreError` (`:157-160`), so a `Native` policy that
cannot be host-matched can never reach a plan at all — planning fails closed
before any build or qualification begins.

The `HostMismatch` **record** is not produced here. It is
`QualificationFailure::HostMismatch` (`qualification.rs:216`), emitted as
`QualificationStatus::Failed(QualificationFailure::HostMismatch)` for native,
deferred-native, and emulated paths (`qualification.rs:694`, `:725`, `:760`).
A non-matching host is a failed record — never a pass, never a skip, and never
an executed smoke (asserted at `qualification.rs1845-1870`). See
`core-qualification.md`.

**A native run does not prove a declared deployment floor.** `floor` is a
build-policy input carried into the cross-tool build command; the doc comment
states the boundary directly (`:433-439`): native evidence proves that the
exact candidate executes on the qualifying host, "it is not independent proof
that the produced binary honours this declared minimum runtime." `validate_policy`
checks only that the floor is *coherent with the triple* (`:142-150`) — never
that the binary meets it.

## `build_manifest`

`build_manifest` (`:166-274`) converts caller-supplied finalized files into an
`eggpack_manifest::ReleaseManifest`. It is the only place this file touches the
filesystem or hashes bytes.

1. **Identity and bounds** (`:170-175`): `contract.product.id` must equal
   `input.product_id`; target count must be 1–256.
2. **Per target** (`:178-261`): expand via `contract.expand(target, release_id)`
   (`:179-181`), rejecting duplicate canonical triples (`:182-184`).
3. **Exact inventory** (`:185-191`): derives the expected file set with
   `expected_release_files`, builds a `ReleaseInventory` from the caller's keys,
   and requires `validate_release_inventory(&expected, &inventory,
   ExtrasPolicy::Exact).is_conformant()`. This is **stricter than the contract's
   default** `AllowExtras` — a missing sidecar and an unexpected extra file are
   both fatal.
4. **Form-specific records** (`:204-256`): `Direct` and `Bundle` reject any
   `archive_members` input (`:206-208`, `:215-217`). `Archive` requires
   `archive_members.len() == members.len()` (`:232-234`) and then looks each
   member up by `source`. Note this is a length check plus per-source lookup,
   **not** `validate_archive_member_inventory`, so a same-length map with wrong
   keys fails as "required archive member path missing" rather than as a
   conformance report.
5. **Evidence and self-check** (`:257-273`): assembles `TargetRecord`s in caller
   order, builds the manifest with `schema_version: 1`, and finishes with
   `manifest.validate()` (`:270-272`) so the constructed document must satisfy
   `eggpack-manifest` bounds — `evidence_references` and `source_revision` are
   bounded there, not here.

`digest_file` (`:276-303`) is the sole byte reader: it resolves the path with
`symlink_metadata` and requires a regular non-symlink file (`:277-280`), streams
through a 64 KiB buffer, accumulates size with `checked_add` (`:293-295`),
rejects zero-length files (`:298-300`), and emits lowercase hex SHA-256
(`:302`). SHA-256 plus size are **integrity facts only** — no signature or
provenance claim is made or implied.

## Boundaries / non-goals

- **Not evidence.** `ReleasePlan` and `PlannedTarget` are declared intent;
  `ReleaseManifest` is the only final-bytes evidence. `PlannedAssetForm` carries
  no filenames, so a plan cannot stand in for a manifest.
- **Never executes a strategy.** `BuildStrategy` is a value, not a switch
  (`:305`).
- **`SupportTier` is inert here.** `resolve` does not require that every
  `Required` target be selected, and no tier comparison exists in this file.
  Selection coverage is caller-driven.
- **No ambient inference.** Toolchain versions are never read from the host,
  and no tool lookup, version probe, or fallback happens at planning.
- **No floor verification** beyond triple coherence (see above).
- **No ordering, publishing, or network.** No semver comparison, no process
  execution, no HTTP, no archive extraction. Archive/tar handling lives in
  `finalization`, not here.
- **Errors are opaque.** `CoreError` carries a fixed message string; no path,
  filename, or file content is ever placed in an error (`:72`).
- `digest_file` rejects symlinks and empty files; it does **not** verify that a
  file sits inside a finalized output root — that containment is
  `finalization`'s invariant.

## Dependencies / dependents

Own dependencies (`crates/eggpack-core/Cargo.toml:14-23`):

| Crate | Used here for |
| --- | --- |
| `eggpack-contract` | `DistributionContract::{resolve, expand}`, `AssetForm`, `ExpandedAssets`, `expected_release_files`, `validate_release_inventory`, `ExtrasPolicy`, `ReleaseInventory` (`:12-15`) |
| `eggpack-manifest` | `ReleaseManifest`, `TargetRecord`, `ArtifactForm`, `ArtifactRecord`, `BundleRecord`, `ArchiveMemberRecord`, `ByteEvidence` (`:16-19`) |
| `serde` / `serde_json` | derive + strict `deny_unknown_fields`; compact plan JSON (`:20`, `:497`) |
| `sha2` | `Sha256` streaming digest (`:21`, `:283`) |
| `toml` | `PackConfig::from_toml` (`:503`) |

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
