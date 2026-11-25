const cardinalNames = ["north", "east", "south", "west"];
const straightRows = [2, 0, 3, 1];
const cornerRows = [[2, 5, 2, 7], [4, 0, 9, 0], [3, 8, 3, 10], [6, 1, 11, 1]];
const endRows = { input: [17, 19, 13, 15], output: [16, 18, 12, 14] };
const splitterFrames = [[160, 70, 7, 0], [90, 84, 4, 13], [164, 64, 4, 0], [94, 86, 5, 12]];
const splitterPatches = { east: [90, 104, 4, -20], west: [94, 96, 5, -18] };
let sprites;

export async function loadFactorioSprites(files) {
  const prefix = "express-";
  const required = ["transport-belt", "underground-belt-structure",
    ...cardinalNames.map(name => `splitter-${name}`),
    "splitter-east-top_patch", "splitter-west-top_patch"];
  const byName = new Map(Array.from(files, file => [file.name, file]));
  const images = await Promise.all(required.map(async name => {
    const file = byName.get(`${prefix}${name}.png`);
    if (!file) throw new Error("Choose Factorio’s data/base/graphics/entity folder.");
    const url = URL.createObjectURL(file);
    try {
      const image = new Image();
      image.src = url;
      await image.decode();
      return [name, image];
    } finally {
      URL.revokeObjectURL(url);
    }
  }));
  const loaded = Object.fromEntries(images);
  const dimensions = {
    "transport-belt": [4096, 2560],
    "underground-belt-structure": [768, 768],
    ...Object.fromEntries(cardinalNames.map((name, direction) => {
      const [w, h] = splitterFrames[direction];
      return [`splitter-${name}`, [w * 8, h * 4]];
    })),
    ...Object.fromEntries(Object.entries(splitterPatches).map(([name, [w, h]]) =>
      [`splitter-${name}-top_patch`, [w * 8, h * 4]])),
  };
  for (const [name, [w, h]] of Object.entries(dimensions)) {
    if (loaded[name].naturalWidth !== w || loaded[name].naturalHeight !== h) {
      throw new Error("These sprite sheets do not match Factorio 2.x.");
    }
  }
  sprites = loaded;
}

export function useOriginalBeltArt() {
  sprites = undefined;
}

function frame(ctx, image, col, row, w, h, x, y) {
  // A Factorio 2.x high-resolution sprite has 64 source pixels per tile.
  ctx.drawImage(image, col * w, row * h, w, h, x - w / 128, y - h / 128, w / 64, h / 64);
}

function gameBelt(ctx, x, y, row) {
  frame(ctx, sprites["transport-belt"], 0, row, 128, 128, x, y);
}

function gameSplitter(ctx, x, y, direction) {
  const horizontal = direction % 2 === 1;
  const [dx, dy] = horizontal ? [0, .5] : [.5, 0];
  gameBelt(ctx, x - dx, y - dy, straightRows[direction]);
  gameBelt(ctx, x + dx, y + dy, straightRows[direction]);
  const name = cardinalNames[direction];
  const [w, h, ox, oy] = splitterFrames[direction];
  frame(ctx, sprites[`splitter-${name}`], 0, 0, w, h, x + ox / 32, y + oy / 32);
  const patch = splitterPatches[name];
  if (patch) {
    const [pw, ph, px, py] = patch;
    frame(ctx, sprites[`splitter-${name}-top_patch`], 0, 0, pw, ph, x + px / 32, y + py / 32);
  }
}

function metal(ctx, height, edges, face) {
  const gradient = ctx.createLinearGradient(0, -height, 0, height);
  gradient.addColorStop(0, edges);
  gradient.addColorStop(.30, face);
  gradient.addColorStop(.70, face);
  gradient.addColorStop(1, edges);
  return gradient;
}

function drawTrack(ctx, side = 0, from = 0, to = 1) {
  const path = new Path2D();
  if (side) {
    const radiansAt = t => Math.PI + side * t * Math.PI / 2;
    path.arc(.5, side * .5, .5, radiansAt(from), radiansAt(to), side < 0);
  } else {
    path.moveTo(from - .5, 0);
    path.lineTo(to - .5, 0);
  }
  ctx.lineCap = "butt";
  for (const [width, fill] of [
    [.91, "#101419"], [.83, "#6e747a"], [.77, "#263b4b"],
    [.69, "#4e8ba6"], [.57, metal(ctx, .30, "#4f565b", "#31363b")],
    [.040, "#24282b"],
  ]) {
    ctx.lineWidth = width;
    ctx.strokeStyle = fill;
    ctx.stroke(path);
  }

  const { a, b } = ctx.getTransform();
  if (Math.hypot(a, b) < 24) return;

  // Narrow metal treads follow the curve through both lanes.
  for (let index = 0; index < 10; index++) {
    const t = (index + .5) / 10;
    if (t < from || t > to) continue;
    ctx.save();
    if (side) {
      const angle = Math.PI + side * t * Math.PI / 2;
      ctx.translate(.5 + Math.cos(angle) * .5, side * .5 + Math.sin(angle) * .5);
      ctx.rotate(-side * (1 - t) * Math.PI / 2);
    } else {
      ctx.translate(t - .5, 0);
    }
    for (const y of [-.25, .025]) {
      ctx.fillStyle = "#20272a";
      ctx.fillRect(-.019, y, .042, .225);
      ctx.fillStyle = "#728087";
      ctx.fillRect(.014, y, .016, .225);
    }
    ctx.restore();
  }
}

