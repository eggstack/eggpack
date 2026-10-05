---
name: generated-ci-and-draft-staging
description: Work on Eggpack's generated release workflow, the ci generate/check drift contract, the nine internal ci _* runner commands, the build/qualification handoff file names, or GitHub draft staging and publication. Use when changing renderer output, renaming a handoff file, editing a checked-in workflow, or touching anything that could publish or clobber a release.
---

# Generated CI and draft staging

Two hard boundaries meet here. Generated workflow YAML is **derived output** — the
renderer is the authority, not the YAML. Staging is **draft-only** — publication
is a separate human action and always will be.

## Never hand-edit the generated workflow

If you need different YAML, change the input or the renderer, then regenerate and
prove the result:

```bash
eggpack ci generate --ci-plan <plan.json> --github-policy <policy.json> --output <workflow.yml>
eggpack ci check   --ci-plan <plan.json> --github-policy <policy.json> --workflow <workflow.yml>
```

Two mutually exclusive input modes, and supplying both is an error:

| Mode | `generate` inputs | Renders |
|---|---|---|
| exact | `--ci-plan` | `render_release_github` — a full release graph |
| reusable | `--workflow-shape` + `--contract` | `render_reusable_release_github` — a reusable workflow; release identity resolves at run time |

`ci generate` writes **exactly one file**, the `--output`, through `atomic_write`.
No directory creation, no second write, so a generate run cannot touch anything
the caller did not name. The output path is symlink-rejected and its parent must
already be a real directory — a missing parent is an error, not something it
creates for you.

`ci check` **never writes**: no `atomic_write`, no `fs::write`, no
`create_dir_all` in that path. Its only success effect is one `println!`. That is
what makes it safe in pre-merge or scheduled CI — a red run cannot have mutated
the tree it inspected. It reads the workflow bounded (8 MiB), refuses symlinks,
and compares with **CRLF→LF normalization only**. There is no fuzzy matching, so
a genuinely equivalent-but-reformatted file is reported as drift. Drift reports
as:

```text
eggpack: ci check: drift detected (expected N bytes, found M bytes, first difference at Some(N))
```

Verify the pipeline end to end with the worked example in
[docs/quickstart.md](../../docs/quickstart.md) — every command and expected output
there has been run verbatim.

## The `ci _*` commands are wrappers, not a scripting interface

Nine internal commands exist so a rendered step is a fixed-shape file-in/
file-out call: `_verify-source`, `_resolve-release`, `_capture-build`,
`_qualify-target`, `_validate-consumer`, `_evaluate-gate`, `_aggregate`,
`_prepare-stage`, `_stage-github-draft`. Each is documented in
[cli.md](../../architecture/cli.md) §"The nine internal `ci _*` runner commands"
with its exact reads and writes.

The input grammar is: paths, a target name that must already exist in the plan,
and nothing else. There is **no shell invocation, no caller-supplied program
name, no expression evaluation, and no flag whose value is interpreted as a
command**. If you find yourself wanting to pass a command through one of these,
the design is telling you the capability belongs somewhere else.

Two user-facing commands exist: `ci generate` and `ci check`. Exit 0 on success;
exit 1 with a single-line `eggpack: <message>` on failure.

## The handoff file names are a cross-process contract

Generated CI does not call library functions. It shells out to the `eggpack`
binary, and the two sides agree on fixed names and JSON documents —
`build-handoff.json`, `candidates/`, `evidence.json`, `gate-outcome.json`,
`consumer-evidence.json`, and the `eggpack-runtime/` set.

**Changing a name, a JSON shape, or a validation rule here is a change to the
contract between two independently versioned processes.** It will fail at CI
*runtime*, not at compile time — one process writes a document the other no longer
accepts. When you change one, change the encode/decode pair together and
exercise the end-to-end orchestration tests, not just the unit tests.

`release-manifest.json` goes **beside** the finalized output root, never inside
it. The finalizer writes only into a new output root under a caller-secured
parent.

## Draft staging: what it will never do

Staging targets a **draft**, on the **exact existing tag**. It does not create a
tag, does not move one, does not publish, does not auto-publish, and has no
`--clobber`. A rerun that finds a same-name remote asset with a different digest
**fails closed** rather than overwriting. Reruns of an unchanged release reuse the
existing draft and re-reconcile the exact remote asset set.

That no-clobber refusal is a feature that has already caught a real problem, and
it is the reason integrity evidence is trustworthy. When a candidate consumer
reconciles its policy against Eggpack's, and a draft-rerun path in that consumer
uses `--clobber`, **do not weaken the Eggpack policy to match it** — that is
recorded as an explicit open item in the registry for the Eggsearch preflight.
Report the difference; let a human decide.

Presentation and wrapper files are **generated by default**, not hand-written. If
you are hand-authoring an installer presentation or product wrapper, stop and
check whether the generator is supposed to produce it.

## Credentials

The token is read from the environment at the staging step and is never echoed.
Captured process output is never returned to callers — a failure reports a
classification, not the contents. Keep it that way; redaction is enforced by
tests.

## Reference

- [architecture/ci.md](../../architecture/ci.md) — projection model, job vocabulary, gates
- [architecture/ci-rendering.md](../../architecture/ci-rendering.md) — renderers, policy, drift
- [architecture/ci-consumer-seam.md](../../architecture/ci-consumer-seam.md) — handoffs, validator, runtime identity
- [architecture/github.md](../../architecture/github.md) — the `GithubApi` seam, reconciliation, redaction
- [architecture/cli.md](../../architecture/cli.md) — commands, file-safety helpers, exit discipline
- [docs/quickstart.md](../../docs/quickstart.md) — verified worked example
