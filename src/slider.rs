use std::f64::consts::TAU;

use anyhow::Result;
use bevy_ecs::{
    prelude::{Component, Entity},
    system::Commands,
};
use valence::{
    prelude::DVec3,
    protocol::{entity_meta::EulerAngle, ItemKind},
};

use crate::ring::{create_rotated_item, Ring};

#[derive(Component)]
pub struct Slider {
    ticks: usize,
    radius: f64,
    head: Entity,
    tail: Entity,
    body: Entity,
}

#[derive(Component)]
pub struct SliderBody {}

#[derive(Component)]
pub struct SliderBodyPart;

impl Slider {
    pub fn new(
        start: DVec3,
        end: DVec3,
        radius: f64,
        instance: Entity,
        commands: &mut Commands,
    ) -> Result<Slider> {
        let ticks = 10000;
        let border_item = ItemKind::WhiteConcrete;

        let head = Ring::without_speed(start, radius, border_item, ticks, instance, commands)?;
        let head = commands.spawn(head).id();

        let tail = Ring::without_speed(end, radius, border_item, ticks, instance, commands)?;
        let tail = commands.spawn(tail).id();

        let body = SliderBody::new(start, end, radius, instance, commands);
        let body = commands.spawn(body).id();

        Ok(Self {
            radius,
            ticks,
            head,
            tail,
            body,
        })
    }
}

impl SliderBody {
    fn new(
        start: DVec3,
        end: DVec3,
        radius: f64,
        instance: Entity,
        commands: &mut Commands,
    ) -> Self {
        let vec = end - start;
        let dir = vec / vec.length();
        let angle = dir.dot(DVec3::new(1.0, 0.0, 0.0));
        let angle_degrees = (360.0 * angle / TAU) as f32;
        let perp_vec = if vec.x != 0.0 {
            DVec3::new(-vec.y, vec.x, 0.0)
        } else {
            DVec3::new(0.0, vec.y, 0.0)
        };
        let perp_dir = perp_vec / perp_vec.length();

        let rotation = EulerAngle {
            pitch: 0.0,
            yaw: 0.0,
            roll: -angle_degrees + 90.0,
        };

        // Offset to place block border exactly on the start and end
        let offset_start = start + dir * 0.25;
        let offset_end = end - dir * 0.25;
        let offset_vec = offset_end - offset_start;

        let armor_stands_count = (offset_vec.length() / 0.25).ceil() as usize;
        let delta = offset_vec / armor_stands_count as f64;

        let line_points = (0..armor_stands_count).map(|i| offset_start + delta * i as f64);
        let upper_line_points = line_points.clone().map(|point| point + perp_dir * radius);
        let lower_line_points = line_points.map(|point| point - perp_dir * radius);

        // Spawn slider body
        upper_line_points
            .chain(lower_line_points)
            .map(|point| {
                let (mc_entity, equipment) =
                    create_rotated_item(ItemKind::WhiteConcrete, rotation, point, instance);

                (mc_entity, equipment, SliderBodyPart)
            })
            .for_each(|bundle| {
                commands.spawn(bundle);
            });

        Self {}
    }
}
