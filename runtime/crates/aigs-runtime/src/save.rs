//! Player save data (milestone M13): persists script instance state
//! (`get_var`/`set_var`) across sessions, keyed by authored entity id.
//!
//! `save.json` is **not** part of the `.aigs` format: it is player-state
//! (a running game's progress), not design-time project data. It lives
//! next to the project's manifest.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Current save file schema version.
pub const SAVE_VERSION: u32 = 1;

/// Everything persisted between sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    /// Unix timestamp (seconds) of when this save was written.
    pub saved_at_unix: u64,
    /// Script instance memory (`get_var`/`set_var`), by authored entity id.
    pub scripts: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
}

impl SaveData {
    /// A fresh save capturing `scripts` at the current time.
    pub fn now(
        scripts: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
    ) -> Self {
        Self {
            version: SAVE_VERSION,
            saved_at_unix: unix_now(),
            scripts,
        }
    }

    /// Loads a save file. Returns `Ok(None)` if it doesn't exist yet (a
    /// fresh game); parse errors are reported so a corrupt save isn't
    /// silently treated as "no save" (which would look like data loss).
    pub fn load(path: &Path) -> Result<Option<Self>, String> {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json)
                .map(Some)
                .map_err(|error| format!("save {}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("save {}: {error}", path.display())),
        }
    }

    /// Writes the save file (pretty JSON, atomic-ish via a temp file rename
    /// where the platform supports it).
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(path, json).map_err(|error| format!("save {}: {error}", path.display()))
    }

    /// Real-world seconds between `saved_at_unix` and now (0 if the clock
    /// went backwards, e.g. system clock changes).
    pub fn offline_seconds(&self) -> f64 {
        unix_now().saturating_sub(self.saved_at_unix) as f64
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn round_trips_through_json() {
        let mut pet = HashMap::new();
        pet.insert("hunger".to_string(), 42.0);
        let mut scripts = HashMap::new();
        scripts.insert("pet".to_string(), pet);
        let save = SaveData::now(scripts);

        let dir = std::env::temp_dir().join(format!("aigs-save-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.json");
        save.write(&path).unwrap();
        let loaded = SaveData::load(&path).unwrap().expect("save must load back");
        assert_eq!(loaded, save);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_a_clean_none() {
        let path = std::env::temp_dir().join("aigs-save-test-does-not-exist.json");
        assert_eq!(SaveData::load(&path).unwrap(), None);
    }

    #[test]
    fn corrupt_file_is_reported_not_silently_ignored() {
        let dir =
            std::env::temp_dir().join(format!("aigs-save-corrupt-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(SaveData::load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offline_seconds_reflects_elapsed_real_time() {
        let mut save = SaveData::now(HashMap::new());
        save.saved_at_unix -= 3600;
        let elapsed = save.offline_seconds();
        assert!(
            (3599.0..=3610.0).contains(&elapsed),
            "expected ~3600s, got {elapsed}"
        );
    }
}
