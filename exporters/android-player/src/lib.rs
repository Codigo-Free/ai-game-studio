//! Android player (milestone M15): the generic native library that runs an
//! exported `.aigs` project on Android via WGPU/Vulkan.
//!
//! Mirrors the Desktop (M7) and Web (M14) self-player design: this is one
//! build artifact, not compiled per-project. Unlike Web, reading the
//! project's bytes is synchronous here — they're bundled straight into the
//! APK's `assets/` at export time (see `exporters/android-player/assets/data`
//! and `exporters/android`), so there's no `fetch`-style round trip needed;
//! `AndroidAssets` reads through the NDK's `AAssetManager` like `PathBuf`
//! reads through `std::fs` on Desktop.
//!
//! No project-specific persistence yet: same limitation as Web (see
//! SPEC.md) — there is no `save.json` equivalent here either.

use aigs_project::{AssetKind, Project, Scene};
use aigs_runtime::{
    AndroidAssets, AppConfig, AssetSource, AssetStore, AudioPlayer, GamePlayer, ScriptHost,
};
use android_activity::AndroidApp;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    std::panic::set_hook(Box::new(|info| log::error!("aigs: panic: {info}")));

    if let Err(error) = boot(app) {
        log::error!("aigs: failed to start: {error}");
    }
}

fn boot(app: AndroidApp) -> Result<(), String> {
    let assets = AndroidAssets::new(&app);
    let manifest_text = assets
        .read_to_string("game.aigs")
        .map_err(|e| format!("game.aigs: {e}"))?;
    let project = Project::from_json(&manifest_text).map_err(|e| format!("game.aigs: {e}"))?;

    let mut scenes = std::collections::HashMap::new();
    for scene_path in &project.scenes {
        let text = assets
            .read_to_string(scene_path)
            .map_err(|e| format!("{scene_path}: {e}"))?;
        let scene = Scene::from_json(&text).map_err(|e| format!("{scene_path}: {e}"))?;
        scenes.insert(scene_path.clone(), scene);
    }

    let audio = AudioPlayer::load(&assets, &project.assets);
    let scripts = ScriptHost::load(&assets, &project.assets);
    let has_audio_assets = project.assets.iter().any(|a| a.kind == AssetKind::Audio);
    if has_audio_assets && !audio.is_enabled() {
        log::warn!("aigs: audio disabled (no output device)");
    }

    let config = AppConfig {
        title: project.name.clone(),
        ..AppConfig::default()
    };
    // `setup`/`update` bridged through an `Rc<RefCell<_>>`, same pattern as
    // the CLI (Desktop) and the Web player.
    let game: std::rc::Rc<std::cell::RefCell<Option<GamePlayer<AssetStore>>>> = Default::default();
    let game_setup = std::rc::Rc::clone(&game);
    aigs_runtime::run_android(
        app,
        config,
        move |world, renderer| match AssetStore::load(renderer, &assets, &project.assets) {
            Ok(store) => match GamePlayer::new(&project, scenes, store, audio, scripts, world) {
                Ok(player) => {
                    for warning in player.warnings() {
                        log::warn!("aigs: {warning}");
                    }
                    *game_setup.borrow_mut() = Some(player);
                }
                Err(error) => log::error!("aigs: {error}"),
            },
            Err(error) => log::error!("aigs: asset error: {error}"),
        },
        move |world, time, input| {
            if let Some(player) = game.borrow_mut().as_mut() {
                player.update(world, time, input);
            }
        },
    )
    .map_err(|e| e.to_string())
}
