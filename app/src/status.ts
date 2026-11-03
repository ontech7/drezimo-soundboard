import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { MacroProgress } from "./macros";
import type { DisplaySettings } from "./settings";

export type BoardStatus =
  { state: "searching" } | { state: "busy"; port: string } | { state: "connected"; port: string; firmware: string };

export type FirmwareUpdate =
  { state: "updating"; from: string; to: string } | { state: "restarting"; to: string } | { state: "failed"; reason: string };

export type VoicemodStatus = { state: "connecting" | "connected" | "unreachable" | "keyRejected" | "missingKey" };

export interface PageSummary {
  name: string;
  part: number;
  parts: number;
  number: number;
  count: number;
}

export interface Status {
  board: BoardStatus;
  voicemod: VoicemodStatus;
  /** Whether the playback shown comes from Voicemod's log, rather than from guesses */
  voicemodLog: boolean;
  page: PageSummary | null;
  /** Voicemod sent its soundboards: with no page, there is none to show */
  loaded: boolean;
  firmwareUpdate: FirmwareUpdate | null;
  /** As the connected board keeps them */
  displaySettings: DisplaySettings | null;
  /** Voicemod's, while connected */
  license: "free" | "pro" | null;
  /** Voicemod's own version, while connected */
  voicemodVersion: string | null;
  /** The macro running, if any */
  runningMacro: MacroProgress | null;
  /** A sound played while Voicemod's Mute for me was on, which is still on */
  mutedPlay: boolean;
}

export function useStatus(): Status | null {
  const [status, setStatus] = useState<Status | null>(null);

  useEffect(() => {
    // Listening first, then asking: an event that arrives meanwhile is newer than the answer
    let updated = false;
    const unlisten = listen<Status>("status", (event) => {
      updated = true;
      setStatus(event.payload);
    });
    unlisten
      .then(() => invoke<Status>("status"))
      .then((current) => {
        if (!updated) setStatus(current);
      })
      .catch((error) => console.error("cannot follow the app status", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return status;
}
