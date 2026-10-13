# Privora v0.4 — Settings reference

Settings are local to this Windows/user profile and are stored in the Privora `state.json` file. Existing v0.3 state files remain readable; missing preference fields use `AppPreferences::default()`.

## Appearance & feel

- **Midnight / OLED Black** — changes the base background, sidebar, panel, and border palette. Individual success/warning panels retain their meaning-based colors.
- **Accent color** — Violet, Ocean blue, Mint, or Rose. The accent is applied to primary actions, selection state, and icon emphasis.
- **Interface scale** — sets the egui global zoom from 85% to 130%.
- **Compact spacing** — reduces general spacing, button padding, and sidebar navigation height.
- **Smooth interface animations** — toggles the egui animation duration and animated active-navigation indicator.

## File workflow

- **Confirm before decrypting files** — prompts before starting a decryption/extraction operation.
- **Clear the access code after success** — removes the entered code from the field after successful extraction. This is recommended on shared devices.
- **Remember extraction location** — when disabled, the default output folder is not saved in local app state. The selected folder remains available for the current running session.

## Privacy & activity

- **Keep recent unlock activity** — controls whether new successful unlocks are added to the visible recent activity list. It does not change the separate per-profile unlock counter.
- **Show full paths in history** — controls whether package and extraction history displays full local paths or a shortened path/folder name.
- **Clear recent activity** — removes visible extraction history. It intentionally does not reset local unlock counters, because using history cleanup as a way to reset a policy counter would be misleading.

## Persistence

Most settings are previewed immediately; use **Save preferences** to persist them deliberately. **Restore defaults** resets preferences and the default extraction folder. Settings are not synchronized between users or devices.

## Security limits

These preferences are usability controls, not an anti-tamper boundary. A user controlling the device can alter local state or modify an offline application. The local unlock counter is not a global person/device counter. Expiry checks depend on the device clock. Do not describe offline enforcement as unbreakable DRM.
