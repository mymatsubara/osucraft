use anyhow::Result;
use tracing::warn;
use valence::{math::from_yaw_and_pitch, prelude::*, Despawned};

use std::cmp::max;

use crate::{
    beatmap::{BeatmapData, CircleSize},
    color::Color,
    digit::{DigitWriter, TextPosition},
    hit_score::{HitScore, HitScoreNumber},
    minecraft::{to_ticks, PLAYER_EYE_OFFSET},
    osu::Hitwindow,
    ring::Ring,
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
    filling_block: BlockState,
    combo_number: u32,
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
}

pub fn update_hitcircle(
    mut commands: Commands,
    mut hitcircles: Query<(Entity, &mut Hitcircle), Without<Despawned>>,
    rings: Query<&Ring>,
    mut instances: Query<(Entity, &mut Instance)>,
) {
    for (entity, mut hitcircle) in &mut hitcircles {
        if hitcircle.ticks == 0 {
            commands.entity(entity).insert(Despawned);
            if let Err(error) =
                hitcircle.despawn(&mut commands, &rings, &mut instances, HitScore::Miss)
            {
                warn!("Error while despawning hitcircle: {}", error);
            };
        } else {
            hitcircle.ticks -= 1;
        }
    }
}

impl Hitcircle {
    pub fn new(
        center: impl Into<DVec3>,
        radius: HitcircleRadius,
        blocks: HitcircleBlocks,
        hitwindow: HitwindowTicks,
        preempt_ticks: usize,
        combo_number: u32,
        mut instance: (Entity, Mut<Instance>),
        commands: &mut Commands,
    ) -> Result<Self> {
        let center = center.into().floor();
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
            filling_block: blocks.filling.state(),
            combo_number,
        };

        hitcircle.draw_circle(&mut instance.1);

        Ok(hitcircle)
    }

    pub fn from_beatmap(
        center: impl Into<DVec3>,
        beatmap: &BeatmapData,
        color: Color,
        scale: f64,
        combo_number: u32,
        tps: usize,
        instance: (Entity, Mut<Instance>),
        commands: &mut Commands,
    ) -> Result<Self> {
        let radius = HitcircleRadius::from(beatmap.cs, scale);
        let hitwindow = HitwindowTicks::from(&beatmap.od.into(), tps);
        let preempt_ticks = beatmap.ar.to_mc_ticks(tps);
        let blocks: HitcircleBlocks = color.into();

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

    pub fn hit_score(&self, client: &Client) -> Option<HitScore> {
        self.raycast_client(client)
            .is_some()
            .then_some(self.hitwindow.hit_score(self.ticks as u32))
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
        rings: &Query<&Ring>,
        instances: &mut Query<(Entity, &mut Instance)>,
        hit: HitScore,
    ) -> Result<()> {
        let mut instance = instances.get_mut(self.instance)?;
        self.fill(&mut instance.1, &Block::new(BlockState::AIR));

        if let Ok(ring) = rings.get(self.circle_ring) {
            ring.despawn(commands);
        }
        if let Ok(approach_circle) = rings.get(self.approach_circle) {
            approach_circle.despawn(commands);
        }

        commands.spawn(HitScoreNumber::new(
            hit,
            BlockPos::at(self.center() + DVec3::new(0.0, 0.0, -1.0)),
            5,
            instance,
        ));

        Ok(())
    }

    pub fn draw_circle(&self, instance: &mut Mut<Instance>) {
        self.fill(instance, &Block::new(self.filling_block));
        self.draw_combo_number(
            instance,
            self.combo_number,
            Block::new(BlockState::WHITE_CONCRETE),
        );
    }

    pub fn instance(&self) -> Entity {
        self.instance
    }

    pub fn center(&self) -> DVec3 {
        self.center
    }

    fn fill(&self, instance: &mut Mut<Instance>, block: &Block) {
        self.circle_block_positions().for_each(|pos| {
            instance.set_block(pos, block.clone());
        });
    }

    fn draw_combo_number(&self, instance: &mut Mut<Instance>, combo_number: u32, block: Block) {
        let origin = BlockPos::at(self.center);

        DigitWriter {
            scale: max((self.radius / 5.5) as usize, 1),
            position: TextPosition::Center,
        }
        .draw(combo_number as usize, origin, block, instance);
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
    pub fn from(cs: CircleSize, scale: f64) -> Self {
        let circle = ((54.4 - 4.48 * cs.0) * scale).ceil();
        Self {
            circle,
            approach_circle: circle * 3.0,
        }
    }
}

impl From<Color> for HitcircleBlocks {
    fn from(color: Color) -> Self {
        let block_color = color.to_block_color();
        let (block, item) = (block_color.block(), block_color.item());

        Self {
            approach_circle: item,
            circle_ring: ItemKind::WhiteConcrete,
            filling: block,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hitcircle_radius() {
        let scale = 1.0;

        let cs = CircleSize(4.2);
        let radius = HitcircleRadius::from(cs, scale);
        assert_eq!(radius.circle, 36.0);
    }
}
