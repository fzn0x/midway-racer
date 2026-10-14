//! WebAssembly bindings that run Laguna Racer and 280-ZZZAP in the
//! browser.
//!
//! The page collects ROM files into a [`RomLibrary`], creates an
//! [`Emulator`] from it and, once per frame, hands the player's
//! [`InputState`] to [`Emulator::run_frame`] before drawing
//! [`Emulator::draw_picture`] and queueing [`Emulator::audio`].

use i8080::Instruction;
use mw8080bw::{
    ChipStatus, Controls, DigitalPedal, DigitalSteering, Game, Gear, Machine, Orientation, Rgb,
    RomFiles,
};
use wasm_bindgen::prelude::*;

/// Returns the refresh rate of the board, about 59.54 frames per second.
#[wasm_bindgen(js_name = frameRate)]
pub fn frame_rate() -> f64 {
    mw8080bw::timing::FRAME_RATE_HZ
}

/// Describes every supported game.
#[wasm_bindgen(js_name = supportedGames)]
pub fn supported_games() -> Vec<GameDescription> {
    Game::ALL.into_iter().map(GameDescription::new).collect()
}

/// A supported game and the ROM files it needs.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct GameDescription {
    /// MAME's short name for the set, which names its zip archive.
    #[wasm_bindgen(getter_with_clone, js_name = shortName)]
    pub short_name: String,
    /// The game's title.
    #[wasm_bindgen(getter_with_clone)]
    pub title: String,
    /// Year of release.
    pub year: u16,
    /// Who made it.
    #[wasm_bindgen(getter_with_clone)]
    pub manufacturer: String,
    /// The file names of the ROM chips.
    #[wasm_bindgen(getter_with_clone)]
    pub chips: Vec<String>,
    /// The DIP switches.
    #[wasm_bindgen(getter_with_clone, js_name = dipSwitches)]
    pub dip_switches: Vec<DipSwitchDescription>,
    /// The factory setting of the DIP switch bank.
    #[wasm_bindgen(js_name = defaultDipSwitches)]
    pub default_dip_switches: u8,
}

impl GameDescription {
    fn new(game: Game) -> Self {
        let info = game.info();
        Self {
            short_name: info.short_name.to_owned(),
            title: info.title.to_owned(),
            year: info.year,
            manufacturer: info.manufacturer.to_owned(),
            chips: info.roms.iter().map(|chip| chip.name.to_owned()).collect(),
            dip_switches: info
                .dip_switches
                .iter()
                .map(|switch| DipSwitchDescription {
                    name: switch.name.to_owned(),
                    mask: switch.mask,
                    values: switch
                        .settings
                        .iter()
                        .map(|setting| setting.value)
                        .collect(),
                    labels: switch
                        .settings
                        .iter()
                        .map(|setting| setting.label.to_owned())
                        .collect(),
                })
                .collect(),
            default_dip_switches: info.default_dip_switches,
        }
    }
}

/// A group of DIP switches and its settings.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct DipSwitchDescription {
    /// What the switches control.
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    /// The bits of the switch bank the group occupies.
    pub mask: u8,
    /// The value of each setting.
    #[wasm_bindgen(getter_with_clone)]
    pub values: Vec<u8>,
    /// The description of each setting.
    #[wasm_bindgen(getter_with_clone)]
    pub labels: Vec<String>,
}

/// ROM files collected from the player, such as MAME's `lagunar.zip`.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct RomLibrary {
    files: RomFiles,
}

#[wasm_bindgen]
impl RomLibrary {
    /// Creates an empty library.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a ROM dump, or the files inside a zip archive.
    ///
    /// # Errors
    ///
    /// Fails when a zip archive cannot be read.
    #[wasm_bindgen(js_name = addFile)]
    pub fn add_file(&mut self, name: &str, data: Vec<u8>) -> Result<(), JsError> {
        self.files
            .add_file(name, data)
            .map_err(|error| JsError::new(&format!("{name}: {error}")))
    }

