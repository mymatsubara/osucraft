use std::cmp::max;

use valence::prelude::*;

const DIGIT_SIZE: (usize, usize) = (3, 5);
const DIGIT_MASKS: [[[bool; DIGIT_SIZE.0]; DIGIT_SIZE.1]; 10] = [
    // 0
    [
        [true, true, true],
        [true, false, true],
        [true, false, true],
        [true, false, true],
        [true, true, true],
    ],
    // 1
    [
        [false, true, false],
        [false, true, false],
        [false, true, false],
        [false, true, false],
        [false, true, false],
    ],
    // 2
    [
        [true, true, true],
        [false, false, true],
        [true, true, true],
        [true, false, false],
        [true, true, true],
    ],
    // 3
    [
        [true, true, true],
        [false, false, true],
        [false, true, true],
        [false, false, true],
        [true, true, true],
    ],
    // 4
    [
        [true, false, true],
        [true, false, true],
        [true, true, true],
        [false, false, true],
        [false, false, true],
    ],
    // 5
    [
        [true, true, true],
        [true, false, false],
        [true, true, true],
        [false, false, true],
        [true, true, true],
    ],
    // 6
    [
        [true, true, true],
        [true, false, false],
        [true, true, true],
        [true, false, true],
        [true, true, true],
    ],
    // 7
    [
        [true, true, true],
        [false, false, true],
        [false, false, true],
        [false, false, true],
        [false, false, true],
    ],
    // 8
    [
        [true, true, true],
        [true, false, true],
        [true, true, true],
        [true, false, true],
        [true, true, true],
    ],
    // 9
    [
        [true, true, true],
        [true, false, true],
        [true, true, true],
        [false, false, true],
        [true, true, true],
    ],
];

pub enum TextPosition {
    Right,
    Center,
    Left,
}

pub struct DigitWriter {
    pub scale: usize,
    pub position: TextPosition,
}

impl DigitWriter {
    pub fn iter_block_positions(
        &self,
        number: usize,
        origin: BlockPos,
    ) -> impl Iterator<Item = impl Iterator<Item = BlockPos>> + '_ {
        // Get number of digits from number
        let digits_iter = DigitsIter::new(number);
        let digits = digits_iter.len() as i32;

        // Calculate offset for each digit
        let scale = self.scale;
        let digit_spacing = scale as i32;

        let digit_size = ((DIGIT_SIZE.0 * scale) as i32, (DIGIT_SIZE.1 * scale) as i32);
        let position_offset: BlockPos = match self.position {
            TextPosition::Right => BlockPos { x: 0, y: 0, z: 0 },
            TextPosition::Center => BlockPos {
                x: ((digit_size.0 + digit_spacing) * (digits - 1)) / 2 - 1,
                y: -digit_size.1 / 2 - 1,
                z: 0,
            },
            TextPosition::Left => BlockPos {
                x: digit_size.0 * digits + digit_spacing * (digits - 1),
                y: 0,
                z: 0,
            },
        };

        digits_iter
            .enumerate()
            .map(move |(i, digit)| {
                let digit_offset = BlockPos {
                    x: i as i32 * -(digit_size.0 + digit_spacing),
                    y: 0,
                    z: 0,
                };

                (digit, digit_offset + position_offset + origin)
            })
            .map(|(digit, digit_origin)| self.iter_digit_block_positions(digit, digit_origin))
    }

    /// `base` is the position of the digit's bottom left block
    fn iter_digit_block_positions(
        &self,
        digit: u8,
        origin: BlockPos,
    ) -> impl Iterator<Item = BlockPos> {
        let scale = self.scale;
        let digit = digit as usize;

        (0..DIGIT_SIZE.1).flat_map(move |y| {
            (0..DIGIT_SIZE.0)
                .filter(move |&x| has_block(digit, x, y))
                .flat_map(move |x| {
                    (0..scale).flat_map(move |x_offset| {
                        (0..scale).map(move |y_offset| BlockPos {
                            x: (x * scale + x_offset) as i32 + origin.x,
                            y: (y * scale + y_offset) as i32 + origin.y,
                            z: origin.z,
                        })
                    })
                })
        })
    }
}

#[derive(Clone, Copy)]
struct DigitsIter {
    number: usize,
    digits: u8,
}

impl DigitsIter {
    fn new(number: usize) -> Self {
        let mut temp = number;
        let mut digits = 1;

        while temp >= 10 {
            temp /= 10;
            digits += 1;
        }

        Self { number, digits }
    }

    fn len(&self) -> u8 {
        self.digits
    }
}

impl Iterator for DigitsIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.digits == 0 {
            return None;
        }

        self.digits -= 1;
        let quotient = max(10_usize.pow(self.digits as u32), 1);
        let result = (self.number / quotient) as u8;

        self.number %= quotient;

        Some(result)
    }
}

fn has_block(digit: usize, x: usize, y: usize) -> bool {
    DIGIT_MASKS[digit][DIGIT_SIZE.1 - y - 1][DIGIT_SIZE.0 - x - 1]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn digits_iter() {
        assert_eq!(DigitsIter::new(0).collect::<Vec<_>>(), vec![0]);
        assert_eq!(DigitsIter::new(9).collect::<Vec<_>>(), vec![9]);
        assert_eq!(DigitsIter::new(10).collect::<Vec<_>>(), vec![1, 0]);
        assert_eq!(DigitsIter::new(666).collect::<Vec<_>>(), vec![6, 6, 6]);
        assert_eq!(DigitsIter::new(1000).collect::<Vec<_>>(), vec![1, 0, 0, 0]);
    }
}
