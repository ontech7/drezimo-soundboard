import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { AnchorPicker, Row, Section, Segmented, Slider, Toggle } from "../controls";
import { type Texts, useTexts } from "../i18n";
import { type OverlayScreen, type OverlaySettings, type SettingsView, saveSettings } from "../settings";
import type { Status } from "../status";

const DURATIONS = [2, 4, 6, 10];

/** A screen connected (src-tauri/src/overlay/mod.rs), in pixels */
interface Screen {
  /** The system's name, unique: what the settings keep */
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  main: boolean;
}

type Side = keyof Texts["screenSides"];

/**
 * Where a screen is beside the main one: the system's names say little ("\\.\DISPLAY2", "Monitor #41005"),
 * and Windows' own numbers often differ from them.
 */
function side(screen: Screen, main: Screen | undefined): Side {
  if (screen.main || !main) return "main";
  const centre = (one: Screen) => [one.x + one.width / 2, one.y + one.height / 2];
  const [x, y] = centre(screen);
  const [mainX, mainY] = centre(main);
  if (Math.abs(x - mainX) >= Math.abs(y - mainY)) return x < mainX ? "left" : "right";
  return y < mainY ? "above" : "below";
}

/** Each screen's label, numbered when two are on the same side at the same size */
function labels(texts: Texts, screens: Screen[]): Map<string, string> {
  const main = screens.find((screen) => screen.main);
  const labels = new Map<string, string>();
  const used = new Map<string, number>();
  for (const screen of screens) {
    const label = `${texts.screenSides[side(screen, main)]} · ${screen.width}×${screen.height}`;
    const count = (used.get(label) ?? 0) + 1;
    used.set(label, count);
    labels.set(screen.name, count > 1 ? `${label} (${count})` : label);
  }
  return labels;
}

function screenValue(screen: OverlayScreen): string {
  return typeof screen === "string" ? screen : `named:${screen.named}`;
}

function ScreenSelect({
  value,
  disabled,
  onChange,
}: {
  value: OverlayScreen;
  disabled: boolean;
  onChange: (screen: OverlayScreen) => void;
}) {
  const texts = useTexts();
  // Null until known
  const [screens, setScreens] = useState<Screen[] | null>(null);
  // Asked again on focus, as screens come and go
  const refresh = () => {
    invoke<Screen[]>("screens")
      .then(setScreens)
      .catch((error) => console.error("cannot list the screens", error));
  };
  useEffect(refresh, []);

  const saved = typeof value === "string" ? null : value.named;
  const known = screens ?? [];
  const names = labels(texts, known);
  // With one screen there is nothing to pick; the main one is already an option, unless it is the one saved
  const listed = known.filter((screen) => (known.length > 1 && !screen.main) || screen.name === saved);
  const missing = saved !== null && !known.some((screen) => screen.name === saved);
  return (
    <select
      aria-label={texts.screen}
      value={screenValue(value)}
      disabled={disabled}
      onFocus={refresh}
      onChange={(event) => {
        const picked = event.target.value;
        onChange(picked === "mouse" || picked === "main" ? picked : { named: picked.slice("named:".length) });
      }}
    >
      <option value="mouse">{texts.screenMouse}</option>
      <option value="main">{texts.screenMain}</option>
      {listed.map((screen) => (
        <option key={screen.name} value={`named:${screen.name}`}>
          {names.get(screen.name)}
        </option>
      ))}
      {missing && <option value={`named:${saved}`}>{screens === null ? texts.screen : texts.savedScreenGone}</option>}
    </select>
  );
}

export function OverlayPage({ settings, status }: { settings: SettingsView; status: Status }) {
  const texts = useTexts();
  const overlay = settings.overlay;
  // A duration set by hand in the file still shows
  const durations = DURATIONS.includes(overlay.seconds) ? DURATIONS : [...DURATIONS, overlay.seconds].sort((a, b) => a - b);
  const change = (changed: Partial<OverlaySettings>) => saveSettings({ ...settings, overlay: { ...overlay, ...changed } });
  const preview = (changed: Partial<OverlaySettings>) => invoke("preview_overlay", { overlay: { ...overlay, ...changed } });
  // As the overlay itself: it shows only while the board and Voicemod are there
  const canPreview =
    overlay.enabled && status.board.state === "connected" && status.voicemod.state === "connected" && status.page !== null;

  return (
    <>
      <Section>
        <Row label={texts.showOverlay} text={texts.showOverlayText}>
          <Toggle label={texts.showOverlay} checked={overlay.enabled} onChange={(enabled) => change({ enabled })} />
        </Row>
        <Row label={texts.screen} disabled={!overlay.enabled}>
          <ScreenSelect value={overlay.screen} disabled={!overlay.enabled} onChange={(screen) => change({ screen })} />
        </Row>
        <Row label={texts.position} disabled={!overlay.enabled}>
          <AnchorPicker
            label={texts.position}
            names={texts.anchors}
            value={overlay.anchor}
            disabled={!overlay.enabled}
            onChange={(anchor) => change({ anchor })}
          />
        </Row>
        <Row label={texts.size} text={canPreview ? texts.sizeText : undefined} disabled={!overlay.enabled}>
          <Slider
            label={texts.size}
            value={overlay.size}
            min={70}
            max={100}
            step={5}
            disabled={!overlay.enabled}
            onPreview={(size) => preview({ size })}
            onCommit={(size) => change({ size })}
          />
        </Row>
        <Row label={texts.opacity} disabled={!overlay.enabled}>
          <Slider
            label={texts.opacity}
            value={overlay.opacity}
            min={40}
            max={100}
            step={5}
            disabled={!overlay.enabled}
            onPreview={(opacity) => preview({ opacity })}
            onCommit={(opacity) => change({ opacity })}
          />
        </Row>
        <Row label={texts.duration} text={texts.durationText} disabled={!overlay.enabled}>
          <Segmented
            label={texts.duration}
            options={durations.map((value) => ({ value, label: texts.seconds(value) }))}
            value={overlay.seconds}
            disabled={!overlay.enabled}
            onChange={(seconds) => change({ seconds })}
          />
        </Row>
      </Section>

      <div className="actions">
        <button
          type="button"
          className="button"
          disabled={!canPreview}
          onClick={() => invoke("preview_overlay", { overlay: null })}
        >
          {texts.preview}
        </button>
        {overlay.enabled && !canPreview && <p className="muted">{texts.previewNeeds}</p>}
      </div>
    </>
  );
}
