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

export const SoundIcon = () => (
  <Icon>
    <path d="M9 17V5l11-2v12" />
    <circle cx="6.5" cy="17" r="2.5" />
    <circle cx="17.5" cy="15" r="2.5" />
  </Icon>
);

export const VoiceIcon = () => (
  <Icon>
    <rect x="9" y="3" width="6" height="11" rx="3" />
    <path d="M5.5 11a6.5 6.5 0 0 0 13 0" />
    <line x1="12" y1="17.5" x2="12" y2="21" />
  </Icon>
);

export const DiagnosticsIcon = () => (
  <Icon>
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <polyline points="7 10 10 12.5 7 15" />
    <line x1="12.5" y1="15" x2="16.5" y2="15" />
  </Icon>
);

export const StopIcon = () => (
  <Icon>
    <rect x="6" y="6" width="12" height="12" rx="2" />
  </Icon>
);

/** The sounds muted for this computer only */
export const MuteMeIcon = () => (
  <Icon>
    <path d="M11 5 6.5 9H3.5v6h3l4.5 4z" />
    <line x1="16" y1="9.5" x2="21" y2="14.5" />
    <line x1="21" y1="9.5" x2="16" y2="14.5" />
  </Icon>
);

export const VoiceChangerIcon = () => (
  <Icon>
    <line x1="4" y1="10" x2="4" y2="14" />
    <line x1="8" y1="7" x2="8" y2="17" />
    <line x1="12" y1="4" x2="12" y2="20" />
    <line x1="16" y1="8" x2="16" y2="16" />
    <line x1="20" y1="10.5" x2="20" y2="13.5" />
  </Icon>
);

export const HearMeIcon = () => (
  <Icon>
    <path d="M4 15v-3a8 8 0 0 1 16 0v3" />
    <rect x="3" y="14" width="4.5" height="6.5" rx="1.5" />
    <rect x="16.5" y="14" width="4.5" height="6.5" rx="1.5" />
  </Icon>
);
