/** The supported games, as plain objects for the interface. */

import { supportedGames } from "./wasm/midway_racer_web.js";

export interface DipSetting {
  readonly value: number;
  readonly label: string;
}

export interface DipSwitch {
  readonly name: string;
  readonly mask: number;
  readonly settings: readonly DipSetting[];
}

export interface GameInfo {
  /** MAME's short name for the set, which names its zip archive. */
  readonly shortName: string;
  readonly title: string;
  readonly year: number;
  readonly manufacturer: string;
  /** The file names of the ROM chips. */
  readonly chips: readonly string[];
  readonly dipSwitches: readonly DipSwitch[];
  readonly defaultDipSwitches: number;
}

/**
 * Copies the game descriptions out of WebAssembly, freeing the copies the
 * bindings hand out, so the interface can read them freely.
 */
export function loadGames(): GameInfo[] {
  return supportedGames().map((game) => {
    const switches = game.dipSwitches.map((dipSwitch) => {
      const values = dipSwitch.values;
      const labels = dipSwitch.labels;
      const copy: DipSwitch = {
        name: dipSwitch.name,
        mask: dipSwitch.mask,
        settings: [...values].map((value, index) => ({
          value,
          label: labels[index] ?? String(value),
        })),
      };
      dipSwitch.free();
      return copy;
    });
    const info: GameInfo = {
      shortName: game.shortName,
      title: game.title,
      year: game.year,
      manufacturer: game.manufacturer,
      chips: game.chips,
      dipSwitches: switches,
      defaultDipSwitches: game.defaultDipSwitches,
    };
    game.free();
    return info;
  });
}
