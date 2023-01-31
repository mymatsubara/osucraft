use anyhow::{bail, Result};
use tracing::info;
use valence::prelude::*;

use std::f64::consts::TAU;

pub struct HitcircleRing {
    blocks: Vec<EntityId>,
    velocity: f32,
    ticks: usize,
}

impl HitcircleRing {
    pub fn new<C>(
        center: impl Into<Vec3<f64>>,
        radius: f64,
        velocity: f32,
        server: &mut Server<C>,
        world: WorldId,
    ) -> Result<Self>
    where
        C: Config<EntityState = ()>,
    {
        if radius <= 0.0 {
            bail!("Hitcircle ring must have a radius greater than 0.0");
        }

        if velocity <= 0.0 {
            bail!("Hitcircle ring must have speed greater than 0.0");
        }

        let center = center.into();

        // Calculate block positions/yaw/
        let number_of_blocks = (TAU * radius) as u32 + 1;
        let d_angle = TAU / number_of_blocks as f64;
        let blocks = (0..number_of_blocks)
            .map(|n| {
                let angle = d_angle * n as f64;
                let dir = Vec3::new(angle.cos(), angle.sin(), 0.0);
                let pos = center + radius * dir;

                let (id, entity) = server.entities.insert(EntityKind::FallingBlock, ());
                if let TrackedData::FallingBlock(block) = entity.data_mut() {
                    block.set_no_gravity(true);
                }

                entity.set_world(world);
                entity.set_position(pos);
                // entity.set_yaw(45.0);
                // entity.set_pitch();

                id
            })
            .collect();

        let ticks = (radius / velocity as f64) * 15.0;

        let mut ring = Self {
            blocks,
            ticks: ticks as usize,
            velocity,
        };

        ring.refresh_velocity(server);

        Ok(ring)
    }

    pub fn refresh_velocity<C>(&mut self, server: &mut Server<C>)
    where
        C: Config,
    {
        let len = self.blocks.len() as f32;
        let noise = (self.ticks % 2) as f32 * 0.00001 + 1.0;

        self.blocks.iter_mut().enumerate().for_each(|(n, block)| {
            let entity = &mut server.entities[*block];
            let angle = TAU as f32 / len * n as f32;
            let dir = Vec3::new(angle.cos(), angle.sin(), 0.0);

            entity.set_velocity(-self.velocity * noise * dir);
        })
    }

    pub fn tick<C>(&mut self, server: &mut Server<C>) -> bool
    where
        C: Config,
    {
        if self.ticks == 0 {
            return false;
        }

        self.ticks -= 1;
        self.refresh_velocity(server);

        if self.ticks == 0 {
            for block in self.blocks.iter() {
                server.entities.delete(*block);
            }
        }
        true
    }
}
