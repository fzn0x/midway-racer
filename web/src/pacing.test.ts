import { describe, expect, it } from "vitest";

import { audioFramesDue, FrameClock, MAX_CATCH_UP_FRAMES } from "./pacing";

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

describe("audioFramesDue", () => {
  it("tops the queue up to the lead time", () => {
    // 80 ms at 48 kHz is 3840 samples, about 4.8 frames of 806 samples.
    expect(audioFramesDue(0, 48_000, FRAME_RATE, 0.08)).toBe(
      MAX_CATCH_UP_FRAMES,
    );
    expect(audioFramesDue(2_500, 48_000, FRAME_RATE, 0.08)).toBe(2);
    expect(audioFramesDue(3_500, 48_000, FRAME_RATE, 0.08)).toBe(1);
    expect(audioFramesDue(3_840, 48_000, FRAME_RATE, 0.08)).toBe(0);
    expect(audioFramesDue(9_000, 44_100, FRAME_RATE, 0.08)).toBe(0);
  });
});
