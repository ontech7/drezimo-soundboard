import { type CSSProperties, type ReactNode, useEffect, useState } from "react";
import { ACTION_COLOURS, MACRO_COLOUR, type Rgb, rgb, SOUND_COLOUR, VOICE_COLOUR } from "../customPages";
import type { Beats } from "./KnobHalf";
import { Half } from "./Stage";
import { held, type Lane, SoundLanes } from "./SoundLanes";
import type { Shown } from "./story";
import type { ExplainTexts, SwitchAction } from "./texts";

interface Props {
  shown: Shown;
  /** The action's name, under the half until the gesture acts */
  name: string;
  beats: Beats;
  words: ExplainTexts;
  macro: { name: string; steps: string[]; again: "stop" | "restart" } | null;
  pageName: string | null;
  /** A hold-turn's knob let go: what it showed while held goes */
  released: boolean;
}

/** The right half of a knob explanation; mounted again for each loop */
export function ActionHalf(props: Props) {
  const { shown, name, beats, words } = props;
  const half = (caption: string | null, scene: ReactNode) => (
    <Half caption={caption ?? name} live={caption !== null}>
      {scene}
    </Half>
  );
  const acted = beats.count > 0;

  if (shown.kind === "turn") {
    switch (shown.action) {
      case "page": {
        const pages = words.sample.pages;
        const at = clamp(beats.sum, 0, pages.length - 1);
        return half(acted ? words.onPage(pages[at]) : null, <Display words={words} pages={pages} at={at} />);
      }
      case "rows": {
        const row = acted && !props.released ? clamp(beats.sum - 1, 0, 5) : null;
        return half(row === null ? null : words.onRow(row + 1), <Pads row={row} />);
      }
      case "soundboardForOthers":
      case "soundboardForMe":
      case "microphone":
        return half(null, <Volume icon={shown.action} label={words.volumes[shown.action]} level={60 + 5 * beats.sum} />);
      case "playingSound":
        return half(null, <Volume icon="playingSound" label={words.sample.sounds[0]} level={60 + 5 * beats.sum} />);
      case "voice": {
        const at = clamp(1 + beats.sum, 0, words.sample.voices.length - 1);
        return half(acted ? words.onVoice(words.sample.voices[at]) : null, <Voices voices={words.sample.voices} at={at} />);
      }
      case "nothing":
        return half(null, <Nothing words={words} />);
    }
  }

  switch (shown.action) {
    case "menu": {
      // A tap with the menu open picks the page it shows and closes it
      const open = beats.count % 2 === 1;
      const caption = acted ? (open ? words.menuOpen : words.menuPicked) : null;
      return half(caption, <Display words={words} pages={words.sample.pages} at={0} menu={open} />);
    }
    case "stopSounds":
      return half(acted ? words.allStopped : null, <StopAll words={words} stopped={acted} />);
    case "muteMic":
    case "muteForMe":
    case "voiceChanger":
    case "hearMyself":
    case "background": {
      // From one loop to the next, so that it goes back and forth
      const on = beats.total % 2 === 0;
      const state = words.switches[shown.action][on ? 0 : 1];
      return half(acted ? state : null, <Switch action={shown.action} on={on} state={state} />);
    }
    case "randomVoice":
      return <RandomVoice {...props} />;
    case "macro":
      return <MacroSteps {...props} />;
    case "page": {
      const pages = [...words.sample.pages];
      pages[2] = props.pageName ?? pages[2];
      return half(
        acted ? words.onPage(pages[2]) : null,
        <Display words={words} pages={pages} at={acted ? 2 : 0} flash={acted} />
      );
    }
    case "resetVolumes":
      return half(acted ? words.volumesBack : null, <ResetVolumes words={words} back={acted} />);
    case "nothing":
      return half(null, <Nothing words={words} />);
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/** Drezimo's display, with the page's name sliding and the soundboard menu */
function Display(props: { words: ExplainTexts; pages: string[]; at: number; menu?: boolean; flash?: boolean }) {
  const { words, pages, at } = props;
  return (
    <div>
      <div className={props.flash ? "explain-oled flash" : "explain-oled"}>
        <small>{words.pageCounter(at + 1, pages.length)}</small>
        <div className="explain-oled-names">
          <div className="explain-oled-strip" style={{ translate: `${-144 * at}px 0` }}>
            {pages.map((page, i) => (
              <span key={i}>{page}</span>
            ))}
          </div>
        </div>
        <div className={props.menu ? "explain-oled-menu open" : "explain-oled-menu"}>
          <small>{words.menuTitle}</small>
          {pages.slice(0, 3).map((page, i) => (
            <p key={i} className={i === 0 ? "selected" : undefined}>
              {page}
            </p>
          ))}
        </div>
      </div>
      <div className="explain-dots">
        {pages.map((_, i) => (
          <i key={i} className={i === at ? "on" : undefined} />
        ))}
      </div>
    </div>
  );
}

// The tour's kinds of pads, row by row: sounds, voices, macros and Stop
const LAYOUT = "SSSSSS" + "SSSSSS" + "VVVVVV" + "MSSRSM" + "SSSS.." + "......";
const LEDS: Record<string, Rgb> = { S: SOUND_COLOUR, V: VOICE_COLOUR, M: MACRO_COLOUR, R: ACTION_COLOURS.stopSounds };

function Pads({ row }: { row: number | null }) {
  return (
    <div className={row === null ? "explain-pads" : "explain-pads rowing"}>
      {[...LAYOUT].map((letter, i) => {
        const led = LEDS[letter];
        const white = led && Math.floor(i / 6) === row;
        const style = led ? ({ "--led": white ? "#fff" : rgb(led) } as CSSProperties) : undefined;
        return <span key={i} className={led ? (white ? "cap lit white" : "cap lit") : "cap"} style={style} />;
      })}
    </div>
  );
}

const ICONS = {
  soundboardForOthers: (
    <>
      <path d="M4 9v6h4l5 4V5L8 9H4z" />
      <path d="M16.5 8.5a5 5 0 0 1 0 7" />
      <path d="M19 6a8.5 8.5 0 0 1 0 12" />
    </>
  ),
  soundboardForMe: (
    <>
      <path d="M4 15v-3a8 8 0 0 1 16 0v3" />
      <rect x="3" y="14" width="4" height="6" rx="1.5" />
      <rect x="17" y="14" width="4" height="6" rx="1.5" />
    </>
  ),
  playingSound: (
    <>
      <path d="M9 18V6l11-2v12" />
      <circle cx="6.5" cy="18" r="2.5" />
      <circle cx="17.5" cy="16" r="2.5" />
    </>
  ),
  microphone: (
    <>
      <rect x="9" y="3" width="6" height="11" rx="3" />
      <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
    </>
  ),
  voice: <path d="M4 12h2M8 8v8M12 5v14M16 8v8M20 11v2" />,
  ear: (
    <>
      <path d="M7 10a5 5 0 0 1 10 0c0 3-3 4-3 7a3 3 0 0 1-5 2" />
      <path d="M10 10a2 2 0 0 1 4 0" />
    </>
  ),
  waves: <path d="M3 9c2-2 4-2 6 0s4 2 6 0 4-2 6 0M3 15c2-2 4-2 6 0s4 2 6 0 4-2 6 0" />,
};

function Icon({ children }: { children: ReactNode }) {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true">
      {children}
    </svg>
  );
}

function Volume({ icon, label, level }: { icon: keyof typeof ICONS; label: string; level: number }) {
  const shown = clamp(level, 0, 100);
  return (
    <div className="explain-volume">
      <div className="explain-volume-head">
        <Icon>{ICONS[icon]}</Icon>
        <span>{label}</span>
        <b>{shown}%</b>
      </div>
      <div className="explain-bar">
        <i style={{ width: `${shown}%` }} />
      </div>
    </div>
  );
}

function Voices({ voices, at }: { voices: string[]; at: number }) {
  return (
    <div className="explain-voices">
      <div style={{ translate: `0 ${44 - at * 44}px` }}>
        {voices.map((voice, i) => (
          <p key={i} className={i === at ? "on" : undefined}>
            {voice}
          </p>
        ))}
      </div>
      <span className="explain-voices-frame" />
    </div>
  );
}

const SWITCH_ICONS: Record<SwitchAction, keyof typeof ICONS> = {
  muteMic: "microphone",
  muteForMe: "soundboardForMe",
  voiceChanger: "voice",
  hearMyself: "ear",
  background: "waves",
};

function Switch({ action, on, state }: { action: SwitchAction; on: boolean; state: string }) {
  return (
    <div className={on ? "explain-switch" : "explain-switch off"}>
      <div className="explain-disc">
        <Icon>{ICONS[SWITCH_ICONS[action]]}</Icon>
        <span className="explain-slash" />
      </div>
      <span className="explain-pill">{state}</span>
    </div>
  );
}

function StopAll({ words, stopped }: { words: ExplainTexts; stopped: boolean }) {
  const [lanes, setLanes] = useState<Lane[]>(() => {
    const now = performance.now();
    return words.sample.sounds.map((name, i) => ({
      id: i,
      name,
      length: 9000,
      played: 1200 + i * 1500,
      since: now,
      state: "playing",
    }));
  });
  useEffect(() => {
    if (!stopped) return;
    const now = performance.now();
    setLanes((current) => current.map((lane) => held(lane, now, "stopped")));
  }, [stopped]);
  return <SoundLanes lanes={lanes} />;
}

function RandomVoice({ name, beats, words }: Props) {
  const voices = words.sample.voices;
  const [at, setAt] = useState(1);
  const [landed, setLanded] = useState<string | null>(null);
  useEffect(() => {
    if (beats.count === 0) return;
    const picked = 1 + Math.floor(Math.random() * (voices.length - 1));
    const land = () => {
      setAt(picked);
      setLanded(voices[picked]);
    };
    // With less motion asked for, it lands at once instead of spinning
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
      land();
      return;
    }
    const timers: number[] = [];
    let delay = 0;
    for (let i = 0; i < 7; i++) {
      delay += 70 + i * 22;
      timers.push(window.setTimeout(() => setAt((current) => 1 + (current % (voices.length - 1))), delay));
    }
    timers.push(window.setTimeout(land, delay + 120));
    return () => timers.forEach((timer) => clearTimeout(timer));
  }, [beats.count, voices]);
  return (
    <Half caption={landed ? words.onVoice(landed) : name} live={landed !== null}>
      <Voices voices={voices} at={at} />
    </Half>
  );
}

