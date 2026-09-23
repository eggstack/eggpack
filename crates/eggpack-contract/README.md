# eggpack-contract

`eggpack-contract` defines the portable schema-v1 release layout shared by Eggpack producer tooling and, where useful, Eggup consumers. It describes release targets, aliases, asset names, checksum sidecars, archive members, and installed names. It does not build, publish, download, verify, or extract artifacts.

## Schema v1

Every document declares `schema_version = 1`. Unknown fields and unsupported schema versions fail explicitly.

```toml
schema_version = 1

[product]
id = "eggsact"

[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = ["linux-x64"]

[targets.asset]
kind = "direct"
asset = "{product}-{version}-{target}"
install = "{product}"

[targets.checksum]
sidecar = "{asset}.sha256"
```

The three fixed asset forms are `direct` (one asset/member), `bundle` (several sibling assets), and `archive` (one archive with explicitly named members). Templates accept only `{product}`, `{version}`, `{target}`, and `{alias}`; checksum templates additionally accept `{asset}`. Expansion is data substitution, never executable templating.

Expanded release filenames and install names must be flat and unique under exact and ASCII-case comparison. Archive member sources are literal normalized relative paths; absolute paths, traversal, backslashes, drive prefixes, and empty or dot components are rejected. Checksums describe integrity metadata, not authenticity.

The crate is synchronous and side-effect free. It has no network, process execution, archive extraction, installer, release-selection, publication, or service-management responsibilities. Schema-v1 behavior and representative fixtures were imported from the qualified unpublished `eggup-dist` predecessor at Eggup M002 corrective commit `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7`.
