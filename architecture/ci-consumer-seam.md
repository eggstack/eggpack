# `eggpack-ci` — Consumer Seam, Handoffs, and Runtime Identity — Deep Dive

The half of `eggpack-ci` (`crates/eggpack-ci/src/lib.rs`, 10256 lines) concerned
with agreeing on artifacts and identities rather than doing release work: the
build/qualification handoff documents, the external consumer validator and its
private process runner, the Zig/cross-tool provisioning contract, and the
reusable-workflow runtime identity. It divides from [ci.md](ci.md) (projection
model, job vocabulary, gates, aggregation) and [ci-rendering.md](ci-rendering.md)
(policy types, renderers, drift checking). Regions: `:487-713`, `:941-963`,
`:1837-2207`, `:3840-4705`.

> **Known-stale citations in this file.** Commit `61b2c03` moved
> `crates/eggpack-cli/src/main.rs` and `crates/eggpack-core/src/builder.rs`
> substantially, and this file's anchors into those two were not re-based — see
> [overview.md](overview.md) §Citation-verification state. Anchors into
> `crates/eggpack-ci/src/lib.rs` are current; the `:1837-2207` and
> `:3840-4705` region bounds above are the pre-`61b2c03` values and are now
> `:1837-2209` and `:3840-4707`. Treat the `main.rs` and `builder.rs` line
> references as indicative until someone re-bases them by symbol name. The prose,
> the handoff constant names, and the seam contract itself are unaffected.

## Responsibility

| Job | Region | Problem it solves |
|---|---|---|
| Build/qualification handoff | `:1837-2207` | The generated workflow does not call library functions; it shells out to the `eggpack` binary. Two independently versioned processes must agree on artifact layout and document shape. |
| External consumer validator | `:3840-4705` | A third party needs to assert something about a release without eggpack claiming it. The seam runs caller-supplied code and returns bounded evidence. |
| Runtime identity | `:4425-4623` | A checked-in reusable workflow cannot hardcode a release id, source revision, or tag, or it would be regenerated per release. The shape is identity-free; the plan is resolved per invocation. |

All three are *agreement* mechanisms, not security boundaries against a hostile
caller: the caller supplies the contracts, and validation is fail-closed against
malformed or out-of-bounds documents, not against a determined adversary.

## The build/qualification handoff formats

`BUILD_HANDOFF_FILE` = `build-handoff.json` (`:941`), `QUALIFICATION_EVIDENCE_FILE`
= `evidence.json` (`:943`), `CANDIDATES_DIR` = `candidates` (`:945`),
`GATE_OUTCOME_FILE` = `gate-outcome.json` (`:947`, serialized at `:3528`). These
four strings are the entire contract between the rendered workflow and the
`eggpack` binary. A change to a name, a JSON shape, or a validation rule here
survives compilation and fails at CI runtime: nothing in the type system ties a
producer write to a consumer read.

| Document | Type | Producer | Consumer | Validated on read |
|---|---|---|---|---|
| build handoff | `BuildHandoffV1` (`:1862`) | `project_build_handoff` (`:1944`), written by CLI `_capture-build` | `validate_build_artifact_dir` (`:2062`), `reconstruct_attempt` (`:2005`) | `from_json` (`:1885`) + `validate` (`:1896`), on every read |
| candidate bytes | files under `candidates/` | `stage_build_artifact_dir` (`:2152`) / CLI staging | same | size, regular non-symlink file, exact inventory, no extras (`:2073-2105`) |
| qualification evidence | `QualificationEvidence` (core) | qualification job | `validate_qualification_artifact_dir` (`:2111`) | identity must equal the handoff (`:2118-2123`) |
| consumer evidence | `ConsumerValidationEvidenceV1` (`:3968`) | `run_consumer_validator` (`:4151`) | `decode_consumer_evidence` (`:2586`), `evaluate_gate_with_consumer` (`:2600`) | `from_json` (`:3997`) + `validate` (`:4008`); gate re-validates at `:2607` |
| workflow shape | `ReleaseWorkflowShapeV1` (`:4472`) | checked in | `render_reusable_release_github` (`:2947`), `check_reusable_release_github` (`:3048`) | `from_json` (`:4493`) + `validate` (`:4511`) |

