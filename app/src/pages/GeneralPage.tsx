import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Row, Section, Segmented, Toggle } from "../controls";
import { useTexts } from "../i18n";
import { type Language, type SettingsView, saveSettings } from "../settings";

interface About {
  version: string;
  firmware: string;
}

// macOS has no login item yet (docs/project.md, phase 5)
const WINDOWS = navigator.userAgent.includes("Windows");

export function GeneralPage({ settings }: { settings: SettingsView }) {
  const texts = useTexts();
  // Undefined until known; null where the app cannot change it
  const [startAtLogin, setStartAtLogin] = useState<boolean | null>();
  const [about, setAbout] = useState<About>();

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
  const languages: { value: Language; label: string }[] = [
    { value: "system", label: texts.system },
    { value: "italian", label: "Italiano" },
    { value: "english", label: "English" },
  ];

  return (
    <>
      <Section>
        <Row label={texts.language}>
          <Segmented
            label={texts.language}
            options={languages}
            value={settings.language}
            onChange={(language) => saveSettings({ ...settings, language })}
          />
        </Row>
        {WINDOWS && startAtLogin !== undefined && (
          <Row
            label={texts.startWithWindows}
            text={startAtLogin === null ? texts.onlyInstalled : texts.startWithWindowsText}
            disabled={startAtLogin === null}
          >
            <Toggle
              label={texts.startWithWindows}
              checked={startAtLogin === true}
              disabled={startAtLogin === null}
              onChange={changeStartAtLogin}
            />
          </Row>
        )}
      </Section>

      {about && (
        <Section title={texts.about}>
          <Row label={texts.version}>
            <span className="value">{about.version}</span>
          </Row>
          <Row label={texts.boardFirmware}>
            <span className="value">{about.firmware}</span>
          </Row>
          <Row label={texts.updates} text={texts.updatesText} />
        </Section>
      )}
    </>
  );
}
