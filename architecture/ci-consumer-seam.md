# `eggpack-ci` — Consumer Seam, Handoffs, and Runtime Identity — Deep Dive

The half of `eggpack-ci` concerned with agreeing on artifacts and identities
rather than doing release work: the build/qualification handoff documents, the
external consumer validator and its bounded runner call, the Zig/cross-tool
provisioning contract, and the reusable-workflow runtime identity. It divides
from [ci.md](ci.md) (projection model, job vocabulary, gates, aggregation) and
[ci-rendering.md](ci-rendering.md) (policy types, renderers, drift checking).

The crate is a module tree, so this file names the module that holds each
construct: `handoff.rs` (handoff documents), `consumer_validator.rs` (external
validator), `identity.rs` (runtime identity), `policy.rs` (shared constants,
Zig provisioning, path rules), `release_plan.rs` (orchestration graph),
`planning.rs` (`CIPlan`), `validate.rs` (shared string rules), `drift.rs`
(drift checking), and `render_release.rs` (rendering).

> **Citation state.** The anchors below were re-based by symbol name in the
> 2026-10-23 post-implementation review pass and read against the tree at the
> SHA pinned in [overview.md](overview.md) §Baseline. `crates/eggpack-ci/src/lib.rs`
> is now the 59-line crate root; the only anchor in this file that points into it
> is the `mod tests;` declaration at `:18-19`. Every other anchor names a module
> in the crate's module tree.

## Responsibility

| Job | Module | Problem it solves |
|---|---|---|
| Build/qualification handoff | `handoff.rs` | The generated workflow does not call library functions; it shells out to the `eggpack` binary. Two independently versioned processes must agree on artifact layout and document shape. |
| External consumer validator | `consumer_validator.rs` | A third party needs to assert something about a release without eggpack claiming it. The seam runs caller-supplied code and returns bounded evidence. |
| Runtime identity | `identity.rs` | A checked-in reusable workflow cannot hardcode a release id, source revision, or tag, or it would be regenerated per release. The shape is identity-free; the plan is resolved per invocation. |

All three are *agreement* mechanisms, not security boundaries against a hostile
caller: the caller supplies the contracts, and validation is fail-closed against
malformed or out-of-bounds documents, not against a determined adversary.

## The build/qualification handoff formats

`BUILD_HANDOFF_FILE` = `build-handoff.json`
(`crates/eggpack-ci/src/policy.rs:542`), `QUALIFICATION_EVIDENCE_FILE` =
`evidence.json` (`policy.rs:545`), `CANDIDATES_DIR` = `candidates`
(`policy.rs:548`), `GATE_OUTCOME_FILE` = `gate-outcome.json` (`policy.rs:551`,
written by the gate job at `crates/eggpack-ci/src/render_release.rs:1047`). These
four strings are the entire contract between the rendered workflow and the
`eggpack` binary. A change to a name, a JSON shape, or a validation rule here
survives compilation and fails at CI runtime: nothing in the type system ties a
producer write to a consumer read.

| Document | Type | Producer | Consumer | Validated on read |
|---|---|---|---|---|
| build handoff | `BuildHandoffV1` (`handoff.rs:43`) | `project_build_handoff` (`handoff.rs:125`), written by CLI `_capture-build` | `validate_build_artifact_dir` (`handoff.rs:243`), `reconstruct_attempt` (`handoff.rs:186`) | `from_json` (`handoff.rs:66`) + `validate` (`handoff.rs:77`), on every read |
| candidate bytes | files under `candidates/` | `stage_build_artifact_dir` (`handoff.rs:333`) / CLI staging | same | size, regular non-symlink file, exact inventory, no extras (`handoff.rs:253-286`) |
| qualification evidence | `QualificationEvidence` (core) | qualification job | `validate_qualification_artifact_dir` (`handoff.rs:292`) | identity must equal the handoff (`handoff.rs:299-304`) |
| consumer evidence | `ConsumerValidationEvidenceV1` (`consumer_validator.rs:138`) | `run_consumer_validator` (`consumer_validator.rs:323`) | `decode_consumer_evidence` (`release_plan.rs:406`), `evaluate_gate_with_consumer` (`release_plan.rs:420`) | `from_json` (`consumer_validator.rs:172`) + `validate` (`consumer_validator.rs:183`); gate re-validates at `release_plan.rs:426-428` |
| workflow shape | `ReleaseWorkflowShapeV1` (`identity.rs:65`) | checked in | `render_reusable_release_github` (`render_release.rs:276`), `check_reusable_release_github` (`drift.rs:88`) | `from_json` (`identity.rs:86`) + `validate` (`identity.rs:104`) |

