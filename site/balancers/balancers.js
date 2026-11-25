import { blueprintString, getLayout, maxLanes } from "./balancer-layout.js";
import { drawBelt, drawSplitter, drawTunnelEnd, loadFactorioSprites, useOriginalBeltArt } from "./belt-renderer.js";

const form = document.querySelector("#balancer-form");
const inputCount = document.querySelector("#input-count");
const outputCount = document.querySelector("#output-count");
const board = document.querySelector(".balancer-board");
const area = document.querySelector(".diagram-area");
const canvas = document.querySelector("#belt-diagram");
const zoom = document.querySelector("#preview-zoom");
const footprint = document.querySelector("#footprint");
const copyButton = document.querySelector("#copy-blueprint");
const copyLabel = document.querySelector("#copy-label");
const status = document.querySelector("#blueprint-status");
const manualCopy = document.querySelector("#manual-copy");
const blueprintField = document.querySelector("#blueprint-string");
const layoutError = document.querySelector("#layout-error");

const graphicsFiles = document.querySelector("#game-files");
const chooseGraphics = document.querySelector("#use-game-graphics");
const restoreGraphics = document.querySelector("#restore-graphics");
const graphicsStatus = document.querySelector("#graphics-status");

let layout;
let blueprint;
let queuedFrame;

function drawTunnel(ctx, input, output) {
  ctx.save();
  ctx.setLineDash([.11, .10]);
  ctx.beginPath();
  ctx.moveTo(input.x + .5, input.y + .5);
  ctx.lineTo(output.x + .5, output.y + .5);
  ctx.lineCap = "round";
  ctx.strokeStyle = "#142529";
  ctx.lineWidth = .081;
  ctx.stroke();
  ctx.strokeStyle = "#6cdce9";
  ctx.lineWidth = .036;
  ctx.stroke();
  ctx.restore();
}

function drawGrid(ctx, w, h, x, y, tile) {
  const step = tile * Math.ceil(12 / tile);
  ctx.fillStyle = "#282722";
  ctx.fillRect(0, 0, w, h);
  ctx.beginPath();
  for (let gx = x % step; gx <= w; gx += step) {
    ctx.moveTo(gx, 0);
    ctx.lineTo(gx, h);
  }
  for (let gy = y % step; gy <= h; gy += step) {
    ctx.moveTo(0, gy);
    ctx.lineTo(w, gy);
  }
  ctx.lineWidth = 1;
  ctx.strokeStyle = "#e0d3ad16";
  ctx.stroke();
}

function drawPorts(ctx, rows, x, title, tile) {
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillStyle = "#ddd4c5";
  ctx.font = `700 ${Math.max(10, Math.min(20, tile * .32)) / tile}px ui-monospace, monospace`;
  for (const [index, y] of rows.entries()) {
    if (tile >= 9 || rows.length === 1) ctx.fillText(String(index + 1), x, y + .5);
  }
  ctx.fillStyle = "#c0b4a0";
  ctx.font = `700 ${(tile < 25 ? 10 : 13) / tile}px ui-monospace, monospace`;
  ctx.fillText(title, x, rows[0] - 13 / tile);
}

function drawPart(ctx, part, feeds, tile) {
  if (tile < 5) {
    ctx.fillStyle = { splitter: "#63b5d5", belt: "#7a8589" }[part.kind] ?? "#4889a6";
    const horizontal = part.direction % 2 === 1;
    const splitter = part.kind === "splitter";
    ctx.fillRect(part.x + .12, part.y + .12, splitter && !horizontal ? 1.76 : .76,
      splitter && horizontal ? 1.76 : .76);
  } else if (part.kind === "belt") {
    drawBelt(ctx, part, feeds.get(`${part.x},${part.y}`));
  } else if (part.kind === "splitter") {
    drawSplitter(ctx, part);
  } else {
    drawTunnelEnd(ctx, part);
  }
}

