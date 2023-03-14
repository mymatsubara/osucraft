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
    pub fn new(hit_score: HitScore, origin: BlockPos, ticks: usize, instance: Entity) -> Self {
        Self {
            score: hit_score,
            ticks,
            origin,
            instance,
        }
    }

    pub fn spawn(self, commands: &mut Commands, instances: &mut Query<&mut Instance>) -> Entity {
        let block_state = match self.score {
            HitScore::Hit300 => BlockState::LIGHT_BLUE_CONCRETE,
            HitScore::Hit100 => BlockState::GREEN_CONCRETE,
            HitScore::Hit50 => BlockState::YELLOW_CONCRETE,
            HitScore::Miss => BlockState::RED_CONCRETE,
        };
        let block = Block::new(block_state);

        self.draw(block, &mut instances.get_mut(self.instance).unwrap());

        commands.spawn(self).id()
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
