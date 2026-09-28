# ADR-0005: Native Qualification for Cross-Tool Builds

Status: proposed

Date: 2026-09-28

Decision owners: project maintainers

Related specification sections:

- plans/000-long-term-specification.md#9-build-and-target-model
- plans/000-long-term-specification.md#8-release-planning

Related decisions:

- ADR-0004 (first-party native Cargo/cargo-zigbuild adapter boundary);
- ADR-0003 (checked-in generated CI and explicit publication gate).

Affected subsystem roadmaps:

- build and qualification;
- CI/release orchestration;
- ecosystem adoption.

## 1. Status note

This record is **proposed**, not accepted. It documents an unresolved
producer-contract question that currently blocks Ecosystem M001
(`plans/implementation/ecosystem-adoption/001-eggsact-direct-release-adoption-and-live-draft-qualification.md`).

No decision in this record has been implemented. No producer behavior has
changed. The finding is recorded so the maintainer can choose an option; the
companion corrective plan
`plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md`
stays unstarted and unapproved until this ADR is accepted.

## 2. Context

`BuildStrategy` selects **how** a candidate is produced:

- `NativeCargo` — `cargo build --target <triple>`;
- `CargoZigbuild` — `cargo zigbuild --target <triple>[.floor]`, for an
  explicit compatibility floor such as glibc 2.17.

`Qualification` declares **how** the resulting candidate is proved. Today
Eggpack couples those two independent axes: a `Qualification::Native` policy
is only admissible when the build strategy is also `NativeCargo`. That rule is
enforced at two declaration validators:

- `crates/eggpack-core/src/lib.rs` — `validate_policy`, reached through
  `PackConfig::resolve`:

  ```rust
  if policy.qualification == Qualification::Native {
      let host = /* explicit qualification_host, else build host */;
      if policy.strategy == BuildStrategy::CargoZigbuild || !host_matches_target(host, triple) {
          return Err(err("native qualification requires a matching native build/qualification host"));
      }
  }
  ```

- `crates/eggpack-ci/src/lib.rs` — `CIPlan::validate`, which additionally
  requires `policy.strategy == BuildStrategy::NativeCargo` before checking
  that the qualification host matches the target triple.

The **execution** path does not make that coupling.
`eggpack_core::qualification` decides its method and whether to run the
candidate smoke purely from `Qualification` and the observed host, and
consults `policy.strategy` nowhere in the `Qualification::Native` branch. It
already requires a matching host (`host == required &&
target_matches_host(&target.target, host)`) before executing, and requires a
smoke binding for every executing classification.

The consequence is that Eggpack can express "cross-build for a deployment
floor" only when the resulting candidate is **never executed**. It cannot
express "cross-build for a deployment floor, then execute natively on a
matching host".

## 3. Why this blocks first-consumer adoption

The first planned consumer, eggsact, publishes five direct binary targets. Its
current hand-written release workflow already does the thing Eggpack forbids
it from declaring, on the two Linux targets:

| Target | Strategy | Runner | Build reason | Current qualification |
|---|---|---|---|---|
| `x86_64-unknown-linux-gnu` | CargoZigbuild | `ubuntu-latest` (native x86-64) | glibc 2.17 floor | executes `--version`, `--help`, MCP handshake |
| `aarch64-unknown-linux-gnu` | CargoZigbuild | `ubuntu-24.04-arm` (native aarch64) | glibc 2.17 floor | executes `--version`, `--help`, MCP handshake |

The build is cross-*floored*, not cross-*hosted*: the candidate runs on a host
that natively matches the target OS and architecture. The three macOS and
Windows targets are `NativeCargo` with `Qualification::Native` and are
unaffected.

Without a change, there is no admissible configuration for the two Linux
targets that preserves the current evidence. The available workarounds each
lose something the plan requires:

- `Qualification::Structural` — `qualify_target` sets `should_execute = false`
  and emits no smoke evidence, so the bounded CLI-level candidate smoke is
  dropped. Ecosystem M001 §7 requires it and the ecosystem roadmap invariant
  "migration cannot weaken current release coverage/qualification" forbids it.
- `Qualification::DeferredNative` with `qualification_host` set to the matching
  native host — this does execute the smoke and record
  `DeferredNativeOnNativeHost`/`Passed`, so functional evidence is preserved,
  but the producer policy would declare "qualification is deferred" for a
  same-job native run. The evidence record stays accurate; the declared intent
  does not describe what happens.

This is the exact condition named in Ecosystem M001 §20: *"generated workflow
cannot express all five current targets/runners"*, which directs a stop and
re-plan rather than a mechanical application of the plan.

The gap is not eggsact-specific. Any producer that needs a compatibility floor
on a same-architecture host — the common glibc-floor-on-modern-runner case —
hits it.

## 4. Considered options

### Option A — decouple qualification from build strategy (recommended)

Permit `Qualification::Native` with any build strategy, as long as the
effective qualification host matches the target triple's OS and architecture.
Concretely, drop the `strategy == NativeCargo` term from both declaration
validators and keep every host-matching and smoke requirement.

Strengths:

- restores the true independence of the two axes;
- preserves all current eggsact release evidence exactly, including the core
  CLI-level smoke on both Linux targets;
- the execution path needs no behavior change, so the blast radius is two
  validation predicates plus tests;