`BuildHandoffOutput` (`handoff.rs:25`) carries the contract logical selector, an
explicit Cargo package and binary, a relative path below the handoff root, an
exact size, and a deterministic `handoff_identity`; `BuildHandoffV1`
(`handoff.rs:43`) adds `schema_version`, `release_id`, `source_revision`, the
canonical target, the build `strategy`, and canonically ordered outputs, all
under `deny_unknown_fields`. `validate` (`handoff.rs:77`) enforces
`schema_version == 1`; `safe_metadata(.., 256)` identities; `safe_target`
(`crates/eggpack-ci/src/validate.rs:5`); 1..=256 outputs in strictly ascending
`LogicalOutputSelector` order with no duplicates; unique non-empty
`handoff_identity` bounded to 256 bytes; `safe_identifier` (`validate.rs:12`)
package and binary; non-zero sizes; and `validate_relative_path`
(`handoff.rs:109`) per path, rejecting absolute roots, drive colons, NUL, `..`,
empty segments, and >512 bytes. `from_json` bounds the document at
`MAX_HANDOFF_JSON` (`handoff.rs:20`, 1 MiB) before parsing
(`handoff.rs:67-68`); `to_json` validates before serializing.

`project_build_handoff` (`handoff.rs:125`) is pure — no I/O. It derives a
deterministic `relative_path` per selector (`candidate-direct`,
`candidate-bundle-{index}`, `candidate-archive-{sha256(source)}`,
`handoff.rs:141-153`) and sorts outputs canonically (`handoff.rs:168`), setting
`size: 1` as a non-zero placeholder (`handoff.rs:164`) with an explicit comment
that callers must not treat it as evidence (`handoff.rs:155-158`). The runner
replaces it with the observed size and re-validates — the CLI does exactly that
at `crates/eggpack-cli/src/main.rs:1077`, then re-validates at
`main.rs:1002-1004` and `validate_build_artifact_dir` (`main.rs:1033`).

### `reconstruct_attempt`

`reconstruct_attempt` (`handoff.rs:186`) turns a validated handoff plus
downloaded files back into an in-memory `eggpack_core::BuildAttempt`, requiring
the handoff identity to equal the plan's release id, source revision, target, and
strategy (`handoff.rs:192-199`) and each output to be a non-symlink, regular,
non-empty file. What it deliberately does **not** re-verify:

- It does not hash the candidate. No SHA-256 appears in the handoff at all; the
  reconstructed `CandidateArtifact` carries the observed size only. Byte identity
  is established later, by the qualification job and by the consumer validator's
  expected digest.
- It accepts a handoff whose `size` is still the placeholder `1` without comparing
  it to the observed length (`handoff.rs:210-214`), while
  `validate_build_artifact_dir` (`handoff.rs:261`) has no such exception and
  requires exact size equality. **The two readers of the same document disagree
  about size enforcement.**
- It synthesizes process evidence rather than observing any: `tool_summary` is
  `format!("ci-handoff {}", target.target)` (`handoff.rs:230`) and `ProcessEvidence`
  is a literal `CommandOutcome::Success` with `stdout_bytes: 0, stderr_bytes: 0`
  (`handoff.rs:231-235`). A reconstructed attempt therefore always claims a
  successful process. The doc (`handoff.rs:181-185`) says the result must still
  pass M003/M004 validation; these fabricated fields are not evidence.

### Directory validators and path derivation

`validate_build_artifact_dir` (`handoff.rs:243`) requires `candidates/` to be a
real directory (not a symlink), every handoff output present as a non-symlink
regular file with exact size, and — importantly — **no extra entries**: any
non-file entry or unlisted name is rejected, and the final count must equal the
inventory (`handoff.rs:267-286`). `validate_qualification_artifact_dir`
(`handoff.rs:292`) runs that first, then decodes `evidence.json` and requires the
evidence identity to match the handoff's release id, source revision, and target
(`handoff.rs:299-304`).

`stage_build_artifact_dir` (`handoff.rs:333`) creates or reuses a real output
directory, hard-links each candidate (falling back to copy,
`handoff.rs:309-331`), writes the handoff, then **re-validates the directory it
just wrote** (`handoff.rs:358`), so a size mismatch fails here. Two caveats: the
handoff write is a plain `std::fs::write` (`handoff.rs:356`), not an atomic
replace, unlike the CLI's `atomic_write`
(`crates/eggpack-cli/src/main.rs:1108`); and this function has no caller outside
this crate's own tests — the CLI stages inline at `main.rs:1020-1034` using the
literals `"candidates"` and `"build-handoff.json"` rather than the constants.
`cargo_output_path` (`handoff.rs:364`) derives
`<cargo_target_dir>/<target>/release/<binary>` after
`validate_canonical_target_dir` (`crates/eggpack-ci/src/policy.rs:555`),
appending `.exe` for targets whose triple contains the `-windows-` component
(`handoff.rs:380-384`). The suffix is appended to the full binary name, so a
dotted name resolves to `tool.cli.exe`; this agrees with
`discover_candidate` in `eggpack-core` and with Cargo's own naming.

