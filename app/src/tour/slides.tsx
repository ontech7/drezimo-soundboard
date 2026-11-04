import { type CSSProperties, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { Row, Section, Segmented, Toggle } from "../controls";
import {
  ACTION_COLOURS,
  LOCKED_COLOUR,
  MACRO_COLOUR,
  PLAYING_COLOUR,
  type Rgb,
  rgb,
  SOUND_COLOUR,
  UNAVAILABLE_COLOUR,
  VOICE_COLOUR,
} from "../customPages";
import { DisplayCanvas } from "../DisplayPreview";
import { Cap } from "../Drezimo";
import { Swatches } from "../editing";
import { useTexts } from "../i18n";
import { MacroIcon, MuteMeIcon, PlayIcon, PlusIcon, SoundIcon, StopIcon, VoiceIcon } from "../icons";
import { languages, WINDOWS } from "../pages/SettingsPage";
import { describe, Timeline, Tinted } from "../pages/MacrosPage";
import { type PageId, Sidebar } from "../Sidebar";
import {
  FIRST_TARGET,
  type Kind,
  type Legend,
  MACRO_PAD,
  padName,
  type Pictures,
  SAMPLE_PADS,
  sampleMacro,
  SAVED_MACRO_PAD,
} from "./sample";

export const OVERLAY_WIDTH = 372;

export function Parts({ part, onPick }: { part: number; onPick: (part: number) => void }) {
  const texts = useTexts();
  return (
    <div className="tour-side tour-parts">
      {texts.tour.parts.map((item, i) => (
        <button
          key={i}
          type="button"
          className={i === part ? "choice on" : "choice"}
          aria-pressed={i === part}
          onClick={() => onPick(i)}
        >
          <span className="tour-number">{i + 1}</span>
          <span className="text">
            <span className="name">{item.title}</span>
            <span className="muted">{item.text}</span>
          </span>
        </button>
      ))}
    </div>
  );
}

const LEGENDS: { id: Exclude<Legend, "white">; led: Rgb | null; breathe?: boolean }[] = [
  { id: "blue", led: SOUND_COLOUR },
  { id: "green", led: PLAYING_COLOUR },
  { id: "breathe", led: PLAYING_COLOUR, breathe: true },
  { id: "purple", led: VOICE_COLOUR },
  { id: "amber", led: MACRO_COLOUR },
  { id: "red", led: ACTION_COLOURS.stopSounds },
  { id: "dimRed", led: LOCKED_COLOUR },
  { id: "off", led: null },
];

export function Colours({ legend, onPick }: { legend: Legend | null; onPick: (legend: Legend | null) => void }) {
  const texts = useTexts();
  return (
    <div className="tour-side">
      <div className="choices wide">
        {LEGENDS.map(({ id, led, breathe }) => {
          const [name, meaning] = texts.tour.legend[id];
          const on = legend === id;
          return (
            <button
              key={id}
              type="button"
              className={on ? "choice on" : "choice"}
              aria-pressed={on}
              onClick={() => onPick(on ? null : id)}
            >
              <Cap led={led} breathe={breathe} small />
              <span className="text">
                <span className="name">{name}</span>
                <span className="muted">{meaning}</span>
              </span>
            </button>
          );
        })}
      </div>
      <p className="tour-warning">
        <span className="cap lit small blink" style={{ "--led": rgb(UNAVAILABLE_COLOUR) } as CSSProperties} />
        {texts.tour.allRed}
      </p>
    </div>
  );
}

type Screen = "home" | "peek" | "row";

export function Screens({ pictures }: { pictures: Pictures | null }) {
  const texts = useTexts();
  const [screen, setScreen] = useState<Screen>("home");
  const screens: Screen[] = ["home", "peek", "row"];
  return (
    <>
      <div className="tour-display">
        <div className="tour-bezel">
          <DisplayCanvas frame={pictures?.[screen] ?? ""} className="tour-oled" />
        </div>
        <Segmented
          label={texts.tour.screensLabel}
          options={screens.map((value) => ({ value, label: texts.tour.screens[value] }))}
          value={screen}
          onChange={setScreen}
        />
        <p className="muted">{texts.tour.screenTexts[screen]}</p>
      </div>
      <figure className="tour-overlay">
        {pictures && <OverlayPicture picture={pictures.overlay} />}
        <figcaption className="muted">{texts.tour.onThePc}</figcaption>
      </figure>
    </>
  );
}

function OverlayPicture({ picture }: { picture: ImageData }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    canvas.current?.getContext("2d")?.putImageData(picture, 0, 0);
  }, [picture]);
  return (
    <canvas
      ref={canvas}
      width={picture.width}
      height={picture.height}
      style={{ width: OVERLAY_WIDTH, aspectRatio: `${picture.width} / ${picture.height}` }}
    />
  );
}

