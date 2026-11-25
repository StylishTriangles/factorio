import { generateLayout, maxLanes } from "./generate-layout.js";
import { layouts } from "./layouts.js";

export { maxLanes };

const directions = [[0, -1], [1, 0], [0, 1], [-1, 0]];

function compactLayout(pattern) {
  const rows = pattern.map(row => row.split(" "));
  const width = rows[0].length;
  const parts = [];
  const inputRows = [];
  const outputRows = [];
  for (const [y, row] of rows.entries()) {
    for (const [x, tile] of row.entries()) {
      if (tile === "." || tile === "s") continue;
      const kind = { S: "splitter", i: "input", o: "output" }[tile[0]] ?? "belt";
      parts.push({ x, y, kind, direction: "^>v<".indexOf(tile.at(-1)) });
      if (x === 0) inputRows.push(y);
      if (x === width - 1) outputRows.push(y);
    }
  }
  return { width, height: rows.length, parts, inputRows, outputRows };
}

export function getLayout(inputs, outputs) {
  for (const count of [inputs, outputs]) {
    if (!Number.isInteger(count) || count < 1 || count > maxLanes) {
      throw new RangeError(`Choose between 1 and ${maxLanes} belts per side.`);
    }
  }
  const pattern = layouts[`${inputs}x${outputs}`];
  const layout = pattern ? compactLayout(pattern) : generateLayout(inputs, outputs);
  const feeds = new Map();
  const exits = new Map();
  for (const { x, y, kind, direction } of layout.parts) {
    if (kind === "output") exits.set(`${x},${y},${direction}`, true);
    if (kind === "input") continue;
    const [dx, dy] = directions[direction];
    const lanes = kind === "splitter" ? 2 : 1;
    for (let lane = 0; lane < lanes; lane++) {
      const target = `${x + dx + Math.abs(dy) * lane},${y + dy + Math.abs(dx) * lane}`;
      const incoming = feeds.get(target) ?? [];
      incoming.push(direction);
      feeds.set(target, incoming);
    }
  }

  const tunnels = [];
  for (const { x, y, kind, direction } of layout.parts) {
    if (kind !== "input") continue;
    const [dx, dy] = directions[direction];
    // Express underground belts can span eight empty tiles between their ends.
    for (let distance = 1; distance <= 9; distance++) {
      const endX = x + dx * distance;
      const endY = y + dy * distance;
      if (!exits.has(`${endX},${endY},${direction}`)) continue;
      tunnels.push([{ x, y }, { x: endX, y: endY }]);
      break;
    }
  }
  return { ...layout, feeds, tunnels };
}

export async function blueprintString({ parts, inputRows, outputRows }) {
  const entities = parts.map(({ x, y, kind, direction }, index) => {
    const name = { belt: "transport-belt", splitter: "splitter" }[kind] ?? "underground-belt";
    const [dx, dy] = directions[direction];
    const splitter = kind === "splitter";
    const entity = {
      entity_number: index + 1,
      name: `express-${name}`,
      position: { x: x + .5 + (splitter ? .5 * Math.abs(dy) : 0), y: y + .5 + (splitter ? .5 * Math.abs(dx) : 0) },
      direction: direction * 4,
    };
    if (kind === "input" || kind === "output") entity.type = kind;
    return entity;
  });
  const json = JSON.stringify({ blueprint: {
    item: "blueprint",
    label: `${inputRows.length} → ${outputRows.length} express belt balancer`,
    icons: [{ index: 1, signal: { type: "item", name: "express-splitter" } }],
    entities,
    version: 2 * 2 ** 48,
  } });
  try {
    const compressed = new Blob([json]).stream().pipeThrough(new CompressionStream("deflate"));
    const bytes = new Uint8Array(await new Response(compressed).arrayBuffer());
    return `0${bytes.toBase64?.() ?? btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join(""))}`;
  } catch {
    // Factorio 2.x also accepts uncompressed blueprint JSON via Import string.
    return json;
  }
}
