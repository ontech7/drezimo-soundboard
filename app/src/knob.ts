import type { Library, PageTarget } from "./customPages";
import type { Texts } from "./i18n";
import type { Macro } from "./macros";
import type { PressAction, TurnAction } from "./settings";

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
  "resetVolumes",
  "nothing",
];

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
