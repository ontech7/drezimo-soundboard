import type { CSSProperties } from "react";
// One colour, as printed on the board
import logo from "../assets/drezimo-logo-mono.svg?raw";
import { type Rgb, rgb } from "../customPages";
import { DisplayCanvas } from "../DisplayPreview";
import { useTexts } from "../i18n";
import { type Legend, SAMPLE_KEYS, SAMPLE_PADS } from "./sample";

/** Where each part is on the board, in points at 1.5 per millimetre, with its number beside it */
const PARTS: { outline: CSSProperties; marker: CSSProperties }[] = [
  {
    outline: { left: 87, top: 18.5, width: 59, height: 36.5, borderRadius: 5 },
    marker: { left: 76, top: 8 },
  },
  {
    outline: { left: 154.5, top: 20.6, width: 29.5, height: 29.5, borderRadius: "50%" },
    marker: { left: 178, top: 9.6 },
  },
  {
    outline: { left: 9, top: 72, width: 177.5, height: 38.5, borderRadius: 6 },
    marker: { left: -10, top: 81 },
  },
  {
    outline: { left: 9, top: 108.5, width: 177.5, height: 176.6, borderRadius: 6 },
    marker: { left: -10, top: 187 },
  },
];

/** A keycap lit from below by its LED */
export function Cap({ led, breathe, dim, small }: { led: Rgb | null; breathe?: boolean; dim?: boolean; small?: boolean }) {
  const classes = ["cap", led ? "lit" : "", breathe ? "breathe" : "", dim ? "dim" : "", small ? "small" : ""];
  return <span className={classes.join(" ")} style={led ? ({ "--led": rgb(led) } as CSSProperties) : undefined} />;
}

interface Props {
  /** Its display's picture */
  frame: string;
  /** Centred in the stage, rather than on its left */
  centred: boolean;
  /** The pads and keys of another colour dim */
  legend: Legend | null;
  /** The part outlined, by its place in texts.tour.parts */
  part: number | null;
}

/** The board as it looks, with the tour's page on it */
export function Board({ frame, centred, legend, part }: Props) {
  const texts = useTexts();
  const dim = (own: Legend) => legend !== null && own !== legend;
  return (
    <div className={centred ? "tour-board centred" : "tour-board"} aria-hidden="true">
      <div className="board-logo" dangerouslySetInnerHTML={{ __html: logo }} />
      <div className="board-display">
        <DisplayCanvas frame={frame} className="board-oled" />
      </div>
      <span className="board-knob" />
      <div className="board-keys">
        {SAMPLE_KEYS.map((key, i) => (
          <div key={i}>
            <Cap led={key.led} dim={dim(key.legend)} />
            <span>{texts.buttonNames[i]}</span>
          </div>
        ))}
      </div>
      <div className="board-pads">
        {SAMPLE_PADS.map((pad, i) => (
          <Cap key={i} led={pad.led} breathe={pad.breathe} dim={dim(pad.legend)} />
        ))}
      </div>
      {part !== null && (
        <>
          <span className="board-outline" style={PARTS[part].outline} />
          {PARTS.map(({ marker }, i) => (
            <span key={i} className={i === part ? "board-marker on" : "board-marker"} style={marker}>
              {i + 1}
            </span>
          ))}
        </>
      )}
    </div>
  );
}
