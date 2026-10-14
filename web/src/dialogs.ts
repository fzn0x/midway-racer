/** The help and settings dialogs. */

import { byId, element } from "./dom";
import type { GameInfo } from "./games";
import type { Settings } from "./settings";

/** Opens `dialog` modally and calls `onClose` once it closes. */
export function openDialog(
  dialog: HTMLDialogElement,
  onClose: () => void,
): void {
  dialog.addEventListener("close", onClose, { once: true });
  dialog.showModal();
}

/** Lets a click on the dimmed page around `dialog` close it. */
export function closeOnBackdropClick(dialog: HTMLDialogElement): void {
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) {
      dialog.close();
    }
  });
}

/** What changing the settings does. */
export interface SettingsActions {
  displayChanged(): void;
  /** The player chose a new DIP switch bank for the running game. */
  dipSwitchesChanged(bank: number): void;
}

export class SettingsDialog {
  readonly dialog = byId("settings-dialog", HTMLDialogElement);
  readonly #note = byId("dip-note");
  readonly #switches = byId("dip-switches");
  readonly #defaults = byId("dip-defaults", HTMLButtonElement);
  readonly #actions: SettingsActions;
  #game: GameInfo | undefined;
  #bank = 0;

  constructor(settings: Settings, actions: SettingsActions) {
    this.#actions = actions;
    closeOnBackdropClick(this.dialog);

    const toggles = [
      ["setting-crt", "crt"],
      ["setting-square-pixels", "squarePixels"],
    ] as const;
    for (const [id, key] of toggles) {
      const checkbox = byId(id, HTMLInputElement);
      checkbox.checked = settings[key];
      checkbox.addEventListener("change", () => {
        settings[key] = checkbox.checked;
        actions.displayChanged();
      });
    }

    this.#defaults.addEventListener("click", () => {
      if (this.#game) {
        this.#setBank(this.#game.defaultDipSwitches);
      }
    });
  }

  /** Shows the DIP switches of `game`, whose bank is set to `bank`. */
  showGame(game: GameInfo | undefined, bank: number): void {
    this.#game = game;
    this.#bank = bank;
    this.#render();
  }

  #setBank(bank: number): void {
    this.#bank = bank;
    this.#actions.dipSwitchesChanged(bank);
    this.#render();
  }

  #render(): void {
    const game = this.#game;
    this.#switches.replaceChildren();
    this.#defaults.hidden = !game;
    if (!game) {
      this.#note.textContent = "Start a game to change its DIP switches.";
      return;
    }
    this.#note.textContent = `${game.title} reads most of these when a game starts. Test modes take over after a reset.`;

    for (const dipSwitch of game.dipSwitches) {
      const label = element("label", { className: "dip-switch" });
      const select = element("select");
      for (const setting of dipSwitch.settings) {
        const option = element("option", { text: setting.label });
        option.value = String(setting.value);
        select.append(option);
      }
      select.value = String(this.#bank & dipSwitch.mask);
      select.addEventListener("change", () => {
        this.#setBank((this.#bank & ~dipSwitch.mask) | Number(select.value));
      });
      label.append(element("span", { text: dipSwitch.name }), select);
      this.#switches.append(label);
    }
  }
}
