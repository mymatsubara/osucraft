use anyhow::Result;
use osu_file_parser::{colours::Colour, Colours, OsuFile};

use crate::{
    color::{Color, DEFAULT_COMBO_COLORS},
    hitcircle::HitcircleBlocks,
};

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
