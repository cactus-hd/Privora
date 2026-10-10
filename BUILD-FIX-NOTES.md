# Build fix notes

This hotfix addresses the compile errors seen with eframe/egui 0.36.2:

- `Context::style()` / `Context::set_style()` were replaced by the current theme-aware APIs.
- Integer literals passed into custom icon coordinate closures were explicitly made floating-point literals.

Please verify locally with:

```powershell
cargo fmt --all
cargo check --workspace
cargo test -p privora-core
cargo run --release -p privora-desktop
```

Warnings and errors must be reviewed from the actual local build. This archive has not been compiled in the packaging environment because Rust/Cargo are not installed there.
