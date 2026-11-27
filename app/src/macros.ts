import { invoke } from "@tauri-apps/api/core";
import type { LedAnimation, Library, Rgb } from "./customPages";
import { useSaved } from "./saved";

/** Voicemod's switches a macro can set (src-tauri/src/macros.rs) */
export type Switch = "muteMic" | "voiceChanger" | "hearMyself" | "muteForMe" | "background";
export type SwitchState = "on" | "off" | "toggle";

export type Step =
  | { type: "sound"; id: string; name: string; waitEnd: boolean }
  | { type: "wait"; millis: number }
  | { type: "voice"; id: string; name: string }
  | { type: "randomVoice" }
  | { type: "previousVoice" }
  | { type: "switch"; switch: Switch; state: SwitchState }
  | { type: "stopSounds" }
  | { type: "goToPage"; page: string };

/** What a step that is no sound or wait is, on the time line */
export type ActionStep = Exclude<Step, { type: "sound" } | { type: "wait" }>;

/** A volume along a clip, from its start */
export interface Level {
  /** Milliseconds */
  at: number;
  percent: number;
}

/** A sound on the time line (src-tauri/src/macros.rs) */
export interface Clip {
  id: string;
  name: string;
  track: number;
  /** Milliseconds from the macro's start */
  at: number;
  /** How long it plays, in milliseconds: to its end when none */
  cut?: number | null;
  /** Voicemod's own volume when empty */
  volume?: Level[];
}

export type TimedStep = ActionStep & { at: number };

export interface Timeline {
  clips: Clip[];
  actions: TimedStep[];
}

export interface Macro {
  id: string;
  name: string;
  /** The LED at rest on its pads */
  colour: Rgb | null;
  /** How it moves at rest, unless a pad has its own */
  animation?: LedAnimation;
  /** What a press does while it runs */
  again: "stop" | "restart";
  /** How many times it runs; 0 for until stopped */
  repeat: number;
  /** At the end, the voice and the switches it changed go back */
  restore: boolean;
  /** One after the other, for a macro from before the time line not edited since */
  steps: Step[];
  /** Run instead of the steps */
  timeline?: Timeline | null;
}

/** Where the macro running is (src-tauri/src/controller/running.rs) */
export interface MacroProgress {
  id: string;
  /** From 1 */
  step: number;
  steps: number;
  /** When the round under way started, in milliseconds since 1970 */
  roundStarted: number;
}

export const SWITCHES: Switch[] = ["muteMic", "voiceChanger", "hearMyself", "muteForMe", "background"];

// As the app keeps at most (src-tauri/src/macros.rs)
export const MAX_MACROS = 100;
export const MAX_STEPS = 50;
export const MIN_WAIT = 100;
export const MAX_WAIT = 60_000;
export const MAX_TRACKS = 6;
export const MAX_CLIPS = 50;
/** Ten minutes, in milliseconds */
export const MAX_AT = 600_000;
export const MIN_CUT = 100;
/** A sound whose length is not known yet is drawn this long, in seconds */
export const GUESSED_LENGTH = 3;

const MACROS_STORE = { event: "macros", read: "macros", write: "set_macros", argument: "macros" };

/** The user's macros, saved a moment after the last change, why the last save failed, and a save at once */
export function useMacros(): [Macro[] | null, (macros: Macro[]) => void, string | null, () => Promise<void>] {
  return useSaved<Macro>(MACROS_STORE);
}

export function newMacroId(): string {
  return `macro-${crypto.randomUUID()}`;
}

/** Runs the macro as a press on one of its pads would */
export function tryMacro(id: string): void {
  invoke("try_macro", { id }).catch((error) => console.error("cannot try the macro", error));
}

/** Stops the macro running, whatever a press on its pads does */
export function stopMacro(): void {
  invoke("stop_macro").catch((error) => console.error("cannot stop the macro", error));
}

/** A sound's length in seconds, once known */
export function soundLength(library: Library | null, id: string): number | null {
  for (const soundboard of library?.soundboards ?? []) {
    const sound = soundboard.sounds.find((item) => item.id === id);
    if (sound) return sound.length;
  }
  return null;
}

/** How long a step holds the macro, in seconds: null for a sound whose length is not known yet */
export function stepLength(step: Step, library: Library | null): number | null {
  if (step.type === "wait") return step.millis / 1000;
  if (step.type !== "sound" || !step.waitEnd) return 0;
  return soundLength(library, step.id);
}

/** How long a clip plays, in milliseconds, and whether that is known */
export function clipLength(clip: Clip, library: Library | null): { millis: number; known: boolean } {
  const whole = soundLength(library, clip.id);
  const full = whole === null ? GUESSED_LENGTH * 1000 : Math.round(whole * 1000);
  if (clip.cut != null) return { millis: Math.min(clip.cut, full), known: true };
  return { millis: full, known: whole !== null };
}

/**
 * The macro on its time line: its own, or its steps laid out as they run, a sound that the next step does not wait for
 * on the next free track
 */
export function timelineOf(macro: Macro, library: Library | null): Timeline {
  if (macro.timeline) return macro.timeline;
  const clips: Clip[] = [];
  const actions: TimedStep[] = [];
  let at = 0;
  for (const step of macro.steps) {
    if (step.type === "wait") {
      at += step.millis;
    } else if (step.type === "sound") {
      const millis = clipLength({ id: step.id, name: step.name, track: 0, at }, library).millis;
      const busy = (track: number) =>
        clips.some((clip) => clip.track === track && clip.at < at + millis && at < clip.at + clipLength(clip, library).millis);
      let track = 0;
      while (track < MAX_TRACKS - 1 && busy(track)) track += 1;
      clips.push({ id: step.id, name: step.name, track, at });
      if (step.waitEnd) at += millis;
    } else {
      actions.push({ ...step, at });
    }
  }
  return { clips, actions };
}

/** How long a round takes, in seconds: the last clip's end or the last action */
export function timelineLength(timeline: Timeline, library: Library | null): number {
  const ends = [
    ...timeline.clips.map((clip) => clip.at + clipLength(clip, library).millis),
    ...timeline.actions.map((action) => action.at),
  ];
  return Math.max(0, ...ends) / 1000;
}

/** Its sounds and actions in the order they start, as steps, to name them */
export function stepsOf(macro: Macro): Step[] {
  if (!macro.timeline) return macro.steps;
  const sounds = macro.timeline.clips.map((clip) => ({
    at: clip.at,
    step: { type: "sound", id: clip.id, name: clip.name, waitEnd: false } as Step,
  }));
  const actions = macro.timeline.actions.map(({ at, ...step }) => ({ at, step: step as Step }));
  return [...sounds, ...actions].sort((a, b) => a.at - b.at).map((item) => item.step);
}

/** How long it runs, in seconds; null while a sound's length is not known */
export function macroLength(macro: Macro, library: Library | null): number | null {
  if (macro.timeline) {
    const known = macro.timeline.clips.every((clip) => clipLength(clip, library).known);
    return known ? timelineLength(macro.timeline, library) : null;
  }
  const lengths = macro.steps.map((step) => stepLength(step, library));
  return lengths.some((length) => length === null) ? null : lengths.reduce<number>((sum, next) => sum + (next ?? 0), 0);
}

/** Asks Voicemod for the sounds' lengths, for the time line: the app asks at most once a minute */
export function readSoundLengths(): void {
  invoke("read_sound_lengths").catch((error) => console.error("cannot ask for the sounds' lengths", error));
}
