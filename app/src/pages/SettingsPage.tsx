import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { BackupCard } from "../BackupCard";
import { Row, Section, Segmented, Toggle } from "../controls";
import { type Texts, useTexts } from "../i18n";
import { type Language, type SettingsView, saveSettings } from "../settings";
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

function updateText(texts: Texts, update: UpdateState): string {
  switch (update.state) {
    case "local":
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
}

export function SettingsPage({ settings, status, onTour }: Props) {
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
          <Row label={texts.boardFirmware}>
            <span className="value">{about.firmware}</span>
          </Row>
          <Row label={texts.voicemodLicense}>
            <span className="value">{status.license ? texts.license(status.license) : "—"}</span>
          </Row>
          {update && <Row label={texts.updates} text={updateText(texts, update)} />}
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
