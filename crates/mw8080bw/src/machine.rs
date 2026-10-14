//! The complete board, run one video frame at a time.

use i8080::{Bus, Cpu};

use crate::controls::{Controls, Gear};
use crate::game::Game;
use crate::mb14241::Mb14241;
use crate::memory::{Memory, RAM_SIZE};
use crate::rom::ProgramRom;
use crate::timing::{CYCLES_PER_FRAME, CYCLES_PER_LINE, LINES_PER_FRAME, VISIBLE_LINES};
use crate::video::Screen;
use crate::watchdog::Watchdog;

/// Scan line at which the mid-screen interrupt fires: vertical counter
/// `80h`, outside blanking.
const MID_SCREEN_LINE: u32 = 96;
/// Scan line at which the vertical blanking interrupt fires: vertical
/// counter `DAh`, inside blanking.
const VBLANK_LINE: u32 = VISIBLE_LINES;
/// `RST 1`, which the board jams onto the bus at mid-screen.
const RST_1: u8 = 0xcf;
/// `RST 2`, which the board jams onto the bus at vertical blanking.
const RST_2: u8 = 0xd7;

/// Frames the coin switch stays closed for each coin, and open between
/// consecutive coins: 100 ms, as a coin rolling through the mechanism.
const COIN_PULSE_FRAMES: u8 = 6;

/// A Midway 8080 board running one of the racing games.
///
/// ```
/// use mw8080bw::{Game, Machine, ProgramRom};
///
/// // A program that enables interrupts and spins; real games come from
/// // `RomFiles::program_rom`.
/// let rom = ProgramRom::from_image(Game::LagunaRacer, &[0xfb, 0xc3, 0x01, 0x00])?;
/// let mut machine = Machine::new(rom);
/// machine.run_frame();
/// assert_eq!(machine.frame(), 1);
/// # Ok::<(), mw8080bw::RomError>(())
/// ```
#[derive(Debug)]
pub struct Machine {
    game: Game,
    cpu: Cpu,
    board: Board,
    screen: Screen,
    /// CPU cycle at which the next frame starts.
    frame_start: u64,
    frame: u64,
}

impl Machine {
    /// Powers up a board with `rom` installed and the game's factory DIP
    /// switch settings.
    pub fn new(rom: ProgramRom) -> Self {
        let (game, image) = rom.into_parts();
        Self {
            game,
            cpu: Cpu::new(),
            board: Board {
                memory: Memory::new(image),
                shifter: Mb14241::new(),
                watchdog: Watchdog::new(0),
                controls: Controls::default(),
                dip_switches: game.info().default_dip_switches,
                coin: CoinMechanism::default(),
                sound: SoundLatches::default(),
                now: 0,
            },
            screen: Screen::new(),
            frame_start: 0,
            frame: 0,
        }
    }

    /// Returns the game installed on the board.
    pub fn game(&self) -> Game {
        self.game
    }

    /// Resets the board as the watchdog does: the program restarts, RAM
    /// keeps its contents.
    pub fn reset(&mut self) {
        let now = self.cpu.cycles();
        self.cpu.reset();
        self.cpu.clear_interrupt();
        self.board.shifter = Mb14241::new();
        self.board.watchdog = Watchdog::new(now);
        self.board.sound = SoundLatches::default();
    }

    /// Returns the positions of the cabinet's controls.
    pub fn controls(&self) -> Controls {
        self.board.controls
    }

    /// Moves the cabinet's controls.
    pub fn set_controls(&mut self, controls: Controls) {
        self.board.controls = controls;
    }

    /// Drops a coin into the coin mechanism. Coins inserted in quick
    /// succession queue up and are registered one after the other.
    pub fn insert_coin(&mut self) {
        self.board.coin.insert();
    }

    /// Returns the DIP switch bank.
    pub fn dip_switches(&self) -> u8 {
        self.board.dip_switches
    }

    /// Sets the DIP switch bank. Most settings are read when a game
    /// starts.
    pub fn set_dip_switches(&mut self, dip_switches: u8) {
        self.board.dip_switches = dip_switches;
    }

