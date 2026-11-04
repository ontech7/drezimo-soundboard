import type { Outcome } from "../pressAgain";
import type { KnobSettings, PressAction, PressAgain, TurnAction } from "../settings";

export type KnobGesture = keyof KnobSettings;
export type KnobAction = TurnAction["action"] | PressAction["action"];
export type Behaviour = Exclude<PressAgain, "voicemod">;

/** What a knob row's explanation shows, as the board's gestures behave (src-tauri/src/knob.rs) */
export interface KnobStory {
  action: KnobAction;
  /** Per loop: a double tap with no action of its own is two taps */
  times: 1 | 2;
  /** The tap's action, for a double tap or a long press that has none */
  borrowed: boolean;
  /** The tap waits 0.3 s for a second one before it acts */
  waits: boolean;
}

export function knobStory(gesture: KnobGesture, knob: KnobSettings): KnobStory {
  const own = knob[gesture].action;
  const tap = knob.tap.action;
  const doubleTapUsed = knob.doubleTap.action !== "nothing";
  const canBorrow = gesture === "doubleTap" || gesture === "longPress";
  const borrowed = canBorrow && own === "nothing" && tap !== "nothing";
  return {
    action: borrowed ? tap : own,
    times: borrowed && gesture === "doubleTap" ? 2 : 1,
    borrowed,
    // With no double-tap action a tap acts at once, and so does a long hold that counts as one
    waits: doubleTapUsed && (gesture === "tap" || (gesture === "longPress" && borrowed)),
  };
}

/** What a pad pressed again does to its sound, from the choice and what the app makes of it */
export interface PadStory {
  behaviour: Behaviour;
  stopsAll: boolean;
  asInVoicemod: boolean;
  /** Voicemod cannot do the choice for this type of sound: the pad does `behaviour` */
  instead: boolean;
}

export function padStory(choice: PressAgain, own: Behaviour, outcome: Outcome | null): PadStory {
  const does = outcome?.does ?? choice;
  const behaviour = does === "voicemod" ? own : does;
  return {
    behaviour,
    stopsAll: outcome?.stopsAll ?? false,
    asInVoicemod: choice === "voicemod",
    instead: choice !== "voicemod" && behaviour !== choice,
  };
}