// What the Pages slide puts on a pad: a sound, a voice, Mute for me or a macro
const KINDS: Record<Kind, { colour: Rgb; icon: ReactNode }> = {
  sound: { colour: SOUND_COLOUR, icon: <SoundIcon /> },
  voice: { colour: VOICE_COLOUR, icon: <VoiceIcon /> },
  action: { colour: ACTION_COLOURS.muteForMe, icon: <MuteMeIcon /> },
  macro: { colour: MACRO_COLOUR, icon: <MacroIcon /> },
};

export function Pages() {
  const texts = useTexts();
  const [target, setTarget] = useState(FIRST_TARGET);
  // What the user put on the pads, and the colours they picked
  const [placed, setPlaced] = useState<Record<number, Kind>>({});
  const [colours, setColours] = useState<Record<number, Rgb>>({});

  const kindAt = (pad: number) => placed[pad] ?? SAMPLE_PADS[pad].kind;
  const nameAt = (pad: number) => {
    const kind = placed[pad];
    if (kind === "sound") return texts.tour.soundSample;
    if (kind === "voice") return texts.tour.voiceSample;
    if (kind === "action") return texts.actionNames.muteForMe;
    if (kind === "macro") return padName(texts, SAVED_MACRO_PAD);
    return padName(texts, pad);
  };
  const colourAt = (pad: number) => colours[pad] ?? (placed[pad] ? KINDS[placed[pad]].colour : SAMPLE_PADS[pad].rest);
  const pick = (colour: Rgb | null) => {
    const next = { ...colours };
    if (colour) next[target] = colour;
    else delete next[target];
    setColours(next);
  };
  const targetKind = kindAt(target);

  return (
    <>
      <div className="tour-editor">
        <div className="tour-chips" aria-hidden="true">
          <span className="chip selected">{texts.tour.page}</span>
          <span className="chip">{texts.tour.otherPage}</span>
          <span className="chip add">
            <PlusIcon />
          </span>
        </div>
        <div className="pads">
          {SAMPLE_PADS.map((pad, i) => {
            const kind = kindAt(i);
            const colour = colourAt(i);
            const classes = [
              "pad",
              kind ? "lit" : "empty",
              i === target ? "selected" : "",
              pad.locked && !placed[i] ? "locked" : "",
            ];
            return (
              <button
                key={i}
                type="button"
                className={classes.join(" ")}
                style={colour ? ({ "--led": rgb(colour) } as CSSProperties) : undefined}
                aria-label={kind ? texts.padLabel(i + 1, nameAt(i)) : texts.emptyPadLabel(i + 1)}
                aria-pressed={i === target}
                onClick={() => setTarget(i)}
              >
                {kind ? <span className="name">{nameAt(i)}</span> : <PlusIcon />}
              </button>
            );
          })}
        </div>
      </div>

      <div className="tour-inspector">
        <div>
          <p className="muted">{texts.tour.padAt(Math.floor(target / 6) + 1, (target % 6) + 1)}</p>
          <p className="title">{targetKind ? nameAt(target) : texts.tour.emptyPad}</p>
        </div>
        <div className="tour-kinds">
          {(Object.keys(KINDS) as Kind[]).map((kind) => (
            <button
              key={kind}
              type="button"
              className={kind === targetKind ? "choice on" : "choice"}
              aria-pressed={kind === targetKind}
              onClick={() => {
                setPlaced({ ...placed, [target]: kind });
                pick(null);
              }}
            >
              <Tinted colour={KINDS[kind].colour}>{KINDS[kind].icon}</Tinted>
              <span className="name">{texts.tour.kinds[kind]}</span>
            </button>
          ))}
        </div>
        <div>
          <p className="muted">{texts.ledColour}</p>
          <Swatches colour={colours[target] ?? null} disabled={!targetKind} onChange={pick} />
        </div>
      </div>
    </>
  );
}

