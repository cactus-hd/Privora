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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockRecord {
    pub name: String,
    pub output_dir: String,
    pub unlocked_at: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LocalState {
    pub packages: Vec<PackageRecord>,
    /// Successful unlock count kept in this app profile only. It is not a global counter.
    pub activation_counts: HashMap<String, u32>,
    pub unlocked: Vec<UnlockRecord>,
    pub export_folder: String,
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

    pub fn add_package(&mut self, manifest: Manifest, path: String, fingerprint: String) {
        self.packages
            .retain(|p| p.manifest.package_id != manifest.package_id);
        self.packages.insert(
            0,
            PackageRecord {
                manifest,
                path,
                fingerprint,
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
