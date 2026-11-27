import { type CSSProperties, type KeyboardEvent, useEffect, useRef, useState } from "react";
import { useTexts } from "../i18n";
import { CheckIcon, ChevronLeft, ChevronRight } from "../icons";
import { Stage } from "../tour/Tour";
import type { Changelog, Scene, SceneWords } from "./changelog";

/** What is new in the version just installed, page by page as the tour, the fixes last */
export function WhatsNew({ version, changelog, onClose }: { version: string; changelog: Changelog; onClose: () => void }) {
  const texts = useTexts();
  const words = texts.news;
  const pages = changelog.highlights.length + (changelog.fixes.length > 0 ? 1 : 0);
  const [page, setPage] = useState(0);
  const dialog = useRef<HTMLDialogElement>(null);
  const next = useRef<HTMLButtonElement>(null);
  // The tour may hand over to it, its button going at once: then the page itself has the focus back
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  const last = page === pages - 1;
  const highlight = changelog.highlights[page];

  useEffect(() => {
    dialog.current?.showModal();
    next.current?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
      else document.querySelector<HTMLElement>("main")?.focus();
    };
  }, [opener]);

  useEffect(() => {
    if (!dialog.current?.contains(document.activeElement)) next.current?.focus();
  }, [page]);

  const go = (to: number) => setPage(Math.max(0, Math.min(pages - 1, to)));
  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    if (event.key === "ArrowRight") go(page + 1);
    else if (event.key === "ArrowLeft") go(page - 1);
    else return;
    event.preventDefault();
  };

  return (
    <dialog
      ref={dialog}
      className="tour news-dialog"
      aria-labelledby="news-title"
      onKeyDown={onKeyDown}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <header>
        <span className="news-version">
          <span className="dot ok" />
          {words.updatedTo(version)}
        </span>
        <div className="tour-dots">
          {Array.from({ length: pages }, (_, i) => (
            <button
              key={i}
              type="button"
              className={i < page ? "seen" : undefined}
              aria-label={texts.tour.goTo(i + 1)}
              aria-current={i === page ? "step" : undefined}
              onClick={() => go(i)}
            >
              <span />
            </button>
          ))}
        </div>
        <button type="button" className="tour-skip" onClick={onClose}>
          {texts.close}
        </button>
      </header>

      {highlight ? (
        <Stage>
          <div key={page} className="tour-scene">
            <SceneOf scene={highlight.scene} words={changelog.words} />
          </div>
        </Stage>
      ) : (
        // Words to read, at their size whatever the window's
        <div className="tour-well news-fixes-well">
          <Fixes fixes={changelog.fixes} />
        </div>
      )}

      <div className="tour-text" aria-live="polite">
        <p className="tour-counter">{texts.tour.counter(page + 1, pages)}</p>
        <h2 id="news-title">{highlight ? highlight.title : changelog.fixesTitle}</h2>
        {highlight && <p className="muted">{highlight.text}</p>}
      </div>

      <footer>
        {page > 0 && (
          <button type="button" className="button ghost" onClick={() => go(page - 1)}>
            <ChevronLeft />
            {texts.tour.back}
          </button>
        )}
        <button ref={next} type="button" className="button" onClick={() => (last ? onClose() : go(page + 1))}>
          {last ? words.done : texts.tour.next}
          {!last && <ChevronRight />}
        </button>
      </footer>
    </dialog>
  );
}

function SceneOf({ scene, words }: { scene: Scene; words: SceneWords }) {
  switch (scene) {
    case "macroTogether":
      return <MacroTogether words={words} />;
    case "ledsAtRest":
      return <LedsAtRest words={words} />;
    case "displayOnScreen":
      return <DisplayOnScreen words={words} />;
    case "knobTimes":
      return <KnobTimes words={words} />;
    case "pagesNoGaps":
      return <PagesNoGaps words={words} />;
  }
}

function MacroTogether({ words }: { words: SceneWords }) {
  const clips = [
    { name: "Night Call", off: [words.loop, words.stopsOthers] },
    { name: "Hey DJ!", off: [words.stopsOthers] },
    { name: "Air horn", off: [words.mutesOthers] },
  ];
  return (
    <>
      <div className="news-lanes">
        {clips.map((clip, i) => (
          <div key={clip.name} className="news-lane">
            <span className="news-lane-label">
              {words.track} {i + 1}
            </span>
            <div className={`news-clip clip-${i}`}>
              <span>{clip.name}</span>
              <small>
                {clip.off.map((option, j) => (
                  <s key={option}>
                    {j > 0 && " · "}
                    {option}
                  </s>
                ))}
              </small>
            </div>
          </div>
        ))}
        <div className="news-playhead" />
      </div>
      <div className="news-chips at-right">
        <span className="news-chip ok">{words.overEachOther}</span>
        <span className="news-chip">{words.putBack}</span>
      </div>
    </>
  );
}