## The external consumer validator

The one place the producer runs foreign, caller-supplied code.

`ConsumerValidatorV1` (`consumer_validator.rs:43`) can specify `schema_version`,
one `LogicalOutputSelector`, an interpreter, a repository-relative `script` path, a
timeout, and stdout/stderr limits. There is deliberately no arbitrary executable
field, no shell string, no environment map, and no caller-supplied argument vector
(`consumer_validator.rs:34-40`). `ValidatorInterpreterV1`
(`consumer_validator.rs:29`) has exactly one variant, `Python3`, and host mapping
is finite — `python3` on Linux/macOS, `python` on Windows — via
`python_interpreter_exe` (`consumer_validator.rs:219`) delegating to the
`cfg!`-testable `python_interpreter_exe_for_windows`
(`consumer_validator.rs:224`). `validate` (`consumer_validator.rs:79`) enforces
`schema_version == 1`; interpreter is `Python3`; `validate_release_input_path`
(`crates/eggpack-ci/src/policy.rs:525`) on the script (bounded relative, no
absolute root, no drive colon, no backslash, no `.`/`..`/empty segments,
per-segment ≤128, total ≤512); `timeout_ms` in 1 000..=600 000; both output limits
in 1..=8 000 000. `from_json` (`consumer_validator.rs:62`) bounds the document at
`MAX_VALIDATOR_JSON` (`consumer_validator.rs:14`, 64 KiB). Separately,
`project_release_plan_with_consumer` (`crates/eggpack-ci/src/release_plan.rs:356`)
requires ≤256 validators (`release_plan.rs:363-365`), each target to exist in the
`CIPlan` (`release_plan.rs:368-374`), and each selector to be an actual build
output of that target's bindings (`release_plan.rs:375-386`).

`ConsumerValidationRequest` (`consumer_validator.rs:233`) supplies the validator
plus absolute `script_path`, `candidate_path`, `work_dir`, the `expected_size` and
`expected_sha256` to verify against, release identity strings, an optional
`AtomicBool` cancellation flag, and an optional exact PATH override
(`consumer_validator.rs:254-258`). The CLI populates it at
`crates/eggpack-cli/src/main.rs:1354-1366` with the size and digest taken from the
**qualification evidence**, not from the handoff (`main.rs:1276-1277`), and
refuses outright to run if that evidence is not `Passed` (`main.rs:1235-1240`).

Candidate identity is observed *before* any execution
(`consumer_validator.rs:369`) so evidence always carries validated linkage. A
candidate that could not be read is recorded as **absent** — size `0`, digest
`None` (`consumer_validator.rs:378-381`) — never as a synthetic digest.
`run_consumer_validator` (`consumer_validator.rs:323`) is then fail-closed
throughout. Caller *misuse* returns `Err(CiError)`; validator *outcomes* return
bounded evidence.

| Step | Citation | Result on failure |
|---|---|---|
| re-validate config; script, candidate, work dir absolute; work dir a real directory; `expected_sha256` 64 lowercase hex, not all-zero; identity strings in bounds | `consumer_validator.rs:326-350` | `Err` |
| candidate hashed, size and digest compared to expected | `consumer_validator.rs:369-389` | `Failed(CandidateMismatch)` |
| script is a regular non-symlink file of 1 B..=1 MiB | `consumer_validator.rs:356-363`, `:390-397` | `Failed(ScriptUnavailable)` |
| restore POSIX exec bits on the transferred candidate | `consumer_validator.rs:400-407`, `:451` | `Failed(CandidateMismatch)` |
| bounded `python --version` preflight must contain "Python 3" | `consumer_validator.rs:488-507` | `Failed(InterpreterUnavailable)` |
| fixed `python3 <script> <candidate>` invocation | `consumer_validator.rs:526-534`, `:521` | `Passed` or one of six failures |

`ensure_candidate_executable` (`consumer_validator.rs:451`) sets mode `| 0o111` on
the candidate after its bytes were verified. The doc
(`consumer_validator.rs:446-450`) argues this cannot change identity because size
and digest cover bytes only. That is correct, but it makes this the one place the
CI layer mutates a candidate file in place, between verification and execution,
with no re-check afterwards.

