import type { Outcome } from "../pressAgain";
import type { KnobSettings, PressAction, PressAgain, TurnAction } from "../settings";

export type { KnobGesture } from "../settings";
import type { KnobGesture } from "../settings";
export type Behaviour = Exclude<PressAgain, "voicemod">;

/** What the knob's gesture does: a turn's action answers each detent, a press's each time it acts */
export type Shown = { kind: "turn"; action: TurnAction["action"] } | { kind: "press"; action: PressAction["action"] };

/** What a knob row's explanation shows, as the board's gestures behave (src-tauri/src/knob.rs) */
export interface KnobStory {
  shown: Shown;
  /** The press whose macro or page it shows */
  press: PressAction | null;
  /** The tap's action, for a double tap or a long press that has none: a double tap is then two taps */
  borrowed: boolean;
  /** The tap waits for a second one before it acts */
  waits: boolean;
  /** The knob's times, in milliseconds */
  doubleTapMs: number;
  longPressMs: number;
}

export function knobStory(gesture: KnobGesture, knob: KnobSettings): KnobStory {
  if (gesture === "turn" || gesture === "holdTurn") {
    return {
      shown: { kind: "turn", action: knob[gesture].action },
      press: null,
      borrowed: false,
      waits: false,
      doubleTapMs: knob.doubleTapMs,
      longPressMs: knob.longPressMs,
    };
  }
  const own = knob[gesture];
  const borrowed = gesture !== "tap" && own.action === "nothing" && knob.tap.action !== "nothing";
  const press = borrowed ? knob.tap : own;
  return {
    shown: { kind: "press", action: press.action },
    press,
    borrowed,
    // With no double-tap action a tap acts at once, and so does a long hold that counts as one
    waits: knob.doubleTap.action !== "nothing" && (gesture === "tap" || (gesture === "longPress" && borrowed)),
    doubleTapMs: knob.doubleTapMs,
    longPressMs: knob.longPressMs,
  };
}

/**
 * A macro run once whose steps take no time is over before a second tap can come (the 0.3 s of a double tap):
 * that tap starts it again instead of stopping it. `lengths` in seconds, null when not known.
 */
export function overAtOnce(repeat: number, lengths: (number | null)[]): boolean {
  let total = 0;
  for (const length of lengths) {
    if (length === null) return false;
    total += length;
  }
  return repeat === 1 && total < 0.3;
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
