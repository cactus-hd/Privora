# Changelog

## 0.2.0 — Offline alpha refresh

- Replaced Unicode pictogram characters with small vector-drawn icons to avoid missing-glyph squares.
- Improved typography using a platform font when available, larger base text styles, and more comfortable spacing.
- Added a Copy access code action and kept secret-bearing license notes as an explicit save operation.
- Introduced `.vora` v2 (`PRVORA02`) with an AES-256-GCM encrypted manifest and separate HKDF-SHA256 metadata/payload keys.
- Kept read compatibility for legacy `.vora` v1 (`PRVORA01`) packages. V1 metadata remains plaintext.
- Added round-trip, chunked payload, metadata confidentiality, wrong-code, expiry and tamper-oriented tests.
- Documented that the local activation counter, system-clock expiry, local plaintext metadata history and temporary plaintext ZIP files are limitations of this offline alpha.

### Verification status

This archive has been structurally checked, but this environment did not have a Rust toolchain available to run `cargo check` or `cargo test`. Run the checks in `README.md` on a machine with the specified Rust toolchain before relying on the build. This alpha has not undergone an independent security audit.

## Build compatibility hotfix

- Updated egui 0.36 theme styling calls to use `style_of(Theme::Dark)` and `set_style_of(Theme::Dark, ...)`.
- Made custom vector icon coordinate literals explicitly `f32` so Rust type inference accepts the painter closures and coordinate arrays.
