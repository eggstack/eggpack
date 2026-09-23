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

The crate is synchronous and side-effect free. It has no network, process execution, archive extraction, installer, release-selection, publication, or service-management responsibilities. Schema-v1 behavior and conformance behavior were fidelity-ported from the qualified unpublished `eggup-dist` predecessor at Eggup M002 corrective commit `0a68f29fce44adf5f12d79f1b440a2c08aca9cb7` and closed M003 commit `9941c58d7039410c728860f9e4e382881d4ccf54`.

## Pure conformance checks

`expected_release_files` expands the contract for a caller-selected target/version and returns labeled required asset and sidecar names. Callers pass filenames they already obtained to `ReleaseInventory`; the validator does not discover releases or inspect files. `ExtrasPolicy::AllowExtras` is the default, while `Exact` reports undeclared entries.

```rust
use eggpack_contract::{
    expected_release_files, validate_release_inventory, DistributionContract, ExtrasPolicy,
    ReleaseInventory,
};

let contract = DistributionContract::parse_toml_str(include_str!(
    "tests/fixtures/simple-direct.toml"
))?;
let expected = expected_release_files(&contract, "linux-x64", "1.2.6")?;
let observed = ReleaseInventory::new(expected.iter().map(|file| file.file_name.clone()))?;
let report = validate_release_inventory(&expected, &observed, ExtrasPolicy::Exact);
assert!(report.is_conformant());
# Ok::<(), Box<dyn std::error::Error>>(())
```

For archives, callers supply a listing to `ArchiveMemberInventory`; Eggpack checks required paths but never opens or extracts the archive. Consumer tests can serialize `ObservedTargetMapping` values as TOML from their own runtime/bootstrap mapping code and pass them to `validate_observed_mapping`. Eggpack compares these facts with the canonical expansion and does not parse shell, PowerShell, Rust, or workflow sources. Findings are typed, sorted, bounded, and available through `ConformanceReport`.
