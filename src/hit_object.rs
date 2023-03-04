use crate::{color::Color, hitcircle::HitcircleBlocks};

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
}