`BuildHandoffOutput` (`:1844`) carries the contract logical selector, an explicit
Cargo package and binary, a relative path below the handoff root, an exact size,
and a deterministic `handoff_identity`; `BuildHandoffV1` (`:1862`) adds
`schema_version`, `release_id`, `source_revision`, the canonical target, the build
`strategy`, and canonically ordered outputs, all under `deny_unknown_fields`.
`validate` (`:1896`) enforces `schema_version == 1`; `safe_metadata(.., 256)`
identities; `safe_target` (`:371`); 1..=256 outputs in strictly ascending
`LogicalOutputSelector` order with no duplicates; unique non-empty
`handoff_identity` bounded to 256 bytes; `safe_identifier` (`:377`) package and
binary; non-zero sizes; and `validate_relative_path` (`:1928`) per path, rejecting
absolute roots, drive colons, NUL, `..`, empty segments, and >512 bytes.
`from_json` bounds the document at `MAX_HANDOFF_JSON` (`:1838`, 1 MiB) before
parsing; `to_json` validates before serializing.

`project_build_handoff` (`:1944`) is pure — no I/O. It derives a deterministic
`relative_path` per selector (`candidate-direct`, `candidate-bundle-{index}`,
`candidate-archive-{sha256(source)}`, `:1960-1973`) and sorts outputs canonically
(`:1987`), setting `size: 1` as a non-zero placeholder (`:1983`) with an explicit
comment that callers must not treat it as evidence (`:1974-1977`). The runner
replaces it with the observed size and re-validates — the CLI does exactly that at
`crates/eggpack-cli/src/main.rs470`, then `validate_build_artifact_dir`
(`main.rs504`).

### `reconstruct_attempt`

`reconstruct_attempt` (`:2005`) turns a validated handoff plus downloaded files
back into an in-memory `eggpack_core::BuildAttempt`, requiring the handoff identity
to equal the plan's release id, source revision, target, and strategy
(`:2011-2018`) and each output to be a non-symlink, regular, non-empty file. What
it deliberately does **not** re-verify:

- It does not hash the candidate. No SHA-256 appears in the handoff at all; the
  reconstructed `CandidateArtifact` carries the observed size only. Byte identity
  is established later, by the qualification job and by the consumer validator's
  expected digest.
- It accepts a handoff whose `size` is still the placeholder `1` without comparing
  it to the observed length (`:2029-2033`), while `validate_build_artifact_dir`
  (`:2080`) has no such exception and requires exact size equality. **The two
  readers of the same document disagree about size enforcement.**
- It synthesizes process evidence rather than observing any: `tool_summary` is
  `format!("ci-handoff {target}")` (`:2049`) and `ProcessEvidence` is a literal
  `CommandOutcome::Success` with `stdout_bytes: 0, stderr_bytes: 0` (`:2050-2054`).
  A reconstructed attempt therefore always claims a successful process. The doc
  (`:2000-2004`) says the result must still pass M003/M004 validation; these
  fabricated fields are not evidence.

### Directory validators and path derivation

`validate_build_artifact_dir` (`:2062`) requires `candidates/` to be a real
directory (not a symlink), every handoff output present as a non-symlink regular
file with exact size, and — importantly — **no extra entries**: any non-file entry
or unlisted name is rejected, and the final count must equal the inventory
(`:2086-2105`). `validate_qualification_artifact_dir` (`:2111`) runs that first, then
decodes `evidence.json` and requires the evidence identity to match the handoff's
release id, source revision, and target (`:2118-2123`).