    /// Returns how many files the library holds.
    #[wasm_bindgen(getter, js_name = fileCount)]
    pub fn file_count(&self) -> usize {
        self.files.files().len()
    }

    /// Returns the short name of the game the files match best, if any.
    pub fn identify(&self) -> Option<String> {
        self.files
            .identify()
            .map(|game| game.info().short_name.to_owned())
    }

    /// Checks every ROM chip of the game called `short_name`.
    ///
    /// # Errors
    ///
    /// Fails when no supported game has that short name.
    pub fn check(&self, short_name: &str) -> Result<Vec<ChipReport>, JsError> {
        let report = self.files.check(game(short_name)?);
        Ok(report
            .chips
            .iter()
            .map(|check| ChipReport {
                name: check.chip.name.to_owned(),
                status: match check.status {
                    ChipStatus::Verified { .. } => "verified",
                    ChipStatus::ChecksumMismatch { .. } => "mismatch",
                    ChipStatus::WrongSize { .. } => "wrong-size",
                    ChipStatus::Missing => "missing",
                }
                .to_owned(),
                detail: check.status.to_string(),
                usable: check.status.is_usable(),
            })
            .collect())
    }
}

/// What was found for one ROM chip.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ChipReport {
    /// The chip's file name.
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    /// `verified`, `mismatch`, `wrong-size` or `missing`.
    #[wasm_bindgen(getter_with_clone)]
    pub status: String,
    /// A sentence describing what was found.
    #[wasm_bindgen(getter_with_clone)]
    pub detail: String,
    /// Whether the chip's data can be used.
    pub usable: bool,
}

/// The player's input for one frame. The page updates it as keys,
/// buttons and touches change.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Default)]
#[expect(clippy::struct_excessive_bools, reason = "independent buttons")]
pub struct InputState {
    /// Steer left with a digital input, such as a key.
    pub left: bool,
    /// Steer right with a digital input.
    pub right: bool,
    /// Accelerate with a digital input.
    pub accelerate: bool,
    /// Analogue steering added to the digital one, from -1 (left) to 1
    /// (right), such as a gamepad stick.
    pub steering: f32,
    /// Analogue accelerator travel, from 0 to 1, such as a trigger.
    pub accelerator: f32,
    /// Whether the shifter is in high gear.
    #[wasm_bindgen(js_name = highGear)]
    pub high_gear: bool,
    /// Whether the start button is held.
    pub start: bool,
}

#[wasm_bindgen]
impl InputState {
    /// Creates an input with nothing pressed.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A Midway board running one of the racing games.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Emulator {
    machine: Machine,
    steering: DigitalSteering,
    pedal: DigitalPedal,
    controls: Controls,
    picture: Vec<u32>,
}

#[wasm_bindgen]
impl Emulator {
    /// Powers up a board running the game called `short_name` from the
    /// library's files, synthesizing sound at `sample_rate` samples per
    /// second, or silently when it is zero.
    ///
    /// # Errors
    ///
    /// Fails when the game is unknown or its ROMs are incomplete.
    #[wasm_bindgen(constructor)]
    pub fn new(library: &RomLibrary, short_name: &str, sample_rate: u32) -> Result<Self, JsError> {
        let rom = library.files.program_rom(game(short_name)?)?;
        let mut machine = Machine::new(rom);
        if sample_rate > 0 {
            machine.enable_audio(sample_rate);
        }
        let (width, height) = machine.game().info().orientation.picture_size();
        Ok(Self {
            machine,
            steering: DigitalSteering::new(),
            pedal: DigitalPedal::new(),
            controls: Controls::default(),
            picture: vec![0; width * height],
        })
    }

