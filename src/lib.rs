pub mod animation;
mod animation_persistence;
pub mod attribute;
pub mod avts;
pub mod camera;
pub mod csli;
pub mod dds;
pub mod document;
pub mod editor;
pub mod fennel;
pub mod game_host;
pub mod image;
pub mod number;
pub mod projection;
pub mod reference;
pub mod reference_runtime;
pub mod renderer;
pub mod rfz;
pub mod ruhuna;
pub mod scene;
pub mod srplayer_runtime;
pub mod surf_file_table;
pub mod target_pass;
pub mod text;
pub mod texture;
pub mod transform;
pub mod vtbf;
pub mod yabx;

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};

    pub(crate) fn game_data_path(relative: impl AsRef<Path>) -> Option<PathBuf> {
        let Some(root) = std::env::var_os("GAME_DATA_CORPUS") else {
            eprintln!("skipping: GAME_DATA_CORPUS is not set");
            return None;
        };
        Some(PathBuf::from(root).join(relative))
    }
}
