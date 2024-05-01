use std::path::PathBuf;

use colored::Colorize;
use osucraft::audio::AudioPlayer;

use osucraft::configs::Configs;
use osucraft::osu::{Osu, OsuInstance};
use osucraft::plugin::OsuPlugin;
use rodio::OutputStream;
use tracing::Level;
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;

#[derive(Component)]
struct Test;

pub fn main() {
    let log_level = if cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::WARN
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let audio_player = AudioPlayer::new(&stream_handle).unwrap();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_plugin(OsuPlugin)
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(reposition_clients)
        .insert_resource(Osu::new(0.3, audio_player))
        .run();
}

fn setup(world: &mut World) {
    // Init configs
    let configs = Configs::open();
    let configs_path = Configs::path();
    let header = format!(
        "================= CONFIGS ({}) =================",
        configs_path.display()
    );
    println!("{}", header.cyan());
    println!("{configs}\n");
    let info = format!(
        "INFO: To update any config modify the file '{}' and restart the server.",
        configs_path.display()
    );
    println!("{}", info.yellow());
    println!(
        "{}",
        "INFO: The server is running on minecraft version 1.19.3\n".yellow()
    );

    let server = world.resource::<Server>();
    let mut instance = server.new_instance(DimensionId::default());

    // Init osu
    world.resource::<Osu>().init(&mut instance);
    Osu::init_inventory_selections(world, PathBuf::from(configs.songs_directory()));

    world.spawn((instance, OsuInstance));

    println!("Server is running on: {}", "127.0.0.1:25565".green())
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    osu: Res<Osu>,
) {
    let instance = instances.single();
    let spawn_pos = osu.player_spawn_pos();

    for mut client in &mut clients {
        client.set_position(spawn_pos);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
    }
}

fn reposition_clients(osu: Res<Osu>, mut clients: Query<&mut Client>) {
    for mut client in &mut clients {
        if client.position().y < 0.0 {
            client.set_position(osu.player_spawn_pos());
        }
    }
}