    /// Returns the title of the game.
    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.machine.game().info().title.to_owned()
    }

    /// Returns the short name of the game.
    #[wasm_bindgen(getter, js_name = shortName)]
    pub fn short_name(&self) -> String {
        self.machine.game().info().short_name.to_owned()
    }

    /// Returns whether the monitor is mounted vertically.
    #[wasm_bindgen(getter)]
    pub fn vertical(&self) -> bool {
        self.machine.game().info().orientation == Orientation::Vertical
    }

    /// Returns the width of the picture in pixels.
    #[wasm_bindgen(getter, js_name = pictureWidth)]
    pub fn picture_width(&self) -> usize {
        self.machine.game().info().orientation.picture_size().0
    }

    /// Returns the height of the picture in pixels.
    #[wasm_bindgen(getter, js_name = pictureHeight)]
    pub fn picture_height(&self) -> usize {
        self.machine.game().info().orientation.picture_size().1
    }

    /// Returns the DIP switch bank.
    #[wasm_bindgen(getter, js_name = dipSwitches)]
    pub fn dip_switches(&self) -> u8 {
        self.machine.dip_switches()
    }

    /// Sets the DIP switch bank.
    #[wasm_bindgen(setter, js_name = dipSwitches)]
    pub fn set_dip_switches(&mut self, dip_switches: u8) {
        self.machine.set_dip_switches(dip_switches);
    }

    /// Returns the steering wheel position of the last frame, -1 to 1.
    #[wasm_bindgen(getter)]
    pub fn steering(&self) -> f32 {
        self.controls.steering
    }

    /// Returns the accelerator travel of the last frame, 0 to 1.
    #[wasm_bindgen(getter)]
    pub fn accelerator(&self) -> f32 {
        self.controls.accelerator
    }

    /// Runs one frame with the player's `input`.
    #[wasm_bindgen(js_name = runFrame)]
    pub fn run_frame(&mut self, input: &InputState) {
        let seconds = (1.0 / mw8080bw::timing::FRAME_RATE_HZ) as f32;
        let steering = self.steering.update(input.left, input.right, seconds);
        let accelerator = self.pedal.update(input.accelerate, seconds);
        self.controls = Controls {
            steering: (steering + input.steering).clamp(-1.0, 1.0),
            accelerator: accelerator.max(input.accelerator).clamp(0.0, 1.0),
            gear: if input.high_gear {
                Gear::High
            } else {
                Gear::Low
            },
            start: input.start,
        };
        self.machine.set_controls(self.controls);
        self.machine.run_frame();
    }

    /// Drops a coin into the coin mechanism.
    #[wasm_bindgen(js_name = insertCoin)]
    pub fn insert_coin(&mut self) {
        self.machine.insert_coin();
    }

    /// Resets the board, as the watchdog does.
    pub fn reset(&mut self) {
        self.machine.reset();
    }

    /// Draws the picture as RGBA pixels into WebAssembly memory and returns
    /// its address, for a `Uint8ClampedArray` view of `pictureWidth *
    /// pictureHeight * 4` bytes.
    #[wasm_bindgen(js_name = drawPicture)]
    pub fn draw_picture(&mut self) -> *const u32 {
        let game = self.machine.game();
        self.machine
            .screen()
            .draw_picture(game, &mut self.picture, to_rgba);
        self.picture.as_ptr()
    }

    /// Returns the sound of the last frame, as mono samples from -1 to 1.
    pub fn audio(&self) -> Vec<f32> {
        self.machine.audio_samples().to_vec()
    }

    /// Describes the CPU registers, for the debugger.
    pub fn registers(&self) -> String {
        let cpu = self.machine.cpu();
        let registers = cpu.registers;
        let flags = registers.flags;
        let flag = |set: bool, name: char| if set { name } else { '-' };
        format!(
            "PC {:04X}  SP {:04X}\nA  {:02X}    {}{}{}{}{}\nBC {:04X}  DE {:04X}  HL {:04X}\nINTE {}  cycles {}",
            registers.pc,
            registers.sp,
            registers.a,
            flag(flags.sign, 'S'),
            flag(flags.zero, 'Z'),
            flag(flags.aux_carry, 'A'),
            flag(flags.parity, 'P'),
            flag(flags.carry, 'C'),
            registers.bc(),
            registers.de(),
            registers.hl(),
            u8::from(cpu.interrupts_enabled()),
            cpu.cycles(),
        )
    }

    /// Disassembles `count` instructions from the program counter, for the
    /// debugger.
    pub fn disassemble(&self, count: usize) -> Vec<String> {
        let mut address = self.machine.cpu().registers.pc;
        (0..count)
            .map(|_| {
                let bytes: Vec<u8> = (0..3)
                    .map(|offset| self.machine.peek(address.wrapping_add(offset)))
                    .collect();
                let instruction = Instruction::decode(&bytes);
                let line = format!("{address:04X}  {instruction}");
                address = address.wrapping_add(instruction.size());
                line
            })
            .collect()
    }
}

