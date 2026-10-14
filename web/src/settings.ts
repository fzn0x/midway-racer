/** Player preferences, remembered in local storage. */

export interface Settings {
  /** Sound volume, from 0 to 1. */
  volume: number;
  muted: boolean;
  /** Whether to draw scanlines and vignetting like a CRT. */
  crt: boolean;
  /** Whether to draw square pixels instead of the monitor's 4:3 shape. */
  squarePixels: boolean;
  /** DIP switch banks chosen by the player, by game short name. */
  dipSwitches: Record<string, number>;
}

const STORAGE_KEY = "midway-racer:settings";

export const DEFAULT_SETTINGS: Readonly<Settings> = {
  volume: 0.8,
  muted: false,
  crt: true,
  squarePixels: false,
  dipSwitches: {},
};

/** Returns the stored settings, falling back to defaults for anything
 * missing or invalid. */
export function loadSettings(): Settings {
  const settings: Settings = { ...DEFAULT_SETTINGS, dipSwitches: {} };
  let stored: unknown;
  try {
    stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
  } catch {
    return settings;
  }
  if (!isRecord(stored)) {
    return settings;
  }
  if (typeof stored.volume === "number" && Number.isFinite(stored.volume)) {
    settings.volume = Math.min(1, Math.max(0, stored.volume));
  }
  for (const flag of ["muted", "crt", "squarePixels"] as const) {
    const value = stored[flag];
    if (typeof value === "boolean") {
      settings[flag] = value;
    }
  }
  if (isRecord(stored.dipSwitches)) {
    for (const [game, bank] of Object.entries(stored.dipSwitches)) {
      if (isByte(bank)) {
        settings.dipSwitches[game] = bank;
      }
    }
  }
  return settings;
}

/** Remembers `settings`; storage being unavailable is not an error. */
export function saveSettings(settings: Readonly<Settings>): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Private browsing or a full quota: the settings last this visit only.
  }
}

function isByte(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= 0 &&
    value <= 0xff
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
