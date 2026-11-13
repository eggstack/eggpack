# Build/Qualification M008a Corrective — Floor Contradiction Mask and ELF Offset Overflow

Status: **closed**.

Subsystem: Build and qualification.

Predecessor: `plans/implementation/build-qualification/008-compatibility-floor-verification-corrective.md` (M008, implementation `7e7cb6a`, closure `plans/closure/build-qualification/008-status.md`).

Triggered by: the 2026-10-23 post-implementation review of the M008 chain, which found that M008's new bounded ELF floor reader could report a **contradicted** floor as `Unverified`, and that its virtual-address translation contained an unchecked addition.

M008's closure record remains historical and is not rewritten. This corrective is a new `NNNa` plan, per the repository's closure discipline.

## 1. Objective

Make the compatibility-floor inspector fail closed in the two ways the review found it did not.

The milestone's headline guarantee is that a declared floor which the produced bytes contradict is a **hard failure**. The implementation did not always deliver that: an ordinary system binary could carry the floor contradiction and still be recorded as `Unverified` — which is admissible, so the target passed qualification with a false declared floor.

## 2. Current evidence and the defects

### 2.1 Defect A — an unrecognized version name masks a contradicted floor

`parse_glibc_version` returned `Err(UnrecognizedVersion)` for any `GLIBC_`-prefixed name that is not a plain 2–3 component numeric version, and `read_glibc_requirement` propagated that error with `?`. The consequence: the scan was abandoned at the **first** such name, discarding every other requirement the image carries.

`GLIBC_PRIVATE` and `GLIBC_ABI_DT_RELR` are real, common `GLIBC_`-prefixed version names. On the review machine, 38 system binaries require `GLIBC_PRIVATE` and 47 require `GLIBC_ABI_DT_RELR`.

The failure is not "a binary became unverifiable". It is that the *contradiction* was lost:

```text
$ readelf -V /usr/bin/getent
  Name: GLIBC_2.36    Name: GLIBC_ABI_DT_RELR    Name: GLIBC_2.38    Name: GLIBC_PRIVATE  ...

floor = glibc 2.17
wrong: Unverified(UnrecognizedVersionRequirement)   -> admissible -> the target PASSES
right: NotSatisfied                                  -> a hard failure
```

`/usr/bin/getent` requires `GLIBC_2.38`, which is far newer than the declared 2.17 floor. The reader saw `GLIBC_ABI_DT_RELR` before `GLIBC_2.38`, abandoned the scan, and reported the floor as unverifiable — and an unverifiable floor is admissible, so the release shipped a binary that violates its own declared floor.

This directly contradicts M008 §7:

> *"A floor that is declared and **contradicted** by the bytes … MUST fail closed and MUST NOT be reported as `Unverified`."*

The synthetic pair isolates it: an image requiring `GLIBC_2.34` alone against a 2.17 floor is `NotSatisfied`; the same image with `GLIBC_PRIVATE` added becomes `Unverified`.

### 2.2 Defect B — the virtual-address translation can overflow

`map_vaddr` translated a virtual address back into a file offset with an unchecked addition:

```rust
return Ok(segment.offset + (vaddr - segment.vaddr));
```

`segment.offset` is read straight from the ELF program header and is never bounded against the file length. Every other offset computation in the module is overflow-checked; this one was not. A crafted `PT_LOAD` with `p_offset` near the top of the `u64` range overflows the addition, which **panics** in a dev or test build and wraps in a release build. The producer's qualification path must not panic on a candidate's bytes, and a release build must not translate a virtual address into a wrapped file offset.

### 2.3 The observed rule that was actually implemented

The implemented rule was "the first unrecognized name ends the inspection". The required rule is "the highest numeric requirement decides, and an unrecognized name is recorded". Those are different rules, and only the second one satisfies M008 §7.

## 3. Invariants

- A floor declared and **contradicted** by the bytes MUST be a hard failure and MUST NOT be recorded as `Unverified`. This holds regardless of any other version name the image carries.
- Every requirement in the image MUST be visited. An unrecognized name MUST NOT discard the requirements observed alongside it.
- A floor whose numeric requirements fit the declared floor but which also carries an unrecognized name MUST NOT be reported as `Verified`: the floor is not proven. It stays `Unverified`, which is admissible but is not a positive claim.
- Every offset computation MUST be overflow-checked, and every offset MUST be checked against the real file length before the read.
- No arithmetic on candidate bytes may panic. A malformed candidate is a recorded status, never a crash.
- The set of floor statuses, their meanings, their ranking, and the admissibility matrix MUST NOT change. This corrective changes how a status is *decided*, not which statuses exist.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-core/src/floor.rs` only.

**Out of scope:**

- The status enum, the reason enum, `planned_floor_status`, `floor_status_admissible`, `aggregate_floor_status`, and the ranking that drives them. Their behaviour is correct and is unchanged.
- The qualification and evidence surfaces that consume the status (`crates/eggpack-core/src/qualification.rs`, `crates/eggpack-core/src/lib.rs`). They already enforce that `NotSatisfied` fails the qualification; they are unchanged.
- The bounded ELF reader's structural rules — header parsing, program-header and dynamic-table bounds, the string-table bound, and the exact-length reads. They are correct and are unchanged.
- Any change to `ReleaseManifest` v1, `schema_version`, or the published `eggpack-manifest` crate.
- `crates/eggpack-ci/**` and `crates/eggpack-github/**`.

## 5. Required production changes

1. Replace the abort-on-unrecognized behaviour with a classification that distinguishes three cases: not a glibc name, a numeric glibc version, and a `GLIBC_`-prefixed name that is not a plain numeric version.

2. `read_glibc_requirement` visits **every** requirement. It returns both the highest numeric requirement observed and whether an unrecognized name was observed.

3. The status decision order is:
   - a numeric requirement that exceeds the declared floor → `NotSatisfied` (decided first, and not downgraded by an unrecognized name observed alongside it);
   - otherwise, an unrecognized name was observed → `Unverified(UnrecognizedVersionRequirement)`;
   - otherwise, no requirement at all → `Unverified(NoVersionRequirement)`;
   - otherwise → `Verified`.

4. The virtual-address translation uses an overflow-checked addition and reports a malformed candidate rather than panicking or wrapping.

## 6. Required tests

1. A synthetic image carrying a numeric requirement that exceeds the declared floor **and** an unrecognized name (`GLIBC_PRIVATE`, `GLIBC_ABI_DT_RELR`) is `NotSatisfied` — the defect's exact shape.
2. The same image against a floor its numeric requirements satisfy is `Unverified(UnrecognizedVersionRequirement)`, never `Verified`.
3. An image carrying only an unrecognized name is `Unverified`, never `Verified`.
4. A numeric requirement that appears **after** an unrecognized one is still observed by the reader.
5. A crafted `PT_LOAD` whose file offset is near the top of the `u64` range reports a malformed candidate instead of panicking.
6. The existing floor tests continue to pass unchanged.

## 7. Verification

`cargo test -p eggpack-core --all-targets --all-features --locked` and `cargo clippy -p eggpack-core --all-targets --all-features --locked -- -D warnings`.

Additionally, the correction was checked against real system binaries on the review machine, which is the evidence the synthetic tests cannot give: `/usr/bin/getent`, `/usr/bin/iconv`, and `/usr/bin/gencat` now report `NotSatisfied` against a 2.17 floor, where they previously reported `Unverified` and passed.

The workspace gate is not the acceptance criterion for this corrective; it runs once for the whole review line.

## 8. Closure

Closed at `plans/closure/build-qualification/008a-status.md`.
