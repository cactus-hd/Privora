# Changelog

## 0.4.0 — UI and preferences refresh

- Reworked the app shell, navigation, hero panel, page spacing and top bar toward a calmer Apple-inspired utility aesthetic.
- Added native accent palettes (Violet, Blue, Mint, Rose) and Midnight/OLED Black appearance preferences.
- Added global interface scale, compact spacing and animation controls.
- Added confirmation before decryption, post-unlock access-code clearing, remember-folder control, activity-history control and full-path display preference.
- Added explicit save/reset actions in Settings.
- Added a safe activity-clear action that preserves local unlock counters.
- Extended local-state preferences with defaults so existing v0.3 profiles remain readable.

## 0.3.0 — UX and offline license policy

- Refined navigation, page hierarchy, typography, spacing, empty states, and license search while retaining Privora's dark violet identity.
- Added offline license policy fields: start date, expiry date, recipient label, and an encrypted private note.
- Clarified that recipient labels are descriptive only and do not prove identity.
- Expanded core tests to check that recipient labels, notes, and policy metadata are absent from the public `.vora` v2 bytes.
- Added Windows installer packaging with Inno Setup, a PowerShell build helper, and step-by-step Windows/Android platform notes.
- Added a size-conscious release profile using thin LTO, one codegen unit, and symbol stripping.
- Documented Android blockers: desktop file dialogs, Activity setup, and Android Storage Access Framework integration must be implemented before the current app can ship as an APK.

## 0.2.0 — Offline alpha refresh

- Replaced Unicode pictogram characters with small vector-drawn icons to avoid missing-glyph squares.
- Improved typography using a platform font when available, larger base text styles, and more comfortable spacing.
- Added a Copy access code action and kept secret-bearing license notes as an explicit save operation.
- Introduced `.vora` v2 (`PRVORA02`) with an AES-256-GCM encrypted manifest and separate HKDF-SHA256 metadata/payload keys.
- Kept read compatibility for legacy `.vora` v1 (`PRVORA01`) packages. V1 metadata remains plaintext.
- Added round-trip, chunked payload, metadata confidentiality, wrong-code, expiry and tamper-oriented tests.
- Documented that the local activation counter, system-clock expiry, local plaintext metadata history and temporary plaintext ZIP files are limitations of this offline alpha.

### Verification status

This source archive was structurally checked, but the current environment does not have Rust/Cargo installed, so `cargo check`, `cargo test`, and desktop UI execution could not be run here. Run the checks in `README.md` on your development machine before relying on this build. No independent security audit has been performed.
