const north = 0;
const east = 1;
const south = 2;
const west = 3;

export const maxLanes = 64;

const row = (lane, bank = 0) => 6 * lane + 3 * bank;
const laneFor = (index, bit) => {
  const low = (1 << bit) - 1;
  return (index & ~(low * 2 + 1)) | ((index & low) << 1) | ((index >> bit) & 1);
};

// Lay out a butterfly (or a butterfly followed by its reverse when reducing).
// Successive splitter columns connect track pairs that differ in one bit.
export function generateLayout(inputs, outputs) {
  if (![inputs, outputs].every(count => Number.isInteger(count) && count >= 1 && count <= maxLanes)) {
    throw new RangeError(`Belt counts must be integers from 1 to ${maxLanes}.`);
  }

  const parts = [];
  const add = (x, y, direction = east, kind = "belt") => parts.push({ x, y, kind, direction });
  const eastBelts = (from, to, y) => {
    for (let x = from; x < to; x++) add(x, y);
  };

  function splitterColumn(x, bank, capacity, inputLanes, outputLanes) {
    for (let lane = 0; lane < capacity; lane += 2) {
      const upper = row(lane, bank);
      const lower = row(lane + 1, bank);
      add(x + 2, upper, east, "splitter");
      if (inputLanes.has(lane)) eastBelts(x, x + 2, upper);
      if (inputLanes.has(lane + 1)) {
        add(x, lower);
        for (let y = lower; y > upper + 1; y--) add(x + 1, y, north);
        add(x + 1, upper + 1);
      }
      if (outputLanes.has(lane)) eastBelts(x + 3, x + 5, upper);
      if (outputLanes.has(lane + 1)) {
        for (let y = upper + 1; y < lower; y++) add(x + 3, y, south);
        eastBelts(x + 3, x + 5, lower);
      }
    }
    return x + 5;
  }

  function connectColumns(x, bank, capacity, sourceBit, targetBit) {
    // Two interleaved row banks give each vertical leg room to tunnel under horizontal traffic.
    const wires = Array.from({ length: capacity }, (_, index) => {
      const from = row(laneFor(index, sourceBit), bank);
      const to = row(laneFor(index, targetBit), 1 - bank);
      return { from, to, top: Math.min(from, to), bottom: Math.max(from, to) };
    });
    wires.sort((a, b) => a.top - b.top);
    const columns = [];
    const bend = new Map();
    for (const wire of wires) {
      const free = columns.findIndex(bottom => bottom < wire.top);
      const column = free < 0 ? columns.length : free;
      columns[column] = wire.bottom;
      wire.at = x + 1 + column;
      bend.set(wire.from, { at: wire.at, source: true });
      bend.set(wire.to, { at: wire.at, source: false });
    }
    const end = x + columns.length + 2;
    for (const { at, from, to } of wires) {
      eastBelts(x, at, from);
      eastBelts(at + 1, end, to);
      const step = Math.sign(to - from);
      const heading = step > 0 ? south : north;
      for (let y = from; y !== to; y += step) {
        const crossed = bend.get(y + step);
        const crossesBelt = crossed && (crossed.source ? at < crossed.at : at > crossed.at);
        if (crossesBelt) {
          add(at, y, heading, "input");
          y += 2 * step;
          add(at, y, heading, "output");
        } else {
          add(at, y, heading);
        }
      }
      add(at, to);
    }
    return end;
  }

  function square(start, size, openInputs, openOutputs, reverseStages = false) {
    if (size === 1) {
      eastBelts(start, start + 3, 0);
      return { end: start + 2, bank: 0 };
    }

    const bits = Math.ceil(Math.log2(size));
    const capacity = 2 ** bits;
    const feedback = capacity - size;
    const stages = Array.from({ length: bits }, (_, bit) => bit);
    if (reverseStages) stages.push(...stages.slice(0, -1).reverse());
    const allLanes = new Set(Array.from({ length: capacity }, (_, lane) => lane));
    const liveLanes = count => new Set([...allLanes].filter(lane => lane < count || lane >= size));
    const first = start + feedback + 1;
    let x = first;
    let bank = 0;
    for (const [stage, bit] of stages.entries()) {
      if (stage > 0) {
        x = connectColumns(x, bank, capacity, stages[stage - 1], bit);
        bank = 1 - bank;
      }
      x = splitterColumn(x, bank, capacity,
        stage === 0 ? liveLanes(openInputs) : allLanes,
        stage === stages.length - 1 ? liveLanes(openOutputs) : allLanes,
      );
    }

    const end = x + feedback + 1;
    for (let lane = 0; lane < openInputs; lane++) eastBelts(start, first, row(lane));
    for (let lane = 0; lane < openOutputs; lane++) eastBelts(x, end + 1, row(lane, bank));

    // Wrap surplus outputs around the outside of the bank into surplus inputs.
    for (let index = 0; index < feedback; index++) {
      const lane = size + index;
      const inlet = row(lane);
      const outlet = row(lane, bank);
      const left = start + 1 + index;
      const right = end - 1 - index;
      const bottom = row(capacity, bank) + feedback - index;
      eastBelts(x, right, outlet);
      for (let y = outlet; y < bottom; y++) add(right, y, south);
      for (let cx = right; cx > left; cx--) add(cx, bottom, west);
      for (let y = bottom; y > inlet; y--) add(left, y, north);
      eastBelts(left, first, inlet);
    }
    return { end, bank };
  }

  let start = 0;
  if (inputs > outputs) {
    // A full Beneš network tolerates unused ports without extra feedback traffic.
    const capacity = 2 ** Math.ceil(Math.log2(inputs));
    start = square(0, capacity, inputs, outputs, true).end + 1;
  }
  const result = square(start, outputs, Math.min(inputs, outputs), outputs);
  return {
    width: result.end + 1,
    height: parts.reduce((height, part) => Math.max(height, part.y + (part.kind === "splitter" ? 2 : 1)), 0),
    parts,
    inputRows: Array.from({ length: inputs }, (_, lane) => row(lane)),
    outputRows: Array.from({ length: outputs }, (_, lane) => row(lane, result.bank)),
  };
}
