# Build/Qualification Milestone 008a Closure — Floor Contradiction Mask and ELF Offset Overflow

Status: closed

Source plan: `plans/implementation/build-qualification/008a-floor-contradiction-mask-and-offset-overflow.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md` (registered there)

Triggered by: the 2026-10-23 post-implementation review of the M008 chain.

Reviewed baseline: `e0e1510` (the tip of the review chain), on top of M008's implementation `7e7cb6a`.

## Executive finding

Build/Qualification 008a is closed. A declared compatibility floor that the produced bytes contradict now fails the qualification, on every image, and the floor reader no longer panics on a crafted candidate.

The milestone's design was right and most of its implementation was right. The bounded ELF reader is genuinely bounded: every table is capped, every offset is checked against the real file length before the read, and every comparison is against the actual candidate bytes rather than against a native run. Two defects sat inside that structure and both are now closed.

## Defect A — an unrecognized version name masked a contradicted floor

**This was a fail-open in the milestone's headline guarantee.** A floor declared and contradicted by the bytes was supposed to be a hard failure; on an ordinary system binary it was recorded as `Unverified` — which is admissible — so the target passed qualification with a false declared floor.

The cause was the refusal order inside the reader. `parse_glibc_version` returned `Err(UnrecognizedVersion)` for any `GLIBC_`-prefixed name that is not a plain 2–3 component numeric version, and `read_glibc_requirement` propagated that error with `?`. The scan was therefore abandoned at the **first** such name, discarding every other requirement the image carries.

`GLIBC_PRIVATE` and `GLIBC_ABI_DT_RELR` are real and common. On the review machine, 38 system binaries require `GLIBC_PRIVATE` and 47 require `GLIBC_ABI_DT_RELR` — so the abandonment path was the *normal* path, not an edge case.

The evidence that this lost a real contradiction:

```text
$ readelf -V /usr/bin/getent
  Name: GLIBC_2.36  Flags: none
  Name: GLIBC_ABI_DT_RELR  Flags: none
  Name: GLIBC_2.38  Flags: none
  Name: GLIBC_PRIVATE  Flags: none
  ...

floor = glibc 2.17
before the correction: Unverified(UnrecognizedVersionRequirement)  -> admissible -> the target PASSES
after  the correction: NotSatisfied                                -> a hard failure
```

`getent` requires `GLIBC_2.38`, far newer than the declared 2.17 floor. The reader saw `GLIBC_ABI_DT_RELR` before `GLIBC_2.38`, abandoned the scan, and reported the floor as unverifiable.

The synthetic pair isolates the mechanism: an image requiring `GLIBC_2.34` alone against a 2.17 floor is `NotSatisfied`; adding `GLIBC_PRIVATE` to the same image flipped it to `Unverified`.

### Production implementation evidence

`crates/eggpack-core/src/floor.rs`:

- `parse_glibc_version` now classifies a name into three cases — not a glibc name, a numeric glibc version, or a `GLIBC_`-prefixed name that is not a plain numeric version — and returns that classification instead of an error. It no longer can abort a scan.
- `read_glibc_requirement` visits **every** requirement and returns both the highest numeric requirement observed and whether an unrecognized name was observed.
- The status decision order is: a numeric requirement that exceeds the declared floor → `NotSatisfied`, decided **first** and not downgraded by an unrecognized name observed alongside it; otherwise an unrecognized name → `Unverified(UnrecognizedVersionRequirement)`; otherwise no requirement → `Unverified(NoVersionRequirement)`; otherwise → `Verified`.
- The `FloorReadError::UnrecognizedVersion` variant was removed: the condition it represented is now a recorded classification, not an error.

### Behaviour preserved

- An image whose numeric requirements fit the declared floor but that also carries an unrecognized name is `Unverified(UnrecognizedVersionRequirement)`, **not** `Verified`. The floor is not proven, and the milestone never claimed it was.
- An image carrying only an unrecognized name is `Unverified`, never `Verified`.
- The status enum, the reason enum, `planned_floor_status`, `floor_status_admissible`, `aggregate_floor_status`, and the ranking that drives them are unchanged. `NotSatisfied` still ranks above every `Unverified` reason, so a contradiction still dominates in the aggregate.

