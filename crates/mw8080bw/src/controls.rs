//! The cabinet's controls: steering wheel, accelerator pedal and gear
//! shifter, plus helpers that drive them from digital inputs such as keys.

/// Wheel converter reading when the wheel is centred.
const WHEEL_CENTRE: u8 = 0x7f;
/// Wheel converter reading at full right lock.
const WHEEL_RIGHT: u8 = 0x01;
/// Wheel converter reading at full left lock.
const WHEEL_LEFT: u8 = 0xfe;
/// Highest reading of the 4-bit accelerator encoder.
const PEDAL_FLOORED: u8 = 0x0f;

/// The position of the two-speed gear shifter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Gear {
    /// Low gear, for accelerating out of curves.
    #[default]
    Low,
    /// High gear, for top speed.
    High,
}

impl Gear {
    /// Returns the other gear.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Low => Self::High,
            Self::High => Self::Low,
        }
    }
}

/// The positions of the cabinet's controls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    /// Steering wheel position, from -1.0 (full left) through 0.0
    /// (centred) to 1.0 (full right).
    pub steering: f32,
    /// Accelerator pedal travel, from 0.0 (released) to 1.0 (floored).
    pub accelerator: f32,
    /// The gear shifter.
    pub gear: Gear,
    /// Whether the start button is held. It is wired, but the games
    /// start as soon as a coin is inserted.
    pub start: bool,
}

impl Controls {
    /// Returns the 4-bit reading of the accelerator's position encoder.
    #[expect(clippy::cast_sign_loss, reason = "the travel is clamped to 0..=1")]
    pub fn pedal_value(&self) -> u8 {
        let travel = if self.accelerator.is_nan() {
            0.0
        } else {
            self.accelerator.clamp(0.0, 1.0)
        };
        (travel * f32::from(PEDAL_FLOORED)).round() as u8
    }

    /// Returns the reading of the steering wheel's potentiometer
    /// converter: `7F` when centred, falling to `01` at full right lock
    /// and rising to `FE` at full left lock.
    #[expect(clippy::cast_sign_loss, reason = "the reading is clamped to 1..=254")]
    pub fn wheel_value(&self) -> u8 {
        let steering = if self.steering.is_nan() {
            0.0
        } else {
            self.steering.clamp(-1.0, 1.0)
        };
        let travel = if steering >= 0.0 {
            steering * f32::from(WHEEL_CENTRE - WHEEL_RIGHT)
        } else {
            steering * f32::from(WHEEL_LEFT - WHEEL_CENTRE)
        };
        (f32::from(WHEEL_CENTRE) - travel).round() as u8
    }
}

/// Turns digital left and right inputs into a steering wheel that turns
/// while a direction is held and recentres when it is released.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DigitalSteering {
    position: f32,
}

impl DigitalSteering {
    /// Wheel travel per second while turning: full lock in 0.4 s.
    const TURN_RATE: f32 = 2.5;
    /// Wheel travel per second while recentring or reversing.
    const RETURN_RATE: f32 = 5.0;

    /// Creates a centred wheel.
    pub const fn new() -> Self {
        Self { position: 0.0 }
    }

    /// Moves the wheel for `seconds` of held inputs and returns its new
    /// position, in the range of [`Controls::steering`].
    pub fn update(&mut self, left: bool, right: bool, seconds: f32) -> f32 {
        let (target, rate) = match (left, right) {
            (true, false) if self.position > 0.0 => (-1.0, Self::RETURN_RATE),
            (false, true) if self.position < 0.0 => (1.0, Self::RETURN_RATE),
            (true, false) => (-1.0, Self::TURN_RATE),
            (false, true) => (1.0, Self::TURN_RATE),
            _ => (0.0, Self::RETURN_RATE),
        };
        self.position = approach(self.position, target, rate * seconds);
        self.position
    }

    /// Returns the wheel position.
    pub const fn position(&self) -> f32 {
        self.position
    }
}

/// Turns a digital accelerator input into progressive pedal travel.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DigitalPedal {
    travel: f32,
}

