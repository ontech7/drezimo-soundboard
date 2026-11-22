import { describe, expect, it } from "vitest";
import { padSound, SOUNDS } from "./sound";

describe("padSound", () => {
  it("gives every pad a sound, all of them on each page", () => {
    for (let page = 0; page < 3; page++) {
      const heard = new Set(Array.from({ length: 36 }, (_, i) => padSound(i + 1, page)));
      expect(heard.size).toBe(SOUNDS.length);
      for (const sound of heard) expect(sound).toBeLessThan(SOUNDS.length);
    }
  });

  it("changes the sounds when the page turns", () => {
    expect(padSound(1, 1)).not.toBe(padSound(1, 0));
  });
});
