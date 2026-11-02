import { type Activity, describe } from "../board";
import { DisplayPreview } from "../DisplayPreview";
import { type Texts, useTexts } from "../i18n";
import type { BoardStatus, FirmwareUpdate, PageSummary, Status, VoicemodStatus } from "../status";

interface Line {
  dot: "waiting" | "warning" | "ok" | "error";
  title: string;
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

function voicemodLine(texts: Texts, status: VoicemodStatus, page: PageSummary | null, fromLog: boolean): Line {
  switch (status.state) {
    case "connecting":
      return { dot: "waiting", title: texts.connectingVoicemod, detail: "" };
    case "connected":
      return {
        dot: "ok",
        title: texts.voicemodConnected,
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
        <p className="status-title">{line.title}</p>
        {line.detail && <p className="muted">{line.detail}</p>}
        {line.note && <p className="note">{line.note}</p>}
      </div>
    </div>
  );
}

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

export function StatusPage({ status, activity }: { status: Status; activity: Activity[] }) {
  const texts = useTexts();
  return (
    <>
      <div className="pair">
        <section className="card">
          <h2>{texts.board}</h2>
          <StatusLine line={boardLine(texts, status.board, status.firmwareUpdate)} />
        </section>
        <section className="card">
          <h2>{texts.voicemod}</h2>
          <StatusLine line={voicemodLine(texts, status.voicemod, status.page, status.voicemodLog)} />
        </section>
      </div>

      <section className="card">
        <h2>{texts.display}</h2>
        <DisplayPreview />
      </section>

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
