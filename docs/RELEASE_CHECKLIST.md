# Release checklist

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` (fix warnings, or document why a lint is intentionally allowed)
- [ ] Manual visual QA of all five pages on Windows at multiple window sizes.
- [ ] Verify a new v2 package does not contain package name, recipient label, note, validity date, expiry date, or `max_activations` as plaintext.
- [ ] Verify `state.json` does not contain the private license-note text or access code.
- [ ] Wrong code and changed/tampered metadata must fail.
- [ ] Round-trip text, image, audio and large video samples; compare SHA-256 hashes before/after.
- [ ] Expiry and future start-date policy checks.
- [ ] Confirm the activation counter is only a local best-effort restriction, not global enforcement.
- [ ] Build and test the Inno Setup installer on a clean Windows account.
- [ ] Review temporary plaintext ZIP behavior and local JSON metadata before using sensitive production data.
- [ ] Obtain an independent security review before public/production claims.
