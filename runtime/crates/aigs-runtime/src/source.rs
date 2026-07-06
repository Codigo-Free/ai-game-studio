//! Where a project's bytes come from (milestone M14): a local directory on
//! Desktop, or an in-memory bundle fetched ahead of time on Web, where there
//! is no filesystem and fetching is asynchronous. `AssetStore`, `AudioPlayer`
//! and `ScriptHost` read through this instead of touching `std::fs` directly,
//! so the same loading/decoding code runs on both targets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Reads a project's files by path relative to the project root.
pub trait AssetSource {
    fn read(&self, relative_path: &str) -> std::io::Result<Vec<u8>>;

    fn read_to_string(&self, relative_path: &str) -> std::io::Result<String> {
        let bytes = self.read(relative_path)?;
        String::from_utf8(bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    /// The real filesystem root this source reads from, if any. Only
    /// Desktop has one; it's what lets `ScriptHost` watch `.rhai` files for
    /// hot reload. `None` means "no filesystem to watch" (Web, in-memory).
    fn as_path(&self) -> Option<&Path> {
        None
    }
}

/// Desktop: reads straight from disk, relative to the project's root
/// directory. Implemented for `PathBuf` rather than `Path` so `&dyn
/// AssetSource` trait objects can be built from it (`Path` is itself
/// unsized, which rules out the `&Path -> &dyn AssetSource` coercion).
impl AssetSource for PathBuf {
    fn read(&self, relative_path: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.join(relative_path))
    }

    fn as_path(&self) -> Option<&Path> {
        Some(self.as_path())
    }
}

/// Web: every file was already fetched once (bytes can't be read
/// synchronously in a browser) and is served from memory from then on.
#[derive(Default)]
pub struct MemoryAssets(HashMap<String, Vec<u8>>);

impl MemoryAssets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, relative_path: impl Into<String>, bytes: Vec<u8>) {
        self.0.insert(relative_path.into(), bytes);
    }
}

impl AssetSource for MemoryAssets {
    fn read(&self, relative_path: &str) -> std::io::Result<Vec<u8>> {
        self.0.get(relative_path).cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{relative_path} was not prefetched"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_assets_round_trip_bytes_and_text() {
        let mut mem = MemoryAssets::new();
        mem.insert("scripts/pet.rhai", b"fn on_start() {}".to_vec());
        assert_eq!(
            mem.read_to_string("scripts/pet.rhai").unwrap(),
            "fn on_start() {}"
        );
        assert!(mem.read("missing.txt").is_err());
    }

    #[test]
    fn path_reads_relative_to_root() {
        let dir = std::env::temp_dir().join(format!("aigs-source-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello.txt"), b"hi").unwrap();
        assert_eq!(dir.read("hello.txt").unwrap(), b"hi");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
