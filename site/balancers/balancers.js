import { layouts } from "./layouts.js";

const board = document.querySelector(".balancer-board");
const canvas = document.querySelector("#belt-diagram");
const sizes = [...document.querySelectorAll('input[name="size"]')];
const footprint = document.querySelector("#footprint");
const copyButton = document.querySelector("#copy-blueprint");
const copyLabel = document.querySelector("#copy-label");
const status = document.querySelector("#blueprint-status");
const manualCopy = document.querySelector("#manual-copy");
const blueprintField = document.querySelector("#blueprint-string");

const directions = [[0, -1], [1, 0], [0, 1], [-1, 0]];
const blue = "#4998ff";
const cyan = "#5cdef1";

function readLayout(size) {
  const rows = layouts[size].map(row => row.split(" "));
  const width = rows[0].length;
  const parts = [];
  const feeds = new Map();
  const exits = new Map();
  for (const [y, row] of rows.entries()) {
    for (const [x, tile] of row.entries()) {
      if (tile === "." || tile === "s") continue;
      const kind = { S: "splitter", i: "input", o: "output" }[tile[0]] ?? "belt";
      const direction = kind === "splitter" ? 1 : "^>v<".indexOf(tile.at(-1));
      const part = { x, y, kind, direction };
      parts.push(part);
      if (kind === "output") exits.set(`${x},${y},${direction}`, part);
      if (kind === "input") continue;

      const [dx, dy] = directions[direction];
      const lanes = kind === "splitter" ? 2 : 1;
      for (let lane = 0; lane < lanes; lane++) {
        const target = `${x + dx},${y + dy + lane}`;
        const incoming = feeds.get(target) ?? [];
        incoming.push(direction);
        feeds.set(target, incoming);
      }
    }
  }

  const tunnels = [];
  for (const part of parts.filter(part => part.kind === "input")) {
    const [dx, dy] = directions[part.direction];
    for (let distance = 1; distance <= 9; distance++) {
      const exit = exits.get(`${part.x + dx * distance},${part.y + dy * distance},${part.direction}`);
      if (!exit) continue;
      tunnels.push([part, exit]);
      break;
    }
  }
  return { size, width, parts, feeds, tunnels };
}

async function blueprintString(layout) {
  const entities = layout.parts.map(({ x, y, kind, direction }, index) => {
    const name = { belt: "transport-belt", splitter: "splitter" }[kind] ?? "underground-belt";
    const entity = {
      entity_number: index + 1,
      name: `express-${name}`,
      position: { x: x + .5, y: y + (kind === "splitter" ? 1 : .5) },
      direction: direction * 4,
    };
    if (kind === "input" || kind === "output") entity.type = kind;
    return entity;
  });
  const json = JSON.stringify({ blueprint: {
    item: "blueprint",
    label: `${layout.size} × ${layout.size} express belt balancer`,
    icons: [{ index: 1, signal: { type: "item", name: "express-splitter" } }],
    entities,
    version: 2 * 2 ** 48,
  } });
  try {
    const compressed = new Blob([json]).stream().pipeThrough(new CompressionStream("deflate"));
    const bytes = new Uint8Array(await new Response(compressed).arrayBuffer());
    return `0${btoa(String.fromCharCode(...bytes))}`;
  } catch {
    // Factorio 2.x also accepts uncompressed blueprint JSON via Import string.
    return json;
  }
}

function box(ctx, x, y, width, height, radius, fill, border, lineWidth) {
  ctx.beginPath();
  ctx.roundRect(x, y, width, height, radius);
  ctx.fillStyle = fill;
  ctx.fill();
  ctx.strokeStyle = border;
  ctx.lineWidth = lineWidth;
  ctx.stroke();
}

function line(ctx, points, color, width, cap = "round") {
  ctx.beginPath();
  ctx.moveTo(...points[0]);
  for (const point of points.slice(1)) ctx.lineTo(...point);
  ctx.strokeStyle = color;
  ctx.lineWidth = width;
  ctx.lineCap = cap;
  ctx.lineJoin = "round";
  ctx.stroke();
}

function chevron(ctx, x, y, radius, color = blue) {
  line(ctx, [[x - radius, y - radius], [x, y], [x - radius, y + radius]], color, .047);
}

function drawBelt(ctx, { x, y, direction }, incoming) {
  ctx.save();
  ctx.translate(x + .5, y + .5);
  ctx.rotate((direction - 1) * Math.PI / 2);
  box(ctx, -.473, -.43, .946, .86, .045, "#2c2d2c", "#54534e", .027);
  const turn = incoming.length === 1 && (incoming[0] - direction + 4) % 2 === 1;
  const entry = turn ? [0, incoming[0] === (direction + 1) % 4 ? -.36 : .36] : [-.42, 0];
  const track = [entry, [0, 0], [.42, 0]];
  line(ctx, track, "#11181f", .49, "butt");
  line(ctx, track, "#24303a", .40, "butt");
  if (turn) {
    line(ctx, [entry, [0, 0], [.23, 0]], "#3575bd", .055);
    chevron(ctx, .30, 0, .17);
  } else {
    chevron(ctx, -.015, 0, .16);
    chevron(ctx, .23, 0, .16);
  }
  ctx.restore();
}

