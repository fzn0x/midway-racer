/** Messages between the page and the sound worklet. */

/**
 * Drops every queued sample. Each flush starts a new generation, so the
 * page can tell reports sent before the flush from those sent after it.
 */
export interface FlushMessage {
  readonly flush: number;
}

/** What the page sends: samples to play, or a flush. */
export type ToWorklet = Float32Array | FlushMessage;

/** What the worklet sends: samples played since its previous report. */
export interface PlayedReport {
  readonly generation: number;
  readonly played: number;
}