`ConsumerValidationFailure` (`consumer_validator.rs:104`) classifies seven bounded
causes: `NonZeroExit`, `Timeout`, `OutputLimit`, `InterpreterUnavailable`,
`CandidateMismatch`, `ScriptUnavailable`, `Cancelled`. `ConsumerValidationOutcome`
(`consumer_validator.rs:124`) is `Passed` or `Failed(..)`.
`ConsumerValidationEvidenceV1` (`consumer_validator.rs:138`) records schema
version, release/source/target identity, selector, interpreter, outcome, observed
size, and observed digest — nothing else; a test asserts the encoded JSON contains
no `stdout`, `stderr`, or `output` key
(`crates/eggpack-ci/src/tests.rs:3236-3238`). A
candidate that was never read is recorded as absent (`size` 0, digest `null`) and
validation rejects either half without the other: this document is evidence of
what the producer saw, so no digest is ever synthesized for a file that was not
hashed.

The validator is **caller-supplied**, so it is not a security boundary against a
hostile caller: a caller who supplies `script` supplies arbitrary code that eggpack
will execute with the candidate's path as an argument. What the design buys is
narrower and still real. The *shape* of the invocation is fixed and finite — one
interpreter, one script path, one candidate path, no shell, no argument vector, no
env map (`consumer_validator.rs:526-534`). The *environment* is built by the
shared runner, which clears it and re-adds a five-variable allowlist
(`crates/eggpack-core/src/builder.rs:735`, `:724`); the CI side contributes only
the caller's explicit `PATH` (`consumer_validator.rs:299-300`), stdin is null
(`builder.rs:736`), and the working directory is explicit
(`consumer_validator.rs:304`). The *outcome is bounded*: timeout, output limits, a
seven-value classification; a validator cannot hang or flood the job. This is a
**separation-of-concerns** mechanism: `Passed` means an independent script exited
zero against the exact candidate. It is not an authenticity claim, and eggpack
does not assert it on the validator's behalf — M003 remains the provider-neutral
binary qualification authority (`consumer_validator.rs:36-38`).

## The validator's process execution

**No independent runner.** `eggpack-ci` declares no `command-group` dependency
(`crates/eggpack-ci/Cargo.toml:14-19` lists only `eggpack-contract`,
`eggpack-core`, `serde`, `serde_json`, `sha2`), and it does not spawn a process
itself. `run_validator_process` (`consumer_validator.rs:521`) and
`run_interpreter_preflight` (`consumer_validator.rs:488`) build an
`InterpreterCommandSpec` (`consumer_validator.rs:293`) and hand it to the shared
bounded runner in `eggpack-core`:

- `run_validator_process` calls `eggpack_core::run_interpreter_bounded`
  (`consumer_validator.rs:535`), which is the workspace's single bounded
  process-group runner (`crates/eggpack-core/src/builder.rs:485`).
- `run_interpreter_preflight` calls the same entry point
  (`consumer_validator.rs:504`), with the timeout capped at 30 s
  (`consumer_validator.rs:499`) and the output bound capped at 8 KiB
  (`consumer_validator.rs:500-501`).

Everything the old private runner did is therefore core's contract, not the CI
crate's: process-group/job termination on every failure path, a cleared
environment with a five-variable allowlist (`builder.rs:735`, `:724`), null
stdin (`builder.rs:736`), the exact output truncation in `drain`
(`builder.rs:942-958`, which keeps `n.min(limit - len)`), and the bounded reader
joins. The runner outcome maps onto the consumer-validation outcome exactly
(`consumer_validator.rs:536-555`): success → `Passed`, non-zero exit →
`Failed(NonZeroExit)`, deadline → `Failed(Timeout)`, output bound →
`Failed(OutputLimit)`, cancellation → `Failed(Cancelled)`, and an interpreter that
cannot be launched → `Failed(InterpreterUnavailable)`.

This is the state after CI M003m. The earlier reimplementation — its own
`std::process::Command` spawns, bare `Child::kill()`, an over-limit
`read_limited` that appended the whole chunk before the limit check, and a
preflight that discarded its over-limit flags — is gone; see the closure record
`plans/closure/ci-release-orchestration/003m-status.md` and
[process-execution.md](process-execution.md), which now records the CI crate as
having no process-spawning code at all.

## Zig and cross-tool provisioning

`CrossToolProvisioningV1` (`crates/eggpack-ci/src/policy.rs:65`) is a provider
*how* policy: it bounds `cargo install --locked` for cargo-zigbuild (1..=60
minutes) and curl `--connect-timeout` (5..=300 s) / `--max-time` (60..=3600 s) for
the Zig download (`policy.rs:98-107`). Which *versions* are required comes from
`ToolchainRequirement` in the resolved `TargetPolicy`; the split is documented at
`policy.rs:66-74`. No generic package manager, arbitrary URL, or setup command is
representable. `ZigOfficialArchiveV1` (`policy.rs:49`) holds one digest per
supported Linux host architecture; `validate` (`policy.rs:78`) requires exactly 64
lowercase hex characters and rejects the all-zero digest as a sentinel
(`policy.rs:161-168`); `digest_for` (`policy.rs:87`) maps `X86_64`/`Aarch64` and
fails for `Armv7`.

