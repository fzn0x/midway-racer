use core::fmt;

const REGISTERS: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];
const PAIRS: [&str; 4] = ["B", "D", "H", "SP"];
const STACK_PAIRS: [&str; 4] = ["B", "D", "H", "PSW"];
const CONDITIONS: [&str; 8] = ["NZ", "Z", "NC", "C", "PO", "PE", "P", "M"];
const ALU: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];
const ALU_IMMEDIATE: [&str; 8] = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];

/// A decoded instruction, which displays in Intel assembler syntax.
///
/// Undocumented opcodes are shown with a leading `*`, e.g. `*NOP`.
///
/// ```
/// use i8080::Instruction;
///
/// let instruction = Instruction::decode(&[0x21, 0x00, 0x24]);
/// assert_eq!(instruction.to_string(), "LXI H,2400H");
/// assert_eq!(instruction.size(), 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Instruction {
    opcode: u8,
    operand: u16,
}

impl Instruction {
    /// Decodes the instruction at the start of `bytes`. Operand bytes that a
    /// short slice lacks read as zero.
    pub fn decode(bytes: &[u8]) -> Self {
        let byte = |index: usize| bytes.get(index).copied().unwrap_or_default();
        let opcode = byte(0);
        let operand = match size_of(opcode) {
            2 => u16::from(byte(1)),
            3 => u16::from_le_bytes([byte(1), byte(2)]),
            _ => 0,
        };
        Self { opcode, operand }
    }

    /// Returns the opcode byte.
    pub const fn opcode(self) -> u8 {
        self.opcode
    }

    /// Returns the length of the instruction in bytes, from one to three.
    pub const fn size(self) -> u16 {
        size_of(self.opcode)
    }
}

/// Returns the length in bytes of the instruction starting with `opcode`.
const fn size_of(opcode: u8) -> u16 {
    match opcode {
        // MVI, the ALU immediates, OUT and IN.
        0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e | 0xc6 | 0xce | 0xd6 | 0xde
        | 0xe6 | 0xee | 0xf6 | 0xfe | 0xd3 | 0xdb => 2,
        // LXI, SHLD, LHLD, STA, LDA, jumps and calls.
        0x01 | 0x11 | 0x21 | 0x31 | 0x22 | 0x2a | 0x32 | 0x3a | 0xc2 | 0xc3 | 0xc4 | 0xca
        | 0xcb | 0xcc | 0xcd | 0xd2 | 0xd4 | 0xda | 0xdc | 0xdd | 0xe2 | 0xe4 | 0xea | 0xec
        | 0xed | 0xf2 | 0xf4 | 0xfa | 0xfc | 0xfd => 3,
        _ => 1,
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opcode = self.opcode;
        let destination = REGISTERS[usize::from(opcode >> 3 & 0b111)];
        let source = REGISTERS[usize::from(opcode & 0b111)];
        let pair = PAIRS[usize::from(opcode >> 4 & 0b11)];
        let stack_pair = STACK_PAIRS[usize::from(opcode >> 4 & 0b11)];
        let condition = CONDITIONS[usize::from(opcode >> 3 & 0b111)];
        let byte = IntelHex::byte(self.operand as u8);
        let word = IntelHex::word(self.operand);

        match opcode {
            0x00 => f.write_str("NOP"),
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38 => f.write_str("*NOP"),
            0x01 | 0x11 | 0x21 | 0x31 => write!(f, "LXI {pair},{word}"),
            0x02 | 0x12 => write!(f, "STAX {pair}"),
            0x0a | 0x1a => write!(f, "LDAX {pair}"),
            0x03 | 0x13 | 0x23 | 0x33 => write!(f, "INX {pair}"),
            0x0b | 0x1b | 0x2b | 0x3b => write!(f, "DCX {pair}"),
            0x09 | 0x19 | 0x29 | 0x39 => write!(f, "DAD {pair}"),
            0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => write!(f, "INR {destination}"),
            0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => write!(f, "DCR {destination}"),
            0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e => {
                write!(f, "MVI {destination},{byte}")
            }
            0x07 => f.write_str("RLC"),
            0x0f => f.write_str("RRC"),
            0x17 => f.write_str("RAL"),
            0x1f => f.write_str("RAR"),
            0x22 => write!(f, "SHLD {word}"),
            0x27 => f.write_str("DAA"),
            0x2a => write!(f, "LHLD {word}"),
            0x2f => f.write_str("CMA"),
            0x32 => write!(f, "STA {word}"),
            0x37 => f.write_str("STC"),
            0x3a => write!(f, "LDA {word}"),
            0x3f => f.write_str("CMC"),
            0x76 => f.write_str("HLT"),
            0x40..=0x75 | 0x77..=0x7f => write!(f, "MOV {destination},{source}"),
            0x80..=0xbf => write!(f, "{} {source}", ALU[usize::from(opcode >> 3 & 0b111)]),
            0xc0 | 0xc8 | 0xd0 | 0xd8 | 0xe0 | 0xe8 | 0xf0 | 0xf8 => write!(f, "R{condition}"),
            0xc1 | 0xd1 | 0xe1 | 0xf1 => write!(f, "POP {stack_pair}"),
            0xc5 | 0xd5 | 0xe5 | 0xf5 => write!(f, "PUSH {stack_pair}"),
            0xc2 | 0xca | 0xd2 | 0xda | 0xe2 | 0xea | 0xf2 | 0xfa => {
                write!(f, "J{condition} {word}")
            }
            0xc3 => write!(f, "JMP {word}"),
            0xcb => write!(f, "*JMP {word}"),
            0xc4 | 0xcc | 0xd4 | 0xdc | 0xe4 | 0xec | 0xf4 | 0xfc => {
                write!(f, "C{condition} {word}")
            }
            0xc6 | 0xce | 0xd6 | 0xde | 0xe6 | 0xee | 0xf6 | 0xfe => {
                write!(
                    f,
                    "{} {byte}",
                    ALU_IMMEDIATE[usize::from(opcode >> 3 & 0b111)]
                )
            }
            0xc7 | 0xcf | 0xd7 | 0xdf | 0xe7 | 0xef | 0xf7 | 0xff => {
                write!(f, "RST {}", opcode >> 3 & 0b111)
            }
            0xc9 => f.write_str("RET"),
            0xd9 => f.write_str("*RET"),
            0xcd => write!(f, "CALL {word}"),
            0xdd | 0xed | 0xfd => write!(f, "*CALL {word}"),
            0xd3 => write!(f, "OUT {byte}"),
            0xdb => write!(f, "IN {byte}"),
            0xe3 => f.write_str("XTHL"),
            0xe9 => f.write_str("PCHL"),
            0xeb => f.write_str("XCHG"),
            0xf3 => f.write_str("DI"),
            0xf9 => f.write_str("SPHL"),
            0xfb => f.write_str("EI"),
        }
    }
}

