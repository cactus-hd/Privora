# Privora UI/UX direction (v0.4)

## Product feel

Privora should feel calm, precise, and premium rather than dense or cyberpunk. The inspiration is modern Apple utility software: strong typographic hierarchy, generous spacing, restrained color, direct language, and short task flows. It is still a native Rust/egui interface, not a web design system.

## Visual system

- Base appearance: Midnight; optional OLED Black background mode.
- Accent choices: Violet, Ocean blue, Mint, and Rose.
- High-contrast neutral text; accent color is reserved for selected navigation, primary actions, and small emphasis.
- Rounded panel corners, thinner borders, softer selected-state treatment, and wider page gutters.
- Native vector icons are drawn with egui primitives rather than system font glyphs.
- System font is used when available (Segoe UI on Windows, system sans-serif fallbacks elsewhere).
- Interface scale can be changed from 85% through 130%.

## Interaction model

- Navigation uses animated selected-state indicators; motion can be disabled.
- Compact mode reduces spacing and control padding for smaller windows.
- Decryption can require an explicit confirmation before plaintext files are written.
- Access codes can be automatically cleared after successful unlock.
- The user can disable recent unlock history, choose whether paths are shown in full, and opt out of storing the last extraction folder.
- Settings preview immediately; saving persists preferences to local app state. Reset to defaults is explicit.

## Review checklist

Test Windows at 1040x700 and 1360x900. Test OLED mode and each accent. Verify UI scaling, compact mode, reduced motion, clear activity, remembered folder disabled, confirmation before unlock, and access-code clearing. Confirm the user can create a package and import/unlock an existing package after changing every preference.

## Limitations

Some older form/page surfaces remain hand-drawn and may not reflect every OLED shade perfectly. Do not claim native macOS glass materials; the look is an egui approximation. Review actual screenshots on Windows before calling the design complete.
