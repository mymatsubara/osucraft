use bevy_ecs::schedule::{IntoSystemDescriptor, SystemSet};
use valence::bevy_app::Plugin;

use crate::{
    beatmap_selection::{handle_beatmap_selection_clicks, update_beatmap_selection_inventory},
    commands::{execute_commands, register_mc_commands},
    hit_score::update_score_hit_numbers,
    hitcircle::update_hitcircle,
    inventory::{open_queued_inventories, InventoriesToOpen},
    osu::{send_welcome_message, update_osu},
    ring::update_rings,
    song_selection::{handle_song_selection_clicks, update_song_selection_inventory},
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
                .with_system(open_queued_inventories)
                .with_system(update_song_selection_inventory)
                .with_system(handle_song_selection_clicks.after(open_queued_inventories))
                .with_system(update_beatmap_selection_inventory)
                .with_system(handle_beatmap_selection_clicks)
                .with_system(register_mc_commands)
                .with_system(execute_commands)
                .with_system(send_welcome_message),
        )
        .init_resource::<InventoriesToOpen>();
    }
}
