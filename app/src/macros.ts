import { invoke } from "@tauri-apps/api/core";
import type { Library, Rgb } from "./customPages";
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

export interface Macro {
  id: string;
  name: string;
  /** The LED at rest on its pads */
  colour: Rgb | null;
  /** What a press does while it runs */
  again: "stop" | "restart";
  /** How many times it runs; 0 for until stopped */
  repeat: number;
  /** At the end, the voice and the switches it changed go back */
  restore: boolean;
  steps: Step[];
}

/** Where the macro running is (src-tauri/src/controller/running.rs) */
export interface MacroProgress {
  id: string;
  /** From 1 */
  step: number;
  steps: number;
}

export const SWITCHES: Switch[] = ["muteMic", "voiceChanger", "hearMyself", "muteForMe", "background"];

// As the app keeps at most (src-tauri/src/macros.rs)
export const MAX_MACROS = 100;
export const MAX_STEPS = 50;
export const MIN_WAIT = 100;
export const MAX_WAIT = 60_000;

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

/** How long a step holds the macro, in seconds: null for a sound whose length is not known yet */
export function stepLength(step: Step, library: Library | null): number | null {
  if (step.type === "wait") return step.millis / 1000;
  if (step.type !== "sound" || !step.waitEnd) return 0;
  for (const soundboard of library?.soundboards ?? []) {
    const sound = soundboard.sounds.find((item) => item.id === step.id);
    if (sound) return sound.length;
  }
  return null;
}
