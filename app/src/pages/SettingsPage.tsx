import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { BackupCard } from "../BackupCard";
import { Row, Section, Segmented, Toggle } from "../controls";
import { type Texts, useTexts } from "../i18n";
import { type Language, type SettingsView, saveSettings } from "../settings";
import { newsOfLine } from "../news/changelog";
import type { Status } from "../status";
import { type UpdateState, useUpdateState } from "../updates";

interface About {
  version: string;
  firmware: string;
}

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
const dayFormat = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" });

/** The time, and the day too when it was not today: checks wait until nobody uses the app, maybe for days */
function when(texts: Texts, millis: number): string {
  const at = new Date(millis);
  const time = timeFormat.format(at);
  return at.toDateString() === new Date().toDateString()
    ? texts.appUpdate.at(time)
    : texts.appUpdate.onDay(dayFormat.format(at), time);
}

/** A build made here does not update itself and says nothing about it */
type PublishedUpdateState = Exclude<UpdateState, { state: "local" }>;

function updateText(texts: Texts, update: PublishedUpdateState): string {
  switch (update.state) {
    case "unavailable":
    case "waiting":
    case "checking":
      return texts.appUpdate[update.state];
    case "upToDate":
      return texts.appUpdate.upToDate(when(texts, update.checkedAt));
    case "downloading":
      return texts.appUpdate.downloading(update.version);
    case "ready":
      return texts.appUpdate.ready(update.version);
    case "failed":
      return texts.appUpdate.failed(when(texts, update.at));
  }
}

/** The app's updates, with a button to look for one now, or to install at once the one downloaded */
function UpdateRow({ update }: { update: PublishedUpdateState }) {
  const texts = useTexts();
  const run = (command: string) => () => {
    invoke(command).catch((error) => console.error(`cannot ${command}`, error));
  };
  return (
    <Row label={texts.updates} text={updateText(texts, update)}>
      {update.state === "ready" ? (
        <button type="button" className="button" onClick={run("install_update")}>
          {texts.appUpdate.installNow}
        </button>
      ) : (
        update.state !== "unavailable" && (
          <button
            type="button"
            className="button secondary"
            disabled={update.state === "checking" || update.state === "downloading"}
            onClick={run("check_for_updates")}
          >
            {texts.appUpdate.checkNow}
          </button>
        )
      )}
    </Row>
  );
}

function languages(texts: Texts): { value: Language; label: string }[] {
  return [
    { value: "system", label: texts.system },
    { value: "italian", label: "Italiano" },
    { value: "english", label: "English" },
  ];
}

interface Props {
  settings: SettingsView;
  status: Status;
  onTour: () => void;
  /** Shows this version's news again, for a version that has some */
  onNews: (version: string) => void;
}

export function SettingsPage({ settings, status, onTour, onNews }: Props) {
  const texts = useTexts();
  // Undefined until known; null where the app cannot change it
  const [startAtLogin, setStartAtLogin] = useState<boolean | null>();
  const [about, setAbout] = useState<About>();
  const update = useUpdateState();

  useEffect(() => {
    invoke<boolean | null>("start_at_login").then(setStartAtLogin);
    invoke<About>("about").then(setAbout);
  }, []);

  const changeStartAtLogin = (enabled: boolean) => {
    invoke("set_start_at_login", { enabled })
      .catch((error) => console.error("cannot change the login item", error))
      .then(() => invoke<boolean | null>("start_at_login"))
      .then(setStartAtLogin);
  };

  return (
    <>
      <Section>
        <Row label={texts.language}>
          <Segmented
            label={texts.language}
            options={languages(texts)}
            value={settings.language}
            onChange={(language) => saveSettings({ ...settings, language })}
          />
        </Row>
        {startAtLogin !== undefined && (
          <Row
            label={texts.startAtLogin}
            text={startAtLogin === null ? texts.onlyInstalled : texts.startAtLoginText}
            disabled={startAtLogin === null}
          >
            <Toggle
              label={texts.startAtLogin}
              checked={startAtLogin === true}
              disabled={startAtLogin === null}
              onChange={changeStartAtLogin}
            />
          </Row>
        )}
        <Row label={texts.tour.label} text={texts.tour.settingText}>
          <button type="button" className="button secondary" onClick={onTour}>
            {texts.tour.watch}
          </button>
        </Row>
      </Section>

      {about && (
        <Section title={texts.about}>
          <Row label={texts.version}>
            <span className="value">{about.version}</span>
          </Row>
          {newsOfLine(about.version, settings.shownLanguage) && (
            <Row label={texts.news.label} text={texts.news.settingText}>
              <button type="button" className="button secondary" onClick={() => onNews(about.version)}>
                {texts.news.open}
              </button>
            </Row>
          )}
          <Row label={texts.boardFirmware}>
            <span className="value">{about.firmware}</span>
          </Row>
          <Row label={texts.voicemodLicense}>
            <span className="value">{status.license ? texts.license(status.license) : "—"}</span>
          </Row>
          {update && update.state !== "local" && <UpdateRow update={update} />}
        </Section>
      )}

      <BackupCard />

      <Section>
        <Row label={texts.showDiagnostics} text={texts.diagnosticsText}>
          <Toggle
            label={texts.showDiagnostics}
            checked={settings.diagnostics}
            onChange={(diagnostics) => saveSettings({ ...settings, diagnostics })}
          />
        </Row>
      </Section>
    </>
  );
}
