use valence::{
    prelude::Block,
    protocol::{BlockState, ItemKind},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BlockColor {
    block: BlockState,
    item: ItemKind,
    color: Color,
}

impl Color {
    fn dist(&self, color: Color) -> u32 {
        self.r.abs_diff(color.r) as u32
            + self.g.abs_diff(color.g) as u32
            + self.b.abs_diff(color.b) as u32
    }

    pub fn to_block_color(self) -> BlockColor {
        MC_PALLETE
            .iter()
            .min_by_key(|block| block.color.dist(self))
            .unwrap()
            .clone()
    }
}

impl BlockColor {
    pub fn block(&self) -> Block {
        Block::new(self.block)
    }

    pub fn item(&self) -> ItemKind {
        self.item
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self { r, g, b }
    }
}

impl From<[u8; 3]> for Color {
    fn from(value: [u8; 3]) -> Self {
        Self {
            r: value[0],
            g: value[1],
            b: value[2],
        }
    }
}

const MC_PALLETE: [BlockColor; 16] = [
    BlockColor {
        block: BlockState::WHITE_CONCRETE,
        item: ItemKind::WhiteConcrete,
        color: Color {
            r: 209,
            g: 215,
            b: 216,
        },
    },
    BlockColor {
        block: BlockState::ORANGE_CONCRETE,
        item: ItemKind::OrangeConcrete,
        color: Color {
            r: 226,
            g: 97,
            b: 0,
        },
    },
    BlockColor {
        block: BlockState::MAGENTA_CONCRETE,
        item: ItemKind::MagentaConcrete,
        color: Color {
            r: 170,
            g: 45,
            b: 160,
        },
    },
    BlockColor {
        block: BlockState::LIGHT_BLUE_CONCRETE,
        item: ItemKind::LightBlueConcrete,
        color: Color {
            r: 31,
            g: 139,
            b: 201,
        },
    },
    BlockColor {
        block: BlockState::YELLOW_CONCRETE,
        item: ItemKind::YellowConcrete,
        color: Color {
            r: 241,
            g: 176,
            b: 13,
        },
    },
    BlockColor {
        block: BlockState::LIME_CONCRETE,
        item: ItemKind::LimeConcrete,
        color: Color {
            r: 95,
            g: 171,
            b: 19,
        },
    },
    BlockColor {
        block: BlockState::PINK_CONCRETE,
        item: ItemKind::PinkConcrete,
        color: Color {
            r: 215,
            g: 101,
            b: 144,
        },
    },
    BlockColor {
        block: BlockState::GRAY_CONCRETE,
        item: ItemKind::GrayConcrete,
        color: Color {
            r: 52,
            g: 56,
            b: 60,
        },
    },
    BlockColor {
        block: BlockState::LIGHT_GRAY_CONCRETE,
        item: ItemKind::LightGrayConcrete,
        color: Color {
            r: 126,
            g: 126,
            b: 116,
        },
    },
    BlockColor {
        block: BlockState::CYAN_CONCRETE,
        item: ItemKind::CyanConcrete,
        color: Color {
            r: 13,
            g: 120,
            b: 137,
        },
    },
    BlockColor {
        block: BlockState::PURPLE_CONCRETE,
        item: ItemKind::PurpleConcrete,
        color: Color {
            r: 101,
            g: 26,
            b: 158,
        },
    },
    BlockColor {
        block: BlockState::BLUE_CONCRETE,
        item: ItemKind::BlueConcrete,
        color: Color {
            r: 41,
            g: 43,
            b: 145,
        },
    },
    BlockColor {
        block: BlockState::BROWN_CONCRETE,
        item: ItemKind::BrownConcrete,
        color: Color {
            r: 96,
            g: 57,
            b: 25,
        },
    },
    BlockColor {
        block: BlockState::GREEN_CONCRETE,
        item: ItemKind::GreenConcrete,
        color: Color {
            r: 72,
            g: 91,
            b: 31,
        },
    },
    BlockColor {
        block: BlockState::RED_CONCRETE,
        item: ItemKind::RedConcrete,
        color: Color {
            r: 142,
            g: 26,
            b: 26,
        },
    },
    BlockColor {
        block: BlockState::BLACK_CONCRETE,
        item: ItemKind::BlackConcrete,
        color: Color { r: 2, g: 3, b: 7 },
    },
];

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn color_to_block_conversion() {
        let pink = Color {
            r: 233,
            g: 102,
            b: 161,
        };
        let block_color = pink.to_block_color();
        assert_eq!(block_color.block, BlockState::PINK_CONCRETE);
        assert_eq!(block_color.item, ItemKind::PinkConcrete);

        let blue = Color {
            r: 63,
            g: 140,
            b: 240,
        };
        let block_color = blue.to_block_color();
        assert_eq!(block_color.block, BlockState::LIGHT_BLUE_CONCRETE);
        assert_eq!(block_color.item, ItemKind::LightBlueConcrete);

        let yellow = Color {
            r: 191,
            g: 152,
            b: 38,
        };
        let block_color = yellow.to_block_color();
        assert_eq!(block_color.block, BlockState::YELLOW_CONCRETE);
        assert_eq!(block_color.item, ItemKind::YellowConcrete);
    }
}
