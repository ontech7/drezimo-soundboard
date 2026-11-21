import {
  AdditiveBlending,
  BoxGeometry,
  BufferGeometry,
  CanvasTexture,
  Color,
  CylinderGeometry,
  ExtrudeGeometry,
  Group,
  InstancedMesh,
  Mesh,
  MeshBasicMaterial,
  MeshPhysicalMaterial,
  MeshStandardMaterial,
  Object3D,
  PlaneGeometry,
  Shape,
} from "three";
import layoutJson from "./layout.json";
import { createOled } from "./oled";

export type Layout = typeof layoutJson;
export const layout: Layout = layoutJson;

const BOARD_SIZE = { width: 124.3, depth: 190.35, thickness: 1.6 };
const GLOW_SIZE = 34;
const GLOW_Y = 5.15;
const SWITCH = { width: 14, height: 9.5 };
const GLASS_MARGIN = { side: 2.365, back: 1.9, front: 6.17 };

function roundedRect(width: number, depth: number, radius: number) {
  const shape = new Shape();
  const x = width / 2;
  const y = depth / 2;
  shape.moveTo(-x + radius, -y);
  shape.lineTo(x - radius, -y);
  shape.quadraticCurveTo(x, -y, x, -y + radius);
  shape.lineTo(x, y - radius);
  shape.quadraticCurveTo(x, y, x - radius, y);
  shape.lineTo(-x + radius, y);
  shape.quadraticCurveTo(-x, y, -x, y - radius);
  shape.lineTo(-x, -y + radius);
  shape.quadraticCurveTo(-x, -y, -x + radius, -y);
  return shape;
}

/** An OEM-like keycap: a rounded square that narrows towards its top, bottom at y = 0. */
function keycapGeometry(base: number, height: number): BufferGeometry {
  const bevel = 1.2;
  const geometry = new ExtrudeGeometry(roundedRect(base - 2 * bevel, base - 2 * bevel, 2.2), {
    depth: height - 2 * bevel,
    bevelEnabled: true,
    bevelThickness: bevel,
    bevelSize: bevel,
    bevelSegments: 4,
    curveSegments: 6,
  });
  geometry.rotateX(-Math.PI / 2);
  geometry.translate(0, bevel, 0);
  const position = geometry.attributes.position;
  for (let i = 0; i < position.count; i++) {
    const narrowing = 1 - 0.26 * Math.min(1, Math.max(0, position.getY(i) / height));
    position.setX(i, position.getX(i) * narrowing);
    position.setZ(i, position.getZ(i) * narrowing - (1 - narrowing) * 1.5);
  }
  geometry.computeVertexNormals();
  return geometry;
}

function glowTexture() {
  const size = 128;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const g = canvas.getContext("2d")!;
  const gradient = g.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  gradient.addColorStop(0, "rgba(255,255,255,1)");
  gradient.addColorStop(0.35, "rgba(255,255,255,0.55)");
  gradient.addColorStop(1, "rgba(255,255,255,0)");
  g.fillStyle = gradient;
  g.fillRect(0, 0, size, size);
  return new CanvasTexture(canvas);
}

/** The LED shines through the switch: a keycap glows at its skirt and stays white at its top. */
function lightFromBelow(shader: { vertexShader: string; fragmentShader: string }) {
  const declare = "#include <common>\nvarying float vCapHeight;";
  shader.vertexShader = shader.vertexShader
    .replace("#include <common>", declare)
    .replace("#include <begin_vertex>", "#include <begin_vertex>\nvCapHeight = position.y;");
  shader.fragmentShader = shader.fragmentShader
    .replace("#include <common>", declare)
    .replace(
      "#include <emissivemap_fragment>",
      "#include <emissivemap_fragment>\ntotalEmissiveRadiance *= 1.0 - smoothstep(0.0, 6.5, vCapHeight);"
    );
}

export interface Key {
  n: number;
  x: number;
  z: number;
  cap: Mesh;
  capMaterial: MeshStandardMaterial;
  glow: MeshBasicMaterial;
  pressedAt: number;
  restY: number;
}

