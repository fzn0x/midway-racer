import "./styles.css";

import { AudioOutput } from "./audio";
import { Controls } from "./controls";
import { byId } from "./dom";
import { type Command, Keyboard } from "./keyboard";
import { Loader } from "./loader";
import { Panel } from "./panel";
import { clearFiles, loadFiles, saveFiles, type StoredFile } from "./rom-store";
import { Session } from "./session";
import { TouchControls } from "./touch";
import { loadSettings, saveSettings } from "./settings";
import init, {
  Emulator,
  type GameDescription,
  RomLibrary,
  supportedGames,
} from "./wasm/midway_racer_web.js";

/** Files larger than this cannot be ROM sets of these games and are ignored. */
const MAX_FILE_SIZE = 4 * 1024 * 1024;
/** Volume change per key press. */
const VOLUME_STEP = 0.1;

class App {
  readonly #memory: WebAssembly.Memory;
  readonly #audio: AudioOutput | undefined;
  readonly #settings = loadSettings();
  readonly #games: readonly GameDescription[] = supportedGames();
  readonly #loader: Loader;
  readonly #panel: Panel;
  readonly #controls: Controls;
  readonly #gameSection = byId("game");
  readonly #screen = byId("screen", HTMLCanvasElement);
  readonly #title = byId("game-title");
  #files = new Map<string, StoredFile>();
  #library = new RomLibrary();
  #session: Session | undefined;

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
    const keyboard = new Keyboard(window, (command) => {
      this.#command(command);
    });
    const touch = new TouchControls((command) => {
      this.#command(command);
    });
    this.#controls = new Controls(keyboard, touch);

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
  }

  /**
   * Restores the ROM files stored on a previous visit, going straight to
   * the game when they hold exactly one playable set.
   */
  async start(): Promise<void> {
    const stored = await loadFiles().catch((error: unknown) => {
      console.warn("cannot read stored ROM files", error);
      return [];
    });
    for (const file of stored) {
      this.#files.set(file.name, file);
    }
    const problems = this.#rebuildLibrary();
    this.#loader.setStatus(
      stored.length > 0
        ? `Restored ${plural(stored.length, "file")} from your last visit.`
        : "",
    );
    this.#reportProblems(problems);
    this.#loader.render(this.#library);

    const playable = this.#playableGames();
    if (playable.length === 1 && playable[0]) {
      this.#play(playable[0]);
    }
  }

  #command(command: Command): void {
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

  #play(shortName: string): void {
    this.#session?.stop();
    const emulator = new Emulator(
      this.#library,
      shortName,
      this.#audio?.sampleRate ?? 0,
    );
    this.#session = new Session(
      emulator,
      this.#memory,
      this.#screen,
      this.#controls,
      this.#panel,
      this.#audio,
    );
    this.#gameSection.dataset.orientation = emulator.vertical
      ? "vertical"
      : "horizontal";
    this.#title.textContent = `${emulator.title} · insert a coin with C`;
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
    this.#gameSection.hidden = true;
    this.#title.textContent = "Intel 8080 arcade emulator";
    this.#loader.render(this.#library);
    this.#loader.visible = true;
  }

  /** Returns the short names of the games the library can run. */
  #playableGames(): string[] {
    if (this.#library.fileCount === 0) {
      return [];
    }
    return this.#games
      .map((game) => game.shortName)
      .filter((shortName) =>
        this.#library.check(shortName).every((chip) => chip.usable),
      );
  }

  async #addFiles(files: File[]): Promise<void> {
    const ignored: string[] = [];
    for (const file of files) {
      if (file.size > MAX_FILE_SIZE) {
        ignored.push(file.name);
        continue;
      }
      this.#files.set(file.name, {
        name: file.name,
        data: await file.arrayBuffer(),
      });
    }
    const problems = this.#rebuildLibrary();

    this.#loader.setStatus(`Checked ${plural(files.length, "file")}.`);
    if (ignored.length > 0) {
      problems.push(`${ignored.join(", ")}: too large to be a ROM set`);
    }
    this.#reportProblems(problems);
    this.#loader.render(this.#library);

    await saveFiles([...this.#files.values()]).catch((error: unknown) => {
      console.warn("cannot store ROM files", error);
    });
  }

  async #forget(): Promise<void> {
    this.#files.clear();
    this.#rebuildLibrary();
    this.#loader.setStatus("Forgot the ROM files.");
    this.#loader.render(this.#library);
    await clearFiles().catch((error: unknown) => {
      console.warn("cannot clear stored ROM files", error);
    });
  }

  /** Reads every file into a fresh library and returns the files it rejected. */
  #rebuildLibrary(): string[] {
    this.#library.free();
    this.#library = new RomLibrary();
    const problems: string[] = [];
    for (const file of this.#files.values()) {
      try {
        this.#library.addFile(file.name, new Uint8Array(file.data));
      } catch (error) {
        problems.push(errorMessage(error));
        this.#files.delete(file.name);
      }
    }
    return problems;
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

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
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
    status.textContent = `The emulator could not start: ${errorMessage(error)}`;
    status.dataset.tone = "error";
  }
});