- the existing `host_matches_target` / `target_matches_host` guards and the
  mandatory smoke binding already prevent a non-matching host from being
  declared or executed.

Costs and risks:

- it is a deliberate change to `PackConfig` admissibility, so it is a producer
  contract change and needs its own plan, tests, and closure;
- it widens what a producer may declare, so the guard must be proven by
  negative tests (mismatched host arch/OS still rejected, smoke still
  mandatory, structural-only still non-executing).

### Option B — keep the rule and label the Linux targets `DeferredNative`

No producer-code change. Declare `qualification_host` explicitly on both
Linux targets and accept that the recorded intent reads "deferred" for a
same-job native run.

Strengths: zero Eggpack change; all functional evidence retained.

Costs: the declarative intent misdescribes the run; the gap stays latent for
the next floor-on-matching-host consumer; a future reader must know that
`DeferredNative` here means "native, same job". It also leaves the two Linux
targets declaring a classification whose documented meaning is "inspected on a
different host and remains pending".

### Option C — keep the rule and label the Linux targets `Structural`

No producer-code change, but the core candidate smoke disappears for both
Linux targets and the release relies only on the gating consumer MCP
handshake.

Costs: this weakens current release evidence, contradicting Ecosystem M001
§7/§19 and the ecosystem adoption roadmap invariant. Not recommended.

### Option D — introduce a distinct classification, e.g. `NativeCrossBuilt`

Add a new `Qualification` variant meaning "built with cross tools, executed
natively on a matching host".

Strengths: most explicit and self-documenting.

Costs: a larger change than Option A. It grows the finite `Qualification`
enum, the `QualificationMethod` enum, and every exhaustive match over both,
plus a schema-version consideration for `PackConfig`/`CIPlan` documents
already checked in. Reserved for later if Option A proves too permissive in
practice.

## 5. Decision

**Undecided. Option A is recommended.**

This ADR is accepted only when the maintainer selects an option. Selecting
Option A authorizes
`plans/implementation/build-qualification/006-native-qualification-for-cross-tool-builds.md`
to start. Selecting Option B or C requires no producer code change and instead
records the declared-intent compromise in the Ecosystem M001 closure.

## 6. Consequences of Option A, if accepted

- `PackConfig` admits `Qualification::Native` with `CargoZigbuild` when the
  qualification host matches the target OS/arch. Documents that previously
  resolved continue to resolve identically; no existing document changes
  meaning, so no schema version bump is required.
- `qualify_target` behavior is unchanged. Evidence for such a target reports
  `QualificationMethod::Native` with executed process outcomes, which is
  already a supported and validated combination.
- Ecosystem M001 becomes implementable with all five targets at `native`
  qualification, and its §7 core CLI-level smoke applies uniformly.
- eggsact's Eggpack tool pin (`release/eggpack/github-policy.json`,
  `eggpack_tool.revision`) must be re-pointed at the corrective's
  implementation revision, since the current pin predates the change.
- The change is reusable, not eggsact-specific: it is a general producer
  capability, not a per-consumer special case, so it does not fork the schema
  around one repository.

## 7. Compatibility and migration

- No checked-in Eggpack document in this repository declares
  `Qualification::Native` together with `CargoZigbuild`; nothing in-tree
  changes meaning.
- `crates/eggpack-core/src/lib.rs` contains a test that asserts cross-tool
  versions are rejected on a `NativeCargo` target. That rule is unaffected and
  the test must remain green.
- `architecture/core.md` documents `BuildStrategy` but does not state the
  qualification coupling, so no architecture document contradicts Option A.
  If Option D is chosen, `architecture/core.md` and
  `crates/eggpack-core/README.md` require a matching update.

## 8. Security and reliability implications

Option A does not change what is executed, where, or with which credentials.
It relaxes a *declaration* validator, so the risk is declaring an
inappropriate host, not executing on one. The retained guards bound that
risk:

- the qualification host must match the target's OS and architecture, both at
  declaration (`host_matches_target`) and at execution
  (`target_matches_host`);
- every executing classification still requires a smoke binding, enforced by
  `QualificationBindingsV1::validate_for` and by `qualify_target`;
- a host that does not match produces a failed `HostMismatch` evidence record
  rather than a skipped or passing result;
- generated workflows remain read-only except for the single staging job, and
  qualification jobs run with `contents: read` only.

No new network, credential, archive, publication, or privilege surface is
introduced.

## 9. Verification required before acceptance is recorded as achieved

- negative test: `Qualification::Native` with `CargoZigbuild` and a
  non-matching qualification host still fails to resolve;
- negative test: `Qualification::Native` still requires a smoke binding;
- positive test: a five-target eggsact-shaped shape with two
  CargoZigbuild + native targets renders a reusable workflow and passes
  `eggpack ci check` with zero drift;
- positive test: `qualify_target` on a CargoZigbuild-built candidate at a
  matching native host yields `QualificationMethod::Native` with executed
  processes;
- existing `NativeCargo`/cross-tool rejection and glibc-floor rules stay green;
- full workspace `cargo fmt`, `cargo test --workspace --all-targets
  --all-features --locked`, and `./scripts/check-local.sh` green;
- hosted CI green on all lanes.

## 10. Supersession

Superseded only by a later accepted ADR. If Option A is rejected in favor of
Option D, this record is superseded by the Option D ADR and the accepted
options are not rewritten.
