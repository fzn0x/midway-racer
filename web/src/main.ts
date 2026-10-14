import "./styles.css";

import { AudioOutput } from "./audio";
import { Controls } from "./controls";
import { CpuView } from "./debugger";
import { closeOnBackdropClick, openDialog, SettingsDialog } from "./dialogs";
import { byId } from "./dom";
import { type GameInfo, loadGames } from "./games";
import { type Command, Keyboard } from "./keyboard";
import { Loader } from "./loader";
import { Panel } from "./panel";
import { RomCollection } from "./roms";
import { Session } from "./session";
import { loadSettings, saveSettings } from "./settings";
import { TouchControls } from "./touch";
import init, { Emulator } from "./wasm/midway_racer_web.js";

/** Volume change per key press. */
const VOLUME_STEP = 0.1;
const DEFAULT_SUBTITLE = "Intel 8080 arcade emulator";

class App {
  readonly #memory: WebAssembly.Memory;
  readonly #audio: AudioOutput | undefined;
  readonly #settings = loadSettings();
  readonly #games: readonly GameInfo[] = loadGames();
  readonly #roms = new RomCollection();
  readonly #loader: Loader;
  readonly #panel: Panel;
  readonly #keyboard: Keyboard;
  readonly #controls: Controls;
  readonly #cpuView = new CpuView(() => this.#session?.stepFrame());
  readonly #settingsDialog: SettingsDialog;
  readonly #helpDialog = byId("help-dialog", HTMLDialogElement);
  readonly #gameSection = byId("game");
  readonly #screen = byId("screen", HTMLCanvasElement);
  readonly #subtitle = byId("game-title");
  #session: Session | undefined;
  #game: GameInfo | undefined;
  /** Whether the player last used a pointer rather than the keyboard. */
  #pointerUsed = false;

