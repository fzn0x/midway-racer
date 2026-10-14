/** A view of the emulated CPU: registers and disassembly at the PC. */

import { byId, element } from "./dom";
import type { Emulator } from "./wasm/midway_racer_web.js";

/** Instructions listed from the program counter. */
const LISTING_LENGTH = 8;
/** Animation frames between refreshes while the game runs, so about ten
 * a second. */
const REFRESH_INTERVAL = 6;

export class CpuView {
  readonly #details = byId("debugger", HTMLDetailsElement);
  readonly #registers = byId("cpu-registers");
  readonly #listing = byId("cpu-disassembly");
  #frames = 0;

  constructor(stepFrame: () => void) {
    byId("step-frame", HTMLButtonElement).addEventListener("click", stepFrame);
  }

  /** Opens or closes the view. */
  toggle(): void {
    this.#details.open = !this.#details.open;
  }

  /** Shows the CPU of `emulator`; while it runs, only every few frames. */
  show(emulator: Emulator, running: boolean): void {
    if (!this.#details.open) {
      return;
    }
    if (running && ++this.#frames % REFRESH_INTERVAL !== 0) {
      return;
    }
    this.#registers.textContent = emulator.registers();
    this.#listing.replaceChildren(
      ...emulator
        .disassemble(LISTING_LENGTH)
        .map((line) => element("li", { text: line })),
    );
  }
}
