import assert from "node:assert/strict";
import test from "node:test";

import { formatCountdown, splitDisplayAmount } from "./helpers.ts";

test("amount keeps meaningful precision while hiding an all-zero fractional part", () => {
  assert.deepEqual(splitDisplayAmount("15,000.0000000"), { whole: "15,000", fraction: null });
  assert.deepEqual(splitDisplayAmount("15,000.0050000"), { whole: "15,000", fraction: "0050000" });
  assert.deepEqual(splitDisplayAmount("150000000000"), { whole: "150000000000", fraction: null });
});

test("countdown returns a locale-neutral duration", () => {
  assert.equal(formatCountdown(3661), "1h 1m 1s");
  assert.equal(formatCountdown(-1), "0h 0m 0s");
});
