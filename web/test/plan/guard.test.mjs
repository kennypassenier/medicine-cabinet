import test from "node:test";
import assert from "node:assert/strict";
import { checkDose, doseCheck, minutesAgo } from "../../js/plan/guard.js";

test("minimum gap blocks and reports the wait", () => {
  const v = checkDose([90], 240, null);
  assert.equal(v.allowed, false);
  assert.equal(v.wait_minutes, 150);
});

test("daily maximum blocks", () => {
  assert.equal(checkDose([60, 600], null, 2).allowed, false);
  assert.equal(checkDose([60, 600], null, 3).allowed, true);
});

test("nothing taken is always allowed", () => {
  assert.equal(checkDose([], 240, 1).allowed, true);
});

test("the gap is open exactly when it has elapsed", () => {
  assert.equal(checkDose([239], 240, null).allowed, false);
  const at = checkDose([240], 240, null);
  assert.equal(at.allowed, true);
  assert.equal(at.wait_minutes, null);
  assert.equal(checkDose([241], 240, null).allowed, true);
});

test("a one minute wait is reported as one", () => {
  const v = checkDose([239], 240, null);
  assert.equal(v.wait_minutes, 1);
  assert.ok(v.reason.includes("wait 1 more"));
});

test("just taken waits the full gap", () => {
  assert.equal(checkDose([0], 120, null).wait_minutes, 120);
});

test("only the most recent dose counts for the gap", () => {
  assert.equal(checkDose([600, 30, 400], 60, null).wait_minutes, 30);
});

test("no limits means always allowed", () => {
  assert.equal(checkDose([0, 1, 2, 3], null, null).allowed, true);
});

test("the daily maximum window is exclusive at 24 hours", () => {
  assert.equal(checkDose([1440, 100], null, 2).allowed, true);
  assert.equal(checkDose([1439, 100], null, 2).allowed, false);
});

test("the daily maximum reports when the oldest dose drops out", () => {
  const v = checkDose([60, 600], null, 2);
  assert.equal(v.allowed, false);
  assert.equal(v.wait_minutes, 1440 - 600);
  assert.ok(v.reason.includes("maximum (2)"));
});

test("a maximum of zero blocks everything", () => {
  const v = checkDose([], null, 0);
  assert.equal(v.allowed, false);
  assert.equal(v.wait_minutes, null);
});

test("the gap is checked before the daily maximum", () => {
  const v = checkDose([10, 20, 30], 60, 3);
  assert.equal(v.wait_minutes, 50);
  assert.ok(v.reason.includes("last dose"));
});

test("old doses outside the day do not trigger the maximum", () => {
  assert.equal(checkDose([2000, 3000, 4000], null, 1).allowed, true);
});

// The paracetamol example: gap 240 minutes, at most 4 per 24 hours.
const paracetamol = { id: 1, min_gap_minutes: 240, max_per_day: 4 };
const taken = (at, id = 1, status = "taken") => ({ medicine_id: id, status, taken_at: at });

test("paracetamol taken at 08:00, asked at 10:30: wait 90 minutes", () => {
  const v = doseCheck(paracetamol, [taken("2026-10-30T08:00")], "2026-10-30T10:30");
  assert.deepEqual(v, {
    allowed: false,
    reason: "The last dose was 150 minutes ago; wait 90 more.",
    wait_minutes: 90,
  });
});

test("paracetamol is allowed again at 12:00 and at 12:01", () => {
  assert.equal(doseCheck(paracetamol, [taken("2026-10-30T08:00")], "2026-10-30T12:00").allowed, true);
  assert.equal(doseCheck(paracetamol, [taken("2026-10-30T08:00")], "2026-10-30T11:59").wait_minutes, 1);
});

test("paracetamol hits the daily maximum of four", () => {
  const log = ["2026-10-30T00:30", "2026-10-30T05:00", "2026-10-30T09:30", "2026-10-30T14:00"].map((t) => taken(t));
  const v = doseCheck(paracetamol, log, "2026-10-30T18:30");
  assert.equal(v.allowed, false);
  assert.equal(v.reason, "4 doses in the last 24 hours is the maximum (4).");
  assert.equal(v.wait_minutes, 1440 - (18 * 60 + 30 - 30)); // the 00:30 dose leaves the window in 6 h
});

test("a dose from yesterday counts across midnight", () => {
  const v = doseCheck(paracetamol, [taken("2026-10-29T23:00")], "2026-10-30T00:30");
  assert.equal(v.wait_minutes, 150);
});

test("other medicines, skipped doses, future doses and unreadable rows are ignored", () => {
  const log = [taken("2026-10-30T10:00", 2), taken("2026-10-30T10:00", 1, "skipped"), taken("2026-10-30T11:00"), taken("garbage")];
  assert.deepEqual(minutesAgo(log, 1, "2026-10-30T10:30"), []);
  assert.equal(doseCheck(paracetamol, log, "2026-10-30T10:30").allowed, true);
});

test("only the 50 most recent taken doses are looked at", () => {
  const log = Array.from({ length: 60 }, (_, n) => taken(`2026-10-${String(1 + (n % 28)).padStart(2, "0")}T08:00`));
  assert.equal(minutesAgo(log, 1, "2026-10-30T10:30").length, 50);
});
