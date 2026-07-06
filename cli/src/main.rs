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
    /// Export the project as a standalone game folder.
    Export {
        /// Path to the project manifest (game.aigs).
        manifest: PathBuf,
        /// Directory where the game folder is created.
        #[arg(long, default_value = "dist")]
        output: PathBuf,
        /// Also produce a .zip next to the game folder (Desktop only).
        #[arg(long)]
        zip: bool,
        /// Export platform.
        #[arg(long, value_enum, default_value = "desktop")]
        target: ExportTarget,
        /// Optimized, signed release build (Android only; needs a signing
        /// keystore configured in the player template's Cargo.toml). Without
        /// this flag, Android exports are debug builds signed with a local
        /// debug keystore — fine for testing, not for distribution.
        #[arg(long)]
        release: bool,
    },
    /// Print the scripting API as machine-readable JSON: every lifecycle
    /// function and callable a `.rhai` script can use. Intended for AI
    /// agents and editor tooling (see sdk/aigs-format/scripting-api.json).
    ScriptApi,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ExportTarget {
    Desktop,
    Web,
    Android,
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
            target,
            release,
        } => match target {
            ExportTarget::Desktop => export_project_desktop(&manifest, &output, zip),
            ExportTarget::Web => export_project_web(&manifest, &output),
            ExportTarget::Android => export_project_android(&manifest, &output, release),
        },
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

fn export_project_desktop(manifest: &Path, output: &Path, zip: bool) -> ExitCode {
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

/// The web player has no wasm equivalent of "the running CLI binary" (see
/// exporters/web-player) — it's a prebuilt bundle expected next to `aigs`,
/// in a `web-player/` folder, mirroring how an exported Desktop game finds
/// `data/game.aigs` next to its own executable.
fn locate_web_player() -> Result<(PathBuf, PathBuf), String> {
    let exe = std::env::current_exe().map_err(|err| format!("cannot locate {err}"))?;
    let dir = exe
        .parent()
        .ok_or("running executable has no parent directory")?
        .join("web-player");
    let js = dir.join("aigs_web_player.js");
    let wasm = dir.join("aigs_web_player_bg.wasm");
    if !js.is_file() || !wasm.is_file() {
        return Err(format!(
            "web player bundle not found at {} (expected aigs_web_player.js + aigs_web_player_bg.wasm; \
             build exporters/web-player with wasm-bindgen and place the output there)",
            dir.display()
        ));
    }
    Ok((js, wasm))
}

fn export_project_web(manifest: &Path, output: &Path) -> ExitCode {
    let (player_js, player_wasm) = match locate_web_player() {
        Ok(paths) => paths,
        Err(err) => return fail(&err),
    };
    match aigs_export_web::export(
        manifest,
        &aigs_export_web::ExportOptions {
            player_js: &player_js,
            player_wasm: &player_wasm,
            output,
        },
    ) {
        Ok(report) => {
            println!(
                "exported to {} ({} files)",
                report.game_dir.display(),
                report.files_copied
            );
            println!("serve it with any static file server and open index.html, e.g.:");
            println!("  npx serve {}", report.game_dir.display());
            ExitCode::SUCCESS
        }
        Err(err) => fail(&format!("export: {err}")),
    }
}

/// The Android player template has no build-once artifact to copy the way
/// Web's does (see `aigs_export_android`'s module docs for why): it's the
/// `exporters/android-player` crate's source, expected next to `aigs` in an
/// `android-player-template/` folder — actually building it needs the
/// Android NDK/SDK and `cargo-apk` installed on this machine.
fn locate_android_player_template() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|err| format!("cannot locate {err}"))?;
    let dir = exe
        .parent()
        .ok_or("running executable has no parent directory")?
        .join("android-player-template");
    if !dir.join("Cargo.toml").is_file() {
        return Err(format!(
            "Android player template not found at {} (expected a copy of exporters/android-player)",
            dir.display()
        ));
    }
    Ok(dir)
}

fn export_project_android(manifest: &Path, output: &Path, release: bool) -> ExitCode {
    let player_template = match locate_android_player_template() {
        Ok(path) => path,
        Err(err) => return fail(&err),
    };
    match aigs_export_android::export(
        manifest,
        &aigs_export_android::ExportOptions {
            player_template: &player_template,
            output,
            release,
        },
    ) {
        Ok(report) => {
            println!("exported to {}", report.apk.display());
            println!("install it with: adb install {}", report.apk.display());
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
