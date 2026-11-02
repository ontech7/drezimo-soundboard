import type { ReactNode } from "react";
// Inline markup rather than an <img>: nothing left to load once the page is built
import logo from "./assets/drezimo-logo.svg?raw";
import { useTexts } from "./i18n";

export type PageId = "status" | "board" | "overlay" | "general";

const ICONS: Record<PageId, ReactNode> = {
  status: <polyline points="2 12 6 12 9 5 15 19 18 12 22 12" />,
  board: (
    <>
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="7" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
      <rect x="14" y="14" width="7" height="7" rx="1.5" />
    </>
  ),
  overlay: (
    <>
      <rect x="2" y="4" width="20" height="14" rx="2" />
      <rect x="12" y="10" width="7" height="5" rx="1" />
      <line x1="8" y1="21" x2="16" y2="21" />
    </>
  ),
  general: (
    <>
      <line x1="4" y1="7" x2="20" y2="7" />
      <line x1="4" y1="17" x2="20" y2="17" />
      <circle cx="9" cy="7" r="2.5" />
      <circle cx="15" cy="17" r="2.5" />
    </>
  ),
};

const PAGES: PageId[] = ["status", "board", "overlay", "general"];

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
              <svg viewBox="0 0 24 24" aria-hidden="true">
                {ICONS[id]}
              </svg>
              {texts.nav[id]}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