The version-to-archive mapping is pure and finite. `zig_archive_name`
(`policy.rs:117`) emits `zig-x86_64-linux-{version}.tar.xz` or
`zig-aarch64-linux-{version}.tar.xz`, failing for non-Linux hosts, `Armv7`, or a
version that fails `safe_tool_version` (`crates/eggpack-ci/src/validate.rs:23`,
≤64 chars from `[A-Za-z0-9.+-_]`). `zig_download_url` (`policy.rs:133`) is fixed to
`https://ziglang.org/download/{version}/{name}` — no caller URL, mirror, or
redirect target is representable. `zig_expected_digest` (`policy.rs:146`) requires
a Linux host and returns the policy's pinned digest.

`validate_zig_archive_listing` (`policy.rs:179`) is a network-free layout check
over a file listing: non-empty, ≤4096 entries, each ≤512 bytes, no absolute root,
NUL, `.`, `..`, or empty components, exactly one top-level directory, and a
`<top>/zig` entry; it returns that top-level name. Its doc (`policy.rs:170-178`)
is careful about scope: symlink and traversal rejection *during extraction* is
enforced by the rendered script and by `validate_provisioned_zig_dir`, not here.
The latter (`policy.rs:231`) verifies the extracted tree offline: root is a real
directory, exactly one child, that child is a real directory, `<top>/zig` is a
regular non-symlink non-empty file, and on Unix has at least one exec bit
(`policy.rs:262-267`); it returns the path to the `zig` executable.

