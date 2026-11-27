import { invoke } from "@tauri-apps/api/core";
import { type CSSProperties, type KeyboardEvent, type ReactNode, useEffect, useRef } from "react";
import { type BoardView, goToPage, isLit, ledColor, type PadView, pressKey, pressPad } from "../boardView";
import { ChevronLeft, ChevronRight, CloseIcon, EditIcon, HearMeIcon, MuteMeIcon, StopIcon, VoiceChangerIcon } from "../icons";
import { PadPicture } from "../PadPicture";
import { type Texts, useTexts } from "../i18n";
import { PAD_COUNT, type Rgb, UNAVAILABLE_COLOUR } from "../customPages";
import { MiniBoard } from "../MiniBoard";
import { saveSettings, type Settings } from "../settings";
import type { Status } from "../status";
import { useNews } from "../updates";
import { SearchSelect } from "../SearchSelect";
import { holdWithShift } from "../VirtualKnob";

interface State {
  dot: "ok" | "off" | "waiting" | "warning" | "error";
  /** In a few words, for the dot's tooltip */
  label: string;
  /** What it means, and what to do */
  line: string;
}

/** Drezimo's state and what to do about it, beside its display: the sidebar says only whether it is connected */
function boardState(texts: Texts, status: Status, playable: boolean): State {
  const { board, firmwareUpdate: update } = status;
  const line = texts.boardLine;
  if (update?.state === "updating") {
    return { dot: "waiting", label: texts.updatingBoard, line: line.updating(update.from, update.to) };
  }
  if (update?.state === "restarting") {
    return { dot: "waiting", label: texts.restartingBoard, line: line.restarting(update.to) };
  }
  if (board.state === "busy") return { dot: "warning", label: texts.busy, line: line.busy };
  if (board.state === "searching") {
    return { dot: "off", label: texts.searching, line: playable ? line.searchingPlayHere : line.searching };
  }
  if (update?.state === "failed") return { dot: "error", label: texts.boardConnected, line: line.failed(update.reason) };
  return { dot: "ok", label: texts.boardConnected, line: playable ? line.connected : line.connectedOnly };
}

/** The Soundboard page's title, with Drezimo's state: a dot beside it, what it means below */
export function SoundboardTitle({ status, playable }: { status: Status; playable: boolean }) {
  const texts = useTexts();
  const { dot, label, line } = boardState(texts, status, playable);
  return (
    <div className="page-title">
      <h1>
        {texts.nav.soundboard}
        <span className={`dot ${dot}`} role="img" aria-label={label} title={label} />
      </h1>
      <p className="muted">{line}</p>
    </div>
  );
}

/** What the version just installed brings, once */
function WhatIsNew() {
  const texts = useTexts();
  const [news, dismiss] = useNews();
  if (!news) return null;
  return (
    <div className="notice news">
      <span className="dot ok" />
      <p>
        <strong>{texts.updatedTo(news.version)}</strong>
        {news.notes && <span className="muted">{news.notes}</span>}
      </p>
      <button type="button" className="icon-button small" aria-label={texts.close} title={texts.close} onClick={dismiss}>
        <CloseIcon />
      </button>
    </div>
  );
}

// The function keys, as the board numbers them
const STOP = 2;
const MUTE_ME = 3;

/**
 * Only the others hear the sounds: easy to miss, as Voicemod's switch for it is out of sight, and its mixer's volume
 * for the user too
 */
function MutedForMe({ settings, cause }: { settings: Settings; cause: "switch" | "volume" }) {
  const texts = useTexts();
  // A volume at zero is no choice of the user's to remember: closed until the app starts again
  const seen = () =>
    cause === "switch"
      ? saveSettings({ ...settings, muteForMeNoticeSeen: true })
      : invoke("close_volume_notice").catch((error) => console.error("cannot close the notice", error));
  if (cause === "switch" && settings.muteForMeNoticeSeen) return null;
  return (
    <div className="notice warning">
      <span className="dot warning" />
      <p>
        <strong>{texts.mutedForMe}</strong>
        <span className="muted">{cause === "switch" ? texts.mutedForMeDetail : texts.mutedForMeVolume}</span>
      </p>
      <div className="notice-actions">
        <button type="button" className="button secondary" onClick={seen}>
          {texts.fineAsItIs}
        </button>
        <button
          type="button"
          className="button"
          onClick={() => {
            invoke("hear_sounds_again").catch((error) => console.error("cannot bring the sounds back", error));
            seen();
          }}
        >
          {texts.hearAgain}
        </button>
      </div>
    </div>
  );
}

