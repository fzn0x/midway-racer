use crate::timing::CYCLES;
use crate::{Bus, Registers};

/// Clock cycles that elapse for each [`Cpu::step`] while the CPU is halted.
const HALTED_STEP_CYCLES: u32 = 4;

/// An Intel 8080 processor.
///
/// The CPU owns its registers and a free-running cycle counter; memory and
/// I/O live behind the [`Bus`] passed to [`Cpu::step`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cpu {
    /// The programmer-visible registers.
    pub registers: Registers,
    halted: bool,
    cycles: u64,
}

impl Cpu {
    /// Creates a processor in its power-on state, with every register
    /// cleared and interrupts disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asserts the `RESET` input.
    ///
    /// Like the real chip, this clears the program counter and leaves the
    /// halt state, but keeps the other registers unchanged.
    pub const fn reset(&mut self) {
        self.registers.pc = 0;
        self.halted = false;
    }

    /// Returns the number of clock cycles executed since creation.
    pub const fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Returns whether the CPU is stopped by a `HLT` instruction.
    pub const fn is_halted(&self) -> bool {
        self.halted
    }

    /// Executes a single instruction and returns how many clock cycles it
    /// took.
    pub fn step<B: Bus + ?Sized>(&mut self, bus: &mut B) -> u32 {
        let cycles = if self.halted {
            HALTED_STEP_CYCLES
        } else {
            let opcode = self.fetch_byte(bus);
            self.execute(bus, opcode)
        };
        self.cycles += u64::from(cycles);
        cycles
    }

    /// Executes instructions until the cycle counter reaches `cycle`.
    ///
    /// Instructions are never split, so the counter may end up a few cycles
    /// past the target; that surplus is naturally paid back when running
    /// towards the next absolute target.
    pub fn run_until<B: Bus + ?Sized>(&mut self, bus: &mut B, cycle: u64) {
        while self.cycles < cycle {
            self.step(bus);
        }
    }

    fn execute<B: Bus + ?Sized>(&mut self, bus: &mut B, opcode: u8) -> u32 {
        let cycles = u32::from(CYCLES[usize::from(opcode)]);
        match opcode {
            // NOP, and its undocumented aliases.
            0x00 | 0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38 => {}

            // LXI rp,d16
            0x01 | 0x11 | 0x21 | 0x31 => {
                let value = self.fetch_word(bus);
                self.set_pair(opcode >> 4, value);
            }
            // STAX B / STAX D
            0x02 | 0x12 => bus.write(self.pair(opcode >> 4), self.registers.a),
            // LDAX B / LDAX D
            0x0a | 0x1a => self.registers.a = bus.read(self.pair(opcode >> 4)),
            // SHLD a16
            0x22 => {
                let address = self.fetch_word(bus);
                write_word(bus, address, self.registers.hl());
            }
            // LHLD a16
            0x2a => {
                let address = self.fetch_word(bus);
                let value = read_word(bus, address);
                self.registers.set_hl(value);
            }
            // STA a16
            0x32 => {
                let address = self.fetch_word(bus);
                bus.write(address, self.registers.a);
            }
            // LDA a16
            0x3a => {
                let address = self.fetch_word(bus);
                self.registers.a = bus.read(address);
            }
            // MVI r,d8
            0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e => {
                let value = self.fetch_byte(bus);
                self.set_register(bus, opcode >> 3, value);
            }
            // MOV r,r
            0x40..=0x75 | 0x77..=0x7f => {
                let value = self.register(bus, opcode);
                self.set_register(bus, opcode >> 3, value);
            }
            // HLT
            0x76 => self.halted = true,
            // XCHG
            0xeb => {
                let hl = self.registers.hl();
                self.registers.set_hl(self.registers.de());
                self.registers.set_de(hl);
            }

            // ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP r
            0x80..=0xbf => {
                let value = self.register(bus, opcode);
                self.alu(opcode >> 3, value);
            }
            // ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI d8
            0xc6 | 0xce | 0xd6 | 0xde | 0xe6 | 0xee | 0xf6 | 0xfe => {
                let value = self.fetch_byte(bus);
                self.alu(opcode >> 3, value);
            }
            // INR r
            0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => {
                let result = self.register(bus, opcode >> 3).wrapping_add(1);
                let flags = &mut self.registers.flags;
                flags.aux_carry = result & 0x0f == 0x00;
                flags.set_zsp(result);
                self.set_register(bus, opcode >> 3, result);
            }
            // DCR r
            0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => {
                let result = self.register(bus, opcode >> 3).wrapping_sub(1);
                let flags = &mut self.registers.flags;
                flags.aux_carry = result & 0x0f != 0x0f;
                flags.set_zsp(result);
                self.set_register(bus, opcode >> 3, result);
            }
            // INX rp
            0x03 | 0x13 | 0x23 | 0x33 => {
                self.set_pair(opcode >> 4, self.pair(opcode >> 4).wrapping_add(1));
            }
            // DCX rp
            0x0b | 0x1b | 0x2b | 0x3b => {
                self.set_pair(opcode >> 4, self.pair(opcode >> 4).wrapping_sub(1));
            }
            // DAD rp
            0x09 | 0x19 | 0x29 | 0x39 => {
                let (sum, carry) = self.registers.hl().overflowing_add(self.pair(opcode >> 4));
                self.registers.set_hl(sum);
                self.registers.flags.carry = carry;
            }
            // DAA
            0x27 => self.decimal_adjust_accumulator(),

            // RLC
            0x07 => {
                let a = self.registers.a;
                self.registers.a = a.rotate_left(1);
                self.registers.flags.carry = a & 0x80 != 0;
            }
            // RRC
            0x0f => {
                let a = self.registers.a;
                self.registers.a = a.rotate_right(1);
                self.registers.flags.carry = a & 0x01 != 0;
            }
            // RAL
            0x17 => {
                let a = self.registers.a;
                self.registers.a = a << 1 | u8::from(self.registers.flags.carry);
                self.registers.flags.carry = a & 0x80 != 0;
            }
            // RAR
            0x1f => {
                let a = self.registers.a;
                self.registers.a = a >> 1 | u8::from(self.registers.flags.carry) << 7;
                self.registers.flags.carry = a & 0x01 != 0;
            }
            // CMA
            0x2f => self.registers.a = !self.registers.a,
            // STC
            0x37 => self.registers.flags.carry = true,
            // CMC
            0x3f => self.registers.flags.carry = !self.registers.flags.carry,

            _ => todo!("opcode {opcode:#04x} is not implemented yet"),
        }
        cycles
    }

