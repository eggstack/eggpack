# Build/Qualification M008 — Compatibility Floor Verification Corrective

Status: **blocked** on Build/Qualification M007.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/build-qualification-roadmap.md`.
Primary class: capability / evidence-corrective.
Historical upstream authority: ADR-0005 §9 *"Compatibility-floor evidence boundary"*
(`plans/adrs/ADR-0005-native-qualification-for-cross-tool-builds.md:276-286`),
which states that Eggpack's glibc floor is a build-policy input and that
independently verifying the final ELF's required GLIBC symbol versions "is
separate qualification hardening and must be planned independently before
Eggpack claims direct evidence for the floor itself".
Affected closed milestones: Build/Qualification M001 (introduced the
`CompatibilityFloor` policy surface and `validate_policy`), Build/Qualification
M005 (extended the floor semantics to the `CargoZigbuild` triple rewrite),
Build/Qualification M006 (established that native execution is not floor
proof). All three closures remain historical and are not rewritten.
Dependency: **Build/Qualification M007** (`plans/implementation/build-qualification/007-evidence-digest-unification-corrective.md`)
must be closed first, because M008 reuses the canonical file-digest helper M007
establishes when it records the inspected bytes of a candidate.

## 1. Objective

Make the declared compatibility floor a **verified or explicitly unverified
fact** rather than a silently dropped policy input, and implement the
independent verification that ADR-0005 §9 requires before Eggpack claims direct
evidence for a floor.

Concretely:

1. Every target that declares a `CompatibilityFloor` MUST carry a
   machine-readable **floor verification status** in the producer's own
   evidence, distinguishing "the toolchain was bound to this floor" from
   "the produced bytes were independently inspected against this floor" from
   "this floor was declared and is not independently verified".
2. A declared floor MUST never be silently discarded. Under
   `BuildStrategy::NativeCargo` the floor is currently accepted by
   `validate_policy` and then has no effect at all; that MUST become visible.
3. Independent verification MUST be implemented where it is technically
   possible: a GNU/Linux ELF candidate's required `GLIBC_x.y` symbol versions
   are read from the binary and compared against the declared floor.

## 2. Current evidence and the defect

### 2.1 The floor is a declared policy surface

`CompatibilityFloor` (`crates/eggpack-core/src/lib.rs:419-436`) has three
variants: `None`, `Glibc { major, minor }`, and `Macos { major, minor }`. It is
carried on `TargetPolicy.floor` (`crates/eggpack-core/src/lib.rs:454-460`) and
is part of `PackConfig` v1.

### 2.2 It is enforced in exactly one place

`validate_policy` (`crates/eggpack-core/src/lib.rs:157-165`) checks only that
the floor is *shape-compatible* with the target triple:

- `Glibc` requires a `-linux-gnu` triple;
- `Macos` requires an `-apple-darwin` triple.

The floor is *honored* in exactly one place, `cargo_command`
(`crates/eggpack-core/src/builder.rs:357-367`), and only under
`BuildStrategy::CargoZigbuild`, where the target triple is rewritten to
`<triple>.<major>.<minor>` — the cargo-zigbuild glibc-version-suffix
convention. A `Macos` floor under `CargoZigbuild` is refused outright
(`crates/eggpack-core/src/builder.rs:362-364`).

**Under `BuildStrategy::NativeCargo`, a declared floor is accepted and then has
no effect whatsoever.** No error, no flag, no check, no record. The floor
survives into the `ReleasePlan` as intent and stops there.

The workspace already states this boundary correctly in the `TargetPolicy.floor`
doc comment (`crates/eggpack-core/src/lib.rs:456-459`): *"Native qualification
evidence proves that the exact candidate executes on the qualifying native
host; it is not independent proof that the produced binary honours this declared
minimum runtime."* M008 does not contradict that statement — it makes the
unproven floor visible instead of leaving it implicit.

### 2.3 Nothing records that it went unverified

- `QualificationEvidence` (`crates/eggpack-core/src/qualification.rs:290-315`)
  has no floor field.
- `ReleaseManifest`'s `TargetRecord`
  (`crates/eggpack-manifest/src/lib.rs:78-83`) is `{ target, form }` only — no
  floor, no qualification method, no toolchain.

The manifest is final-bytes-only *by design* (ADR-0002), so its silence about
the floor is correct. The problem is that the **plan** carries the declaration
and no artifact anywhere records whether the floor was honored, so "declared
glibc 2.17" and "glibc 2.17 verified" are indistinguishable in every producer
output.

### 2.4 The repository already names this gap

`plans/subsystems/build-qualification-roadmap.md:177` states: *"Independent
verification that a produced binary honours a declared glibc/macOS floor remains
unplanned and is not claimed by M006."* ADR-0005 §9 says the same and requires
it to be planned independently. This milestone is that plan.

### 2.5 The structural reader exists but does not read far enough

`inspect_candidate` (`crates/eggpack-core/src/qualification.rs:1090-1245`)
reads ELF magic and `e_machine`, PE/COFF headers, and the Mach-O header. It
does **not** read program headers, the dynamic segment, or symbol version
requirements. Independent floor verification therefore requires extending the
reader, not starting one.

## 3. Invariants

- A declared compatibility floor is **never silently dropped**. It is either
  honored by the toolchain, independently verified against the produced bytes,
  or explicitly recorded as unverified.
- Native execution evidence is **never** floor proof. A candidate that runs on
  a modern matching host proves execution, not a deployment floor. This is
  ADR-0005 §9 and it is restated here because M008 touches exactly this
  boundary.
- Floor verification describes **the exact produced bytes**. A verification
  result is bound to the candidate's digest and size, so it cannot be carried
  across a byte-changing transformation.
- Verification is **fail-closed**: an unreadable, malformed, or ambiguous
  binary yields `Unverified` with a reason, never `Verified` and never a
  silent pass.
- A verification result MUST NOT claim more than it checked. An ELF with no
  `GLIBC_x.y` symbol version requirements is compatible with any floor, and
  MUST be recorded as such rather than as "floor 2.17 confirmed".
- The published `ReleaseManifest` v1 is **unchanged**. Publishing floor
  evidence to consumers is explicitly out of scope and would need its own ADR.
- A floor declaration remains producer policy. It is never inferred from a
  contract, a consumer, or an ambient host.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-core/**` and the producer-internal evidence
