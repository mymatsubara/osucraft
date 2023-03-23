use anyhow::{anyhow, Context, Result};
use osu_file_parser::{Decimal, OsuFile};
use std::{collections::VecDeque, num::ParseFloatError, path::PathBuf, time::Duration};
use valence::{
    prelude::Color,
    protocol::{Text, TextFormat},
};

use bevy_ecs::prelude::Entity;

use crate::{hit_object::HitObject, hit_score::HitScore, minecraft::to_ticks};

#[derive(Clone)]
pub struct Beatmap {
    pub data: BeatmapData,
    pub state: BeatmapState,
}

#[derive(Clone)]
pub struct BeatmapData {
    pub od: OverallDifficulty,
    pub ar: ApproachRate,
    pub cs: CircleSize,
    pub hp: HpDrainRate,
    pub hit_objects: Vec<HitObject>,
    pub audio_path: PathBuf,
    pub artist: String,
    pub title: String,
    pub difficulty_name: String,
}

#[derive(Clone, Debug)]
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
    pub health: f64,
}

pub enum Grade {
    SS,
    S,
    A,
    B,
    C,
    D,
}

#[derive(Copy, Clone)]
pub struct OverallDifficulty(pub f64);

#[derive(Copy, Clone)]
pub struct ApproachRate(pub f64);

#[derive(Copy, Clone)]
pub struct HpDrainRate(pub f64);

#[derive(Copy, Clone)]
pub struct CircleSize(pub f64);

impl Default for BeatmapState {
    fn default() -> Self {
        Self {
            health: 1.0,
            play_time: Default::default(),
            hits300: 0,
            hits100: 0,
            hits50: 0,
            misses: 0,
            active_hit_objects: Default::default(),
            next_hit_object_idx: Default::default(),
            score: 0,
            combo: 0,
            max_combo: 0,
        }
    }
}

impl BeatmapState {
    pub fn accuracy(&self) -> f32 {
        let divisor = self.hits300 + self.hits100 + self.hits50 + self.misses;
        if divisor == 0 {
            return 0.0;
        }

        (self.hits300 as f32 * 100.0
            + self.hits100 as f32 * 100.0 / 3.0
            + self.hits50 as f32 * 100.0 / 6.0)
            / divisor as f32
    }

    /// https://osu.ppy.sh/wiki/en/Gameplay/Grade
    pub fn grade(&self) -> Grade {
        let accuracy = self.accuracy();
        let hits = (self.hits300 + self.hits100 + self.hits50 + self.misses) as f64;

        let percentage300s = self.hits300 as f64 / hits;
        let percentage50s = self.hits50 as f64 / hits;

        if accuracy >= 99.999 {
            Grade::SS
        } else if self.misses == 0 && percentage300s > 0.9 && percentage50s <= 0.01 {
            Grade::S
        } else if (percentage300s > 0.8 && self.misses == 0) || percentage300s > 0.9 {
            Grade::A
        } else if (percentage300s > 0.7 && self.misses == 0) || percentage300s > 0.8 {
            Grade::B
        } else if percentage300s > 0.6 {
            Grade::C
        } else {
            Grade::D
        }
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

impl Beatmap {
    pub fn try_from(osu_file: OsuFile, beatmap_dir: PathBuf) -> Result<Self> {
        let difficulty = osu_file.difficulty.clone().unwrap_or_default();
        let metadata = osu_file.metadata.clone().unwrap_or_default();

        let to_f64 =
            |decimal: Decimal| -> Result<f64, ParseFloatError> { decimal.to_string().parse() };
        let audio_path = audio_path_from(&osu_file, beatmap_dir)
            .with_context(|| "beatmap audio file not found")?;

        let title = metadata
            .title
            .map(|title| title.into())
            .unwrap_or("Not named".to_string());
        let difficulty_name: String = metadata
            .version
            .map(|version| version.into())
            .unwrap_or("Not named".to_string());
        let artist: String = metadata
            .artist
            .map(|artist| artist.into())
            .unwrap_or("Not named".to_string());

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
                audio_path,
                artist,
                difficulty_name,
                title,
            },
            state: Default::default(),
        })
    }

    pub fn score_text(&self) -> Vec<Text> {
        let empty = "".color(Color::WHITE);
        let score_bar = "=========== SCORE ============".color(Color::YELLOW);

        let song = "Song: ".color(Color::DARK_AQUA) + self.data.title.clone().color(Color::WHITE);
        let artist =
            "Artist: ".color(Color::DARK_AQUA) + self.data.artist.clone().color(Color::WHITE);
        let difficulty = "Difficulty: ".color(Color::DARK_AQUA)
            + self.data.difficulty_name.clone().color(Color::WHITE);

        let hits = "300: ".color(Color::BLUE)
            + self.state.hits300.to_string().color(Color::WHITE)
            + "  100: ".color(Color::GREEN)
            + self.state.hits100.to_string().color(Color::WHITE)
            + "  50: ".color(Color::YELLOW)
            + self.state.hits50.to_string().color(Color::WHITE)
            + "  X: ".color(Color::RED)
            + self.state.misses.to_string().color(Color::WHITE);

        let stats = "Combo: ".color(Color::LIGHT_PURPLE)
            + format!("x{}", self.state.max_combo).color(Color::WHITE)
            + "   Accuracy: ".color(Color::DARK_GREEN)
            + format!("{:.2}%", self.state.accuracy()).color(Color::WHITE);

        let grade = match self.state.grade() {
            Grade::SS => "SS".color(Color::GOLD),
            Grade::S => "S".color(Color::GOLD),
            Grade::A => "A".color(Color::GREEN),
            Grade::B => "B".color(Color::BLUE),
            Grade::C => "C".color(Color::DARK_PURPLE),
            Grade::D => "D".color(Color::RED),
        };
        let score = "Score: ".color(Color::GOLD)
            + self.state.score.to_string().color(Color::WHITE)
            + "   Grade: ".color(Color::GOLD)
            + grade;

        vec![
            score_bar,
            empty.clone(),
            song,
            artist,
            difficulty,
            empty.clone(),
            score,
            hits,
            stats,
            empty,
        ]
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

impl HpDrainRate {
    pub fn drain(&self, hp: f64, hit: HitScore) -> f64 {
        let drain = match hit {
            HitScore::Hit300 => 10.2 - self.0,
            HitScore::Hit100 => 8.0 - self.0,
            HitScore::Hit50 => 4.0 - self.0,
            HitScore::Miss => -2.0 * self.0,
        } / 100.0;

        (hp + drain).clamp(0.0, 1.0)
    }
}

pub fn audio_path_from(osu_file: &OsuFile, beatmap_dir: PathBuf) -> Option<PathBuf> {
    let audio_file: PathBuf = osu_file
        .general
        .clone()
        .and_then(|g| g.audio_filename.map(|f| f.into()))?;

    let audio_path = beatmap_dir.join(audio_file);

    audio_path.exists().then_some(audio_path)
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