  constructor(memory: WebAssembly.Memory, audio: AudioOutput | undefined) {
    this.#memory = memory;
    this.#audio = audio;

    this.#loader = new Loader(this.#games, {
      addFiles: (files) => void this.#addFiles(files),
      play: (shortName) => {
        this.#play(shortName);
      },
      forget: () => void this.#forget(),
    });
    this.#panel = new Panel({
      insertCoin: () => this.#session?.insertCoin(),
      togglePause: () => {
        this.#togglePause();
      },
      reset: () => this.#session?.reset(),
      shift: () => this.#session?.shift(),
      changeGame: () => {
        this.#changeGame();
      },
      toggleMute: () => {
        this.#setSound({ muted: !this.#settings.muted });
      },
      setVolume: (volume) => {
        this.#setSound({ volume, muted: false });
      },
    });
    this.#keyboard = new Keyboard(window, (command) => {
      this.#command(command);
    });
    const touch = new TouchControls((command) => {
      this.#command(command);
    });
    this.#controls = new Controls(this.#keyboard, touch);

    this.#settingsDialog = new SettingsDialog(this.#settings, {
      displayChanged: () => {
        saveSettings(this.#settings);
        this.#applyDisplay();
      },
      dipSwitchesChanged: (bank) => {
        this.#setDipSwitches(bank);
      },
    });
    closeOnBackdropClick(this.#helpDialog);
    byId("open-help").addEventListener("click", () => {
      this.#openDialog(this.#helpDialog);
    });
    byId("open-settings").addEventListener("click", () => {
      this.#settingsDialog.showGame(
        this.#game,
        this.#session?.emulator.dipSwitches ?? 0,
      );
      this.#openDialog(this.#settingsDialog.dialog);
    });

    // Browsers only start sound after the player interacts with the page.
    for (const type of ["pointerdown", "keydown"] as const) {
      window.addEventListener(type, () => void audio?.resume(), {
        passive: true,
      });
    }
    audio?.onRunningChange(() => {
      this.#setSound({});
    });
    this.#setSound({});
    this.#applyDisplay();

    // A control clicked or tapped hands the keyboard back to the game, so
    // the keys keep driving; keyboard users keep their place.
    window.addEventListener(
      "pointerdown",
      () => {
        this.#pointerUsed = true;
      },
      { passive: true, capture: true },
    );
    window.addEventListener(
      "keydown",
      () => {
        this.#pointerUsed = false;
      },
      { passive: true, capture: true },
    );
    document.addEventListener("click", (event) => {
      if (event.detail > 0) {
        this.#returnFocusToGame();
      }
    });
  }

  /** Moves focus off a control of the game or the top bar, so that the
   * keys drive the game again. */
  #returnFocusToGame(): void {
    const focused = document.activeElement;
    if (
      this.#session &&
      focused instanceof HTMLElement &&
      focused.closest("#game, .topbar")
    ) {
      focused.blur();
    }
  }

  /**
   * Restores the ROM files stored on a previous visit, going straight to
   * the game when they hold exactly one playable set.
   */
  async start(): Promise<void> {
    const { count, problems } = await this.#roms.restore();
    this.#loader.setStatus(
      count > 0
        ? `Restored ${plural(count, "file")} from your last visit.`
        : "",
    );
    this.#reportProblems(problems);
    this.#loader.render(this.#roms.library);

    const playable = this.#roms.playableGames(this.#games);
    if (playable.length === 1 && playable[0]) {
      this.#play(playable[0]);
    }
  }

  #command(command: Command): void {
    if (command === "help") {
      this.#openDialog(this.#helpDialog);
      return;
    }
    if (command === "debugger") {
      this.#cpuView.toggle();
      return;
    }
    const session = this.#session;
    if (!session) {
      return;
    }
    switch (command) {
      case "coin":
        session.insertCoin();
        break;
      case "pause":
        this.#togglePause();
        break;
      case "reset":
        session.reset();
        break;
      case "shift":
        session.shift();
        break;
      case "step":
        session.stepFrame();
        break;
      case "mute":
        this.#setSound({ muted: !this.#settings.muted });
        break;
      case "volume-down":
        this.#setSound({
          volume: this.#settings.volume - VOLUME_STEP,
          muted: false,
        });
        break;
      case "volume-up":
        this.#setSound({
          volume: this.#settings.volume + VOLUME_STEP,
          muted: false,
        });
        break;
    }
  }

  /** Opens a dialog, pausing the game and freeing the keyboard meanwhile. */
  #openDialog(dialog: HTMLDialogElement): void {
    if (dialog.open) {
      return;
    }
    const session = this.#session;
    const wasPaused = session?.paused ?? false;
    if (session) {
      session.paused = true;
    }
    this.#keyboard.enabled = false;
    const openedByPointer = this.#pointerUsed;
    openDialog(dialog, () => {
      this.#keyboard.enabled = true;
      if (session && session === this.#session) {
        session.paused = wasPaused;
      }
      // Closing returns focus to the button that opened the dialog, which
      // would catch Space; keyboard users keep it.
      if (openedByPointer) {
        this.#returnFocusToGame();
      }
    });
  }

  /** Changes the sound settings, applies and remembers them. */
  #setSound(change: { volume?: number; muted?: boolean }): void {
    const settings = this.#settings;
    if (change.volume !== undefined) {
      settings.volume = Math.min(
        1,
        Math.max(0, Math.round(change.volume * 20) / 20),
      );
    }
    if (change.muted !== undefined) {
      settings.muted = change.muted;
    }
    saveSettings(settings);
    if (this.#audio) {
      this.#audio.volume = settings.muted ? 0 : settings.volume;
    }
    this.#panel.showSound({
      volume: settings.volume,
      muted: settings.muted,
      available: this.#audio !== undefined,
      running: this.#audio?.running ?? false,
    });
  }

  #applyDisplay(): void {
    const section = this.#gameSection;
    section.dataset.crt = this.#settings.crt ? "on" : "off";
    section.dataset.pixels = this.#settings.squarePixels ? "square" : "monitor";
  }

  /** Sets the running game's DIP switches and remembers them. */
  #setDipSwitches(bank: number): void {
    const game = this.#game;
    if (!game || !this.#session) {
      return;
    }
    this.#session.emulator.dipSwitches = bank;
    this.#settings.dipSwitches[game.shortName] = bank;
    saveSettings(this.#settings);
  }

  #play(shortName: string): void {
    const game = this.#games.find(
      (candidate) => candidate.shortName === shortName,
    );
    if (!game) {
      return;
    }
    this.#session?.stop();
    const emulator = new Emulator(
      this.#roms.library,
      shortName,
      this.#audio?.sampleRate ?? 0,
    );
    const dipSwitches = this.#settings.dipSwitches[shortName];
    if (dipSwitches !== undefined) {
      emulator.dipSwitches = dipSwitches;
    }
    this.#game = game;
    this.#session = new Session(
      emulator,
      this.#memory,
      this.#screen,
      this.#controls,
      this.#panel,
      this.#cpuView,
      this.#audio,
    );
    this.#gameSection.dataset.orientation = emulator.vertical
      ? "vertical"
      : "horizontal";
    this.#subtitle.textContent = `${game.title} · ${game.manufacturer}, ${String(game.year)}`;
    this.#loader.visible = false;
    this.#gameSection.hidden = false;
    this.#session.start();
  }

  #togglePause(): void {
    if (this.#session) {
      this.#session.paused = !this.#session.paused;
    }
  }

  #changeGame(): void {
    this.#session?.stop();
    this.#session = undefined;
    this.#game = undefined;
    this.#gameSection.hidden = true;
    this.#subtitle.textContent = DEFAULT_SUBTITLE;
    this.#loader.render(this.#roms.library);
    this.#loader.visible = true;
  }

  async #addFiles(files: File[]): Promise<void> {
    const problems = await this.#roms.add(files);
    this.#loader.setStatus(`Checked ${plural(files.length, "file")}.`);
    this.#reportProblems(problems);
    this.#loader.render(this.#roms.library);
  }

  async #forget(): Promise<void> {
    await this.#roms.forget();
    this.#loader.setStatus("Forgot the ROM files.");
    this.#loader.render(this.#roms.library);
  }

  #reportProblems(problems: readonly string[]): void {
    if (problems.length > 0) {
      this.#loader.setStatus(
        `Some files could not be read: ${problems.join("; ")}`,
        "error",
      );
    }
  }
}

function plural(count: number, noun: string): string {
  return `${String(count)} ${noun}${count === 1 ? "" : "s"}`;
}

async function main(): Promise<void> {
  const { memory } = await init();
  const audio = await AudioOutput.create().catch((error: unknown) => {
    console.warn("sound is unavailable", error);
    return undefined;
  });
  await new App(memory, audio).start();
}

main().catch((error: unknown) => {
  console.error(error);
  const status = document.getElementById("loader-status");
  if (status) {
    const message = error instanceof Error ? error.message : String(error);
    status.textContent = `The emulator could not start: ${message}`;
    status.dataset.tone = "error";
  }
});
