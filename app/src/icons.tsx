import type { ReactNode } from "react";

// The app's icons: 24×24, drawn with a 2-unit stroke in the text's colour (see .icon in styles.css)

function Icon({ children }: { children: ReactNode }) {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true">
      {children}
    </svg>
  );
}

export const SoundboardIcon = () => (
  <Icon>
    <rect x="3" y="3" width="5" height="5" rx="1" />
    <rect x="9.5" y="3" width="5" height="5" rx="1" />
    <rect x="16" y="3" width="5" height="5" rx="1" />
    <rect x="3" y="9.5" width="5" height="5" rx="1" />
    <rect x="9.5" y="9.5" width="5" height="5" rx="1" />
    <rect x="16" y="9.5" width="5" height="5" rx="1" />
    <rect x="3" y="16" width="5" height="5" rx="1" />
    <rect x="9.5" y="16" width="5" height="5" rx="1" />
    <rect x="16" y="16" width="5" height="5" rx="1" />
  </Icon>
);

export const BoardIcon = () => (
  <Icon>
    <rect x="5" y="2.5" width="14" height="19" rx="2.5" />
    <rect x="8" y="5.5" width="5" height="3" rx="0.5" />
    <circle cx="15.5" cy="7" r="1.25" />
    <path d="M8.5 13h.01M12 13h.01M15.5 13h.01M8.5 17h.01M12 17h.01M15.5 17h.01" />
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

export const PagesIcon = () => (
  <Icon>
    <path d="M8 3h11a2 2 0 0 1 2 2v11" />
    <rect x="3" y="7" width="14" height="14" rx="2" />
  </Icon>
);

export const MicMuteIcon = () => (
  <Icon>
    <path d="M15 10.5V6a3 3 0 0 0-5.8-1M9 9v2a3 3 0 0 0 5 2.2M5.5 11a6.5 6.5 0 0 0 10.8 4.9M18.5 11a6.4 6.4 0 0 1-.5 2.5" />
    <line x1="12" y1="17.5" x2="12" y2="21" />
    <line x1="4" y1="4" x2="20" y2="20" />
  </Icon>
);

export const ShuffleIcon = () => (
  <Icon>
    <path d="M4 7h3c4.5 0 5.5 10 10 10h3M4 17h3c1.7 0 2.9-1.4 3.9-3.3M13.1 10.3C14.1 8.4 15.3 7 17 7h3" />
    <polyline points="17.5 4.5 20 7 17.5 9.5" />
    <polyline points="17.5 14.5 20 17 17.5 19.5" />
  </Icon>
);

/** Voicemod's background effects */
export const BackgroundIcon = () => (
  <Icon>
    <path d="M3 16c3 0 3-3 6-3s3 3 6 3 3-3 6-3M3 10c3 0 3-3 6-3s3 3 6 3 3-3 6-3" />
  </Icon>
);

export const GoToIcon = () => (
  <Icon>
    <line x1="5" y1="12" x2="19" y2="12" />
    <polyline points="13 6 19 12 13 18" />
  </Icon>
);

export const MacroIcon = () => (
  <Icon>
    <path d="M4 6h9M4 12h6M4 18h9" />
    <path d="M16 9l4 3-4 3z" />
  </Icon>
);

export const PlusIcon = () => (
  <Icon>
    <line x1="12" y1="6" x2="12" y2="18" />
    <line x1="6" y1="12" x2="18" y2="12" />
  </Icon>
);

export const DotsIcon = () => (
  <Icon>
    <path d="M6 12h.01M12 12h.01M18 12h.01" />
  </Icon>
);

export const CloseIcon = () => (
  <Icon>
    <line x1="6" y1="6" x2="18" y2="18" />
    <line x1="18" y1="6" x2="6" y2="18" />
  </Icon>
);

export const SearchIcon = () => (
  <Icon>
    <circle cx="10.5" cy="10.5" r="6.5" />
    <line x1="20" y1="20" x2="15.2" y2="15.2" />
  </Icon>
);

export const CheckIcon = () => (
  <Icon>
    <polyline points="5 12.5 9.5 17 19 7.5" />
  </Icon>
);

export const TrashIcon = () => (
  <Icon>
    <path d="M4 7h16M9 7V4.5h6V7M6.5 7l1 12.5h9l1-12.5" />
  </Icon>
);

export const LockIcon = () => (
  <Icon>
    <path d="M7 11V8a5 5 0 0 1 10 0v3" />
    <rect x="5" y="11" width="14" height="9" rx="1.5" />
  </Icon>
);

export const ChevronUp = () => (
  <Icon>
    <polyline points="7 14 12 9 17 14" />
  </Icon>
);

export const ClockIcon = () => (
  <Icon>
    <circle cx="12" cy="12" r="8.5" />
    <polyline points="12 7.5 12 12 15 14" />
  </Icon>
);

export const PlayIcon = () => (
  <Icon>
    <polygon points="8 5 19 12 8 19" />
  </Icon>
);

export const ChevronDown = () => (
  <Icon>
    <polyline points="7 10 12 15 17 10" />
  </Icon>
);
