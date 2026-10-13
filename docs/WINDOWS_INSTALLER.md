# Windows installer: step by step

The included installer uses **Inno Setup** to produce a normal per-user `Setup.exe`. For this small Rust desktop app, that is the simplest first release path. The installer places the app under `%LOCALAPPDATA%\Programs\Privora`, creates a Start Menu shortcut, and optionally creates a desktop shortcut. It does not remove the user's local profile on uninstall.

## 1. Build the release executable

Install Rust via [rustup.rs](https://rustup.rs/) and the Visual Studio C++ Build Tools workload as described in the main README. Open PowerShell at the repository root and run:

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build --release -p privora-desktop
```

The executable should be here:

```text
target\release\privora-desktop.exe
```

Or run the helper included with this repo:

```powershell
.\scripts\build-windows.ps1 -Test
```

## 2. Install Inno Setup

Install Inno Setup 6 from its official website: <https://jrsoftware.org/isinfo.php>.

## 3. Build the installer

At the repository root, run:

```powershell
.\scripts\build-windows.ps1 -Installer
```

This performs formatting checks, builds the release executable, then invokes `ISCC.exe` to compile `packaging\windows\Privora.iss`.

Expected installer:

```text
dist\Privora-Setup-0.3.0-x64.exe
```

If Inno Setup is installed but the script cannot find its compiler, add the folder containing `ISCC.exe` to PATH or compile `packaging\windows\Privora.iss` from the Inno Setup GUI.

## 4. Test before sharing

- Test installation and uninstall in a Windows account or virtual machine without your Rust toolchain.
- Test the app launch, creating a `.vora`, copying the access code, importing, wrong-code failure, expiry and extraction.
- Confirm Windows Defender/SmartScreen behavior. Unsigned binaries can show reputation warnings; code signing is a separate release step.
- Never distribute a `.license.txt` containing a real access code with the package in a public channel.

## Why Setup.exe, not MSI?

Inno Setup produces a straightforward installer and is adequate for an early consumer release. If an organization later requires MSI deployment, use WiX Toolset to author and test an MSI package; MSI is not required to ship a Windows installer. The script in this repository intentionally produces `Setup.exe` only.
