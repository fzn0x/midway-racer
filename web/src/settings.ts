/** Player preferences, remembered in local storage. */

export interface Settings {
  /** Sound volume, from 0 to 1. */
  volume: number;
  muted: boolean;
}

const STORAGE_KEY = "midway-racer:settings";

export const DEFAULT_SETTINGS: Readonly<Settings> = {
  volume: 0.8,
  muted: false,
};

/** Returns the stored settings, falling back to defaults for anything
 * missing or invalid. */
export function loadSettings(): Settings {
  const settings = { ...DEFAULT_SETTINGS };
  let stored: unknown;
  try {
    stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
  } catch {
    return settings;
  }
  if (typeof stored !== "object" || stored === null) {
    return settings;
  }
  const record = stored as Record<string, unknown>;
  if (typeof record.volume === "number" && Number.isFinite(record.volume)) {
    settings.volume = Math.min(1, Math.max(0, record.volume));
  }
  if (typeof record.muted === "boolean") {
    settings.muted = record.muted;
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