`stage_build_artifact_dir` (`:2152`) creates or reuses a real output directory,
hard-links each candidate (falling back to copy, `:2144`), writes the handoff, then
**re-validates the directory it just wrote** (`:2177`), so a size mismatch fails
here. Two caveats: the handoff write is a plain `std::fs::write` (`:2175`), not an
atomic replace, unlike the CLI's `atomic_write` (`main.rs503`); and this function
has no caller outside this crate's own tests — the CLI stages inline at
`main.rs491-505` using the literals `"candidates"` and `"build-handoff.json"`
rather than the constants. `cargo_output_path` (`:2183`) derives
`<cargo_target_dir>/<target>/release/<binary>` after `validate_canonical_target_dir`
(`:951`), appending `.exe` for targets whose triple contains the `-windows-`
component (`:2201-2203`). The suffix is appended to the full binary name, so a
dotted name resolves to `tool.cli.exe`; this agrees with
`discover_candidate` in `eggpack-core` and with Cargo's own naming.

## The external consumer validator

The one place the producer runs foreign, caller-supplied code.

`ConsumerValidatorV1` (`:3873`) can specify `schema_version`, one
`LogicalOutputSelector`, an interpreter, a repository-relative `script` path, a
timeout, and stdout/stderr limits. There is deliberately no arbitrary executable
field, no shell string, no environment map, and no caller-supplied argument vector
(`:3864-3870`). `ValidatorInterpreterV1` (`:3859`) has exactly one variant,
`Python3`, and host mapping is finite — `python3` on Linux/macOS, `python` on
Windows — via `python_interpreter_exe` (`:4032`) delegating to the `cfg!`-testable
`python_interpreter_exe_for_windows` (`:4037`). `validate` (`:3909`) enforces
`schema_version == 1`; interpreter is `Python3`; `validate_release_input_path`
(`:924`) on the script (bounded relative, no absolute root, no drive colon, no
backslash, no `.`/`..`/empty segments, per-segment ≤128, total ≤512); `timeout_ms`
in 1 000..=600 000; both output limits in 1..=8 000 000. `from_json` (`:3892`)
bounds the document at `MAX_VALIDATOR_JSON` (`:3846`, 64 KiB). Separately,
`project_release_plan_with_consumer` (`:2536`) requires ≤256 validators, each
target to exist in the `CIPlan`, and each selector to be an actual build output of
that target's bindings (`:2546-2567`).

`ConsumerValidationRequest` (`:4046`) supplies the validator plus absolute
`script_path`, `candidate_path`, `work_dir`, the `expected_size` and
`expected_sha256` to verify against, release identity strings, an optional
`AtomicBool` cancellation flag, and an optional exact PATH override (`:4071`). The
CLI populates it at `main.rs766-778` with the size and digest taken from the
**qualification evidence**, not from the handoff (`main.rs770-771`), and refuses
outright to run if that evidence is not `Passed` (`main.rs729-734`).

Candidate identity is observed *before* any execution (`:4198-4200`) so evidence
always carries validated linkage; a synthetic `evidence_size.max(1)` / all-zero
digest stand-in is substituted when the candidate is itself the failure point
(`:4206-4211`). `run_consumer_validator` (`:4151`) is then fail-closed throughout.
Caller *misuse* returns `Err(CiError)`; validator *outcomes* return bounded
evidence.

| Step | Citation | Result on failure |
|---|---|---|
| re-validate config; script, candidate, work dir absolute; work dir a real directory; `expected_sha256` 64 lowercase hex; identity strings in bounds | `:4154-4184` | `Err` |
| candidate hashed, size and digest compared to expected | `:4200-4219` | `Failed(CandidateMismatch)` |
| script is a regular non-symlink file of 1 B..=1 MiB | `:4190-4197`, `:4220` | `Failed(ScriptUnavailable)` |
| restore POSIX exec bits on the transferred candidate | `:4230-4237`, `:4281` | `Failed(CandidateMismatch)` |
| bounded `python --version` preflight must contain "Python 3" | `:4256-4264`, `:4372` | `Failed(InterpreterUnavailable)` |
| fixed `python3 <script> <candidate>` invocation | `:4266`, `:4625` | `Passed` or one of six failures |

