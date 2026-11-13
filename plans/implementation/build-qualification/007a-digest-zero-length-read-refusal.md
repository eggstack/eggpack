# Build/Qualification M007a Corrective — Digest Zero-Length Read Refusal

Status: **closed**.

Subsystem: Build and qualification.

Predecessor: `plans/implementation/build-qualification/007-evidence-digest-unification-corrective.md` (M007, implementation `e3c6192`, closure `plans/closure/build-qualification/007-status.md`).

Triggered by: the 2026-10-23 post-implementation review of the M007 chain, which found that M007's new canonical digest helper did not refuse a read that produced zero bytes.

M007's closure record remains historical and is not rewritten. This corrective is a new `NNNa` plan, per the repository's closure discipline.

## 1. Objective

Close the one gap in M007's canonical digest helper: a read that produces zero bytes must be refused, so a zero-length digest record can never be produced.

M007 replaced two divergent in-crate digest helpers with one canonical helper (`crates/eggpack-core/src/digest.rs` `file_digest`) carrying the *stricter* of the two prior semantics. The consolidation is sound. The gap is in one edge of the refusal order.

## 2. Current evidence and the defect

M007's helper refuses an empty file **before** the read, by observing `symlink_metadata(path).len() == 0`. The pre-change helper it replaced in `crates/eggpack-core/src/lib.rs` (`digest_file`) refused an empty file **after** the read, by observing that the accumulated byte count was zero.

The two orders differ in exactly one window: between the pre-read metadata observation and the read itself.

- Pre-change (`digest_file`): the emptiness decision was made from the bytes actually read, so a file truncated in the window produced a zero byte count and was refused.
- Post-M007 (`file_digest`): the emptiness decision is made from the pre-read observation, so a file truncated in the window is read as zero bytes. The post-read re-check then compares the post-read length (`0`) against the bytes read (`0`) — they agree — and the helper returns `Ok((0, sha256-of-no-bytes))`.

That is a zero-length digest record, which the pre-change helper refused and which the milestone's own invariant forbids:

> *"An empty file MUST be refused before it is read, so a zero-length input can never produce a digest record."*
> — M007 §3

The sentence is true about the *pre-read* refusal but, as written, does not hold for the truncation window: the helper's doc comment claims a zero-length input can never produce a digest record, and the truncation window is a counterexample.

### 2.1 Impact

The observable outcome of the gap is still a failure, so this is not a fail-open:

- the manifest path ends in `manifest.validate()`, and `eggpack-manifest` refuses `size == 0`;
- the finalization path compares the digest size against the qualification evidence size;
- the qualification path compares the size against the candidate's metadata length;
- `eggpack-github`'s staging path already refused emptiness pre-read, so it never had the gap.

The defect is the **record itself**: a digest authority that can return a zero-length record violates its own stated invariant, and any future caller that does not repeat the downstream size check would admit it. The helper is the single authority; its contract must hold on its own.

## 3. Invariants

- A recorded digest MUST describe a non-empty regular file. A zero-length read result MUST be refused.
- The refusal MUST happen regardless of *when* the emptiness appears — before the read or during it.
- The hashed bytes MUST NOT change: same hash function, same encoding, same lowercase hex representation.
- The refusal MUST be typed and distinguishable: `DigestError::Empty`, the same variant the pre-read refusal already uses. No new variant, no sentinel value, no diagnostic that becomes vaguer than the pre-change one.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-core/src/digest.rs` only.

**Out of scope:**

- The SHA-256 hex *acceptance set* (the validator rule), which is ADR-0006 and is implemented by `eggpack-manifest`. This corrective changes no acceptance set.
- Any change to `ReleaseManifest` v1, `schema_version`, or the published `eggpack-manifest` crate.
- The TOCTOU post-read re-check, which is correct and stays.
- `crates/eggpack-ci/**` and `crates/eggpack-github/**`, which consume the helper and inherit the correction automatically.

## 5. Required production changes

1. In `file_digest`, after `hash_stream` returns, refuse a zero-length read result with `DigestError::Empty` before the post-read check runs.

2. Update the helper's doc comment so the refusal order states all five steps and the zero-length-read refusal is stated explicitly, with the truncation window named as the reason it is not redundant with the pre-read refusal.

## 6. Required tests

1. A test that asserts a zero-length read result is refused: a file that is non-empty at the pre-read observation and empty by the read is refused with `DigestError::Empty`.
2. The existing `empty_file_is_refused_before_it_is_read` test continues to pass.
3. The digest's value, hex representation, and length are unchanged for a normal file.

## 7. Verification

`cargo test -p eggpack-core --all-targets --all-features --locked` and `cargo clippy -p eggpack-core --all-targets --all-features --locked -- -D warnings`.

The workspace gate is not the acceptance criterion for this corrective; it runs once for the whole review line.

## 8. Closure

Closed at `plans/closure/build-qualification/007a-status.md`.
