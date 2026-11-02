import { type Activity, describe } from "../board";
import { Row, Section } from "../controls";
import { DisplayPreview } from "../DisplayPreview";
import { useTexts } from "../i18n";
import type { Status } from "../status";
import { BoardTestCard } from "./BoardTestCard";

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

export function DiagnosticsPage({ status, activity }: { status: Status; activity: Activity[] }) {
  const texts = useTexts();
  const { board } = status;
  const connected = status.voicemod.state === "connected";
  return (
    <>
      <Section title={texts.connections}>
        <Row label={texts.boardPort}>
          <span className="value">{board.state === "searching" ? "—" : board.port}</span>
        </Row>
        <Row label={texts.boardFirmware}>
          <span className="value">{board.state === "connected" ? board.firmware : "—"}</span>
        </Row>
        <Row label={texts.voicemodLog} text={connected && !status.voicemodLog ? texts.logUnreadable : undefined}>
          <span className="value">{connected ? (status.voicemodLog ? texts.logRead : texts.logNotRead) : "—"}</span>
        </Row>
      </Section>

      <BoardTestCard status={status} />

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
