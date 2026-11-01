import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type BoardStatus =
  { state: "searching" } | { state: "busy"; port: string } | { state: "connected"; port: string; firmware: string };

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
  page: PageSummary | null;
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
