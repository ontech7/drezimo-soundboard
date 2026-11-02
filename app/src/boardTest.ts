import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Rgb } from "./customPages";

export type LedTest = "keys" | "red" | "green" | "blue" | "white";
export type DisplayTest = "frame" | "full" | "checkers";

/** A board test running (src-tauri/src/board_test.rs) */
export interface BoardTest {
  leds: LedTest;
  display: DisplayTest;
  /** Each key pressed at least once, numbered as the LEDs: the pads, then the function keys */
  tested: boolean[];
  held: boolean[];
  knob: { left: boolean; right: boolean; pressed: boolean; held: boolean };
}

export const LED_TESTS: LedTest[] = ["keys", "red", "green", "blue", "white"];
export const DISPLAY_TESTS: DisplayTest[] = ["frame", "full", "checkers"];

const HELD: Rgb = [255, 255, 255];
const TESTED: Rgb = [34, 224, 90];
const FILLS: Record<Exclude<LedTest, "keys">, Rgb> = {
  red: [255, 42, 42],
  green: [34, 224, 90],
  blue: [47, 107, 255],
  white: [255, 255, 255],
};

/** Key `index`'s LED, as the board lights it: null while off */
export function testLed(test: BoardTest, index: number): Rgb | null {
  if (test.leds !== "keys") return FILLS[test.leds];
  if (test.held[index]) return HELD;
  return test.tested[index] ? TESTED : null;
}

export function useBoardTest(): BoardTest | null {
  const [test, setTest] = useState<BoardTest | null>(null);

  useEffect(() => {
    let updated = false;
    const unlisten = listen<BoardTest | null>("board-test", (event) => {
      updated = true;
      setTest(event.payload);
    });
    unlisten
      .then(() => invoke<BoardTest | null>("board_test"))
      .then((current) => {
        if (!updated) setTest(current);
      })
      .catch((error) => console.error("cannot follow the board test", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return test;
}

export function startBoardTest(): void {
  invoke("start_board_test").catch((error) => console.error("cannot start the board test", error));
}

export function setBoardTestLeds(leds: LedTest): void {
  invoke("set_board_test_leds", { leds }).catch((error) => console.error("cannot change the test's LEDs", error));
}

export function setBoardTestDisplay(display: DisplayTest): void {
  invoke("set_board_test_display", { display }).catch((error) => console.error("cannot change the test's display", error));
}

export function restartBoardTest(): void {
  invoke("restart_board_test").catch((error) => console.error("cannot restart the board test", error));
}

export function stopBoardTest(): void {
  invoke("stop_board_test").catch((error) => console.error("cannot stop the board test", error));
}
