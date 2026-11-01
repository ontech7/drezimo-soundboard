import { useEffect } from "react";
// Inline markup rather than an <img>: nothing left to load once the page is built
import logo from "./assets/drezimo-logo.svg?raw";
import { useBoardActivity } from "./board";
import { type BoardStatus, type PageSummary, type VoicemodStatus, useStatus } from "./status";
import { windowReady } from "./window";

interface Line {
  dot: "waiting" | "warning" | "ok" | "error";
  title: string;
  detail: string;
}

function boardLine(status: BoardStatus): Line {
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

function voicemodLine(status: VoicemodStatus, page: PageSummary | null): Line {
  switch (status.state) {
    case "connecting":
      return { dot: "waiting", title: "Connecting to Voicemod…", detail: "" };
    case "connected":
      return { dot: "ok", title: "Connected", detail: page ? pageText(page) : "Loading the soundboards…" };
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
        <StatusLine line={boardLine(status.board)} />
      </section>

      <section className="card">
        <h2>Voicemod</h2>
        <StatusLine line={voicemodLine(status.voicemod, status.page)} />
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