const OFF: null[] = Array(PAD_COUNT).fill(null);
// A soft shimmer over the dark pads, while waiting
const WAITING: Rgb = [120, 120, 130];
const ALL = (colour: Rgb) => OFF.map(() => colour);

interface Waiting {
  title: string;
  text: string;
  leds: (Rgb | null)[];
  effect?: "blink" | "wave";
}

/** Why there are no pads to play yet, with the board as it looks meanwhile */
function waiting(texts: Texts, status: Status): Waiting {
  switch (status.voicemod.state) {
    case "connecting":
      return { title: texts.connectingVoicemod, text: texts.connectingText, leds: ALL(WAITING), effect: "wave" };
    case "unreachable":
      return { title: texts.openVoicemod, text: texts.openVoicemodText, leds: ALL(UNAVAILABLE_COLOUR), effect: "blink" };
    case "keyRejected":
      return { title: texts.keyRejected, text: texts.keyRejectedDetail, leds: ALL(UNAVAILABLE_COLOUR), effect: "blink" };
    case "missingKey":
      return { title: texts.missingKey, text: texts.missingKeyDetail, leds: ALL(UNAVAILABLE_COLOUR), effect: "blink" };
    case "connected":
      return status.loaded
        ? { title: texts.nothingToPlay, text: texts.nothingToPlayText, leds: OFF }
        : { title: texts.loadingSoundboards, text: texts.loadingText, leds: ALL(WAITING), effect: "wave" };
  }
}

function Waiting({ status, onMakePage }: { status: Status; onMakePage: () => void }) {
  const texts = useTexts();
  const shown = waiting(texts, status);
  const empty = status.voicemod.state === "connected" && status.loaded;
  return (
    <section className="card waiting-card">
      <MiniBoard leds={shown.leds} effect={shown.effect} solid />
      <div>
        <p className="status-title">{shown.title}</p>
        <p className="muted">{shown.text}</p>
        {empty && (
          <button type="button" className="button" onClick={onMakePage}>
            {texts.makePage}
          </button>
        )}
      </div>
    </section>
  );
}

// The board's function keys but the page keys, which the page arrows stand for
const KEYS: { index: number; hint: keyof Texts["keyHints"]; icon: ReactNode }[] = [
  { index: STOP, hint: "stop", icon: <StopIcon /> },
  { index: MUTE_ME, hint: "muteMe", icon: <MuteMeIcon /> },
  { index: 4, hint: "voice", icon: <VoiceChangerIcon /> },
  { index: 5, hint: "hearMe", icon: <HearMeIcon /> },
];

function Pad({ index, pad }: { index: number; pad: PadView }) {
  const texts = useTexts();
  const classes = ["pad", pad.led.effect, isLit(pad.led) ? "lit" : "", pad.locked ? "locked" : "", pad.voice ? "voice" : ""];
  // Pressed and released as on the board, for the sounds that play while held: only what this pad pressed is released
  const pressed = useRef(false);
  const press = (down: boolean) => {
    if (pressed.current === down) return;
    pressed.current = down;
    pressPad(index, down);
  };
  const isKey = (event: KeyboardEvent<HTMLButtonElement>) => event.key === "Enter" || event.key === " ";

  // A press never outlives the pad, nor the window's focus or showing: a held sound would play on
  useEffect(() => {
    const release = () => {
      if (!pressed.current) return;
      pressed.current = false;
      pressPad(index, false);
    };
    window.addEventListener("blur", release);
    document.addEventListener("visibilitychange", release);
    return () => {
      window.removeEventListener("blur", release);
      document.removeEventListener("visibilitychange", release);
      release();
    };
  }, [index]);

  return (
    <button
      type="button"
      className={classes.join(" ")}
      style={{ "--led": ledColor(pad.led) } as CSSProperties}
      disabled={!pad.name}
      aria-hidden={!pad.name}
      title={pad.locked ? `${pad.name}\n${texts.locked}` : pad.name || undefined}
      onPointerDown={(event) => {
        if (event.button !== 0 || !pad.name) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        // Shift holds the knob, as a hand on it: the pad is then peeked
        holdWithShift(event);
        press(true);
      }}
      onPointerUp={() => press(false)}
      onPointerCancel={() => press(false)}
      onLostPointerCapture={() => press(false)}
      onBlur={() => press(false)}
      onKeyDown={(event) => {
        if (!isKey(event)) return;
        event.preventDefault();
        if (!event.repeat) press(true);
      }}
      onKeyUp={(event) => {
        if (isKey(event)) press(false);
      }}
    >
      {pad.name && <PadPicture voice={pad.voice} picture={pad.picture} action={pad.action} macro={pad.macro} />}
      <span className="name">{pad.name}</span>
    </button>
  );
}

