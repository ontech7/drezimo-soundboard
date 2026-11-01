import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export interface BoardStatus {
  connected: boolean;
  port: string | null;
  firmware: string | null;
}

export type BoardMessage =
  | { type: "hello"; firmware: string }
  | { type: "pad"; index: number; pressed: boolean }
  | { type: "button"; index: number; pressed: boolean }
  | { type: "encoder"; step: number }
  | { type: "encoderButton"; pressed: boolean }
  | { type: "ok" }
  | { type: "error"; reason: string };

export interface Activity {
  id: number;
  at: Date;
  text: string;
}

const BUTTON_NAMES = ["PREV", "NEXT", "STOP", "MUTE ME", "VOICE", "HEAR ME"];

export function useBoardStatus(): BoardStatus | null {
  const [status, setStatus] = useState<BoardStatus | null>(null);

  useEffect(() => {
    invoke<BoardStatus>("board_status").then(setStatus);
    const unlisten = listen<BoardStatus>("board-status", (event) => setStatus(event.payload));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  return status;
}

export function useBoardActivity(limit: number): Activity[] {
  const [activity, setActivity] = useState<Activity[]>([]);

  useEffect(() => {
    let nextId = 0;
    const unlisten = listen<BoardMessage>("board-message", (event) => {
      const text = describe(event.payload);
      if (text) setActivity((items) => [{ id: nextId++, at: new Date(), text }, ...items].slice(0, limit));
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [limit]);

  return activity;
}

function describe(message: BoardMessage): string | null {
  const action = (pressed: boolean) => (pressed ? "pressed" : "released");
  switch (message.type) {
    case "pad":
      return `Pad ${message.index + 1} ${action(message.pressed)}`;
    case "button":
      return `${BUTTON_NAMES[message.index]} ${action(message.pressed)}`;
    case "encoder":
      return message.step > 0 ? "Knob turned right" : "Knob turned left";
    case "encoderButton":
      return `Knob ${action(message.pressed)}`;
    case "error":
      return `Board error: ${message.reason}`;
    default:
      return null;
  }
}
