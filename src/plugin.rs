use bevy_ecs::schedule::{IntoSystemDescriptor, SystemSet};
use valence::bevy_app::Plugin;

use crate::{hitcircle::update_hitcircle, osu::update_osu, ring::update_rings};

pub struct OsuPlugin;

impl Plugin for OsuPlugin {
    fn build(&self, app: &mut valence::prelude::App) {
        app.add_system_set(
            SystemSet::new()
                .label("osu")
                .with_system(update_osu)
                .with_system(update_rings)
                .with_system(update_hitcircle),
        );
    }
}
