/** Deciding how many emulated frames to run on each animation frame. */

/**
 * Most frames emulated at once to catch up. Longer stalls, such as a
 * background tab, are skipped rather than fast-forwarded.
 */
export const MAX_CATCH_UP_FRAMES = 4;

/** Paces emulation by elapsed time, at the board's refresh rate. */
export class FrameClock {
  readonly #frameRate: number;
  #start: number;
  #frames = 0;

  /** Starts a clock running at `frameRate` frames per second at `now` ms. */
  constructor(frameRate: number, now: number) {
    this.#frameRate = frameRate;
    this.#start = now;
  }

  /** Starts counting again from `now`, for instance after a pause. */
  restart(now: number): void {
    this.#start = now;
    this.#frames = 0;
  }

  /** Returns how many frames to emulate to keep up at `now` ms. */
  framesDue(now: number): number {
    const due = Math.floor(((now - this.#start) / 1000) * this.#frameRate);
    this.#frames = Math.max(this.#frames, due - MAX_CATCH_UP_FRAMES);
    const behind = Math.max(0, due - this.#frames);
    this.#frames += behind;
    return behind;
  }
}
