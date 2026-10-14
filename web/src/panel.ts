/** The control panel beside the screen: shifter, gauges and buttons. */

import { byId } from "./dom";

/** What the panel's buttons do. */
export interface PanelActions {
  insertCoin(): void;
  togglePause(): void;
  reset(): void;
  shift(): void;
  changeGame(): void;
}

/** What the panel shows. */
export interface PanelState {
  steering: number;
  accelerator: number;
  highGear: boolean;
  paused: boolean;
}

export class Panel {
  readonly #shifter = byId("shifter", HTMLButtonElement);
  readonly #wheel = byId("wheel-gauge");
  readonly #pedal = byId("pedal-gauge");
  readonly #pause = byId("pause", HTMLButtonElement);
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
}

/** Fills a gauge to `value`, from -1 or 0 to 1. */
function setGauge(gauge: HTMLElement, value: number): void {
  gauge.style.setProperty("--value", value.toFixed(3));
  gauge.setAttribute("aria-valuenow", value.toFixed(2));
}
