import { type KeyboardEvent, type ReactNode, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTexts } from "../i18n";
import { ChevronLeft, ChevronRight } from "../icons";
import { Board } from "./Board";
import { type Legend, usePictures } from "./sample";
import { AllSet, Colours, Macro, OVERLAY_WIDTH, Pages, Parts, Screens } from "./slides";

// The slides are drawn at this size, then scaled down to fit a smaller window
const STAGE_WIDTH = 708;
const STAGE_HEIGHT = 318;

/** What the board and the app can do, slide by slide: shown at the first start, then from General */
export function Tour({ onClose }: { onClose: (finished: boolean) => void }) {
  const texts = useTexts();
  const slides = texts.tour.slides;
  const [slide, setSlide] = useState(0);
  const [part, setPart] = useState(0);
  const [legend, setLegend] = useState<Legend | null>(null);
  const pictures = usePictures(texts, OVERLAY_WIDTH);
  const dialog = useRef<HTMLDialogElement>(null);
  const next = useRef<HTMLButtonElement>(null);
  // Where the focus goes back when the tour goes
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  const last = slide === slides.length - 1;

  useEffect(() => {
    dialog.current?.showModal();
    next.current?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, [opener]);

  // A control that went with the last slide, as Back with the first one, leaves its focus to Next: the arrows keep working
  useEffect(() => {
    if (!dialog.current?.contains(document.activeElement)) next.current?.focus();
  }, [slide]);

  const go = (to: number) => setSlide(Math.max(0, Math.min(slides.length - 1, to)));
  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    if (event.key === "ArrowRight") go(slide + 1);
    else if (event.key === "ArrowLeft") go(slide - 1);
    else return;
    event.preventDefault();
  };

  let scene: ReactNode = null;
  if (slide === 1) scene = <Parts part={part} onPick={setPart} />;
  if (slide === 2) scene = <Colours legend={legend} onPick={setLegend} />;
  if (slide === 3) scene = <Screens pictures={pictures} />;
  if (slide === 4) scene = <Pages />;
  if (slide === 5) scene = <Macro pictures={pictures} />;
  if (slide === 6) scene = <AllSet />;

  return (
    <dialog
      ref={dialog}
      className="tour"
      aria-labelledby="tour-title"
      onKeyDown={onKeyDown}
      onCancel={(event) => {
        event.preventDefault();
        onClose(false);
      }}
    >
      <header>
        <div className="tour-dots">
          {slides.map((_, i) => (
            <button
              key={i}
              type="button"
              className={i < slide ? "seen" : undefined}
              aria-label={texts.tour.goTo(i + 1)}
              aria-current={i === slide ? "step" : undefined}
              onClick={() => go(i)}
            >
              <span />
            </button>
          ))}
        </div>
        <button type="button" className="tour-skip" onClick={() => onClose(false)}>
          {texts.tour.skip}
        </button>
      </header>

      <Stage>
        {slide <= 2 && (
          <Board
            frame={pictures?.home ?? ""}
            centred={slide === 0}
            legend={slide === 2 ? legend : null}
            part={slide === 1 ? part : null}
          />
        )}
        <div key={slide} className="tour-scene">
          {scene}
        </div>
      </Stage>

      <div className="tour-text" aria-live="polite">
        <p className="tour-counter">{texts.tour.counter(slide + 1, slides.length)}</p>
        <h2 id="tour-title">{slides[slide].title}</h2>
        <p className="muted">{slides[slide].text}</p>
      </div>

      <footer>
        {slide > 0 && (
          <button type="button" className="button ghost" onClick={() => go(slide - 1)}>
            <ChevronLeft />
            {texts.tour.back}
          </button>
        )}
        <button ref={next} type="button" className="button" onClick={() => (last ? onClose(true) : go(slide + 1))}>
          {last ? texts.tour.start : texts.tour.next}
          <ChevronRight />
        </button>
      </footer>
    </dialog>
  );
}

function Stage({ children }: { children: ReactNode }) {
  const well = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);

  useLayoutEffect(() => {
    const element = well.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      const { clientWidth: width, clientHeight: height } = element;
      // Nothing to fit while the dialog is not shown yet
      if (width > 0 && height > 0) setScale(Math.min(1, width / STAGE_WIDTH, height / STAGE_HEIGHT));
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  return (
    <div ref={well} className="tour-well">
      <div className="tour-stage" style={{ scale: String(scale) }}>
        {children}
      </div>
    </div>
  );
}
