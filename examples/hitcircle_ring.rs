use osucraft::hitcircle::{update_hitcircle, update_rings, Hitcircle, HitcircleBlocks};
use osucraft::osu::{ApproachRate, Beatmap, CircleSize, Osu, OverallDifficulty};
use rand::Rng;
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
    server: Res<Server>,
) {
    if hitcircles.get_single().is_err() {
        let tps = server.shared().tps() as usize;
        let scale = osu.scale();
        let beatmap = Beatmap {
            ar: ApproachRate(9.0),
            od: OverallDifficulty(8.0),
            cs: CircleSize(4.5),
        };

        let spawn_pos = osu.player_spawn_pos();
        let instance = instances.single_mut();
        let center = DVec3::new(spawn_pos.x, spawn_pos.y, 0.0);

        let blocks = HitcircleBlocks {
            approach_circle: ItemKind::PinkConcrete,
            circle_ring: ItemKind::WhiteConcrete,
            filling: Block::new(BlockState::PINK_CONCRETE),
            combo_number: Block::new(BlockState::WHITE_CONCRETE),
        };
        let combo_number = rand::thread_rng().gen_range(0..=9);

        let ring = Hitcircle::from_beatmap(
            center,
            &beatmap,
            scale,
            blocks,
            combo_number,
            tps,
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