impl DigitalPedal {
    /// Pedal travel per second while pressed: floored in 0.25 s.
    const PRESS_RATE: f32 = 4.0;
    /// Pedal travel per second while released.
    const RELEASE_RATE: f32 = 8.0;

    /// Creates a released pedal.
    pub const fn new() -> Self {
        Self { travel: 0.0 }
    }

    /// Moves the pedal for `seconds` of input and returns its new travel,
    /// in the range of [`Controls::accelerator`].
    pub fn update(&mut self, pressed: bool, seconds: f32) -> f32 {
        self.travel = if pressed {
            approach(self.travel, 1.0, Self::PRESS_RATE * seconds)
        } else {
            approach(self.travel, 0.0, Self::RELEASE_RATE * seconds)
        };
        self.travel
    }

    /// Returns the pedal travel.
    pub const fn travel(&self) -> f32 {
        self.travel
    }
}

/// Moves `current` towards `target` by at most `step`.
fn approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steering(steering: f32) -> u8 {
        Controls {
            steering,
            ..Controls::default()
        }
        .wheel_value()
    }

    fn pedal(accelerator: f32) -> u8 {
        Controls {
            accelerator,
            ..Controls::default()
        }
        .pedal_value()
    }

    #[test]
    fn wheel_reading_falls_to_the_right() {
        assert_eq!(steering(0.0), 0x7f);
        assert_eq!(steering(1.0), 0x01);
        assert_eq!(steering(-1.0), 0xfe);
        assert_eq!(steering(0.5), 0x40);
        assert_eq!(steering(-0.5), 0xbf);
    }

    #[test]
    fn wheel_reading_clamps_out_of_range_input() {
        assert_eq!(steering(7.0), 0x01);
        assert_eq!(steering(-7.0), 0xfe);
        assert_eq!(steering(f32::NAN), 0x7f);
    }

    #[test]
    fn pedal_reading_has_sixteen_steps() {
        assert_eq!(pedal(0.0), 0);
        assert_eq!(pedal(0.5), 8);
        assert_eq!(pedal(1.0), 15);
        assert_eq!(pedal(2.0), 15);
        assert_eq!(pedal(-1.0), 0);
        assert_eq!(pedal(f32::NAN), 0);
    }

    #[test]
    fn gears_toggle() {
        assert_eq!(Gear::default(), Gear::Low);
        assert_eq!(Gear::Low.toggled(), Gear::High);
        assert_eq!(Gear::High.toggled(), Gear::Low);
    }

    #[test]
    fn digital_steering_turns_then_recentres() {
        let mut wheel = DigitalSteering::new();
        assert!((wheel.update(false, true, 0.2) - 0.5).abs() < 1e-6);
        assert!((wheel.update(false, true, 0.2) - 1.0).abs() < 1e-6);
        assert!(
            (wheel.update(false, true, 1.0) - 1.0).abs() < 1e-6,
            "stops at full lock"
        );

        assert!((wheel.update(false, false, 0.1) - 0.5).abs() < 1e-6);
        assert!(wheel.update(false, false, 1.0).abs() < 1e-6);
        assert!(
            wheel.update(true, true, 1.0).abs() < 1e-6,
            "opposite inputs cancel"
        );
    }

    #[test]
    fn digital_steering_reverses_quickly() {
        let mut wheel = DigitalSteering::new();
        wheel.update(true, false, 1.0);
        assert!((wheel.position() + 1.0).abs() < 1e-6);
        wheel.update(false, true, 0.1);
        assert!((wheel.position() + 0.5).abs() < 1e-6);
    }

    #[test]
    fn digital_pedal_presses_progressively_and_releases_faster() {
        let mut pedal = DigitalPedal::new();
        assert!((pedal.update(true, 0.125) - 0.5).abs() < 1e-6);
        assert!((pedal.update(true, 1.0) - 1.0).abs() < 1e-6);
        assert!((pedal.update(false, 0.0625) - 0.5).abs() < 1e-6);
        assert!(pedal.update(false, 1.0).abs() < 1e-6);
        assert!(pedal.travel().abs() < 1e-6);
    }
}
