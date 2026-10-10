$ErrorActionPreference = "Stop"
cargo fmt --all -- --check
cargo check --workspace
cargo test -p privora-core
cargo clippy --workspace --all-targets -- -D warnings
