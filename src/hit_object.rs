use std::time::Duration;

use anyhow::Result;
use osu_file_parser::{colours::Colour, OsuFile};

use crate::{
    beatmap::{ApproachRate, CircleSize},
    color::{Color, DEFAULT_COMBO_COLORS},
    hitcircle::HitcircleRadius,
};

const OVERLAP_THRESHOLD_MS: u32 = 1200;

#[derive(Default)]
/// https://osu.ppy.sh/wiki/en/Client/File_formats/Osu_%28file_format%29#hit-objects
pub struct HitObject {
    /// In osu!pixels
    x: u32,
    // In osu!pixels
    y: u32,
    /// In milliseconds since the start of the beatmap
    time: u32,
    combo_number: u32,
    color: Color,
    params: HitObjectParams,
}

pub enum HitObjectParams {
    Hitcircle,
    Slider,
    Spinner,
    OsuManiaHold,
}

impl Default for HitObjectParams {
    fn default() -> Self {
        Self::Hitcircle
    }
}

impl HitObject {
    pub fn from(osu_file: &OsuFile) -> Result<Vec<Self>> {
        let mut combo_number = 1;
        let hitobjects = osu_file.hitobjects.clone().unwrap_or_default().0;

        let mut result = Vec::with_capacity(hitobjects.len());
        let colors = osu_file
            .colours
            .clone()
            .map(|colors| {
                let mut colors = colors
                    .0
                    .iter()
                    .filter_map(|color| {
                        if let Colour::Combo(combo, rgb) = color {
                            Some((
                                combo,
                                Color {
                                    r: rgb.red,
                                    g: rgb.green,
                                    b: rgb.blue,
                                },
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                colors.sort_by_key(|(combo, _)| **combo);
                colors.into_iter().map(|(_, color)| color).collect()
            })
            .unwrap_or_else(|| DEFAULT_COMBO_COLORS.to_vec());

        let mut cur_color = colors.len() - 1;

        for hitobject in hitobjects {
            // Update combo
            if hitobject.new_combo {
                combo_number = 1;
                cur_color =
                    (cur_color + 1 + hitobject.combo_skip_count.get() as usize) % colors.len();
            } else {
                combo_number += 1;
            }

            result.push(Self {
                x: hitobject.position.x.to_string().parse()?,
                y: hitobject.position.y.to_string().parse()?,
                color: colors[cur_color],
                time: hitobject.time.to_string().parse()?,
                combo_number,
                params: hitobject.obj_params.clone().into(),
            });
        }

        Ok(result)
    }

    // Calculate z value such that there is no overlap with other hitcircles
    //
    // `remaining`: is the list of the remaining hitobjects in the song ordered in chronological order.
    pub fn z(&self, remaining: &[HitObject], _cs: CircleSize) -> i32 {
        match remaining
            .iter()
            .take_while(|other| other.time < self.time + OVERLAP_THRESHOLD_MS)
            .enumerate()
            .find(|(_, other)| self.intersect(other, _cs))
        {
            Some((overlapping_idx, _)) => {
                remaining[overlapping_idx].z(&remaining[overlapping_idx + 1..], _cs) - 1
            }
            None => 0,
        }
    }

    pub fn intersect(&self, other: &HitObject, cs: CircleSize) -> bool {
        let radius = HitcircleRadius::from(cs, 1.0).circle;
        let dist = (self.x.abs_diff(other.x).pow(2) + self.y.abs_diff(other.y).pow(2)) as f64;
        let dist = dist.sqrt();

        dist < radius * 2.0
    }

    pub fn x(&self) -> u32 {
        self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }

    pub fn time(&self) -> u32 {
        self.time
    }

    pub fn combo_number(&self) -> u32 {
        self.combo_number
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn params(&self) -> &HitObjectParams {
        &self.params
    }
}

impl From<osu_file_parser::hitobjects::HitObjectParams> for HitObjectParams {
    fn from(hitobject: osu_file_parser::hitobjects::HitObjectParams) -> Self {
        match hitobject {
            osu_file_parser::hitobjects::HitObjectParams::HitCircle => HitObjectParams::Hitcircle,
            osu_file_parser::hitobjects::HitObjectParams::Slider(_) => HitObjectParams::Slider,
            osu_file_parser::hitobjects::HitObjectParams::Spinner { end_time } => {
                HitObjectParams::Spinner
            }
            osu_file_parser::hitobjects::HitObjectParams::OsuManiaHold { end_time } => {
                HitObjectParams::OsuManiaHold
            }
            _ => panic!("unexpected hitobject from osu file"),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{beatmap::CircleSize, hitcircle::HitcircleRadius};

    use super::HitObject;

    #[test]
    fn hitobject_z() {
        let cs = CircleSize(5.0);
        let radius = HitcircleRadius::from(cs, 1.0).circle as u32;

        let hitobjects = vec![
            HitObject {
                x: 0,
                y: 0,
                ..Default::default()
            },
            HitObject {
                x: radius,
                y: 0,
                ..Default::default()
            },
            HitObject {
                x: 2 * radius,
                y: 0,
                ..Default::default()
            },
            HitObject {
                x: 4 * radius,
                y: 0,
                ..Default::default()
            },
        ];

        assert_eq!(hitobjects[0].z(&hitobjects[1..], cs), -2);
        assert_eq!(hitobjects[1].z(&hitobjects[2..], cs), -1);
        assert_eq!(hitobjects[2].z(&hitobjects[3..], cs), 0);
        assert_eq!(hitobjects[3].z(&hitobjects[4..], cs), 0);
    }
}
