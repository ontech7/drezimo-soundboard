import { type CSSProperties, type KeyboardEvent, useEffect, useRef } from "react";
import { type Activity, describe } from "../board";
import { type BoardView, goToPage, isLit, ledColor, type PadView, pressKey, pressPad, useBoardView } from "../boardView";
import { DisplayPreview } from "../DisplayPreview";
import { ChevronLeft, ChevronRight } from "../icons";
import { type Texts, useTexts } from "../i18n";
import type { BoardStatus, FirmwareUpdate, PageSummary, Status } from "../status";

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
      return { dot: "ok", title: texts.boardConnected, detail: `Firmware ${status.firmware} · ${status.port}` };
  }
}

function voicemodLine(texts: Texts, status: Status): Line {
  const { voicemod, page, voicemodLog: fromLog, license } = status;
  switch (voicemod.state) {
    case "connecting":
      return { dot: "waiting", title: texts.connectingVoicemod, detail: "" };
    case "connected":
      return {
        dot: "ok",
        title: texts.voicemodConnected,
        badge: license ? texts.license(license) : undefined,
        detail: page ? pageText(texts, page) : texts.loadingSoundboards,
        note: fromLog ? undefined : texts.logUnreadable,
      };
    case "unreachable":
      return { dot: "waiting", title: texts.voicemodUnreachable, detail: texts.voicemodUnreachableDetail };
    case "keyRejected":
      return { dot: "error", title: texts.keyRejected, detail: texts.keyRejectedDetail };
    case "missingKey":
      return { dot: "error", title: texts.missingKey, detail: texts.missingKeyDetail };
  }
}

function pageText(texts: Texts, page: PageSummary): string {
  const name = page.parts > 1 ? `${page.name} ${page.part}/${page.parts}` : page.name;
  return texts.page(name, page.number, page.count);
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

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

// STOP, MUTE ME, VOICE, HEAR ME: the page keys sit above the pads instead
const KEYS = [2, 3, 4, 5];

function Pad({ index, pad }: { index: number; pad: PadView }) {
  const texts = useTexts();
  const classes = ["pad", pad.led.effect, isLit(pad.led) ? "lit" : "", pad.locked ? "locked" : ""];
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
      title={pad.locked ? texts.locked : undefined}
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
      <span>{pad.name}</span>
    </button>
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
      </div>
      <div className="pads">
        {view.pads.map((pad, index) => (
          <Pad key={index} index={index} pad={pad} />
        ))}
      </div>
    </section>
  );
}

function Keys({ view }: { view: BoardView | null }) {
  const texts = useTexts();
  return (
    <section className="card">
      <h2>{texts.keys}</h2>
      <div className="keys">
        {KEYS.map((index) => {
          const led = view?.keys[index];
          const lit = led !== undefined && isLit(led);
          return (
            <button
              key={index}
              type="button"
              className={lit ? "key lit" : "key"}
              // MUTE ME, VOICE and HEAR ME are switches, lit while on
              aria-pressed={index === 2 ? undefined : lit}
              style={led ? ({ "--led": ledColor(led) } as CSSProperties) : undefined}
              disabled={!view}
              onClick={() => pressKey(index)}
            >
              {texts.buttonNames[index]}
            </button>
          );
        })}
      </div>
    </section>
  );
}

export function StatusPage({ status, activity }: { status: Status; activity: Activity[] }) {
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

      <div className="pair">
        <section className="card">
          <h2>{texts.display}</h2>
          <DisplayPreview />
        </section>
        <Keys view={view} />
      </div>

      <section className="card activity">
        <h2>{texts.activity}</h2>
        {activity.length === 0 ? (
          <p className="muted">{texts.noActivity}</p>
        ) : (
          <ul>
            {activity.map((item) => (
              <li key={item.id}>
                <span>{describe(item.message, texts)}</span>
                <time className="muted">{timeFormat.format(item.at)}</time>
              </li>
            ))}
          </ul>
        )}
      </section>
    </>
  );
}
