# Android output: status and preparation steps

## Current status: not build-ready as an APK

The core `.vora` engine is Rust and `eframe` has Android support, but this specific v0.4 app is still a desktop application. It uses `rfd::FileDialog` for open/save/folder dialogs; `rfd` 0.17.2 documents Linux, Windows and macOS support, not Android. The repository also does not yet include an Android app entry point, manifest/Gradle packaging project, or Android Storage Access Framework (SAF) bridge. Therefore, merely adding a Rust Android target will **not** produce a working Privora APK.

## 1. Install the Android toolchain

Install Android Studio and, in SDK Manager, install an Android SDK Platform, Android SDK Build-Tools, Android SDK Platform-Tools, and the Android NDK (Side by side). Use a supported JDK that matches the installed Android Gradle Plugin. Keep the exact SDK/NDK versions recorded for reproducible builds.

Configure `ANDROID_HOME` to the Android SDK directory. `cargo-ndk` can usually detect the NDK installed by Android Studio; if not, set `ANDROID_NDK_HOME` to the actual NDK folder.

## 2. Add Rust Android targets

Open PowerShell and run:

```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
```

These commands prepare Rust's target standard libraries and the cargo-ndk helper; **they do not build this current desktop app into an APK**.

## 3. Work still required in the project

Before an APK can be built and tested, Privora needs:

1. An Android launcher/activity and Gradle project configured to launch the `eframe`/`android-activity` native Rust app.
2. An Android-specific implementation for file selection and save locations using Android's Storage Access Framework (SAF), replacing `rfd` calls for Android. It must handle Android content URIs, permissions and streaming instead of assuming every selected item is a normal filesystem `Path`.
3. Android-specific Cargo dependency configuration and any required `cdylib` / native entry-point setup for the pinned `eframe` version.
4. Tests on a real Android device or emulator for large files, permission loss, backgrounding, orientation, and memory pressure.

The UI and `.vora` core can remain Rust. A small Android platform bridge may still be needed to call system document-picker APIs; that does not require rewriting the product or its crypto core in Flutter.

## 4. Build flow after that work is added

The expected flow will be: build the Rust native library with `cargo ndk`, place the `.so` files in the Android project's `jniLibs` directory, then use Gradle to package/sign the APK or AAB. The exact commands depend on the final Android activity/Gradle scaffold and must be added to this repository once that scaffold exists. Do not publish an APK before the UI, file picker and encryption round-trip have been tested on-device.
