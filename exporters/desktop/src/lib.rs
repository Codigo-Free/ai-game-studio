//! Desktop exporter (milestone M7): turns an `.aigs` project into a
//! distributable folder that runs without AI Game Studio installed.
//!
//! Layout produced (self-player design, see `docs/arquitectura.md`):
//!
//! ```text
//! <output>/<slug>/
//! ├── <slug>[.exe]      # copy of the player binary (the `aigs` CLI itself,
//! │                     # which auto-runs data/game.aigs when present)
//! └── data/
//!     ├── game.aigs
//!     ├── scenes/…      # every scene listed in the manifest
//!     └── assets/…      # every asset listed in the manifest
//! ```

use std::io::Write;
use std::path::{Path, PathBuf};

use aigs_project::{Project, Scene};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("manifest: {0}")]
    Format(#[from] aigs_project::FormatError),
    #[error("io error on {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("output {0} already exists; remove it or choose another --output")]
    OutputExists(String),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

/// Options for a desktop export.
pub struct ExportOptions<'a> {
    /// The self-player binary to bundle (normally `std::env::current_exe()`).
    pub player: &'a Path,
    /// Directory where the game folder is created (default `dist/`).
    pub output: &'a Path,
    /// Also produce `<slug>.zip` next to the folder.
    pub zip: bool,
}

/// What an export produced, for reporting.
pub struct ExportReport {
    pub game_dir: PathBuf,
    pub executable: PathBuf,
    pub zip_file: Option<PathBuf>,
    pub files_copied: usize,
}

/// Exports the project at `manifest` for the current desktop platform.
pub fn export(manifest: &Path, options: &ExportOptions) -> Result<ExportReport, ExportError> {
    let project = Project::load(manifest)?;
    let root = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();

    // Validate every scene before touching the disk.
    for scene_path in &project.scenes {
        Scene::load(&root.join(scene_path))?;
    }

    let slug = slugify(&project.name);
    let game_dir = options.output.join(&slug);
    if game_dir.exists() {
        return Err(ExportError::OutputExists(game_dir.display().to_string()));
    }
    let data_dir = game_dir.join("data");
    create_dir(&data_dir)?;

    // Project data: manifest + scenes + assets, preserving relative paths.
    let mut files_copied = 0;
    copy_file(manifest, &data_dir.join("game.aigs"))?;
    files_copied += 1;
    for relative in project
        .scenes
        .iter()
        .chain(project.assets.iter().map(|asset| &asset.path))
    {
        let destination = data_dir.join(relative);
        if let Some(parent) = destination.parent() {
            create_dir(parent)?;
        }
        copy_file(&root.join(relative), &destination)?;
        files_copied += 1;
    }

    // The player binary, renamed to the game.
    let extension = options
        .player
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let executable = game_dir.join(format!("{slug}{extension}"));
    copy_file(options.player, &executable)?;
    files_copied += 1;

    let zip_file = if options.zip {
        let path = options.output.join(format!("{slug}.zip"));
        zip_directory(&game_dir, &path, &slug)?;
        Some(path)
    } else {
        None
    };

    Ok(ExportReport {
        game_dir,
        executable,
        zip_file,
        files_copied,
    })
}

/// Folder/executable name derived from the game name.
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    let mut collapsed = String::with_capacity(slug.len());
    let mut previous_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !previous_dash {
                collapsed.push('-');
            }
            previous_dash = true;
        } else {
            collapsed.push(c);
            previous_dash = false;
        }
    }
    if collapsed.is_empty() {
        "game".to_string()
    } else {
        collapsed
    }
}

fn create_dir(path: &Path) -> Result<(), ExportError> {
    std::fs::create_dir_all(path).map_err(|source| ExportError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn copy_file(from: &Path, to: &Path) -> Result<(), ExportError> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|source| ExportError::Io {
            path: from.display().to_string(),
            source,
        })
}

/// Zips `directory` into `zip_path`, with entries rooted at `prefix/`.
fn zip_directory(directory: &Path, zip_path: &Path, prefix: &str) -> Result<(), ExportError> {
    let file = std::fs::File::create(zip_path).map_err(|source| ExportError::Io {
        path: zip_path.display().to_string(),
        source,
    })?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    add_dir_to_zip(&mut writer, directory, Path::new(prefix), &options)?;
    writer.finish()?;
    Ok(())
}

fn add_dir_to_zip(
    writer: &mut zip::ZipWriter<std::fs::File>,
    directory: &Path,
    zip_root: &Path,
    options: &zip::write::SimpleFileOptions,
) -> Result<(), ExportError> {
    let entries = std::fs::read_dir(directory).map_err(|source| ExportError::Io {
        path: directory.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ExportError::Io {
            path: directory.display().to_string(),
            source,
        })?;
        let path = entry.path();
        let zipped = zip_root.join(entry.file_name());
        if path.is_dir() {
            add_dir_to_zip(writer, &path, &zipped, options)?;
        } else {
            writer.start_file(zipped.to_string_lossy().replace('\\', "/"), *options)?;
            let bytes = std::fs::read(&path).map_err(|source| ExportError::Io {
                path: path.display().to_string(),
                source,
            })?;
            writer.write_all(&bytes).map_err(|source| ExportError::Io {
                path: zip_path_display(&zipped),
                source,
            })?;
        }
    }
    Ok(())
}

fn zip_path_display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_example() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-world/game.aigs")
    }

    #[test]
    fn slugify_handles_names() {
        assert_eq!(slugify("Robot Rescue"), "robot-rescue");
        assert_eq!(slugify("¡Mi Juego 2!"), "mi-juego-2");
        assert_eq!(slugify("***"), "game");
    }

    #[test]
    fn exports_full_layout() {
        let temp = tempfile::tempdir().unwrap();
        // Any file works as a fake player binary for layout tests.
        let player = temp.path().join("fake-player");
        std::fs::write(&player, b"#!player").unwrap();

        let report = export(
            &repo_example(),
            &ExportOptions {
                player: &player,
                output: &temp.path().join("dist"),
                zip: true,
            },
        )
        .unwrap();

        assert!(report.game_dir.ends_with("hello-world"));
        assert!(report.executable.is_file());
        assert!(report.game_dir.join("data/game.aigs").is_file());
        assert!(report
            .game_dir
            .join("data/scenes/main.scene.aigs")
            .is_file());
        assert!(report
            .game_dir
            .join("data/scenes/level1.scene.aigs")
            .is_file());
        assert!(report.game_dir.join("data/assets/hero.png").is_file());
        assert_eq!(
            report.files_copied, 5,
            "manifest + 2 scenes + 1 asset + player"
        );
        let zip = report.zip_file.unwrap();
        assert!(zip.is_file());
        assert!(std::fs::metadata(&zip).unwrap().len() > 0);
    }

    #[test]
    fn refuses_to_overwrite_existing_export() {
        let temp = tempfile::tempdir().unwrap();
        let player = temp.path().join("fake-player");
        std::fs::write(&player, b"#!player").unwrap();
        let options = ExportOptions {
            player: &player,
            output: &temp.path().join("dist"),
            zip: false,
        };
        export(&repo_example(), &options).unwrap();
        assert!(matches!(
            export(&repo_example(), &options),
            Err(ExportError::OutputExists(_))
        ));
    }
}
