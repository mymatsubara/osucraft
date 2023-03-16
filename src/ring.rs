use anyhow::{bail, Result};
use std::f64::consts::TAU;

use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    system::{Commands, Query},
};
use valence::{
    equipment::{Equipment, EquipmentSlot},
    prelude::{DVec3, EntityKind, McEntity, TrackedData},
    protocol::{entity_meta::EulerAngle, ItemKind, ItemStack},
    Despawned,
};

#[derive(Component)]
pub struct Ring {
    armor_stands: Vec<Entity>,
    speed: f64,
    ticks: usize,
}

#[derive(Component)]
pub struct HitcircleRingPart;

impl Ring {
    // `speed` should be given in blocks per tick
    pub fn with_speed(
        center: impl Into<DVec3>,
        outer_radius: f64,
        inner_radius: f64,
        item: ItemKind,
        ticks: usize,
        instance: Entity,
        commands: &mut Commands,
    ) -> Result<Self> {
        let speed = (outer_radius - inner_radius).abs() / (ticks - 2).max(1) as f64;
        Self::new(center, outer_radius, speed, item, ticks, instance, commands)
    }

    pub fn without_speed(
        center: impl Into<DVec3>,
        radius: f64,
        item: ItemKind,
        ticks: usize,
        instance: Entity,
        commands: &mut Commands,
    ) -> Result<Self> {
        Self::new(center, radius, 0.0, item, ticks, instance, commands)
    }

    fn new(
        center: impl Into<DVec3>,
        radius: f64,
        speed: f64,
        item: ItemKind,
        ticks: usize,
        instance: Entity,
        commands: &mut Commands,
    ) -> Result<Self> {
        if radius <= 0.0 {
            bail!("Ring must have a radius greater than 0.0");
        }

        let center = center.into();

        // Calculate block positions/yaw/
        let number_of_blocks = (1.7 * TAU * radius) as u32;
        let d_angle = TAU / number_of_blocks as f64;
        let armor_stands = (0..number_of_blocks)
            .map(|n| {
                let angle = d_angle * n as f64;
                let roll = -(angle * 360.0 / TAU) as f32;
                let dir = DVec3::new(angle.cos(), angle.sin(), 0.0);
                let pos = center + radius * dir;

                let rotation = EulerAngle {
                    pitch: 0.0,
                    yaw: 0.0,
                    roll,
                };
                create_rotated_item(item, rotation, pos, instance)
            })
            .map(|bundle| commands.spawn(bundle).id())
            .collect();

        let ring = Self {
            armor_stands,
            ticks,
            speed,
        };

        Ok(ring)
    }

    pub fn update_position(
        &mut self,
        ring_entities: &mut Query<&mut McEntity, With<HitcircleRingPart>>,
    ) {
        if self.speed == 0.0 {
            return;
        }

        let len = self.armor_stands.len() as f64;

        self.armor_stands
            .iter()
            .enumerate()
            .for_each(|(n, entity)| {
                if let Ok(mut entity) = ring_entities.get_mut(*entity) {
                    let angle = TAU / len * n as f64;
                    let dir = DVec3::new(angle.cos(), angle.sin(), 0.0);
                    let mov = -self.speed * dir;
                    let new_pos = entity.position() + mov;

                    entity.set_position(new_pos);
                }
            });
    }

    pub fn despawn(&self, commands: &mut Commands) {
        for armor_stand in &self.armor_stands {
            if let Some(mut armor_stand) = commands.get_entity(*armor_stand) {
                armor_stand.insert(Despawned);
            }
        }
    }
}

/// Creates an invisible `ArmorStand` entity equiped with the `item` on the head
fn create_rotated_item(
    item: ItemKind,
    rotation: EulerAngle,
    position: impl Into<DVec3>,
    instance: Entity,
) -> (McEntity, Equipment, HitcircleRingPart) {
    // Equipment
    let mut equipment = Equipment::new();
    let item = ItemStack::new(item, 1, None);
    equipment.set(item, EquipmentSlot::Helmet);

    // Armor stand
    let mut armor_stand = McEntity::new(EntityKind::ArmorStand, instance);
    if let TrackedData::ArmorStand(armor_stand) = armor_stand.data_mut() {
        armor_stand.set_invisible(true);
        armor_stand.set_no_gravity(true);
        armor_stand.set_tracker_head_rotation(rotation);
    }

    let position = rotated_item_to_armor_stand_position(position, rotation);
    armor_stand.set_position(position);

    (armor_stand, equipment, HitcircleRingPart {})
}

const ARMOR_STAND_OFFSET: DVec3 = DVec3::new(0.5, -2.2, 0.5);

/// Returns the armor stand position such that the helmet item position is centered in `pos`
/// NOTE: if `rotation.roll` and `rotation.pitch` are simultaneously not zero, you may expect wrong results
pub fn rotated_item_to_armor_stand_position(
    pos: impl Into<DVec3>,
    rotation: impl Into<EulerAngle>,
) -> DVec3 {
    let EulerAngle { roll, pitch, .. } = rotation.into();
    let (roll, pitch) = (to_radians(roll as f64), to_radians(pitch as f64));

    let roll_offset = DVec3::new(-roll.sin(), 1.0 - roll.cos(), 0.0);
    let pitch_offset = DVec3::new(0.0, 1.0 - pitch.cos(), -pitch.sin());

    let rotation_offset = (roll_offset + pitch_offset) * 0.25;
    pos.into() + ARMOR_STAND_OFFSET + rotation_offset
}

fn to_radians(degrees: f64) -> f64 {
    degrees * TAU / 360.0
}

pub fn update_rings(
    mut commands: Commands,
    mut rings: Query<(&mut Ring, Entity)>,
    mut ring_entities: Query<&mut McEntity, With<HitcircleRingPart>>,
) {
    for (mut ring, entity) in &mut rings {
        if ring.ticks == 0 {
            ring.despawn(&mut commands);
            commands.entity(entity).insert(Despawned);
        } else {
            ring.ticks -= 1;
            ring.update_position(&mut ring_entities);
        }
    }
}
