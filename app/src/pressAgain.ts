import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Library } from "./customPages";
import type { Texts } from "./i18n";
import type { PressAgain, PressAgainByMode } from "./settings";

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

/** The modes with a choice of their own, in the order the Drezimo page lists them */
export const MODE_ROWS: PlaybackMode[] = ["PlayRestart", "PlayPause", "PlayStop", "PlayLoopOnPress", "PlayOverlap"];

export function modeKey(mode: PlaybackMode): keyof PressAgainByMode | null {
  if (mode === "Unknown") return null;
  return `p${mode.slice(1)}` as keyof PressAgainByMode;
}

/** The choice for a sound of that mode, as set on the Drezimo page */
export function typeChoice(byMode: PressAgainByMode, mode: PlaybackMode | null): PressAgain {
  const key = mode && modeKey(mode);
  return key ? byMode[key] : "voicemod";
}

/** The choice that does exactly what Voicemod does with a sound of that mode (src-tauri/src/press_again.rs) */
export function voicemodsOwn(mode: PlaybackMode): PressAgain | null {
  const own: Record<PlaybackMode, PressAgain | null> = {
    PlayRestart: "restart",
    PlayPause: "pause",
    PlayStop: "stop",
    PlayLoopOnPress: "hold",
    PlayOverlap: "overlap",
    Unknown: null,
  };
  return own[mode];
}

/** What can be picked for a mode: as in Voicemod first, then each different thing the choices come to */
export function choicesFor(mode: PlaybackMode, outcomes: Outcomes): PressAgain[] {
  const does = CHOICES.map((choice) => outcomes[mode][choice].does);
  const others = does.filter(
    (choice, index) => does.indexOf(choice) === index && choice !== "voicemod" && choice !== voicemodsOwn(mode)
  );
  return ["voicemod", ...others];
}

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

/** The names of the sounds in the licence, by their mode, each sound once */
export function soundNamesByMode(library: Library | null): Map<PlaybackMode, string[]> {
  const seen = new Set<string>();
  const names = new Map<PlaybackMode, string[]>();
  for (const soundboard of library?.soundboards ?? []) {
    for (const sound of soundboard.sounds) {
      if (!sound.enabled || seen.has(sound.id)) continue;
      seen.add(sound.id);
      const mode = sound.mode ?? "Unknown";
      names.set(mode, [...(names.get(mode) ?? []), sound.name]);
    }
  }
  return names;
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
