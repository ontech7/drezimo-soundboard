import { type CSSProperties, useEffect, useRef, useState } from "react";
import { DISPLAY_HEIGHT, DISPLAY_WIDTH, frameBytes, isLit, useDisplayFrame } from "./display";
import { useTexts } from "./i18n";

const LIT = [235, 240, 255];

export function DisplayPreview() {
  return <DisplayCanvas frame={useDisplayFrame()} />;
}

/**
 * The CSS size at which each of the display's pixels takes a whole number of the screen's pixels, `scale` or a little
 * more of them, so the picture stays sharp at any display scaling.
 */
export function useCrispSize(scale: number): { width: number; height: number } {
  const [ratio, setRatio] = useState(window.devicePixelRatio);

  useEffect(() => {
    // Fires when the window moves to a screen at another scale
    const media = matchMedia(`(resolution: ${ratio}dppx)`);
    const change = () => setRatio(window.devicePixelRatio);
    media.addEventListener("change", change);
    return () => media.removeEventListener("change", change);
  }, [ratio]);

  const pixels = Math.max(1, Math.round(ratio)) * scale;
  return { width: (DISPLAY_WIDTH * pixels) / ratio, height: (DISPLAY_HEIGHT * pixels) / ratio };
}

/** The live display, small beside the page's title, three times as big once clicked */
export function DisplayButton() {
  const texts = useTexts();
  const frame = useDisplayFrame();
  const small = useCrispSize(1);
  const big = useCrispSize(3);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", escape);
    return () => document.removeEventListener("keydown", escape);
  }, [open]);

  if (!frame) return null;
  return (
    <>
      <button
        type="button"
        className="display-button"
        aria-label={texts.displayPreview}
        title={texts.displayPreview}
        onClick={() => setOpen(true)}
      >
        <DisplayCanvas frame={frame} className="display-small" style={small} />
      </button>
      {open && (
        <div className="display-zoom" role="presentation" onClick={() => setOpen(false)}>
          <DisplayCanvas frame={frame} className="display-small" style={big} />
        </div>
      )}
    </>
  );
}

/** A display picture, as the hex of its DISPLAY message */
export function DisplayCanvas({
  frame,
  className = "display",
  style,
}: {
  frame: string;
  className?: string;
  style?: CSSProperties;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const context = canvas.current?.getContext("2d");
    if (!context || !frame) return;
    const bytes = frameBytes(frame);
    const image = context.createImageData(DISPLAY_WIDTH, DISPLAY_HEIGHT);
    for (let y = 0; y < DISPLAY_HEIGHT; y++) {
      for (let x = 0; x < DISPLAY_WIDTH; x++) {
        if (!isLit(bytes, x, y)) continue;
        image.data.set([...LIT, 255], (y * DISPLAY_WIDTH + x) * 4);
      }
    }
    context.putImageData(image, 0, 0);
  }, [frame]);

  return <canvas ref={canvas} className={className} style={style} width={DISPLAY_WIDTH} height={DISPLAY_HEIGHT} />;
}
