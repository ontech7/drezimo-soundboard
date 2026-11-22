import { describe, expect, it } from "vitest";
import { loadDownloads, pickDownloads, RELEASES_PAGE } from "./download";

const base = "https://github.com/ontech7/drezimo-soundboard/releases/download/v1.0.1/";
const release = {
  tag_name: "v1.0.1",
  assets: [
    "Drezimo.Soundboard_1.0.1_aarch64.dmg",
    "Drezimo.Soundboard_1.0.1_x64-setup.exe",
    "Drezimo.Soundboard_1.0.1_x64-setup.exe.sig",
    "latest.json",
  ].map((name) => ({ browser_download_url: base + name })),
};

const answer = (status: number, body: unknown) =>
  (async () => new Response(JSON.stringify(body), { status })) as unknown as typeof fetch;

describe("pickDownloads", () => {
  it("finds the installer and the disk image of the release", () => {
    expect(pickDownloads(release)).toEqual({
      version: "1.0.1",
      exe: base + "Drezimo.Soundboard_1.0.1_x64-setup.exe",
      dmg: base + "Drezimo.Soundboard_1.0.1_aarch64.dmg",
    });
  });

  it("is null without a Windows installer", () => {
    expect(pickDownloads({ tag_name: "v2.0.0", assets: [] })).toBeNull();
    expect(pickDownloads({ message: "Not Found" })).toBeNull();
    expect(pickDownloads(null)).toBeNull();
  });
});

describe("loadDownloads", () => {
  it("uses the latest release", async () => {
    expect((await loadDownloads(answer(200, release))).version).toBe("1.0.1");
  });

  it.each([
    ["rate-limited", answer(403, { message: "API rate limit exceeded" })],
    ["empty", answer(200, { tag_name: "v1.0.2", assets: [] })],
    ["offline", (async () => Promise.reject(new TypeError("Failed to fetch"))) as unknown as typeof fetch],
  ])("falls back to the Releases page when %s", async (_, fetcher) => {
    expect(await loadDownloads(fetcher)).toEqual({ version: "", exe: RELEASES_PAGE, dmg: RELEASES_PAGE });
  });
});
