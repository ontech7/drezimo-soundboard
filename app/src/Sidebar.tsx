import type { ReactNode } from "react";
// Inline markup rather than an <img>: nothing left to load once the page is built
import logo from "./assets/drezimo-logo.svg?raw";
import { useTexts } from "./i18n";
import { BoardIcon, GeneralIcon, OverlayIcon, StatusIcon } from "./icons";

export type PageId = "status" | "board" | "overlay" | "general";

const ICONS: Record<PageId, ReactNode> = {
  status: <StatusIcon />,
  board: <BoardIcon />,
  overlay: <OverlayIcon />,
  general: <GeneralIcon />,
};

const PAGES: PageId[] = ["status", "general", "board", "overlay"];

export function Sidebar({ page, onSelect }: { page: PageId; onSelect: (page: PageId) => void }) {
  const texts = useTexts();
  return (
    <nav className="sidebar">
      <div className="logo" role="img" aria-label="Drezimo Soundboard" dangerouslySetInnerHTML={{ __html: logo }} />
      <ul>
        {PAGES.map((id) => (
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
    </nav>
  );
}
