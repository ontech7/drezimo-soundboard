import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type Language = "system" | "english" | "italian";

export type Anchor = "topLeft" | "top" | "topRight" | "left" | "center" | "right" | "bottomLeft" | "bottom" | "bottomRight";

export interface OverlaySettings {
  enabled: boolean;
  anchor: Anchor;
  /** In percent */
  opacity: number;
  seconds: number;
  /** In percent of the full size */
  size: number;
}

/** The app's settings (src-tauri/src/settings.rs) */
export interface Settings {
  language: Language;
  /** In percent */
  ledBrightness: number;
  overlay: OverlaySettings;
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

export function saveSettings({ language, ledBrightness, overlay }: Settings): void {
  invoke("set_settings", { settings: { language, ledBrightness, overlay } }).catch((error) =>
    console.error("cannot save the settings", error)
  );
}

export function saveDisplaySettings(settings: DisplaySettings): void {
  invoke("set_display_settings", { settings }).catch((error) => console.error("cannot change the display's settings", error));
}
