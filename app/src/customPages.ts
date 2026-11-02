import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

/** What an action pad does (src-tauri/src/soundboard.rs) */
export type Action =
  "stopSounds" | "muteMic" | "voiceChanger" | "randomVoice" | "hearMyself" | "muteForMe" | "background" | "goToPage";

export type Content =
  | { kind: "sound"; id: string; name: string }
  | { kind: "voice"; id: string; name: string }
  | { kind: "action"; action: Action; page?: string };

export type Rgb = [number, number, number];

export type CustomPad = Content & {
  /** Shown instead of its own name, when not empty */
  label: string;
  /** The LED at rest, instead of its kind's colour */
  colour: Rgb | null;
};

/** A page of the user's own (src-tauri/src/custom_pages.rs) */
export interface CustomPage {
  id: string;
  name: string;
  /** 36, from the top left */
  pads: (CustomPad | null)[];
}

export interface LibraryItem {
  id: string;
  name: string;
  /** Within the user's licence */
  enabled: boolean;
  picture: string | null;
}

/** Voicemod's sounds and voices, as it last sent them */
export interface Library {
  soundboards: { id: string; name: string; sounds: LibraryItem[] }[];
  voices: LibraryItem[];
  /** Whether the board has a Favourites page */
  favourites: boolean;
}

export const PAD_COUNT = 36;

// The LEDs' colours, as the board shows them (src-tauri/src/controller/mod.rs)
export const SOUND_COLOUR: Rgb = [47, 107, 255];
export const VOICE_COLOUR: Rgb = [150, 80, 255];
export const ACTION_COLOURS: Record<Action, Rgb> = {
  stopSounds: [255, 42, 42],
  muteMic: [255, 42, 42],
  voiceChanger: [34, 224, 90],
  randomVoice: [150, 80, 255],
  hearMyself: [34, 224, 90],
  muteForMe: [255, 140, 0],
  background: [0, 200, 220],
  goToPage: [120, 120, 130],
};

export const ACTIONS: Action[] = [
  "stopSounds",
  "muteMic",
  "voiceChanger",
  "randomVoice",
  "hearMyself",
  "muteForMe",
  "background",
  "goToPage",
];

// The Favourites and Voices pages' ids (src-tauri/src/soundboard.rs)
export const FAVOURITES_ID = "favourites";
export const VOICES_ID = "voices";

export function rgb(colour: Rgb): string {
  return `rgb(${colour.join(", ")})`;
}

/** As the app keeps at most (src-tauri/src/custom_pages.rs) */
export const MAX_PAGES = 50;

/** Where choices made for a pad go: that pad, then the free pads after it, then the free ones before it */
export function padsFrom(pads: (CustomPad | null)[], start: number): number[] {
  const free = (i: number) => i !== start && !pads[i];
  const indices = [...pads.keys()];
  return [start, ...indices.filter((i) => i > start && free(i)), ...indices.filter((i) => i < start && free(i))];
}

export function emptyPage(name: string): CustomPage {
  return { id: newId(), name, pads: Array(PAD_COUNT).fill(null) };
}

export function newId(): string {
  return `page-${crypto.randomUUID()}`;
}

/** At most a number of characters, never half of one */
export function shortened(text: string, max: number): string {
  return Array.from(text).slice(0, max).join("");
}

/** The user's pages, saved a moment after the last change, and why the last save failed, if it did */
export function useCustomPages(): [CustomPage[] | null, (pages: CustomPage[]) => void, string | null] {
  const [pages, setPages] = useState<CustomPage[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // While a change is on its way, an older copy coming back must not undo what was typed since
  const saving = useRef(false);
  const latest = useRef(0);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    let updated = false;
    const unlisten = listen<CustomPage[]>("custom-pages", (event) => {
      updated = true;
      if (!saving.current) setPages(event.payload);
    });
    unlisten
      .then(() => invoke<CustomPage[]>("custom_pages"))
      .then((current) => {
        if (!updated) setPages(current);
      })
      .catch((error) => console.error("cannot follow the pages", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  const save = useCallback((next: CustomPage[]) => {
    setPages(next);
    saving.current = true;
    const change = ++latest.current;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      invoke("set_custom_pages", { pages: next })
        .then(() => setError(null))
        .catch((reason) => setError(String(reason)))
        .finally(() => {
          // A newer change may already be on its way
          if (latest.current === change) saving.current = false;
        });
    }, 300);
  }, []);

  return [pages, save, error];
}

export function useLibrary(): Library | null {
  const [library, setLibrary] = useState<Library | null>(null);

  useEffect(() => {
    let updated = false;
    const unlisten = listen<Library>("library", (event) => {
      updated = true;
      setLibrary(event.payload);
    });
    unlisten
      .then(() => invoke<Library>("library"))
      .then((current) => {
        if (!updated) setLibrary(current);
      })
      .catch((error) => console.error("cannot follow Voicemod's sounds", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return library;
}
