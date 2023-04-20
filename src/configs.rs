use anyhow::Result;
use colored::Colorize;
use directories::BaseDirs;
use std::fmt::Display;
use std::str;
use std::{fs, path::PathBuf};

use bevy_ecs::system::Resource;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Resource, Serialize, Deserialize, Debug)]
pub struct Configs {
    songs_directory: String,
}

impl Configs {
    pub fn open() -> Self {
        Self::read().unwrap_or_else(|_| {
            let default_configs = Self::default();

            if let Err(error) = default_configs.save() {
                warn!("Error while saving configs file: {}", error);
            }

            default_configs
        })
    }

    pub fn path() -> PathBuf {
        PathBuf::from("configs.json")
    }

    fn read() -> Result<Self> {
        let path = Self::path();
        let file_data = fs::read(path)?;
        let json = str::from_utf8(file_data.as_slice())?;
        Ok(serde_json::from_str(json)?)
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(Self::path(), json)?;

        Ok(())
    }

    pub fn songs_directory(&self) -> &str {
        &self.songs_directory
    }
}

impl Default for Configs {
    fn default() -> Self {
        let local_dir = BaseDirs::new()
            .map(|base_dirs| base_dirs.data_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./"));
        let songs_directory = local_dir.join("osu!").join("Songs");

        Self {
            songs_directory: songs_directory.to_str().unwrap().to_owned(),
        }
    }
}

impl Display for Configs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", "Songs directory".cyan(), self.songs_directory)
    }
}
