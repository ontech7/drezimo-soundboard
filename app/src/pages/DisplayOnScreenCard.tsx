import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { AnchorPicker, Row, Section, Segmented, Toggle } from "../controls";
import { useTexts } from "../i18n";
import type { Anchor } from "../settings";

/** The display on the PC screen (src-tauri/src/overlay/display.rs): never saved, off at each start */
interface DisplayLook {
  shown: boolean;
  anchor: Anchor;
  /** The screen's pixels for each of the display's dots, at a 100 % scale */
  size: number;
  inputs: boolean;
}

const SIZES = [2, 3];

export function DisplayOnScreenCard() {
  const texts = useTexts();
  const [look, setLook] = useState<DisplayLook | null>(null);

  useEffect(() => {
    invoke<DisplayLook>("display_look").then(setLook);
  }, []);

  if (!look) return null;
  const change = (part: Partial<DisplayLook>) => {
    const next = { ...look, ...part };
    setLook(next);
    invoke("set_display_look", { look: next });
  };
  const off = !look.shown;
  return (
    <Section title={texts.displayOnScreen}>
      <Row label={texts.showDisplayOnScreen} text={texts.showDisplayOnScreenText}>
        <Toggle label={texts.showDisplayOnScreen} checked={look.shown} onChange={(shown) => change({ shown })} />
      </Row>
      <Row label={texts.position} text={texts.displayOnScreenPlace} disabled={off}>
        <AnchorPicker
          label={texts.position}
          names={texts.anchors}
          value={look.anchor}
          disabled={off}
          onChange={(anchor) => change({ anchor })}
        />
      </Row>
      <Row label={texts.size} text={texts.displayOnScreenSize} disabled={off}>
        <Segmented
          label={texts.size}
          options={SIZES.map((value) => ({ value, label: `${value}×` }))}
          value={look.size}
          disabled={off}
          onChange={(size) => change({ size })}
        />
      </Row>
      <Row label={texts.latestInputs} text={texts.latestInputsText} disabled={off}>
        <Toggle label={texts.latestInputs} checked={look.inputs} disabled={off} onChange={(inputs) => change({ inputs })} />
      </Row>
    </Section>
  );
}
