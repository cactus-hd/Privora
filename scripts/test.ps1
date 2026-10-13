$ErrorActionPreference = "Stop"
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