function MacroSteps({ name, beats, words, macro }: Props) {
  const shown = macro ?? words.sample.macro;
  const steps = shown.steps.length;
  // Pressed again while it runs, a macro stops or starts over, as it is set
  const stopped = beats.count > 1 && (macro?.again ?? "stop") === "stop";
  const [step, setStep] = useState(-1);
  useEffect(() => {
    if (beats.count === 0 || stopped) return;
    const timers = Array.from({ length: steps + 1 }, (_, i) => window.setTimeout(() => setStep(i), i * 380));
    return () => timers.forEach((timer) => clearTimeout(timer));
  }, [beats.count, steps, stopped]);
  const done = step >= steps;
  const caption = stopped ? words.macroStopped : done ? words.macroDone : null;
  return (
    <Half caption={caption ?? name} live={caption !== null}>
      <div className="explain-steps">
        <h4>{shown.name}</h4>
        {shown.steps.map((label, i) => (
          <p key={i} className={i < step ? "done" : i === step && !stopped ? "running" : undefined}>
            <span>{label}</span>
          </p>
        ))}
      </div>
    </Half>
  );
}

const RESET: { now: number; saved: number }[] = [
  { now: 35, saved: 80 },
  { now: 90, saved: 55 },
  { now: 20, saved: 70 },
];

function ResetVolumes({ words, back }: { words: ExplainTexts; back: boolean }) {
  return (
    <div className="explain-reset">
      {RESET.map((volume, i) => {
        const level = back ? volume.saved : volume.now;
        return (
          <div key={i}>
            <span>{words.resetLabels[i]}</span>
            <div className="explain-bar">
              <i style={{ width: `${level}%` }} />
            </div>
            <b>{level}%</b>
          </div>
        );
      })}
    </div>
  );
}

function Nothing({ words }: { words: ExplainTexts }) {
  return (
    <p className="explain-nothing">
      <b>—</b>
      {words.nothing}
    </p>
  );
}