/// Formats a number the way Intel's assembler expects it: in hexadecimal,
/// with an `H` suffix and a leading zero when the first digit is a letter.
struct IntelHex {
    value: u16,
    digits: usize,
}

impl IntelHex {
    const fn byte(value: u8) -> Self {
        Self {
            value: value as u16,
            digits: 2,
        }
    }

    const fn word(value: u16) -> Self {
        Self { value, digits: 4 }
    }
}

impl fmt::Display for IntelHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let leading_digit = self.value >> ((self.digits - 1) * 4);
        if leading_digit >= 0xa {
            f.write_str("0")?;
        }
        write!(f, "{:0width$X}H", self.value, width = self.digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Cpu};

    #[test]
    fn formats_operands_in_intel_syntax() {
        let cases: [(&[u8], &str); 16] = [
            (&[0x00], "NOP"),
            (&[0x10], "*NOP"),
            (&[0x3e, 0x2a], "MVI A,2AH"),
            (&[0x36, 0xff], "MVI M,0FFH"),
            (&[0x31, 0x00, 0x24], "LXI SP,2400H"),
            (&[0x2a, 0x7b, 0xa1], "LHLD 0A17BH"),
            (&[0x78], "MOV A,B"),
            (&[0x76], "HLT"),
            (&[0x9e], "SBB M"),
            (&[0xfe, 0x0a], "CPI 0AH"),
            (&[0xc8], "RZ"),
            (&[0xe2, 0x34, 0x12], "JPO 1234H"),
            (&[0xfc, 0x00, 0xc0], "CM 0C000H"),
            (&[0xf5], "PUSH PSW"),
            (&[0xcf], "RST 1"),
            (&[0xdd, 0x05, 0x00], "*CALL 0005H"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(Instruction::decode(bytes).to_string(), expected);
        }
    }

    #[test]
    fn missing_operand_bytes_read_as_zero() {
        let instruction = Instruction::decode(&[0xc3, 0x34]);
        assert_eq!(instruction.to_string(), "JMP 0034H");
        assert_eq!(Instruction::decode(&[]).to_string(), "NOP");
    }

    struct Ram(Vec<u8>);

    impl Bus for Ram {
        fn read(&mut self, address: u16) -> u8 {
            self.0[usize::from(address)]
        }

        fn write(&mut self, address: u16, value: u8) {
            self.0[usize::from(address)] = value;
        }

        fn input(&mut self, _port: u8) -> u8 {
            0
        }

        fn output(&mut self, _port: u8, _value: u8) {}
    }

    /// Whether `opcode` may move the program counter somewhere else:
    /// returns, jumps, calls, restarts and PCHL.
    fn transfers_control(opcode: u8) -> bool {
        matches!(opcode & 0b1100_0111, 0xc0 | 0xc2 | 0xc4 | 0xc7)
            || matches!(
                opcode,
                0xc3 | 0xcb | 0xc9 | 0xd9 | 0xcd | 0xdd | 0xed | 0xfd | 0xe9
            )
    }

    #[test]
    fn sizes_match_what_the_cpu_consumes() {
        for opcode in (0..=u8::MAX).filter(|&opcode| !transfers_control(opcode)) {
            let instruction = Instruction::decode(&[opcode]);
            let mut ram = Ram(vec![0; 0x1_0000]);
            ram.0[0x1000] = opcode;
            let mut cpu = Cpu::new();
            cpu.registers.pc = 0x1000;
            cpu.registers.sp = 0x8000;
            cpu.step(&mut ram);
            assert_eq!(
                cpu.registers.pc,
                0x1000 + instruction.size(),
                "{instruction} ({opcode:#04x})"
            );
        }
    }
}
