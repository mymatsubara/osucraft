use std::time::Duration;

use valence::{instance::ChunkEntry, prelude::*};

use crate::minecraft::to_ticks;

const DEFAULT_SCREEN_SIZE: (f64, f64) = (640.0, 480.0);
const DEFAULT_SPAWN_POS: DVec3 = DVec3::new(DEFAULT_SCREEN_SIZE.0 / 2.0, 240.0, -450.0);

#[derive(Resource)]
pub struct Osu {
    scale: f64,
    cur_beatmap: Option<Beatmap>,
}

pub struct Beatmap {
    pub od: OverallDifficulty,
    pub ar: ApproachRate,
    pub cs: CircleSize,
}

#[derive(Copy, Clone)]
pub struct OverallDifficulty(pub f64);

#[derive(Copy, Clone)]
pub struct ApproachRate(pub f64);

#[derive(Copy, Clone)]
pub struct CircleSize(pub f64);

#[derive(PartialEq, Eq, Debug)]
pub struct Hitwindow {
    pub window_300: Duration,
    pub window_100: Duration,
    pub window_50: Duration,
}

pub enum HitScore {
    Hit300,
    Hit100,
    Hit50,
    Miss,
}

impl Osu {
    pub fn new(scale: f64) -> Self {
        Self {
            scale,
            cur_beatmap: None,
        }
    }

    pub fn init(&self, instance: &mut Instance) {
        self.init_chunks(instance);
        self.init_screen(instance);
        self.init_player_spawn(instance);
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

/// https://osu.ppy.sh/wiki/en/Beatmap/Approach_rate
impl ApproachRate {
    /// Since I don't know how to fade-in blocks, I will consider that the preempt duration starts at halfway through the fade-in phase
    pub fn to_preempt_duration(self) -> Duration {
        let ar = self.0;
        let ms = if ar < 5.0 {
            1200.0 + 600.0 * (5.0 - ar) / 5.0
        } else if ar == 5.0 {
            1200.0
        } else {
            1200.0 - 750.0 * (ar - 5.0) / 5.0
        };

        Duration::from_millis(ms as u64)
    }

    pub fn to_fade_in_duration(self) -> Duration {
        let ar = self.0;
        let ms = if ar < 5.0 {
            800.0 + 400.0 * (5.0 - ar) / 5.0
        } else if ar == 5.0 {
            800.0
        } else {
            800.0 - 500.0 * (ar - 5.0) / 5.0
        };

        Duration::from_millis(ms as u64)
    }

    pub fn to_mc_preempt_ticks(self, tps: usize) -> usize {
        to_ticks(
            tps,
            (self.to_preempt_duration() + self.to_fade_in_duration()) / 2,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn od_hitwindow() {
        let hitwindow: Hitwindow = OverallDifficulty(10.0).into();
        assert_eq!(
            hitwindow,
            Hitwindow {
                window_300: Duration::from_millis(20),
                window_100: Duration::from_millis(60),
                window_50: Duration::from_millis(100)
            }
        );

        let hitwindow: Hitwindow = OverallDifficulty(5.0).into();
        assert_eq!(
            hitwindow,
            Hitwindow {
                window_300: Duration::from_millis(50),
                window_100: Duration::from_millis(100),
                window_50: Duration::from_millis(150)
            }
        );

        let hitwindow: Hitwindow = OverallDifficulty(1.0).into();
        assert_eq!(
            hitwindow,
            Hitwindow {
                window_300: Duration::from_millis(74),
                window_100: Duration::from_millis(132),
                window_50: Duration::from_millis(190)
            }
        );
    }

    #[test]
    fn ar_duration() {
        let ar = ApproachRate(10.0);
        let preempt = ar.to_preempt_duration();
        let fade_in = ar.to_fade_in_duration();
        assert_eq!(preempt, Duration::from_millis(450));
        assert_eq!(fade_in, Duration::from_millis(300));

        let ar = ApproachRate(5.0);
        let preempt = ar.to_preempt_duration();
        let fade_in = ar.to_fade_in_duration();
        assert_eq!(preempt, Duration::from_millis(1200));
        assert_eq!(fade_in, Duration::from_millis(800));

        let ar = ApproachRate(1.0);
        let preempt = ar.to_preempt_duration();
        let fade_in = ar.to_fade_in_duration();
        assert_eq!(preempt, Duration::from_millis(1680));
        assert_eq!(fade_in, Duration::from_millis(1120));
    }
}
