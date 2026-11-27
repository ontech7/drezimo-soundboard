import { type CSSProperties, useRef, useState } from "react";
import { ANIMATIONS, type LedAnimation, type Rgb, rgb, SWATCHES } from "./customPages";
import { Segmented } from "./controls";
import { useTexts } from "./i18n";
import { useDismiss } from "./dismiss";
import { ChevronDown } from "./icons";

// What the Pages and Macro editors share

/** A tab's name typed in place: Enter or leaving it keeps it, Escape gives up */
export function RenameField(props: { name: string; label: string; max: number; onDone: (name: string) => void }) {
  const [value, setValue] = useState(props.name);
  const done = useRef(false);
  const finish = (result: string) => {
    if (done.current) return;
    done.current = true;
    props.onDone(result.trim());
  };
  return (
    <input
      className="chip rename"
      aria-label={props.label}
      value={value}
      maxLength={props.max}
      autoFocus
      onFocus={(event) => event.currentTarget.select()}
      onChange={(event) => setValue(event.target.value)}
      onBlur={() => finish(value)}
      onKeyDown={(event) => {
        if (event.key === "Enter") finish(value);
        if (event.key === "Escape") finish("");
      }}
    />
  );
}

interface ColourProps {
  /** The colour picked, or null for the usual one */
  colour: Rgb | null;
  /** The colour shown, the usual one when none is picked */
  shown: Rgb;
  onChange: (colour: Rgb | null) => void;
  /** Opens below the button rather than above */
  below?: boolean;
  /** How it moves at rest, to pick when given */
  animation?: LedAnimation;
  onAnimation?: (animation: LedAnimation) => void;
}

/** The LED's colour at rest: a button showing it, and the colours to pick from */
export function ColourPicker({ colour, shown, onChange, below, animation, onAnimation }: ColourProps) {
  const texts = useTexts();
  const [open, setOpen] = useState(false);
  const popover = useDismiss<HTMLDivElement>(open, () => setOpen(false));
  return (
    <div className="anchor" ref={popover}>
      <button
        type="button"
        className="colour-button"
        aria-label={texts.ledColour}
        title={texts.ledColour}
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <span className={`dot ${animation ?? "solid"}`} style={{ "--led": rgb(shown) } as CSSProperties} />
        <ChevronDown />
      </button>
      {open && (
        <div className={below ? "popover below" : "popover"} role="dialog" aria-label={texts.ledAtRest}>
          <p className="muted">{texts.ledAtRest}</p>
          <Swatches colour={colour} onChange={onChange} />
          {onAnimation && (
            <>
              <p className="muted">{texts.ledAnimation}</p>
              <Segmented
                label={texts.ledAnimation}
                options={ANIMATIONS.map((value) => ({ value, label: texts.animations[value] }))}
                value={animation ?? "solid"}
                onChange={onAnimation}
              />
            </>
          )}
        </div>
      )}
    </div>
  );
}

/** The LEDs' colours to pick from, and Auto for the usual one */
export function Swatches(props: { colour: Rgb | null; disabled?: boolean; onChange: (colour: Rgb | null) => void }) {
  const texts = useTexts();
  return (
    <div className="swatches">
      {[null, ...SWATCHES].map((swatch, i) => {
        const name = swatch ? texts.colourNames[i - 1] : texts.automatic;
        const on = swatch ? props.colour?.join() === swatch.join() : props.colour === null;
        return (
          <button
            key={i}
            type="button"
            className={on ? "swatch on" : "swatch"}
            aria-label={name}
            title={name}
            aria-pressed={on}
            disabled={props.disabled}
            style={swatch ? ({ "--led": rgb(swatch) } as CSSProperties) : undefined}
            onClick={() => props.onChange(swatch)}
          >
            {swatch ? null : "Auto"}
          </button>
        );
      })}
    </div>
  );
}