documents it writes (`ReleasePlan`, qualification evidence). A new floor
verification status, the independent ELF inspection, and the plan/evidence
plumbing that carries it.

**Out of scope:**

- any change to `ReleaseManifest` v1, its fields, its `schema_version`, or the
  published `eggpack-manifest` crate. Publishing the floor to consumers is a
  separate compatibility decision and requires its own ADR.
- Windows/PE/COFF floor verification. PE/COFF has no libc floor concept;
  `CompatibilityFloor::Glibc` is already refused for non-GNU/Linux triples and
  `Macos` is Darwin-only, so no PE/COFF work is implied.
- macOS Mach-O deployment-target verification. The Mach-O `minos` field is the
  natural source, but the first milestone establishes the status model and the
  ELF path; Mach-O verification is named as a follow-on, not silently included.
- Changing which floors are *admissible* under `NativeCargo`. See §5 WP2 for
  the recommended handling.
- Any change to qualification execution, smoke bindings, host matching, or the
  `Qualification` / `QualificationMethod` enums.
- Any consumer-side or product-side change.

## 5. Required production changes

### 5.1 A floor verification status

Introduce a finite, serializable status with at least these meanings:

| Status | Meaning |
|---|---|
| `NotDeclared` | the target declares `CompatibilityFloor::None` |
| `ToolchainBound` | the floor was bound into the build toolchain (the `CargoZigbuild` triple rewrite) and the toolchain is trusted to honor it |
| `Verified` | the produced bytes were independently inspected and satisfy the declared floor |
| `Unverified` | a floor was declared but could not be independently verified, with a bounded reason |

