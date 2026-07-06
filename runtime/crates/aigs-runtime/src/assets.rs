//! Asset pipeline (milestone M2): loads the assets declared in a project
//! manifest and uploads images to the GPU.

use std::collections::HashMap;

use aigs_project::{Asset, AssetKind};
use aigs_render::{Renderer, TextureId};

use crate::source::AssetSource;

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("asset \"{id}\": failed to read {path}: {source}")]
    Read {
        id: String,
        path: String,
        source: std::io::Error,
    },
    #[error("asset \"{id}\": failed to decode image {path}: {source}")]
    Decode {
        id: String,
        path: String,
        source: image::ImageError,
    },
}

/// A texture uploaded to the GPU plus its pixel dimensions.
#[derive(Debug, Clone, Copy)]
pub struct TextureInfo {
    pub id: TextureId,
    /// Full texture size in pixels.
    pub width: f32,
    pub height: f32,
    /// Spritesheet grid and frame size, when the asset declares one.
    pub sheet: Option<crate::components::SheetGrid>,
    pub frame_width: f32,
    pub frame_height: f32,
}

/// Runtime catalog of loaded assets, keyed by asset id.
#[derive(Default)]
pub struct AssetStore {
    textures: HashMap<String, TextureInfo>,
}

impl AssetStore {
    /// Loads every asset of `assets` by reading through `source` (a local
    /// directory on Desktop, prefetched bytes on Web). Non-image assets are
    /// skipped (audio has its own loader, see [`crate::AudioPlayer`]).
    pub fn load(
        renderer: &mut Renderer,
        source: &dyn AssetSource,
        assets: &[Asset],
    ) -> Result<Self, AssetError> {
        let mut store = Self::default();
        for asset in assets {
            if asset.kind != AssetKind::Image {
                continue;
            }
            let bytes = source
                .read(&asset.path)
                .map_err(|source| AssetError::Read {
                    id: asset.id.clone(),
                    path: asset.path.clone(),
                    source,
                })?;
            let decoded = image::load_from_memory(&bytes)
                .map_err(|source| AssetError::Decode {
                    id: asset.id.clone(),
                    path: asset.path.clone(),
                    source,
                })?
                .to_rgba8();
            let (width, height) = decoded.dimensions();
            let id = renderer.create_texture_rgba(width, height, decoded.as_raw());
            let sheet = asset.spritesheet.map(|sheet| crate::components::SheetGrid {
                columns: (width / sheet.frame_width.max(1)).max(1),
                rows: (height / sheet.frame_height.max(1)).max(1),
            });
            let (frame_width, frame_height) = match asset.spritesheet {
                Some(spec) => (spec.frame_width as f32, spec.frame_height as f32),
                None => (width as f32, height as f32),
            };
            store.textures.insert(
                asset.id.clone(),
                TextureInfo {
                    id,
                    width: width as f32,
                    height: height as f32,
                    sheet,
                    frame_width,
                    frame_height,
                },
            );
        }
        Ok(store)
    }

    pub fn texture(&self, asset_id: &str) -> Option<TextureInfo> {
        self.textures.get(asset_id).copied()
    }

    /// Number of loaded textures.
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}
