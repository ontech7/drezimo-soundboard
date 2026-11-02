import { invoke } from "@tauri-apps/api/core";
import { AnchorPicker, Row, Section, Segmented, Slider, Toggle } from "../controls";
import { useTexts } from "../i18n";
import { type OverlaySettings, type SettingsView, saveSettings } from "../settings";
import type { Status } from "../status";

const DURATIONS = [2, 4, 6, 10];

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
        <Row label={texts.position} text={texts.positionText} disabled={!overlay.enabled}>
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
