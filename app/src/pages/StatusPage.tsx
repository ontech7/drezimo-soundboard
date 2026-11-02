import { type CSSProperties, type KeyboardEvent, type ReactNode, useEffect, useRef } from "react";
import { type BoardView, goToPage, isLit, ledColor, type PadView, pressKey, pressPad, useBoardView } from "../boardView";
import { ChevronLeft, ChevronRight, HearMeIcon, MuteMeIcon, StopIcon, VoiceChangerIcon } from "../icons";
import { PadPicture } from "../PadPicture";
import { type Texts, useTexts } from "../i18n";
import type { BoardStatus, FirmwareUpdate, Status } from "../status";

interface Line {
  dot: "waiting" | "warning" | "ok" | "error";
  title: string;
  badge?: string;
  detail: string;
  note?: string;
}

function boardLine(texts: Texts, status: BoardStatus, update: FirmwareUpdate | null): Line {
  if (update?.state === "updating") {
    return { dot: "warning", title: texts.updatingBoard, detail: texts.updatingDetail(update.from, update.to) };
  }
  if (update?.state === "restarting") {
    return { dot: "warning", title: texts.restartingBoard, detail: texts.restartingDetail(update.to) };
  }
  const note = update?.state === "failed" ? texts.updateFailed(update.reason) : undefined;
  return { ...connectionLine(texts, status), note };
}

function connectionLine(texts: Texts, status: BoardStatus): Line {
  switch (status.state) {
    case "searching":
      return { dot: "waiting", title: texts.searching, detail: texts.searchingDetail };
    case "busy":
      return { dot: "warning", title: texts.busy, detail: texts.busyDetail };
    case "connected":
      return { dot: "ok", title: texts.boardConnected, detail: "" };
  }
}

function voicemodLine(texts: Texts, status: Status): Line {
  const { voicemod, page, license } = status;
  switch (voicemod.state) {
    case "connecting":
      return { dot: "waiting", title: texts.connectingVoicemod, detail: "" };
    case "connected":
      return {
        dot: "ok",
        title: texts.voicemodConnected,
        badge: license ? texts.license(license) : undefined,
        detail: page ? "" : texts.loadingSoundboards,
      };
    case "unreachable":
      return { dot: "waiting", title: texts.voicemodUnreachable, detail: texts.voicemodUnreachableDetail };
    case "keyRejected":
      return { dot: "error", title: texts.keyRejected, detail: texts.keyRejectedDetail };
    case "missingKey":
      return { dot: "error", title: texts.missingKey, detail: texts.missingKeyDetail };
  }
}

function StatusLine({ line }: { line: Line }) {
  return (
    <div className="status">
      <span className={`dot ${line.dot}`} />
      <div>
        <p className="status-title">
          {line.title}
          {line.badge && <span className="badge">{line.badge}</span>}
        </p>
        {line.detail && <p className="muted">{line.detail}</p>}
        {line.note && <p className="note">{line.note}</p>}
      </div>
    </div>
  );
}

// The board's function keys but the page keys, which the page arrows stand for
const KEYS: { index: number; hint: keyof Texts["keyHints"]; icon: ReactNode }[] = [
  { index: 2, hint: "stop", icon: <StopIcon /> },
  { index: 3, hint: "muteMe", icon: <MuteMeIcon /> },
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
      {pad.name && <PadPicture voice={pad.voice} picture={pad.picture} action={pad.action} />}
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
            aria-pressed={index === 2 ? undefined : lit}
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

function Pads({ view }: { view: BoardView | null }) {
  const texts = useTexts();
  if (!view) {
    return (
      <section className="card">
        <p className="muted">{texts.noPages}</p>
      </section>
    );
  }
  const count = view.pages.length;
  return (
    <section className="card">
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
        <select aria-label={texts.soundboard} value={view.page} onChange={(event) => goToPage(Number(event.target.value))}>
          {view.pages.map((title, index) => (
            <option key={index} value={index}>
              {title}
            </option>
          ))}
        </select>
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

export function StatusPage({ status }: { status: Status }) {
  const texts = useTexts();
  const view = useBoardView();
  return (
    <>
      <div className="pair">
        <section className="card">
          <StatusLine line={boardLine(texts, status.board, status.firmwareUpdate)} />
        </section>
        <section className="card">
          <StatusLine line={voicemodLine(texts, status)} />
        </section>
      </div>

      <Pads view={view} />
    </>
  );
}
