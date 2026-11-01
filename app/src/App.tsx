import logo from "./assets/drezimo-logo.svg";
import { useBoardActivity, useBoardStatus } from "./board";

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

export default function App() {
  const status = useBoardStatus();
  const activity = useBoardActivity(12);
  const connected = status?.connected ?? false;

  return (
    <main>
      <header>
        <img className="logo" src={logo} alt="Drezimo Soundboard" />
      </header>

      <section className="card">
        <h2>Board</h2>
        <div className="status">
          <span className={`dot ${connected ? "on" : ""}`} />
          <div>
            <p className="status-title">{connected ? "Connected" : "Looking for the board…"}</p>
            <p className="muted">
              {connected ? `Firmware ${status?.firmware} · ${status?.port}` : "Plug the board into a USB port."}
            </p>
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
