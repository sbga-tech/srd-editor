use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::ruhuna::{RuhunaFont, RuhunaSamplerState};
use crate::texture::TextureList;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderAssetError(pub String);

impl fmt::Display for RenderAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RenderAssetError {}

/// Backend-neutral DDS byte sources for the texture indices used by one SRD
/// draw set. GPU backends translate these bytes into their native resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrdTextureSourceSet {
    sources: Vec<Option<Vec<u8>>>,
}

impl SrdTextureSourceSet {
    pub fn load_required(
        game_data_root: &Path,
        definitions: &TextureList,
        required_indices: impl IntoIterator<Item = usize>,
    ) -> Result<Self, RenderAssetError> {
        let required = required_indices.into_iter().collect::<BTreeSet<_>>();
        let mut sources = vec![None; definitions.textures.len()];
        for index in required {
            let definition = definitions.textures.get(index).ok_or_else(|| {
                RenderAssetError(format!("required TEX index {index} is outside TEXL"))
            })?;
            let path = definition
                .external_dds_path(game_data_root)
                .map_err(|error| RenderAssetError(error.to_string()))?;
            let bytes = fs::read(&path).map_err(|error| {
                RenderAssetError(format!("failed to read {}: {error}", path.display()))
            })?;
            sources[index] = Some(bytes);
        }
        Ok(Self { sources })
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<&[u8]>> {
        self.sources.iter().map(Option::as_deref)
    }
}

/// Backend-neutral DDS pages and exact sampler metadata for one Ruhuna font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelAtlasSourceSet {
    pages: Vec<Vec<u8>>,
    sampler: RuhunaSamplerState,
}

impl FennelAtlasSourceSet {
    pub fn from_font(font: &RuhunaFont) -> Result<Self, RenderAssetError> {
        if font.textures.len() != 1 {
            return Err(RenderAssetError(format!(
                "Fennel atlas currently requires the binary-proven single TextureResource, found {}",
                font.textures.len()
            )));
        }
        let texture = &font.textures[0];
        let pages = font
            .atlas_pages(texture)
            .map_err(|error| RenderAssetError(error.to_string()))?
            .iter()
            .map(|page| font.atlas_page_bytes(page).to_vec())
            .collect();
        let sampler = font
            .atlas_sampler_state(texture)
            .map_err(|error| RenderAssetError(error.to_string()))?;
        Ok(Self { pages, sampler })
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn pages(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.pages.iter().map(Vec::as_slice)
    }

    pub fn sampler(&self) -> RuhunaSamplerState {
        self.sampler
    }
}
