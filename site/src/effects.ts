import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  CanvasTexture,
  Color,
  Group,
  Mesh,
  MeshBasicMaterial,
  PlaneGeometry,
  Points,
  PointsMaterial,
} from "three";
import { ripple, spark, type Rgb } from "./motion";

const RIPPLES = 8;
const BURSTS = 8;
const SPARKS_PER_BURST = 14;

function softTexture(draw: (g: CanvasRenderingContext2D, size: number) => void) {
  const size = 128;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  draw(canvas.getContext("2d")!, size);
  return new CanvasTexture(canvas);
}

const ringTexture = () =>
  softTexture((g, size) => {
    const half = size / 2;
    const gradient = g.createRadialGradient(half, half, half * 0.55, half, half, half);
    gradient.addColorStop(0, "rgba(255,255,255,0)");
    gradient.addColorStop(0.55, "rgba(255,255,255,1)");
    gradient.addColorStop(1, "rgba(255,255,255,0)");
    g.fillStyle = gradient;
    g.fillRect(0, 0, size, size);
  });

const dotTexture = () =>
  softTexture((g, size) => {
    const half = size / 2;
    const gradient = g.createRadialGradient(half, half, 0, half, half, half);
    gradient.addColorStop(0, "rgba(255,255,255,1)");
    gradient.addColorStop(0.3, "rgba(255,255,255,0.8)");
    gradient.addColorStop(1, "rgba(255,255,255,0)");
    g.fillStyle = gradient;
    g.fillRect(0, 0, size, size);
  });

/**
 * What a press throws off: a ring of light spreading over the keys and a handful of sparks,
 * in the key's colour. Everything is made once and reused, nothing is allocated per press.
 */
const ignore = () => {};
const dot = dotTexture();

export function createEffects(keySize: number) {
  const group = new Group();

  const ringGeometry = new PlaneGeometry(keySize * 1.3, keySize * 1.3).rotateX(-Math.PI / 2);
  const ringMap = ringTexture();
  const rings = Array.from({ length: RIPPLES }, () => {
    const material = new MeshBasicMaterial({
      map: ringMap,
      transparent: true,
      blending: AdditiveBlending,
      depthWrite: false,
      depthTest: false,
      toneMapped: false,
    });
    const mesh = new Mesh(ringGeometry, material);
    mesh.visible = false;
    mesh.renderOrder = 2;
    mesh.raycast = ignore;
    group.add(mesh);
    return { mesh, material, colour: new Color(), startedAt: -Infinity };
  });

  const count = BURSTS * SPARKS_PER_BURST;
  const positions = new Float32Array(count * 3);
  const colours = new Float32Array(count * 3);
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new BufferAttribute(positions, 3));
  geometry.setAttribute("color", new BufferAttribute(colours, 3));
  const points = new Points(
    geometry,
    new PointsMaterial({
      size: 6,
      map: dot,
      vertexColors: true,
      transparent: true,
      blending: AdditiveBlending,
      depthWrite: false,
      toneMapped: false,
    })
  );
  points.frustumCulled = false;
  points.raycast = ignore;
  points.renderOrder = 3;
  group.add(points);

  const bursts = Array.from({ length: BURSTS }, () => ({
    origin: [0, 0, 0] as Rgb,
    colour: [0, 0, 0] as Rgb,
    velocities: Array.from({ length: SPARKS_PER_BURST }, () => [0, 0, 0] as Rgb),
    startedAt: -Infinity,
  }));

  let nextRing = 0;
  let nextBurst = 0;
  let sparksAlive = false;

  function burst(x: number, y: number, z: number, colour: Rgb, now: number) {
    const ring = rings[nextRing];
    nextRing = (nextRing + 1) % RIPPLES;
    ring.startedAt = now;
    ring.colour.setRGB(...colour);
    ring.mesh.position.set(x, y, z);

    const sparks = bursts[nextBurst];
    nextBurst = (nextBurst + 1) % BURSTS;
    sparks.startedAt = now;
    sparksAlive = true;
    sparks.origin[0] = x;
    sparks.origin[1] = y;
    sparks.origin[2] = z;
    sparks.colour[0] = colour[0];
    sparks.colour[1] = colour[1];
    sparks.colour[2] = colour[2];
    for (const velocity of sparks.velocities) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 25 + Math.random() * 55;
      velocity[0] = Math.cos(angle) * speed;
      velocity[1] = 90 + Math.random() * 110;
      velocity[2] = Math.sin(angle) * speed;
    }
  }

  function update(now: number) {
    for (const ring of rings) {
      const shape = ripple(now - ring.startedAt);
      ring.mesh.visible = shape !== null;
      if (!shape) continue;
      ring.mesh.scale.setScalar(shape.scale);
      ring.material.color.copy(ring.colour).multiplyScalar(1.6 * shape.opacity);
    }

    if (!sparksAlive) return;
    sparksAlive = false;
    let i = 0;
    for (const sparks of bursts) {
      for (const velocity of sparks.velocities) {
        const flight = spark(now - sparks.startedAt, velocity);
        const at = i * 3;
        if (flight) {
          sparksAlive = true;
          for (let axis = 0; axis < 3; axis++) {
            positions[at + axis] = sparks.origin[axis] + flight.offset[axis];
            colours[at + axis] = (0.25 + sparks.colour[axis]) * 3 * flight.brightness;
          }
        } else {
          colours[at] = colours[at + 1] = colours[at + 2] = 0;
        }
        i++;
      }
    }
    geometry.attributes.position.needsUpdate = true;
    geometry.attributes.color.needsUpdate = true;
  }

  return { group, burst, update };
}

/** A pool of light on the floor under the Drezimo, in the colour its LEDs give off. */
export function createFloorGlow(size: number) {
  const material = new MeshBasicMaterial({
    map: dot,
    transparent: true,
    blending: AdditiveBlending,
    depthWrite: false,
    toneMapped: false,
  });
  const mesh = new Mesh(new PlaneGeometry(size, size).rotateX(-Math.PI / 2), material);
  mesh.raycast = ignore;
  mesh.renderOrder = -1;
  return { mesh, material };
}
