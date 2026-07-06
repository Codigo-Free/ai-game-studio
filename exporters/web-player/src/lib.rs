//! Web player (milestone M14): the generic wasm module that runs any
//! exported `.aigs` project in the browser.
//!
//! Mirrors the Desktop self-player design (M7): this is one build artifact,
//! not compiled per-project. At startup it `fetch`es `data/game.aigs` and
//! everything it references, relative to wherever `index.html` is served
//! from — exactly the same `data/` layout the Desktop exporter writes next
//! to its executable (see `exporters/desktop`).
//!
//! No project-specific persistence yet: there is no `save.json` equivalent
//! on Web (no filesystem), so games needing cross-session state should
//! treat that as a known limitation of this exporter (see SPEC.md).

use std::cell::RefCell;
use std::rc::Rc;

use aigs_project::{AssetKind, Project, Scene};
use aigs_runtime::{
    AppConfig, AssetSource, AssetStore, AudioPlayer, GamePlayer, MemoryAssets, ScriptHost,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = boot().await {
            web_sys::console::error_1(&format!("aigs: failed to start: {error}").into());
        }
    });
}

async fn boot() -> Result<(), String> {
    let manifest_text = fetch_text("data/game.aigs").await?;
    let project = Project::from_json(&manifest_text).map_err(|e| format!("game.aigs: {e}"))?;

    let mut assets = MemoryAssets::new();
    for scene_path in &project.scenes {
        let text = fetch_text(&format!("data/{scene_path}")).await?;
        assets.insert(scene_path.clone(), text.into_bytes());
    }
    for asset in &project.assets {
        let bytes = fetch_bytes(&format!("data/{}", asset.path)).await?;
        assets.insert(asset.path.clone(), bytes);
    }

    let mut scenes = std::collections::HashMap::new();
    for scene_path in &project.scenes {
        let text = String::from_utf8(assets.read(scene_path).unwrap())
            .map_err(|e| format!("{scene_path}: not valid UTF-8: {e}"))?;
        let scene = Scene::from_json(&text).map_err(|e| format!("{scene_path}: {e}"))?;
        scenes.insert(scene_path.clone(), scene);
    }

    // Audio and scripts read through the same in-memory bundle; only image
    // bytes wait for the renderer, created inside `run`'s `setup` closure.
    let audio = AudioPlayer::load(&assets, &project.assets);
    let scripts = ScriptHost::load(&assets, &project.assets);
    let has_audio_assets = project.assets.iter().any(|a| a.kind == AssetKind::Audio);
    if has_audio_assets && !audio.is_enabled() {
        web_sys::console::warn_1(&"aigs: audio disabled (no output device)".into());
    }

    let config = AppConfig {
        title: project.name.clone(),
        ..AppConfig::default()
    };
    // `setup` builds the `GamePlayer` (it needs the renderer, only available
    // once WebGPU/WebGL init resolves); `update` drives it every tick. Same
    // `Rc<RefCell<Option<_>>>` bridge `cli/src/main.rs` uses natively.
    let game: Rc<RefCell<Option<GamePlayer<AssetStore>>>> = Rc::default();
    let game_setup = Rc::clone(&game);
    aigs_runtime::run(
        config,
        move |world, renderer| match AssetStore::load(renderer, &assets, &project.assets) {
            Ok(store) => match GamePlayer::new(&project, scenes, store, audio, scripts, world) {
                Ok(player) => {
                    for warning in player.warnings() {
                        web_sys::console::warn_1(&warning.into());
                    }
                    *game_setup.borrow_mut() = Some(player);
                }
                Err(error) => web_sys::console::error_1(&format!("aigs: {error}").into()),
            },
            Err(error) => web_sys::console::error_1(&format!("aigs: asset error: {error}").into()),
        },
        move |world, time, input| {
            if let Some(player) = game.borrow_mut().as_mut() {
                player.update(world, time, input);
            }
        },
    )
    .map_err(|e| e.to_string())
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let bytes = fetch_bytes(url).await?;
    String::from_utf8(bytes).map_err(|e| format!("{url}: not valid UTF-8: {e}"))
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("no window (not running in a browser)")?;
    let response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("{url}: fetch failed: {e:?}"))?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| format!("{url}: unexpected fetch response"))?;
    if !response.ok() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|e| format!("{url}: {e:?}"))?,
    )
    .await
    .map_err(|e| format!("{url}: {e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}
