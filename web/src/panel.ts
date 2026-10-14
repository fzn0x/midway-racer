/** The control panel beside the screen: shifter, gauges and buttons. */

import { byId } from "./dom";

/** What the panel's controls do. */
export interface PanelActions {
  insertCoin(): void;
  togglePause(): void;
  reset(): void;
  shift(): void;
  changeGame(): void;
  toggleMute(): void;
  /** The volume slider moved, to a volume from 0 to 1. */
  setVolume(volume: number): void;
}

/** What the panel shows about the game. */
export interface PanelState {
  steering: number;
  accelerator: number;
  highGear: boolean;
  paused: boolean;
}

/** What the panel shows about the sound. */
export interface SoundState {
  /** Volume from 0 to 1. */
  volume: number;
  muted: boolean;
  /** Whether the browser can play sound at all. */
  available: boolean;
  /** Whether sound is playing, which needs a user gesture first. */
  running: boolean;
}

export class Panel {
  readonly #shifter = byId("shifter", HTMLButtonElement);
  readonly #wheel = byId("wheel-gauge");
  readonly #pedal = byId("pedal-gauge");
  readonly #pause = byId("pause", HTMLButtonElement);
  readonly #mute = byId("mute", HTMLButtonElement);
  readonly #volume = byId("volume", HTMLInputElement);
  readonly #soundHint = byId("sound-hint");
  #shown: PanelState | undefined;

  constructor(actions: PanelActions) {
    const bind = (id: string, action: () => void) => {
      byId(id, HTMLButtonElement).addEventListener("click", action);
    };
    bind("insert-coin", () => {
      actions.insertCoin();
    });
    bind("pause", () => {
      actions.togglePause();
    });
    bind("reset", () => {
      actions.reset();
    });
    bind("shifter", () => {
      actions.shift();
    });
    bind("change-game", () => {
      actions.changeGame();
    });
    bind("mute", () => {
      actions.toggleMute();
    });
    this.#volume.addEventListener("input", () => {
      actions.setVolume(this.#volume.valueAsNumber / 100);
    });
  }

  /** Updates the panel, touching the page only where something changed. */
  show(state: PanelState): void {
    const shown = this.#shown;
    if (shown?.highGear !== state.highGear) {
      this.#shifter.setAttribute("aria-pressed", String(state.highGear));
    }
    if (shown?.paused !== state.paused) {
      this.#pause.textContent = state.paused ? "Resume" : "Pause";
      this.#pause.setAttribute("aria-pressed", String(state.paused));
    }
    if (shown?.steering !== state.steering) {
      setGauge(this.#wheel, state.steering);
    }
    if (shown?.accelerator !== state.accelerator) {
      setGauge(this.#pedal, state.accelerator);
    }
    this.#shown = { ...state };
  }

  /** Shows the volume and whether sound is on. */
  showSound(state: SoundState): void {
    this.#volume.value = String(Math.round(state.volume * 100));
    this.#volume.disabled = !state.available;
    this.#mute.disabled = !state.available;
    this.#mute.setAttribute("aria-pressed", String(state.muted));
    this.#mute.textContent = state.muted ? "Unmute" : "Mute";

    if (!state.available) {
      this.#soundHint.textContent = "Sound is not available in this browser.";
    } else if (!state.running) {
      this.#soundHint.textContent =
        "Click or press a key to turn the sound on.";
    } else {
      this.#soundHint.textContent = "";
    }
    this.#soundHint.hidden = this.#soundHint.textContent === "";
  }
}

/** Fills a gauge to `value`, from -1 or 0 to 1. */
function setGauge(gauge: HTMLElement, value: number): void {
  gauge.style.setProperty("--value", value.toFixed(3));
  gauge.setAttribute("aria-valuenow", value.toFixed(2));
}