## Defect B — the virtual-address translation could overflow

`map_vaddr` translated a virtual address back into a file offset with an unchecked addition whose operand, `segment.offset`, is read straight from the ELF program header and is never bounded against the file length. Every other offset computation in the module was overflow-checked; this one was not.

A crafted `PT_LOAD` with `p_offset` near the top of the `u64` range overflowed the addition. The overflow **panicked** in a dev or test build and wrapped in a release build — so `cargo test` could crash on a crafted candidate, and a release build could translate a virtual address into a wrapped file offset.

### Production implementation evidence

`crates/eggpack-core/src/floor.rs` `map_vaddr` now uses an overflow-checked addition and reports `FloorReadError::Malformed` — which the status decision maps to `Unverified(MalformedCandidate)` — rather than panicking or wrapping.

## Test evidence

`crates/eggpack-core/src/floor.rs` tests:

- `an_unrecognized_version_name_does_not_mask_a_contradicted_floor` — for both `GLIBC_PRIVATE` and `GLIBC_ABI_DT_RELR`, an image carrying a numeric requirement above the declared floor is `NotSatisfied`; the same image against a floor its numeric requirements satisfy is `Unverified(UnrecognizedVersionRequirement)`; an image with only the unrecognized name is `Unverified`.
- `an_unrecognized_version_name_does_not_hide_a_numeric_requirement` — a numeric requirement that appears **after** an unrecognized one is still observed by the reader.
- `a_virtual_address_translation_cannot_overflow` — a crafted `PT_LOAD` reports `Err(FloorReadError::Malformed)` instead of panicking.
- The existing floor tests — the highest-required-version reader, the 32-bit and big-endian readers, the refusal of truncated/malformed/non-ELF inputs, the oversized-table refusals, the comparison direction, the plan-time status derivation, and the admissibility matrix — continue to pass unchanged.

## Verification

```text
$ cargo test -p eggpack-core --all-targets --all-features --locked
test result: ok. 85 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out

$ cargo clippy -p eggpack-core --all-targets --all-features --locked -- -D warnings
(clean)
```

Additionally, the correction was checked against real system binaries on the review machine — the evidence the synthetic tests cannot give:

```text
before: /usr/bin/getent -> Unverified(UnrecognizedVersionRequirement)
        /usr/bin/iconv  -> Unverified(UnrecognizedVersionRequirement)
        /usr/bin/gencat -> Unverified(UnrecognizedVersionRequirement)
after:  /usr/bin/getent -> NotSatisfied
        /usr/bin/iconv  -> NotSatisfied
        /usr/bin/gencat -> NotSatisfied
        /bin/ls        -> NotSatisfied   (control: unchanged)
        /usr/bin/grep   -> NotSatisfied   (control: unchanged)
        /usr/bin/env    -> NotSatisfied   (control: unchanged)
```

## Invariants re-checked

- A floor declared and contradicted by the bytes is a hard failure and is never reported as `Unverified`. This holds regardless of any other version name the image carries.
- Every requirement in the image is visited; an unrecognized name never discards the requirements observed alongside it.
- The set of floor statuses, their meanings, and their ranking is unchanged. `NotSatisfied` remains a failure of the qualification, not a skip.
- No arithmetic on candidate bytes panics. A malformed candidate is a recorded status, never a crash.
- The reader remains bounded: every table cap, every length check before a read, and the exact-length reads are untouched.
- The inspection remains read-only and lock-free, and a native run is still not used as floor proof (ADR-0005 §9).
- No acceptance set moved; `ReleaseManifest` v1 and the published `eggpack-manifest` crate are untouched.

## Historical record

M008's own closure record (`plans/closure/build-qualification/008-status.md`) remains as written. The two defects it did not cover are recorded here and in the M008a plan; no history was rewritten.
