//! Turning the player's input into cabinet controls and commands.

use gilrs::ev::state::ButtonData;
use gilrs::{Axis, Button, EventType, Gamepad, Gilrs};
use minifb::{Key, KeyRepeat, Window};
use mw8080bw::{Controls, DigitalPedal, DigitalSteering, Gear};

const STEER_LEFT: &[Key] = &[Key::Left, Key::A];
const STEER_RIGHT: &[Key] = &[Key::Right, Key::D];
const ACCELERATE: &[Key] = &[Key::Up, Key::W];
const START: &[Key] = &[Key::Key1, Key::Enter];

/// Stick travel ignored around the centre, against drift.
const STICK_DEAD_ZONE: f32 = 0.12;
/// Trigger travel ignored at rest.
const TRIGGER_DEAD_ZONE: f32 = 0.05;

/// Something the player asks for besides driving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    InsertCoin,
    TogglePause,
    Reset,
    ToggleMute,
    VolumeDown,
    VolumeUp,
    Quit,
}

/// The keyboard state the input mapping reads.
pub(crate) trait Keyboard {
    /// Returns whether `key` is held down.
    fn is_down(&self, key: Key) -> bool;
    /// Returns the keys pressed since the last update, without repeats.
    fn pressed(&self) -> Vec<Key>;
}

impl Keyboard for Window {
    fn is_down(&self, key: Key) -> bool {
        self.is_key_down(key)
    }

    fn pressed(&self) -> Vec<Key> {
        self.get_keys_pressed(KeyRepeat::No)
    }
}

/// The player's controls, carried from frame to frame.
#[derive(Debug, Default)]
pub(crate) struct Input {
    steering: DigitalSteering,
    pedal: DigitalPedal,
    gear: Gear,
    gamepads: Option<Gilrs>,
}

impl Input {
    /// Creates the input mapping, with gamepad support when the platform
    /// provides it.
    pub(crate) fn new() -> Self {
        let gamepads = Gilrs::new()
            .inspect_err(|error| eprintln!("warning: gamepads are unavailable: {error}"))
            .ok();
        Self {
            gamepads,
            ..Self::default()
        }
    }

    /// Handles the keys and buttons pressed since the last update: shifts
    /// gear and returns the commands issued.
    pub(crate) fn commands(&mut self, keyboard: &impl Keyboard) -> Vec<Command> {
        let mut commands = Vec::new();
        for key in keyboard.pressed() {
            let command = match key {
                Key::Space | Key::LeftShift | Key::RightShift => {
                    self.gear = self.gear.toggled();
                    continue;
                }
                Key::C | Key::Key5 => Command::InsertCoin,
                Key::P | Key::Pause => Command::TogglePause,
                Key::F3 => Command::Reset,
                Key::M => Command::ToggleMute,
                Key::Minus | Key::NumPadMinus => Command::VolumeDown,
                Key::Equal | Key::NumPadPlus => Command::VolumeUp,
                Key::Escape => Command::Quit,
                _ => continue,
            };
            commands.push(command);
        }

        while let Some(event) = self.gamepads.as_mut().and_then(Gilrs::next_event) {
            if let EventType::ButtonPressed(button, _) = event.event {
                match button {
                    Button::West => self.gear = self.gear.toggled(),
                    Button::LeftTrigger => self.gear = Gear::Low,
                    Button::RightTrigger => self.gear = Gear::High,
                    Button::Select => commands.push(Command::InsertCoin),
                    _ => {}
                }
            }
        }
        commands
    }

    /// Moves the controls for `seconds` of held keys and buttons and
    /// returns them.
    pub(crate) fn controls(&mut self, keyboard: &impl Keyboard, seconds: f32) -> Controls {
        let held = |keys: &[Key]| keys.iter().any(|&key| keyboard.is_down(key));
        let pad = self.gamepad_state();

        let steering = self.steering.update(
            held(STEER_LEFT) || pad.digital_left,
            held(STEER_RIGHT) || pad.digital_right,
            seconds,
        );
        let accelerator = self
            .pedal
            .update(held(ACCELERATE) || pad.digital_accelerate, seconds);

        Controls {
            steering: (steering + stick(pad.stick_x)).clamp(-1.0, 1.0),
            accelerator: accelerator.max(trigger(pad.trigger)),
            gear: self.gear,
            start: held(START) || pad.start,
        }
    }

    /// Returns the gear the shifter is in.
    pub(crate) fn gear(&self) -> Gear {
        self.gear
    }

    /// Reads every connected gamepad, combined.
    fn gamepad_state(&self) -> PadState {
        self.gamepads
            .iter()
            .flat_map(Gilrs::gamepads)
            .fold(PadState::default(), |state, (_, gamepad)| {
                state.combine(PadState::read(&gamepad))
            })
    }
}