The status is finite and closed. It is **not** a free-form string, and it is
not `Option<bool>`.

### 5.2 Where the status lives

- On `PlannedTarget` / `ReleasePlan`, so the declared floor and its status are
  visible in the producer's intent artifact.
- On `QualificationEvidence`, so the produced bytes' floor status is bound to
  the evidence for that exact candidate.
- **Not** on `ReleaseManifest`.

A `ReleasePlan` that declares a floor and does not state a status for it is
invalid. A `QualificationEvidence` whose floor status does not agree with the
plan's declared floor is invalid.

### 5.3 Independent ELF floor verification

Extend the existing structural reader to compute, for a GNU/Linux ELF
candidate, the **maximum `GLIBC_x.y` symbol version requirement** present in
the binary's version requirements, and compare it against the declared floor.

The check must:

- read the program headers to locate `PT_DYNAMIC`;
- read the dynamic entries for the version-requirement structures;
- collect the `GLIBC_` symbol version names and take the highest `major.minor`;
- treat a candidate with **no** `GLIBC_` version requirement as compatible with
  any declared floor, and record that specifically rather than as a generic
  pass;
- treat a malformed, truncated, non-ELF, or unreadable binary as
  `Unverified` with a reason;
- compare `required_glibc <= declared_floor`, so that a binary requiring a
  *newer* glibc than declared **fails** the declared floor.

The comparison direction matters and must be asserted by a test: a binary
requiring `GLIBC_2.34` does **not** satisfy a declared `2.17` floor.

All parsing is bounded: fixed maximum header, program-header, and
dynamic-table sizes; every offset is checked against the file length; every
read is exact-length. This matches the existing `MAX_BINARY_HEADER` discipline
at `crates/eggpack-core/src/qualification.rs:1206`.

### 5.4 The `NativeCargo` case — recommended handling

Under `BuildStrategy::NativeCargo` the floor cannot be bound into the
toolchain, but it **can** be independently verified from the produced ELF. The
recommended handling is therefore:

- a declared glibc floor under `NativeCargo` is **accepted**, and the
  candidate's floor status is computed by independent inspection;
- the result is `Verified` when the bytes satisfy the floor and `Unverified`
  (with a reason) otherwise;
- **a floor that is declared and cannot be verified is not an error** by
  itself, because a native build on a floor-matching host legitimately
  produces floor-compatible bytes — but the status says so explicitly, and no
  downstream artifact may describe it as verified.

The alternative — refusing a glibc floor under `NativeCargo` outright — is
**not recommended**, because it would reject a legitimate producer
configuration and would break the eggsact-shaped five-target shape already
registered in this repository, where the two GNU/Linux targets declare glibc
2.17 floors. If the maintainers prefer refusal, that is a producer-contract
change and must be recorded as such in the closure record rather than folded
silently into this milestone.

### 5.5 The floor must be carried, not dropped

`cargo_command` (`crates/eggpack-core/src/builder.rs:357-367`) continues to
bind the floor into the `CargoZigbuild` triple exactly as it does today, and
continues to refuse a `Macos` floor under `CargoZigbuild` exactly as it does
today. **No existing floor behaviour changes.** What changes is that the
resulting status is now recorded rather than implied.

## 6. Ordered work packages

### WP1 — Failing discriminating tests first

- A `GLIBC_2.34`-requiring ELF against a declared `2.17` floor yields
  `Unverified`/floor-not-satisfied, not `Verified`.
- A `GLIBC_2.17`-requiring ELF against a declared `2.17` floor yields
  `Verified`.
- An ELF with no `GLIBC_` version requirement against a declared `2.17` floor
  yields a distinct "no version requirement" result, not a generic `Verified`
  and not an error.