function Keys({ view }: { view: BoardView }) {
  const texts = useTexts();
  return (
    <div className="function-keys">
      {KEYS.map(({ index, hint, icon }) => {
        const led = view.keys[index];
        const lit = isLit(led);
        return (
          <button
            key={index}
            type="button"
            className={lit ? "icon-button key lit" : "icon-button key"}
            aria-label={texts.keyHints[hint]}
            title={texts.keyHints[hint]}
            // MUTE ME, VOICE and HEAR ME are switches, lit while on
            aria-pressed={index === STOP ? undefined : lit}
            style={{ "--led": ledColor(led) } as CSSProperties}
            onClick={() => pressKey(index)}
          >
            {icon}
          </button>
        );
      })}
    </div>
  );
}

function Pads({ view, onEditPage }: { view: BoardView; onEditPage: (id: string) => void }) {
  const texts = useTexts();
  const count = view.pages.length;
  const own = view.page < view.own;
  return (
    <section className="card soundboard">
      <div className="pages">
        <button
          type="button"
          className="icon-button"
          aria-label={texts.previousPage}
          disabled={count < 2}
          onClick={() => goToPage((view.page - 1 + count) % count)}
        >
          <ChevronLeft />
        </button>
        <SearchSelect
          label={texts.soundboard}
          options={view.pages.map((title, index) => ({ value: index, label: title }))}
          value={view.page}
          onChange={goToPage}
          searchLabel={texts.searchPage}
          empty={texts.noPage}
        />
        <button
          type="button"
          className="icon-button"
          aria-label={texts.nextPage}
          disabled={count < 2}
          onClick={() => goToPage((view.page + 1) % count)}
        >
          <ChevronRight />
        </button>
        <span className="muted counter">
          {view.page + 1}/{count}
        </span>
        {own && (
          <button
            type="button"
            className="icon-button"
            aria-label={texts.editPage}
            title={texts.editPage}
            onClick={() => onEditPage(view.ids[view.page])}
          >
            <EditIcon />
          </button>
        )}
        <Keys view={view} />
      </div>
      <div className="pads">
        {view.pads.map((pad, index) => (
          <Pad key={index} index={index} pad={pad} />
        ))}
      </div>
    </section>
  );
}

interface Props {
  status: Status;
  settings: Settings;
  view: BoardView | null | undefined;
  onMakePage: () => void;
  /** Opens a page of the user's own in the Pages editor */
  onEditPage: (id: string) => void;
}

export function SoundboardPage({ status, settings, view, onMakePage, onEditPage }: Props) {
  // The pads not known yet, or not there yet while the status already has a page: nothing, rather than a wrong state
  const settling = view === undefined || (view === null && status.voicemod.state === "connected" && status.page !== null);
  return (
    <>
      <WhatIsNew />
      {status.mutedPlay && <MutedForMe settings={settings} cause={status.mutedPlay} />}
      {view ? <Pads view={view} onEditPage={onEditPage} /> : !settling && <Waiting status={status} onMakePage={onMakePage} />}
    </>
  );
}
