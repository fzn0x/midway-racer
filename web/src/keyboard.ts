/** Keyboard controls, by physical key position so any layout works. */

/** Something a key press asks for, besides driving. */
export type Command =
  | "coin"
  | "pause"
  | "reset"
  | "shift"
  | "mute"
  | "volume-down"
  | "volume-up"
  | "help"
  | "debugger"
  | "step";

export const STEER_LEFT = ["ArrowLeft", "KeyA"] as const;
export const STEER_RIGHT = ["ArrowRight", "KeyD"] as const;
export const ACCELERATE = ["ArrowUp", "KeyW"] as const;
export const START = ["Digit1", "Enter", "NumpadEnter"] as const;

const COMMANDS: Readonly<Partial<Record<string, Command>>> = {
  KeyC: "coin",
  Digit5: "coin",
  KeyP: "pause",
  Pause: "pause",
  F3: "reset",
  Space: "shift",
  ShiftLeft: "shift",
  ShiftRight: "shift",
  KeyM: "mute",
  Minus: "volume-down",
  NumpadSubtract: "volume-down",
  Equal: "volume-up",
  NumpadAdd: "volume-up",
  KeyH: "help",
  F1: "help",
  Backquote: "debugger",
  KeyN: "step",
};

/** Keys the game owns, whose default actions such as scrolling are blocked. */
const GAME_KEYS = new Set<string>([
  ...STEER_LEFT,
  ...STEER_RIGHT,
  ...ACCELERATE,
  ...START,
  "ArrowDown",
  ...Object.keys(COMMANDS),
]);

export class Keyboard {
  readonly #held = new Set<string>();

  /** Listens to `target`, reporting command keys to `onCommand`. */
  constructor(target: Window, onCommand: (command: Command) => void) {
    target.addEventListener("keydown", (event) => {
      if (
        !this.enabled ||
        isTyping(event) ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey
      ) {
        return;
      }
      if (GAME_KEYS.has(event.code)) {
        event.preventDefault();
      }
      this.#held.add(event.code);
      const command = COMMANDS[event.code];
      if (command && !event.repeat) {
        onCommand(command);
      }
    });
    target.addEventListener("keyup", (event) => {
      this.#held.delete(event.code);
    });
    // Keys released while the page is in the background never report a
    // keyup, so forget them all rather than leave them stuck.
    target.addEventListener("blur", () => {
      this.#held.clear();
    });
  }

  /** Whether the keyboard drives the game; off while a dialog is open. */
  enabled = true;

  /** Returns whether any of `codes` is held down. */
  isHeld(codes: readonly string[]): boolean {
    return codes.some((code) => this.#held.has(code));
  }
}

/** Whether the key press is meant for a form control. */
function isTyping(event: KeyboardEvent): boolean {
  const target = event.target;
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLSelectElement ||
    target instanceof HTMLTextAreaElement
  );
}