- A truncated, malformed, or non-ELF file yields `Unverified` with a reason,
  never `Verified`.
- A `ReleasePlan` that declares a floor but omits a status is rejected.
- A `QualificationEvidence` whose floor status contradicts the plan's declared
  floor is rejected.
- A `NativeCargo` target that declares a glibc floor produces a plan that
  *records* the floor rather than dropping it — the test asserts the floor is
  present in the plan and that a status accompanies it.

### WP2 — Status model and plumbing

Define the status type, carry it on the plan and the evidence, and add the
cross-checks that make a missing or contradictory status an error. Validate
that every in-tree `PackConfig`/`ReleasePlan` fixture still resolves.

### WP3 — ELF floor inspection

Extend the structural reader to compute the maximum required `GLIBC_x.y`
version. Reuse the existing bounded-header discipline and the existing
`CandidateFormat::Elf` path. The inspection runs against the exact candidate
bytes, and its result is bound to the candidate's digest and size.

### WP4 — Wiring the status through qualification

`qualify_target` records the floor status for the candidate it inspected. The
status is computed from the bytes actually qualified, so it cannot be carried
across a byte-changing transformation.

### WP5 — Verification and documentation

Run the full gate. Update `architecture/core-qualification.md`,
`architecture/core-planning.md`, `architecture/core-build.md`,
`crates/eggpack-core/README.md`, the build/qualification roadmap, and the
registry. State the ADR-0005 §9 boundary explicitly in the new documentation
rather than leaving it implied.

## 7. Failure, restart and contention semantics

- An unverifiable floor is a **recorded status**, not a hard failure of the
  release. A hard failure here would break every producer that declares a
  floor without an inspectable binary, which is a wider behavioural change
  than this milestone authorizes.
- A floor that is declared and **contradicted** by the bytes — the binary
  requires a newer glibc than declared — is a hard failure of the
  qualification for that target, because the declared floor is then false.
  This case MUST fail closed and MUST NOT be reported as `Unverified`.
- The inspection is read-only and holds no lock. Two concurrent inspections of
  the same candidate observe the same bytes or a changed-file error; each
  observation is self-consistent.
- Restarting re-inspects from the beginning. No resumable state exists.

## 8. Compatibility and migration

- No change to `ReleaseManifest` v1: no field added, no field removed, no
  `schema_version` bump. The published `eggpack-manifest` crate is untouched
  and needs no new release.
- `PackConfig` v1 accepts the same documents it accepts today. A target that
  declares a glibc floor under `NativeCargo` still resolves; it now carries a
  status.
- No checked-in workflow changes, so `eggpack ci check` drift status is
  unchanged for existing consumers. Consumers that adopt the new plan or
  evidence shape must re-render and re-pin, which the closure record must
  state.
- No new dependency. ELF parsing is done with the existing standard-library
  and `sha2` surface; no external ELF crate is introduced, to keep the
  dependency and supply-chain footprint unchanged.
- `eggup-eggpack` is unaffected: it reads `ReleaseManifest`, which does not
  change.

## 9. Required tests

- unit tests for the version-requirement reader: minimal valid ELF, ELF with
  multiple `GLIBC_` versions, ELF with no version requirements, truncated
  dynamic table, truncated program headers, out-of-range offsets, 32-bit and
  64-bit endianness, non-ELF input;
- comparison-direction tests: required newer than declared fails; required equal
  to declared passes; required older than declared passes;
- bound tests: an oversized program-header count or dynamic-entry count is
  refused rather than parsed;
- plan and evidence validation tests for a missing, contradictory, or
  impossible status;
- a regression test that a `NativeCargo` target with a declared glibc floor
  still resolves and still produces a valid plan;
- a regression test that the `CargoZigbuild` triple rewrite at
  `crates/eggpack-core/src/builder.rs:358-367` is byte-unchanged, and that the
  `Macos`-under-zigbuild refusal is unchanged.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-core --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
