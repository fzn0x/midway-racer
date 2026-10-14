/** A game being played: the emulation loop and its inputs and outputs. */

import { AUDIO_LEAD_SECONDS, type AudioOutput } from "./audio";
import type { Controls } from "./controls";
import type { CpuView } from "./debugger";
import type { PadCommand } from "./gamepad";
import { audioFramesDue, FrameClock } from "./pacing";
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
  readonly #controls: Controls;
  readonly #panel: Panel;
  readonly #cpuView: CpuView;
  readonly #audio: AudioOutput | undefined;
  readonly #input = new InputState();
  /** The board's refresh rate; read once the WebAssembly module is ready. */
  readonly #frameRate = frameRate();
  readonly #clock = new FrameClock(this.#frameRate, performance.now());
  /** Whether the sound card paces emulation, rather than the clock. */
  #audible = false;
  #highGear = false;
  #paused = false;
  #animationFrame = 0;

  constructor(
    emulator: Emulator,
    memory: WebAssembly.Memory,
    canvas: HTMLCanvasElement,
    controls: Controls,
    panel: Panel,
    cpuView: CpuView,
    audio: AudioOutput | undefined,
  ) {
    this.emulator = emulator;
    this.#memory = memory;
    this.#renderer = new Renderer(
      canvas,
      emulator.pictureWidth,
      emulator.pictureHeight,
    );
    this.#controls = controls;
    this.#panel = panel;
    this.#cpuView = cpuView;
    this.#audio = audio;
  }

  /** Starts emulating. */
  start(): void {
    this.#clock.restart(performance.now());
    this.#animationFrame = requestAnimationFrame(this.#tick);
    this.#showGear();
  }

  /** Stops emulating and frees the emulator. */
  stop(): void {
    cancelAnimationFrame(this.#animationFrame);
    this.#audio?.flush();
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
    this.#cpuView.show(this.emulator, false);
  }

  /** Runs a single frame while paused, for the debugger. */
  stepFrame(): void {
    if (!this.#paused) {
      return;
    }
    this.#runFrame();
    this.#renderer.draw(this.#memory, this.emulator.drawPicture());
    this.#showPanel();
    this.#cpuView.show(this.emulator, false);
  }

  insertCoin(): void {
    this.emulator.insertCoin();
  }

  reset(): void {
    this.emulator.reset();
  }

  /** Moves the gear shifter to the other gear. */
  shift(): void {
    this.highGear = !this.#highGear;
  }

  get highGear(): boolean {
    return this.#highGear;
  }

  /** Moves the gear shifter to high gear, or to low gear. */
  set highGear(high: boolean) {
    this.#highGear = high;
    this.#showGear();
  }

  readonly #tick = (now: number): void => {
    this.#animationFrame = requestAnimationFrame(this.#tick);
    this.#controls.poll((command) => {
      this.#padCommand(command);
    });
    if (this.#paused) {
      return;
    }

    // While sound plays, keep its queue topped up so the game runs at the
    // sound card's pace; otherwise follow the clock.
    const audio = this.#audio?.running ? this.#audio : undefined;
    if ((audio !== undefined) !== this.#audible) {
      this.#audible = audio !== undefined;
      this.#clock.restart(now);
      this.#audio?.flush();
    }
    const frames = audio
      ? audioFramesDue(
          audio.queued,
          audio.sampleRate,
          this.#frameRate,
          AUDIO_LEAD_SECONDS,
        )
      : this.#clock.framesDue(now);

    for (let frame = 0; frame < frames; frame++) {
      this.#runFrame();
      audio?.play(this.emulator.audio());
    }
    if (frames > 0) {
      this.#renderer.draw(this.#memory, this.emulator.drawPicture());
      this.#showPanel();
      this.#cpuView.show(this.emulator, true);
    }
  };

  #runFrame(): void {
    this.#controls.apply(this.#input);
    this.#input.highGear = this.#highGear;
    this.emulator.runFrame(this.#input);
  }

  #padCommand(command: PadCommand): void {
    switch (command) {
      case "coin":
        this.insertCoin();
        break;
      case "shift":
        this.shift();
        break;
      case "low-gear":
        this.highGear = false;
        break;
      case "high-gear":
        this.highGear = true;
        break;
    }
  }

  #showGear(): void {
    this.#controls.showGear(this.#highGear);
    this.#showPanel();
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
