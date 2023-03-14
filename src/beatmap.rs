use anyhow::{anyhow, Result};
use osu_file_parser::{Decimal, OsuFile};
use std::{collections::VecDeque, num::ParseFloatError, time::Duration};

use bevy_ecs::prelude::Entity;

use crate::{hit_object::HitObject, hit_score::HitScoreNumber, minecraft::to_ticks};

pub struct Beatmap {
    pub data: BeatmapData,
    pub state: BeatmapState,
}

pub struct BeatmapData {
    pub od: OverallDifficulty,
    pub ar: ApproachRate,
    pub cs: CircleSize,
    pub hit_objects: Vec<HitObject>,
}

#[derive(Clone, Default, Debug)]
pub struct BeatmapState {
    pub play_time: Duration,
    pub hits300: usize,
    pub hits100: usize,
    pub hits50: usize,
    pub misses: usize,
    pub active_hit_objects: VecDeque<Entity>,
    pub next_hit_object_idx: usize,
}

#[derive(Copy, Clone)]
pub struct OverallDifficulty(pub f64);

#[derive(Copy, Clone)]
pub struct ApproachRate(pub f64);

#[derive(Copy, Clone)]
pub struct CircleSize(pub f64);

impl TryFrom<OsuFile> for Beatmap {
    type Error = anyhow::Error;

    fn try_from(osu_file: OsuFile) -> std::result::Result<Self, Self::Error> {
        let difficulty = osu_file.difficulty.clone().unwrap_or_default();
        let to_f64 =
            |decimal: Decimal| -> Result<f64, ParseFloatError> { decimal.to_string().parse() };

        Ok(Self {
            data: BeatmapData {
                od: OverallDifficulty(to_f64(
                    difficulty
                        .overall_difficulty
                        .ok_or(anyhow!("beatmap does not contain overall difficulty"))?
                        .into(),
                )?),
                cs: CircleSize(to_f64(
                    difficulty
                        .circle_size
                        .ok_or(anyhow!("beatmap does not contain circle size"))?
                        .into(),
                )?),
                ar: ApproachRate(to_f64(
                    difficulty
                        .approach_rate
                        .ok_or(anyhow!("beatmap does not contain approach rate"))?
                        .into(),
                )?),
                hit_objects: HitObject::from(&osu_file)?,
            },
            state: Default::default(),
        })
    }
}

/// https://osu.ppy.sh/wiki/en/Beatmap/Approach_rate
impl ApproachRate {
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

    /// Since I don't know how to fade-in blocks, I will consider that the preempt duration starts at halfway through the fade-in phase
    pub fn to_mc_duration(self) -> Duration {
        (self.to_preempt_duration() + self.to_fade_in_duration()) / 2
    }

    pub fn to_mc_ticks(self, tps: usize) -> usize {
        to_ticks(tps, self.to_mc_duration())
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::osu::Hitwindow;

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
