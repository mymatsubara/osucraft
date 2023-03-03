use anyhow::{bail, Result};
use valence::{
    equipment::{Equipment, EquipmentSlot},
    math::from_yaw_and_pitch,
    prelude::*,
    protocol::entity_meta::EulerAngle,
    Despawned,
};

use std::{cmp::max, f64::consts::TAU, ops::RangeInclusive, time::Duration};

use crate::{
    digit::{DigitWriter, TextPosition},
    minecraft::{to_ticks, PLAYER_EYE_OFFSET},
    osu::{ApproachRate, Beatmap, CircleSize, HitScore, Hitwindow, OverallDifficulty},
};

#[derive(Component)]
pub struct Hitcircle {
    approach_circle: Entity,
    circle_ring: Entity,
    instance: Entity,
    center: DVec3,
    radius: f64,
    ticks: usize,
    hitwindow: HitwindowTicks,
}

pub struct HitwindowTicks {
    window_300: u32,
    window_100: u32,
    window_50: u32,
}

pub struct HitcircleRadius {
    pub circle: f64,
    pub approach_circle: f64,
}

pub struct HitcircleBlocks {
    pub approach_circle: ItemKind,
    pub circle_ring: ItemKind,
    pub filling: Block,
    pub combo_number: Block,
}

#[derive(Component)]
pub struct Ring {
    armor_stands: Vec<Entity>,
    speed: f64,
    ticks: usize,
}

#[derive(Component)]
pub struct HitcircleRingPart;

#[derive(Component)]
pub struct RotatedBlock;

impl Hitcircle {
    pub fn new(
        center: impl Into<DVec3>,
        radius: HitcircleRadius,
        blocks: HitcircleBlocks,
        hitwindow: HitwindowTicks,
        preempt_ticks: usize,
        combo_number: usize,
        mut instance: (Entity, Mut<Instance>),
        commands: &mut Commands,
    ) -> Result<Self> {
        let center = center.into();
        let approach_circle = Ring::with_speed(
            center,
            radius.approach_circle,
            radius.circle,
            blocks.approach_circle,
            preempt_ticks,
            instance.0,
            commands,
        )?;
        let approach_circle = commands.spawn(approach_circle).id();

        let mut circle_ring_center = center;
        circle_ring_center.z = center.z.floor() - 0.25;
        let circle_ticks = preempt_ticks + hitwindow.window_50 as usize;

        let circle_ring = Ring::without_speed(
            circle_ring_center,
            radius.circle,
            blocks.circle_ring,
            circle_ticks,
            instance.0,
            commands,
        )?;
        let circle_ring = commands.spawn(circle_ring).id();

        let hitcircle = Self {
            instance: instance.0,
            approach_circle,
            circle_ring,
            center,
            radius: radius.circle,
            ticks: circle_ticks,
            hitwindow,
        };

        hitcircle.fill(&mut instance.1, &blocks.filling);
        hitcircle.draw_combo_number(&mut instance.1, combo_number, blocks.combo_number);

        Ok(hitcircle)
    }

    pub fn from_beatmap(
        center: impl Into<DVec3>,
        beatmap: &Beatmap,
        scale: f64,
        blocks: HitcircleBlocks,
        combo_number: usize,
        tps: usize,
        instance: (Entity, Mut<Instance>),
        commands: &mut Commands,
    ) -> Result<Self> {
        let radius = HitcircleRadius::from(beatmap.cs, scale);
        let hitwindow = HitwindowTicks::from(&beatmap.od.into(), tps);
        let preempt_ticks = beatmap.ar.to_mc_preempt_ticks(tps);
        Self::new(
            center,
            radius,
            blocks,
            hitwindow,
            preempt_ticks,
            combo_number,
            instance,
            commands,
        )
    }

    pub fn raycast_client(&self, client: &Client) -> Option<DVec3> {
        let origin = client.position() + PLAYER_EYE_OFFSET;
        let direction = from_yaw_and_pitch(client.yaw(), client.pitch());
        let direction = DVec3::new(direction.x as f64, direction.y as f64, direction.z as f64);

        self.raycast(origin, direction)
    }

    pub fn raycast(&self, origin: DVec3, direction: DVec3) -> Option<DVec3> {
        if direction.z == 0.0 {
            return None;
        }

        let direction_scale = (self.center.z - origin.z) / direction.z;
        if direction_scale < 0.0 {
            // Direction not pointing to hitcircle plane
            return None;
        }

        let intersection = origin + direction * direction_scale;
        let dist = self.center.distance(intersection);

        (dist <= self.radius).then_some(intersection)
    }

