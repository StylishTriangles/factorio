import assert from "node:assert/strict";
import { test } from "node:test";
import { inflateSync } from "node:zlib";
import { blueprintString, getLayout, maxLanes } from "../site/balancers/balancer-layout.js";

test("asymmetric layouts expose the requested ports and pair their underground belts", () => {
  for (const [inputs, outputs] of [[1, 1], [3, 5], [4, 5], [7, 4], [18, 17], [33, 64], [64, 33]]) {
    const layout = getLayout(inputs, outputs);
    const label = `${inputs} → ${outputs}`;
    assert.equal(layout.inputRows.length, inputs, label);
    assert.equal(layout.outputRows.length, outputs, label);

    const occupancy = new Map();
    for (const part of layout.parts) {
      const { x, y, kind, direction } = part;
      const tiles = [[x, y]];
      if (kind === "splitter") tiles.push(direction % 2 === 1 ? [x, y + 1] : [x + 1, y]);
      for (const [column, row] of tiles) {
        assert.ok(column >= 0 && column < layout.width && row >= 0 && row < layout.height, label);
        const key = `${column},${row}`;
        assert.ok(!occupancy.has(key), `${label} overlaps at ${key}`);
        occupancy.set(key, part);
      }
    }

    for (const [x, rows] of [[0, layout.inputRows], [layout.width - 1, layout.outputRows]]) {
      const ports = [...occupancy.values()].filter(part => part.x === x);
      assert.deepEqual(ports.map(part => part.y).sort((a, b) => a - b), rows, label);
      assert.ok(ports.every(part => part.kind === "belt" && part.direction === 1), label);
    }
    assert.equal(layout.tunnels.length, layout.parts.filter(part => part.kind === "input").length, label);
    assert.equal(new Set(layout.tunnels.map(([, end]) => `${end.x},${end.y}`)).size,
      layout.parts.filter(part => part.kind === "output").length, label);
  }
  assert.throws(() => getLayout(0, 1), RangeError);
  assert.throws(() => getLayout(1, maxLanes + 1), RangeError);
});

test("large blueprints round-trip all express entities", async () => {
  const layout = getLayout(64, 33);
  const encoded = await blueprintString(layout);
  assert.equal(encoded[0], "0");
  const { blueprint } = JSON.parse(inflateSync(Buffer.from(encoded.slice(1), "base64")));
  assert.match(blueprint.label, /^64 → 33/);
  assert.equal(blueprint.entities.length, layout.parts.length);
  assert.ok(blueprint.entities.every(entity => entity.name.startsWith("express-")));
});