export function buildBoard() {
  const board = new Group();
  const caps = new Group();

  const pcb = new Mesh(
    new BoxGeometry(BOARD_SIZE.width, BOARD_SIZE.thickness, BOARD_SIZE.depth),
    new MeshStandardMaterial({ color: 0x0c0d0f, roughness: 0.55, metalness: 0.1 })
  );
  pcb.position.y = -BOARD_SIZE.thickness / 2;
  board.add(pcb);

  const switches = new InstancedMesh(
    new BoxGeometry(SWITCH.width, SWITCH.height, SWITCH.width),
    new MeshPhysicalMaterial({ color: 0x2a2c33, roughness: 0.35, transmission: 0.2, thickness: 2 }),
    layout.keys.length
  );
  const placing = new Object3D();
  layout.keys.forEach((key, i) => {
    placing.position.set(key.x, SWITCH.height / 2, key.z);
    placing.updateMatrix();
    switches.setMatrixAt(i, placing.matrix);
  });
  board.add(switches);

  const glowMap = glowTexture();
  const glowGeometry = new PlaneGeometry(GLOW_SIZE, GLOW_SIZE).rotateX(-Math.PI / 2);
  const capGeometry = keycapGeometry(layout.keycap.base, layout.keycap.height);

  const keys: Key[] = layout.keys.map((key) => {
    const glow = new MeshBasicMaterial({
      map: glowMap,
      transparent: true,
      blending: AdditiveBlending,
      depthWrite: false,
      toneMapped: false,
    });
    const halo = new Mesh(glowGeometry, glow);
    halo.position.set(key.x, GLOW_Y, key.z);
    halo.renderOrder = 1;
    board.add(halo);

    const capMaterial = new MeshStandardMaterial({ color: 0xcfccc5, roughness: 0.55, emissive: new Color(0, 0, 0) });
    capMaterial.onBeforeCompile = lightFromBelow;
    const cap = new Mesh(capGeometry, capMaterial);
    cap.position.set(key.x, layout.keycap.restBottom, key.z);
    cap.userData.key = key.n;
    caps.add(cap);
    return { ...key, cap, capMaterial, glow, pressedAt: -Infinity, restY: layout.keycap.restBottom };
  });

  const { lit } = layout;
  const glass = new Mesh(
    new BoxGeometry(lit.x1 - lit.x0 + 2 * GLASS_MARGIN.side, 1.2, lit.z1 - lit.z0 + GLASS_MARGIN.back + GLASS_MARGIN.front),
    new MeshPhysicalMaterial({ color: 0x050506, roughness: 0.08, clearcoat: 1 })
  );
  glass.position.set((lit.x0 + lit.x1) / 2, lit.y - 0.5, (lit.z0 + lit.z1) / 2 + (GLASS_MARGIN.front - GLASS_MARGIN.back) / 2);
  board.add(glass);

  const oled = createOled();
  const pixels = new Mesh(
    new PlaneGeometry(lit.x1 - lit.x0, lit.z1 - lit.z0).rotateX(-Math.PI / 2),
    new MeshBasicMaterial({ map: oled.texture, toneMapped: false, color: new Color(1.6, 1.7, 1.8) })
  );
  pixels.position.set((lit.x0 + lit.x1) / 2, lit.y + 0.12, (lit.z0 + lit.z1) / 2);
  board.add(pixels);

  const { knob: k } = layout;
  const encoder = new Mesh(
    new BoxGeometry(12, 6.45, 12),
    new MeshStandardMaterial({ color: 0x8a8d93, metalness: 0.8, roughness: 0.35 })
  );
  encoder.position.set(k.x, 6.45 / 2, k.z);
  board.add(encoder);

  const knob = new Group();
  const knobMaterial = new MeshStandardMaterial({ color: 0x1b1c20, metalness: 0.65, roughness: 0.32 });
  const body = new Mesh(new CylinderGeometry(k.diameter / 2, k.diameter / 2 + 0.4, k.height, 60), knobMaterial);
  body.position.y = -k.height / 2;
  const mark = new Mesh(
    new BoxGeometry(0.9, 0.3, k.diameter / 2 - 1.5),
    new MeshBasicMaterial({ color: 0xffb020, toneMapped: false })
  );
  mark.position.set(0, 0.1, -(k.diameter / 4) - 0.4);
  knob.add(body, mark);
  knob.position.set(k.x, k.top, k.z);
  body.userData.knob = true;
  caps.add(knob);

  return { board, caps, keys, knob, oled };
}