    /// Applies the ALU operation encoded in the low three bits of `code` to
    /// the accumulator and `value`: ADD, ADC, SUB, SBB, ANA, XRA, ORA or CMP.
    fn alu(&mut self, code: u8, value: u8) {
        let a = self.registers.a;
        let carry = self.registers.flags.carry;
        match code & 0b111 {
            0 => self.registers.a = self.add(value, false),
            1 => self.registers.a = self.add(value, carry),
            2 => self.registers.a = self.subtract(value, false),
            3 => self.registers.a = self.subtract(value, carry),
            // The 8080 sets the auxiliary carry of AND to the OR of both
            // operands' bit 3 (the 8085 would always set it).
            4 => self.registers.a = self.logical(a & value, (a | value) & 0x08 != 0),
            5 => self.registers.a = self.logical(a ^ value, false),
            6 => self.registers.a = self.logical(a | value, false),
            _ => {
                self.subtract(value, false);
            }
        }
    }

    /// Sets the flags for the `result` of a logical operation, which always
    /// clears the carry, and returns it.
    const fn logical(&mut self, result: u8, aux_carry: bool) -> u8 {
        let flags = &mut self.registers.flags;
        flags.aux_carry = aux_carry;
        flags.carry = false;
        flags.set_zsp(result);
        result
    }

    /// Adds `value` and a carry to the accumulator, updating every flag, and
    /// returns the sum.
    fn add(&mut self, value: u8, carry_in: bool) -> u8 {
        let (result, aux_carry, carry) = add_with_carries(self.registers.a, value, carry_in);
        let flags = &mut self.registers.flags;
        flags.aux_carry = aux_carry;
        flags.carry = carry;
        flags.set_zsp(result);
        result
    }

    /// Subtracts `value` and a borrow from the accumulator, updating every
    /// flag, and returns the difference.
    ///
    /// The 8080 subtracts by adding the one's complement of the operand
    /// with an inverted borrow. The carry out is inverted back into a
    /// borrow, but the auxiliary carry is left exactly as the adder made it.
    fn subtract(&mut self, value: u8, borrow_in: bool) -> u8 {
        let (result, aux_carry, carry) = add_with_carries(self.registers.a, !value, !borrow_in);
        let flags = &mut self.registers.flags;
        flags.aux_carry = aux_carry;
        flags.carry = !carry;
        flags.set_zsp(result);
        result
    }

