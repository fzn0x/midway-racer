import { describe, expect, it } from "vitest";

import { combine, NEUTRAL_PAD, stick, trigger } from "./gamepad";

describe("stick", () => {
  it("ignores its dead zone and keeps full travel", () => {
    expect(stick(0.1)).toBe(0);
    expect(stick(-0.1)).toBe(0);
    expect(stick(1)).toBe(1);
    expect(stick(-1)).toBe(-1);
    expect(stick(0.56)).toBeCloseTo(0.5);
    expect(stick(Number.NaN)).toBe(0);
  });
});

describe("trigger", () => {
  it("ignores slack at rest", () => {
    expect(trigger(0.04)).toBe(0);
    expect(trigger(1)).toBe(1);
    expect(trigger(0.525)).toBeCloseTo(0.5);
  });
});

describe("combine", () => {
  it("lets the strongest input win", () => {
    const combined = combine(
      { ...NEUTRAL_PAD, steering: -0.8, accelerator: 0.2 },
      { ...NEUTRAL_PAD, steering: 0.3, accelerator: 0.9, start: true },
    );
    expect(combined).toEqual({
      ...NEUTRAL_PAD,
      steering: -0.8,
      accelerator: 0.9,
      start: true,
    });
  });
});
