import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Texts } from "./i18n";
import type { PlaybackMode } from "./pressAgain";
import { useSaved } from "./saved";
import { type PressAgain, type ShownPages, showsSoundboard } from "./settings";

/** What an action pad does (src-tauri/src/soundboard.rs) */
export type Action =
  | "stopSounds"
  | "muteMic"
  | "voiceChanger"
  | "randomVoice"
  | "hearMyself"
  | "muteForMe"
  | "background"
  | "goToPage"
  | "overlay"
  | "resetVolumes";

export type Content =
  | { kind: "sound"; id: string; name: string }
  | { kind: "voice"; id: string; name: string }
  | { kind: "action"; action: Action; page?: string }
  | { kind: "macro"; id: string };

export type Rgb = [number, number, number];

export type CustomPad = Content & {
  /** Shown instead of its own name, when not empty */
  label: string;
  /** The LED at rest, instead of its kind's colour */
  colour: Rgb | null;
  /** A sound's, instead of the one set on the Drezimo page */
  pressAgain?: PressAgain | null;
  /** A sound's volume when started from this pad, in percent: Voicemod's own when none */
  volume?: number | null;
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
  /** A sound's, in seconds, once known */
  length: number | null;
  /** A sound's mode in Voicemod */
  mode: PlaybackMode | null;
  /** A sound that stops every other one when it starts, as Voicemod is set */
  stopsOthers: boolean;
}

/** Voicemod's sounds and voices, as it last sent them */
export interface Library {
  /** `custom`: made by the user in Voicemod */
  soundboards: { id: string; name: string; custom: boolean; sounds: LibraryItem[] }[];
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
  overlay: [255, 60, 160],
  resetVolumes: [120, 120, 130],
};

export const MACRO_COLOUR: Rgb = [255, 140, 0];
/** A sound playing, or the voice loaded */
export const PLAYING_COLOUR: Rgb = [34, 224, 90];
/** A sound or a voice outside the licence */
export const LOCKED_COLOUR: Rgb = [90, 16, 16];
/** Every LED, blinking, while Voicemod cannot be reached */
export const UNAVAILABLE_COLOUR: Rgb = [255, 42, 42];
/** The page keys, while there are other pages */
export const PAGE_KEYS_COLOUR: Rgb = [120, 120, 130];

/** The LEDs' colours the user can pick */
export const SWATCHES: Rgb[] = [
  [47, 107, 255],
  [150, 80, 255],
  [34, 224, 90],
  [40, 200, 255],
  [255, 210, 26],
  [255, 140, 0],
  [255, 42, 42],
  [255, 60, 160],
  [200, 200, 210],
];

export const ACTIONS: Action[] = [
  "stopSounds",
  "muteMic",
  "voiceChanger",
  "randomVoice",
  "hearMyself",
  "muteForMe",
  "background",
  "goToPage",
  "overlay",
  "resetVolumes",
];

// The Favourites and Voices pages' ids (src-tauri/src/soundboard.rs)
export const FAVOURITES_ID = "favourites";
export const VOICES_ID = "voices";

export interface PageTarget {
  id: string;
  name: string;
}

/** The pages a pad or the knob can open, as the board has them */
export function pageTargets(pages: CustomPage[], library: Library | null, shown: ShownPages, texts: Texts): PageTarget[] {
  const voices = shown.voices && library?.voices.some((voice) => voice.enabled);
  return [
    ...pages.map((page) => ({ id: page.id, name: page.name })),
    ...(shown.favourites && library?.favourites ? [{ id: FAVOURITES_ID, name: texts.favouritesPage }] : []),
    ...(voices ? [{ id: VOICES_ID, name: texts.voicesPage }] : []),
    ...(library?.soundboards ?? [])
      .filter((soundboard) => showsSoundboard(shown, soundboard))
      .map((soundboard) => ({ id: soundboard.id, name: soundboard.name })),
  ];
}

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

const PAGES_STORE = { event: "custom-pages", read: "custom_pages", write: "set_custom_pages", argument: "pages" };

/** The user's pages, saved a moment after the last change, and why the last save failed, if it did */
export function useCustomPages(): [CustomPage[] | null, (pages: CustomPage[]) => void, string | null] {
  const [pages, save, error] = useSaved<CustomPage>(PAGES_STORE);
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
