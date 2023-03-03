use std::time::Duration;

use valence::prelude::DVec3;

pub const PLAYER_EYE_OFFSET: DVec3 = DVec3::new(0.0, 1.62, 0.0);

pub fn to_ticks(tps: usize, duration: Duration) -> usize {
    let tps_in_ms = 1000.0 / tps as f64;
    (duration.as_millis() as f64 / tps_in_ms).ceil() as usize
}
