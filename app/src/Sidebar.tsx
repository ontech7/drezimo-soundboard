import type { ReactNode } from "react";
// Inline markup rather than an <img>: nothing left to load once the page is built
import logo from "./assets/drezimo-logo.svg?raw";
import { type Texts, useTexts } from "./i18n";
import { BoardIcon, DiagnosticsIcon, GeneralIcon, MacroIcon, OverlayIcon, PagesIcon, SoundboardIcon } from "./icons";
import type { Status } from "./status";

export type PageId = "soundboard" | "pages" | "macros" | "board" | "overlay" | "general" | "diagnostics";

const ICONS: Record<PageId, ReactNode> = {
  soundboard: <SoundboardIcon />,
  pages: <PagesIcon />,
  macros: <MacroIcon />,
  board: <BoardIcon />,
  overlay: <OverlayIcon />,
  general: <GeneralIcon />,
  diagnostics: <DiagnosticsIcon />,
};

/** Drezimo's or Voicemod's state, in a few words: what to do about it shows on the Soundboard page */
export interface Connection {
  state: "ok" | "off" | "waiting" | "warning" | "error";
  label: string;
  /** On hover */
  detail?: string;
}

export function connections(texts: Texts, status: Status): Connection[] {
  const { board, voicemod, firmwareUpdate: update } = status;
  let drezimo: Connection;
  if (update?.state === "updating") drezimo = { state: "waiting", label: texts.updatingBoard };
  else if (update?.state === "restarting") drezimo = { state: "waiting", label: texts.restartingBoard };
  else if (board.state === "connected") drezimo = { state: "ok", label: texts.boardConnected };
  else if (board.state === "busy") drezimo = { state: "warning", label: texts.searching, detail: texts.busy };
  else drezimo = { state: "off", label: texts.searching };

  let app: Connection;
  if (voicemod.state === "connected") app = { state: "ok", label: texts.voicemodConnected };
  else if (voicemod.state === "connecting") app = { state: "waiting", label: texts.connectingVoicemod };
  else if (voicemod.state === "unreachable") app = { state: "off", label: texts.voicemodOff };
  else if (voicemod.state === "keyRejected")
    app = { state: "error", label: texts.voicemodRefused, detail: texts.keyRejectedDetail };
  else app = { state: "error", label: texts.missingKey };
  return [drezimo, app];
}

interface Props {
  pages: PageId[];
  page: PageId;
  connections: Connection[];
  onSelect: (page: PageId) => void;
}

export function Sidebar({ pages, page, connections, onSelect }: Props) {
  const texts = useTexts();
  return (
    <nav className="sidebar">
      <div className="logo" role="img" aria-label="Drezimo Soundboard" dangerouslySetInnerHTML={{ __html: logo }} />
      <ul>
        {pages.map((id) => (
          <li key={id}>
            <button
              type="button"
              className={id === page ? "selected" : undefined}
              aria-current={id === page ? "page" : undefined}
              onClick={() => onSelect(id)}
            >
              {ICONS[id]}
              {texts.nav[id]}
            </button>
          </li>
        ))}
      </ul>
      <div className="connections" role="status">
        {connections.map((connection, i) => (
          <p key={i} title={connection.detail}>
            <span className={`dot ${connection.state}`} />
            {connection.label}
          </p>
        ))}
      </div>
    </nav>
  );
}