function drawLayout() {
  if (!layout) return;
  const { width, height, parts, feeds, tunnels, inputRows, outputRows } = layout;
  const w = board.clientWidth;
  const h = board.clientHeight;
  if (w < 1 || h < 1) return;

  const gutter = w < 500 ? 40 : 66;
  const tile = Number(zoom.value) || Math.max(.25, Math.min(84, (w - gutter * 2) / width, (h - 70) / height));
  const worldWidth = Math.max(w, Math.ceil(width * tile + gutter * 2));
  const worldHeight = Math.max(h, Math.ceil(height * tile + 70));
  area.style.width = worldWidth + "px";
  area.style.height = worldHeight + "px";
  canvas.style.width = w + "px";
  canvas.style.height = h + "px";
  const dpr = window.devicePixelRatio;
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);

  const x = (worldWidth - width * tile) / 2 - board.scrollLeft;
  const y = (worldHeight - height * tile) / 2 - board.scrollTop + 6;
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);
  drawGrid(ctx, w, h, x, y, tile);
  ctx.translate(x, y);
  ctx.scale(tile, tile);

  const left = -x / tile - 2;
  const top = -y / tile - 2;
  const right = (w - x) / tile + 2;
  const bottom = (h - y) / tile + 2;
  const visible = part => part.x >= left && part.x <= right && part.y >= top && part.y <= bottom;
  for (const kind of ["belt", "splitter", "input", "output"]) {
    if (kind === "input" && tile >= 5) {
      for (const [input, output] of tunnels) drawTunnel(ctx, input, output);
    }
    for (const part of parts) {
      if (part.kind === kind && visible(part)) drawPart(ctx, part, feeds, tile);
    }
  }

  const gap = (w < 500 ? 16 : 24) / tile;
  drawPorts(ctx, inputRows, -gap, "IN", tile);
  drawPorts(ctx, outputRows, width + gap, "OUT", tile);
}

function queueDraw() {
  if (queuedFrame) return;
  queuedFrame = requestAnimationFrame(() => {
    queuedFrame = undefined;
    drawLayout();
  });
}

function drawLegend() {
  const context = (selector, scaleX, scaleY, x, y) => {
    const ctx = document.querySelector(selector).getContext("2d");
    ctx.resetTransform();
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    ctx.setTransform(scaleX, 0, 0, scaleY, x, y);
    return ctx;
  };
  const belt = context("#legend-belt", 48, 43, 10, 5);
  drawBelt(belt, { x: 0, y: 0, direction: 1 });

  const splitter = context("#legend-splitter", 46, 26, 8, 6);
  drawSplitter(splitter, { x: 0, y: 0, direction: 1 });

  const underground = context("#legend-underground", 29, 43, 8, 5);
  const input = { x: 0, y: 0, kind: "input", direction: 1 };
  const output = { x: 2.6, y: 0, kind: "output", direction: 1 };
  drawTunnel(underground, input, output);
  drawTunnelEnd(underground, input);
  drawTunnelEnd(underground, output);
}

async function showLayout(inputs, outputs) {
  let next;
  try {
    next = getLayout(inputs, outputs);
  } catch (error) {
    layoutError.textContent = error.message;
    layoutError.hidden = false;
    return;
  }
  layoutError.hidden = true;
  layout = next;
  copyButton.disabled = true;
  manualCopy.hidden = true;
  copyLabel.textContent = "Copy blueprint";
  status.textContent = "";
  footprint.textContent = `${inputs} → ${outputs} · ${next.width} × ${next.height} tiles`;
  canvas.setAttribute("aria-label", `${inputs} input and ${outputs} output express belt layout`);
  board.scrollTo(0, 0);
  queueDraw();

  const code = await blueprintString(next);
  if (layout !== next) return;
  blueprint = code;
  copyButton.disabled = false;
}

form.addEventListener("submit", event => {
  event.preventDefault();
  showLayout(inputCount.valueAsNumber, outputCount.valueAsNumber);
});

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

chooseGraphics.addEventListener("click", () => graphicsFiles.click());
graphicsFiles.addEventListener("change", async () => {
  if (!graphicsFiles.files.length) return;
  chooseGraphics.disabled = true;
  restoreGraphics.disabled = true;
  graphicsStatus.textContent = "Loading game graphics…";
  try {
    await loadFactorioSprites(graphicsFiles.files);
    restoreGraphics.hidden = false;
    graphicsStatus.textContent = "Using game graphics from this device.";
    drawLegend();
    queueDraw();
  } catch (error) {
    graphicsStatus.textContent = error.message;
  } finally {
    graphicsFiles.value = "";
    chooseGraphics.disabled = false;
    restoreGraphics.disabled = false;
  }
});

restoreGraphics.addEventListener("click", () => {
  useOriginalBeltArt();
  restoreGraphics.hidden = true;
  graphicsStatus.textContent = "Default graphics restored.";
  chooseGraphics.focus();
  drawLegend();
  queueDraw();
});

zoom.addEventListener("change", () => {
  board.scrollTo(0, 0);
  queueDraw();
});
board.addEventListener("scroll", queueDraw, { passive: true });
new ResizeObserver(queueDraw).observe(board);
inputCount.max = maxLanes;
outputCount.max = maxLanes;
document.querySelector("#generate-layout").disabled = false;
drawLegend();
showLayout(inputCount.valueAsNumber, outputCount.valueAsNumber);
