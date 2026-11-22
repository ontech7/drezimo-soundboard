const LATEST_API = "https://api.github.com/repos/ontech7/drezimo-soundboard/releases/latest";
export const RELEASES_PAGE = "https://github.com/ontech7/drezimo-soundboard/releases/latest";

export interface Downloads {
  version: string;
  exe: string;
  dmg: string;
}

interface Release {
  tag_name?: unknown;
  assets?: { browser_download_url?: unknown }[];
}

export function pickDownloads(release: unknown): Downloads | null {
  const { tag_name, assets } = (release ?? {}) as Release;
  const urls = (Array.isArray(assets) ? assets : [])
    .map((asset) => asset?.browser_download_url)
    .filter((url): url is string => typeof url === "string");
  const exe = urls.find((url) => url.endsWith("-setup.exe"));
  if (!exe) return null;
  return {
    version: typeof tag_name === "string" ? tag_name.replace(/^v/, "") : "",
    exe,
    dmg: urls.find((url) => url.endsWith(".dmg")) ?? RELEASES_PAGE,
  };
}

export async function loadDownloads(fetcher: typeof fetch = fetch): Promise<Downloads> {
  try {
    const response = await fetcher(LATEST_API, { headers: { Accept: "application/vnd.github+json" } });
    if (response.ok) {
      const picked = pickDownloads(await response.json());
      if (picked) return picked;
    }
  } catch {
    // Offline or blocked: the Releases page still has every installer.
  }
  return { version: "", exe: RELEASES_PAGE, dmg: RELEASES_PAGE };
}