    /// Runs the board for one video frame.
    ///
    /// The CPU runs scan line by scan line, each line being drawn as the
    /// beam passes it, and the two interrupts fire at the lines where the
    /// hardware raises them.
    pub fn run_frame(&mut self) {
        self.board.coin.advance();
        for line in 0..LINES_PER_FRAME {
            match line {
                MID_SCREEN_LINE => self.interrupt(RST_1),
                VBLANK_LINE => self.interrupt(RST_2),
                _ => {}
            }
            self.run_until(self.frame_start + u64::from((line + 1) * CYCLES_PER_LINE));
            if line < VISIBLE_LINES {
                self.screen
                    .render_line(line as usize, self.board.memory.video_ram());
            }
            if self.board.watchdog.has_fired(self.cpu.cycles()) {
                self.reset();
            }
        }
        self.frame_start += u64::from(CYCLES_PER_FRAME);
        self.frame += 1;
    }

    /// Returns the picture drawn during the last frame.
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Returns how many frames have run since power-up.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Returns the values last written to the sound board's latches.
    pub fn sound_latches(&self) -> SoundLatches {
        self.board.sound
    }

    /// Returns the CPU, for inspection.
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    /// Returns the RAM, for inspection.
    pub fn ram(&self) -> &[u8; RAM_SIZE] {
        self.board.memory.ram()
    }

    /// Reads a byte of the CPU's address space without side effects.
    pub fn peek(&self, address: u16) -> u8 {
        self.board.memory.read(address)
    }

    /// Raises an interrupt request, which the board's flip-flop only
    /// latches while the CPU's `INTE` output is high.
    fn interrupt(&mut self, instruction: u8) {
        if self.cpu.interrupts_enabled() {
            self.cpu.raise_interrupt(instruction);
        } else {
            self.cpu.clear_interrupt();
        }
    }

    fn run_until(&mut self, cycle: u64) {
        while self.cpu.cycles() < cycle {
            self.board.now = self.cpu.cycles();
            self.cpu.step(&mut self.board);
        }
    }
}

/// The values last written to the sound board's two latches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SoundLatches {
    /// Port 2: accelerator level in bits 0–3, high gear in bit 4 and low
    /// gear in bit 5.
    pub engine: u8,
    /// Port 5: boom in bit 0, engine off in bit 1, tyre skid in bit 2,
    /// post-crash noise in bit 3 and the coin counter in bit 5.
    pub effects: u8,
}

/// Everything the CPU reaches over its bus.
#[derive(Debug)]
struct Board {
    memory: Memory,
    shifter: Mb14241,
    watchdog: Watchdog,
    controls: Controls,
    dip_switches: u8,
    coin: CoinMechanism,
    sound: SoundLatches,
    /// CPU cycle at which the current instruction started.
    now: u64,
}

impl Board {
    /// Input port 0: pedal in bits 0–3, gear in bit 4, and the active-low
    /// coin and start switches in bits 6 and 7.
    fn input_port_0(&self) -> u8 {
        let mut value = self.controls.pedal_value();
        if self.controls.gear == Gear::High {
            value |= 0x10;
        }
        if !self.coin.is_switch_closed() {
            value |= 0x40;
        }
        if !self.controls.start {
            value |= 0x80;
        }
        value
    }
}

impl Bus for Board {
    fn read(&mut self, address: u16) -> u8 {
        self.memory.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }

    /// Ports 4–7 mirror ports 0–3 on reads.
    fn input(&mut self, port: u8) -> u8 {
        match port & 0b11 {
            0 => self.input_port_0(),
            1 => self.controls.wheel_value(),
            2 => self.dip_switches,
            _ => self.shifter.result(),
        }
    }

    fn output(&mut self, port: u8, value: u8) {
        match port & 0b111 {
            2 => self.sound.engine = value,
            3 => self.shifter.write_data(value),
            4 => self.shifter.write_amount(value),
            5 => self.sound.effects = value,
            7 => self.watchdog.kick(self.now),
            _ => {}
        }
    }
}

/// Feeds queued coins through the coin switch, holding it closed long
/// enough for the program to notice each one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CoinMechanism {
    queued: u8,
    phase: CoinPhase,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CoinPhase {
    #[default]
    Idle,
    /// The switch is closed for this many more frames.
    Closed(u8),
    /// The switch is open for this many more frames before the next coin.
    Open(u8),
}

