# CI/Release M003l — Producer Rule Authority Convergence Corrective

Status: **blocked** on CI M003k merge, and on ADR-0006 for the digest-validator work package.

Repository baseline: `eggstack/eggpack@fb81aa1400e1f333f6500075f78d944a9242b673` (2026-10-23).
Source roadmap: `plans/subsystems/ci-release-orchestration-roadmap.md`.
Primary class: invariant / correctness corrective.
Historical upstream authority: ADR-0002 contract/plan/manifest separation (no layer redefines another's rules); ADR-0003 checked-in generated CI and publication gate.
Affected closed milestones: CI M001 (implementation `5e79f0b91b478f766124224c1e9c6382778fef14`, introduced the CI-side host-match rule), Build/Qualification M001 (introduced core's `validate_policy`), Build/Qualification M005 (implementation `7a206ba73e986352410f94b34c7c616c1ea26019`, introduced the CI-side SHA-256 hex validator), CI M003a (implementation `36f1cc1c96cf72511bbdadc2c480cdd0934a4fc9`, introduced the GitHub-side SHA-256 hex validator and the GitHub-side file digest), CI M003d (implementation `946dd7a827fcbf7552721ed162972ae5ae78b5f2`, introduced the CI-side file digest and the CI-side exact-tag validator), CI M003i (implementation `3ddac9d84fc2e10b8451d38672f66b02e06724df`, introduced core's git tag path validation). All closures remain historical and are not rewritten.
Dependency: **CI M003k** must be merged first — it is in flight (PR `eggstack/eggpack#1`) and modifies `crates/eggpack-ci/src/lib.rs`. This milestone also requires **Build/Qualification M007** for the canonical file-digest helper that `eggpack-ci` and `eggpack-github` consume.

## 1. Objective

Make `eggpack-core` the single authority for the producer rules that
`eggpack-ci` currently re-derives, and remove the divergent second definitions
of those rules.

After this milestone:

- host/target matching has one implementation;
- compatibility-floor applicability has one implementation;
- exact-tag safety has one implementation, whose rule set is the union of the
  safety concerns that today exist in two partial forms;
- the SHA-256 hex acceptance set has one implementation, per ADR-0006;
- the file digest has one implementation, per Build/Qualification M007.

## 2. Current evidence and the defects

### 2.1 Host/target matching is implemented twice

| Site | Location | Visibility |
|---|---|---|
| core | `crates/eggpack-core/src/lib.rs:84-96` `host_matches_target` | private |
| ci | `crates/eggpack-ci/src/lib.rs:423-435` `host_matches_target` | private |

The two are semantically identical — same architecture prefixes
(`x86_64-`, `aarch64-`, `armv7-`), same OS predicates (`-linux-`,
`-apple-darwin`, `-windows-`) — and differ only in local variable names. The CI
copy exists because the core copy is private.

This rule is the **host-matched native qualification** gate that ADR-0005
established. It now has two implementations, one of which is a copy. The CI
copy is the one that gates the generated workflow, so a future change to the
core copy would leave the workflow gating on the stale rule.

### 2.2 Compatibility-floor applicability is implemented twice

| Site | Location |
|---|---|
| core | `crates/eggpack-core/src/lib.rs:157-165`, inside `validate_policy` |
| ci | `crates/eggpack-ci/src/lib.rs:316-320` and `:337-338`, inside `CIPlan::validate` |

Both derive the same rule: `Glibc` requires a `-linux-gnu` triple, `Macos`
requires an `-apple-darwin` triple, and a `Macos` floor is incompatible with
`CargoZigbuild`. The CI copy additionally re-derives the toolchain-validity
predicate and the native-qualification host check at
`crates/eggpack-ci/src/lib.rs:299-331`.

`CIPlan::validate` is a *second* policy validator, not an independent
opinion. It exists because `validate_policy` is private.

### 2.3 Exact-tag safety has two partial, non-overlapping rule sets

| | core | ci |
|---|---|---|
| Site | `crates/eggpack-core/src/builder.rs:427-441` | `crates/eggpack-ci/src/lib.rs:4940-4954` |
| Introduced by | CI M003i (`7c20b68`) | CI M003d (`946dd7a`) |
| Empty / > 128 / control char | rejected | rejected |
| `?`, space, `\` | rejected | rejected |
| `..` | rejected | rejected |
| Leading / trailing `/` | rejected | rejected |
| `#`, `@`, `'`, `"`, `` ` ``, `$` | **allowed** | rejected |
| trailing `.`, `@{`, `^`, `:`, `*`, `[`, `~` | rejected | **allowed** |
| empty, `.`-leading, or `.lock` path component | rejected | **allowed** |

**Neither rule set is a superset of the other.** A tag containing `$` or `#`
is refused by the CI path and accepted by the core git-resolution path. A tag
containing `~` or a `.lock` component is refused by the core path and accepted
by the CI path.

The two rules were written for **different concerns**, and both concerns are
legitimate:

- core's rule guards a tag used as a **git ref path** in
  `git rev-parse --verify refs/tags/<tag>^{commit}`
  (`crates/eggpack-core/src/builder.rs:442-451`);
- ci's rule guards a tag arriving from a **workflow dispatch input**, which
  M003j made a data value in a shell environment
  (`plans/implementation/ci-release-orchestration/003j-dispatch-tag-shell-injection-corrective.md`).

The defect is not that the concerns differ. It is that neither component holds
the **union**, so a tag safe under one concern is accepted without the other
concern's checks. The correct outcome is one canonical *safety* validator
carrying the union of both concern sets, applied wherever an exact tag is
handled, with the separate *canonical-format* check (M003j's `vX.Y.Z` form)
kept as a distinct, named concern layered on top.

### 2.4 The SHA-256 hex acceptance set has three implementations

| Site | Location | Rule |
|---|---|---|
| manifest | `crates/eggpack-manifest/src/lib.rs:307-322` | 64 lowercase hex |
| github | `crates/eggpack-github/src/lib.rs:567-576` | 64 lowercase hex |
| ci | `crates/eggpack-ci/src/lib.rs:632-642` | 64 lowercase hex, and not all-zeros |

The all-zeros digest is refused by CI and accepted by the manifest and by
`eggpack-github` — the crate that computes and records digests over the bytes
actually staged into a draft release. The decision on which rule wins is
**ADR-0006**, which is proposed, not accepted.

### 2.5 The file digest has four implementations

| Site | Location | Notable |
|---|---|---|
| core | `crates/eggpack-core/src/lib.rs:291-324` | no post-read re-check |
| core | `crates/eggpack-core/src/finalization.rs:482-512` | post-read re-check |
| github | `crates/eggpack-github/src/lib.rs:597-628` | post-read re-check |
| ci | `crates/eggpack-ci/src/lib.rs:4663-4695` | returns `(0, String::new())` on error; unchecked `size +=` |

The CI copy is the weakest: it converts every failure — unreadable file,
symlink, empty file, read error — into `(0, String::new())`, and a caller
comparing against the empty string cannot distinguish "no digest" from "no
file". It is also the only one using unchecked size accumulation.

Build/Qualification M007 establishes the canonical helper in core; this
milestone makes CI and GitHub consume it.

## 3. Invariants

- **One rule, one owner.** A producer rule has exactly one implementation. A
  downstream crate that needs the rule calls the owner's function; it does not
  restate the rule.
- `eggpack-core` owns producer policy validation. `eggpack-ci` owns workflow
  projection and rendering. `eggpack-ci` validates a `CIPlan` against the
  producer's own rules by **calling** those rules, not by re-deriving them.
- Exact-tag handling is **fail-closed**: a tag is accepted only if it passes
  every applicable safety concern. The canonical safety validator carries the
  union of the git-ref-path concerns and the shell-metacharacter concerns.
- The canonical **format** check (M003j's `vX.Y.Z` form) remains a separate,
  named concern, distinct from safety. A tag that is safe but not canonical is
  rejected by the format check, not by the safety check, and the diagnostic
  says which one failed.
- The generated workflow's behaviour is **unchanged** by this milestone. Every
  tag the current pipeline accepts is still accepted, and every tag it refuses
  is still refused, with the same pass/fail outcome. Only the *rule location*
  changes. The one intended difference is that a tag now satisfies the union
  of both safety concerns in both places — this can only refuse more, never
  accept more.
- No public schema changes. `ReleaseManifest` v1, `CIPlan` v1, `PackConfig`
  v1, and `DistributionContract` v1 are unchanged, except where ADR-0006
  explicitly authorizes a change to the manifest's digest acceptance set.
- The CI crate's process execution is **not** changed by this milestone. That
  is CI M003m.
- No new dependency. `eggpack-ci` already depends on `eggpack-core`, and
  `eggpack-github` already depends on `eggpack-core`.

## 4. Scope / non-goals

**In scope:** `crates/eggpack-ci/src/lib.rs`, `crates/eggpack-github/src/lib.rs`,
and the minimal `crates/eggpack-core/**` visibility changes needed to expose
the canonical rules.

**Out of scope:**

- CI M003m (process execution and bounded reads) and CI M003n (drift-check and
  staging-payload duplication). Those are separate milestones in this chain.
- Build/Qualification M007's implementation of the canonical digest helper
  and Build/Qualification M008's floor verification. This milestone only
  *consumes* them.
- Any change to the generated workflow's bytes, job graph, permissions, or
  trigger handling.
- Any change to `ReleaseManifest` v1's fields or `schema_version`.
- Any change to the M003j canonical-stable-tag format rule, beyond separating
  it from safety.
- Any new dependency, any new schema, any new command.

## 5. Required production changes

### 5.1 Expose the canonical rules from core

Make the following callable from `eggpack-ci` and `eggpack-github`:

1. the host/target match predicate (currently
   `crates/eggpack-core/src/lib.rs:84`), exposed so `CIPlan::validate` can call
   it instead of using its private copy;
2. the policy validation predicate (currently `validate_policy`,
   `crates/eggpack-core/src/lib.rs:118`), exposed in a form that `CIPlan::validate`
   can apply to a `TargetPolicy` and target triple;
3. the canonical exact-tag **safety** validator — new in core, carrying the
   union of the two existing concern sets;
4. the canonical SHA-256 hex validator — new or shared, per ADR-0006;
5. the canonical file-digest helper — delivered by Build/Qualification M007.

The exposed API must be small and named for what it owns. It is not a general
"validation" entry point.

### 5.2 Replace the CI copies

- `crates/eggpack-ci/src/lib.rs:423-435` is deleted; `CIPlan::validate` calls
  the core predicate.
- `crates/eggpack-ci/src/lib.rs:316-320` and `:337-338` are deleted; the floor
  and strategy rules come from the core policy validation.
- `crates/eggpack-ci/src/lib.rs:4663-4695` is deleted; candidate hashing uses
  the canonical helper and returns a typed error instead of `(0, String::new())`.
- `crates/eggpack-ci/src/lib.rs:632-642` is deleted or reduced to a
  re-export, per ADR-0006's decision.
- `crates/eggpack-github/src/lib.rs:597-628` is deleted; the staging digest uses
  the canonical helper.
- `crates/eggpack-github/src/lib.rs:567-576` is deleted or reduced to a
  re-export, per ADR-0006's decision.

Every one of these deletions must be verified by grep: no second
implementation of the rule remains in the workspace.

### 5.3 The canonical exact-tag safety validator

The union of the two existing concern sets, as one function:

- empty; length bound; control characters — from both;
- `?`, space, `\` — from both;
- `..`; leading and trailing `/` — from both;
- `#`, `@`, `'`, `"`, `` ` ``, `$` — from the CI concern;
- trailing `.`; `@{`; `^`, `:`, `*`, `[`, `~` — from the core concern;
- path components that are empty, begin with `.`, or end with `.lock` — from
  the core concern.

The validator is **safety only**. The M003j canonical-format check remains a
separate function with a separate diagnostic.

One consequence must be stated explicitly rather than discovered later: the
core git-resolution path will now refuse tags containing `#`, `@`, `'`, `"`,
`` ` ``, or `$` that it previously accepted. This is fail-closed and correct —
those characters are special to git and to shells — but it is a behaviour
change on a path that is currently exercised by live consumer releases. The
closure record must list it and confirm that no in-tree tag, workflow input,
or consumer configuration uses such a tag.

### 5.4 Diagnostic compatibility

Diagnostics are single-line and bounded. Where a rule moves from one crate to
another, the **message text** should be preserved where it is already relied
on, and any change must be listed in the closure record. The `ci _*` runner
protocol carries messages to CI logs that consumers may match on; this
milestone must not silently change a message a consumer could be grepping.

## 6. Ordered work packages

### WP1 — Failing discriminating tests first

- **Rule identity:** for a matrix of target triples and host requirements, the
  core predicate and the CI predicate must agree on every case. Write the
  matrix test against the *core* predicate only, so that deleting the CI copy
  is proven not to change any outcome.
- **Floor applicability:** a `Glibc` floor on a non-GNU/Linux triple, a
  `Macos` floor on a non-Darwin triple, and a `Macos` floor under
  `CargoZigbuild` are each still refused, with the same pass/fail outcome.
- **Tag safety union:** a table-driven test asserting that every character set
  refused by either current validator is refused by the canonical validator,
  and that the canonical validator's acceptance set is a **subset** of the
  union of the two current acceptance sets — that is, it accepts nothing that
  either current validator refused.
- **Digest error typing:** the CI candidate-hashing path now returns a typed
  error for a missing file, a symlink, an empty file, and an unreadable file.
  The test asserts no path yields an empty digest string.
- **SHA acceptance:** per ADR-0006's decision, the shared validator's
  acceptance set is asserted identically from all three call sites.

### WP2 — Expose the canonical rules from core

Add the small public surface in §5.1, with doc comments naming the owner and
the rule each function owns.

### WP3 — Replace the CI copies

Delete the five CI-side implementations and route the call sites to core.
Update `CIPlan::validate` to apply the core policy validation rather than
re-deriving it.

### WP4 — Replace the GitHub copies

Delete the GitHub-side digest and SHA validator and route to core.

### WP5 — Verification and documentation

Run the full gate. Update `architecture/ci.md`, `architecture/github.md`,
`architecture/core.md`, `architecture/validation-model.md`,
`crates/eggpack-ci/README.md`, `crates/eggpack-github/README.md`, the CI and
build/qualification roadmaps, and the registry. The validation-model document
in particular must state, once, where each producer rule lives.

## 7. Failure, restart and contention semantics

- Every rule change fails closed. If the canonical validator cannot be reached
  or returns an error, the enclosing operation fails; it never degrades to a
  permissive path.
- The canonical candidate digest returns a typed error. There is no sentinel
  success value, no empty digest, and no zero-size placeholder.
- These validators are pure functions over their inputs. They hold no lock and
  perform no I/O, so concurrent callers are independent and deterministic.
- A partially-applied change is detectable: the workspace grep for each
  retired symbol must return zero matches. The closure record must include
  that grep's output as evidence.
- Restarting is free: the rules are pure.

## 8. Compatibility and migration

- **No generated workflow byte changes.** The rules now live in a different
  crate, but their outcomes are identical, so the rendered YAML is identical
  and `eggpack ci check` drift status is unchanged for every consumer.
- The one intended behaviour change is the exact-tag safety union described in
  §5.3, which can only refuse more. The closure record must confirm that no
  in-tree tag, dispatch input, or consumer configuration uses a character in
  the newly-refused set.
- No schema change, except the ADR-0006-authorized narrowing of the manifest's
  digest acceptance set, which ships as a `eggpack-manifest` patch release.
- No new dependency. `eggpack-ci` and `eggpack-github` already depend on
  `eggpack-core`.
- `eggpack-core` is not published, so widening its public surface has no
  external compatibility consequence.
- `eggup-eggpack` is affected only by ADR-0006's manifest narrowing, and only
  if it persists a placeholder digest.

## 9. Required tests

- the rule-identity matrix described in WP1, over a representative set of
  triples, hosts, floors, and strategies;
- the floor applicability negatives;
- the tag-safety union table, including every character class named in §5.3;
- a positive test that a canonical `v1.2.3` tag and a multi-segment tag such
  as `release/v1.2.3` are still accepted, so the union is not over-tight;
- typed-error tests for the candidate digest path;
- the SHA acceptance tests, per ADR-0006;
- the full existing CI and GitHub suites, unchanged, proving no behavioural
  regression;
- a golden-workflow test proving the rendered YAML is byte-identical before and
  after the change.

## 10. Verification commands

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test -p eggpack-github --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
cargo +1.89.0 test -p eggpack-github --all-targets --locked
./scripts/check-local.sh
```

Add a direct narrowed run for the new tests, e.g.
`cargo test -p eggpack-ci --all-targets --all-features --locked <new_test_name> -- --nocapture`.
Append `-- --test-threads=1` when diagnosing Windows serialization.

Additionally, the closure record must include the grep output proving each
retired symbol has zero remaining definitions.

## 11. Documentation updates

- `architecture/ci.md` — the CI crate consumes core's rules; the retired
  copies are gone.
- `architecture/github.md` — the staging digest uses the canonical helper.
- `architecture/core.md` — the canonical rule surface.
- `architecture/validation-model.md` — a single statement of where each
  producer rule lives.
- `crates/eggpack-ci/README.md`, `crates/eggpack-github/README.md`.
- `architecture/overview.md` — line counts and baseline SHA are already stale
  and are **not** re-based here; see
  `plans/implementation/planning-hygiene/003-post-implementation-review-baseline-reconciliation.md`.
- `plans/subsystems/ci-release-orchestration-roadmap.md` and
  `plans/subsystems/build-qualification-roadmap.md` — the M003l row.

## 12. Acceptance criteria

1. Exactly one implementation of each of the five rules exists in the
   workspace: host/target match, floor applicability, exact-tag safety, SHA-256
   hex acceptance, and file digest. Grep evidence is in the closure record.
2. `CIPlan::validate` applies the core policy validation rather than
   re-deriving it.
3. The canonical exact-tag safety validator refuses every character class that
   either current validator refused, and accepts nothing that either refused.
4. The CI candidate-digest path returns a typed error and never an empty
   digest string; it uses overflow-checked size accumulation.
5. The rendered workflow YAML is byte-identical before and after the change,
   proven by a golden test.
6. The SHA-256 acceptance set is identical at all three call sites, as decided
   by ADR-0006.
7. No schema change beyond what ADR-0006 authorizes; no new dependency; no
   change to the CI crate's process execution.
8. The full workspace gate and `./scripts/check-local.sh` are green, and hosted
   CI is green on all lanes.
9. `git diff --check` is clean.

## 13. Stop conditions

Stop and re-plan if:

- unifying the rules changes any tag acceptance outcome other than the
  intended fail-closed narrowing in §5.3;
- any rendered workflow byte changes;
- the core predicates cannot be exposed without widening `eggpack-core`'s
  public surface in a way that would become a new public contract;
- ADR-0006 is not accepted — the SHA-acceptance work package must be dropped
  and the milestone re-scoped, not guessed at;
- Build/Qualification M007 is not closed, since the digest consolidation
  depends on the canonical helper it delivers;
- a diagnostic message that a consumer could be grepping would change without
  that change being explicitly recorded and accepted;
- the change would require a new dependency, a new schema, or a new command.

## 14. Closure evidence

Write `plans/closure/ci-release-orchestration/003l-status.md` with: the
reviewed baseline and implementation SHA; the rule-ownership table before and
after; the rule-identity matrix results; the tag-safety union table and the
explicit list of newly-refused character classes with confirmation that no
in-tree or consumer tag uses them; the typed-error evidence for the digest
path; the golden-workflow byte-identity proof; the grep output proving each
retired symbol is gone; the verification commands executed and their actual
results; hosted CI run IDs and the lane matrix; the ADR-0006 disposition and
the `eggpack-manifest` release, if applicable; an invariant review against §3;
unresolved findings by severity; and the CI M003m and M003n unblock.

Prior CI M001, M003a, M003d, and M003i closure records remain historical and
unchanged.

## 15. Handoff notes

- Start from the CI M003k merge SHA and the Build/Qualification M007 closure
  SHA. **Do not** implement against `fb81aa1400e1f333f6500075f78d944a9242b673`
  while the M003k PR is open.
- ADR-0006 must be accepted before the SHA-acceptance work package starts.
  If it is accepted with Option B rather than Option A, the manifest's
  acceptance set stays permissive and the shared validator is used only on the
  producer-internal paths — do not assume Option A.
- The canonical exact-tag **safety** validator is a new artifact. Give it a
  name that says "safety", not "tag validation", so that M003j's canonical
  *format* check stays visibly separate.
- This milestone does not touch process execution. CI M003m does, and it
  depends on this milestone landing first.
