import { type CSSProperties, Fragment, type ReactNode } from "react";
// One colour, as printed on the board
import logo from "./assets/drezimo-logo-mono.svg?raw";
import { type Rgb, rgb } from "./customPages";
import { DisplayCanvas } from "./DisplayPreview";
import { useTexts } from "./i18n";

interface CapProps {
  led: Rgb | null;
  breathe?: boolean;
  dim?: boolean;
  small?: boolean;
  pressed?: boolean;
  children?: ReactNode;
}

/** A keycap lit from below by its LED */
export function Cap({ led, breathe, dim, small, pressed, children }: CapProps) {
  const classes = [
    "cap",
    led ? "lit" : "",
    breathe ? "breathe" : "",
    dim ? "dim" : "",
    small ? "small" : "",
    pressed ? "pressed" : "",
  ];
  return (
    <span className={classes.join(" ")} style={led ? ({ "--led": rgb(led) } as CSSProperties) : undefined}>
      {children}
    </span>
  );
}

interface DrezimoProps {
  className: string;
  /** Its display's picture */
  frame: string;
  /** The function keys' caps, from the left, then the pads', from the top left */
  keys: ReactNode[];
  pads: ReactNode[];
  /** The knob is held */
  knobHeld?: boolean;
  children?: ReactNode;
}

/** Drezimo as it looks, at 1.5 points per millimetre */
export function Drezimo({ className, frame, keys, pads, knobHeld, children }: DrezimoProps) {
  const texts = useTexts();
  return (
    <div className={`drezimo ${className}`} aria-hidden="true">
      <div className="board-logo" dangerouslySetInnerHTML={{ __html: logo }} />
      <div className="board-display">
        <DisplayCanvas frame={frame} className="board-oled" />
      </div>
      <span className={knobHeld ? "board-knob held" : "board-knob"} />
      <div className="board-keys">
        {keys.map((key, i) => (
          <div key={i}>
            {key}
            <span>{texts.buttonNames[i]}</span>
          </div>
        ))}
      </div>
      <div className="board-pads">
        {pads.map((pad, i) => (
          <Fragment key={i}>{pad}</Fragment>
        ))}
      </div>
      {children}
    </div>
  );
}