function directionMark(ctx, x, y, size) {
  ctx.beginPath();
  ctx.moveTo(x - size, y - size);
  ctx.lineTo(x + size, y);
  ctx.lineTo(x - size, y + size);
  ctx.lineTo(x - size * .42, y);
  ctx.closePath();
  ctx.fillStyle = "#79d4f5";
  ctx.fill();
}

function housing(ctx, x, y, w, h, radius, face) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, radius);
  ctx.fillStyle = face;
  ctx.fill();
  ctx.strokeStyle = "#10161a";
  ctx.lineWidth = .025;
  ctx.stroke();
}

function screw(ctx, x, y) {
  ctx.fillStyle = "#182026";
  ctx.fillRect(x - .037, y - .037, .074, .074);
  ctx.fillStyle = "#bac1bc";
  ctx.fillRect(x - .024, y - .024, .048, .022);
}

export function drawBelt(ctx, { x, y, direction }, incoming = []) {
  const turn = incoming.length === 1 && (incoming[0] - direction + 4) % 2 === 1;
  if (sprites) {
    const row = turn ? cornerRows[incoming[0]][direction] : straightRows[direction];
    gameBelt(ctx, x + .5, y + .5, row);
    return;
  }
  ctx.save();
  ctx.translate(x + .5, y + .5);
  ctx.rotate((direction - 1) * Math.PI / 2);
  const side = turn ? (incoming[0] === (direction + 1) % 4 ? -1 : 1) : 0;
  drawTrack(ctx, side);
  directionMark(ctx, .25, 0, .094);
  ctx.restore();
}

export function drawSplitter(ctx, { x, y, direction = 1 }) {
  const horizontal = direction % 2 === 1;
  const cx = x + (horizontal ? .5 : 1);
  const cy = y + (horizontal ? 1 : .5);
  if (sprites) {
    gameSplitter(ctx, cx, cy, direction);
    return;
  }
  ctx.save();
  ctx.translate(cx, cy);
  ctx.rotate((direction - 1) * Math.PI / 2);
  for (const lane of [-.5, .5]) {
    ctx.save();
    ctx.translate(0, lane);
    drawTrack(ctx);
    ctx.restore();
  }
  housing(ctx, -.26, -.94, .52, 1.88, .085, metal(ctx, .94, "#222f33", "#596870"));
  housing(ctx, -.19, -.85, .38, 1.70, .058, "#293740");
  for (const y of [-.83, .83]) {
    screw(ctx, -.16, y);
    screw(ctx, .16, y);
  }
  for (const y of [-.54, .54]) {
    housing(ctx, -.11, y - .27, .44, .54, .06, "#2d627d");
    housing(ctx, -.075, y - .20, .375, .40, .05, metal(ctx, .74, "#3c6680", "#5b95ad"));
    directionMark(ctx, .19, y, .09);
  }
  housing(ctx, -.28, -.16, .58, .32, .07, "#18282f");
  screw(ctx, -.18, 0);
  screw(ctx, .18, 0);
  ctx.fillStyle = "#c29150";
  ctx.fillRect(-.055, -.035, .11, .07);
  ctx.restore();
}

export function drawTunnelEnd(ctx, { x, y, direction, kind }) {
  const input = kind === "input";
  if (sprites) {
    gameBelt(ctx, x + .5, y + .5, endRows[kind][direction]);
    const col = input ? direction : (direction + 2) % 4;
    frame(ctx, sprites["underground-belt-structure"], col, input ? 1 : 0, 192, 192, x + .5, y + .5);
    return;
  }
  ctx.save();
  ctx.translate(x + .5, y + .5);
  ctx.rotate((direction - 1) * Math.PI / 2);
  drawTrack(ctx, 0, input ? 0 : .5, input ? .5 : 1);
  housing(ctx, -.24, -.44, .48, .88, .075, "#202d35");
  housing(ctx, -.19, -.37, .38, .74, .048, metal(ctx, .37, "#28495b", "#507b91"));
  directionMark(ctx, .025, 0, .128);
  for (const x of [-.16, .16]) {
    screw(ctx, x, -.35);
    screw(ctx, x, .35);
  }
  ctx.restore();
}
