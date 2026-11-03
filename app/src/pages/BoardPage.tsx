import { invoke } from "@tauri-apps/api/core";
import { Choice, Row, Section, Slider } from "../controls";
import { useTexts } from "../i18n";
import { type DisplaySettings, type SettingsView, saveDisplaySettings, saveSettings } from "../settings";
import type { Status } from "../status";

const DIM_AFTER = [0, 1, 2, 5, 10, 30];
const OFF_AFTER = [0, 5, 10, 15, 30, 60];
const SHIFT_EVERY = [0, 1, 3, 5, 10];

export function BoardPage({ settings, status }: { settings: SettingsView; status: Status }) {
  const texts = useTexts();
  const display = status.displaySettings;
  const change = (changed: Partial<DisplaySettings>) => {
    if (display) saveDisplaySettings({ ...display, ...changed });
  };
  const minutes = (value: number) => (value ? texts.minutes(value) : texts.never);
  const shift = (value: number) => (value ? texts.everyMinutes(value) : texts.off);

  return (
    <>
      <Section title={texts.leds}>
        <Row label={texts.brightness} text={texts.ledBrightnessText}>
          <Slider
            label={texts.brightness}
            value={settings.ledBrightness}
            min={10}
            max={100}
            step={5}
            onPreview={(percent) => invoke("preview_led_brightness", { percent })}
            onCommit={(ledBrightness) => saveSettings({ ...settings, ledBrightness })}
          />
        </Row>
      </Section>

      <Section title={texts.display} text={display ? texts.displaySettingsText : texts.connectForDisplay}>
        <Row label={texts.brightness} disabled={!display}>
          <Slider
            label={texts.brightness}
            value={display?.brightness ?? 50}
            min={0}
            max={100}
            step={5}
            disabled={!display}
            onCommit={(brightness) => change({ brightness })}
          />
        </Row>
        <Row label={texts.dimAfter} text={texts.idleText} disabled={!display}>
          <Choice
            label={texts.dimAfter}
            options={DIM_AFTER.map((value) => ({ value, label: minutes(value) }))}
            value={display?.dimAfter ?? 1}
            describe={minutes}
            disabled={!display}
            onChange={(dimAfter) => change({ dimAfter })}
          />
        </Row>
        <Row label={texts.offAfter} text={texts.idleText} disabled={!display}>
          <Choice
            label={texts.offAfter}
            options={OFF_AFTER.map((value) => ({ value, label: minutes(value) }))}
            value={display?.offAfter ?? 10}
            describe={minutes}
            disabled={!display}
            onChange={(offAfter) => change({ offAfter })}
          />
        </Row>
        <Row label={texts.pixelShift} text={texts.pixelShiftText} disabled={!display}>
          <Choice
            label={texts.pixelShift}
            options={SHIFT_EVERY.map((value) => ({ value, label: shift(value) }))}
            value={display?.shiftEvery ?? 3}
            describe={shift}
            disabled={!display}
            onChange={(shiftEvery) => change({ shiftEvery })}
          />
        </Row>
      </Section>
    </>
  );
}
