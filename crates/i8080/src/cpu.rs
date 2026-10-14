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

            _ => todo!("opcode {opcode:#04x} is not implemented yet"),
        }
        cycles
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
