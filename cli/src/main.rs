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
    /// Export the project as a standalone desktop game folder.
    Export {
        /// Path to the project manifest (game.aigs).
        manifest: PathBuf,
        /// Directory where the game folder is created.
        #[arg(long, default_value = "dist")]
        output: PathBuf,
        /// Also produce a .zip next to the game folder.
        #[arg(long)]
        zip: bool,
    },
    /// Print the scripting API as machine-readable JSON: every lifecycle
    /// function and callable a `.rhai` script can use. Intended for AI
    /// agents and editor tooling (see sdk/aigs-format/scripting-api.json).
    ScriptApi,
}

fn main() -> ExitCode {
    // Self-player mode: an exported game is this same binary renamed, with
    // its project in `data/game.aigs` next to it (see exporters/desktop).
    if std::env::args_os().len() <= 1 {
        if let Some(manifest) = bundled_manifest() {
            return run_project(&manifest);
        }
    }
    match Cli::parse().command {
        Command::Validate { manifest } => validate(&manifest),
        Command::Run { manifest } => run_project(&manifest),
        Command::Export {
            manifest,
            output,
            zip,
        } => export_project(&manifest, &output, zip),
        Command::ScriptApi => script_api(),
    }
}

fn script_api() -> ExitCode {
    match serde_json::to_string_pretty(&aigs_runtime::api_manifest()) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(err) => fail(&format!("script-api: {err}")),
    }
}

/// `data/game.aigs` next to the running executable, if any.
fn bundled_manifest() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let manifest = exe.parent()?.join("data").join("game.aigs");
    manifest.is_file().then_some(manifest)
}

fn export_project(manifest: &Path, output: &Path, zip: bool) -> ExitCode {
    let player = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => return fail(&format!("cannot locate player binary: {err}")),
    };
    match aigs_export_desktop::export(
        manifest,
        &aigs_export_desktop::ExportOptions {
            player: &player,
            output,
            zip,
        },
    ) {
        Ok(report) => {
            println!(
                "exported to {} ({} files)",
                report.game_dir.display(),
                report.files_copied
            );
            println!("run it with: {}", report.executable.display());
            if let Some(zip_file) = report.zip_file {
                println!("archive: {}", zip_file.display());
            }
            ExitCode::SUCCESS
        }
        Err(err) => fail(&format!("export: {err}")),
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
    use aigs_runtime::{AppConfig, AssetStore, AudioPlayer, GamePlayer, SaveData, ScriptHost};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Autosave every 10 seconds of simulated time at the fixed 60 Hz tick.
    const AUTOSAVE_INTERVAL_TICKS: u64 = 600;

    let project = match Project::load(manifest) {
        Ok(project) => project,
        Err(err) => return fail(&format!("{}: {err}", manifest.display())),
    };
    let root = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();
    let save_path = root.join("save.json");
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
    let save_path_setup = save_path.clone();
    let save_path_update = save_path.clone();
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
            let audio = AudioPlayer::load(&root, &project_for_setup.assets);
            let mut scripts = ScriptHost::load(&root, &project_for_setup.assets);
            match SaveData::load(&save_path_setup) {
                Ok(Some(save)) => {
                    println!(
                        "save: loaded ({} entities, {:.0}s since last save)",
                        save.scripts.len(),
                        save.offline_seconds()
                    );
                    scripts.set_offline_seconds(save.offline_seconds());
                    scripts.import_memory(save.scripts);
                }
                Ok(None) => println!("save: none found, starting fresh"),
                Err(error) => eprintln!("warning: {error} (starting fresh)"),
            }
            match GamePlayer::new(&project_for_setup, scenes, store, audio, scripts, world) {
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
                // Periodic autosave (milestone M13): no clean-shutdown hook
                // yet, so the worst case is losing up to this interval.
                if time.tick % AUTOSAVE_INTERVAL_TICKS == 0 && time.tick > 0 {
                    let save = SaveData::now(game.snapshot_scripts());
                    if let Err(error) = save.write(&save_path_update) {
                        eprintln!("warning: autosave failed: {error}");
                    }
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
