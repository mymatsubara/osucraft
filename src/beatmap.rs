use anyhow::{anyhow, Result};
use osu_file_parser::{Decimal, OsuFile};
use std::{cmp::min, collections::VecDeque, num::ParseFloatError, time::Duration};

use bevy_ecs::prelude::Entity;

use crate::{hit_object::HitObject, minecraft::to_ticks};

pub struct Beatmap {
    pub data: BeatmapData,
    pub state: BeatmapState,
}

pub struct BeatmapData {
    pub od: OverallDifficulty,
    pub ar: ApproachRate,
    pub cs: CircleSize,
    pub hp: HpDrainRate,
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
    pub score: usize,
    pub combo: usize,
    pub max_combo: usize,
}

#[derive(Copy, Clone)]
pub struct OverallDifficulty(pub f64);

#[derive(Copy, Clone)]
pub struct ApproachRate(pub f64);

#[derive(Copy, Clone)]
pub struct HpDrainRate(pub f64);

#[derive(Copy, Clone)]
pub struct CircleSize(pub f64);

impl BeatmapState {
    pub fn accuracy(&self) -> f32 {
        (self.hits300 as f32 * 100.0
            + self.hits100 as f32 * 100.0 / 3.0
            + self.hits50 as f32 * 100.0 / 6.0)
            / (self.hits300 + self.hits100 + self.hits50 + self.misses) as f32
    }
}

impl BeatmapData {
    /// https://osu.ppy.sh/wiki/en/Gameplay/Score/ScoreV1/osu%21#difficulty-multiplier
    pub fn difficulty_multiplier(&self) -> f64 {
        ((self.hp.0
            + self.cs.0
            + self.od.0
            + (self.hit_objects.len() as f64 / self.drain_time().as_secs() as f64 * 8.0).min(16.0))
            / 38.0
            * 5.0)
            .round()
    }

    /// Drain time without breaks
    pub fn drain_time(&self) -> Duration {
        if self.hit_objects.is_empty() {
            Duration::ZERO
        } else {
            Duration::from_millis(
                (self.hit_objects.last().unwrap().time() - self.hit_objects.first().unwrap().time())
                    as u64,
            )
        }
    }
}

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
                hp: HpDrainRate(to_f64(
                    difficulty
                        .hp_drain_rate
                        .ok_or(anyhow!("beatmap does no contain hp drain rate"))?
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

    #[test]
    fn beatmap_state_accuracy() {
        let state = BeatmapState {
            hits300: 438,
            hits100: 9,
            hits50: 1,
            misses: 0,
            score: 7_062_746,
            combo: 649,
            ..Default::default()
        };

        let expected_acc = 98.47;
        assert!((state.accuracy() - expected_acc).abs() < 0.01);
    }
}
