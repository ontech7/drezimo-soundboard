import { describe, expect, it } from "vitest";
import { isLanguage, pickLanguage } from "./i18n";

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

describe("isLanguage", () => {
  it.each([
    ["it", true],
    ["en", true],
    ["IT", false],
    ["de", false],
    [null, false],
  ])("%s → %s", (value, expected) => {
    expect(isLanguage(value)).toBe(expected);
  });
});
