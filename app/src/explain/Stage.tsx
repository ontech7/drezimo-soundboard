import type { ReactNode } from "react";
import { Dialog } from "../pages/PadPicker";

interface Props {
  title: string;
  subtitle: string;
  left: ReactNode;
  right: ReactNode;
  /** Changes each time the gesture acts: the arrow between the halves flashes */
  fired: number;
  sentence: string;
  note?: string;
  choiceLabel: string;
  repeats: string;
  closeLabel: string;
  /** The row's own controls: they save as the row's do */
  children: ReactNode;
  onClose: () => void;
}

/** The dialog of a row's explanation: the gesture, what it does, a sentence and the row's choice */
export function Stage(props: Props) {
  return (
    <Dialog label={props.title} title={props.title} subtitle={props.subtitle} className="explainer" onClose={props.onClose}>
      <div className="dialog-body">
        <div className="explain-stage" aria-hidden="true">
          {props.left}
          <span key={`arrow ${props.fired}`} className={props.fired ? "explain-arrow fired" : "explain-arrow"}>
            <svg viewBox="0 0 16 16">
              <path d="M2 8h11M9 4l4 4-4 4" />
            </svg>
          </span>
          {props.right}
        </div>
        <p className="explain-sentence">{props.sentence}</p>
        {props.note && <p className="explain-note">{props.note}</p>}
        <div className="explain-choice">
          <span className="muted">{props.choiceLabel}</span>
          {props.children}
        </div>
      </div>
      <footer>
        <p className="muted">{props.repeats}</p>
        <button type="button" className="button" data-autofocus onClick={props.onClose}>
          {props.closeLabel}
        </button>
      </footer>
    </Dialog>
  );
}

/** One half of the stage, with its caption following the animation */
export function Half({ caption, live, children }: { caption: string; live: boolean; children: ReactNode }) {
  return (
    <figure className="explain-half">
      <div className="explain-scene">{children}</div>
      <figcaption className={live ? "live" : undefined}>{caption}</figcaption>
    </figure>
  );
}
