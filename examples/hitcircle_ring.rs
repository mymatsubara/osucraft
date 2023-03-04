use osucraft::beatmap::{ApproachRate, BeatmapData, CircleSize, OverallDifficulty};
use osucraft::color::Color;
use osucraft::hitcircle::{update_hitcircle, update_rings, Hitcircle};
use osucraft::osu::Osu;
use rand::Rng;
use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, SwingArm};
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
        let beatmap = BeatmapData {
            ar: ApproachRate(9.0),
            od: OverallDifficulty(8.0),
            cs: CircleSize(4.5),
            hit_objects: vec![],
        };

        let spawn_pos = osu.player_spawn_pos();
        let instance = instances.single_mut();
        let center = DVec3::new(spawn_pos.x, spawn_pos.y, 0.0);

        let pink = Color {
            r: 233,
            g: 102,
            b: 161,
        };
        let combo_number = rand::thread_rng().gen_range(0..=9);

        let ring = Hitcircle::from_beatmap(
            center,
            &beatmap,
            pink,
            scale,
            combo_number,
            tps,
            instance,
            &mut commands,
        )
        .unwrap();
        commands.spawn(ring);
    }
}

fn hitcircle_raycast(
    hitcircles: Query<&Hitcircle>,
    clients: Query<(Entity, &Client)>,
    mut swing_arm_events: EventReader<SwingArm>,
) {
    let swing_arm_events: Vec<_> = swing_arm_events.iter().collect();
    for hitcircle in &hitcircles {
        for (client_entity, client) in &clients {
            if swing_arm_events.iter().any(|e| e.client == client_entity) {
                let score = hitcircle.hit_score(client);
                dbg!(score);
            }
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
