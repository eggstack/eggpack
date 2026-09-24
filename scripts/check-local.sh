#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test -p eggpack-ci --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-contract --locked
cargo tree -p eggpack-manifest --locked
cargo tree -p eggpack-core --locked
cargo tree -p eggpack-bootstrap --locked
cargo tree -p eggpack-ci --locked
cargo package -p eggpack-contract --locked --allow-dirty
cargo package -p eggpack-manifest --locked --allow-dirty
cargo package -p eggpack-core --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo package -p eggpack-bootstrap --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"'
cargo package -p eggpack-ci --locked --allow-dirty \
  --config 'patch.crates-io.eggpack-contract.path="crates/eggpack-contract"' \
  --config 'patch.crates-io.eggpack-manifest.path="crates/eggpack-manifest"' \
  --config 'patch.crates-io.eggpack-core.path="crates/eggpack-core"'
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-contract --all-targets --locked
cargo +1.89.0 test -p eggpack-manifest --all-targets --locked
cargo +1.89.0 test -p eggpack-core --all-targets --locked
cargo +1.89.0 test -p eggpack-bootstrap --all-targets --locked
cargo +1.89.0 test -p eggpack-ci --all-targets --locked
