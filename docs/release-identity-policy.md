# Release tag and manifest identity policy

Reusable release workflows normally use the exact Git tag as the manifest
`release_id`. This keeps the historical behavior for opaque tags and existing
Eggpack consumers.

A product whose release tag is `v1.2.3` and manifest/package version is `1.2.3`
can opt into the finite stable-version mapping by adding this field to its
checked-in GitHub policy JSON:

```json
{
  "release_identity_mode": "v_prefixed_stable_semver"
}
```

This is a field added to the normal `GitHubPolicy` document, alongside its
runner, action, trigger, release-input, and staging settings. The mode is read
while rendering the checked-in workflow and emitted as a literal resolver
argument. It is not a `workflow_dispatch` input. For this opt-in, configure
exactly one matching event source:

- `staging.tag_source = "ref_name"` with the `push` trigger; or
- `staging.tag_source = "dispatch_input"` with the `workflow_dispatch` trigger.

The accepted tag grammar is exactly `vMAJOR.MINOR.PATCH`, where each component
is an ASCII decimal integer fitting in `u64`, with no leading zeros except the
single digit `0`. Pre-release labels, build metadata, suffixes, extra
components, whitespace, path characters, and Unicode lookalikes are rejected.

For `v1.2.3`, Eggpack keeps three facts distinct:

- source tag: `v1.2.3` (checkout, source ref, draft, and installer download URL);
- source revision: the exact 40-character commit peeled from that tag;
- manifest `release_id`: `1.2.3` (contract expansion and final artifact names).

Mapped runtime policy carries all three facts. Each mapped-mode job checks the
event-selected tag against the downloaded policy, checks the policy-derived ID
against the release plan, and verifies both `HEAD` and the local tag peel to the
same revision. Finalization and staging retain the manifest ID while draft
reconciliation retains the exact tag. Legacy exact-tag policy files omit the
field and keep their prior serialized identity and workflow output.

The mapping does not change ReleaseManifest v1, authenticate bytes, sign
artifacts, move tags, overwrite immutable assets, or publish releases. Draft
publication remains a separate human action.
