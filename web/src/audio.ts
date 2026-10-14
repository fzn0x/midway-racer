/** Playing the emulator's sound through an audio worklet. */

import type { FlushMessage, PlayedReport } from "./audio-messages";
import processorUrl from "./audio-processor.ts?worker&url";

/** Sound kept queued ahead of playback, in seconds. */
export const AUDIO_LEAD_SECONDS = 0.08;

export class AudioOutput {
  readonly #context: AudioContext;
  readonly #node: AudioWorkletNode;
  readonly #gain: GainNode;
  #queued = 0;
  /** Counts flushes, to ignore progress reports sent before the latest. */
  #generation = 0;

  /**
   * Opens the audio output. Browsers keep it suspended until the player
   * interacts with the page; see {@link AudioOutput.resume}.
   */
  static async create(): Promise<AudioOutput> {
    const context = new AudioContext({ latencyHint: "interactive" });
    await context.audioWorklet.addModule(processorUrl);
    const node = new AudioWorkletNode(context, "midway-racer-sound", {
      numberOfInputs: 0,
      outputChannelCount: [1],
    });
    const gain = new GainNode(context);
    node.connect(gain).connect(context.destination);
    return new AudioOutput(context, node, gain);
  }

  private constructor(
    context: AudioContext,
    node: AudioWorkletNode,
    gain: GainNode,
  ) {
    this.#context = context;
    this.#node = node;
    this.#gain = gain;
    node.port.onmessage = (event: MessageEvent<PlayedReport>) => {
      if (event.data.generation === this.#generation) {
        this.#queued = Math.max(0, this.#queued - event.data.played);
      }
    };
  }

  /** The output's sample rate. */
  get sampleRate(): number {
    return this.#context.sampleRate;
  }

  /** Whether sound is playing; false until the player interacts. */
  get running(): boolean {
    return this.#context.state === "running";
  }

  /** Calls `listener` whenever playback starts or stops. */
  onRunningChange(listener: () => void): void {
    this.#context.addEventListener("statechange", listener);
  }

  /** Samples waiting to be played. */
  get queued(): number {
    return this.#queued;
  }

  /** Sets the volume, from 0 to 1, smoothly to avoid clicks. */
  set volume(volume: number) {
    this.#gain.gain.setTargetAtTime(volume, this.#context.currentTime, 0.015);
  }

  /** Starts playback. Only works during or after a user gesture. */
  async resume(): Promise<void> {
    if (this.#context.state === "suspended") {
      await this.#context.resume();
    }
  }

  /** Queues samples for playback, handing their buffer over. */
  play(samples: Float32Array): void {
    this.#queued += samples.length;
    this.#node.port.postMessage(samples, [samples.buffer]);
  }

  /** Drops the queued samples. */
  flush(): void {
    this.#queued = 0;
    this.#generation++;
    const message: FlushMessage = { flush: this.#generation };
    this.#node.port.postMessage(message);
  }
}