./scripts/check-local.sh
```

Add a direct narrowed run for the new tests, e.g.
`cargo test -p eggpack-core --all-targets --all-features --locked <new_test_name> -- --nocapture`.
Append `-- --test-threads=1` when diagnosing Windows serialization.

## 11. Documentation updates

- `architecture/core-qualification.md` — the floor status in evidence, and the
  ADR-0005 §9 boundary restated.
- `architecture/core-planning.md` — the floor status on the plan.
- `architecture/core-build.md` — the toolchain-bound case.
- `architecture/overview.md` — line counts and baseline SHA are already stale
  and are **not** re-based here; see
  `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`.
- `crates/eggpack-core/README.md` — the floor status model.
- `plans/subsystems/build-qualification-roadmap.md` — the M008 row and the
  removal of the "remains unplanned" sentence at `:177`.

## 12. Acceptance criteria

1. Every target that declares a `CompatibilityFloor` carries a finite,
   machine-readable floor status on the plan and on the qualification evidence,
   and a missing or contradictory status is a validation error.
2. The floor is no longer silently dropped under `NativeCargo`; the declared
   floor and its status are both present in the producer's artifacts.
3. A GNU/Linux ELF candidate's maximum required `GLIBC_x.y` version is computed
   by bounded inspection, and compared against the declared floor in the
   correct direction, with a binary requiring a newer glibc than declared
   failing closed.
4. A malformed or unreadable binary yields `Unverified` with a reason, never
   `Verified`.
5. `ReleaseManifest` v1 and the published `eggpack-manifest` crate are
   unchanged: no field, no `schema_version`, no release.
6. The `CargoZigbuild` triple rewrite and the `Macos`-under-zigbuild refusal
   are byte-unchanged.
7. The full workspace gate and `./scripts/check-local.sh` are green, and hosted
   CI is green on all lanes.
8. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- independent floor verification would require publishing the floor in
  `ReleaseManifest` (that is an ADR-triggering compatibility decision, not
  this milestone);
- the ELF inspection cannot be made bounded and fail-closed;
- the status model would have to become a free-form string or an open enum;
- a `NativeCargo` glibc floor must be **refused** rather than recorded
  unverified, because that is a producer-contract change that needs its own
  decision;
- the work would require a new external dependency;
- any change to qualification execution, host matching, or the
  `Qualification`/`QualificationMethod` enums appears to be required.

## 14. Closure evidence

Write `plans/closure/build-qualification/008-status.md` with: the reviewed
baseline and implementation SHA; the exact status model and its mapping from
the four statuses to concrete binary observations; the inspection evidence with
exact test names and commands; the bound-proving evidence for the ELF reader;
the regression proof that the zigbuild triple rewrite and the macOS refusal are
unchanged; the explicit statement that `ReleaseManifest` v1 and the published
crate are unchanged; verification commands executed and their actual results;
hosted CI run IDs and the lane matrix; an invariant review against §3; the
disposition of the §5.4 alternative if the maintainers chose refusal rather
than recording; unresolved findings by severity — including the named Mach-O
and PE/COFF follow-ons; and the roadmap and registry updates.

The M001, M005, and M006 closure records remain historical and unchanged.

## 15. Handoff notes

- Start from the M007 closure SHA, not from `fb81aa1400e1f333f6500075f78d944a9242b673`.
  M008 reuses M007's canonical digest helper to bind the inspected bytes.
- The Mach-O deployment-target verification is a **named follow-on**, not an
  implicit omission. Record it as such in the closure record's unresolved
  findings.
- Publishing floor evidence to consumers is a **named follow-on** requiring its
  own ADR. Do not add a floor field to `ReleaseManifest` under this plan.
- This milestone changes what Eggpack can *prove*, not what it produces. A
  release produced before it remains valid and unchanged.
