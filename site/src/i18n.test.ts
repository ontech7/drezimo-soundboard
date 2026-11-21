import { describe, expect, it } from "vitest";
import { pickLanguage } from "./i18n";

describe("pickLanguage", () => {
  it.each([
    [["it-IT", "en"], "it"],
    [["en-GB", "it"], "en"],
    [["de-DE", "it"], "it"],
    [["fr"], "en"],
    [[], "en"],
  ])("%j → %s", (languages, expected) => {
    expect(pickLanguage(languages)).toBe(expected);
  });
});
