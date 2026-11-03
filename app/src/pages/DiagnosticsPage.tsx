import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
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
      <Section title={texts.support}>
        <Row label={texts.report} text={texts.reportText}>
          <CopyReport />
        </Row>
      </Section>

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

type Copying = "idle" | "copied" | "failed";

function CopyReport() {
  const texts = useTexts();
  const [copying, setCopying] = useState<Copying>("idle");

  useEffect(() => {
    if (copying === "idle") return;
    const timer = setTimeout(() => setCopying("idle"), 2000);
    return () => clearTimeout(timer);
  }, [copying]);

  const copy = () =>
    invoke<string>("report")
      .then((report) => navigator.clipboard.writeText(report))
      .then(() => setCopying("copied"))
      .catch((error) => {
        console.error("cannot copy the report", error);
        setCopying("failed");
      });

  const label = { idle: texts.copyReport, copied: texts.reportCopied, failed: texts.reportNotCopied }[copying];
  return (
    <button type="button" className="button secondary" onClick={copy}>
      {label}
    </button>
  );
}