function drawSplitter(ctx, { x, y }) {
  box(ctx, x + .04, y + .06, .92, 1.88, .13, "#192b40", blue, .052);
  ctx.beginPath();
  for (const lane of [y + .50, y + 1.50]) {
    ctx.moveTo(x + .15, lane);
    ctx.bezierCurveTo(x + .42, lane, x + .32, y + 1, x + .50, y + 1);
    ctx.bezierCurveTo(x + .67, y + 1, x + .56, lane, x + .84, lane);
  }
  ctx.strokeStyle = blue;
  ctx.lineWidth = .05;
  ctx.lineCap = "round";
  ctx.stroke();
  chevron(ctx, x + .84, y + .50, .11);
  chevron(ctx, x + .84, y + 1.50, .11);
}

function drawTunnel(ctx, input, output) {
  ctx.save();
  ctx.setLineDash([.11, .10]);
  const endpoints = [[input.x + .5, input.y + .5], [output.x + .5, output.y + .5]];
  line(ctx, endpoints, "#12252d", .073);
  line(ctx, endpoints, cyan, .034);
  ctx.restore();
}

function drawTunnelEnd(ctx, { x, y, direction, kind }) {
  const side = kind === "input" ? 1 : -1;
  ctx.save();
  ctx.translate(x + .5, y + .5);
  ctx.rotate((direction - 1) * Math.PI / 2);
  ctx.beginPath();
  ctx.moveTo(-.46 * side, -.30);
  ctx.lineTo(.15 * side, -.30);
  ctx.lineTo(.43 * side, -.15);
  ctx.lineTo(.43 * side, .15);
  ctx.lineTo(.15 * side, .30);
  ctx.lineTo(-.46 * side, .30);
  ctx.closePath();
  ctx.fillStyle = "#204d75";
  ctx.fill();
  ctx.strokeStyle = "#63adeb";
  ctx.lineWidth = .044;
  ctx.stroke();
  chevron(ctx, .12, 0, .14, cyan);
  ctx.restore();
}

function drawLayout({ size, width, parts, feeds, tunnels }) {
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  const dpr = window.devicePixelRatio;
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);
  const gutter = w < 500 ? 27 : 64;
  const tile = Math.min(92, (w - gutter * 2) / width, (h - 86) / size);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr * tile, dpr * tile);
  ctx.translate((w / tile - width) / 2, (h / tile - size) / 2 + 8 / tile);

  for (const part of parts) {
    if (part.kind === "splitter") drawSplitter(ctx, part);
    if (part.kind === "belt") drawBelt(ctx, part, feeds.get(`${part.x},${part.y}`) ?? []);
  }
  for (const [input, output] of tunnels) drawTunnel(ctx, input, output);
  for (const part of parts) {
    if (part.kind === "input" || part.kind === "output") drawTunnelEnd(ctx, part);
  }

  const labelGap = (w < 500 ? 12 : 27) / tile;
  ctx.fillStyle = "#f1eadc";
  ctx.font = `700 ${Math.max(11, Math.min(24, tile * .39)) / tile}px ui-monospace, monospace`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (let index = 0; index < size; index++) {
    ctx.fillText(String(index + 1), -labelGap, index + .5);
    ctx.fillText(String(index + 1), width + labelGap, index + .5);
  }
  ctx.fillStyle = "#bcb3a4";
  ctx.font = `700 ${(w < 500 ? 10 : 17) / tile}px ui-monospace, monospace`;
  ctx.textAlign = "left";
  ctx.fillText("INPUT", -labelGap, -20 / tile);
  ctx.textAlign = "right";
  ctx.fillText("OUTPUT", width + labelGap, -20 / tile);
}

function drawLegend() {
  const belt = document.querySelector("#legend-belt").getContext("2d");
  belt.setTransform(56, 0, 0, 55, 6, -1);
  drawBelt(belt, { x: 0, y: 0, direction: 1 }, []);

  const splitter = document.querySelector("#legend-splitter").getContext("2d");
  splitter.setTransform(58, 0, 0, 31, 3, 1);
  drawSplitter(splitter, { x: 0, y: 0 });

  const underground = document.querySelector("#legend-underground").getContext("2d");
  underground.setTransform(32, 0, 0, 56, 2, -2);
  const input = { x: 0, y: 0, kind: "input", direction: 1 };
  const output = { x: 2.6, y: 0, kind: "output", direction: 1 };
  drawTunnel(underground, input, output);
  drawTunnelEnd(underground, input);
  drawTunnelEnd(underground, output);
}

let layout;
let blueprint;

async function selectSize(size) {
  const next = readLayout(size);
  layout = next;
  copyButton.disabled = true;
  manualCopy.hidden = true;
  copyLabel.textContent = "Copy blueprint";
  status.textContent = "";
  footprint.textContent = `${next.width} × ${size} tiles`;
  canvas.setAttribute("aria-label", `${size} input and ${size} output express belt layout, flowing left to right`);
  drawLayout(next);

  const code = await blueprintString(next);
  if (layout !== next) return;
  blueprint = code;
  copyButton.disabled = false;
}

copyButton.addEventListener("click", async () => {
  const selected = layout;
  const code = blueprint;
  try {
    await navigator.clipboard.writeText(code);
    if (layout !== selected) return;
    manualCopy.hidden = true;
    copyLabel.textContent = "Copied!";
    status.textContent = "Blueprint copied to the clipboard.";
  } catch {
    if (layout !== selected) return;
    manualCopy.hidden = false;
    blueprintField.value = code;
    blueprintField.focus();
    blueprintField.select();
    status.textContent = "Copy the selected blueprint string manually.";
  }
});

for (const size of sizes) {
  size.disabled = false;
  size.addEventListener("change", () => selectSize(Number(size.value)));
}
drawLegend();
selectSize(Number(sizes.find(size => size.checked).value));
new ResizeObserver(() => drawLayout(layout)).observe(board);