    pub fn despawn(
        &self,
        commands: &mut Commands,
        instances: &mut Query<&mut Instance>,
        rings: &mut Query<&mut Ring>,
    ) -> Result<()> {
        self.fill(
            &mut instances.get_mut(self.instance)?,
            &Block::new(BlockState::AIR),
        );

        if let Ok(ring) = rings.get_mut(self.circle_ring) {
            ring.despawn(commands);
        }
        if let Ok(approach_circle) = rings.get_mut(self.approach_circle) {
            approach_circle.despawn(commands);
        }

        Ok(())
    }

    fn fill(&self, instance: &mut Mut<Instance>, block: &Block) {
        self.circle_block_positions().for_each(|pos| {
            instance.set_block(pos, block.clone());
        });
    }

    fn draw_combo_number(&self, instance: &mut Mut<Instance>, combo_number: usize, block: Block) {
        let origin = BlockPos::at(self.center);

        DigitWriter {
            scale: max((self.radius / 5.5) as usize, 1),
            position: TextPosition::Center,
        }
        .iter_block_positions(combo_number, origin)
        .flatten()
        .for_each(|pos| {
            instance.set_block(pos, block.clone());
        });
    }

    fn circle_block_positions(&self) -> impl Iterator<Item = BlockPos> {
        let (center_x, center_y, center_z) = (
            self.center.x as i32,
            self.center.y as i32,
            self.center.z as i32,
        );
        let radius = self.radius as i32;

        (center_x - radius..=center_x + radius).flat_map(move |x| {
            (center_y - radius..=center_y + radius).filter_map(move |y| {
                let rel_x = center_x - x;
                let rel_y = center_y - y;

                (rel_x.pow(2) + rel_y.pow(2) <= radius.pow(2)).then_some(BlockPos {
                    x,
                    y: y - 1,
                    z: center_z,
                })
            })
        })
    }
}

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
        let number_of_blocks = (1.8 * TAU * radius) as u32;
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

    fn despawn(&self, commands: &mut Commands) {
        for armor_stand in self.armor_stands.iter() {
            commands.entity(*armor_stand).insert(Despawned);
        }
    }

    /// Returns `false` the hitcircle ring should stop moving
    pub fn tick(
        &mut self,
        commands: &mut Commands,
        ring_entities: &mut Query<&mut McEntity, With<HitcircleRingPart>>,
    ) -> bool {
        if self.ticks == 0 {
            self.despawn(commands);
            return false;
        }

        self.ticks -= 1;
        self.update_position(ring_entities);

        true
    }
}

impl HitwindowTicks {
    fn from(hitwindow: &Hitwindow, tps: usize) -> Self {
        Self {
            window_300: to_ticks(tps, hitwindow.window_300) as u32,
            window_100: to_ticks(tps, hitwindow.window_100) as u32,
            window_50: to_ticks(tps, hitwindow.window_50) as u32,
        }
    }

    fn hit_score(&self, ticks_left: u32) -> HitScore {
        let hit_time = self.window_50;
        for (window, score) in [
            (self.window_300, HitScore::Hit300),
            (self.window_100, HitScore::Hit100),
            (self.window_50, HitScore::Hit50),
        ]
        .into_iter()
        {
            if (hit_time - window..=hit_time + window).contains(&ticks_left) {
                return score;
            }
        }

        HitScore::Miss
    }
}

/// https://osu.ppy.sh/wiki/en/Beatmap/Circle_size
impl HitcircleRadius {
    fn from(cs: CircleSize, scale: f64) -> Self {
        let circle = ((54.4 - 4.48 * cs.0) * scale).ceil();
        Self {
            circle,
            approach_circle: circle * 3.0,
        }
    }
}

pub fn update_hitcircle(
    mut commands: Commands,
    mut hitcircles: Query<(Entity, &mut Hitcircle)>,
    mut instances: Query<&mut Instance>,
    mut rings: Query<&mut Ring>,
) {
    for (entity, mut hitcircle) in &mut hitcircles {
        if hitcircle.ticks == 0 {
            hitcircle
                .despawn(&mut commands, &mut instances, &mut rings)
                .unwrap();
            commands.entity(entity).insert(Despawned);
        } else {
            hitcircle.ticks -= 1;
        }
    }
}

pub fn update_rings(
    mut commands: Commands,
    mut rings: Query<(&mut Ring, Entity)>,
    mut ring_entities: Query<&mut McEntity, With<HitcircleRingPart>>,
) {
    for (mut ring, entity) in &mut rings {
        if !ring.tick(&mut commands, &mut ring_entities) {
            commands.entity(entity).insert(Despawned);
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hitcircle_radius() {
        let scale = 1.0;

        let cs = CircleSize(4.2);
        let radius = HitcircleRadius::from(cs, scale);
        assert_eq!(radius.circle, 18.0);
    }
}