`ensure_candidate_executable` (`:4281`) sets mode `| 0o111` on the candidate after
its bytes were verified. The doc (`:4276-4280`) argues this cannot change identity
because size and digest cover bytes only. That is correct, but it makes this the
one place the CI layer mutates a candidate file in place, between verification and
execution, with no re-check afterwards.

`ConsumerValidationFailure` (`:3934`) classifies seven bounded causes:
`NonZeroExit`, `Timeout`, `OutputLimit`, `InterpreterUnavailable`,
`CandidateMismatch`, `ScriptUnavailable`, `Cancelled`. `ConsumerValidationOutcome`
(`:3954`) is `Passed` or `Failed(..)`. `ConsumerValidationEvidenceV1` (`:3968`)
records schema version, release/source/target identity, selector, interpreter,
outcome, observed size, and observed digest — nothing else; a test asserts the
encoded JSON contains no `stdout`, `stderr`, or `output` key (`:7611-7613`). A
candidate that was never read is recorded as absent (`size` 0, digest `null`) and
validation rejects either half without the other: this document is evidence of
what the producer saw, so no digest is ever synthesized for a file that was not
hashed.

The validator is **caller-supplied**, so it is not a security boundary against a
hostile caller: a caller who supplies `script` supplies arbitrary code that eggpack
will execute with the candidate's path as an argument. What the design buys is
narrower and still real. The *shape* of the invocation is fixed and finite — one
interpreter, one script path, one candidate path, no shell, no argument vector, no
env map (`:4359-4368`). The *environment is cleared* to `PATH` plus `SYSTEMROOT` on
Windows (`:4347-4354`), stdin is null (`:4356`), the working directory is explicit.
The *outcome is bounded*: timeout, output limits, a seven-value classification; a
validator cannot hang or flood the job. This is a **separation-of-concerns**
mechanism: `Passed` means an independent script exited zero against the exact
candidate. It is not an authenticity claim, and eggpack does not assert it on the
validator's behalf — M003 remains the provider-neutral binary qualification
authority (`:3866-3867`).

## The validator's independent process runner

**A known gap, not an undocumented oversight.** `eggpack-ci` declares no
`command-group` dependency (`crates/eggpack-ci/Cargo.toml` lists only
`eggpack-contract`, `eggpack-core`, `serde`, `serde_json`, `sha2`), and
`run_validator_process` (`:4625`) and `run_interpreter_preflight` (`:4372`) use
`std::process::Command` directly (`:4343`, `:4378`, `:4638`). Verified:

- **No process group anywhere.** No `process_group`, `pre_exec`, `CREATE_SUSPENDED`,
  or job-object use in the crate.
- **Bare `Child::kill()` reaches exactly one process**, at `:4393`, `:4665`, `:4670`,
  `:4678`, `:4685`.
- **Wait-after-kill IS present** at every site: `:4394`, `:4666`, `:4671`, `:4679`,
  `:4686` — a partial match with core
  (`crates/eggpack-core/src/builder.rs557-570`).
- What escapes: a validator script that forks (or Python that spawns) leaves
  grandchildren alive past a timeout or cancellation, still holding the inherited
  stdout/stderr pipes. The kill reaches the direct child only; the grandchildren
  are never signalled. Core's `group_spawn` (`builder.rs533`) uses
  `setpgid(0, 0)` + `killpg` on Unix and `TerminateJobObject` on Windows, so it does
  not have this hole.
- **Output overshoot**: `read_limited` (`:4095`) and `read_limited_stderr` (`:4119`)
  append the full 8 KiB chunk *before* the limit check (`:4106` then `:4107`), so
  retained bytes can exceed the cap by up to 8191 bytes. Core's `drain`
  (`builder.rs622`) computes `keep = n.min(limit - len)` and truncates exactly.
