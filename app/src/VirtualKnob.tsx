import { invoke } from "@tauri-apps/api/core";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { useDisplayFrame } from "./display";
import { useTexts } from "./i18n";

/** A wheel's notch in the legacy `wheelDelta`, Chromium's and WebKit's alike, whatever the system's lines to scroll */
const NOTCH_TICKS = 120;
/** Without it, a notch in pixels at Windows' default of 3 lines */
const NOTCH_PIXELS = 100;
/** A new wheel gesture: its first move is a step, and a touchpad's leftover does not carry over */
const WHEEL_PAUSE = 400;
/** The indicator's turn per step: the knob has 20 steps a turn */
const STEP_DEGREES = 18;

/** What holds the knob down: the mouse on it, its keys, or Shift with a pad or a turn */
type Holder = "pointer" | "key" | "shift";

const holders = new Set<Holder>();
const watchers = new Set<(down: boolean) => void>();

/** Presses or lets go of the knob for `by`: it is down while anything holds it */
export function holdKnob(by: Holder, held: boolean) {
  const was = holders.size > 0;
  if (held) holders.add(by);
  else holders.delete(by);
  const down = holders.size > 0;
  if (down === was) return;
  if (down) letGoWithTheWindow();
  invoke("press_knob", { pressed: down });
  watchers.forEach((watch) => watch(down));
}

function releaseKnob() {
  [...holders].forEach((by) => holdKnob(by, false));
}

/**
 * Shift as a hand on the knob, with a pad pressed or a step turned. Only while the window has the focus: Shift let go in
 * another program sends this one no key-up.
 */
export function holdWithShift(event: { shiftKey: boolean }) {
  if (event.shiftKey && document.hasFocus()) holdKnob("shift", true);
}

let watching = false;

/** As the pads: a press never outlives Shift, nor the window's focus or showing */
function letGoWithTheWindow() {
  if (watching) return;
  watching = true;
  window.addEventListener("keyup", (event) => {
    if (event.key === "Shift") holdKnob("shift", false);
  });
  // A key-up missed all the same: the next move without Shift lets go
  const shiftGone = (event: MouseEvent) => {
    if (!event.shiftKey) holdKnob("shift", false);
  };
  window.addEventListener("pointermove", shiftGone);
  window.addEventListener("wheel", shiftGone);
  window.addEventListener("blur", releaseKnob);
  document.addEventListener("visibilitychange", releaseKnob);
}

/** The notches a wheel event turns, positive away from the user: a touchpad's moves are a fraction of one */
function notches(event: WheelEvent): number {
  const legacy = event as WheelEvent & { wheelDeltaX?: number; wheelDeltaY?: number };
  // With Shift, Chromium turns the wheel sideways
  const ticks = legacy.wheelDeltaY || legacy.wheelDeltaX;
  if (ticks) return ticks / NOTCH_TICKS;
  const moved = event.deltaY || event.deltaX;
  return event.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? -moved / NOTCH_PIXELS : -Math.sign(moved);
}

function turnKnob(step: 1 | -1) {
  invoke("turn_knob", { step });
}

/**
 * Drezimo's knob beside its display, as on the device: the wheel turns it, a click presses it, a click held is a long
 * press, Shift held with a pad or a turn holds it down. It goes through the board's own gestures.
 */
export function VirtualKnob() {
  const texts = useTexts();
  // Shown with the display beside it, once its first picture came
  const shown = useDisplayFrame() !== "";
  const knob = useRef<HTMLButtonElement>(null);
  const [angle, setAngle] = useState(0);
  const [down, setDown] = useState(false);

  const turn = (step: 1 | -1) => {
    setAngle((angle) => angle + step * STEP_DEGREES);
    turnKnob(step);
  };

  useEffect(() => {
    watchers.add(setDown);
    return () => {
      watchers.delete(setDown);
      releaseKnob();
    };
  }, []);

  useEffect(() => {
    const element = knob.current;
    if (!element) return;
    let travel = 0;
    let last = 0;
    // Not passive, so the page does not scroll meanwhile
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const moved = notches(event);
      if (event.timeStamp - last > WHEEL_PAUSE) {
        // A slow notch on macOS can be a fraction of one: a gesture's first move is a step whatever its size
        travel = moved === 0 ? 0 : Math.sign(moved) * Math.max(Math.abs(moved), 1);
      } else {
        travel += moved;
      }
      last = event.timeStamp;
      const steps = Math.trunc(travel);
      if (steps === 0) return;
      travel -= steps;
      // Held only for a step: Shift with a move short of one is no tap
      holdWithShift(event);
      for (let count = 0; count < Math.abs(steps); count++) turn(steps > 0 ? 1 : -1);
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  }, [shown]);

  if (!shown) return null;

  const keyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    const steps: Record<string, 1 | -1> = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 };
    if (event.key in steps) {
      event.preventDefault();
      turn(steps[event.key]);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (!event.repeat) holdKnob("key", true);
    }
  };

  return (
    <button
      ref={knob}
      type="button"
      className={down ? "virtual-knob down" : "virtual-knob"}
      aria-label={texts.virtualKnob}
      title={texts.virtualKnobHint}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        holdKnob("pointer", true);
      }}
      onPointerUp={() => holdKnob("pointer", false)}
      onPointerCancel={() => holdKnob("pointer", false)}
      onLostPointerCapture={() => holdKnob("pointer", false)}
      onKeyDown={keyDown}
      onKeyUp={(event) => {
        if (event.key === "Enter" || event.key === " ") holdKnob("key", false);
      }}
      onBlur={() => holdKnob("key", false)}
    >
      <span className="virtual-knob-mark" style={{ transform: `rotate(${angle}deg)` }} />
    </button>
  );
}
