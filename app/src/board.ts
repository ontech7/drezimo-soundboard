import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Texts } from "./i18n";

export type BoardMessage =
  | { type: "hello"; firmware: string }
  | { type: "pad"; index: number; pressed: boolean }
  | { type: "button"; index: number; pressed: boolean }
  | { type: "encoder"; step: number }
  | { type: "encoderButton"; pressed: boolean }
  | { type: "ok" }
  | { type: "error"; reason: string }
  /** Recognised by the app: the knob's own presses and turns show too */
  | { type: "gesture"; gesture: "tap" | "doubleTap" | "longPress" };

/** As the app sends it (src-tauri/src/knob.rs): a turn carries its step */
type Gesture = "tap" | "doubleTap" | "longPress" | { turn: number } | { holdTurn: number };

export interface Activity {
  id: number;
  at: Date;
  message: BoardMessage;
}

let nextActivityId = 0;

export function useBoardActivity(limit: number): Activity[] {
  const [activity, setActivity] = useState<Activity[]>([]);

  useEffect(() => {
    const add = (message: BoardMessage) =>
      setActivity((items) => [{ id: nextActivityId++, at: new Date(), message }, ...items].slice(0, limit));
    const messages = listen<BoardMessage>("board-message", (event) => {
      if (event.payload.type !== "hello" && event.payload.type !== "ok") add(event.payload);
    });
    const gestures = listen<Gesture>("knob-gesture", (event) => {
      if (typeof event.payload === "string") add({ type: "gesture", gesture: event.payload });
    });
    for (const unlisten of [messages, gestures]) {
      unlisten.catch((error) => console.error("cannot follow the board activity", error));
    }
    return () => {
      for (const unlisten of [messages, gestures]) unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [limit]);

  return activity;
}

/** In the language shown now, also for what happened before a change */
export function describe(message: BoardMessage, texts: Texts): string {
  switch (message.type) {
    case "pad":
      return texts.pad(message.index + 1, message.pressed);
    case "button":
      return texts.button(texts.buttonNames[message.index], message.pressed);
    case "encoder":
      return texts.knobTurned(message.step > 0);
    case "encoderButton":
      return texts.knob(message.pressed);
    case "error":
      return texts.boardError(message.reason);
    case "gesture":
      return texts.knobGesture(message.gesture);
    default:
      return "";
  }
}
