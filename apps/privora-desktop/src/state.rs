use chrono::Utc;
use privora_core::Manifest;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRecord {
    pub manifest: Manifest,
    pub path: String,
    #[serde(default)]
    pub fingerprint: String,
    /// Indicates that the encrypted package contains an issuer note; the note itself is not copied into local JSON.
    #[serde(default)]
    pub has_private_note: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockRecord {
    pub name: String,
    pub output_dir: String,
    pub unlocked_at: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppPreferences {
    /// "Midnight" or "OLED Black".
    pub appearance: String,
    /// "Violet", "Blue", "Mint", or "Rose".
    pub accent: String,
    pub ui_scale: f32,
    pub compact_mode: bool,
    pub animations_enabled: bool,
    pub confirm_before_unlock: bool,
    pub clear_access_code_after_unlock: bool,
    pub remember_export_folder: bool,
    pub keep_unlock_history: bool,
    pub show_full_paths: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            appearance: "Midnight".into(),
            accent: "Violet".into(),
            ui_scale: 1.0,
            compact_mode: false,
            animations_enabled: true,
            confirm_before_unlock: true,
            clear_access_code_after_unlock: true,
            remember_export_folder: true,
            keep_unlock_history: true,
            show_full_paths: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LocalState {
    pub packages: Vec<PackageRecord>,
    /// Successful unlock count kept in this app profile only. It is not a global counter.
    pub activation_counts: HashMap<String, u32>,
    pub unlocked: Vec<UnlockRecord>,
    pub export_folder: String,
    pub preferences: AppPreferences,
}

pub fn state_file_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    base.join("Privora").join("state.json")
}

impl LocalState {
    pub fn load() -> Self {
        let path = state_file_path();
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = state_file_path();
        let parent = path
            .parent()
            .ok_or_else(|| "Cannot resolve Privora data directory".to_string())?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let text = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, text).map_err(|e| e.to_string())?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_package(&mut self, mut manifest: Manifest, path: String, fingerprint: String) {
        self.packages
            .retain(|p| p.manifest.package_id != manifest.package_id);
        let has_private_note = manifest
            .license_note
            .as_deref()
            .is_some_and(|note| !note.is_empty());
        // The .vora contains the real encrypted note. Do not duplicate that note as plaintext in state.json.
        manifest.license_note = None;
        self.packages.insert(
            0,
            PackageRecord {
                manifest,
                path,
                fingerprint,
                has_private_note,
            },
        );
        self.packages.truncate(100);
    }

    pub fn add_unlock(&mut self, name: String, output_dir: String, file_count: usize) {
        self.unlocked.insert(
            0,
            UnlockRecord {
                name,
                output_dir,
                unlocked_at: Utc::now().to_rfc3339(),
                file_count,
            },
        );
        self.unlocked.truncate(100);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_state_without_preferences_uses_defaults() {
        let legacy = br#"{"packages":[],"activation_counts":{},"unlocked":[],"export_folder":""}"#;
        let state: LocalState =
            serde_json::from_slice(legacy).expect("legacy state remains readable");
        assert_eq!(state.preferences, AppPreferences::default());
    }

    #[test]
    fn preferences_round_trip_through_local_state_json() {
        let mut state = LocalState::default();
        state.preferences.accent = "Rose".into();
        state.preferences.ui_scale = 1.2;
        state.preferences.compact_mode = true;
        let encoded = serde_json::to_vec(&state).expect("serialize preferences");
        let decoded: LocalState =
            serde_json::from_slice(&encoded).expect("deserialize preferences");
        assert_eq!(decoded.preferences, state.preferences);
    }

    #[test]
    fn private_license_note_is_not_persisted_in_local_state() {
        let mut state = LocalState::default();
        let manifest = Manifest {
            format_version: 2,
            package_id: "local-state-test".into(),
            name: "Sample package".into(),
            created_at: "2026-10-09T00:00:00Z".into(),
            expires_on: None,
            valid_from: None,
            recipient_label: Some("Recipient label".into()),
            license_note: Some("CONFIDENTIAL-NOTE-DO-NOT-PERSIST".into()),
            max_activations: 2,
            kdf: "Argon2id".into(),
            cipher: "AES-256-GCM".into(),
            salt_hex: "00".repeat(16),
            nonce_prefix_hex: "00".repeat(8),
            plaintext_len: 1,
            chunk_size: 1024 * 1024,
            file_count: 1,
            total_file_bytes: 1,
        };
        state.add_package(manifest, "sample.vora".into(), "fingerprint".into());
        assert!(state.packages[0].has_private_note);
        assert!(state.packages[0].manifest.license_note.is_none());
        let serialized = serde_json::to_string(&state).expect("serialize local state");
        assert!(!serialized.contains("CONFIDENTIAL-NOTE-DO-NOT-PERSIST"));
    }
}
