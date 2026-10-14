/** Every input device, merged into the emulator's input. */

import {
  Gamepads,
  NEUTRAL_PAD,
  type PadCommand,
  type PadInput,
} from "./gamepad";
import {
  ACCELERATE,
  type Keyboard,
  START,
  STEER_LEFT,
  STEER_RIGHT,
} from "./keyboard";
import type { TouchControls } from "./touch";
import type { InputState } from "./wasm/midway_racer_web.js";

export class Controls {
  readonly #keyboard: Keyboard;
  readonly #touch: TouchControls;
  readonly #gamepads = new Gamepads();
  #pad: Readonly<PadInput> = NEUTRAL_PAD;

  constructor(keyboard: Keyboard, touch: TouchControls) {
    this.#keyboard = keyboard;
    this.#touch = touch;
  }

  /** Reads the gamepads, which have no events, once per animation frame. */
  poll(onCommand: (command: PadCommand) => void): void {
    this.#pad = this.#gamepads.poll(onCommand);
  }

  /** Writes what the player is doing into `input`. */
  apply(input: InputState): void {
    const keyboard = this.#keyboard;
    const pad = this.#pad;
    const touch = this.#touch;
    input.left = keyboard.isHeld(STEER_LEFT) || pad.left;
    input.right = keyboard.isHeld(STEER_RIGHT) || pad.right;
    input.accelerate =
      keyboard.isHeld(ACCELERATE) || pad.accelerate || touch.accelerate;
    input.steering = Math.max(-1, Math.min(1, pad.steering + touch.steering));
    input.accelerator = pad.accelerator;
    input.start = keyboard.isHeld(START) || pad.start;
  }

  /** Shows the gear on the touch controls. */
  showGear(high: boolean): void {
    this.#touch.highGear = high;
  }
}
