import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export interface Led {
  color: [number, number, number];
  effect: "solid" | "breathe" | "blink";
}

export interface PadView {
  /** Empty for a pad without a sound or voice */
  name: string;
  /** A voice, rather than a sound */
  voice: boolean;
  /** Its picture's path on the pad-image scheme (src-tauri/src/pictures.rs) */
  picture: string | null;
  led: Led;
  /** Outside the licence */
  locked: boolean;
}

/** The board as the app drives it (src-tauri/src/controller/window.rs) */
export interface BoardView {
  pages: string[];
  page: number;
  pads: PadView[];
  /** The function keys' LEDs, from the left */
  keys: Led[];
}

/** Null while there is nothing to play: Voicemod away, or no soundboards */
export function useBoardView(): BoardView | null {
  const [view, setView] = useState<BoardView | null>(null);

  useEffect(() => {
    let updated = false;
    const unlisten = listen<BoardView | null>("board-view", (event) => {
      updated = true;
      setView(event.payload);
    });
    unlisten
      .then(() => invoke<BoardView | null>("board_view"))
      .then((current) => {
        if (!updated) setView(current);
      })
      .catch((error) => console.error("cannot follow the board", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return view;
}

export function pressPad(index: number, pressed: boolean): void {
  invoke("press_pad", { index, pressed }).catch((error) => console.error("cannot press the pad", error));
}

export function pressKey(index: number): void {
  invoke("press_key", { index }).catch((error) => console.error("cannot press the key", error));
}

export function goToPage(index: number): void {
  invoke("go_to_page", { index }).catch((error) => console.error("cannot change the page", error));
}

export function pictureUrl(path: string): string {
  return convertFileSrc(path, "pad-image");
}

export function isLit(led: Led): boolean {
  return led.color.some((level) => level > 0);
}

export function ledColor(led: Led): string {
  return `rgb(${led.color.join(", ")})`;
}
