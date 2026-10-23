# Build/Qualification Milestone 007a Closure — Digest Zero-Length Read Refusal

Status: closed

Source plan: `plans/implementation/build-qualification/007a-digest-zero-length-read-refusal.md`

Roadmap: `plans/subsystems/build-qualification-roadmap.md` (registered there)

Triggered by: the 2026-10-23 post-implementation review of the M007 chain.

Reviewed baseline: `e0e1510` (the tip of the review chain), on top of M007's implementation `e3c6192`.

## Executive finding

Build/Qualification 007a is closed. The canonical digest helper can no longer produce a zero-length digest record.

M007's consolidation of the crate's two divergent file-digest helpers into one canonical authority was sound, and the authority is genuinely single — `crates/eggpack-core/src/digest.rs` `file_digest` is the only file-digest implementation in the crate, and `eggpack-ci` and `eggpack-github` both consume it. The gap was one edge of the refusal order, and it was a gap the consolidation introduced rather than inherited.

## The defect, as observed

M007 moved the empty-file refusal from **after** the read to **before** it. The pre-change helper decided emptiness from the bytes actually read; the new helper decides it from a `symlink_metadata` observation taken before the read.

The two orders differ in exactly one window — between the pre-read observation and the read. A file truncated in that window is read as zero bytes, and the post-read re-check then compares the post-read length (`0`) against the bytes read (`0`): they agree, and the helper returned `Ok((0, sha256-of-no-bytes))`.

That is a zero-length digest record, which the pre-change helper refused and which the milestone's own §3 invariant forbids.

The observable outcome was still a failure, so this was not a fail-open: the manifest path ends in `manifest.validate()` and `eggpack-manifest` refuses `size == 0`; the finalization and qualification paths compare the size against other evidence; and `eggpack-github`'s staging path already refused emptiness pre-read. The defect was the **record**, not the release outcome: a single digest authority that can return a zero-length record breaks its own contract, and any future caller that does not repeat the downstream size check would admit it.

## Production implementation evidence

`crates/eggpack-core/src/digest.rs`:

- `file_digest` refuses a zero-length read result with `DigestError::Empty` immediately after `hash_stream` returns, before the post-read check runs.
- The doc comment now states the refusal order in five steps and names the truncation window as the reason the zero-length-read refusal is **not** redundant with the pre-read refusal.
- No new error variant was introduced: `DigestError::Empty` already existed for this refusal, and the diagnostic is unchanged.
- The hash function, the encoding, and the lowercase hex representation are untouched.

## Test evidence

`crates/eggpack-core/src/digest.rs` tests:

- `a_zero_length_read_is_refused_even_when_the_file_grew_empty_after_the_check` — a file that is non-empty at the pre-read observation and empty by the read is refused with `DigestError::Empty`, and the post-read step refuses the same path with `ChangedDuringRead`.
- `empty_file_is_refused_before_it_is_read` — unchanged and passing.
- `regular_file_digest_is_the_lowercase_hex_sha256_of_its_bytes` — the digest's value, hex representation, and 64-character length are unchanged.

## Verification

```text
$ cargo test -p eggpack-core --all-targets --all-features --locked
test result: ok. 85 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out

$ cargo clippy -p eggpack-core --all-targets --all-features --locked -- -D warnings
(clean)
```

## Invariants re-checked

- A recorded digest describes a non-empty regular file; a zero-length read result is refused regardless of when the emptiness appears.
- The hashed bytes are unchanged: same hash function, same encoding, same lowercase hex representation.
- The refusal is typed and distinguishable, and no diagnostic became vaguer than the pre-change one.
- The post-read time-of-check/time-of-use guard is untouched and still refuses a path that became a symlink, changed type, or changed length during the read.
- Determinism is preserved: identical bytes produce an identical digest and size record.
- No acceptance set moved. The all-zeros sentinel stays refused everywhere (ADR-0006), and `eggpack-manifest` and `ReleaseManifest` v1 are untouched.

## Note on scope

This corrective touched `crates/eggpack-core/src/digest.rs` only. The `eggpack-ci` and `eggpack-github` consumers inherit the correction automatically, because they call the canonical helper rather than re-implementing it — which is the outcome M007 was for.

## Historical record

M007's own closure record (`plans/closure/build-qualification/007-status.md`) remains as written. The gap it did not cover is recorded here and in the M007a plan; no history was rewritten.
