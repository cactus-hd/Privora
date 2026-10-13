#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace
cargo build --release -p privora-desktop
printf '\nExecutable: %s\n' "$(pwd)/target/release/privora-desktop"
