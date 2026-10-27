import assert from "node:assert/strict";
import { test } from "node:test";
import { reviewWindowBounds } from "./review-window.mjs";

test("automation selects the non-primary work area without spilling onto the main screen", () => {
  const displays = [
    { id: 5, workArea: { x: 0, y: 0, width: 2560, height: 1440 } },
    { id: 1, workArea: { x: -1728, y: 0, width: 1728, height: 1117 } },
  ];
  assert.deepEqual(reviewWindowBounds(displays, 5, 1180, 820), { x: -1454, y: 148, width: 1180, height: 820 });
  assert.deepEqual(reviewWindowBounds(displays, 5, 2000, 1200), { x: -1728, y: 0, width: 1728, height: 1117 });
  assert.equal(reviewWindowBounds(displays.slice(0, 1), 5, 1180, 820), null);
  assert.equal(reviewWindowBounds([...displays.slice(0, 1), { id: 2, workArea: { x: -800, y: 0, width: 800, height: 720 } }], 5, 1180, 820), null);
});
