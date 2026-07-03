//! `aigs` — command line tool for AI Game Studio projects.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aigs_project::{AssetKind, Project, Scene};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aigs", version, about = "AI Game Studio command line tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a project manifest and every scene and asset it references.
    Validate {
        /// Path to the project manifest (game.aigs).
        manifest: PathBuf,
    },
    /// Load a project and run its initial scene in a window.
    Run {
        /// Path to the project manifest (game.aigs).
        manifest: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { manifest } => validate(&manifest),
        Command::Run { manifest } => run_project(&manifest),
    }
}

fn validate(manifest: &Path) -> ExitCode {
    let project = match Project::load(manifest) {
        Ok(project) => project,
        Err(err) => return fail(&format!("{}: {err}", manifest.display())),
    };
    let root = manifest.parent().unwrap_or(Path::new("."));
    let mut errors = Vec::new();

    if !project.scenes.contains(&project.initial_scene) {
        errors.push(format!(
            "initial_scene \"{}\" is not listed in scenes",
            project.initial_scene
        ));
    }
    for scene_path in &project.scenes {
        if let Err(err) = Scene::load(&root.join(scene_path)) {
            errors.push(format!("scene {scene_path}: {err}"));
        }
    }
    for asset in &project.assets {
        if !root.join(&asset.path).is_file() {
            errors.push(format!(
                "asset \"{}\" points to missing file {}",
                asset.id, asset.path
            ));
        }
        if asset.kind == AssetKind::Other {
            eprintln!("warning: asset \"{}\" has kind \"other\"", asset.id);
        }
    }

    if errors.is_empty() {
        println!(
            "OK: \"{}\" — {} scene(s), {} asset(s)",
            project.name,
            project.scenes.len(),
            project.assets.len()
        );
        ExitCode::SUCCESS
    } else {
        for error in &errors {
            eprintln!("error: {error}");
        }
        fail(&format!("{} problem(s) found", errors.len()))
    }
}

/// Runs a project's initial scene: milestone M2 deliverable — the runtime
/// executing a game defined entirely as `.aigs` data.
fn run_project(manifest: &Path) -> ExitCode {
    use aigs_runtime::{instantiate_scene, AppConfig, AssetStore};

    let project = match Project::load(manifest) {
        Ok(project) => project,
        Err(err) => return fail(&format!("{}: {err}", manifest.display())),
    };
    let root = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();
    let scene = match Scene::load(&root.join(&project.initial_scene)) {
        Ok(scene) => scene,
        Err(err) => return fail(&format!("scene {}: {err}", project.initial_scene)),
    };
    if !scene.animations.is_empty() {
        eprintln!(
            "note: scene declares {} animation(s); timeline playback arrives in milestone M4",
            scene.animations.len()
        );
    }

    let config = AppConfig {
        title: project.name.clone(),
        max_frames: std::env::var("AIGS_MAX_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok()),
        ..AppConfig::default()
    };
    let assets = project.assets.clone();
    let result = aigs_runtime::run(
        config,
        move |world, renderer| {
            let store = match AssetStore::load(renderer, &root, &assets) {
                Ok(store) => store,
                Err(err) => {
                    eprintln!("asset error: {err}");
                    std::process::exit(1);
                }
            };
            match instantiate_scene(world, &scene, &store) {
                Ok(instance) => println!(
                    "running \"{}\": {} entities, {} textures",
                    scene.name,
                    instance.len(),
                    store.len()
                ),
                Err(err) => {
                    eprintln!("scene error: {err}");
                    std::process::exit(1);
                }
            }
        },
        |_, _, _| {},
    );

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&format!("runtime error: {err}")),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("validation failed: {message}");
    ExitCode::FAILURE
}
