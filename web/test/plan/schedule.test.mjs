import test from "node:test";
import assert from "node:assert/strict";
import { parseDate, plusDays } from "../../js/plan/date.js";
import { runsOn, validateSlot } from "../../js/plan/schedule.js";

const slot = (time, days, dose) => ({ time, days, dose });
const ok = (s) => assert.doesNotThrow(() => validateSlot(s));
const bad = (s) => assert.throws(() => validateSlot(s));

test("slot runs on selected weekdays", () => {
  const monday = slot("08:00", 1, 1);
  assert.ok(runsOn(monday, parseDate("2026-10-26")));
  assert.ok(!runsOn(monday, parseDate("2026-10-27")));
});

test("slot validation", () => {
  bad(slot("8am", 1, 1));
  bad(slot("08:00", 0, 1));
  bad(slot("08:00", 3, 0));
});

test("slot times are stored canonically and doses bounded", () => {
  assert.equal(validateSlot(slot(" 08:05 ", 1, 0.5)).time, "08:05");
  for (const dose of [NaN, Infinity, -1, 1e12]) bad(slot("08:00", 1, dose));
  bad(slot("8:00", 128, 1));
});

test("slot json defaults to every day and one dose", () => {
  const s = validateSlot({ time: "08:00" });
  assert.deepEqual([s.days, s.dose], [127, 1]);
});

test("slot day mask boundaries", () => {
  ok(slot("08:00", 127, 1));
  bad(slot("08:00", 128, 1));
  bad(slot("08:00", 255, 1));
  bad(slot("08:00", 0, 1));
  ok(slot("08:00", 64, 1));
});

test("slot time and dose boundaries", () => {
  ok(slot("00:00", 1, 0.25));
  ok(slot("23:59", 1, 1));
  bad(slot("24:00", 1, 1));
  bad(slot("", 1, 1));
  bad(slot("08:00", 1, -1));
  bad(slot("08:00", 1, 0));
});

test("a slot runs only on its weekdays across a week", () => {
  const weekend = slot("09:00", 0b110_0000, 1);
  const monday = parseDate("2026-10-26");
  const runs = [0, 1, 2, 3, 4, 5, 6].map((n) => runsOn(weekend, plusDays(monday, n)));
  assert.deepEqual(runs, [false, false, false, false, false, true, true]);
});
