use anyhow::{Context, Result};
use osu_file_parser::{General, OsuFile};
use std::{
    ffi::OsString,
    fs::{read_to_string, File},
    path::{Path, PathBuf},
    time::Duration,
};

use valence::{instance::ChunkEntry, prelude::*};

use crate::{
    audio::AudioPlayer,
    beatmap::{Beatmap, BeatmapData, OverallDifficulty},
};

const DEFAULT_SCREEN_SIZE: (f64, f64) = (640.0, 480.0);
const DEFAULT_SPAWN_POS: DVec3 = DVec3::new(DEFAULT_SCREEN_SIZE.0 / 2.0, 240.0, -450.0);
const OSU_DEFAULT_AUDIO_FILE: &str = "audio.mp3";

#[derive(Resource)]
pub struct Osu {
    scale: f64,
    cur_beatmap: Option<Beatmap>,
    audio_player: AudioPlayer,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Hitwindow {
    pub window_300: Duration,
    pub window_100: Duration,
    pub window_50: Duration,
}

#[derive(Debug, Copy, Clone)]
pub enum HitScore {
    Hit300,
    Hit100,
    Hit50,
    Miss,
}

impl Osu {
    pub fn new(scale: f64, audio_player: AudioPlayer) -> Self {
        Self {
            scale,
            cur_beatmap: None,
            audio_player,
        }
    }

    pub fn init(&self, instance: &mut Instance) {
        self.init_chunks(instance);
        self.init_screen(instance);
        self.init_player_spawn(instance);
    }

    pub fn play(&mut self, beatmap_path: impl AsRef<Path>) -> Result<()> {
        let beatmap_path = beatmap_path.as_ref();
        let osu_file = read_to_string(beatmap_path)?.parse::<OsuFile>()?;

        let audio_file = osu_file
            .general
            .clone()
            .and_then(|g| g.audio_filename.map(|f| f.into()))
            .unwrap_or_else(|| PathBuf::from(OSU_DEFAULT_AUDIO_FILE));

        self.cur_beatmap = Some(Beatmap::try_from(osu_file)?);

        let audio_path = beatmap_path
            .parent()
            .with_context(|| "beatmap path does not contain parent directory")?
            .join(audio_file);

        // Start playing music
        self.audio_player.set_music(audio_path)?;
        self.audio_player.play();

        Ok(())
    }

    fn init_chunks(&self, instance: &mut Instance) {
        let (max_x, _) = self.screen_size();
        let max_z = self.player_spawn_pos().z as i32;

        for x in -1..=(max_x / 16) + 1 {
            for z in (max_z / 16) - 1..=1 {
                if let ChunkEntry::Vacant(chunk) = instance.chunk_entry([x, z]) {
                    chunk.insert(Default::default());
                }
            }
        }
    }

    fn init_screen(&self, instance: &mut Instance) {
        let (max_x, max_y) = self.screen_size();

        for x in 0..=max_x {
            for y in 0..=max_y {
                instance.set_block(
                    BlockPos { x, y, z: 1 },
                    Block::new(BlockState::BLACK_CONCRETE),
                );
            }
        }
    }

    fn init_player_spawn(&self, instance: &mut Instance) {
        let spawn_pos = self.player_spawn_pos();

        let block_pos = BlockPos {
            x: spawn_pos.x as i32,
            y: spawn_pos.y as i32 - 1,
            z: spawn_pos.z as i32,
        };

        instance.set_block(block_pos, Block::new(BlockState::BEDROCK));
    }

    fn screen_size(&self) -> (i32, i32) {
        let x = (DEFAULT_SCREEN_SIZE.0 * self.scale) as i32;
        let y = (DEFAULT_SCREEN_SIZE.1 * self.scale) as i32;

        (x, y)
    }

    pub fn player_spawn_pos(&self) -> DVec3 {
        DEFAULT_SPAWN_POS * self.scale
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn tick(&mut self) {
        if let Some(beatmap) = &mut self.cur_beatmap {}
    }

    pub fn has_finished_music(&self) -> bool {
        self.audio_player.has_finished()
    }
}

// https://osu.ppy.sh/wiki/en/Beatmap/Overall_difficulty
impl From<OverallDifficulty> for Hitwindow {
    fn from(od: OverallDifficulty) -> Self {
        Hitwindow {
            window_300: Duration::from_millis((80.0 - 6.0 * od.0) as u64),
            window_100: Duration::from_millis((140.0 - 8.0 * od.0) as u64),
            window_50: Duration::from_millis((200.0 - 10.0 * od.0) as u64),
        }
    }
}
