import { describe, expect, it } from "vitest";
import { explode, kick, pressDepth, PRESS_MS, ripple, RIPPLE_MS, spark, SPARK_MS, waveColour } from "./motion";

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

describe("ripple", () => {
  it("grows from the keycap and fades out", () => {
    expect(ripple(0)).toEqual({ scale: 1, opacity: 1 });
    const later = ripple(RIPPLE_MS * 0.8)!;
    expect(later.scale).toBeGreaterThan(2.5);
    expect(later.opacity).toBeLessThan(0.1);
    expect(ripple(RIPPLE_MS)).toBeNull();
    expect(ripple(-1)).toBeNull();
  });
});

describe("spark", () => {
  it("leaves its key, falls back and dims", () => {
    const flight = spark(200, [30, 120, 0])!;
    expect(flight.offset[0]).toBeCloseTo(6);
    expect(flight.offset[1]).toBeGreaterThan(0);
    expect(flight.brightness).toBeLessThan(1);
    expect(spark(SPARK_MS - 1, [0, 120, 0])!.offset[1]).toBeLessThan(0);
    expect(spark(SPARK_MS, [0, 0, 0])).toBeNull();
  });
});

describe("kick", () => {
  it("starts at full strength and dies away", () => {
    expect(kick(0)).toBe(1);
    expect(Math.abs(kick(600))).toBeLessThan(0.01);
    expect(kick(-5)).toBe(0);
    expect(kick(Infinity)).toBe(0);
  });
});
