//! Tauri backend of the AI Game Studio editor.
//!
//! The frontend owns the document model; this layer is the bridge to disk
//! and to the runtime: it loads/saves `.aigs` files (validated through
//! `aigs-project`, the format's reference implementation), imports assets
//! into the project and launches the runtime player.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use aigs_project::{Project, Scene};
use base64::Engine;
use serde::Serialize;
use tauri::Emitter;

/// A project loaded from disk: the manifest plus every scene, already parsed
/// and re-serialized so the frontend always sees canonical format JSON.
#[derive(Serialize)]
struct LoadedProject {
    /// Directory containing `game.aigs`; all project paths are relative to it.
    root: String,
    manifest_path: String,
    project: Project,
    scenes: Vec<LoadedScene>,
}

#[derive(Serialize)]
struct LoadedScene {
    /// Path relative to the project root, as listed in the manifest.
    path: String,
    scene: Scene,
}

#[tauri::command]
fn load_project(manifest_path: String) -> Result<LoadedProject, String> {
    let manifest = PathBuf::from(&manifest_path);
    let project = Project::load(&manifest).map_err(|e| e.to_string())?;
    let root = manifest
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let mut scenes = Vec::new();
    for scene_path in &project.scenes {
        let scene = Scene::load(&root.join(scene_path))
            .map_err(|e| format!("scene {scene_path}: {e}"))?;
        scenes.push(LoadedScene {
            path: scene_path.clone(),
            scene,
        });
    }
    Ok(LoadedProject {
        root: root.display().to_string(),
        manifest_path,
        project,
        scenes,
    })
}

/// Creates a new project skeleton (manifest + main scene + assets dir) and
/// returns it loaded.
#[tauri::command]
fn create_project(directory: String, name: String) -> Result<LoadedProject, String> {
    let root = PathBuf::from(&directory);
    let manifest_path = root.join("game.aigs");
    if manifest_path.exists() {
        return Err("directory already contains a game.aigs".into());
    }
    std::fs::create_dir_all(root.join("scenes")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(root.join("assets")).map_err(|e| e.to_string())?;

    let scene_rel = "scenes/main.scene.aigs".to_string();
    let scene_json = serde_json::json!({
        "format": { "kind": "aigs-scene", "version": 0 },
        "name": "main",
        "entities": [{
            "id": "camera",
            "name": "Main Camera",
            "components": { "transform2d": {}, "camera2d": { "zoom": 1.0 } }
        }],
        "animations": []
    });
    let manifest_json = serde_json::json!({
        "format": { "kind": "aigs-project", "version": 0 },
        "name": name,
        "initial_scene": scene_rel,
        "scenes": [scene_rel],
        "assets": []
    });
    write_pretty(&root.join(&scene_rel), &scene_json)?;
    write_pretty(&manifest_path, &manifest_json)?;
    load_project(manifest_path.display().to_string())
}

/// Saves the manifest and every scene. Everything is validated through the
/// reference implementation before touching the disk.
#[tauri::command]
fn save_project(
    manifest_path: String,
    project_json: String,
    scenes: Vec<(String, String)>,
) -> Result<(), String> {
    let manifest = PathBuf::from(&manifest_path);
    let root = manifest.parent().unwrap_or(Path::new("."));

    let project = Project::from_json(&project_json).map_err(|e| e.to_string())?;
    let mut parsed = Vec::new();
    for (path, json) in &scenes {
        let scene = Scene::from_json(json).map_err(|e| format!("scene {path}: {e}"))?;
        parsed.push((path.clone(), scene));
    }
    std::fs::write(
        &manifest,
        project.to_json().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    for (path, scene) in parsed {
        let full = root.join(&path);
        if let Some(dir) = full.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&full, scene.to_json().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Copies an external image into the project's `assets/` directory and
/// returns the new asset entry as JSON-friendly data.
#[derive(Serialize)]
struct ImportedAsset {
    id: String,
    path: String,
}

#[tauri::command]
fn import_asset(project_root: String, source_path: String) -> Result<ImportedAsset, String> {
    let source = PathBuf::from(&source_path);
    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid file name")?
        .to_string();
    let stem = source
        .file_stem()
        .and_then(|n| n.to_str())
        .ok_or("invalid file name")?
        .to_lowercase()
        .replace([' ', '.'], "-");
    let assets_dir = PathBuf::from(&project_root).join("assets");
    std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    let destination = assets_dir.join(&file_name);
    std::fs::copy(&source, &destination).map_err(|e| e.to_string())?;
    Ok(ImportedAsset {
        id: stem,
        path: format!("assets/{file_name}"),
    })
}

/// Returns a file's content as base64 (used for asset thumbnails and the
/// viewport image cache).
#[tauri::command]
fn read_file_base64(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Launches the project in the runtime player (`aigs run`). The binary is
/// resolved from `AIGS_CLI`, then `aigs` on PATH. The player's stdout and
/// stderr are streamed to the editor console as `player-log` / `player-err`
/// events (scene switches, per-second stats, warnings).
#[tauri::command]
fn play_project(app: tauri::AppHandle, manifest_path: String) -> Result<String, String> {
    let binary = std::env::var("AIGS_CLI").unwrap_or_else(|_| "aigs".to_string());
    let mut child = Command::new(&binary)
        .arg("run")
        .arg(&manifest_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "could not launch \"{binary} run\": {e}. Install the CLI with \
                 `cargo install --path cli` or set AIGS_CLI to the binary path."
            )
        })?;
    let pid = child.id();

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                let _ = app.emit("player-log", line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                let _ = app.emit("player-err", line);
            }
        });
    }
    std::thread::spawn(move || {
        let message = match child.wait() {
            Ok(status) if status.success() => "player finished".to_string(),
            Ok(status) => format!("player exited with {status}"),
            Err(e) => format!("player wait error: {e}"),
        };
        let _ = app.emit("player-log", message);
    });

    Ok(format!("player started (pid {pid})"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_project,
            create_project,
            save_project,
            import_asset,
            read_file_base64,
            play_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn write_pretty(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    std::fs::write(
        path,
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
