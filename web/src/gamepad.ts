/** Gamepads, read through the Gamepad API's standard layout. */

/** Something a gamepad button asks for, besides driving. */
export type PadCommand = "coin" | "shift" | "low-gear" | "high-gear";

/** The driving input from every connected gamepad, combined. */
export interface PadInput {
  /** Analogue steering, -1 (left) to 1 (right). */
  steering: number;
  /** Analogue accelerator travel, 0 to 1. */
  accelerator: number;
  left: boolean;
  right: boolean;
  accelerate: boolean;
  start: boolean;
}

/** Stick travel ignored around the centre, against drift. */
const STICK_DEAD_ZONE = 0.12;
/** Trigger travel ignored at rest. */
const TRIGGER_DEAD_ZONE = 0.05;

/** Button positions in the standard gamepad layout. */
const BUTTON = {
  south: 0,
  west: 2,
  leftBumper: 4,
  rightBumper: 5,
  rightTrigger: 7,
  back: 8,
  start: 9,
  dpadLeft: 14,
  dpadRight: 15,
} as const;

const COMMAND_BUTTONS: readonly (readonly [number, PadCommand])[] = [
  [BUTTON.back, "coin"],
  [BUTTON.west, "shift"],
  [BUTTON.leftBumper, "low-gear"],
  [BUTTON.rightBumper, "high-gear"],
];

export const NEUTRAL_PAD: Readonly<PadInput> = {
  steering: 0,
  accelerator: 0,
  left: false,
  right: false,
  accelerate: false,
  start: false,
};

/** Maps stick travel to steering, ignoring the dead zone and rescaling the
 * rest so that steering still starts from zero. */
export function stick(travel: number): number {
  const magnitude = rescale(Math.abs(travel), STICK_DEAD_ZONE);
  return magnitude === 0 ? 0 : Math.sign(travel) * magnitude;
}

/** Maps trigger travel to accelerator travel. */
export function trigger(travel: number): number {
  return rescale(travel, TRIGGER_DEAD_ZONE);
}

function rescale(travel: number, deadZone: number): number {
  if (Number.isNaN(travel) || travel <= deadZone) {
    return 0;
  }
  return Math.min(1, (travel - deadZone) / (1 - deadZone));
}

/** Merges two gamepads' input, the strongest winning. */
export function combine(
  a: Readonly<PadInput>,
  b: Readonly<PadInput>,
): PadInput {
  return {
    steering:
      Math.abs(b.steering) > Math.abs(a.steering) ? b.steering : a.steering,
    accelerator: Math.max(a.accelerator, b.accelerator),
    left: a.left || b.left,
    right: a.right || b.right,
    accelerate: a.accelerate || b.accelerate,
    start: a.start || b.start,
  };
}

export class Gamepads {
  /** Buttons held at the last poll, per gamepad, to detect presses. */
  readonly #held = new Map<number, readonly boolean[]>();

  /** Reads every gamepad, reporting newly pressed command buttons. */
  poll(onCommand: (command: PadCommand) => void): PadInput {
    let input: PadInput = { ...NEUTRAL_PAD };
    for (const pad of navigator.getGamepads()) {
      if (!pad?.connected) {
        continue;
      }
      const held = pad.buttons.map((button) => button.pressed);
      const before = this.#held.get(pad.index) ?? [];
      for (const [index, command] of COMMAND_BUTTONS) {
        if (held[index] && !before[index]) {
          onCommand(command);
        }
      }
      this.#held.set(pad.index, held);

      const pressed = (index: number) => held[index] ?? false;
      input = combine(input, {
        steering: stick(pad.axes[0] ?? 0),
        accelerator: trigger(pad.buttons[BUTTON.rightTrigger]?.value ?? 0),
        left: pressed(BUTTON.dpadLeft),
        right: pressed(BUTTON.dpadRight),
        accelerate: pressed(BUTTON.south),
        start: pressed(BUTTON.start),
      });
    }
    return input;
  }
}
