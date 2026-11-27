import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import {
  ACTION_COLOURS,
  type Library,
  LOCKED_COLOUR,
  MACRO_COLOUR,
  PAGE_KEYS_COLOUR,
  PLAYING_COLOUR,
  type Rgb,
  SOUND_COLOUR,
  VOICE_COLOUR,
} from "../customPages";
import type { Texts } from "../i18n";
import type { Step } from "../macros";

const STOP_COLOUR = ACTION_COLOURS.stopSounds;

// "white", the page keys' colour, has no place in the legend: they always dim
export type Legend = "blue" | "green" | "breathe" | "purple" | "amber" | "red" | "dimRed" | "off" | "white";
export type Kind = "sound" | "voice" | "action" | "macro";

export interface SamplePad {
  /** The LED as the board shows it now, null while off */
  led: Rgb | null;
  breathe: boolean;
  legend: Legend;
  kind: Kind | null;
  /** The LED at rest, as the Pages editor shows it */
  rest: Rgb | null;
  locked: boolean;
}

const LETTERS: Record<string, SamplePad> = {
  S: { led: SOUND_COLOUR, breathe: false, legend: "blue", kind: "sound", rest: SOUND_COLOUR, locked: false },
  P: { led: PLAYING_COLOUR, breathe: false, legend: "green", kind: "sound", rest: SOUND_COLOUR, locked: false },
  Z: { led: PLAYING_COLOUR, breathe: true, legend: "breathe", kind: "sound", rest: SOUND_COLOUR, locked: false },
  L: { led: LOCKED_COLOUR, breathe: false, legend: "dimRed", kind: "sound", rest: SOUND_COLOUR, locked: true },
  V: { led: VOICE_COLOUR, breathe: false, legend: "purple", kind: "voice", rest: VOICE_COLOUR, locked: false },
  G: { led: PLAYING_COLOUR, breathe: false, legend: "green", kind: "voice", rest: VOICE_COLOUR, locked: false },
  M: { led: MACRO_COLOUR, breathe: false, legend: "amber", kind: "macro", rest: MACRO_COLOUR, locked: false },
  // Stop, lit while a sound plays
  R: { led: STOP_COLOUR, breathe: false, legend: "red", kind: "action", rest: ACTION_COLOURS.stopSounds, locked: false },
  ".": { led: null, breathe: false, legend: "off", kind: null, rest: null, locked: false },
};

// The tour's page, row by row, named by texts.tour.names: a sound playing, one paused, one outside the licence, a voice
// loaded, two macros and Stop
const LAYOUT = "SSPSSS" + "SZSSLS" + "VVGVVV" + "MSSRSM" + "SSSS.." + "......";
export const SAMPLE_PADS = [...LAYOUT].map((letter) => LETTERS[letter]);

/** The function keys, from the left: more pages, a sound playing, the voice changer on */
export const SAMPLE_KEYS: { led: Rgb | null; legend: Legend }[] = [
  { led: PAGE_KEYS_COLOUR, legend: "white" },
  { led: PAGE_KEYS_COLOUR, legend: "white" },
  { led: STOP_COLOUR, legend: "red" },
  { led: null, legend: "off" },
  { led: PLAYING_COLOUR, legend: "green" },
  { led: null, legend: "off" },
];

export const PAGES = 5;
const PLAYING = 2;
const PLAYING_LENGTH = 2;
const PLAYING_LEFT = 1.2;
const VOICE_LOADED = LAYOUT.indexOf("G");
const PEEKED = 1;
const PEEKED_LENGTH = 2.4;
/** The macro the Macros slide tries, and the one the Pages slide puts on a pad */
export const MACRO_PAD = 18;
export const SAVED_MACRO_PAD = 23;
/** The pad the Pages slide starts on: an empty one */
export const FIRST_TARGET = 28;
// The sounds and the voice the macro plays
const DRUMROLL = 7;
const TA_DA = 9;
const ROBOT = 12;

export function padName(texts: Texts, pad: number): string {
  return texts.tour.names[pad] ?? "";
}

/** The macro the Macros slide tries, with the sounds it plays */
export function sampleMacro(texts: Texts): { steps: Step[]; library: Library } {
  const item = (pad: number, length: number | null) => ({
    id: `tour-${pad}`,
    name: padName(texts, pad),
    enabled: true,
    picture: null,
    length,
    mode: null,
    stopsOthers: false,
  });
  const drumroll = item(DRUMROLL, 3.2);
  const tada = item(TA_DA, 1.8);
  const robot = item(ROBOT, null);
  return {
    steps: [
      { type: "switch", switch: "muteMic", state: "on" },
      { type: "sound", id: drumroll.id, name: drumroll.name, waitEnd: true },
      { type: "wait", millis: 600 },
      { type: "voice", id: robot.id, name: robot.name },
      { type: "sound", id: tada.id, name: tada.name, waitEnd: true },
    ],
    library: {
      soundboards: [{ id: "tour", name: texts.tour.page, custom: false, sounds: [drumroll, tada] }],
      voices: [robot],
      favourites: false,
    },
  };
}

/** The voice loaded at each of the sample macro's steps */
function macroVoices(texts: Texts): string[] {
  let voice = padName(texts, VOICE_LOADED);
  return sampleMacro(texts).steps.map((step) => {
    if (step.type === "voice") voice = step.name;
    return voice;
  });
}

/** The display's pictures, as the hex of their DISPLAY messages, and the overlay's (src-tauri/src/tour.rs) */
export interface Pictures {
  home: string;
  peek: string;
  row: string;
  ready: string;
  steps: string[];
  overlay: ImageData;
}

/** As the app sends them: the overlay as straight RGBA, in base64 */
type Drawn = Omit<Pictures, "overlay"> & { overlay: { width: number; height: number; pixels: string } };

/** The sample page as the board and the app draw it, the overlay `overlayWidth` points wide */
export function usePictures(texts: Texts, overlayWidth: number): Pictures | null {
  const [pictures, setPictures] = useState<Pictures | null>(null);

  useEffect(() => {
    let current = true;
    const sample = {
      page: texts.tour.page,
      pages: PAGES,
      pads: SAMPLE_PADS.map((pad, i) => ({
        name: padName(texts, i),
        color: pad.led ?? [0, 0, 0],
        breathe: pad.breathe,
        locked: pad.locked,
      })),
      playing: PLAYING,
      played: PLAYING_LENGTH,
      left: PLAYING_LEFT,
      voice: padName(texts, VOICE_LOADED),
      peeked: PEEKED,
      length: PEEKED_LENGTH,
      macroName: padName(texts, MACRO_PAD),
      macroSteps: sampleMacro(texts).steps.length,
      macroVoices: macroVoices(texts),
      overlayWidth: Math.round(overlayWidth * window.devicePixelRatio),
    };
    invoke<Drawn>("tour_pictures", { sample })
      .then(({ overlay, ...screens }) => {
        const binary = atob(overlay.pixels);
        const pixels = new Uint8ClampedArray(binary.length);
        for (let i = 0; i < binary.length; i++) pixels[i] = binary.charCodeAt(i);
        if (current) setPictures({ ...screens, overlay: new ImageData(pixels, overlay.width, overlay.height) });
      })
      .catch((error) => console.error("cannot draw the tour's pictures", error));
    return () => {
      current = false;
    };
  }, [texts, overlayWidth]);

  return pictures;
}
