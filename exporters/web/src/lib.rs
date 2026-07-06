//! Web exporter (milestone M14): turns an `.aigs` project into a folder
//! that runs in a browser, served as static files.
//!
//! Layout produced — the same `data/` shape the Desktop exporter (M7)
//! uses, so both exporters share one mental model:
//!
//! ```text
//! <output>/<slug>/
//! ├── index.html
//! ├── <player>.js        # copy of the prebuilt web player's JS glue
//! ├── <player>_bg.wasm   # copy of the prebuilt web player's wasm binary
//! └── data/
//!     ├── game.aigs
//!     ├── scenes/…       # every scene listed in the manifest
//!     └── assets/…       # every asset listed in the manifest
//! ```
//!
//! Unlike Desktop (where the running `aigs` binary *is* the self-player),
//! there is no wasm equivalent of "the CLI executable" — the player bundle
//! must be built once (`exporters/web-player`, via `wasm-bindgen`) and
//! passed in as prebuilt files. Exporting a project itself is still just
//! copying files, no compilation.

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
}

/// Options for a Web export.
pub struct ExportOptions<'a> {
    /// The web player's JS glue module (`wasm-bindgen --target web` output).
    pub player_js: &'a Path,
    /// The web player's wasm binary, next to `player_js`.
    pub player_wasm: &'a Path,
    /// Directory where the game folder is created (default `dist/`).
    pub output: &'a Path,
}

/// What an export produced, for reporting.
pub struct ExportReport {
    pub game_dir: PathBuf,
    pub index_html: PathBuf,
    pub files_copied: usize,
}

/// Exports the project at `manifest` as a static Web build.
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

    // The player bundle, kept under its own filenames (whatever
    // `wasm-bindgen` named them) so the generated `index.html` can just
    // reference them directly.
    let js_name = file_name(options.player_js)?;
    let wasm_name = file_name(options.player_wasm)?;
    copy_file(options.player_js, &game_dir.join(js_name))?;
    files_copied += 1;
    copy_file(options.player_wasm, &game_dir.join(wasm_name))?;
    files_copied += 1;

    let index_html = game_dir.join("index.html");
    std::fs::write(&index_html, render_index_html(&project.name, js_name)).map_err(|source| {
        ExportError::Io {
            path: index_html.display().to_string(),
            source,
        }
    })?;
    files_copied += 1;

    Ok(ExportReport {
        game_dir,
        index_html,
        files_copied,
    })
}

/// Folder name derived from the game name (same rule as the Desktop
/// exporter, kept independent per-crate rather than shared).
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

fn render_index_html(title: &str, js_name: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
  html, body {{ margin: 0; height: 100%; background: #14141a; }}
  canvas {{ display: block; width: 100%; height: 100%; }}
</style>
</head>
<body>
<script type="module">
  import init from "./{js_name}";
  init();
</script>
</body>
</html>
"#
    )
}

fn file_name(path: &Path) -> Result<&str, ExportError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ExportError::Io {
            path: path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a valid file name"),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_example() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-world/game.aigs")
    }

    fn fake_player_bundle(dir: &Path) -> (PathBuf, PathBuf) {
        let js = dir.join("aigs_web_player.js");
        let wasm = dir.join("aigs_web_player_bg.wasm");
        std::fs::write(&js, b"export default function init() {}").unwrap();
        std::fs::write(&wasm, b"\0asm").unwrap();
        (js, wasm)
    }

    #[test]
    fn slugify_handles_names() {
        assert_eq!(slugify("Robot Rescue"), "robot-rescue");
        assert_eq!(slugify("¡Mi Juego 2!"), "mi-juego-2");
        assert_eq!(slugify("***"), "game");
    }

    #[test]
    fn exports_full_layout_with_player_and_index_html() {
        let temp = tempfile::tempdir().unwrap();
        let (player_js, player_wasm) = fake_player_bundle(temp.path());

        let report = export(
            &repo_example(),
            &ExportOptions {
                player_js: &player_js,
                player_wasm: &player_wasm,
                output: &temp.path().join("dist"),
            },
        )
        .unwrap();

        assert!(report.game_dir.ends_with("hello-world"));
        assert!(report.index_html.is_file());
        assert!(report.game_dir.join("data/game.aigs").is_file());
        assert!(report
            .game_dir
            .join("data/scenes/main.scene.aigs")
            .is_file());
        assert!(report.game_dir.join("data/assets/hero.png").is_file());
        assert!(report.game_dir.join("aigs_web_player.js").is_file());
        assert!(report.game_dir.join("aigs_web_player_bg.wasm").is_file());

        let html = std::fs::read_to_string(&report.index_html).unwrap();
        assert!(html.contains("./aigs_web_player.js"));
        assert!(html.contains("Hello World"));
        assert_eq!(
            report.files_copied, 7,
            "manifest + 2 scenes + 1 asset + js + wasm + index.html"
        );
    }

    #[test]
    fn refuses_to_overwrite_existing_export() {
        let temp = tempfile::tempdir().unwrap();
        let (player_js, player_wasm) = fake_player_bundle(temp.path());
        let options = ExportOptions {
            player_js: &player_js,
            player_wasm: &player_wasm,
            output: &temp.path().join("dist"),
        };
        export(&repo_example(), &options).unwrap();
        assert!(matches!(
            export(&repo_example(), &options),
            Err(ExportError::OutputExists(_))
        ));
    }
}
