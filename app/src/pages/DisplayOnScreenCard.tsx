import { invoke } from "@tauri-apps/api/core";
import { AnchorPicker, Row, Section, Segmented, Toggle } from "../controls";
import { useTexts } from "../i18n";
import { type ScreenDisplaySettings, type SettingsView, saveSettings, type ShowDisplay } from "../settings";

const SIZES = [2, 3];
const WHEN: ShowDisplay[] = ["never", "always", "onActions"];
const ACTIONS = ["onMenu", "onVolumes", "onKnob", "onPage"] as const;

/** Drezimo's display on the PC screen (src-tauri/src/overlay/display.rs), on the Overlay page */
export function DisplayOnScreenCard({ settings }: { settings: SettingsView }) {
  const texts = useTexts();
  const words = texts.screenDisplay;
  const display = settings.screenDisplay;
  const change = (part: Partial<ScreenDisplaySettings>) =>
    saveSettings({ ...settings, screenDisplay: { ...display, ...part } });
  const off = display.when === "never";
  const own = display.ownLook;
  // Following the overlay, it shows where and how big it is then (src-tauri/src/device.rs, screen_look)
  const anchor = own ? display.anchor : settings.overlay.anchor;
  const size = own ? display.size : settings.overlay.size < 90 ? 2 : 3;
  return (
    <Section title={texts.displayOnScreen} text={words.text}>
      <Row label={words.when}>
        <Segmented
          label={words.when}
          options={WHEN.map((value) => ({ value, label: words.whenOptions[value] }))}
          value={display.when}
          onChange={(when) => change({ when })}
        />
      </Row>
      {display.when === "onActions" &&
        ACTIONS.map((action) => (
          <Row key={action} label={words.actions[action]} text={action === "onMenu" ? words.actionsText : undefined}>
            <Toggle
              label={words.actions[action]}
              checked={display[action]}
              onChange={(checked) => change({ [action]: checked })}
            />
          </Row>
        ))}
      <Row label={words.asOverlay} text={own ? words.ownLook : words.inherited} disabled={off}>
        <Toggle label={words.asOverlay} checked={!own} disabled={off} onChange={(same) => change({ ownLook: !same })} />
      </Row>
      <Row label={texts.position} text={texts.displayOnScreenPlace} disabled={off || !own}>
        <AnchorPicker
          label={texts.position}
          names={texts.anchors}
          value={anchor}
          disabled={off || !own}
          onChange={(anchor) => change({ anchor })}
        />
      </Row>
      <Row label={texts.size} text={texts.displayOnScreenSize} disabled={off || !own}>
        <Segmented
          label={texts.size}
          options={SIZES.map((value) => ({ value, label: `${value}×` }))}
          value={size}
          disabled={off || !own}
          onChange={(size) => change({ size })}
        />
      </Row>
      <Row label={texts.latestInputs} text={texts.latestInputsText} disabled={off}>
        <Toggle label={texts.latestInputs} checked={display.inputs} disabled={off} onChange={(inputs) => change({ inputs })} />
      </Row>
      <div className="actions">
        <button type="button" className="button" onClick={() => invoke("preview_screen_display").catch(() => {})}>
          {words.preview}
        </button>
      </div>
    </Section>
  );
}
