Privora v0.4 build fix

Fixes Rust borrow-checker errors E0500/E0501 in the Settings page.
The two vertical gaps inside ui.columns(2, ...) now use cols[0].add_space(...)
and cols[1].add_space(...), rather than borrowing the parent `ui` while the
columns closure already holds a mutable borrow.

From the repository root, run:
  cargo fmt --all
  cargo check --workspace
  cargo test --workspace
  cargo run --release -p privora-desktop

This source correction has been inspected and the archive verified. It has not
been compiled in the packaging environment; successful build still needs to
be confirmed on the user's Rust toolchain.
