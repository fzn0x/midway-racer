/// The five condition flags of the 8080.
///
/// In the processor status word they occupy the bits `S Z 0 AC 0 P 1 CY`;
/// bit 1 always reads as one and bits 3 and 5 always read as zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[allow(clippy::struct_excessive_bools)] // they really are five independent bits
pub struct Flags {
    /// Set when bit 7 of the result is set.
    pub sign: bool,
    /// Set when the result is zero.
    pub zero: bool,
    /// Set on a carry out of bit 3, used by `DAA`.
    pub aux_carry: bool,
    /// Set when the result has an even number of one bits.
    pub parity: bool,
    /// Set on a carry out of bit 7, or a borrow for subtractions.
    pub carry: bool,
}

impl Flags {
    const SIGN: u8 = 0x80;
    const ZERO: u8 = 0x40;
    const AUX_CARRY: u8 = 0x10;
    const PARITY: u8 = 0x04;
    const ALWAYS_SET: u8 = 0x02;
    const CARRY: u8 = 0x01;

    /// Packs the flags into the low byte of the processor status word.
    pub const fn to_byte(self) -> u8 {
        let mut byte = Self::ALWAYS_SET;
        if self.sign {
            byte |= Self::SIGN;
        }
        if self.zero {
            byte |= Self::ZERO;
        }
        if self.aux_carry {
            byte |= Self::AUX_CARRY;
        }
        if self.parity {
            byte |= Self::PARITY;
        }
        if self.carry {
            byte |= Self::CARRY;
        }
        byte
    }

    /// Unpacks the flags from the low byte of a processor status word.
    pub const fn from_byte(byte: u8) -> Self {
        Self {
            sign: byte & Self::SIGN != 0,
            zero: byte & Self::ZERO != 0,
            aux_carry: byte & Self::AUX_CARRY != 0,
            parity: byte & Self::PARITY != 0,
            carry: byte & Self::CARRY != 0,
        }
    }

    /// Updates sign, zero and parity from `result`, as most ALU operations do.
    pub(crate) const fn set_zsp(&mut self, result: u8) {
        self.sign = result & 0x80 != 0;
        self.zero = result == 0;
        self.parity = result.count_ones() % 2 == 0;
    }
}

/// The programmer-visible register file of the 8080.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Registers {
    /// The accumulator.
    pub a: u8,
    /// The condition flags.
    pub flags: Flags,
    /// Register B, the high byte of pair BC.
    pub b: u8,
    /// Register C, the low byte of pair BC.
    pub c: u8,
    /// Register D, the high byte of pair DE.
    pub d: u8,
    /// Register E, the low byte of pair DE.
    pub e: u8,
    /// Register H, the high byte of pair HL.
    pub h: u8,
    /// Register L, the low byte of pair HL.
    pub l: u8,
    /// The stack pointer.
    pub sp: u16,
    /// The program counter.
    pub pc: u16,
}

impl Registers {
    /// Returns register pair BC.
    pub const fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    /// Sets register pair BC.
    pub const fn set_bc(&mut self, value: u16) {
        [self.b, self.c] = value.to_be_bytes();
    }

    /// Returns register pair DE.
    pub const fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    /// Sets register pair DE.
    pub const fn set_de(&mut self, value: u16) {
        [self.d, self.e] = value.to_be_bytes();
    }

    /// Returns register pair HL.
    pub const fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    /// Sets register pair HL.
    pub const fn set_hl(&mut self, value: u16) {
        [self.h, self.l] = value.to_be_bytes();
    }

    /// Returns the processor status word: the accumulator and the flags.
    pub const fn psw(&self) -> u16 {
        u16::from_be_bytes([self.a, self.flags.to_byte()])
    }

    /// Sets the processor status word: the accumulator and the flags.
    pub const fn set_psw(&mut self, value: u16) {
        let [a, flags] = value.to_be_bytes();
        self.a = a;
        self.flags = Flags::from_byte(flags);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_byte_has_fixed_bits() {
        assert_eq!(Flags::default().to_byte(), 0b0000_0010);
        assert_eq!(Flags::from_byte(0xff).to_byte(), 0b1101_0111);
    }

    #[test]
    fn flag_byte_round_trips() {
        for byte in 0..=u8::MAX {
            let normalized = (byte & 0b1101_0101) | 0b0000_0010;
            assert_eq!(Flags::from_byte(byte).to_byte(), normalized);
        }
    }

    #[test]
    fn set_zsp_computes_sign_zero_and_even_parity() {
        let mut flags = Flags::default();

        flags.set_zsp(0x00);
        assert!(flags.zero && flags.parity && !flags.sign);

        flags.set_zsp(0x80);
        assert!(!flags.zero && !flags.parity && flags.sign);

        flags.set_zsp(0x03);
        assert!(!flags.zero && flags.parity && !flags.sign);
    }

    #[test]
    fn register_pairs_are_big_endian() {
        let mut registers = Registers::default();
        registers.set_bc(0x1234);
        registers.set_de(0x5678);
        registers.set_hl(0x9abc);

        assert_eq!((registers.b, registers.c), (0x12, 0x34));
        assert_eq!((registers.d, registers.e), (0x56, 0x78));
        assert_eq!((registers.h, registers.l), (0x9a, 0xbc));
        assert_eq!(registers.bc(), 0x1234);
        assert_eq!(registers.de(), 0x5678);
        assert_eq!(registers.hl(), 0x9abc);
    }

    #[test]
    fn psw_packs_accumulator_and_flags() {
        let mut registers = Registers::default();
        registers.set_psw(0x42ff);

        assert_eq!(registers.a, 0x42);
        assert!(registers.flags.carry && registers.flags.sign);
        assert_eq!(registers.psw(), 0x42d7);
    }
}
