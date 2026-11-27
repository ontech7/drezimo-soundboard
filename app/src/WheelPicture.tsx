import type { CSSProperties, ReactNode } from "react";
import { ACTION_COLOURS, type Action, MACRO_COLOUR, type PageTarget, type Rgb, rgb } from "./customPages";
import type { Texts } from "./i18n";
import {
  BackgroundIcon,
  HearMeIcon,
  MacroIcon,
  MicMuteIcon,
  MuteMeIcon,
  OverlayIcon,
  PagesIcon,
  ResetIcon,
  ShuffleIcon,
  SoundboardIcon,
  StopIcon,
  VoiceChangerIcon,
} from "./icons";
import type { Macro } from "./macros";
import type { PressAction } from "./settings";

export interface WheelSlice {
  name: string;
  icon: ReactNode;
  colour: Rgb;
}

/** A page's or the menu's slice: the window's amber */
const PAGE_COLOUR: Rgb = [255, 176, 32];

const ACTION_ICONS: Partial<Record<PressAction["action"], ReactNode>> = {
  stopSounds: <StopIcon />,
  muteMic: <MicMuteIcon />,
  voiceChanger: <VoiceChangerIcon />,
  randomVoice: <ShuffleIcon />,
  hearMyself: <HearMeIcon />,
  muteForMe: <MuteMeIcon />,
  background: <BackgroundIcon />,
  overlay: <OverlayIcon />,
  resetVolumes: <ResetIcon />,
};

/** A slice as the PC screen shows it (src-tauri/src/controller/wheel.rs) */
export function wheelSlice(action: PressAction, texts: Texts, targets: PageTarget[], macros: Macro[] | null): WheelSlice {
  const missing = texts.knobTab.wheel.missing;
  switch (action.action) {
    case "page":
      return {
        name: targets.find((page) => page.id === action.page)?.name ?? missing,
        icon: <PagesIcon />,
        colour: PAGE_COLOUR,
      };
    case "macro": {
      const item = macros?.find((other) => other.id === action.macroId);
      return { name: item?.name ?? missing, icon: <MacroIcon />, colour: item?.colour ?? MACRO_COLOUR };
    }
    case "menu":
      return { name: texts.knobTab.wheel.menu, icon: <SoundboardIcon />, colour: PAGE_COLOUR };
    case "wheel":
    case "nothing":
      return { name: "", icon: null, colour: PAGE_COLOUR };
    default:
      return {
        name: texts.actionNames[action.action as Action],
        icon: ACTION_ICONS[action.action],
        colour: ACTION_COLOURS[action.action as Action],
      };
  }
}

const OUTER = 186;
const INNER = 82;

function polar(radius: number, degrees: number): [number, number] {
  const angle = (degrees * Math.PI) / 180;
  return [200 + radius * Math.cos(angle), 200 + radius * Math.sin(angle)];
}

/** Slice `index` of `count`, the first centred at the top, a little apart from the next */
function slicePath(index: number, count: number): string {
  const span = 360 / count;
  const gap = count > 1 ? 1.1 : 0;
  const start = -90 - span / 2 + index * span + gap;
  const end = start + span - 2 * gap;
  const large = end - start > 180 ? 1 : 0;
  if (count === 1) return "";
  const [x1, y1] = polar(OUTER, start);
  const [x2, y2] = polar(OUTER, end);
  const innerGap = gap * (OUTER / INNER - 1);
  const [x3, y3] = polar(INNER, end - innerGap);
  const [x4, y4] = polar(INNER, start + innerGap);
  return `M${x1} ${y1}A${OUTER} ${OUTER} 0 ${large} 1 ${x2} ${y2}L${x3} ${y3}A${INNER} ${INNER} 0 ${large} 0 ${x4} ${y4}Z`;
}

/** Drezimo's logo, as the wheel's centre draws it: amber squares drawing a D on a 6×6 grid */
function Logo() {
  const step = 7.5;
  const squares = [];
  for (let row = 0; row < 6; row += 1) {
    for (let column = 0; column < 6; column += 1) {
      const edge = row === 0 || row === 5;
      const lit = edge ? column < 5 : column === 0 || column === 5;
      const size = lit ? 0.8 * step : 0.4 * step;
      const inset = lit ? 0.1 * step : 0.3 * step;
      squares.push(
        <rect
          key={`${row} ${column}`}
          x={177.5 + column * step + inset}
          y={163.5 + row * step + inset}
          width={size}
          height={size}
          rx={size * 0.22}
          className={lit ? "lit" : undefined}
        />
      );
    }
  }
  return <g className="wheel-logo">{squares}</g>;
}

/** The action wheel as the PC screen shows it (src-tauri/src/overlay/wheel.rs); slices to click when `onPick` is given */
export function WheelPicture(props: {
  slices: WheelSlice[];
  selected: number | null;
  label: string;
  /** Under the logo */
  centre?: string;
  onPick?: (index: number) => void;
}) {
  const { slices, selected, onPick } = props;
  const count = Math.max(slices.length, 1);
  return (
    <div className="wheel-picture" role={onPick ? "group" : "img"} aria-label={props.label}>
      <svg viewBox="0 0 400 400" aria-hidden="true">
        <circle cx="200" cy="200" r="196" className="wheel-disc" />
        {slices.map((_, index) => (
          <path
            key={index}
            d={slicePath(index, count)}
            className={index === selected ? "wheel-slice on" : "wheel-slice"}
            onClick={onPick && (() => onPick(index))}
          />
        ))}
        <circle cx="200" cy="200" r="73" className={selected === null && props.centre ? "wheel-centre on" : "wheel-centre"} />
        <Logo />
      </svg>
      {slices.map((slice, index) => {
        const [x, y] = polar((OUTER + INNER) / 2 + 2, -90 + (index * 360) / count);
        const style = { left: `${x / 4}%`, top: `${y / 4}%`, "--tint": rgb(slice.colour) } as CSSProperties;
        const className = index === selected ? "wheel-label on" : "wheel-label";
        const content = (
          <>
            {slice.icon}
            <span>{slice.name}</span>
          </>
        );
        return onPick ? (
          <button
            key={index}
            type="button"
            className={className}
            style={style}
            aria-pressed={index === selected}
            onClick={() => onPick(index)}
          >
            {content}
          </button>
        ) : (
          <span key={index} className={className} style={style}>
            {content}
          </span>
        );
      })}
      {props.centre && <span className={selected === null ? "wheel-centre-name" : "wheel-centre-name on"}>{props.centre}</span>}
    </div>
  );
}