**On the digest's meaning.** The expected digest is a caller-supplied pin. A
download verified against it proves the bytes that arrived match the bytes the
caller expected — transfer integrity. It is not a publisher-authenticity proof:
nothing here establishes that the pinned digest is the one `ziglang.org` would
publish, and no signature or provenance is checked or claimed. The doc at
`policy.rs:141-145` says as much ("the checked-in digest is the authority; no
remotely fetched checksum file is trusted").

## Reusable-workflow runtime identity

A reusable workflow is checked in once and invoked per release. A document
containing `release_id`, `source_revision`, or an exact tag would either be wrong
for every release but one, or require regeneration per release — which defeats
reusability and turns the checked-in workflow into generated output that must itself
be drift-checked.

`ReleaseWorkflowShapeV1` (`crates/eggpack-ci/src/identity.rs:65`) holds only
identity-independent information: canonical ordered `TargetPolicy` targets,
`selected_aliases`, explicit `build_bindings` and `qualification_bindings`, the
per-target `consumer_validators` map, and optional staging intent. Its doc
(`identity.rs:54-61`) states it MUST NOT contain a release id, source revision, an
exact future tag, GitHub release id, or artifact digests/sizes;
`deny_unknown_fields` enforces the negative space, so an injected
`"release_id":"v1"` is rejected
(`crates/eggpack-ci/src/tests.rs:4574`).
`ShapeStagingIntentV1` (`identity.rs:45`) is how staging is expressed in the
shape: `provider` (must be `StagingProvider::GitHubDraft`,
`identity.rs:135-139`), an explicit `tag_source` (`RefName` or `DispatchInput`),
and a `required` flag. It is required for reusable rendering
(`crates/eggpack-ci/src/render_release.rs:281-285`).

`validate` (`identity.rs:104`) enforces `schema_version == 1`, 1..=256 unique
canonically ordered `safe_target` triples, 1..=256 `safe_metadata(alias, 128)`
aliases, and a `validate()` on every embedded validator (`identity.rs:132-134`).
One overclaim: the doc at `identity.rs:102-103` says it validates "embedded
binding/validator documents", but the code validates only the validators
(`identity.rs:132-134`). Bindings are not checked here — core's
`validate_shape` is private (`crates/eggpack-core/src/builder.rs:237`,
`crates/eggpack-core/src/qualification.rs:137`) — so they are validated only
transitively, later, inside `project_ci_plan` and
`project_release_plan_with_consumer` during rendering. `pack_config()`
(`identity.rs:144`) views the shape targets as a `PackConfig`.

`render_reusable_release_github` (`crates/eggpack-ci/src/render_release.rs:276`)
projects the graph against the two placeholders `UNRESOLVED_RELEASE_ID`
(`crates/eggpack-ci/src/identity.rs:37`) and `UNRESOLVED_SOURCE_REVISION`
(`identity.rs:40`), rewrites identity-carrying input paths to workflow-private
storage (`render_release.rs:333`, `:353`), renders, then asserts the rendered
bytes contain neither placeholder (`render_release.rs:369-373`) — a direct proof
that no release identity is embedded. `resolve_runtime_release_plan`
(`identity.rs:162`) turns a runtime identity into a concrete plan through the
`ExactTag` wrapper (`identity.rs:169-176`); its explicit sibling
`resolve_runtime_release_plan_with_mode` (`identity.rs:180`) uses the checked-in
mode. It validates the exact tag with `validate_exact_tag` (≤128 chars, no
control characters, none of `? # @ space \ ' " $ \``, no `..`, no leading/trailing
`/`), the revision with `validate_source_revision` (exactly 40 lowercase hex),
1..=256 aliases (`identity.rs:191-198`), and `schema_version == 1` on the
`PackConfig` (`identity.rs:199-201`); then calls `PackConfig::resolve`
(`identity.rs:202-204`) with the mode-resolved release ID and checks the result's
`release_id` and `source_revision` (`identity.rs:205-209`). Exact mode keeps the
tag as an opaque `release_id` with no product-specific transformation.

CI M003i adds one checked-in opt-in mode,
`v_prefixed_stable_semver`: only `vMAJOR.MINOR.PATCH` with ASCII decimal
components, no leading zeros, and `u64`-bounded values maps to
`MAJOR.MINOR.PATCH`. The renderer rejects a mismatched tag-source/trigger pair
and passes the selected mode as a literal; dispatch has no identity-mode input.
The runtime draft policy carries optional `release_identity_schema_version`,
`release_identity_mode`, `release_id`, and `source_revision` fields only for
this mapped mode. The identity envelope currently accepts version `1` only.
Omitting the mode keeps legacy v1 draft-policy JSON byte-compatible.

Each mapped-mode job verifies the downloaded draft policy against the exact
event-selected tag, release-plan ID, checkout HEAD, and local tag peel. The same
runtime identity artifact is downloaded before build, qualification, consumer
validation, gate, aggregate, and staging work. Staging checks the finalized
manifest's release ID and revision against that policy; the payload and receipt
retain `tag`, `release_id`, and `source_revision` separately. A different tag
pointing at the same commit therefore fails the event-tag comparison.

| Constant | Value | Written by |
|---|---|---|
| `RUNTIME_IDENTITY_DIR` (`crates/eggpack-ci/src/identity.rs:17`) | `eggpack-runtime` | created by the resolve job |
| `RUNTIME_RELEASE_PLAN` (`identity.rs:20`), `RUNTIME_CI_PLAN` (`identity.rs:23`), `RUNTIME_GITHUB_POLICY` (`identity.rs:26`) | `eggpack-runtime/{release-plan,release-ci-plan,github-draft}.json` | `_resolve-release` output flags (`crates/eggpack-ci/src/render_release.rs:362-364`) |
| `RUNTIME_IDENTITY_ARTIFACT` (`identity.rs:29`) | `eggpack-runtime-identity` | uploaded and downloaded by every later job with `if-no-files-found: error` (`render_release.rs:239-242`) |

The resolve job checks out the event-selected exact tag, derives HEAD via
`git rev-parse --verify HEAD^{commit}` (`render_release.rs:714`), writes the three
documents into workflow-private storage, and uploads them as an internal preflight
artifact that every later job downloads *before* its `_verify-source` step
(`render_release.rs:720-723`).

**The security implication, stated plainly: a runtime-supplied plan is a trust
decision made by whoever invokes the workflow.** The checked-in, drift-checked shape
fixes the target set, alias list, bindings, validators, and staging intent. It does
*not* fix the resolved plan's content: the CLI reads the contract and `PackConfig`
from files in the checkout at the invocation ref
(`crates/eggpack-cli/src/main.rs:460-468`) and passes them to
`resolve_runtime_release_plan_with_mode` (`main.rs:478-486`). What ties those files
to a specific commit is `verify_source_revision` (`main.rs:410`), which requires the
given revision to equal HEAD in the explicit source root. Anyone who can invoke the
workflow at a ref they control chooses the plan that ref resolves to — that is the
point of a reusable workflow, and it is why the rendered tag guard
(`crates/eggpack-ci/src/render_release.rs:709-713`, which exports the exact
event-selected tag as `EGGPACK_RELEASE_TAG`) matters more than the shape's
contents. One minor seam weakness: `safe_metadata`
(`crates/eggpack-ci/src/validate.rs:19`) permits commas, but the CLI splits
`--selected` on `,` (`main.rs:469`), so a shape alias containing a comma would be
silently split into two aliases at resolution time; the shape's `selected_aliases`
is not byte-equivalent to what resolution sees.

## Validation and bounds

Bounds are stated per document above. The full set: `BuildHandoffV1` 1 MiB
(`crates/eggpack-ci/src/handoff.rs:20`), paths 512 B with `validate_relative_path`
(`handoff.rs:109`); consumer validator 64 KiB
(`crates/eggpack-ci/src/consumer_validator.rs:14`) with timeout 1 000..=600 000 ms,
limits 1..=8 000 000, script ≤512 B (`consumer_validator.rs:79-97`); validator
script file 1 MiB (`consumer_validator.rs:19`); consumer evidence 64 KiB
(`consumer_validator.rs:16`) with non-zero size and 64-hex digest
(`consumer_validator.rs:183-213`); Zig digests 64 lowercase hex, not all-zero
(`crates/eggpack-ci/src/policy.rs:78-83`, `:161-168`); provisioning timeouts
1..=60 min, 5..=300 s, 60..=3600 s (`policy.rs:98-107`); archive listing ≤4096
entries (`policy.rs:180`); shape 1 MiB (`crates/eggpack-ci/src/identity.rs:31`)
with 1..=256 targets and aliases (`identity.rs:108-131`); runtime tag ≤128 B and
revision exactly 40 hex (`identity.rs:188-189`).

Where enforcement is weaker than it looks:

1. `reconstruct_attempt` tolerates the placeholder size `1`
   (`crates/eggpack-ci/src/handoff.rs:210-214`) while
   `validate_build_artifact_dir` (`handoff.rs:261`) does not. Same document, two
   rules.
2. `reconstruct_attempt` fabricates `ProcessEvidence::Success`
   (`handoff.rs:231-235`), and no candidate digest exists in the handoff at all —
   size only.
3. `decode_consumer_evidence` (`crates/eggpack-ci/src/release_plan.rs:406`) bounds
   and parses but does **not** call `validate()`, unlike `from_json`
   (`crates/eggpack-ci/src/consumer_validator.rs:172`).
   `evaluate_gate_with_consumer` compensates explicitly at
   `release_plan.rs:426-428`; a future caller of the decoder alone would get a
   weaker check.
4. `ReleaseWorkflowShapeV1::validate` does not validate its embedded bindings
   despite its doc comment (`crates/eggpack-ci/src/identity.rs:102-103` vs
   `identity.rs:132-134`).
5. `ensure_candidate_executable` (`consumer_validator.rs:451`) mutates the
   candidate's mode after its bytes were verified, with no re-check afterwards.

The two gaps that the private CI runner used to have — output retention above the
cap, and a preflight that discarded its over-limit flags — are gone with it. Both
now belong to the shared runner: `drain` truncates exactly
(`crates/eggpack-core/src/builder.rs:942-958`) and the preflight's over-limit
outcome is a non-success runner outcome, which the CI side maps to
`Failed(InterpreterUnavailable)` (`consumer_validator.rs:503-506`).

## Boundaries / non-goals

No build, qualification, or finalization (`eggpack-core` performs; here
`reconstruct_attempt` only builds an in-memory structure). No rendering — see
[ci-rendering.md](ci-rendering.md). No publication and no GitHub API access:
staging intent is *described* here and *performed* in `eggpack-github` (see
[github.md](github.md)). No authenticity claim: sizes and SHA-256 digests are
integrity facts, of the consumer validator, the Zig archive pin, and the candidate
identity check alike. No arbitrary command execution surface — the validator's argv
is fixed to two elements plus the interpreter
(`crates/eggpack-ci/src/consumer_validator.rs:526-534`) and the runner commands are
a finite enum (`crates/eggpack-ci/src/runner.rs:13`). No release selection, update,
or rollback (those belong to the product
repository and to Eggup, see
[eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md)). The process
-execution contract is core's, not a private one: the validator runs through
`eggpack_core::run_interpreter_bounded`, so core's `command-group` guarantees
apply here (see [process-execution.md](process-execution.md)).

## Tests

The crate's tests live in `crates/eggpack-ci/src/tests.rs` (7638 lines, declared
out of line by `crates/eggpack-ci/src/lib.rs:18-19`). That module has **81 test
attributes** (80 `#[test]`, 2 `#[tokio::test]`), plus 16
`#[cfg(unix)]`/`#[cfg(windows)]`/`#[cfg(not(windows))]`-gated blocks.

