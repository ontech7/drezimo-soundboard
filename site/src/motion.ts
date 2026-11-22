export const PRESS_MS = 180;

const clamp = (value: number, low = 0, high = 1) => Math.min(high, Math.max(low, value));
const smooth = (value: number) => value * value * (3 - 2 * value);
const stage = (progress: number, from: number, to: number) => smooth(clamp((progress - from) / (to - from)));

export interface Explode {
  shell: number;
  keys: number;
  bottom: number;
  tilt: number;
  spin: number;
}

/**
 * Offsets in mm and angles in radians for the scroll, 0 to 1: the case opens up, holds,
 * then closes again and turns its top to the camera for the display and the knob.
 */
export function explode(progress: number): Explode {
  const closing = stage(progress, 0.68, 0.92);
  const opening = stage(progress, 0.05, 0.45) - closing;
  const keysOut = stage(progress, 0.15, 0.55) - stage(progress, 0.64, 0.86);
  return {
    shell: 42 * opening,
    keys: 42 * opening + 48 * keysOut,
    bottom: -34 * opening,
    tilt: -0.5 * stage(progress, 0, 0.4) + 0.85 * closing,
    spin: 0.6 * stage(progress, 0.3, 0.6) - 0.6 * closing,
  };
}

/** How far a key is pushed down, 0 to 1, this many ms after it was pressed. */
export function pressDepth(sincePress: number): number {
  if (sincePress <= 0 || sincePress >= PRESS_MS) return 0;
  return Math.sin((Math.PI * sincePress) / PRESS_MS);
}

export type Rgb = [number, number, number];

function hsl(hue: number, saturation: number, lightness: number): Rgb {
  const amount = saturation * Math.min(lightness, 1 - lightness);
  const part = (offset: number) => {
    const k = (offset + hue * 12) % 12;
    return lightness - amount * Math.max(-1, Math.min(k - 3, 9 - k, 1));
  };
  return [part(0), part(8), part(4)];
}

/** The first page is a rainbow; the others light the pads with a band of colours each. */
export const PAGE_HUES = [null, 0.55, 0.02] as const;

/** A key's LED colour: hues sweeping diagonally over the board. */
export function waveColour(key: { x: number; z: number }, seconds: number, page = 0): Rgb {
  const along = (key.x - key.z) / 180;
  const band = PAGE_HUES[page % PAGE_HUES.length];
  const hue = band === null ? along * 0.8 - seconds * 0.12 : band + 0.16 * Math.sin(2 * Math.PI * (along - seconds * 0.25));
  const brightness = 0.35 + 0.25 * (1 + Math.sin(2 * Math.PI * (along * 1.5 - seconds * 0.5)));
  return hsl(((hue % 1) + 1) % 1, 1, 0.5).map((channel) => channel * brightness) as Rgb;
}

export const RIPPLE_MS = 700;
export const SPARK_MS = 900;
const GRAVITY = -340;

/** The ring that spreads from a pressed key: its size, 1 to 2.8 keycaps, and how much it still shows. */
export function ripple(sincePress: number): { scale: number; opacity: number } | null {
  if (sincePress < 0 || sincePress >= RIPPLE_MS) return null;
  const t = sincePress / RIPPLE_MS;
  return { scale: 1 + 1.8 * (1 - (1 - t) ** 3), opacity: (1 - t) ** 2 };
}

/** Where a spark is, in mm from its key, and how bright, this many ms after the press. */
export function spark(sincePress: number, velocity: Rgb): { offset: Rgb; brightness: number } | null {
  if (sincePress < 0 || sincePress >= SPARK_MS) return null;
  const s = sincePress / 1000;
  return {
    offset: [velocity[0] * s, velocity[1] * s + 0.5 * GRAVITY * s * s, velocity[2] * s],
    brightness: 1 - sincePress / SPARK_MS,
  };
}

/** A short thump after a press, 1 at once and gone in a few hundred ms. */
export function kick(sincePress: number): number {
  if (!(sincePress >= 0 && sincePress < 1000)) return 0;
  return Math.exp(-sincePress / 90) * Math.cos(sincePress / 40);
}
