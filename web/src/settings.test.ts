import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_SETTINGS, loadSettings, saveSettings } from "./settings";

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
    saveSettings({ volume: 0.35, muted: true });
    expect(loadSettings()).toEqual({ volume: 0.35, muted: true });
  });

  it("repairs invalid or tampered values", () => {
    localStorage.setItem(
      "midway-racer:settings",
      '{"volume": 7, "muted": "yes"}',
    );
    expect(loadSettings()).toEqual({ volume: 1, muted: false });

    localStorage.setItem("midway-racer:settings", "not json");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);

    localStorage.setItem("midway-racer:settings", "null");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
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