impl CoinMechanism {
    fn insert(&mut self) {
        self.queued = self.queued.saturating_add(1);
    }

    fn is_switch_closed(self) -> bool {
        matches!(self.phase, CoinPhase::Closed(_))
    }

    /// Advances the mechanism by one frame.
    fn advance(&mut self) {
        self.phase = match self.phase {
            CoinPhase::Closed(frames) if frames > 1 => CoinPhase::Closed(frames - 1),
            CoinPhase::Closed(_) => CoinPhase::Open(COIN_PULSE_FRAMES),
            CoinPhase::Open(frames) if frames > 1 => CoinPhase::Open(frames - 1),
            CoinPhase::Open(_) | CoinPhase::Idle if self.queued > 0 => {
                self.queued -= 1;
                CoinPhase::Closed(COIN_PULSE_FRAMES)
            }
            CoinPhase::Open(_) | CoinPhase::Idle => CoinPhase::Idle,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Boots `program` on a Laguna Racer board.
    fn boot(program: &[u8]) -> Machine {
        Machine::new(ProgramRom::from_image(Game::LagunaRacer, program).unwrap())
    }

    /// Places `code` at `address` in `program`, growing it as needed.
    fn place(program: &mut Vec<u8>, address: usize, code: &[u8]) {
        if program.len() < address + code.len() {
            program.resize(address + code.len(), 0x00);
        }
        program[address..address + code.len()].copy_from_slice(code);
    }

    #[test]
    fn interrupts_fire_at_mid_screen_and_vertical_blanking() {
        let mut program = Vec::new();
        place(&mut program, 0x0000, &[0x31, 0x00, 0x24]); // LXI SP,2400H
        place(&mut program, 0x0003, &[0xfb]); // EI
        place(&mut program, 0x0004, &[0xc3, 0x04, 0x00]); // JMP 0004H
        place(&mut program, 0x0008, &[0xc3, 0x20, 0x00]); // RST 1: JMP 0020H
        place(&mut program, 0x0010, &[0xc3, 0x30, 0x00]); // RST 2: JMP 0030H
        #[rustfmt::skip]
        place(&mut program, 0x0020, &[
            0x21, 0x00, 0x20, // LXI H,2000H
            0x34,             // INR M
            0x3e, 0xff,       // MVI A,0FFH
            0x32, 0x00, 0x24, // STA 2400H: line 0, already drawn this frame
            0x32, 0x00, 0x3d, // STA 3D00H: line 200, drawn later this frame
            0xfb, 0xc9,       // EI ; RET
        ]);
        #[rustfmt::skip]
        place(&mut program, 0x0030, &[
            0x21, 0x01, 0x20, // LXI H,2001H
            0x34,             // INR M
            0xfb, 0xc9,       // EI ; RET
        ]);
        let mut machine = boot(&program);

        machine.run_frame();
        assert_eq!(machine.ram()[..2], [1, 1], "one interrupt of each kind");
        assert!(machine.screen().is_lit(4, 200));
        assert!(!machine.screen().is_lit(4, 0), "the beam had passed line 0");

        machine.run_frame();
        assert_eq!(machine.ram()[..2], [2, 2]);
        assert!(machine.screen().is_lit(4, 0));
    }

    #[test]
    fn interrupts_are_dropped_while_disabled() {
        // JMP 0000H, with interrupts never enabled.
        let mut machine = boot(&[0xc3, 0x00, 0x00]);
        machine.run_frame();
        assert!(!machine.cpu().interrupt_pending());
    }

    #[test]
    fn frames_take_exactly_33536_cycles_on_average() {
        let mut machine = boot(&[0xc3, 0x00, 0x00]); // JMP 0000H
        for _ in 0..10 {
            machine.run_frame();
        }
        let surplus = machine.cpu().cycles() - 10 * u64::from(CYCLES_PER_FRAME);
        assert!(surplus < 10, "overshoot is bounded by one instruction");
    }

    #[test]
    fn watchdog_resets_a_program_that_stops_kicking_it() {
        #[rustfmt::skip]
        let hang = [
            0x21, 0x02, 0x20, // LXI H,2002H
            0x34,             // INR M: count boots
            0xc3, 0x04, 0x00, // JMP 0004H
        ];
        let mut machine = boot(&hang);
        for _ in 0..70 {
            machine.run_frame();
        }
        assert_eq!(machine.ram()[2], 2, "rebooted once after 1.1 s");

        #[rustfmt::skip]
        let kick = [
            0x21, 0x02, 0x20, // LXI H,2002H
            0x34,             // INR M
            0xd3, 0x07,       // OUT 7: kick the watchdog
            0xc3, 0x04, 0x00, // JMP 0004H
        ];
        let mut machine = boot(&kick);
        for _ in 0..70 {
            machine.run_frame();
        }
        assert_eq!(machine.ram()[2], 1);
    }

    #[test]
    fn input_ports_report_controls_switches_and_coins() {
        #[rustfmt::skip]
        let program = [
            0xdb, 0x00, 0x32, 0x10, 0x20, // IN 0 ; STA 2010H
            0xdb, 0x01, 0x32, 0x11, 0x20, // IN 1 ; STA 2011H
            0xdb, 0x02, 0x32, 0x12, 0x20, // IN 2 ; STA 2012H
            0xdb, 0x04, 0x32, 0x13, 0x20, // IN 4 ; STA 2013H (mirror of port 0)
            0xd3, 0x07,                   // OUT 7
            0xc3, 0x00, 0x00,             // JMP 0000H
        ];
        let mut machine = boot(&program);
        machine.set_controls(Controls {
            steering: 1.0,
            accelerator: 1.0,
            gear: Gear::High,
            start: false,
        });
        machine.run_frame();
        assert_eq!(machine.ram()[0x10..0x14], [0xdf, 0x01, 0x24, 0xdf]);

        machine.set_dip_switches(0xc3);
        machine.insert_coin();
        machine.run_frame();
        assert_eq!(machine.ram()[0x10..0x13], [0x9f, 0x01, 0xc3]);
        assert_eq!(machine.dip_switches(), 0xc3);
    }

    #[test]
    fn output_ports_drive_the_shifter_and_sound_latches() {
        #[rustfmt::skip]
        let program = [
            0x3e, 0xaa, 0xd3, 0x03, // MVI A,0AAH ; OUT 3
            0x3e, 0xf0, 0xd3, 0x03, // MVI A,0F0H ; OUT 3
            0x3e, 0x03, 0xd3, 0x04, // MVI A,03H ; OUT 4
            0xdb, 0x03,             // IN 3
            0x32, 0x20, 0x20,       // STA 2020H
            0x3e, 0x2f, 0xd3, 0x02, // MVI A,2FH ; OUT 2
            0x3e, 0x06, 0xd3, 0x05, // MVI A,06H ; OUT 5
            0x76,                   // HLT
        ];
        let mut machine = boot(&program);
        machine.run_frame();
        assert_eq!(machine.ram()[0x20], 0x85);
        assert_eq!(
            machine.sound_latches(),
            SoundLatches {
                engine: 0x2f,
                effects: 0x06
            }
        );

        machine.reset();
        assert_eq!(machine.sound_latches(), SoundLatches::default());
        assert_eq!(machine.cpu().registers.pc, 0);
        assert_eq!(machine.ram()[0x20], 0x85, "RAM survives a reset");
    }

    #[test]
    fn coins_queue_and_pulse_the_switch() {
        let mut coin = CoinMechanism::default();
        coin.insert();
        coin.insert();

        let mut pattern = String::new();
        for _ in 0..30 {
            coin.advance();
            pattern.push(if coin.is_switch_closed() { '#' } else { '.' });
        }
        assert_eq!(pattern, "######......######............");
    }

    #[test]
    fn peek_reads_through_the_memory_map() {
        let mut machine = boot(&[0x3e, 0x42, 0x32, 0x00, 0x60, 0x76]); // MVI A,42H ; STA 6000H ; HLT
        machine.run_frame();
        assert_eq!(machine.peek(0x0001), 0x42);
        assert_eq!(machine.peek(0x2000), 0x42);
        assert_eq!(machine.game(), Game::LagunaRacer);
    }
}
