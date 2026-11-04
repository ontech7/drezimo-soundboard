import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Library } from "./customPages";
import type { Texts } from "./i18n";
import type { PressAgain } from "./settings";

/** A sound's own rule in Voicemod */
export type PlaybackMode = "PlayRestart" | "PlayOverlap" | "PlayStop" | "PlayPause" | "PlayLoopOnPress" | "Unknown";

/** What a choice comes to for a sound (src-tauri/src/press_again.rs) */
export interface Outcome {
  /** The choice, or what the pad does instead where Voicemod cannot do it */
  does: PressAgain;
  /** Stopping the sound, or starting it again, stops every other sound too */
  stopsAll: boolean;
}

export type Outcomes = Record<PlaybackMode, Record<PressAgain, Outcome>>;

export const CHOICES: PressAgain[] = ["voicemod", "restart", "overlap", "stop", "pause", "hold"];

const MODES: PlaybackMode[] = ["PlayRestart", "PlayOverlap", "PlayStop", "PlayPause", "PlayLoopOnPress", "Unknown"];

/** As chosen, stopping every sound, or what Voicemod allows instead */
export type Reach = "chosen" | "stopsAll" | "instead";

export function reach(choice: PressAgain, outcome: Outcome): Reach {
  if (outcome.stopsAll) return "stopsAll";
  return outcome.does === choice ? "chosen" : "instead";
}

/** What a press again does to a sound, in a few words */
export function outcomeText(texts: Texts, choice: PressAgain, mode: PlaybackMode, outcome: Outcome): string {
  const words = texts.pressDoes;
  if (outcome.does === "voicemod") {
    const what = words.voicemod[mode];
    return choice === "voicemod" ? what : words.asInVoicemod(what);
  }
  if (outcome.stopsAll) {
    if (outcome.does === "stop") return words.stopsEverySound;
    if (outcome.does === "hold") return words.stopsEverySoundWhenLetGo;
    return words.withOthersStopped(words[outcome.does]);
  }
  if (outcome.does === "stop" && mode === "PlayLoopOnPress") return words.startsAndStops;
  const what = words[outcome.does];
  return outcome.does === choice ? what : words.notStacked(what);
}

/** How many of the sounds in the licence have each mode, the most common first */
export function soundsByMode(library: Library | null): [PlaybackMode, number][] {
  const seen = new Set<string>();
  const counts = new Map<PlaybackMode, number>();
  for (const soundboard of library?.soundboards ?? []) {
    for (const sound of soundboard.sounds) {
      if (!sound.enabled || seen.has(sound.id)) continue;
      seen.add(sound.id);
      const mode = sound.mode ?? "Unknown";
      counts.set(mode, (counts.get(mode) ?? 0) + 1);
    }
  }
  return MODES.filter((mode) => counts.has(mode))
    .map((mode): [PlaybackMode, number] => [mode, counts.get(mode) ?? 0])
    .sort((a, b) => b[1] - a[1]);
}

/** They change only with `singleStop`: whether Voicemod's bridge, which stops a single sound, was found */
export function usePressOutcomes(singleStop: boolean): Outcomes | null {
  const [outcomes, setOutcomes] = useState<Outcomes | null>(null);
  useEffect(() => {
    invoke<Outcomes>("press_outcomes", { singleStop })
      .then(setOutcomes)
      .catch((error) => console.error("cannot read what a press again does", error));
  }, [singleStop]);
  return outcomes;
}
