import { useEffect } from "react";
// Inline markup rather than an <img>: nothing left to load once the page is built
import logo from "./assets/drezimo-logo.svg?raw";
import { useBoardActivity } from "./board";
import { DisplayPreview } from "./DisplayPreview";
import { type BoardStatus, type FirmwareUpdate, type PageSummary, type VoicemodStatus, useStatus } from "./status";
import { windowReady } from "./window";

interface Line {
  dot: "waiting" | "warning" | "ok" | "error";
  title: string;
  detail: string;
  note?: string;
}

function boardLine(status: BoardStatus, update: FirmwareUpdate | null): Line {
  if (update?.state === "updating") {
    return {
      dot: "warning",
      title: "Updating the board…",
      detail: `Firmware ${update.from} → ${update.to}. Keep it plugged in.`,
    };
  }
  if (update?.state === "restarting") {
    return { dot: "warning", title: "Restarting the board…", detail: `With firmware ${update.to}.` };
  }
  const note = update?.state === "failed" ? `Update failed: ${update.reason} Tried again when the app starts.` : undefined;
  return { ...connectionLine(status), note };
}

function connectionLine(status: BoardStatus): Line {
  switch (status.state) {
    case "searching":
      return { dot: "waiting", title: "Looking for the board…", detail: "Plug the board into a USB port." };
    case "busy":
      return {
        dot: "warning",
        title: "Board in use by another program",
        detail: "Close the other program and the app connects by itself.",
      };
    case "connected":
      return { dot: "ok", title: "Connected", detail: `Firmware ${status.firmware} · ${status.port}` };
  }
}

function voicemodLine(status: VoicemodStatus, page: PageSummary | null, fromLog: boolean): Line {
  switch (status.state) {
    case "connecting":
      return { dot: "waiting", title: "Connecting to Voicemod…", detail: "" };
    case "connected":
      return {
        dot: "ok",
        title: "Connected",
        detail: page ? pageText(page) : "Loading the soundboards…",
        note: fromLog ? undefined : "Voicemod's log cannot be read: the lengths of the sounds are estimated.",
      };
    case "unreachable":
      return {
        dot: "waiting",
        title: "Voicemod is not running",
        detail: "Open Voicemod and the app connects by itself.",
      };
    case "keyRejected":
      return { dot: "error", title: "Voicemod refused the connection", detail: "Update the app to the latest version." };
    case "missingKey":
      return { dot: "error", title: "No Voicemod key", detail: "This build was made without VOICEMOD_CLIENT_KEY." };
  }
}

function pageText(page: PageSummary): string {
  const name = page.parts > 1 ? `${page.name} ${page.part}/${page.parts}` : page.name;
  return `${name} · page ${page.number} of ${page.count}`;
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

export default function App() {
  const status = useStatus();
  const activity = useBoardActivity(12);

  // The first frame waits for the status, so the window never shows a state that changes at once
  useEffect(() => {
    if (status) windowReady();
  }, [status]);

  if (!status) return null;

  return (
    <main>
      <header>
        <div className="logo" role="img" aria-label="Drezimo Soundboard" dangerouslySetInnerHTML={{ __html: logo }} />
      </header>

      <section className="card">
        <h2>Board</h2>
        <StatusLine line={boardLine(status.board, status.firmwareUpdate)} />
      </section>

      <section className="card">
        <h2>Voicemod</h2>
        <StatusLine line={voicemodLine(status.voicemod, status.page, status.voicemodLog)} />
      </section>

      <section className="card">
        <h2>Display</h2>
        <DisplayPreview />
      </section>

      <section className="card activity">
        <h2>Activity</h2>
        {activity.length === 0 ? (
          <p className="muted">Press a key or turn the knob.</p>
        ) : (
          <ul>
            {activity.map((item) => (
              <li key={item.id}>
                <span>{item.text}</span>
                <time className="muted">{timeFormat.format(item.at)}</time>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
