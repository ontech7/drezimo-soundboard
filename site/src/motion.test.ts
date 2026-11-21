import { describe, expect, it } from "vitest";
import { explode, pressDepth, PRESS_MS, waveColour } from "./motion";

describe("explode", () => {
  it("is closed at the top of the page", () => {
    for (const value of Object.values(explode(0))) expect(value).toBeCloseTo(0);
  });

  it("opens the shell up, the keys higher and the bottom down halfway", () => {
    const open = explode(0.6);
    expect(open.shell).toBeGreaterThan(0);
    expect(open.keys).toBeGreaterThan(open.shell);
    expect(open.bottom).toBeLessThan(0);
  });

  it("opens all the way first, then closes again for the end", () => {
    let last = explode(0);
    for (let p = 0.05; p <= 0.6; p += 0.05) {
      const now = explode(p);
      expect(now.shell).toBeGreaterThanOrEqual(last.shell);
      expect(now.bottom).toBeLessThanOrEqual(last.bottom);
      last = now;
    }
    const end = explode(1);
    for (const part of ["shell", "keys", "bottom"] as const) expect(end[part]).toBeCloseTo(0);
    expect(end.spin).toBeCloseTo(0);
  });

  it("stays put past either end", () => {
    expect(explode(-1)).toEqual(explode(0));
    expect(explode(3)).toEqual(explode(1));
  });
});

describe("pressDepth", () => {
  it("rests before and after the press", () => {
    expect(pressDepth(0)).toBe(0);
    expect(pressDepth(PRESS_MS)).toBe(0);
    expect(pressDepth(400)).toBe(0);
  });

  it("is all the way down halfway through", () => {
    expect(pressDepth(PRESS_MS / 2)).toBeCloseTo(1);
  });
});

describe("waveColour", () => {
  it("gives colours in range that move with time", () => {
    const key = { x: 10, z: 20 };
    const now = waveColour(key, 0);
    for (const channel of now) {
      expect(channel).toBeGreaterThanOrEqual(0);
      expect(channel).toBeLessThanOrEqual(1);
    }
    expect(waveColour(key, 0.7)).not.toEqual(now);
  });
});
