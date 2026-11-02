import type { CSSProperties } from "react";
import { PAD_COUNT, type Rgb, rgb } from "./customPages";

interface Props {
  /** Each pad's LED, from the top left: null while off */
  leds: (Rgb | null)[];
  /** All blinking, as the board does without Voicemod, or a wave running across while something loads */
  effect?: "blink" | "wave";
  /** Filled with the LED's colour, rather than tinted */
  solid?: boolean;
}

/** The board's 36 pads, small, as a picture */
export function MiniBoard({ leds, effect, solid }: Props) {
  const classes = ["mini-board", solid ? "solid" : "", effect ?? ""];
  return (
    <div className={classes.join(" ")} aria-hidden="true">
      {Array.from({ length: PAD_COUNT }, (_, i) => {
        const led = leds[i];
        // The wave runs from the top left corner
        const style = { "--led": led ? rgb(led) : undefined, "--step": (i % 6) + Math.floor(i / 6) } as CSSProperties;
        return <span key={i} className={led ? "lit" : undefined} style={style} />;
      })}
    </div>
  );
}
