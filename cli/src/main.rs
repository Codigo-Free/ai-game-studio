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
    use aigs_runtime::{AppConfig, AssetStore, GamePlayer};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    let project = match Project::load(manifest) {
        Ok(project) => project,
        Err(err) => return fail(&format!("{}: {err}", manifest.display())),
    };
    let root = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut scenes = HashMap::new();
    for path in &project.scenes {
        match Scene::load(&root.join(path)) {
            Ok(scene) => {
                scenes.insert(path.clone(), scene);
            }
            Err(err) => return fail(&format!("scene {path}: {err}")),
        }
    }

    let config = AppConfig {
        title: project.name.clone(),
        max_frames: std::env::var("AIGS_MAX_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok()),
        ..AppConfig::default()
    };
    // The player is created in `setup` (it needs the GPU asset store) and
    // driven from `update`; the Rc bridges the two closures.
    let player: Rc<RefCell<Option<GamePlayer<AssetStore>>>> = Rc::default();
    let player_setup = Rc::clone(&player);
    let assets = project.assets.clone();
    let project_for_setup = project.clone();
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
            match GamePlayer::new(&project_for_setup, scenes, store, world) {
                Ok(game) => {
                    for warning in game.warnings() {
                        eprintln!("warning: {warning}");
                    }
                    println!(
                        "running \"{}\": {} entities, {} animation(s)",
                        game.current_scene(),
                        world.len(),
                        game.animation_count()
                    );
                    *player_setup.borrow_mut() = Some(game);
                }
                Err(err) => {
                    eprintln!("player error: {err}");
                    std::process::exit(1);
                }
            }
        },
        move |world, time, input| {
            if let Some(game) = player.borrow_mut().as_mut() {
                let scene_before = game.current_scene().to_string();
                let warnings_before = game.warnings().len();
                game.update(world, time, input);
                if game.current_scene() != scene_before {
                    println!("scene: {}", game.current_scene());
                }
                for warning in &game.warnings()[warnings_before.min(game.warnings().len())..] {
                    eprintln!("warning: {warning}");
                }
                // One stats line per second, consumed by the editor console.
                if time.tick % 60 == 0 && time.tick > 0 {
                    println!(
                        "stats: fps={:.0} entities={} frame_ms={:.1}",
                        time.fps,
                        world.len(),
                        if time.fps > 0.0 {
                            1000.0 / time.fps
                        } else {
                            0.0
                        }
                    );
                }
            }
        },
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
