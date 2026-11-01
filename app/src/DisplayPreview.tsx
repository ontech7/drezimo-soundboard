import { useEffect, useRef } from "react";
import { DISPLAY_HEIGHT, DISPLAY_WIDTH, frameBytes, isLit, useDisplayFrame } from "./display";

const LIT = [235, 240, 255];

export function DisplayPreview() {
  const frame = useDisplayFrame();
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

  return <canvas ref={canvas} className="display" width={DISPLAY_WIDTH} height={DISPLAY_HEIGHT} />;
}