/// Looks a game up by its short name.
fn game(short_name: &str) -> Result<Game, JsError> {
    Game::from_short_name(short_name)
        .ok_or_else(|| JsError::new(&format!("{short_name} is not a supported game")))
}

/// Packs a colour as a little-endian RGBA pixel, the byte order of
/// `ImageData`.
fn to_rgba(color: Rgb) -> u32 {
    u32::from_le_bytes([color.red, color.green, color.blue, 0xff])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library_with_test_program() -> RomLibrary {
        // LXI H,2400H ; MVI M,0FFH ; OUT 7 ; JMP 0005H
        let mut first = vec![0x21, 0x00, 0x24, 0x36, 0xff, 0xd3, 0x07, 0xc3, 0x05, 0x00];
        first.resize(0x800, 0);
        let mut library = RomLibrary::new();
        library.add_file("lagunar.h", first).unwrap();
        for name in ["lagunar.g", "lagunar.f", "lagunar.e"] {
            library.add_file(name, vec![0; 0x800]).unwrap();
        }
        library
    }

    #[test]
    fn describes_the_supported_games() {
        let games = supported_games();
        assert_eq!(games[0].short_name, "lagunar");
        assert_eq!(games[0].chips.len(), 4);
        assert_eq!(
            games[0].dip_switches[0].values.len(),
            games[0].dip_switches[0].labels.len()
        );
    }

    #[test]
    fn identifies_and_checks_rom_files() {
        let library = library_with_test_program();
        assert_eq!(library.file_count(), 4);
        assert_eq!(library.identify().as_deref(), Some("lagunar"));
        let report = library.check("lagunar").unwrap();
        assert!(
            report
                .iter()
                .all(|chip| chip.status == "mismatch" && chip.usable)
        );
    }

    #[test]
    fn runs_frames_and_draws_rgba_pictures() {
        let mut emulator = Emulator::new(&library_with_test_program(), "lagunar", 48_000).unwrap();
        assert!(emulator.vertical());
        assert_eq!(
            (emulator.picture_width(), emulator.picture_height()),
            (224, 260)
        );

        let input = InputState {
            right: true,
            accelerator: 1.0,
            ..InputState::new()
        };
        emulator.run_frame(&input);
        assert!(emulator.steering() > 0.0);
        assert!((emulator.accelerator() - 1.0).abs() < f32::EPSILON);
        assert!(emulator.audio().len() > 800, "a frame of sound");

        emulator.draw_picture();
        let white = to_rgba(Rgb::WHITE);
        // Raster line 0 is the picture's rightmost column; its first byte
        // lights rows 4 to 11.
        assert_eq!(emulator.picture[4 * 224 + 223], white);
        assert_eq!(emulator.picture[3 * 224 + 223], to_rgba(Rgb::BLACK));
    }

    #[test]
    fn exposes_debugger_views() {
        let mut emulator = Emulator::new(&library_with_test_program(), "lagunar", 0).unwrap();
        assert_eq!(emulator.audio(), Vec::<f32>::new());
        emulator.run_frame(&InputState::new());
        assert!(emulator.registers().starts_with("PC 000"));
        let listing = emulator.disassemble(2);
        assert_eq!(listing.len(), 2);
        assert!(
            listing
                .iter()
                .any(|line| line.contains("OUT 07H") || line.contains("JMP 0005H"))
        );
    }
}
