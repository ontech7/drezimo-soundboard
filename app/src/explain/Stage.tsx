import { listen } from "@tauri-apps/api/event";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { Dialog } from "../pages/PadPicker";

interface Props {
  title: string;
  subtitle: string;
  /** The two halves and the arrow between them */
  scene: ReactNode;
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
  const close = useRef(props.onClose);
  useEffect(() => {
    close.current = props.onClose;
  });
  // Hidden in the tray, the window may still run its timers (WebView2 is not told): the explanation goes
  useEffect(() => {
    const unlisten = listen("window-closed", () => close.current());
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);
  const sentence = useId();
  const note = useId();

  return (
    <Dialog
      label={`${props.subtitle}: ${props.title}`}
      title={props.title}
      subtitle={props.subtitle}
      className="explainer"
      describedBy={props.note ? `${sentence} ${note}` : sentence}
      onClose={props.onClose}
    >
      <div className="dialog-body">
        <div className="explain-stage" aria-hidden="true">
          {props.scene}
        </div>
        <p id={sentence} className="explain-sentence">
          {props.sentence}
        </p>
        {props.note && (
          <p id={note} className="explain-note">
            {props.note}
          </p>
        )}
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

/** Between the halves: it lights at each beat and fades, or only lights while the system asks for less motion */
export function Arrow({ fired }: { fired: number }) {
  const [lit, setLit] = useState(false);
  useEffect(() => {
    if (!fired) return;
    setLit(true);
    const timer = window.setTimeout(() => setLit(false), 120);
    return () => clearTimeout(timer);
  }, [fired]);
  return (
    <span className={lit ? "explain-arrow lit" : "explain-arrow"}>
      <svg viewBox="0 0 16 16">
        <path d="M2 8h11M9 4l4 4-4 4" />
      </svg>
    </span>
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
