//! Asset pipeline (milestone M2): loads the assets declared in a project
//! manifest and uploads images to the GPU.

use std::collections::HashMap;
use std::path::Path;

use aigs_project::{Asset, AssetKind};
use aigs_render::{Renderer, TextureId};

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
    pub width: f32,
    pub height: f32,
}

/// Runtime catalog of loaded assets, keyed by asset id.
#[derive(Default)]
pub struct AssetStore {
    textures: HashMap<String, TextureInfo>,
}

impl AssetStore {
    /// Loads every asset of `assets`, resolving paths relative to `root`
    /// (the directory containing `game.aigs`). Non-image assets are skipped
    /// until later milestones (audio in Phase 2).
    pub fn load(
        renderer: &mut Renderer,
        root: &Path,
        assets: &[Asset],
    ) -> Result<Self, AssetError> {
        let mut store = Self::default();
        for asset in assets {
            if asset.kind != AssetKind::Image {
                continue;
            }
            let path = root.join(&asset.path);
            let bytes = std::fs::read(&path).map_err(|source| AssetError::Read {
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
            store.textures.insert(
                asset.id.clone(),
                TextureInfo {
                    id,
                    width: width as f32,
                    height: height as f32,
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
