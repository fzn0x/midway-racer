/**
 * The audio worklet that plays the emulator's sound on the audio thread.
 *
 * The page posts each frame's samples; the worklet plays them in order and
 * reports how many it played, so the page can keep the queue topped up.
 */

import type { PlayedReport, ToWorklet } from "./audio-messages";

// The AudioWorkletGlobalScope, which TypeScript's DOM library lacks.
declare abstract class AudioWorkletProcessor {
  readonly port: MessagePort;
  abstract process(
    inputs: Float32Array[][],
    outputs: Float32Array[][],
  ): boolean;
}
declare function registerProcessor(
  name: string,
  processor: new () => AudioWorkletProcessor,
): void;

/** Render quanta, of 128 samples each, between two progress reports. */
const REPORT_INTERVAL = 8;
/** How much of the previous sample each one keeps when the queue runs dry,
 * so an underrun fades out over a few milliseconds instead of clicking. */
const UNDERRUN_FADE = 0.995;

class SoundProcessor extends AudioWorkletProcessor {
  readonly #queue: Float32Array[] = [];
  #chunk: Float32Array | undefined;
  #position = 0;
  #last = 0;
  #played = 0;
  #quanta = 0;
  #generation = 0;

  constructor() {
    super();
    this.port.onmessage = (event: MessageEvent<ToWorklet>) => {
      const message = event.data;
      if (message instanceof Float32Array) {
        this.#queue.push(message);
      } else {
        this.#queue.length = 0;
        this.#chunk = undefined;
        this.#played = 0;
        this.#quanta = 0;
        this.#generation = message.flush;
      }
    };
  }

  override process(
    _inputs: Float32Array[][],
    outputs: Float32Array[][],
  ): boolean {
    const channels = outputs[0] ?? [];
    const [first, ...rest] = channels;
    if (first) {
      for (let index = 0; index < first.length; index++) {
        first[index] = this.#next();
      }
      for (const channel of rest) {
        channel.set(first);
      }
    }
    if (++this.#quanta === REPORT_INTERVAL) {
      const report: PlayedReport = {
        generation: this.#generation,
        played: this.#played,
      };
      this.port.postMessage(report);
      this.#played = 0;
      this.#quanta = 0;
    }
    return true;
  }

  #next(): number {
    while (!this.#chunk || this.#position === this.#chunk.length) {
      this.#chunk = this.#queue.shift();
      this.#position = 0;
      if (!this.#chunk) {
        this.#last *= UNDERRUN_FADE;
        return this.#last;
      }
    }
    this.#last = this.#chunk[this.#position++] ?? 0;
    this.#played++;
    return this.#last;
  }
}

registerProcessor("midway-racer-sound", SoundProcessor);
