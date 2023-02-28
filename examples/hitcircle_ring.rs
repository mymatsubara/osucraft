use osucraft::hitcircle::{
    rotated_item_to_armor_stand_position, update_hitcircle, update_rings, Hitcircle, Ring,
};
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::equipment::{Equipment, EquipmentSlot};
use valence::prelude::*;
use valence::protocol::entity_meta::EulerAngle;

const SPAWN_POS: DVec3 = DVec3::new(0.0, 64.0, 0.0);

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
        .add_system(test)
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -10..10 {
        for x in -10..10 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    let pos = [
        SPAWN_POS.x as i32,
        SPAWN_POS.y as i32 - 1,
        SPAWN_POS.z as i32,
    ];
    instance.set_block(pos, Block::new(BlockState::GLASS));

    let instance = world.spawn(instance).id();

    // Armor stand
    for i in 0..10 {
        let i = i as f32;
        let rotation = EulerAngle {
            pitch: i * 15.0,
            yaw: 0.0,
            roll: i * 15.0,
        };
        let mut armor_stand = McEntity::new(EntityKind::ArmorStand, instance);
        if let TrackedData::ArmorStand(armor_stand_data) = armor_stand.data_mut() {
            armor_stand_data.set_no_gravity(false);
            armor_stand_data.set_tracker_head_rotation(rotation)
        }

        armor_stand.set_position(rotated_item_to_armor_stand_position(SPAWN_POS, rotation));
        let mut equipment = Equipment::new();
        let item = ItemStack::new(ItemKind::GreenWool, 1, None);
        equipment.set(item, EquipmentSlot::Helmet);

        world.spawn((armor_stand, equipment, Test));
    }
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.single();

    for mut client in &mut clients {
        client.set_position(SPAWN_POS);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
    }
}

fn spawn_hitcircle_rings(
    mut commands: Commands,
    hitcircles: Query<Entity, With<Hitcircle>>,
    mut instances: Query<(Entity, &mut Instance)>,
) {
    if hitcircles.get_single().is_err() {
        let instance = instances.single_mut();
        let center = SPAWN_POS + DVec3::new(0.0, 0.0, 10.0);
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

fn test(server: Res<Server>, mut armor_stands: Query<&mut McEntity, With<Test>>) {
    // let tick = server.current_tick();
    // let cycle = 4;
    // let radius = 5.0;
    // let angle = (tick % cycle) as f64 / cycle as f64 * TAU;

    // for mut armor_stand in &mut armor_stands {
    //     armor_stand.set_position(SPAWN_POS + DVec3::new(angle.sin(), 0.0, angle.cos()) * radius);
    // }
}
