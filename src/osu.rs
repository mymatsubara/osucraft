use anyhow::{Context, Result};
use osu_file_parser::OsuFile;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    time::Duration,
};
use tracing::warn;

use valence::{client::event::SwingArm, instance::ChunkEntry, prelude::*, Despawned};

use crate::{
    audio::AudioPlayer,
    beatmap::{Beatmap, OverallDifficulty},
    hit_score::HitScore,
    hitcircle::Hitcircle,
    ring::Ring,
};

const SCREEN_MARGIN_RATIO: f64 = 0.5;
const DEFAULT_SCREEN_SIZE: (f64, f64) = (640.0, 480.0);
const DEFAULT_SPAWN_POS: DVec3 = DVec3::new(
    DEFAULT_SCREEN_SIZE.0 / 1.75,
    DEFAULT_SCREEN_SIZE.1 * (1.0 + 2.0 * SCREEN_MARGIN_RATIO) / 2.25,
    -450.0,
);
const OSU_DEFAULT_AUDIO_FILE: &str = "audio.mp3";

#[derive(Component)]
pub struct OsuInstance;

#[derive(Resource)]
pub struct Osu {
    scale: f64,
    screen_z: f64,
    cur_beatmap: Option<Beatmap>,
    audio_player: AudioPlayer,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Hitwindow {
    pub window_300: Duration,
    pub window_100: Duration,
    pub window_50: Duration,
}

impl Osu {
    pub fn new(scale: f64, audio_player: AudioPlayer) -> Self {
        Self {
            scale,
            screen_z: 0.0,
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
        let (screen_x, _) = self.screen_size();
        let (margin_x, _) = self.screen_margin();
        let max_x = screen_x + margin_x;
        let max_z = self.player_spawn_pos().z as i32;

        for x in -1 - (margin_x / 16)..=(max_x / 16) + 1 {
            for z in (max_z / 16) - 1..=1 {
                if let ChunkEntry::Vacant(chunk) = instance.chunk_entry([x, z]) {
                    chunk.insert(Default::default());
                }
            }
        }
    }

    fn init_screen(&self, instance: &mut Instance) {
        let (max_x, max_y) = self.screen_size();
        let (margin_x, margin_y) = self.screen_margin();

        for x in -margin_x..=max_x + margin_x {
            for y in 0..=max_y + 2 * margin_y {
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
            z: spawn_pos.z as i32 - 1,
        };

        instance.set_block(block_pos, Block::new(BlockState::BEDROCK));
    }

    fn screen_size(&self) -> (i32, i32) {
        let x = (DEFAULT_SCREEN_SIZE.0 * self.scale) as i32;
        let y = (DEFAULT_SCREEN_SIZE.1 * self.scale) as i32;

        (x, y)
    }

    fn screen_margin(&self) -> (i32, i32) {
        let screen_size = self.screen_size();
        let x = screen_size.0 as f64 * SCREEN_MARGIN_RATIO;
        let y = screen_size.1 as f64 * SCREEN_MARGIN_RATIO;

        (x as i32, y as i32)
    }

    pub fn player_spawn_pos(&self) -> DVec3 {
        DEFAULT_SPAWN_POS * self.scale
    }

    pub fn scale(&self) -> f64 {
        self.scale
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

pub fn update_osu(
    mut osu: ResMut<Osu>,
    server: Res<Server>,
    mut commands: Commands,
    hitcircles: Query<&mut Hitcircle>,
    rings: Query<&Ring>,
    clients: Query<&Client>,
    mut instances_set: ParamSet<(
        Query<(Entity, &mut Instance), With<OsuInstance>>,
        Query<(Entity, &mut Instance)>,
    )>,
    mut swing_arm_events: EventReader<SwingArm>,
) {
    if instances_set.p0().get_single().is_err() {
        warn!("Server should have one OsuInstance");
        return;
    };

    let beatmap = osu.cur_beatmap.take();

    osu.cur_beatmap = match beatmap {
        // Beatmap has finished
        Some(beatmap)
            if beatmap.state.active_hit_objects.is_empty()
                && beatmap.state.next_hit_object_idx >= beatmap.data.hit_objects.len()
                && osu.audio_player.has_finished() =>
        {
            dbg!(beatmap.state);
            None
        }
        // Beatmap is playing
        Some(mut beatmap) => {
            // Remove expired hitcircles
            let expired_hitcircles_count = beatmap
                .state
                .active_hit_objects
                .iter()
                .take_while(|&&entity| matches!(hitcircles.get(entity), Err(_)))
                .count();
            beatmap.state.misses += expired_hitcircles_count;
            for _ in 0..expired_hitcircles_count {
                beatmap.state.active_hit_objects.pop_front();
            }

            if let Some(next_hitobject) = beatmap
                .data
                .hit_objects
                .get(beatmap.state.next_hit_object_idx)
            {
                // Check we need to spawn the next hitcircle
                let play_time = osu.audio_player.play_time();
                beatmap.state.play_time = play_time;
                let look_ahead = beatmap.data.ar.to_mc_duration();
                let threshold = play_time + look_ahead;

                if threshold.as_millis() as u32 >= next_hitobject.time() {
                    // Spawn hitcircle
                    let screen_size = osu.screen_size();
                    let margin_size = osu.screen_margin();
                    let z_offset = next_hitobject.z(
                        &beatmap.data.hit_objects[beatmap.state.next_hit_object_idx + 1..],
                        beatmap.data.cs,
                    );

                    let center = DVec3::new(
                        screen_size.0 as f64 - next_hitobject.x() as f64 * osu.scale(),
                        next_hitobject.y() as f64 * osu.scale() + margin_size.1 as f64,
                        osu.screen_z + z_offset as f64,
                    );

                    let color = next_hitobject.color();
                    let scale = osu.scale;
                    let combo_number = next_hitobject.combo_number();
                    let tps = server.shared().tps() as usize;

                    let mut osu_instances = instances_set.p0();
                    let osu_instance = osu_instances.get_single_mut().unwrap();
                    match Hitcircle::from_beatmap(
                        center,
                        &beatmap.data,
                        color,
                        scale,
                        combo_number,
                        tps,
                        osu_instance,
                        &mut commands,
                    ) {
                        Ok(hitcircle) => {
                            let hitcircle_entity = commands.spawn(hitcircle).id();

                            beatmap.state.active_hit_objects.push_back(hitcircle_entity);
                            beatmap.state.next_hit_object_idx += 1;
                        }
                        Err(error) => {
                            warn!("Error while creating hitcircle: {}", error.to_string());
                        }
                    }
                }
            }

            // Check hitcircle hit
            if let Some(&hitcircle_entity) = beatmap.state.active_hit_objects.front() {
                for clicked_client in swing_arm_events
                    .iter()
                    .filter_map(|event| clients.get(event.client).ok())
                {
                    if let Ok(hitcircle) = hitcircles.get(hitcircle_entity) {
                        if let Some(hit) = hitcircle.hit_score(clicked_client) {
                            match hit {
                                HitScore::Hit300 => beatmap.state.hits300 += 1,
                                HitScore::Hit100 => beatmap.state.hits100 += 1,
                                HitScore::Hit50 => beatmap.state.hits50 += 1,
                                HitScore::Miss => beatmap.state.misses += 1,
                            }

                            dbg!(hit);

                            let mut instances = instances_set.p1();
                            commands.entity(hitcircle_entity).insert(Despawned);
                            hitcircle
                                .despawn(&mut commands, &rings, &mut instances, hit)
                                .unwrap();
                            beatmap.state.active_hit_objects.pop_front();
                        }
                    }
                }
            }

            // Update health

            Some(beatmap)
        }
        _ => None,
    };
}