const LED_PADS: [string, "solid" | "breathe" | "blink"][] = [
  ["#ffb020", "breathe"],
  ["#22e05a", "solid"],
  ["#3a8bff", "blink"],
  ["#ff3ca0", "breathe"],
  ["#96e61e", "solid"],
  ["#dc28ff", "blink"],
  ["#00c896", "breathe"],
  ["#ff5a3c", "solid"],
  ["#b496ff", "breathe"],
  ["#ff2a2a", "blink"],
  ["#c8c8d2", "solid"],
  ["#ffd27a", "breathe"],
];

function LedsAtRest({ words }: { words: SceneWords }) {
  return (
    <>
      <div className="news-pads">
        {LED_PADS.map(([colour, effect], i) => (
          <div key={i} className={`news-pad ${effect}`} style={{ "--led": colour } as CSSProperties}>
            <i />
          </div>
        ))}
      </div>
      <div className="news-legend">
        <span className="solid">
          <i />
          {words.solid}
        </span>
        <span className="breathe">
          <i />
          {words.breathe}
        </span>
        <span className="blink">
          <i />
          {words.blink}
        </span>
      </div>
    </>
  );
}

function DisplayOnScreen({ words }: { words: SceneWords }) {
  return (
    <>
      <div className="news-screen">
        <div className="news-oled">
          SOUNDBOARD
          <br />
          VOL 78%
          <div className="news-oled-bar">
            <b />
          </div>
        </div>
        <div className="news-overlay">
          {Array.from({ length: 18 }, (_, i) => (
            <span key={i} />
          ))}
        </div>
        <div className="news-taskbar" />
      </div>
      <div className="news-chips at-bottom">
        <span className="news-chip">{words.never}</span>
        <span className="news-chip">{words.always}</span>
        <span className="news-chip on">{words.withActions}</span>
      </div>
    </>
  );
}

function KnobTimes({ words }: { words: SceneWords }) {
  const knob = (speed: "fast" | "slow", seconds: number, caption: string) => (
    <div className="news-knob">
      <div className={`news-dial ${speed}`}>
        <svg viewBox="0 0 120 120" aria-hidden="true">
          <circle className="track" cx="60" cy="60" r="54" pathLength={100} />
          <circle className="fill" cx="60" cy="60" r="54" pathLength={100} />
        </svg>
        <span className="news-cap" />
      </div>
      <p>
        <b>{words.seconds(seconds)}</b>
        {caption}
      </p>
    </div>
  );
  return (
    <div className="news-knobs">
      {knob("fast", 0.6, words.untilNow)}
      {knob("slow", 1.5, words.asYouSet)}
    </div>
  );
}

const SLOT = 54;

/** Where a tile comes from, as an offset from where it ends */
function offsetFrom(from: number, to: number) {
  const x = ((from % 6) - (to % 6)) * SLOT;
  const y = (Math.floor(from / 6) - Math.floor(to / 6)) * SLOT;
  return { "--from-x": `${x}px`, "--from-y": `${y}px` };
}

function PagesNoGaps({ words }: { words: SceneWords }) {
  const at = (slot: number): CSSProperties => ({ left: (slot % 6) * SLOT, top: Math.floor(slot / 6) * SLOT });
  // The pad at 3 cleared: those after it move back one
  const filled = [0, 1, 2, 4, 5, 6, 7, 8];
  return (
    <>
      <div className="news-grid">
        {Array.from({ length: 12 }, (_, i) => (
          <span key={i} className="news-cell" style={at(i)} />
        ))}
        {filled.map((from, to) => (
          <span
            key={from}
            className={from === 6 ? "news-tile picked" : "news-tile"}
            style={{ ...at(to), ...offsetFrom(from, to) } as CSSProperties}
          />
        ))}
        <span className="news-tile cleared" style={at(3)} />
      </div>
      <div className="news-chips at-bottom">
        <span className="news-chips-label">{words.startFrom}</span>
        <span className="news-chip">1/3</span>
        <span className="news-chip on">2/3</span>
        <span className="news-chip">3/3</span>
      </div>
    </>
  );
}

function Fixes({ fixes }: { fixes: Changelog["fixes"] }) {
  return (
    <ul className="news-fixes">
      {fixes.map((fix, i) => (
        <li key={i} style={{ animationDelay: `${i * 60}ms` }}>
          <CheckIcon />
          <span>{fix}</span>
        </li>
      ))}
    </ul>
  );
}
