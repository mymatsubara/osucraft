use anyhow::{Context, Result};
use osu_file_parser::OsuFile;
use std::{cmp::max, fs::read_to_string, path::PathBuf, time::Duration};
use tracing::{error, warn};

use valence::{
    client::event::{DropItem, StartSneaking, SwapItemInHand, SwingArm},
    instance::ChunkEntry,
    prelude::*,
    protocol::{
        packets::s2c::play::BossBar,
        types::{BossBarAction, BossBarColor, BossBarDivision, BossBarFlags, SoundCategory},
        Sound,
    },
    Despawned,
};

use crate::{
    audio::AudioPlayer,
    beatmap::{audio_path_from, Beatmap, OverallDifficulty},
    beatmap_selection::BeatmapSelectionInventory,
    hit_score::HitScore,
    hitcircle::Hitcircle,
    ring::Ring,
    song_selection::SongSelectionInventory,
};

const SCREEN_MARGIN_RATIO: f64 = 0.5;
const DEFAULT_SCREEN_SIZE: (f64, f64) = (640.0, 480.0);
const DEFAULT_SPAWN_POS: DVec3 = DVec3::new(
    DEFAULT_SCREEN_SIZE.0 / 1.75,
    DEFAULT_SCREEN_SIZE.1 * (1.0 + 2.0 * SCREEN_MARGIN_RATIO) / 2.25,
    -500.0,
);

#[derive(Component)]
pub struct OsuInstance;

#[derive(Resource)]
pub struct Osu {
    scale: f64,
    screen_z: f64,
    audio_player: AudioPlayer,
    life_bar_uuid: Uuid,
    state: Option<OsuState>,
    beatmap_selection_data: Option<BeatmapSelectionData>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Hitwindow {
    pub window_300: Duration,
    pub window_100: Duration,
    pub window_50: Duration,
}

#[derive(Clone)]
pub enum OsuState {
    SongSelection,
    BeatmapSelection,
    PrePlaying { ticks_left: usize, beatmap: Beatmap },
    Playing(Beatmap),
    ScoreDisplay,
}

#[derive(Clone)]
pub struct BeatmapSelectionData {
    pub beatmap_dir: PathBuf,
    pub beatmaps: Vec<OsuFile>,
}

pub enum OsuStateChange {
    SongSelection,
    BeatmapSelection(BeatmapSelectionData),
    PrePlaying { beatmap_path: PathBuf },
    Playing(Beatmap),
    ScoreDisplay(Beatmap),
    Failed,
}

impl Osu {
    pub fn new(scale: f64, audio_player: AudioPlayer) -> Self {
        Self {
            scale,
            screen_z: 0.0,
            state: None,
            life_bar_uuid: Uuid::new_v4(),
            audio_player,
            beatmap_selection_data: None,
        }
    }

    pub fn init(&self, instance: &mut Instance) {
        self.init_chunks(instance);
        self.init_screen(instance);
        self.init_player_spawn(instance);
    }

    pub fn change_state(
        &mut self,
        state_change: OsuStateChange,
        clients: &mut Query<&mut Client>,
    ) -> Result<()> {
        self.audio_player.stop();
        let mut go_to_beatmap_selection = |messages: Vec<Text>| -> Result<()> {
            for mut client in clients.iter_mut() {
                for text in messages.iter() {
                    client.send_message(text.clone());
                }
                client.write_packet(&BossBar {
                    id: self.life_bar_uuid,
                    action: BossBarAction::Remove,
                });
            }

            if let Some(beatmap_selection_data) = self.beatmap_selection_data.take() {
                self.change_state(
                    OsuStateChange::BeatmapSelection(beatmap_selection_data),
                    clients,
                )?;
            } else {
                self.change_state(OsuStateChange::SongSelection, clients)?;
            }

            Ok(())
        };

        match state_change {
            OsuStateChange::SongSelection => {
                self.state = Some(OsuState::SongSelection);
            }
            OsuStateChange::BeatmapSelection(data) => {
                if let Some(osu_file) = data.beatmaps.first() {
                    if let Some(audio_path) = audio_path_from(osu_file, data.beatmap_dir.clone()) {
                        self.audio_player.set_music(audio_path)?;
                        self.audio_player.play();
                    }
                }

                self.beatmap_selection_data = Some(data);
                self.state = Some(OsuState::BeatmapSelection);
            }
            OsuStateChange::PrePlaying { beatmap_path } => {
                let osu_file = read_to_string(&beatmap_path)?.parse::<OsuFile>()?;
                let beatmap_dir = beatmap_path
                    .parent()
                    .with_context(|| "beatmap path does not contain parent directory")?;
                let beatmap = Beatmap::try_from(osu_file, beatmap_dir.to_path_buf())?;
                let time_per_tick = 1000 / 20;
                let ticks_left = beatmap
                    .data
                    .hit_objects
                    .first()
                    .map(|hit_object| max((3000 - hit_object.time() as i32) / time_per_tick, 0))
                    .unwrap_or(60) as usize;

                self.state = Some(OsuState::PrePlaying {
                    beatmap,
                    ticks_left,
                })
            }
            OsuStateChange::Playing(beatmap) => {
                // Start playing music
                self.audio_player.set_music(&beatmap.data.audio_path)?;
                self.audio_player.play();

                self.state = Some(OsuState::Playing(beatmap));
            }
            OsuStateChange::ScoreDisplay(beatmap) => {
                let score_texts = beatmap.score_text();
                go_to_beatmap_selection(score_texts)?;
            }
            OsuStateChange::Failed => {
                let messages = vec!["Beatmap failed!".color(Color::RED)];
                go_to_beatmap_selection(messages)?;
            }
        };

        Ok(())
    }