/** How long each step of the sample macro shows, sped up */
const STEP_TIMES = [700, 1700, 800, 700, 1300];

export function Macro({ pictures }: { pictures: Pictures | null }) {
  const texts = useTexts();
  const { steps, library } = useMemo(() => sampleMacro(texts), [texts]);
  // From 1 while it runs
  const [step, setStep] = useState(0);
  const running = step > 0 ? { id: "tour", step, steps: steps.length } : null;

  useEffect(() => {
    if (step === 0) return;
    const timer = setTimeout(() => setStep(step < steps.length ? step + 1 : 0), STEP_TIMES[step - 1]);
    return () => clearTimeout(timer);
  }, [step, steps.length]);

  return (
    <>
      <div className="tour-macro">
        <div className="macro-head">
          <span className="chip selected" aria-hidden="true">
            <span className="dot" style={{ "--led": rgb(MACRO_COLOUR) } as CSSProperties} />
            {padName(texts, MACRO_PAD)}
          </span>
          <button type="button" className={running ? "button" : "button outline"} onClick={() => setStep(running ? 0 : 1)}>
            {running ? <StopIcon /> : <PlayIcon />}
            {running ? texts.stopMacro : texts.tryMacro}
          </button>
        </div>
        <Timeline steps={steps} library={library} running={running} />
        <ol className="steps" aria-label={texts.stepsLabel}>
          {steps.map((item, i) => {
            const shown = describe(item, texts, library, []);
            const state = running ? (i + 1 < step ? "done" : i + 1 === step ? "now" : "") : "";
            return (
              <li key={i} className={`step ${state}`}>
                <div className="step-row">
                  <span className="number">{i + 1}</span>
                  <Tinted colour={shown.colour}>{shown.icon}</Tinted>
                  <span className="summary">
                    <span className="title">{shown.title}</span>
                    {shown.detail && <span className="muted">{shown.detail}</span>}
                  </span>
                  {state === "now" && <span className="badge now">{texts.runningNow}</span>}
                </div>
              </li>
            );
          })}
        </ol>
      </div>

      <div className="tour-macro-side">
        <div className="tour-bezel">
          <DisplayCanvas frame={(running ? pictures?.steps[step - 1] : pictures?.ready) ?? ""} className="tour-oled small" />
        </div>
        <p>
          <Cap led={MACRO_COLOUR} breathe={running !== null} small />
          {texts.tour.breathes}
        </p>
        <p className="muted">{texts.tour.macroOptions}</p>
      </div>
    </>
  );
}

const SIDEBAR: PageId[] = ["soundboard", "pages", "macros", "board", "overlay"];

/** The Settings page, where the tour stays */
export function AllSet() {
  const texts = useTexts();
  const nothing = () => {};
  return (
    <div className="tour-window" inert>
      <div className="app">
        <Sidebar
          pages={SIDEBAR}
          page="settings"
          connections={[
            { state: "ok", label: texts.boardConnected },
            { state: "ok", label: texts.voicemodConnected },
          ]}
          onSelect={nothing}
        />
        <main>
          <h1>{texts.nav.settings}</h1>
          <Section>
            <Row label={texts.language}>
              <Segmented label={texts.language} options={languages(texts)} value="system" onChange={nothing} />
            </Row>
            {WINDOWS && (
              <Row label={texts.startWithWindows} text={texts.startWithWindowsText}>
                <Toggle label={texts.startWithWindows} checked onChange={nothing} />
              </Row>
            )}
            <Row label={texts.tour.label} text={texts.tour.settingText}>
              <button type="button" className="button secondary calling">
                {texts.tour.watch}
              </button>
            </Row>
          </Section>
        </main>
      </div>
    </div>
  );
}
