import { type CSSProperties, type ReactNode, useState } from "react";
import { pictureUrl } from "./boardView";
import { ACTION_COLOURS, type Action, rgb } from "./customPages";
import {
  BackgroundIcon,
  GoToIcon,
  HearMeIcon,
  MicMuteIcon,
  MuteMeIcon,
  ShuffleIcon,
  SoundIcon,
  StopIcon,
  VoiceChangerIcon,
  VoiceIcon,
} from "./icons";

const ACTION_ICONS: Record<Action, ReactNode> = {
  stopSounds: <StopIcon />,
  muteMic: <MicMuteIcon />,
  voiceChanger: <VoiceChangerIcon />,
  randomVoice: <ShuffleIcon />,
  hearMyself: <HearMeIcon />,
  muteForMe: <MuteMeIcon />,
  background: <BackgroundIcon />,
  goToPage: <GoToIcon />,
};

interface Props {
  voice?: boolean;
  /** On the pad-image scheme */
  picture?: string | null;
  action?: Action | null;
  large?: boolean;
}

/** Voicemod's picture of a sound or voice, over its kind's icon until it loads, or an action's icon in its colour */
export function PadPicture({ voice, picture, action, large }: Props) {
  const size = large ? " large" : "";
  if (action) {
    return (
      <span className={`picture action${size}`} style={{ "--tint": rgb(ACTION_COLOURS[action]) } as CSSProperties}>
        {ACTION_ICONS[action]}
      </span>
    );
  }
  // Keyed by the picture: the next one never shows the last one's while it loads
  return <Picture key={picture ?? ""} voice={voice} picture={picture} className={`picture${voice ? " voice" : ""}${size}`} />;
}

function Picture({ voice, picture, className }: { voice?: boolean; picture?: string | null; className: string }) {
  const [loaded, setLoaded] = useState(false);
  return (
    <span className={className}>
      {!loaded && (voice ? <VoiceIcon /> : <SoundIcon />)}
      {picture && (
        <img
          src={pictureUrl(picture)}
          alt=""
          loading="lazy"
          draggable={false}
          className={loaded ? "loaded" : undefined}
          onLoad={() => setLoaded(true)}
        />
      )}
    </span>
  );
}