- **Preflight discards its over-limit flags**: `:4402-4403` binds
  `let (out, _) = ...` / `let (err, _) = ...`, so an over-limit `--version` flood is
  neither failed on nor reported. Preflight only needs "Python 3" in the first 8 KiB.
- One thing CI's runner does *better*: `take_finished_over` (`:4416`) joins only
  already-finished readers and fails the run immediately on an over-limit reader
  (`:4669-4673`), catching a flood at the first chunk rather than at the deadline.

See [process-execution.md](process-execution.md), which records this as the largest
process-execution risk difference in the workspace: a reimplementation that will not
fail a compile-time or test-time check when core's contract drifts.

## Zig and cross-tool provisioning

`CrossToolProvisioningV1` (`:503`) is a provider *how* policy: it bounds
`cargo install --locked` for cargo-zigbuild (1..=60 minutes, `:543-544`) and curl
`--connect-timeout` (5..=300 s) / `--max-time` (60..=3600 s) for the Zig download
(`:545-548`). Which *versions* are required comes from `ToolchainRequirement` in the
resolved `TargetPolicy`; the split is documented at `:494-500`. No generic package
manager, arbitrary URL, or setup command is representable. `ZigOfficialArchiveV1`
(`:487`) holds one digest per supported Linux host architecture; `validate` (`:516`)
requires exactly 64 lowercase hex characters and rejects the all-zero digest as a
sentinel (`:522`); `digest_for` (`:531`) maps `X86_64`/`Aarch64` and fails for
`Armv7`.

The version-to-archive mapping is pure and finite. `zig_archive_name` (`:561`) emits
`zig-x86_64-linux-{version}.tar.xz` or `zig-aarch64-linux-{version}.tar.xz`, failing
for non-Linux hosts, `Armv7`, or a version that fails `safe_tool_version` (`:386`,
≤64 chars from `[A-Za-z0-9.+-_]`). `zig_download_url` (`:577`) is fixed to
`https://ziglang.org/download/{version}/{name}` — no caller URL, mirror, or redirect
target is representable. `zig_expected_digest` (`:590`) requires a Linux host and
returns the policy's pinned digest.

`validate_zig_archive_listing` (`:623`) is a network-free layout check over a file
listing: non-empty, ≤4096 entries, each ≤512 bytes, no absolute root, NUL, `.`,
`..`, or empty components, exactly one top-level directory, and a `<top>/zig` entry;
it returns that top-level name. Its doc (`:614-622`) is careful about scope: symlink
and traversal rejection *during extraction* is enforced by the rendered script and by
`validate_provisioned_zig_dir`, not here. The latter (`:675`) verifies the extracted
tree offline: root is a real directory, exactly one child, that child is a real
directory, `<top>/zig` is a regular non-symlink non-empty file, and on Unix has at
least one exec bit (`:705-711`); it returns the path to the `zig` executable.

