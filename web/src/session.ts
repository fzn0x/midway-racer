/** A game being played: the emulation loop and its inputs and outputs. */

import {
  ACCELERATE,
  type Keyboard,
  START,
  STEER_LEFT,
  STEER_RIGHT,
} from "./keyboard";
import { FrameClock } from "./pacing";
import type { Panel } from "./panel";
import { Renderer } from "./renderer";
import {
  type Emulator,
  InputState,
  frameRate,
} from "./wasm/midway_racer_web.js";

export class Session {
  readonly emulator: Emulator;
  readonly #memory: WebAssembly.Memory;
  readonly #renderer: Renderer;
  readonly #keyboard: Keyboard;
  readonly #panel: Panel;
  readonly #input = new InputState();
  readonly #clock = new FrameClock(frameRate(), performance.now());
  #highGear = false;
  #paused = false;
  #animationFrame = 0;

  constructor(
    emulator: Emulator,
    memory: WebAssembly.Memory,
    canvas: HTMLCanvasElement,
    keyboard: Keyboard,
    panel: Panel,
  ) {
    this.emulator = emulator;
    this.#memory = memory;
    this.#renderer = new Renderer(
      canvas,
      emulator.pictureWidth,
      emulator.pictureHeight,
    );
    this.#keyboard = keyboard;
    this.#panel = panel;
  }

  /** Starts emulating. */
  start(): void {
    this.#clock.restart(performance.now());
    this.#animationFrame = requestAnimationFrame(this.#tick);
    this.#showPanel();
  }

  /** Stops emulating and frees the emulator. */
  stop(): void {
    cancelAnimationFrame(this.#animationFrame);
    this.#input.free();
    this.emulator.free();
  }

  get paused(): boolean {
    return this.#paused;
  }

  set paused(paused: boolean) {
    this.#paused = paused;
    this.#clock.restart(performance.now());
    this.#showPanel();
  }

  insertCoin(): void {
    this.emulator.insertCoin();
  }

  reset(): void {
    this.emulator.reset();
  }

  /** Moves the gear shifter to the other gear. */
  shift(): void {
    this.#highGear = !this.#highGear;
    this.#showPanel();
  }

  readonly #tick = (now: number): void => {
    this.#animationFrame = requestAnimationFrame(this.#tick);
    if (this.#paused) {
      return;
    }
    const frames = this.#clock.framesDue(now);
    for (let frame = 0; frame < frames; frame++) {
      this.#readInput();
      this.emulator.runFrame(this.#input);
    }
    if (frames > 0) {
      this.#renderer.draw(this.#memory, this.emulator.drawPicture());
      this.#showPanel();
    }
  };

  #readInput(): void {
    const keyboard = this.#keyboard;
    const input = this.#input;
    input.left = keyboard.isHeld(STEER_LEFT);
    input.right = keyboard.isHeld(STEER_RIGHT);
    input.accelerate = keyboard.isHeld(ACCELERATE);
    input.start = keyboard.isHeld(START);
    input.highGear = this.#highGear;
  }

  #showPanel(): void {
    this.#panel.show({
      steering: this.emulator.steering,
      accelerator: this.emulator.accelerator,
      highGear: this.#highGear,
      paused: this.#paused,
    });
  }
}
