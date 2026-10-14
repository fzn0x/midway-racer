import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_SETTINGS, loadSettings, saveSettings } from "./settings";

const KEY = "midway-racer:settings";

/** An in-memory stand-in for `localStorage`. */
function memoryStorage(): Pick<Storage, "getItem" | "setItem"> {
  const items = new Map<string, string>();
  return {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => {
      items.set(key, value);
    },
  };
}

describe("settings", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", memoryStorage());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("starts from the defaults", () => {
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("remembers what was saved", () => {
    const settings = {
      volume: 0.35,
      muted: true,
      crt: false,
      squarePixels: true,
      dipSwitches: { lagunar: 0x24 },
    };
    saveSettings(settings);
    expect(loadSettings()).toEqual(settings);
  });

  it("repairs invalid or tampered values", () => {
    localStorage.setItem(
      KEY,
      JSON.stringify({
        volume: 7,
        muted: "yes",
        crt: 0,
        dipSwitches: { lagunar: 300, "280zzzap": 1.5, "280zzzapa": 0xc0 },
      }),
    );
    expect(loadSettings()).toEqual({
      ...DEFAULT_SETTINGS,
      volume: 1,
      dipSwitches: { "280zzzapa": 0xc0 },
    });

    localStorage.setItem(KEY, "not json");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);

    localStorage.setItem(KEY, "[1, 2]");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("does not share the default DIP switch record", () => {
    loadSettings().dipSwitches.lagunar = 1;
    expect(DEFAULT_SETTINGS.dipSwitches).toEqual({});
  });

  it("survives storage being unavailable", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
    });
    expect(() => {
      saveSettings(DEFAULT_SETTINGS);
    }).not.toThrow();
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });
});