**On the digest's meaning.** The expected digest is a caller-supplied pin. A
download verified against it proves the bytes that arrived match the bytes the
caller expected — transfer integrity. It is not a publisher-authenticity proof:
nothing here establishes that the pinned digest is the one `ziglang.org` would
publish, and no signature or provenance is checked or claimed. The doc at `:585-589`
says as much ("the checked-in digest is the authority; no remotely fetched checksum
file is trusted").

## Reusable-workflow runtime identity

A reusable workflow is checked in once and invoked per release. A document
containing `release_id`, `source_revision`, or an exact tag would either be wrong
for every release but one, or require regeneration per release — which defeats
reusability and turns the checked-in workflow into generated output that must itself
be drift-checked.

`ReleaseWorkflowShapeV1` (`:4472`) holds only identity-independent information:
canonical ordered `TargetPolicy` targets, `selected_aliases`, explicit
`build_bindings` and `qualification_bindings`, the per-target `consumer_validators`
map, and optional staging intent. Its doc (`:4461-4469`) states it MUST NOT contain
a release id, source revision, exact future tag, GitHub release id, or artifact
digests/sizes; `deny_unknown_fields` enforces the negative space, so an injected
`"release_id":"v1"` is rejected (`:8397-8400`). `ShapeStagingIntentV1` (`:4452`) is
how staging is expressed in the shape: `provider` (must be
`StagingProvider::GitHubDraft`, `:4543`), an explicit `tag_source` (`RefName` or
`DispatchInput`), and a `required` flag. It is required for reusable rendering
(`:2953-2956`), and the shape's tag source must equal the provider staging policy's
(`:3010-3014`).

`validate` (`:4511`) enforces `schema_version == 1`, 1..=256 unique canonically
ordered `safe_target` triples, 1..=256 `safe_metadata(alias, 128)` aliases, and a
`validate()` on every embedded validator. One overclaim: the doc at `:4509-4510`
says it validates "embedded binding/validator documents", but the code validates
only the validators (`:4539-4541`). Bindings are not checked here — core's
`validate_shape` is private (`crates/eggpack-core/src/builder.rs237`,
`crates/eggpack-core/src/qualification.rs:137`) — so they are validated only
transitively, later, inside `project_ci_plan` and `project_release_plan_with_consumer`
during rendering. `pack_config()` (`:4551`) views the shape targets as a `PackConfig`.

`render_reusable_release_github` (`:2947`) projects the graph against the two
placeholders `UNRESOLVED_RELEASE_ID` (`:4445`) and `UNRESOLVED_SOURCE_REVISION`
(`:4447`), rewrites identity-carrying input paths to workflow-private storage
(`:3004-3005`, `:3018`), renders, then asserts the rendered bytes contain neither
placeholder (`:3041-3043`) — a direct proof that no release identity is embedded.
`resolve_runtime_release_plan` (`:4781`) turns a runtime identity into a concrete
plan through the `ExactTag` wrapper (`:4789-4790`); its explicit sibling
`resolve_runtime_release_plan_with_mode` (`:4799`) uses the checked-in mode. It
validates the exact tag with `validate_exact_tag` (`:4744`: ≤128 chars, no
control characters, none of `? # @ space \ ' " $ \``, no `..`, no leading/trailing
`/`), the revision with `validate_source_revision` (`:4575`: exactly 40 lowercase
hex), 1..=256 aliases, and `schema_version == 1` on the `PackConfig`; then calls
`PackConfig::resolve` (`:4820-4822`) with the mode-resolved release ID and checks
the result's `release_id` and `source_revision` (`:4823-4828`). Exact mode keeps
the tag as an opaque `release_id` with no product-specific transformation.

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
| `RUNTIME_IDENTITY_DIR` (`:4615`) | `eggpack-runtime` | created by the resolve job |
| `RUNTIME_RELEASE_PLAN` (`:4617`), `RUNTIME_CI_PLAN` (`:4619`), `RUNTIME_GITHUB_POLICY` (`:4621`) | `eggpack-runtime/{release-plan,release-ci-plan,github-draft}.json` | `_resolve-release` output flags (`:3143-3145`) |
| `RUNTIME_IDENTITY_ARTIFACT` (`:4623`) | `eggpack-runtime-identity` | uploaded and downloaded by every later job with `if-no-files-found: error` |

The resolve job checks out the event-selected exact tag, derives HEAD via
`git rev-parse --verify HEAD^{commit}` (`:3227`), writes the three documents into
workflow-private storage, and uploads them as an internal preflight artifact that
every later job downloads *before* its `_verify-source` step (`:2938-2946`).

**The security implication, stated plainly: a runtime-supplied plan is a trust
decision made by whoever invokes the workflow.** The checked-in, drift-checked shape
fixes the target set, alias list, bindings, validators, and staging intent. It does
*not* fix the resolved plan's content: the CLI reads the contract and `PackConfig`
from files in the checkout at the invocation ref (`crates/eggpack-cli/src/main.rs123`,
`:126`) and passes them to `resolve_runtime_release_plan` (`main.rs133`). What ties
those files to a specific commit is `verify_source_revision` (`main.rs119`), which
requires the given revision to equal HEAD in the explicit source root. Anyone who can
invoke the workflow at a ref they control chooses the plan that ref resolves to —
that is the point of a reusable workflow, and it is why the rendered tag guard
(`:3213-3218`) matters more than the shape's contents. One minor seam weakness:
`safe_metadata` (`:383`) permits commas, but the CLI splits `--selected` on `,`
(`main.rs129`), so a shape alias containing a comma would be silently split into two
aliases at resolution time; the shape's `selected_aliases` is not byte-equivalent to
what resolution sees.

## Validation and bounds

Bounds are stated per document above. The full set: `BuildHandoffV1` 1 MiB
(`:1838`), paths 512 B with `validate_relative_path` (`:1928-1940`); consumer
validator 64 KiB (`:3846`) with timeout 1 000..=600 000 ms, limits
1..=8 000 000, script ≤512 B (`:3909-3928`); validator script file 1 MiB
(`:3849`); consumer evidence 64 KiB (`:3847`) with non-zero size and 64-hex digest
(`:4008-4026`); Zig digests 64 lowercase hex, not all-zero (`:516-527`, `:602-611`);
provisioning timeouts 1..=60 min, 5..=300 s, 60..=3600 s (`:542-553`); archive
listing ≤4096 entries (`:624`); shape 1 MiB (`:4439`) with 1..=256 targets and
aliases (`:4511-4547`); runtime tag ≤128 B (`:4560`) and revision exactly 40 hex
(`:4576`).

Where enforcement is weaker than it looks:

1. `reconstruct_attempt` tolerates the placeholder size `1` (`:2029-2033`) while
   `validate_build_artifact_dir` (`:2080`) does not. Same document, two rules.
2. `reconstruct_attempt` fabricates `ProcessEvidence::Success` (`:2050-2054`), and
   no candidate digest exists in the handoff at all — size only.
3. `decode_consumer_evidence` (`:2586`) bounds and parses but does **not** call
   `validate()`, unlike `from_json` (`:3997`). `evaluate_gate_with_consumer`
   compensates explicitly at `:2607`; a future caller of the decoder alone would get a
   weaker check.
4. `ReleaseWorkflowShapeV1::validate` does not validate its embedded bindings
   despite its doc comment (`:4509-4510` vs `:4539-4541`).
5. `read_limited` retention can exceed its cap by up to 8 KiB (`:4105-4111`); core's
   `drain` truncates exactly (`builder.rs622`).
6. The interpreter preflight discards its over-limit flags (`:4402-4403`).
7. `ensure_candidate_executable` (`:4281`) mutates the candidate's mode after its
   bytes were verified, with no re-check afterwards.

## Boundaries / non-goals

No build, qualification, or finalization (`eggpack-core` performs; here
`reconstruct_attempt` only builds an in-memory structure). No rendering — see
[ci-rendering.md](ci-rendering.md). No publication and no GitHub API access:
staging intent is *described* here and *performed* in `eggpack-github` (see
[github.md](github.md)). No authenticity claim: sizes and SHA-256 digests are
integrity facts, of the consumer validator, the Zig archive pin, and the candidate
identity check alike. No arbitrary command execution surface — the validator's argv
is fixed to three elements (`:4359-4368`) and the runner commands are a finite enum
(`:972`). No release selection, update, or rollback (those belong to the product
repository and to Eggup, see
[eggup-manifest-consumer-v1.md](eggup-manifest-consumer-v1.md)). No shared
process-execution contract: core's `command-group` guarantees are unavailable here.

## Tests

The test module has **62 test attributes** (60 `#[test]`, 2 `#[tokio::test]`), plus
11 `#[cfg(unix)]`/`#[cfg(windows)]`-gated blocks.

| Test | Line | What it proves |
|---|---|---|
| `archive_handoff_identity_is_bounded_deterministic_and_slot_distinct` | `:4953` | archive-slot handoff identity is deterministic and distinct from `Direct` |
| `m002_handoff_rejects_traversal_and_swaps` | `:5376` | handoff traversal and selector swap rejection |
| `m002a_generated_orchestration_executes_end_to_end` | `:6250` | the rendered command/argument/layout contract drives build → capture → qualify → gate → aggregate → M004 finalization directly |
| `m003d_validator_config_validation` | `:7561` | bounds on schema version, interpreter, script path, timeout, output limits |
| `m003d_consumer_execution_matrix` | `:7664` | pass; non-zero exit; timeout; stdout flood → `OutputLimit`; hermetic missing interpreter; size+digest mismatch; symlinked script; symlinked candidate; cancellation → `Cancelled`; misuse → `Err` |
| `m003d_consumer_gate_matrix` (`:7979`), `m003d_consumer_attach_rejects_mismatch` (`:8102`), `m003d_consumer_render_gates_and_detects_drift` (`:8187`), `m003d_python_mapping_and_hermetic_lookup` (`:7856`) | | gate treatment of passed/failed/required/non-gating evidence; attaching a validator to an unknown target or non-existent selector fails; the consumer job renders and drift-detects; the `python3`/`python` mapping |
| `m003d_shape_validation` (`:8388`), `m003d_runtime_resolve_matrix` (`:8418`), `m003d_reusable_check_detects_shape_drift` (`:8585`), `m003d_runner_commands_cover_consumer_and_resolve` (`:8256`) | | shape round-trips and rejects an injected `release_id`; runtime resolution deterministic and fails closed on bad tag/revision/selection; a shape change is detected as drift; `RunnerCommand` covers the consumer and resolve commands |
| `m003g_validator_restores_exec_on_transferred_candidate` | `:9482` | exec bits restored, bytes untouched |
| `m005_zig_archive_mapping_is_finite_and_official`, `..._listing_validation_without_network`, `m005_provisioned_zig_dir_checks_without_network` | `:9581`, `:10066`, `:10097` | exact archive names and the fixed `ziglang.org` URL; listing layout validation offline; provisioned dir validation offline, including symlinked and non-executable `zig` rejection |

Three honest caveats:

- The orchestration test is named `..._end_to_end`, but its own doc comment
  (`:6246-6248`) says the same contract "drives build -> capture -> qualify -> gate
  -> aggregate -> M004 finalization directly (**no GitHub invocation**)". It ends at
  aggregate/finalization. **Publication is not exercised anywhere in this crate.**
- The missing-interpreter case in `m003d_consumer_execution_matrix` is
  `#[cfg(not(windows))]` (`:7729`): Windows `CreateProcess` searches the
  application directory, working directory, and system directories unconditionally,
  so an emptied PATH cannot simulate a missing interpreter there (`:7723-7728`). The
  Windows lane proves the `python` mapping and the preflight only through the other
  matrix cases.
- No test covers a validator that forks, so the process-group gap is not exercised
  either way.

## Dependencies / dependents

Dependencies: `eggpack-contract`, `eggpack-core`, `serde`, `serde_json`, `sha2`.
Dev-dependencies: `serde_yaml` (test assertions only), plus `eggpack-manifest`,
`eggpack-bootstrap`, `eggpack-github`, and `tokio` so the crate's orchestration tests
can drive the real downstream crates without a network. Notably absent:
`command-group`, which `eggpack-core` pins at `5.0.1`
(`crates/eggpack-core/Cargo.toml:21`, `Cargo.lock:61`).

Dependents: `eggpack-cli`, the only production consumer. It calls
`resolve_runtime_release_plan` (`crates/eggpack-cli/src/main.rs133`),
`project_build_handoff` (`:451`), `validate_build_artifact_dir` (`:515`),
`reconstruct_attempt` (`:572`, `:931`), `validate_qualification_artifact_dir`
(`:628`), `run_consumer_validator` (`:790`), and
`ReleaseWorkflowShapeV1::from_json` (`:321`, `:380`).
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
