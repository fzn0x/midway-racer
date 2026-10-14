//! The Fujitsu MB14241 barrel shifter.
//!
//! The 8080 has no fast way to shift a value by an arbitrary number of
//! bits, which sprite drawing on a bitmap needs constantly. The MB14241
//! (or, on many boards, an equivalent built from TTL chips) does it in
//! hardware: the CPU writes bytes into a 16-bit register and reads back the
//! eight bits found a programmable distance below its top.

/// An MB14241 barrel shifter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mb14241 {
    register: u16,
    amount: u8,
}

impl Mb14241 {
    /// Creates a shifter with a cleared register and a shift of zero.
    pub const fn new() -> Self {
        Self {
            register: 0,
            amount: 0,
        }
    }

    /// Shifts `value` into the top of the register; the previous top byte
    /// moves to the bottom.
    pub const fn write_data(&mut self, value: u8) {
        self.register = self.register >> 8 | (value as u16) << 8;
    }

    /// Sets the shift amount from the low three bits of `value`.
    pub const fn write_amount(&mut self, value: u8) {
        self.amount = value & 0b111;
    }

    /// Returns the byte found `amount` bits below the top of the register.
    pub const fn result(&self) -> u8 {
        (self.register << self.amount >> 8) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_bits_in_from_the_previous_byte() {
        let mut shifter = Mb14241::new();
        shifter.write_data(0b1010_1010);
        shifter.write_data(0b1111_0000);

        let results: Vec<u8> = (0..8)
            .map(|amount| {
                shifter.write_amount(amount);
                shifter.result()
            })
            .collect();
        assert_eq!(
            results,
            [
                0b1111_0000,
                0b1110_0001,
                0b1100_0010,
                0b1000_0101,
                0b0000_1010,
                0b0001_0101,
                0b0010_1010,
                0b0101_0101,
            ]
        );
    }

    #[test]
    fn only_the_low_three_bits_set_the_amount() {
        let mut shifter = Mb14241::new();
        shifter.write_data(0x00);
        shifter.write_data(0x01);
        shifter.write_amount(0xf9);
        assert_eq!(shifter.result(), 0x02);
    }

    #[test]
    fn matches_mame_with_its_inverted_count_and_15_bit_register() {
        // MAME stores `~count & 7` and shifts new data in at bit 7 of a
        // 15-bit register; both formulations must agree everywhere.
        for (previous, value, amount) in [(0x12, 0x34, 0), (0xff, 0x00, 3), (0x81, 0x7e, 7)] {
            let mut shifter = Mb14241::new();
            shifter.write_data(previous);
            shifter.write_data(value);
            shifter.write_amount(amount);

            let mame_register = (u16::from(previous) << 7) >> 8 | u16::from(value) << 7;
            let mame_result = (mame_register >> (!amount & 7)) as u8;
            assert_eq!(shifter.result(), mame_result);
        }
    }
}
