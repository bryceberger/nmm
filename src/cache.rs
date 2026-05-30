use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Result, eyre};

#[derive(Debug, Clone)]
pub struct CachePath<'a> {
    pub game: &'a Path,
    pub mod_id: u64,
    pub file_id: u64,
}

impl<'a> CachePath<'a> {
    pub fn new(game: &'a str, mod_id: u64, file_id: u64) -> Result<Self> {
        let game_path: &Path = game.as_ref();
        if !game_path.is_relative() || game_path.iter().count() != 1 {
            return Err(eyre!("invalid game name: {game}"));
        }

        Ok(Self {
            game: game_path,
            mod_id,
            file_id,
        })
    }
}

pub struct Cache {
    base: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Self> {
        let Some(cache_dir) = dirs::cache_dir() else {
            return Err(eyre!("failed to get cache dir"));
        };
        let base = cache_dir.join("nmm");
        fs::create_dir_all(&base)?;
        Ok(Self { base })
    }

    fn get_path_for(&self, path: &CachePath) -> PathBuf {
        let mut result = self.base.clone();
        result.push(path.game);
        result.push(path.mod_id.to_string());
        result.push(path.file_id.to_string());
        result
    }

    pub fn get(&self, path: &CachePath) -> Result<Option<PathBuf>> {
        let expected_path = self.get_path_for(path);
        if expected_path.try_exists()? {
            Ok(Some(expected_path))
        } else {
            Ok(None)
        }
    }

    pub fn put(&self, path: &CachePath, source_path: &Path, file_name: &str) -> Result<PathBuf> {
        let target_path = self.get_path_for(path);
        fs::create_dir_all(&target_path)?;
        fs::rename(source_path, target_path.join(file_name))?;
        Ok(target_path)
    }
}