| Test | Line | What it proves |
|---|---|---|
| `archive_handoff_identity_is_bounded_deterministic_and_slot_distinct` | `tests.rs:293` | archive-slot handoff identity is deterministic and distinct from `Direct` |
| `m002_handoff_rejects_traversal_and_swaps` | `tests.rs:713` | handoff traversal and selector swap rejection |
| `m002a_generated_orchestration_executes_end_to_end` | `tests.rs:1589` | the rendered command/argument/layout contract drives build → capture → qualify → gate → aggregate → M004 finalization directly |
| `m003d_validator_config_validation` | `tests.rs:3186` | bounds on schema version, interpreter, script path, timeout, output limits |
| `m003d_consumer_execution_matrix` | `tests.rs:3289` | pass; non-zero exit; timeout; stdout flood → `OutputLimit`; hermetic missing interpreter; size+digest mismatch; symlinked script; symlinked candidate; cancellation → `Cancelled`; misuse → `Err` |
| `m003d_consumer_gate_matrix` (`tests.rs:4063`), `m003d_consumer_attach_rejects_mismatch` (`tests.rs:4179`), `m003d_consumer_render_gates_and_detects_drift` (`tests.rs:4364`), `m003d_python_mapping_and_hermetic_lookup` (`tests.rs:3940`) | | gate treatment of passed/failed/required/non-gating evidence; attaching a validator to an unknown target or non-existent selector fails; the consumer job renders and drift-detects; the `python3`/`python` mapping |
| `m003d_shape_validation` (`tests.rs:4574`), `m003d_runtime_resolve_matrix` (`tests.rs:4608`), `m003d_reusable_check_detects_shape_drift` (`tests.rs:4833`), `m003d_runner_commands_cover_consumer_and_resolve` (`tests.rs:4439`) | | shape round-trips and rejects an injected `release_id`; runtime resolution deterministic and fails closed on bad tag/revision/selection; a shape change is detected as drift; `RunnerCommand` covers the consumer and resolve commands |
| `m003g_validator_restores_exec_on_transferred_candidate` | `tests.rs:5732` | exec bits restored, bytes untouched |
| `m005_zig_archive_mapping_is_finite_and_official`, `..._listing_validation_without_network`, `m005_provisioned_zig_dir_checks_without_network` | `tests.rs:5831`, `:6214`, `:6240` | exact archive names and the fixed `ziglang.org` URL; listing layout validation offline; provisioned dir validation offline, including symlinked and non-executable `zig` rejection |

