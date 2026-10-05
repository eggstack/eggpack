//! Fixture integration tests: real-layout contracts parse and expand.

use eggpack_contract::DistributionContract;

fn load(name: &str) -> DistributionContract {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).expect("fixture readable");
    DistributionContract::parse_toml_str(&text).expect("fixture valid")
}

#[test]
fn simple_direct_fixture_expands() {
    let c = load("simple-direct.toml");
    assert_eq!(c.product.id, "eggsact");
    let e = c.expand("linux-x64", "1.2.6").unwrap();
    match e.assets {
        eggpack_contract::ExpandedAssets::Direct(d) => {
            assert_eq!(d.asset_file, "eggsact-1.2.6-x86_64-unknown-linux-gnu");
            assert_eq!(d.install_name, "eggsact");
        }
        _ => panic!("expected direct"),
    }
}

#[test]
fn codegg_bundle_fixture_expands_three_members() {
    let c = load("codegg-bundle.toml");
    let e = c.expand("x86_64-unknown-linux-gnu", "0.9.0").unwrap();
    match e.assets {
        eggpack_contract::ExpandedAssets::Bundle(b) => assert_eq!(b.entries.len(), 3),
        _ => panic!("expected bundle"),
    }
}

#[test]
fn egress_archive_fixture_expands_members() {
    let c = load("egress-archive.toml");
    let e = c.expand("macos-arm64", "2.1.0").unwrap();
    match e.assets {
        eggpack_contract::ExpandedAssets::Archive(a) => {
            assert_eq!(a.members.len(), 2);
            assert!(a.archive_file.ends_with(".tar.gz"));
        }
        _ => panic!("expected archive"),
    }
}

#[test]
fn fixtures_round_trip_deterministically() {
    for name in [
        "simple-direct.toml",
        "codegg-bundle.toml",
        "egress-archive.toml",
    ] {
        let c = load(name);
        let rendered = c.to_toml_string().unwrap();
        let reparsed = DistributionContract::parse_toml_str(&rendered).unwrap();
        assert_eq!(c, reparsed, "{name}");
    }
}

#[test]
fn consumer_direct_targets_fixture_matches_the_live_consumer_shape() {
    // `consumer-direct-targets.toml` is a byte-identical copy of the adopted
    // eggsact contract at `eggstack/eggsact@d4e6e5c`
    // (`release/eggpack/distribution.toml`). It pins the exact public asset,
    // sidecar, and install names both adopted consumers publish, including
    // the contract-level `.exe` rule for `*-pc-windows-msvc` that the older
    // `eggsact-direct-targets.toml` fixture does not carry.
    let c = load("consumer-direct-targets.toml");
    assert_eq!(c.product.id, "eggsact");

    // Public asset names are version-free by contract.
    for (target, asset, install) in [
        ("linux-x64", "eggsact-x86_64-unknown-linux-gnu", "eggsact"),
        (
            "linux-arm64",
            "eggsact-aarch64-unknown-linux-gnu",
            "eggsact",
        ),
        ("macos-x64", "eggsact-x86_64-apple-darwin", "eggsact"),
        ("macos-arm64", "eggsact-aarch64-apple-darwin", "eggsact"),
        (
            "windows-x64",
            "eggsact-x86_64-pc-windows-msvc.exe",
            "eggsact.exe",
        ),
    ] {
        assert_eq!(
            c.resolve(target).unwrap().triple,
            c.resolve(target).unwrap().triple
        );
        let e = c.expand(target, "v1.2.7").unwrap();
        match e.assets {
            eggpack_contract::ExpandedAssets::Direct(d) => {
                assert_eq!(d.asset_file, asset, "{target}");
                assert_eq!(d.sidecar_file, format!("{asset}.sha256"), "{target}");
                assert_eq!(d.install_name, install, "{target}");
            }
            _ => panic!("expected direct"),
        }
    }

    // ARMv7 stays a product-recognized Cargo-fallback host and is
    // deliberately absent from the producer contract.
    assert!(c.resolve("armv7-unknown-linux-gnueabihf").is_err());
    assert_eq!(c.targets.len(), 5);
}
