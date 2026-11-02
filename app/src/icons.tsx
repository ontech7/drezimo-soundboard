import type { ReactNode } from "react";

// The app's icons: 24×24, drawn with a 2-unit stroke in the text's colour (see .icon in styles.css)

function Icon({ children }: { children: ReactNode }) {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true">
      {children}
    </svg>
  );
}

export const StatusIcon = () => (
  <Icon>
    <polyline points="2 12 6 12 9 5 15 19 18 12 22 12" />
  </Icon>
);

export const BoardIcon = () => (
  <Icon>
    <rect x="3" y="3" width="7" height="7" rx="1.5" />
    <rect x="14" y="3" width="7" height="7" rx="1.5" />
    <rect x="3" y="14" width="7" height="7" rx="1.5" />
    <rect x="14" y="14" width="7" height="7" rx="1.5" />
  </Icon>
);

export const OverlayIcon = () => (
  <Icon>
    <rect x="2" y="4" width="20" height="14" rx="2" />
    <rect x="12" y="10" width="7" height="5" rx="1" />
    <line x1="8" y1="21" x2="16" y2="21" />
  </Icon>
);

export const GeneralIcon = () => (
  <Icon>
    <line x1="4" y1="7" x2="20" y2="7" />
    <line x1="4" y1="17" x2="20" y2="17" />
    <circle cx="9" cy="7" r="2.5" />
    <circle cx="15" cy="17" r="2.5" />
  </Icon>
);

export const ChevronLeft = () => (
  <Icon>
    <polyline points="15 5 8 12 15 19" />
  </Icon>
);

export const ChevronRight = () => (
  <Icon>
    <polyline points="9 5 16 12 9 19" />
  </Icon>
);
