import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export const DISPLAY_WIDTH = 128;
export const DISPLAY_HEIGHT = 64;

/** The board's display picture, as the hex of its DISPLAY message: bands of 8 rows, least significant bit on top */
export function useDisplayFrame(): string {
  const [frame, setFrame] = useState("");

  useEffect(() => {
    let updated = false;
    const unlisten = listen<string>("display", (event) => {
      updated = true;
      setFrame(event.payload);
    });
    unlisten
      .then(() => invoke<string>("display_frame"))
      .then((current) => {
        if (!updated) setFrame(current);
      })
      .catch((error) => console.error("cannot follow the display", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return frame;
}

export function frameBytes(frame: string): Uint8Array {
  return Uint8Array.from(frame.match(/../g) ?? [], (byte) => parseInt(byte, 16));
}

export function isLit(bytes: Uint8Array, x: number, y: number): boolean {
  return ((bytes[Math.floor(y / 8) * DISPLAY_WIDTH + x] >> (y % 8)) & 1) === 1;
}
