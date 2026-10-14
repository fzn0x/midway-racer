/** On-screen controls for touch screens. */

import { byId } from "./dom";

/** Something a touch button asks for, besides driving. */
export type TouchCommand = "coin" | "shift";

export class TouchControls {
  /** Steering from the wheel strip, -1 (left) to 1 (right). */
  steering = 0;
  /** Whether the pedal is held. */
  accelerate = false;

  readonly #root = byId("touch-controls");
  readonly #wheel = byId("touch-wheel");

  constructor(onCommand: (command: TouchCommand) => void) {
    this.#bindWheel();
    this.#bindPedal(byId("touch-gas"));
    byId("touch-coin", HTMLButtonElement).addEventListener("click", () => {
      onCommand("coin");
    });
    byId("touch-gear", HTMLButtonElement).addEventListener("click", () => {
      onCommand("shift");
    });

    // Show the controls on touch screens, and as soon as anyone touches.
    const coarse = matchMedia("(pointer: coarse)");
    this.#root.hidden = !coarse.matches;
    window.addEventListener(
      "pointerdown",
      (event) => {
        if (event.pointerType === "touch") {
          this.#root.hidden = false;
        }
      },
      { passive: true },
    );
  }

  /** Shows the gear on the gear button. */
  set highGear(high: boolean) {
    const gear = byId("touch-gear");
    gear.setAttribute("aria-pressed", String(high));
    gear.textContent = high ? "High" : "Low";
  }

  /** The strip steers like a wheel: where the finger is, the wheel turns. */
  #bindWheel(): void {
    const wheel = this.#wheel;
    const steer = (event: PointerEvent) => {
      const bounds = wheel.getBoundingClientRect();
      const position = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      this.#setSteering(Math.max(-1, Math.min(1, position)));
    };
    wheel.addEventListener("pointerdown", (event) => {
      wheel.setPointerCapture(event.pointerId);
      steer(event);
    });
    wheel.addEventListener("pointermove", (event) => {
      if (wheel.hasPointerCapture(event.pointerId)) {
        steer(event);
      }
    });
    const release = () => {
      this.#setSteering(0);
    };
    wheel.addEventListener("pointerup", release);
    wheel.addEventListener("pointercancel", release);
  }

  #bindPedal(pedal: HTMLElement): void {
    const press = (pressed: boolean) => {
      this.accelerate = pressed;
      pedal.setAttribute("aria-pressed", String(pressed));
    };
    pedal.addEventListener("pointerdown", (event) => {
      pedal.setPointerCapture(event.pointerId);
      press(true);
    });
    pedal.addEventListener("pointerup", () => {
      press(false);
    });
    pedal.addEventListener("pointercancel", () => {
      press(false);
    });
  }

  #setSteering(steering: number): void {
    this.steering = steering;
    this.#wheel.style.setProperty("--steering", steering.toFixed(3));
  }
}
