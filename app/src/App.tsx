import { useEffect } from "react";
// Inline markup rather than an <img>: nothing left to load once the page is built
import logo from "./assets/drezimo-logo.svg?raw";
import { type BoardStatus, useBoardActivity, useBoardStatus } from "./board";
import { windowReady } from "./window";

const STATUS_TITLES: Record<BoardStatus["state"], string> = {
  searching: "Looking for the board…",
  busy: "Board in use by another program",
  connected: "Connected",
};

function statusDetail(status: BoardStatus): string {
  switch (status.state) {
    case "searching":
      return "Plug the board into a USB port.";
    case "busy":
      return "Close the other program and the app connects by itself.";
    case "connected":
      return `Firmware ${status.firmware} · ${status.port}`;
  }
}

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

export default function App() {
  const status = useBoardStatus();
  const activity = useBoardActivity(12);

  // The first frame waits for the board status, so the window never shows a state that changes at once
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
        <div className="status">
          <span className={`dot ${status.state}`} />
          <div>
            <p className="status-title">{STATUS_TITLES[status.state]}</p>
            <p className="muted">{statusDetail(status)}</p>
          </div>
        </div>
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
