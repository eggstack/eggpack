#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p eggpack-contract --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo doc --workspace --no-deps --locked
cargo tree -p eggpack-contract --locked
cargo package -p eggpack-contract --locked --allow-dirty
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test -p eggpack-contract --all-targets --locked
