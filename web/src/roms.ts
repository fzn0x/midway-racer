/** The player's ROM files: read into the emulator and kept in storage. */

import type { GameInfo } from "./games";
import { checkChips } from "./loader";
import { clearFiles, loadFiles, saveFiles, type StoredFile } from "./rom-store";
import { RomLibrary } from "./wasm/midway_racer_web.js";

/** Files larger than this cannot be ROM sets of these games and are ignored. */
const MAX_FILE_SIZE = 4 * 1024 * 1024;

export class RomCollection {
  readonly #files = new Map<string, StoredFile>();
  #library = new RomLibrary();
  /** Counts forgets, so files still being read when the player forgot
   * everything are dropped rather than stored again. */
  #generation = 0;

  /** The files, read by the emulator. */
  get library(): RomLibrary {
    return this.#library;
  }

  /**
   * Restores the files stored on a previous visit. Returns how many there
   * were and the problems reading them.
   */
  async restore(): Promise<{ count: number; problems: string[] }> {
    const stored = await loadFiles().catch((error: unknown) => {
      console.warn("cannot read stored ROM files", error);
      return [];
    });
    for (const file of stored) {
      // Files the player dropped while storage was being read are newer.
      if (!this.#files.has(file.name)) {
        this.#files.set(file.name, file);
      }
    }
    return { count: stored.length, problems: this.#rebuild() };
  }

  /** Adds files, replacing those with the same names, and stores them.
   * Returns the problems reading them. */
  async add(files: readonly File[]): Promise<string[]> {
    const generation = this.#generation;
    const tooLarge: string[] = [];
    const read: StoredFile[] = [];
    for (const file of files) {
      if (file.size > MAX_FILE_SIZE) {
        tooLarge.push(file.name);
      } else {
        read.push({ name: file.name, data: await file.arrayBuffer() });
      }
    }
    if (generation !== this.#generation) {
      return ["the files were forgotten while being read"];
    }
    for (const file of read) {
      this.#files.set(file.name, file);
    }
    const problems = this.#rebuild();
    if (tooLarge.length > 0) {
      problems.push(`${tooLarge.join(", ")}: too large to be a ROM set`);
    }
    await saveFiles([...this.#files.values()]).catch((error: unknown) => {
      console.warn("cannot store ROM files", error);
    });
    return problems;
  }

  /** Forgets every file, here at once and in storage when the promise
   * settles. */
  async forget(): Promise<void> {
    this.#generation++;
    this.#files.clear();
    this.#rebuild();
    await clearFiles().catch((error: unknown) => {
      console.warn("cannot clear stored ROM files", error);
    });
  }

  /** Returns the short names of the `games` the files can run. */
  playableGames(games: readonly GameInfo[]): string[] {
    if (this.#library.fileCount === 0) {
      return [];
    }
    return games
      .map((game) => game.shortName)
      .filter((shortName) =>
        checkChips(this.#library, shortName).every((chip) => chip.usable),
      );
  }

  /** Reads every file into a fresh library, dropping the unreadable ones,
   * and returns why they were unreadable. */
  #rebuild(): string[] {
    this.#library.free();
    this.#library = new RomLibrary();
    const problems: string[] = [];
    for (const file of this.#files.values()) {
      try {
        this.#library.addFile(file.name, new Uint8Array(file.data));
      } catch (error) {
        problems.push(error instanceof Error ? error.message : String(error));
        this.#files.delete(file.name);
      }
    }
    return problems;
  }
}
