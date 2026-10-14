import { describe, expect, it } from "vitest";

import { FrameClock, MAX_CATCH_UP_FRAMES } from "./pacing";

const FRAME_RATE = 59.541985;

describe("FrameClock", () => {
  it("keeps the board's refresh rate", () => {
    const clock = new FrameClock(FRAME_RATE, 0);
    let total = 0;
    for (let step = 1; step <= 60; step++) {
      total += clock.framesDue((step * 1000) / 60);
    }
    expect(total).toBe(59);
  });

  it("skips long stalls instead of fast-forwarding", () => {
    const clock = new FrameClock(FRAME_RATE, 0);
    expect(clock.framesDue(10_000)).toBe(MAX_CATCH_UP_FRAMES);
    expect(clock.framesDue(10_000)).toBe(0);
    expect(clock.framesDue(10_017)).toBe(1);
  });

  it("starts over after a restart", () => {
    const clock = new FrameClock(FRAME_RATE, 0);
    clock.framesDue(1_000);
    clock.restart(5_000);
    expect(clock.framesDue(5_010)).toBe(0);
    expect(clock.framesDue(5_020)).toBe(1);
  });
});
