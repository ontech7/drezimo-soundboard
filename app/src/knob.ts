import type { Library, PageTarget } from "./customPages";
import type { Texts } from "./i18n";
import type { Macro } from "./macros";
import type { KnobSettings, PressAction, TurnAction } from "./settings";

/** A wheel's notch in the legacy `wheelDelta`, Chromium's and WebKit's alike, whatever the system's lines to scroll */
const NOTCH_TICKS = 120;
/** Without it, a notch in pixels at Windows' default of 3 lines */
const NOTCH_PIXELS = 100;

/** The notches a wheel event turns, positive away from the user: a touchpad's moves are a fraction of one */
export function notches(event: WheelEvent): number {
  const legacy = event as WheelEvent & {
    wheelDeltaX?: number;
    wheelDeltaY?: number;
    webkitDirectionInvertedFromDevice?: boolean;
  };
  // macOS's natural scrolling turns the numbers round, not the hand's move
  const sense = legacy.webkitDirectionInvertedFromDevice ? -1 : 1;
  // Sideways only with Shift, when the system turns the wheel that way: a touchpad's own sideways move is no turn
  const ticks = legacy.wheelDeltaY || (event.shiftKey ? legacy.wheelDeltaX : 0);
  if (ticks) return (sense * ticks) / NOTCH_TICKS;
  const moved = event.deltaY || (event.shiftKey ? event.deltaX : 0);
  return sense * (event.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? -moved / NOTCH_PIXELS : -Math.sign(moved));
}

/** In the order the selects list them */
export const TURN_ACTIONS: TurnAction["action"][] = [
  "page",
  "rows",
  "soundboardForOthers",
  "soundboardForMe",
  "playingSound",
  "microphone",
  "voice",
  "nothing",
];
export const PRESS_ACTIONS: PressAction["action"][] = [
  "menu",
  "stopSounds",
  "muteMic",
  "muteForMe",
  "voiceChanger",
  "hearMyself",
  "background",
  "randomVoice",
  "macro",
  "page",
  "overlay",
  "resetVolumes",
  "nothing",
];

/** What a gesture does, as its select names it */
export function actionName(texts: Texts, gesture: keyof KnobSettings, knob: KnobSettings): string {
  return gesture === "turn" || gesture === "holdTurn"
    ? texts.knobTab.turnActions[knob[gesture].action]
    : texts.knobTab.pressActions[knob[gesture].action];
}

/** What goes through Voicemod's bridge (src-tauri/src/voicemod_bridge.rs) */
export const BRIDGE_ACTIONS = new Set(["soundboardForOthers", "soundboardForMe", "playingSound", "microphone", "resetVolumes"]);

/** Why a press's macro or page does nothing: it was deleted since */
export function goneTarget(
  choice: PressAction,
  macros: Macro[] | null,
  targets: PageTarget[],
  library: Library | null,
  texts: Texts
): string | undefined {
  if (choice.action === "macro") {
    const missing = macros !== null && choice.macroId !== "" && !macros.some((item) => item.id === choice.macroId);
    return missing ? texts.knobTab.macroGone : undefined;
  }
  if (choice.action === "page") {
    // Before Voicemod sends its soundboards, no page is known to be gone
    const loaded = (library?.soundboards.length ?? 0) > 0;
    const missing = loaded && choice.page !== "" && !targets.some((page) => page.id === choice.page);
    return missing ? texts.knobTab.pageGone : undefined;
  }
  return undefined;
}
