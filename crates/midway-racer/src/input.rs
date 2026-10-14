//! Turning the player's input into cabinet controls and commands.

use minifb::{Key, KeyRepeat, Window};
use mw8080bw::{Controls, DigitalPedal, DigitalSteering, Gear};

const STEER_LEFT: &[Key] = &[Key::Left, Key::A];
const STEER_RIGHT: &[Key] = &[Key::Right, Key::D];
const ACCELERATE: &[Key] = &[Key::Up, Key::W];
const START: &[Key] = &[Key::Key1, Key::Enter];

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
}

impl Input {
    /// Handles the keys pressed since the last update: shifts gear and
    /// returns the commands issued.
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
        commands
    }

    /// Moves the controls for `seconds` of held keys and returns them.
    pub(crate) fn controls(&mut self, keyboard: &impl Keyboard, seconds: f32) -> Controls {
        let held = |keys: &[Key]| keys.iter().any(|&key| keyboard.is_down(key));
        Controls {
            steering: self
                .steering
                .update(held(STEER_LEFT), held(STEER_RIGHT), seconds),
            accelerator: self.pedal.update(held(ACCELERATE), seconds),
            gear: self.gear,
            start: held(START),
        }
    }

    /// Returns the gear the shifter is in.
    pub(crate) fn gear(&self) -> Gear {
        self.gear
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
}
