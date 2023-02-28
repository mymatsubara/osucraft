use valence::{instance::ChunkEntry, prelude::*};

const DEFAULT_SCREEN_SIZE: (f64, f64) = (640.0, 480.0);
const DEFAULT_SPAWN_POS: DVec3 = DVec3::new(DEFAULT_SCREEN_SIZE.0 / 2.0, 240.0, -340.0);

#[derive(Resource)]
pub struct Osu {
    scale: f64,
}

impl Osu {
    pub fn new(scale: f64) -> Self {
        Self { scale }
    }

    pub fn init(&self, instance: &mut Instance) {
        self.init_chunks(instance);
        self.init_screen(instance);
        self.init_player_spawn(instance);
    }

    fn init_chunks(&self, instance: &mut Instance) {
        let (max_x, _) = self.screen_size();
        let max_z = self.player_spawn_pos().z as i32;

        for x in -1..=(max_x / 16) + 1 {
            for z in (max_z / 16) - 1..=1 {
                if let ChunkEntry::Vacant(chunk) = instance.chunk_entry([x, z]) {
                    chunk.insert(Default::default());
                }
            }
        }
    }

    fn init_screen(&self, instance: &mut Instance) {
        let (max_x, max_y) = self.screen_size();

        for x in 0..=max_x {
            for y in 0..=max_y {
                instance.set_block(
                    BlockPos { x, y, z: 1 },
                    Block::new(BlockState::BLACK_CONCRETE),
                );
            }
        }
    }

    fn init_player_spawn(&self, instance: &mut Instance) {
        let spawn_pos = self.player_spawn_pos();

        let block_pos = BlockPos {
            x: spawn_pos.x as i32,
            y: spawn_pos.y as i32 - 1,
            z: spawn_pos.z as i32,
        };

        instance.set_block(block_pos, Block::new(BlockState::BEDROCK));
    }

    fn screen_size(&self) -> (i32, i32) {
        let x = (DEFAULT_SCREEN_SIZE.0 * self.scale) as i32;
        let y = (DEFAULT_SCREEN_SIZE.1 * self.scale) as i32;

        (x, y)
    }

    pub fn player_spawn_pos(&self) -> DVec3 {
        DEFAULT_SPAWN_POS * self.scale
    }
}