/// The parts of a gamepad's state that drive the cabinet.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[expect(clippy::struct_excessive_bools, reason = "independent buttons")]
struct PadState {
    stick_x: f32,
    trigger: f32,
    digital_left: bool,
    digital_right: bool,
    digital_accelerate: bool,
    start: bool,
}

impl PadState {
    fn read(gamepad: &Gamepad<'_>) -> Self {
        Self {
            stick_x: gamepad.value(Axis::LeftStickX),
            trigger: gamepad
                .button_data(Button::RightTrigger2)
                .map_or(0.0, ButtonData::value),
            digital_left: gamepad.is_pressed(Button::DPadLeft),
            digital_right: gamepad.is_pressed(Button::DPadRight),
            digital_accelerate: gamepad.is_pressed(Button::South),
            start: gamepad.is_pressed(Button::Start),
        }
    }

    /// Merges two gamepads' states, the strongest input winning.
    fn combine(self, other: Self) -> Self {
        Self {
            stick_x: if other.stick_x.abs() > self.stick_x.abs() {
                other.stick_x
            } else {
                self.stick_x
            },
            trigger: self.trigger.max(other.trigger),
            digital_left: self.digital_left || other.digital_left,
            digital_right: self.digital_right || other.digital_right,
            digital_accelerate: self.digital_accelerate || other.digital_accelerate,
            start: self.start || other.start,
        }
    }
}

/// Maps stick travel to steering, ignoring the dead zone and rescaling the
/// rest so that steering still starts from zero.
fn stick(travel: f32) -> f32 {
    rescale(travel.abs(), STICK_DEAD_ZONE).copysign(travel)
}

/// Maps trigger travel to accelerator travel.
fn trigger(travel: f32) -> f32 {
    rescale(travel, TRIGGER_DEAD_ZONE)
}

fn rescale(travel: f32, dead_zone: f32) -> f32 {
    if travel.is_nan() || travel <= dead_zone {
        0.0
    } else {
        ((travel - dead_zone) / (1.0 - dead_zone)).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeKeyboard {
        down: Vec<Key>,
        pressed: Vec<Key>,
    }

    impl Keyboard for FakeKeyboard {
        fn is_down(&self, key: Key) -> bool {
            self.down.contains(&key)
        }

        fn pressed(&self) -> Vec<Key> {
            self.pressed.clone()
        }
    }

    #[test]
    fn keys_issue_commands_and_shift_gear() {
        let mut input = Input::default();
        let keyboard = FakeKeyboard {
            pressed: vec![Key::C, Key::Space, Key::P, Key::Escape, Key::Q],
            ..FakeKeyboard::default()
        };
        assert_eq!(
            input.commands(&keyboard),
            [Command::InsertCoin, Command::TogglePause, Command::Quit]
        );
        assert_eq!(input.gear(), Gear::High);
    }

    #[test]
    fn held_keys_steer_and_accelerate_progressively() {
        let mut input = Input::default();
        let keyboard = FakeKeyboard {
            down: vec![Key::Right, Key::Up, Key::Enter],
            ..FakeKeyboard::default()
        };
        let controls = input.controls(&keyboard, 0.1);
        assert!(controls.steering > 0.0 && controls.steering < 1.0);
        assert!(controls.accelerator > 0.0 && controls.accelerator < 1.0);
        assert!(controls.start);

        let controls = input.controls(&keyboard, 1.0);
        assert!((controls.steering - 1.0).abs() < f32::EPSILON);
        assert!((controls.accelerator - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stick_ignores_its_dead_zone_and_keeps_full_travel() {
        assert!(stick(0.1).abs() < f32::EPSILON);
        assert!(stick(-0.1).abs() < f32::EPSILON);
        assert!((stick(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((stick(-1.0) + 1.0).abs() < f32::EPSILON);
        assert!((stick(0.56) - 0.5).abs() < 1e-6);
        assert!(stick(f32::NAN).abs() < f32::EPSILON);
    }

    #[test]
    fn trigger_ignores_slack_at_rest() {
        assert!(trigger(0.04).abs() < f32::EPSILON);
        assert!((trigger(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((trigger(0.525) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_strongest_gamepad_input_wins() {
        let left = PadState {
            stick_x: -0.8,
            trigger: 0.2,
            ..PadState::default()
        };
        let right = PadState {
            stick_x: 0.3,
            trigger: 0.9,
            start: true,
            ..PadState::default()
        };
        let combined = left.combine(right);
        assert!((combined.stick_x + 0.8).abs() < f32::EPSILON);
        assert!((combined.trigger - 0.9).abs() < f32::EPSILON);
        assert!(combined.start);
    }
}
