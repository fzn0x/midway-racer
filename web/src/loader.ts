/** The screen that asks for the game's ROMs and shows what was found. */

import { byId, element } from "./dom";
import type { GameDescription, RomLibrary } from "./wasm/midway_racer_web.js";

/** What the player does on the loader screen. */
export interface LoaderActions {
  /** The player dropped or chose files. */
  addFiles(files: File[]): void;
  /** The player wants to play the game called `shortName`. */
  play(shortName: string): void;
  /** The player wants the stored ROMs forgotten. */
  forget(): void;
}

const STATUS_TEXT: Readonly<Record<string, string>> = {
  verified: "verified",
  mismatch: "checksum differs",
  "wrong-size": "wrong size",
  missing: "missing",
};

export class Loader {
  readonly #root = byId("loader");
  readonly #status = byId("loader-status");
  readonly #sets = byId("rom-sets");
  readonly #dropZone = byId("drop-zone");
  readonly #input = byId("rom-input", HTMLInputElement);
  readonly #forgetButton = byId("forget-roms", HTMLButtonElement);
  readonly #games: readonly GameDescription[];
  readonly #actions: LoaderActions;

  constructor(games: readonly GameDescription[], actions: LoaderActions) {
    this.#games = games;
    this.#actions = actions;

    byId("choose-files").addEventListener("click", () => {
      this.#input.click();
    });
    this.#input.addEventListener("change", () => {
      const files = [...(this.#input.files ?? [])];
      this.#input.value = "";
      if (files.length > 0) {
        actions.addFiles(files);
      }
    });
    this.#forgetButton.addEventListener("click", () => {
      actions.forget();
    });

    // Files can be dropped anywhere while the loader is showing.
    document.addEventListener("dragover", (event) => {
      if (!this.#root.hidden) {
        event.preventDefault();
        this.#dropZone.classList.add("drop-zone--active");
      }
    });
    document.addEventListener("dragleave", (event) => {
      if (event.relatedTarget === null) {
        this.#dropZone.classList.remove("drop-zone--active");
      }
    });
    document.addEventListener("drop", (event) => {
      if (this.#root.hidden) {
        return;
      }
      event.preventDefault();
      this.#dropZone.classList.remove("drop-zone--active");
      if (event.dataTransfer) {
        void droppedFiles(event.dataTransfer).then((files) => {
          if (files.length > 0) {
            actions.addFiles(files);
          }
        });
      }
    });
  }

  /** Shows or hides the loader. */
  set visible(visible: boolean) {
    this.#root.hidden = !visible;
  }

  /** Shows a message under the drop zone. */
  setStatus(message: string, tone: "info" | "error" = "info"): void {
    this.#status.textContent = message;
    this.#status.dataset.tone = tone;
  }

  /** Shows how the library's files match each game. */
  render(library: RomLibrary): void {
    const loaded = library.fileCount > 0;
    this.#forgetButton.hidden = !loaded;
    this.#sets.replaceChildren();

    for (const game of this.#games) {
      const chips = loaded ? library.check(game.shortName) : undefined;
      if (chips && !chips.some((chip) => chip.status !== "missing")) {
        continue;
      }
      this.#sets.append(this.#renderSet(game, chips));
    }

    if (loaded && this.#sets.childElementCount === 0) {
      this.setStatus(
        "None of these files belong to a supported game. Check the list of expected files below.",
        "error",
      );
      for (const game of this.#games) {
        this.#sets.append(this.#renderSet(game, undefined));
      }
    }
  }

  #renderSet(
    game: GameDescription,
    chips: ReturnType<RomLibrary["check"]> | undefined,
  ): HTMLElement {
    const card = element("article", { className: "rom-set" });
    const header = element("header", { className: "rom-set__header" });
    header.append(
      element("h3", { text: game.title }),
      element("p", {
        className: "rom-set__meta",
        text: `${game.manufacturer}, ${String(game.year)} · ${game.shortName}.zip`,
      }),
    );
    card.append(header);

    const list = element("ul", { className: "chip-list" });
    game.chips.forEach((name, index) => {
      const chip = chips?.[index];
      const status = chip?.status ?? "expected";
      const item = element("li", { className: `chip chip--${status}` });
      item.append(
        element("span", { className: "chip__name", text: name }),
        element("span", {
          className: "chip__status",
          text: chip ? (STATUS_TEXT[status] ?? status) : "needed",
        }),
      );
      if (chip) {
        item.title = chip.detail;
      }
      list.append(item);
    });
    card.append(list);

    if (chips?.every((chip) => chip.usable)) {
      if (!chips.every((chip) => chip.status === "verified")) {
        card.append(
          element("p", {
            className: "rom-set__warning",
            text: "Some chips differ from known good dumps, so the game may misbehave.",
          }),
        );
      }
      const play = element("button", {
        className: "button button--primary",
        text: `Play ${game.title}`,
      });
      play.type = "button";
      play.addEventListener("click", () => {
        this.#actions.play(game.shortName);
      });
      card.append(play);
    }
    return card;
  }
}

/** Collects the files of a drop, looking inside dropped folders. */
async function droppedFiles(transfer: DataTransfer): Promise<File[]> {
  const entries = [...transfer.items]
    .map((item) => item.webkitGetAsEntry())
    .filter((entry): entry is FileSystemEntry => entry !== null);
  if (entries.length === 0) {
    return [...transfer.files];
  }
  const files = await Promise.all(entries.map((entry) => entryFiles(entry, 2)));
  return files.flat();
}

async function entryFiles(
  entry: FileSystemEntry,
  depth: number,
): Promise<File[]> {
  if (isFile(entry)) {
    return [
      await new Promise<File>((resolve, reject) => {
        entry.file(resolve, reject);
      }),
    ];
  }
  if (depth === 0 || !isDirectory(entry)) {
    return [];
  }
  const reader = entry.createReader();
  const children: FileSystemEntry[] = [];
  for (;;) {
    const batch = await new Promise<FileSystemEntry[]>((resolve, reject) => {
      reader.readEntries(resolve, reject);
    });
    if (batch.length === 0) {
      break;
    }
    children.push(...batch);
  }
  const files = await Promise.all(
    children.map((child) => entryFiles(child, depth - 1)),
  );
  return files.flat();
}

function isFile(entry: FileSystemEntry): entry is FileSystemFileEntry {
  return entry.isFile;
}

function isDirectory(
  entry: FileSystemEntry,
): entry is FileSystemDirectoryEntry {
  return entry.isDirectory;
}
