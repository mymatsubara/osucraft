use osucraft::hitcircle::{update_hitcircle, update_rings, Hitcircle};
use osucraft::osu::Osu;
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;

#[derive(Component)]
struct Test;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(update_rings)
        .add_system(update_hitcircle)
        .add_system(spawn_hitcircle_rings)
        .add_system(hitcircle_raycast)
        .add_system(test)
        .insert_resource(Osu::new(0.5))
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    world.resource::<Osu>().init(&mut instance);

    world.spawn(instance);
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

fn spawn_hitcircle_rings(
    mut commands: Commands,
    hitcircles: Query<Entity, With<Hitcircle>>,
    mut instances: Query<(Entity, &mut Instance)>,
    osu: Res<Osu>,
) {
    if hitcircles.get_single().is_err() {
        let spawn_pos = osu.player_spawn_pos();
        let instance = instances.single_mut();
        let center = DVec3::new(spawn_pos.x, spawn_pos.y, 0.0);
        let outer_radius = 30.0;
        let inner_radius = 10.0;
        let circle_ticks = 25;
        let approach_ticks = 20;
        let item = ItemKind::PinkConcrete;
        let filling = Block::new(BlockState::PINK_CONCRETE);
        let combo_number = 1;

        let ring = Hitcircle::new(
            center,
            outer_radius,
            inner_radius,
            circle_ticks,
            approach_ticks,
            item,
            filling,
            combo_number,
            instance,
            &mut commands,
        )
        .unwrap();
        commands.spawn(ring);
    }
}

fn hitcircle_raycast(hitcircles: Query<&Hitcircle>, clients: Query<&Client>) {
    for hitcircle in &hitcircles {
        for client in &clients {
            let hit = hitcircle.raycast_client(client);
            dbg!(hit);
        }
    }
}

fn test(server: Res<Server>, mut armor_stands: Query<&mut McEntity, With<Test>>) {
    // let tick = server.current_tick();
    // let cycle = 4;
    // let radius = 5.0;
    // let angle = (tick % cycle) as f64 / cycle as f64 * TAU;

    // for mut armor_stand in &mut armor_stands {
    //     armor_stand.set_position(SPAWN_POS + DVec3::new(angle.sin(), 0.0, angle.cos()) * radius);
    // }
}
