# Skills

Task-shaped guidance for agents working in this repository. Each skill answers one
recurring question and routes you to the normative document instead of restating it.

| Skill | Use it when |
|---|---|
| [pre-submit-gate](pre-submit-gate/SKILL.md) | Finishing a change, before committing, or asked "is this ready to submit?" |
| [architecture-routing](architecture-routing/SKILL.md) | Before reading source — which deep dive is normative, which crate owns which boundary, and whether a suspected bug is doc drift |
| [planning-and-closure](planning-and-closure/SKILL.md) | Opening, planning, or closing a milestone; correcting a closed one; updating the registry |
| [generated-ci-and-draft-staging](generated-ci-and-draft-staging/SKILL.md) | Touching `ci generate`/`ci check`, the `ci _*` runner commands, the file-in/file-out seam, or draft staging/publication |
| [contract-and-manifest-surface](contract-and-manifest-surface/SKILL.md) | Adding or changing a target, asset form, install name, or either schema-v1 document |

## How these relate to the rest of the repo

- `AGENTS.md` is the always-loaded operating contract: commands, invariants, and
  planning conventions. Read it first.
- `architecture/` holds the normative design deep dives. The skills route you to
  the right one; the deep dive is what you cite.
- `plans/registry.md` is the active planning control surface. A skill will
  sometimes tell you to check it *before* opening work, because a plan may
  already name the thing you just noticed.
- `plans/003-planning-process.md` is normative for process, and
  `architecture/planning-and-governance.md` narrates it.

Skills are **derived** material. Where a skill and `AGENTS.md` or an
`architecture/` deep dive disagree, the deep dive is right — fix the skill.
Skills carry no invariant of their own. If you find yourself needing a new rule,
put it in `AGENTS.md` or the owning deep dive, then make the skill point at it.

There is deliberately no `opencode.json` or other agent-runtime config in this
repository. `AGENTS.md` plus `architecture/overview.md` is the whole entry path.
