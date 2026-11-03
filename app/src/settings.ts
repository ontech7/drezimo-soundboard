import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type Language = "system" | "english" | "italian";

export type Anchor = "topLeft" | "top" | "topRight" | "left" | "center" | "right" | "bottomLeft" | "bottom" | "bottomRight";

/** What a pad does when pressed again while its sound plays (src-tauri/src/press_again.rs) */
export type PressAgain = "voicemod" | "restart" | "overlap" | "stop" | "pause" | "hold";

/** The screen the mouse is on when the overlay appears, the main one, or one by its system name */
export type OverlayScreen = "mouse" | "main" | { named: string };

export interface OverlaySettings {
  enabled: boolean;
  anchor: Anchor;
  /** In percent */
  opacity: number;
  seconds: number;
  /** In percent of the full size */
  size: number;
  screen: OverlayScreen;
}

/** The app's settings (src-tauri/src/settings.rs) */
export interface Settings {
  language: Language;
  /** In percent */
  ledBrightness: number;
  /** Unless picked for a pad in the Pages editor */
  pressAgain: PressAgain;
  overlay: OverlaySettings;
  /** Shows the Diagnostics page */
  diagnostics: boolean;
  /** The welcome tour was seen through or skipped */
  tourSeen: boolean;
  /** The notice that only the others hear the sounds, while Voicemod's Mute for me is on, was answered */
  muteForMeNoticeSeen: boolean;
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
  const { language, ledBrightness, pressAgain, overlay, diagnostics, tourSeen, muteForMeNoticeSeen } = view;
  const settings = { language, ledBrightness, pressAgain, overlay, diagnostics, tourSeen, muteForMeNoticeSeen };
  invoke("set_settings", { settings }).catch((error) => console.error("cannot save the settings", error));
}

export function saveDisplaySettings(settings: DisplaySettings): void {
  invoke("set_display_settings", { settings }).catch((error) => console.error("cannot change the display's settings", error));
}
