import { CanvasTexture, NearestFilter, SRGBColorSpace } from "three";

const WIDTH = 128;
const HEIGHT = 64;

export interface OledState {
  page: number;
  pages: number;
  line: string;
  /** 0 to 1, the bar under the line; it falls on its own. */
  level: number;
}

/** The SH1106's 128×64 pixels, drawn on a canvas with no smoothing. */
export function createOled() {
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const g = canvas.getContext("2d")!;
  const texture = new CanvasTexture(canvas);
  texture.magFilter = NearestFilter;
  texture.minFilter = NearestFilter;
  texture.generateMipmaps = false;
  texture.colorSpace = SRGBColorSpace;

  function draw(state: OledState, pageLabel: string) {
    g.fillStyle = "#000";
    g.fillRect(0, 0, WIDTH, HEIGHT);
    g.fillStyle = "#fff";
    g.textBaseline = "top";

    g.font = "bold 9px monospace";
    g.fillText("DREZIMO", 2, 2);
    const page = `${pageLabel} ${state.page + 1}/${state.pages}`;
    g.fillText(page, WIDTH - 2 - g.measureText(page).width, 2);
    g.fillRect(0, 13, WIDTH, 1);

    g.font = "bold 16px monospace";
    const width = g.measureText(state.line).width;
    g.fillText(state.line, Math.max(2, (WIDTH - width) / 2), 24);

    g.strokeStyle = "#fff";
    g.strokeRect(10.5, 50.5, WIDTH - 21, 8);
    g.fillRect(12, 52, Math.round((WIDTH - 24) * Math.min(1, state.level)), 5);
    texture.needsUpdate = true;
  }

  return { texture, draw };
}