    pub fn get_action_bar(&self, tps: usize) -> Text {
        match self.state {
            Some(OsuState::SongSelection) => {
                "Sneak<LEFT SHIFT>".color(Color::GOLD)
                    + " to open".color(Color::WHITE)
                    + " SONG SELECTION".color(Color::AQUA)
            }
            Some(OsuState::BeatmapSelection) => {
                "Sneak<LEFT SHIFT>".color(Color::GOLD)
                    + " to open".color(Color::WHITE)
                    + " BEATMAP SELECTION".color(Color::AQUA)
            }
            Some(OsuState::PrePlaying { ticks_left, .. }) => {
                "Beatmap will start in".color(Color::WHITE)
                    + format!(" {}", ticks_left / tps + 1).color(Color::AQUA)
                    + " seconds".color(Color::WHITE)
            }
            _ => "".into(),
        }
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

    pub fn init_inventory_selections(world: &mut World) {
        match SongSelectionInventory::new() {
            Ok(song_selection) => {
                world.spawn(song_selection);
            }
            Err(error) => error!("Error while setting up song selection: {}", error),
        };

        world.spawn(BeatmapSelectionInventory::new());
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
    mut clients: Query<&mut Client>,
    mut instances_set: ParamSet<(
        Query<(Entity, &mut Instance), With<OsuInstance>>,
        Query<(Entity, &mut Instance)>,
    )>,
    song_selections: Query<Entity, (With<SongSelectionInventory>, With<Inventory>)>,
    beatmap_selections: Query<Entity, (With<BeatmapSelectionInventory>, With<Inventory>)>,
    mut swing_arm_events: EventReader<SwingArm>,
    mut drop_item_events: EventReader<DropItem>,
    mut swap_item_hand_events: EventReader<SwapItemInHand>,
    mut sneaking_events: EventReader<StartSneaking>,
) {
    if instances_set.p0().get_single().is_err() {
        warn!("Server should have one OsuInstance");
        return;
    };

    let prev_state = osu.state.clone();
    let tps = server.shared().tps() as usize;
    let possible_state_change: Result<Option<OsuStateChange>> = match prev_state {
        None => Ok(Some(OsuStateChange::SongSelection)),
        Some(OsuState::SongSelection) => {
            for sneaking_event in sneaking_events.iter() {
                match song_selections.get_single() {
                    Ok(inventory_entity) => {
                        commands
                            .entity(sneaking_event.client)
                            .insert(OpenInventory::new(inventory_entity));
                    }
                    Err(_) => {
                        error!("Could not find a SongSelectionInventory component");
                    }
                }
            }

            Ok(None)
        }
        Some(OsuState::BeatmapSelection) => {
            for sneaking_event in sneaking_events.iter() {
                match beatmap_selections.get_single() {
                    Ok(inventory_entity) => {
                        commands
                            .entity(sneaking_event.client)
                            .insert(OpenInventory::new(inventory_entity));
                    }
                    Err(_) => {
                        error!("Could not find a SongSelectionInventory component");
                    }
                }
            }

            Ok(None)
        }
        Some(OsuState::ScoreDisplay) => Ok(None),
        Some(OsuState::PrePlaying {
            beatmap,
            ticks_left,
        }) => {
            if ticks_left == 0 {
                Ok(Some(OsuStateChange::Playing(beatmap)))
            } else {
                osu.state = Some(OsuState::PrePlaying {
                    ticks_left: ticks_left - 1,
                    beatmap,
                });

                Ok(None)
            }
        }
        Some(OsuState::Playing(mut beatmap)) => {
            // Beatmap has finished
            if beatmap.state.active_hit_objects.is_empty()
                && beatmap.state.next_hit_object_idx >= beatmap.data.hit_objects.len()
                && osu.audio_player.has_finished()
            {
                Ok(Some(OsuStateChange::ScoreDisplay(beatmap)))
            }
            // Failed beatmap
            else if beatmap.state.health <= 0.0 {
                Ok(Some(OsuStateChange::Failed))
            }
            // Beatmap is playing
            else {
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
                    beatmap.state.combo = 0;
                    // Update health
                    beatmap.state.health =
                        beatmap.data.hp.drain(beatmap.state.health, HitScore::Miss);

                    for mut client in &mut clients {
                        play_hit_sound(&mut client, HitScore::Miss);
                    }
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
                    for clicked_client_entity in swing_arm_events
                        .iter()
                        .map(|e| e.client)
                        .chain(swap_item_hand_events.iter().map(|e| e.client))
                        .chain(drop_item_events.iter().map(|e| e.client))
                    {
                        let Ok(mut clicked_client) = clients.get_mut(clicked_client_entity) else {
                        continue;
                    };

                        if let Ok(hitcircle) = hitcircles.get(hitcircle_entity) {
                            if let Some(hit) = hitcircle.hit_score(&clicked_client, &rings) {
                                // Update score (https://osu.ppy.sh/wiki/en/Gameplay/Score/ScoreV1/osu%21#hit-circles)
                                let combo = beatmap.state.combo;
                                let combo_multiplier = if combo == 0 { 0 } else { combo - 1 };
                                let difficulty_multiplier = beatmap.data.difficulty_multiplier();
                                let mod_multiplier = 1.0; // Mods not implemented

                                beatmap.state.score += (hit.value() as f64
                                    * (1.0
                                        + (combo_multiplier as f64
                                            * difficulty_multiplier
                                            * mod_multiplier)
                                            / 25.0))
                                    as usize;

                                // Update hit scores
                                match hit {
                                    HitScore::Hit300 => beatmap.state.hits300 += 1,
                                    HitScore::Hit100 => beatmap.state.hits100 += 1,
                                    HitScore::Hit50 => beatmap.state.hits50 += 1,
                                    HitScore::Miss => beatmap.state.misses += 1,
                                }

                                // Update combo
                                match hit {
                                    HitScore::Hit300 | HitScore::Hit100 | HitScore::Hit50 => {
                                        beatmap.state.combo += 1;
                                        beatmap.state.max_combo =
                                            beatmap.state.max_combo.max(beatmap.state.combo);
                                    }
                                    HitScore::Miss => beatmap.state.combo = 0,
                                }

                                // Play hitsound
                                play_hit_sound(&mut clicked_client, hit);

                                // Update health
                                beatmap.state.health =
                                    beatmap.data.hp.drain(beatmap.state.health, hit);

                                // Despawn hit hitcircle
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

                // Update health bar
                for mut client in &mut clients {
                    let text = "Score: ".color(Color::GOLD)
                        + beatmap.state.score.to_string().color(Color::WHITE)
                        + "   Combo: ".color(Color::LIGHT_PURPLE)
                        + format!("x{}", beatmap.state.combo).color(Color::WHITE)
                        + "   Acc: ".color(Color::GREEN)
                        + format!("{:.2}%", beatmap.state.accuracy()).color(Color::WHITE);

                    client.write_packet(&BossBar {
                        id: osu.life_bar_uuid,
                        action: BossBarAction::Add {
                            title: text,
                            health: beatmap.state.health as f32,
                            color: BossBarColor::Blue,
                            division: BossBarDivision::TwentyNotches,
                            flags: BossBarFlags::new(),
                        },
                    });
                }

                osu.state = Some(OsuState::Playing(beatmap));
                Ok(None)
            }
        }
    };

    for mut client in &mut clients {
        client.set_action_bar(osu.get_action_bar(tps));
    }

    if let Ok(Some(state_change)) = possible_state_change {
        if let Err(error) = osu.change_state(state_change, &mut clients) {
            error!("Error while changing osu state: '{}'", error)
        }
    }
}

pub fn send_welcome_message(mut new_clients: Query<&mut Client, Added<Client>>) {
    for mut client in &mut new_clients {
        let title = "Welcome to".color(Color::AQUA) + " osucraft!".color(Color::GOLD);
        let instructions = "To hit a circle press one of the following:".color(Color::BLUE);
        let left_click = " - ".color(Color::RED)
            + "Attack".color(Color::LIGHT_PURPLE)
            + " <LEFT CLICK>".color(Color::GOLD);
        let drop_item = " - ".color(Color::RED)
            + "Drop selected item".color(Color::LIGHT_PURPLE)
            + " <Q>".color(Color::GOLD);
        let swap_item = " - ".color(Color::RED)
            + "Swap item with offhand ".color(Color::LIGHT_PURPLE)
            + " <F>".color(Color::GOLD);
        let empty: Text = "".into();
        let commands = "Commands: ".color(Color::YELLOW);
        let filter_songs = " - ".color(Color::RED)
            + "/filter-songs".color(Color::YELLOW)
            + " <keywords>".color(Color::GRAY);
        let reset_filter = " - ".color(Color::RED) + "/reset-filter".color(Color::YELLOW);

        let messages = [
            title,
            empty.clone(),
            instructions,
            left_click,
            drop_item,
            swap_item,
            empty,
            commands,
            filter_songs,
            reset_filter,
        ];

        for message in messages.into_iter() {
            client.send_message(message);
        }
    }
}

fn play_hit_sound(client: &mut Mut<Client>, hit: HitScore) {
    let (sound, category) = if matches!(hit, HitScore::Miss) {
        (Sound::EntityChickenHurt, SoundCategory::Block)
    } else {
        (Sound::EntityChickenEgg, SoundCategory::Block)
    };
    let position = client.position();
    client.play_sound(sound, category, position, 3.0, 1.0);
}
