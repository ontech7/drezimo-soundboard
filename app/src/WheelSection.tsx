import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { AnchorPicker, Row, Section, Segmented, Slider, Toggle } from "./controls";
import type { Library, PageTarget } from "./customPages";
import { useTexts } from "./i18n";
import { ChevronLeft, ChevronRight } from "./icons";
import { goneTarget, MAX_SLICES, MIN_SLICES, WHEEL_ACTIONS } from "./knob";
import type { Macro } from "./macros";
import { SearchSelect } from "./SearchSelect";
import { type PressAction, type SettingsView, saveSettings, type WheelSettings } from "./settings";
import type { Status } from "./status";
import { WheelPicture, wheelSlice } from "./WheelPicture";

type Kind = "page" | "action" | "macro";

const kindOf = (slice: PressAction): Kind => (slice.action === "page" ? "page" : slice.action === "macro" ? "macro" : "action");

/** The action wheel's slices and its look on the screen, in the Knob tab, once a gesture opens it */
export function WheelSection(props: {
  settings: SettingsView;
  status: Status;
  targets: PageTarget[];
  macros: Macro[] | null;
  library: Library | null;
}) {
  const { settings, targets, macros, library } = props;
  const texts = useTexts();
  const words = texts.knobTab.wheel;
  const wheel = settings.wheel;
  const [editing, setEditing] = useState(0);
  const at = Math.min(editing, wheel.slices.length - 1);
  const slice = wheel.slices[at];
  const kind = kindOf(slice);
  const overlay = settings.overlay;
  const canShow = props.status.voicemod.state === "connected";

  const change = (changed: Partial<WheelSettings>) => saveSettings({ ...settings, wheel: { ...wheel, ...changed } });
  // A new look shows on the screen for a moment, as the overlay's does
  const changeLook = (changed: Partial<WheelSettings>) => {
    change(changed);
    preview(changed);
  };
  const setSlice = (next: PressAction) => change({ slices: wheel.slices.map((other, i) => (i === at ? next : other)) });
  const pickKind = (next: Kind) => {
    if (next === kind) return;
    if (next === "page") setSlice({ action: "page", page: targets[0]?.id ?? "" });
    else if (next === "macro") setSlice({ action: "macro", macroId: macros?.[0]?.id ?? "" });
    else setSlice({ action: "stopSounds" });
  };
  const add = () => {
    // The first action not on the wheel yet, else the first one
    const used = new Set(wheel.slices.map((other) => other.action));
    const fresh = WHEEL_ACTIONS.find((action) => action !== "page" && action !== "macro" && !used.has(action));
    change({ slices: [...wheel.slices, { action: fresh ?? "stopSounds" } as PressAction] });
    setEditing(wheel.slices.length);
  };
  const move = (by: number) => {
    const to = at + by;
    if (to < 0 || to >= wheel.slices.length) return;
    const slices = [...wheel.slices];
    [slices[at], slices[to]] = [slices[to], slices[at]];
    change({ slices });
    setEditing(to);
  };
  const look = wheel.ownLook ? wheel : { anchor: overlay.anchor, size: overlay.size, opacity: overlay.opacity };
  const preview = (changed: Partial<WheelSettings>) =>
    canShow && invoke("preview_wheel", { wheel: { ...wheel, ...changed } }).catch(() => {});

  let value = null;
  if (slice.action === "page") {
    value = (
      <SearchSelect
        label={texts.pageToOpen}
        options={targets.map((page) => ({ value: page.id, label: page.name }))}
        value={slice.page}
        placeholder={texts.choosePage}
        onChange={(page) => setSlice({ action: "page", page })}
        searchLabel={texts.searchPage}
        empty={texts.noPage}
      />
    );
  } else if (slice.action === "macro") {
    value = (
      <SearchSelect
        label={texts.knobTab.whichMacro}
        options={(macros ?? []).map((item) => ({ value: item.id, label: item.name }))}
        value={slice.macroId}
        placeholder={texts.knobTab.chooseMacro}
        onChange={(macroId) => setSlice({ action: "macro", macroId })}
        searchLabel={texts.knobTab.searchMacro}
        empty={texts.knobTab.noMacro}
      />
    );
  } else {
    const actions = WHEEL_ACTIONS.filter((action) => action !== "page" && action !== "macro");
    value = (
      <SearchSelect
        label={words.whatItDoes}
        options={actions.map((action) => ({ value: action, label: texts.knobTab.pressActions[action] }))}
        value={slice.action}
        onChange={(action) => setSlice({ action } as PressAction)}
      />
    );
  }
  const gone = goneTarget(slice, macros, targets, library, texts);

  return (
    <Section title={words.title} text={words.text}>
      <Row label={words.slices} text={words.slicesText}>
        <div className="stepper">
          <button
            type="button"
            className="icon-button"
            aria-label={words.fewer}
            title={words.fewer}
            disabled={wheel.slices.length <= MIN_SLICES}
            onClick={() => change({ slices: wheel.slices.slice(0, -1) })}
          >
            −
          </button>
          <output>{wheel.slices.length}</output>
          <button
            type="button"
            className="icon-button"
            aria-label={words.more}
            title={words.more}
            disabled={wheel.slices.length >= MAX_SLICES}
            onClick={add}
          >
            +
          </button>
        </div>
      </Row>
      <div className="wheel-editor">
        <WheelPicture
          slices={wheel.slices.map((other) => wheelSlice(other, texts, targets, macros))}
          selected={at}
          label={words.title}
          onPick={setEditing}
        />
        <div className="wheel-slice-editor">
          <div className="wheel-slice-head">
            <p className="title">{words.slice(at + 1, wheel.slices.length)}</p>
            <div className="wheel-move">
              <button
                type="button"
                className="icon-button"
                aria-label={words.moveBack}
                title={words.moveBack}
                disabled={at === 0}
                onClick={() => move(-1)}
              >
                <ChevronLeft />
              </button>
              <button
                type="button"
                className="icon-button"
                aria-label={words.moveOn}
                title={words.moveOn}
                disabled={at === wheel.slices.length - 1}
                onClick={() => move(1)}
              >
                <ChevronRight />
              </button>
            </div>
          </div>
          <Segmented
            label={words.kind}
            options={[
              { value: "page" as Kind, label: words.kinds.page },
              { value: "action" as Kind, label: words.kinds.action },
              { value: "macro" as Kind, label: words.kinds.macro },
            ]}
            value={kind}
            onChange={pickKind}
          />
          {value}
          <p className="muted">{gone ?? words.editHint}</p>
        </div>
      </div>
      <Row
        label={words.asOverlay}
        text={
          wheel.ownLook
            ? words.ownLook
            : words.inherited(texts.anchors[overlay.anchor].toLowerCase(), overlay.size, overlay.opacity)
        }
      >
        <Toggle label={words.asOverlay} checked={!wheel.ownLook} onChange={(same) => changeLook({ ownLook: !same })} />
      </Row>
      <Row label={texts.position} disabled={!wheel.ownLook}>
        <AnchorPicker
          label={texts.position}
          names={texts.anchors}
          value={look.anchor}
          disabled={!wheel.ownLook}
          onChange={(anchor) => changeLook({ anchor })}
        />
      </Row>
      <Row label={texts.size} disabled={!wheel.ownLook}>
        <Slider
          label={texts.size}
          value={look.size}
          min={60}
          max={130}
          step={5}
          disabled={!wheel.ownLook}
          onPreview={(size) => preview({ size })}
          onCommit={(size) => change({ size })}
        />
      </Row>
      <Row label={texts.opacity} disabled={!wheel.ownLook}>
        <Slider
          label={texts.opacity}
          value={look.opacity}
          min={40}
          max={100}
          step={5}
          disabled={!wheel.ownLook}
          onPreview={(opacity) => preview({ opacity })}
          onCommit={(opacity) => change({ opacity })}
        />
      </Row>
      <div className="actions">
        <button
          type="button"
          className="button"
          disabled={!canShow}
          onClick={() => invoke("preview_wheel", { wheel: null }).catch(() => {})}
        >
          {words.show}
        </button>
        {!canShow && <p className="muted">{words.showNeeds}</p>}
      </div>
    </Section>
  );
}
