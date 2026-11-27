import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type Language = "system" | "english" | "italian";

export type Anchor = "topLeft" | "top" | "topRight" | "left" | "center" | "right" | "bottomLeft" | "bottom" | "bottomRight";

/** What a pad does when pressed again while its sound plays (src-tauri/src/press_again.rs) */
export type PressAgain = "voicemod" | "restart" | "overlap" | "stop" | "pause" | "hold";

/** The choice for each of Voicemod's playback modes */
export type PressAgainByMode = Record<"playRestart" | "playOverlap" | "playPause" | "playStop" | "playLoopOnPress", PressAgain>;

/** The screen the mouse is on when the overlay appears, the main one, or one by its system name */
export type OverlayScreen = "mouse" | "main" | { named: string };

export interface OverlaySettings {
  enabled: boolean;
  anchor: Anchor;
  /** In percent */
  opacity: number;
  seconds: number;
  /** Shows all the time, until a knob gesture hides it */
  always: boolean;
  /** In percent of the full size */
  size: number;
  screen: OverlayScreen;
}

/** The app's settings (src-tauri/src/settings.rs) */
export interface Settings {
  language: Language;
  /** In percent */
  ledBrightness: number;
  /** By the sound's mode, unless picked for a pad in the Pages editor */
  pressAgainByMode: PressAgainByMode;
  overlay: OverlaySettings;
  /** Shows the Diagnostics page */
  diagnostics: boolean;
  /** The welcome tour was seen through or skipped */
  tourSeen: boolean;
  /** The Macro section's guide was seen */
  macroGuideSeen: boolean;
  /** The notice that only the others hear the sounds, while Voicemod's Mute for me is on, was answered */
  muteForMeNoticeSeen: boolean;
  knob: KnobSettings;
  shownPages: ShownPages;
}

/** Which of Voicemod's pages the board shows after the user's own (src-tauri/src/soundboard.rs) */
export interface ShownPages {
  favourites: boolean;
  voices: boolean;
  /** Only the soundboards the user made in Voicemod */
  onlyMine: boolean;
  /** Soundboards left out, by id */
  hidden: string[];
}

export function showsSoundboard(shown: ShownPages, soundboard: { id: string; custom: boolean }): boolean {
  return (soundboard.custom || !shown.onlyMine) && !shown.hidden.includes(soundboard.id);
}

/** What turning the knob does, plain or held down (src-tauri/src/knob.rs) */
export interface TurnAction {
  action:
    | "page"
    | "rows"
    | "soundboardForOthers"
    | "soundboardForMe"
    | "soundboardForEveryone"
    | "playingSound"
    | "microphone"
    | "voice"
    | "nothing";
}

/** What a tap, a double tap or a long press does */
export type PressAction =
  | {
      action:
        | "menu"
        | "stopSounds"
        | "muteMic"
        | "muteForMe"
        | "voiceChanger"
        | "hearMyself"
        | "background"
        | "overlay"
        | "randomVoice"
        | "resetVolumes"
        | "nothing";
    }
  | { action: "macro"; macroId: string }
  | { action: "page"; page: string };

export interface KnobSettings {
  turn: TurnAction;
  holdTurn: TurnAction;
  tap: PressAction;
  doubleTap: PressAction;
  longPress: PressAction;
}

export interface SettingsView extends Settings {
  /** The language the window speaks */
  shownLanguage: "english" | "italian";
}

/** The display's settings, kept by the board */
export interface DisplaySettings {
  brightness: number;
  /** Minutes, 0 for never */
  dimAfter: number;
  offAfter: number;
  /** Minutes between pixel shifts, 0 for off */
  shiftEvery: number;
}

export function useSettings(): SettingsView | null {
  const [settings, setSettings] = useState<SettingsView | null>(null);

  useEffect(() => {
    let updated = false;
    const unlisten = listen<SettingsView>("settings", (event) => {
      updated = true;
      setSettings(event.payload);
    });
    unlisten
      .then(() => invoke<SettingsView>("settings"))
      .then((current) => {
        if (!updated) setSettings(current);
      })
      .catch((error) => console.error("cannot follow the settings", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return settings;
}

export function saveSettings(view: Settings): void {
  // Only the settings themselves: the view's own fields stay out
  const { language, ledBrightness, pressAgainByMode, overlay, diagnostics, tourSeen, macroGuideSeen } = view;
  const { muteForMeNoticeSeen, knob, shownPages } = view;
  const settings = {
    language,
    ledBrightness,
    pressAgainByMode,
    overlay,
    diagnostics,
    tourSeen,
    macroGuideSeen,
    muteForMeNoticeSeen,
    knob,
    shownPages,
  };
  invoke("set_settings", { settings }).catch((error) => console.error("cannot save the settings", error));
}

export function saveDisplaySettings(settings: DisplaySettings): void {
  invoke("set_display_settings", { settings }).catch((error) => console.error("cannot change the display's settings", error));
}