Three honest caveats:

- The orchestration test is named `..._end_to_end`, but its own doc comment
  (`tests.rs:1585-1587`) says the same contract "drives build -> capture -> qualify
  -> gate -> aggregate -> M004 finalization directly (**no GitHub invocation**)". It
  ends at aggregate/finalization. **Publication is not exercised anywhere in this
  crate.**
- The missing-interpreter case in `m003d_consumer_execution_matrix` is
  `#[cfg(not(windows))]` (`tests.rs:3354`): Windows `CreateProcess` searches the
  application directory, working directory, and system directories unconditionally,
  so an emptied PATH cannot simulate a missing interpreter there
  (`tests.rs:3351-3353`). The Windows lane proves the `python` mapping and the
  preflight only through the other matrix cases.
- No test covers a validator that forks. Since CI M003m the validator runs through
  core's process-group runner, so the group semantics are core's contract; this
  crate simply no longer has a process-group gap to exercise.

## Dependencies / dependents

Dependencies: `eggpack-contract`, `eggpack-core`, `serde`, `serde_json`, `sha2`.
Dev-dependencies: `serde_yaml` (test assertions only), plus `eggpack-manifest`,
`eggpack-bootstrap`, `eggpack-github`, and `tokio` so the crate's orchestration tests
can drive the real downstream crates without a network. Notably absent:
`command-group`, which `eggpack-core` pins at `5.0.1`
(`crates/eggpack-core/Cargo.toml:24`, `Cargo.lock:61-64`); the validator inherits
core's group semantics through `run_interpreter_bounded` instead of declaring its
own.

Dependents: `eggpack-cli`, the only production consumer. It calls
`resolve_runtime_release_plan_with_mode`
(`crates/eggpack-cli/src/main.rs:478-486`), `project_build_handoff` (`main.rs:969`),
`validate_build_artifact_dir` (`main.rs:1033`), `reconstruct_attempt` (`main.rs:773`,
`main.rs:1090`), `validate_qualification_artifact_dir` (`main.rs:1123`),
`run_consumer_validator` (`main.rs:1285`), and
`ReleaseWorkflowShapeV1::from_json` (`main.rs:843`, `main.rs:898`).
`stage_build_artifact_dir` has no production caller.

## Related deep dives

- [ci.md](ci.md) — crate index: projection model, job vocabulary, gates, aggregation
- [ci-rendering.md](ci-rendering.md) — policy types, renderers, drift checking
- [overview.md](overview.md) — workspace-level view
- [cli.md](cli.md) — the `eggpack` binary that executes the generated steps
- [core.md](core.md) — the pipeline this crate projects
- [core-qualification.md](core-qualification.md) — the binary qualification authority the consumer validator does not replace
- [core-finalization.md](core-finalization.md) — M004, where the gate's `Complete` outcome lands
- [process-execution.md](process-execution.md) — bounded process execution, and the group-handling gap detailed above
- [validation-model.md](validation-model.md) — the workspace-wide fail-closed rules
- [determinism.md](determinism.md) — canonical ordering and byte-identical rendering
- [github.md](github.md) — where staging intent is actually performed
- [testing-and-portability.md](testing-and-portability.md) — the golden corpus and the Windows lane
