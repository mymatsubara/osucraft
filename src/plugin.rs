use bevy_ecs::schedule::{IntoSystemDescriptor, SystemSet};
use valence::bevy_app::{CoreStage, Plugin};

use crate::{
    hit_score::update_score_hit_numbers,
    hitcircle::update_hitcircle,
    osu::update_osu,
    ring::update_rings,
    song_selection::{
        handle_song_selection_clicks, reopen_inventory, update_song_selection_inventory,
    },
};

pub struct OsuPlugin;

impl Plugin for OsuPlugin {
    fn build(&self, app: &mut valence::prelude::App) {
        app.add_system_set(
            SystemSet::new()
                .label("osu")
                .with_system(update_osu)
                .with_system(update_rings)
                .with_system(update_hitcircle)
                .with_system(update_score_hit_numbers)
                .with_system(update_song_selection_inventory)
                .with_system(handle_song_selection_clicks)
                .with_system(reopen_inventory.before(handle_song_selection_clicks)),
        );
    }
}
