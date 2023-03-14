use bevy_ecs::{
    prelude::{Component, Entity},
    system::{Commands, Query},
    world::Mut,
};

use valence::{
    prelude::{Block, Instance},
    protocol::{BlockPos, BlockState},
    Despawned,
};

use crate::digit::{DigitWriter, TextPosition};

#[derive(Debug, Copy, Clone)]
pub enum HitScore {
    Hit300,
    Hit100,
    Hit50,
    Miss,
}

#[derive(Component, Clone)]
pub struct HitScoreNumber {
    ticks: usize,
    score: HitScore,
    origin: BlockPos,
    instance: Entity,
}

impl HitScoreNumber {
    pub fn new(
        hit_score: HitScore,
        origin: BlockPos,
        ticks: usize,
        mut instance: (Entity, Mut<Instance>),
    ) -> Self {
        let hit_score_number = Self {
            score: hit_score,
            ticks,
            origin,
            instance: instance.0,
        };

        let block_state = match hit_score_number.score {
            HitScore::Hit300 => BlockState::LIGHT_BLUE_STAINED_GLASS,
            HitScore::Hit100 => BlockState::LIME_STAINED_GLASS,
            HitScore::Hit50 => BlockState::ORANGE_STAINED_GLASS,
            HitScore::Miss => BlockState::RED_STAINED_GLASS,
        };
        let block = Block::new(block_state);

        hit_score_number.draw(block, &mut instance.1);

        hit_score_number
    }

    pub fn despawn(&self, instances: &mut Query<&mut Instance>) {
        if let Ok(mut instance) = instances.get_mut(self.instance) {
            self.draw(Block::new(BlockState::AIR), &mut instance);
        }
    }

    fn draw(&self, block: Block, instance: &mut Mut<Instance>) {
        let origin = self.origin;
        let number = match self.score {
            HitScore::Hit300 => 300,
            HitScore::Hit100 => 100,
            HitScore::Hit50 => 50,
            HitScore::Miss => {
                [
                    (2, 2),
                    (1, 1),
                    (0, 0),
                    (-1, -1),
                    (-2, -2),
                    (-2, 2),
                    (-1, 1),
                    (1, -1),
                    (2, -2),
                ]
                .iter()
                .map(|offset| BlockPos {
                    x: origin.x + offset.0,
                    y: origin.y + offset.1,
                    z: origin.z,
                })
                .for_each(|pos| {
                    instance.set_block(pos, block.clone());
                });

                return;
            }
        };

        DigitWriter {
            scale: 1,
            position: TextPosition::Center,
        }
        .draw(number, self.origin, block, instance);
    }
}

pub fn update_score_hit_numbers(
    mut commands: Commands,
    mut hit_score_numbers: Query<(Entity, &mut HitScoreNumber)>,
    mut instances: Query<&mut Instance>,
) {
    for (entity, mut hit_score_number) in &mut hit_score_numbers {
        if hit_score_number.ticks == 0 {
            hit_score_number.despawn(&mut instances);
            commands.entity(entity).insert(Despawned);
        } else {
            hit_score_number.ticks -= 1;
        }
    }
}