    /// Turns the binary sum of two packed BCD numbers into packed BCD (DAA).
    fn decimal_adjust_accumulator(&mut self) {
        let a = self.registers.a;
        let flags = self.registers.flags;
        let mut correction = 0;
        let mut carry = flags.carry;
        if flags.aux_carry || a & 0x0f > 0x09 {
            correction |= 0x06;
        }
        if flags.carry || a > 0x99 {
            correction |= 0x60;
            carry = true;
        }
        self.registers.a = self.add(correction, false);
        self.registers.flags.carry = carry;
    }

    fn fetch_byte<B: Bus + ?Sized>(&mut self, bus: &mut B) -> u8 {
        let byte = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        byte
    }

    fn fetch_word<B: Bus + ?Sized>(&mut self, bus: &mut B) -> u16 {
        let low = self.fetch_byte(bus);
        let high = self.fetch_byte(bus);
        u16::from_le_bytes([low, high])
    }

    /// Reads the register encoded in the low three bits of `code`:
    /// B, C, D, E, H, L, M (the memory byte at HL) or A.
    fn register<B: Bus + ?Sized>(&self, bus: &mut B, code: u8) -> u8 {
        let registers = &self.registers;
        match code & 0b111 {
            0 => registers.b,
            1 => registers.c,
            2 => registers.d,
            3 => registers.e,
            4 => registers.h,
            5 => registers.l,
            6 => bus.read(registers.hl()),
            _ => registers.a,
        }
    }

    /// Writes the register encoded in the low three bits of `code`, see
    /// [`Cpu::register`].
    fn set_register<B: Bus + ?Sized>(&mut self, bus: &mut B, code: u8, value: u8) {
        let registers = &mut self.registers;
        match code & 0b111 {
            0 => registers.b = value,
            1 => registers.c = value,
            2 => registers.d = value,
            3 => registers.e = value,
            4 => registers.h = value,
            5 => registers.l = value,
            6 => bus.write(registers.hl(), value),
            _ => registers.a = value,
        }
    }

    /// Reads the register pair encoded in the low two bits of `code`:
    /// BC, DE, HL or SP.
    const fn pair(&self, code: u8) -> u16 {
        match code & 0b11 {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            _ => self.registers.sp,
        }
    }

    /// Writes the register pair encoded in the low two bits of `code`, see
    /// [`Cpu::pair`].
    const fn set_pair(&mut self, code: u8, value: u16) {
        match code & 0b11 {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            _ => self.registers.sp = value,
        }
    }
}

/// Adds two bytes and a carry. Returns the sum and the carries out of bit 3
/// and bit 7.
fn add_with_carries(a: u8, b: u8, carry_in: bool) -> (u8, bool, bool) {
    let sum = u16::from(a) + u16::from(b) + u16::from(carry_in);
    // Bit n of `carries` is set when there was a carry into bit n.
    let carries = sum ^ u16::from(a) ^ u16::from(b);
    (sum as u8, carries & 0x010 != 0, carries & 0x100 != 0)
}

fn read_word<B: Bus + ?Sized>(bus: &mut B, address: u16) -> u16 {
    let low = bus.read(address);
    let high = bus.read(address.wrapping_add(1));
    u16::from_le_bytes([low, high])
}

