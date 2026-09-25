# Build and Qualification Milestone 004 — Finalization and Local Aggregation

Status: active

Repository baseline: `25a6f185f865510be0ab26a51a819d949701674c`

Source roadmap: `plans/subsystems/build-qualification-roadmap.md`

Dependencies: Build/Qualification M003 closure; Release Manifest M002 closure (`plans/closure/release-manifest/002-status.md`, satisfied).

Applicable ADRs: `plans/adrs/ADR-0001-producer-consumer-release-boundary.md`, `plans/adrs/ADR-0002-contract-plan-manifest-separation.md`, `plans/adrs/ADR-0004-first-party-native-cargo-build-adapter.md`.

Primary class: capability / finalization / aggregation

## 1. Objective

Convert qualified candidate bytes into contract-named local release artifacts, create required integrity sidecars, assemble declared direct/bundle/archive outputs, compute final byte evidence only after transformations, and aggregate one release identity into a Manifest v1. Reject incomplete, mixed-source, mixed-release, colliding, or unqualified required inputs.

## 2. Readiness and dependencies

Release Manifest M002 is closed and exposes explicit final-file manifest construction. Build M003 is closed at `plans/closure/build-qualification/003-status.md` and provides identity-bound qualification evidence usable by finalization gates.

## 3. Current evidence

`BuildAttempt` provides candidate paths and target/logical selectors. `build_manifest` accepts caller-named finalized files and computes exact sizes/digests but does not finalize, assemble, generate sidecars, or gate on qualification. Contract M001/M002 defines release filenames, archive member paths, and sidecar names.

## 4. Invariants

- Finalization uses only explicit validated candidate/evidence inputs and contract-derived names.
- Required targets must have passing applicable qualification evidence; non-gating/experimental behavior follows explicit policy and remains visible.
- No cross-release or cross-source aggregation.
- Byte-changing operations precede final size/hash and manifest construction.
- Sidecars are integrity only, generated from final bytes, and never imply authenticity.
- Artifact and install-name collision rules remain those of Contract/Manifest v1.
- No directory scan, network access, publication, extraction, installer, or consumer installation authority.
- Archive encoding is explicit producer-side input; never inferred from executable names or hidden defaults.

## 5. Scope

### In scope

- typed local finalization input joining ReleasePlan, BuildAttempt, and M003 evidence;
- direct/bundle file copy into an owner-private release root using contract filenames;
- contract checksum sidecar generation from final artifact bytes;
- explicit supported archive format selection and safe archive assembly for contract member paths;
- final inventory validation and Manifest M002 invocation;
- source/release identity and support-tier aggregation gates;
- all artifact layouts and mixed/partial/collision failure tests;
- docs, CI, and closure.

### Out of scope

- live install/extraction or Eggup receipt/state;
- remote build farm, downloads, publication, signing/notarization;
- arbitrary packaging plugins or generic archive format autodetection;
- changing DistributionContract or Manifest v1 unless a required public semantic is missing, in which case stop for ADR/plan revision.

## 6. Required production changes

Add an explicit finalizer/aggregator API in `eggpack-core`. It must bind candidates and qualification records by canonical target and logical selector, validate release/source identity, derive final names through DistributionContract, write only under a caller-owned private root, and fail without returning partial aggregate success. Generate sidecars only after artifact bytes are finalized. Archive assembly must use a narrowly enumerated encoding with traversal-safe contract member paths and must not overwrite unrelated files. Feed explicit resulting files/member inputs to Manifest M002.

If no repository-owned contract identifies archive encoding and a consumer needs that fact, stop before introducing an implicit format or public schema change. Propose the minimum contract/ADR correction and replan.

## 7. Ordered work packages

1. Revalidate M003 interface and define identity-bound finalization inputs.
2. Implement direct and bundle naming/copying, collision checks, and sidecar generation.
3. Implement explicit archive encoding/assembly only if supported by current contract semantics; otherwise record the concrete blocker and stop for plan/ADR revision.
4. Validate exact inventories and qualification/support-tier gates; call Manifest M002 on final bytes.
5. Add mixed identity, missing/extra, collision, tampering, archive relationship, sidecar, and repeatability tests.
6. Document finalization order, API limits, and public authority boundary.
7. Run local/package/MSRV/hosted matrix and close with M003/CI/bootstrap dependency dispositions.

## 8. Failure, restart, and contention semantics

Use a unique owner-private invocation root and refuse preexisting destinations. A failed operation returns no success manifest and does not reuse stale outputs. Cleanup may remove only paths proven to be owned by this invocation. Concurrent invocations use distinct roots; no shared mutable staging state.

## 9. Compatibility and migration

Producer-core additions only. Contract v1 and Manifest v1 remain stable. Any need to store archive encoding or new qualification claim in a public schema is a stop condition requiring explicit compatibility design.

## 10. Required tests

- direct and bundle exact naming and checksum sidecar generation;
- archive exact member paths/bytes using explicitly chosen encoding;
- hash/size describe final artifact bytes;
- qualification failure/pending required target rejects aggregation;
- source/release/target/selector mismatch rejects aggregation;
- missing/extra/case collision/sidecar collision rejection;
- repeat invocations and concurrent roots do not consume stale files;
- failure leaves no successful manifest and cleanup respects ownership;
- deterministic manifest output for reordered input.

## 11. Verification commands

Use the full commands listed in M003, plus `cargo package -p eggpack-core --locked` with workspace path patches as required by the unpublished workspace crates. Hosted CI must pass Linux stable, Linux Rust 1.89, macOS, and Windows.

## 12. Documentation updates

Update core README/rustdoc, build roadmap, registry, and `plans/closure/build-qualification/004-status.md`; record artifact format support and any remaining consumer-side interpretation gap.

## 13. Acceptance criteria

- One identity-consistent local release can be finalized for each supported contract layout.
- Final manifest evidence corresponds to final bytes and all contract-required files.
- Required qualification gates cannot be bypassed by build success.
- Mixed release/source and partial/colliding output are rejected.
- No public release or consumer-install side effects occur.
- Local and hosted verification passes and M004 closure distinguishes implementation evidence from hosted evidence.

## 14. Stop conditions

Stop for any necessary public schema change, ambiguous archive format ownership, unbounded hooks/processes, partial success semantics, unresolved medium-or-higher integrity/path-safety defect, or failure of required hosted qualification.

## 15. Closure evidence

Record implementation SHA, exact tests, hosted links, archive format coverage, unresolved severity findings, and downstream transitions in `plans/closure/build-qualification/004-status.md`.

## 16. Handoff

After closure, reassess CI Orchestration M002, Bootstrap M002, and any real-consumer work against their explicit dependency graphs. Do not mark another repository migrated without its own reviewed commit/evidence.
