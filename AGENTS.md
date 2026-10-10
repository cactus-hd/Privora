# AGENTS.md — Privora Rust

## Product scope
Privora Offline Alpha protects arbitrary files inside a versioned `.vora` container. The current release is a desktop proof of concept using the encrypted-manifest v2 layout with v1 read compatibility. Do not claim global offline activation enforcement, remote revocation, or uncrackable DRM.

## Architecture rules
- `crates/privora-core` owns container parsing, cryptographic operations, policy checks and safe extraction. It must not depend on egui, eframe, rfd or UI state.
- `apps/privora-desktop` owns UI, file selection, background workers, and the local history file.
- Do not implement cryptography manually. Use reviewed crates and authenticated encryption.
- Do not change the `.vora` layout without incrementing/defining a format version and updating `docs/FORMAT.md`. V2 encrypts the manifest separately and derives metadata/payload keys with HKDF-SHA256.
- Do not log or persist access codes in app history. The create workflow must not automatically write a plaintext secret-bearing sidecar next to `.vora`; saving `.license.txt` must be an explicit user action with a clear warning.
- Never extract paths containing parent traversal or directory separators; never overwrite existing user files silently.

## Completion checklist
Before marking a task complete:
1. `cargo fmt --all -- --check`
2. `cargo check --workspace`
3. `cargo test -p privora-core`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. For UI changes, manually open the app and verify the affected workflow.
6. For crypto/format changes, add tests for valid round-trip, malformed inputs, wrong codes and tampering.
7. Update the README and relevant protocol/security docs.

## Security boundaries
- Code is a high-entropy bearer secret: anyone with both package and code can decrypt.
- Activation limits are local to one application data profile.
- Expiry depends on the system clock.
- Extracted plaintext cannot be controlled by the app.
- This project is not security-audited. Avoid claims such as “uncrackable” or “military-grade”.

## Coding practices
- Keep UI callbacks small; large archive/crypto operations must run off the UI thread.
- Return contextual errors rather than swallowing failures.
- Keep secrets out of logs and history JSON.
- Prefer safe Rust and explicit bounds when parsing untrusted files.
