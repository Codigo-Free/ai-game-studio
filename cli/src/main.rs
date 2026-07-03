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
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { manifest } => validate(&manifest),
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

fn fail(message: &str) -> ExitCode {
    eprintln!("validation failed: {message}");
    ExitCode::FAILURE
}
