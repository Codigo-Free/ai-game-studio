//! Audio (milestone M9): sound effects and per-scene music via kira.
//!
//! On machines without an audio device (CI, headless) the player degrades
//! to a disabled no-op so games and tests still run.
//!
//! Supported audio file formats: `wav`, `mp3`, `ogg` and `flac` — kira
//! features enabled to match the editor's own `AUDIO_EXTENSIONS`
//! (`editor/src/ipc.ts`), since the editor happily imports/previews any of
//! those (it just uses the browser's own `<audio>` tag) but the runtime
//! can only play what kira's enabled features actually decode. Anything
//! else fails with a warning pushed to `AudioPlayer::take_warnings` rather
//! than a crash — this was a real gap hit live (an imported `.mp3` failed
//! to play before `mp3` was added here).

use std::collections::HashMap;
use std::io::Cursor;

use aigs_project::{Asset, AssetKind, Music};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween};

use crate::source::AssetSource;

/// Converts a linear volume (`0.0..=1.0`) to decibels.
fn to_decibels(volume: f32) -> Decibels {
    if volume <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * volume.log10())
    }
}

/// Plays sound effects and scene music. Load once per project.
pub struct AudioPlayer {
    manager: Option<AudioManager<DefaultBackend>>,
    sounds: HashMap<String, StaticSoundData>,
    music: Option<StaticSoundHandle>,
    current_music: Option<String>,
    warnings: Vec<String>,
}

impl AudioPlayer {
    /// Loads every `audio` asset of the project, reading through `source`.
    /// If no audio backend is available the player is created disabled
    /// (with a warning).
    pub fn load(source: &dyn AssetSource, assets: &[Asset]) -> Self {
        let mut player = match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
        {
            Ok(manager) => Self {
                manager: Some(manager),
                sounds: HashMap::new(),
                music: None,
                current_music: None,
                warnings: Vec::new(),
            },
            Err(error) => {
                let mut disabled = Self::disabled();
                disabled
                    .warnings
                    .push(format!("audio disabled: no audio device ({error})"));
                return disabled;
            }
        };
        for asset in assets {
            if asset.kind != AssetKind::Audio {
                continue;
            }
            let bytes = match source.read(&asset.path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    player.warnings.push(format!(
                        "audio asset \"{}\" ({}): {error}",
                        asset.id, asset.path
                    ));
                    continue;
                }
            };
            match StaticSoundData::from_cursor(Cursor::new(bytes)) {
                Ok(sound) => {
                    player.sounds.insert(asset.id.clone(), sound);
                }
                Err(error) => player.warnings.push(format!(
                    "audio asset \"{}\" ({}): {error}",
                    asset.id, asset.path
                )),
            }
        }
        player
    }

    /// An audio player that does nothing (tests, headless machines).
    pub fn disabled() -> Self {
        Self {
            manager: None,
            sounds: HashMap::new(),
            music: None,
            current_music: None,
            warnings: Vec::new(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.manager.is_some()
    }

    /// Problems found while loading or playing (drained by the caller).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Number of loaded audio assets.
    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// Plays a one-shot sound effect.
    pub fn play_sound(&mut self, asset: &str, volume: f32) {
        let Some(manager) = self.manager.as_mut() else {
            return;
        };
        let Some(sound) = self.sounds.get(asset) else {
            self.warnings
                .push(format!("play_sound: unknown audio asset \"{asset}\""));
            return;
        };
        let data = sound.volume(to_decibels(volume));
        if let Err(error) = manager.play(data) {
            self.warnings
                .push(format!("play_sound \"{asset}\": {error}"));
        }
    }

    /// Starts (or keeps) the scene music. Passing the music already playing
    /// lets it continue across scene switches; `None` stops it.
    pub fn set_music(&mut self, music: Option<&Music>) {
        let next_id = music.map(|m| m.asset.clone());
        if next_id == self.current_music {
            return;
        }
        if let Some(handle) = self.music.as_mut() {
            handle.stop(Tween::default());
        }
        self.music = None;
        self.current_music = None;

        let (Some(manager), Some(music)) = (self.manager.as_mut(), music) else {
            return;
        };
        let Some(sound) = self.sounds.get(&music.asset) else {
            self.warnings
                .push(format!("music: unknown audio asset \"{}\"", music.asset));
            return;
        };
        let mut data = sound.volume(to_decibels(music.volume));
        if music.looped {
            data = data.loop_region(0.0..);
        }
        match manager.play(data) {
            Ok(handle) => {
                self.music = Some(handle);
                self.current_music = Some(music.asset.clone());
            }
            Err(error) => self
                .warnings
                .push(format!("music \"{}\": {error}", music.asset)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_player_is_a_safe_no_op() {
        let mut player = AudioPlayer::disabled();
        assert!(!player.is_enabled());
        player.play_sound("anything", 1.0);
        player.set_music(Some(&Music {
            asset: "theme".into(),
            volume: 1.0,
            looped: true,
        }));
        player.set_music(None);
        assert!(player.take_warnings().is_empty(), "no-op must not warn");
    }

    #[test]
    fn decibel_conversion_endpoints() {
        assert_eq!(to_decibels(1.0), Decibels(0.0));
        assert_eq!(to_decibels(0.0), Decibels::SILENCE);
        let half = to_decibels(0.5);
        assert!((half.0 + 6.02).abs() < 0.1, "0.5 ≈ -6 dB, got {}", half.0);
    }
}
