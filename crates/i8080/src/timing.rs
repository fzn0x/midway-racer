//! Instruction timing, in clock periods ("states").

/// The number of states each opcode takes, indexed by opcode.
///
/// Conditional calls and returns are listed with the cost of the condition
/// being false; taking the branch costs [`BRANCH_TAKEN_PENALTY`] more.
/// Conditional jumps cost the same either way.
#[rustfmt::skip]
pub(crate) const CYCLES: [u8; 256] = [
//  x0  x1  x2  x3  x4  x5  x6  x7  x8  x9  xA  xB  xC  xD  xE  xF
     4, 10,  7,  5,  5,  5,  7,  4,  4, 10,  7,  5,  5,  5,  7,  4, // 0x
     4, 10,  7,  5,  5,  5,  7,  4,  4, 10,  7,  5,  5,  5,  7,  4, // 1x
     4, 10, 16,  5,  5,  5,  7,  4,  4, 10, 16,  5,  5,  5,  7,  4, // 2x
     4, 10, 13,  5, 10, 10, 10,  4,  4, 10, 13,  5,  5,  5,  7,  4, // 3x
     5,  5,  5,  5,  5,  5,  7,  5,  5,  5,  5,  5,  5,  5,  7,  5, // 4x
     5,  5,  5,  5,  5,  5,  7,  5,  5,  5,  5,  5,  5,  5,  7,  5, // 5x
     5,  5,  5,  5,  5,  5,  7,  5,  5,  5,  5,  5,  5,  5,  7,  5, // 6x
     7,  7,  7,  7,  7,  7,  7,  7,  5,  5,  5,  5,  5,  5,  7,  5, // 7x
     4,  4,  4,  4,  4,  4,  7,  4,  4,  4,  4,  4,  4,  4,  7,  4, // 8x
     4,  4,  4,  4,  4,  4,  7,  4,  4,  4,  4,  4,  4,  4,  7,  4, // 9x
     4,  4,  4,  4,  4,  4,  7,  4,  4,  4,  4,  4,  4,  4,  7,  4, // Ax
     4,  4,  4,  4,  4,  4,  7,  4,  4,  4,  4,  4,  4,  4,  7,  4, // Bx
     5, 10, 10, 10, 11, 11,  7, 11,  5, 10, 10, 10, 11, 17,  7, 11, // Cx
     5, 10, 10, 10, 11, 11,  7, 11,  5, 10, 10, 10, 11, 17,  7, 11, // Dx
     5, 10, 10, 18, 11, 11,  7, 11,  5,  5, 10,  4, 11, 17,  7, 11, // Ex
     5, 10, 10,  4, 11, 11,  7, 11,  5,  5, 10,  4, 11, 17,  7, 11, // Fx
];

/// Extra states taken by a conditional call or return whose condition holds.
pub(crate) const BRANCH_TAKEN_PENALTY: u32 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_operands_cost_more_than_registers() {
        assert_eq!(CYCLES[0x41], 5, "MOV B,C");
        assert_eq!(CYCLES[0x46], 7, "MOV B,M");
        assert_eq!(CYCLES[0x70], 7, "MOV M,B");
        assert_eq!(CYCLES[0x80], 4, "ADD B");
        assert_eq!(CYCLES[0x86], 7, "ADD M");
        assert_eq!(CYCLES[0x34], 10, "INR M");
    }

    #[test]
    fn undocumented_opcodes_match_their_documented_twins() {
        for nop in [0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38] {
            assert_eq!(CYCLES[nop], CYCLES[0x00]);
        }
        assert_eq!(CYCLES[0xcb], CYCLES[0xc3], "JMP");
        assert_eq!(CYCLES[0xd9], CYCLES[0xc9], "RET");
        for call in [0xdd, 0xed, 0xfd] {
            assert_eq!(CYCLES[call], CYCLES[0xcd], "CALL");
        }
    }

    #[test]
    fn conditional_calls_and_returns_list_the_not_taken_cost() {
        for opcode in (0xc0..=0xf8).step_by(8) {
            assert_eq!(u32::from(CYCLES[opcode]) + BRANCH_TAKEN_PENALTY, 11);
            assert_eq!(u32::from(CYCLES[opcode + 4]) + BRANCH_TAKEN_PENALTY, 17);
        }
    }
}