fn write_word<B: Bus + ?Sized>(bus: &mut B, address: u16, value: u16) {
    let [low, high] = value.to_le_bytes();
    bus.write(address, low);
    bus.write(address.wrapping_add(1), high);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 64 KiB of flat RAM and no I/O devices.
    struct TestBus {
        memory: Vec<u8>,
    }

    impl TestBus {
        fn with_program(program: &[u8]) -> Self {
            let mut memory = vec![0; 0x1_0000];
            memory[..program.len()].copy_from_slice(program);
            Self { memory }
        }
    }

    impl Bus for TestBus {
        fn read(&mut self, address: u16) -> u8 {
            self.memory[usize::from(address)]
        }

        fn write(&mut self, address: u16, value: u8) {
            self.memory[usize::from(address)] = value;
        }

        fn input(&mut self, _port: u8) -> u8 {
            0
        }

        fn output(&mut self, _port: u8, _value: u8) {}
    }

    /// Runs `program` from address zero until it halts. The cycles spent on
    /// the final `HLT` are not counted.
    fn run(cpu: &mut Cpu, program: &[u8]) -> TestBus {
        let mut bus = TestBus::with_program(program);
        while !cpu.is_halted() {
            cpu.step(&mut bus);
        }
        cpu.cycles -= u64::from(CYCLES[0x76]);
        bus
    }

    /// Builds flags from a `"SZAPC"` pattern where `.` marks a clear flag,
    /// e.g. `"S..P."` for sign and parity.
    fn flags(pattern: &str) -> crate::Flags {
        let set: Vec<bool> = pattern.chars().map(|flag| flag != '.').collect();
        assert_eq!(set.len(), 5, "pattern must have five flags");
        crate::Flags {
            sign: set[0],
            zero: set[1],
            aux_carry: set[2],
            parity: set[3],
            carry: set[4],
        }
    }

    #[test]
    fn power_on_state_is_cleared() {
        let cpu = Cpu::new();
        assert_eq!(cpu, Cpu::default());
        assert_eq!(cpu.registers, Registers::default());
        assert_eq!(cpu.cycles(), 0);
        assert!(!cpu.is_halted());
    }

    #[test]
    fn mvi_and_mov_copy_between_registers() {
        let mut cpu = Cpu::new();
        // MVI B,42H ; MOV C,B ; MOV A,C ; HLT
        run(&mut cpu, &[0x06, 0x42, 0x48, 0x79, 0x76]);
        assert_eq!(cpu.registers.b, 0x42);
        assert_eq!(cpu.registers.c, 0x42);
        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.cycles(), 7 + 5 + 5);
    }

    #[test]
    fn m_operand_addresses_memory_through_hl() {
        let mut cpu = Cpu::new();
        // LXI H,0100H ; MVI M,99H ; MOV E,M ; HLT
        let bus = run(&mut cpu, &[0x21, 0x00, 0x01, 0x36, 0x99, 0x5e, 0x76]);
        assert_eq!(bus.memory[0x0100], 0x99);
        assert_eq!(cpu.registers.e, 0x99);
        assert_eq!(cpu.cycles(), 10 + 10 + 7);
    }

    #[test]
    fn lxi_loads_every_register_pair() {
        let mut cpu = Cpu::new();
        let program = [
            0x01, 0x34, 0x12, // LXI B,1234H
            0x11, 0x78, 0x56, // LXI D,5678H
            0x21, 0xbc, 0x9a, // LXI H,9ABCH
            0x31, 0xf0, 0xde, // LXI SP,0DEF0H
            0x76, // HLT
        ];
        run(&mut cpu, &program);
        assert_eq!(cpu.registers.bc(), 0x1234);
        assert_eq!(cpu.registers.de(), 0x5678);
        assert_eq!(cpu.registers.hl(), 0x9abc);
        assert_eq!(cpu.registers.sp, 0xdef0);
        assert_eq!(cpu.cycles(), 4 * 10);
    }

    #[test]
    fn direct_addressing_loads_and_stores() {
        let mut cpu = Cpu::new();
        let program = [
            0x3e, 0x5a, // MVI A,5AH
            0x32, 0x00, 0x20, // STA 2000H
            0x21, 0xcd, 0xab, // LXI H,0ABCDH
            0x22, 0x02, 0x20, // SHLD 2002H
            0x2a, 0x01, 0x20, // LHLD 2001H
            0x3a, 0x03, 0x20, // LDA 2003H
            0x76, // HLT
        ];
        let bus = run(&mut cpu, &program);
        assert_eq!(bus.memory[0x2000..0x2004], [0x5a, 0x00, 0xcd, 0xab]);
        assert_eq!(cpu.registers.hl(), 0xcd00);
        assert_eq!(cpu.registers.a, 0xab);
        assert_eq!(cpu.cycles(), 7 + 13 + 10 + 16 + 16 + 13);
    }

    #[test]
    fn indirect_addressing_goes_through_bc_and_de() {
        let mut cpu = Cpu::new();
        let program = [
            0x01, 0x00, 0x30, // LXI B,3000H
            0x11, 0x01, 0x30, // LXI D,3001H
            0x3e, 0x11, // MVI A,11H
            0x02, // STAX B
            0x3e, 0x22, // MVI A,22H
            0x12, // STAX D
            0x0a, // LDAX B
            0x47, // MOV B,A
            0x1a, // LDAX D
            0x76, // HLT
        ];
        let bus = run(&mut cpu, &program);
        assert_eq!(bus.memory[0x3000..0x3002], [0x11, 0x22]);
        assert_eq!(cpu.registers.b, 0x11);
        assert_eq!(cpu.registers.a, 0x22);
    }

    #[test]
    fn xchg_swaps_de_and_hl() {
        let mut cpu = Cpu::new();
        // LXI D,1234H ; LXI H,5678H ; XCHG ; HLT
        run(&mut cpu, &[0x11, 0x34, 0x12, 0x21, 0x78, 0x56, 0xeb, 0x76]);
        assert_eq!(cpu.registers.de(), 0x5678);
        assert_eq!(cpu.registers.hl(), 0x1234);
    }

    #[test]
    fn undocumented_nops_only_burn_cycles() {
        let mut cpu = Cpu::new();
        run(
            &mut cpu,
            &[0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x76],
        );
        assert_eq!(cpu.registers.pc, 9);
        assert_eq!(cpu.cycles(), 8 * 4);
    }

    #[test]
    fn add_sets_every_flag() {
        let mut cpu = Cpu::new();
        // MVI A,6CH ; MVI D,2EH ; ADD D ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x3e, 0x6c, 0x16, 0x2e, 0x82, 0x76]);
        assert_eq!(cpu.registers.a, 0x9a);
        assert_eq!(cpu.registers.flags, flags("S.AP."));
        assert_eq!(cpu.cycles(), 7 + 7 + 4);
    }

    #[test]
    fn adc_adds_the_carry() {
        let mut cpu = Cpu::new();
        // MVI A,42H ; MVI C,3DH ; ADC C ; HLT
        run(&mut cpu, &[0x3e, 0x42, 0x0e, 0x3d, 0x89, 0x76]);
        assert_eq!(cpu.registers.a, 0x7f);
        assert_eq!(cpu.registers.flags, flags("....."));

        let mut cpu = Cpu::new();
        cpu.registers.flags.carry = true;
        run(&mut cpu, &[0x3e, 0x42, 0x0e, 0x3d, 0x89, 0x76]);
        assert_eq!(cpu.registers.a, 0x80);
        assert_eq!(cpu.registers.flags, flags("S.A.."));
    }

    #[test]
    fn add_immediate_reports_carry_out_of_bit_7() {
        let mut cpu = Cpu::new();
        // MVI A,0F0H ; ADI 20H ; ACI 00H ; HLT
        run(&mut cpu, &[0x3e, 0xf0, 0xc6, 0x20, 0xce, 0x00, 0x76]);
        assert_eq!(cpu.registers.a, 0x11);
        assert_eq!(cpu.registers.flags, flags("...P."));
        assert_eq!(cpu.cycles(), 7 + 7 + 7);
    }

    #[test]
    fn sub_of_itself_clears_the_accumulator() {
        let mut cpu = Cpu::new();
        // MVI A,3EH ; SUB A ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x3e, 0x3e, 0x97, 0x76]);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flags, flags(".ZAP."));
    }

    #[test]
    fn sbb_subtracts_the_borrow() {
        let mut cpu = Cpu::new();
        cpu.registers.flags.carry = true;
        // MVI A,04H ; MVI L,02H ; SBB L ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x3e, 0x04, 0x2e, 0x02, 0x9d, 0x76]);
        assert_eq!(cpu.registers.a, 0x01);
        assert_eq!(cpu.registers.flags, flags("..A.."));
    }

    #[test]
    fn subtract_immediate_sets_borrow() {
        let mut cpu = Cpu::new();
        // MVI A,01H ; SUI 02H ; SBI 00H ; HLT
        run(&mut cpu, &[0x3e, 0x01, 0xd6, 0x02, 0xde, 0x00, 0x76]);
        assert_eq!(cpu.registers.a, 0xfe);
        assert_eq!(cpu.registers.flags, flags("S.A.."));
    }

    #[test]
    fn inr_and_dcr_keep_carry_and_track_aux_carry() {
        let mut cpu = Cpu::new();
        cpu.registers.flags.carry = true;
        // MVI B,0FFH ; INR B ; HLT
        run(&mut cpu, &[0x06, 0xff, 0x04, 0x76]);
        assert_eq!(cpu.registers.b, 0x00);
        assert_eq!(cpu.registers.flags, flags(".ZAPC"));

        let mut cpu = Cpu::new();
        // LXI H,0100H ; MVI M,40H ; DCR M ; HLT
        let bus = run(&mut cpu, &[0x21, 0x00, 0x01, 0x36, 0x40, 0x35, 0x76]);
        assert_eq!(bus.memory[0x0100], 0x3f);
        assert_eq!(cpu.registers.flags, flags("...P."));
        assert_eq!(cpu.cycles(), 10 + 10 + 10);
    }

    #[test]
    fn inx_and_dcx_wrap_without_touching_flags() {
        let mut cpu = Cpu::new();
        // LXI D,0FFFFH ; INX D ; DCX SP ; HLT
        run(&mut cpu, &[0x11, 0xff, 0xff, 0x13, 0x3b, 0x76]);
        assert_eq!(cpu.registers.de(), 0x0000);
        assert_eq!(cpu.registers.sp, 0xffff);
        assert_eq!(cpu.registers.flags, flags("....."));
        assert_eq!(cpu.cycles(), 10 + 5 + 5);
    }

    #[test]
    fn dad_adds_pairs_and_only_sets_carry() {
        let mut cpu = Cpu::new();
        // LXI B,339FH ; LXI H,0A17BH ; DAD B ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x01, 0x9f, 0x33, 0x21, 0x7b, 0xa1, 0x09, 0x76]);
        assert_eq!(cpu.registers.hl(), 0xd51a);
        assert!(!cpu.registers.flags.carry);

        let mut cpu = Cpu::new();
        // LXI H,8000H ; DAD H ; HLT
        run(&mut cpu, &[0x21, 0x00, 0x80, 0x29, 0x76]);
        assert_eq!(cpu.registers.hl(), 0x0000);
        assert_eq!(cpu.registers.flags, flags("....C"));
        assert_eq!(cpu.cycles(), 10 + 10);
    }

    #[test]
    fn daa_adjusts_both_nibbles() {
        let mut cpu = Cpu::new();
        // MVI A,9BH ; DAA ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x3e, 0x9b, 0x27, 0x76]);
        assert_eq!(cpu.registers.a, 0x01);
        assert!(cpu.registers.flags.carry && cpu.registers.flags.aux_carry);
    }

    #[test]
    fn daa_completes_bcd_addition() {
        let mut cpu = Cpu::new();
        // MVI A,38H ; ADI 45H ; DAA ; HLT  (38 + 45 = 83)
        run(&mut cpu, &[0x3e, 0x38, 0xc6, 0x45, 0x27, 0x76]);
        assert_eq!(cpu.registers.a, 0x83);
        assert!(!cpu.registers.flags.carry);

        let mut cpu = Cpu::new();
        // MVI A,99H ; ADI 01H ; DAA ; HLT  (99 + 1 = 100)
        run(&mut cpu, &[0x3e, 0x99, 0xc6, 0x01, 0x27, 0x76]);
        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.flags.carry && cpu.registers.flags.zero);
    }

    #[test]
    fn ana_clears_carry_and_ors_bit_3_into_aux_carry() {
        let mut cpu = Cpu::new();
        cpu.registers.flags.carry = true;
        // MVI A,0FCH ; MVI C,0FH ; ANA C ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x3e, 0xfc, 0x0e, 0x0f, 0xa1, 0x76]);
        assert_eq!(cpu.registers.a, 0x0c);
        assert_eq!(cpu.registers.flags, flags("..AP."));

        let mut cpu = Cpu::new();
        // MVI A,0F0H ; ANI 70H ; HLT
        run(&mut cpu, &[0x3e, 0xf0, 0xe6, 0x70, 0x76]);
        assert_eq!(cpu.registers.a, 0x70);
        assert_eq!(cpu.registers.flags, flags("....."));
    }

    #[test]
    fn xra_and_ora_clear_both_carries() {
        let mut cpu = Cpu::new();
        cpu.registers.flags = flags("S.A.C");
        // MVI A,5AH ; XRA A ; HLT
        run(&mut cpu, &[0x3e, 0x5a, 0xaf, 0x76]);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flags, flags(".Z.P."));

        let mut cpu = Cpu::new();
        cpu.registers.flags = flags("..A.C");
        // MVI A,33H ; ORI 0FH ; HLT
        run(&mut cpu, &[0x3e, 0x33, 0xf6, 0x0f, 0x76]);
        assert_eq!(cpu.registers.a, 0x3f);
        assert_eq!(cpu.registers.flags, flags("...P."));

        let mut cpu = Cpu::new();
        // MVI A,0FFH ; MVI B,0F0H ; XRI 0FH ; ORA B ; HLT
        run(&mut cpu, &[0x3e, 0xff, 0x06, 0xf0, 0xee, 0x0f, 0xb0, 0x76]);
        assert_eq!(cpu.registers.a, 0xf0);
    }

    #[test]
    fn cmp_compares_without_storing() {
        let mut cpu = Cpu::new();
        // MVI A,0AH ; MVI E,05H ; CMP E ; HLT  (Intel 8080 manual example)
        run(&mut cpu, &[0x3e, 0x0a, 0x1e, 0x05, 0xbb, 0x76]);
        assert_eq!(cpu.registers.a, 0x0a);
        assert!(!cpu.registers.flags.carry && !cpu.registers.flags.zero);

        let mut cpu = Cpu::new();
        // MVI A,02H ; CPI 05H ; HLT
        run(&mut cpu, &[0x3e, 0x02, 0xfe, 0x05, 0x76]);
        assert_eq!(cpu.registers.a, 0x02);
        assert!(cpu.registers.flags.carry && !cpu.registers.flags.zero);

        let mut cpu = Cpu::new();
        // MVI A,42H ; CPI 42H ; HLT
        run(&mut cpu, &[0x3e, 0x42, 0xfe, 0x42, 0x76]);
        assert!(!cpu.registers.flags.carry && cpu.registers.flags.zero);
    }

    #[test]
    fn rotates_move_bits_through_or_around_the_carry() {
        // (opcode, accumulator, carry in, expected accumulator, carry out),
        // from the Intel 8080 manual examples.
        let cases = [
            (0x07, 0xf2, false, 0xe5, true),  // RLC
            (0x0f, 0xf2, false, 0x79, false), // RRC
            (0x17, 0xb5, false, 0x6a, true),  // RAL
            (0x1f, 0x6a, true, 0xb5, false),  // RAR
        ];
        for (opcode, a, carry, expected, expected_carry) in cases {
            let mut cpu = Cpu::new();
            cpu.registers.flags.carry = carry;
            run(&mut cpu, &[0x3e, a, opcode, 0x76]);
            assert_eq!(cpu.registers.a, expected, "opcode {opcode:#04x}");
            assert_eq!(
                cpu.registers.flags.carry, expected_carry,
                "opcode {opcode:#04x}"
            );
            assert_eq!(cpu.cycles(), 7 + 4);
        }
    }

    #[test]
    fn cma_stc_and_cmc_flip_bits() {
        let mut cpu = Cpu::new();
        // MVI A,51H ; CMA ; STC ; HLT
        run(&mut cpu, &[0x3e, 0x51, 0x2f, 0x37, 0x76]);
        assert_eq!(cpu.registers.a, 0xae);
        assert_eq!(cpu.registers.flags, flags("....C"));

        let mut cpu = Cpu::new();
        // CMC ; CMC ; CMC ; HLT
        run(&mut cpu, &[0x3f, 0x3f, 0x3f, 0x76]);
        assert_eq!(cpu.registers.flags, flags("....C"));
    }

    #[test]
    fn halted_cpu_idles_until_reset() {
        let mut cpu = Cpu::new();
        let mut bus = TestBus::with_program(&[0x76]);
        cpu.step(&mut bus);
        assert!(cpu.is_halted());

        assert_eq!(cpu.step(&mut bus), HALTED_STEP_CYCLES);
        assert_eq!(cpu.registers.pc, 1);

        cpu.reset();
        assert!(!cpu.is_halted());
        assert_eq!(cpu.registers.pc, 0);
    }

    #[test]
    fn run_until_stops_at_or_just_past_the_target() {
        let mut cpu = Cpu::new();
        // MVI A,00H (7 cycles), repeated.
        let mut bus = TestBus::with_program(&[0x3e, 0x00].repeat(16));
        cpu.run_until(&mut bus, 20);
        assert_eq!(cpu.cycles(), 21);
        cpu.run_until(&mut bus, 21);
        assert_eq!(cpu.cycles(), 21);
    }
}
