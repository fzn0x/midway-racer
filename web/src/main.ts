import "./styles.css";

import { Loader } from "./loader";
import { clearFiles, loadFiles, saveFiles, type StoredFile } from "./rom-store";
import init, { RomLibrary, supportedGames } from "./wasm/midway_racer_web.js";

/** Files larger than this cannot be ROM sets of these games and are ignored. */
const MAX_FILE_SIZE = 4 * 1024 * 1024;

class App {
  readonly #loader: Loader;
  #files = new Map<string, StoredFile>();
  #library = new RomLibrary();

  constructor() {
    this.#loader = new Loader(supportedGames(), {
      addFiles: (files) => void this.#addFiles(files),
      play: (shortName) => {
        this.#loader.setStatus(`${shortName} is ready to run.`);
      },
      forget: () => void this.#forget(),
    });
  }

  /** Restores the ROM files stored on a previous visit. */
  async start(): Promise<void> {
    const stored = await loadFiles().catch((error: unknown) => {
      console.warn("cannot read stored ROM files", error);
      return [];
    });
    for (const file of stored) {
      this.#files.set(file.name, file);
    }
    const problems = this.#rebuildLibrary();
    if (stored.length > 0) {
      this.#loader.setStatus(
        `Restored ${plural(stored.length, "file")} from your last visit.`,
      );
    } else {
      this.#loader.setStatus("");
    }
    this.#reportProblems(problems);
    this.#loader.render(this.#library);
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
  await init();
  await new App().start();
}

main().catch((error: unknown) => {
  console.error(error);
  const status = document.getElementById("loader-status");
  if (status) {
    status.textContent = `The emulator could not start: ${errorMessage(error)}`;
    status.dataset.tone = "error";
  }
});
