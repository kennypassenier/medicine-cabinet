import test from "node:test";
import assert from "node:assert/strict";
import { parseDate, toDays, fromDays, weekday, daysUntil, plusDays, iso, parseMinutes, parseDatetime, PlanError } from "../../js/plan/date.js";

const d = (s) => parseDate(s);

test("round trips through days", () => {
  for (const t of ["1970-01-01", "2024-02-29", "2026-10-30", "2100-03-01"]) {
    assert.deepEqual(fromDays(toDays(d(t))), d(t));
  }
});

test("weekday is monday based", () => {
  assert.equal(weekday(d("2026-10-30")), 4); // a Friday
  assert.equal(weekday(d("2026-10-26")), 0);
  assert.equal(weekday(d("1970-01-01")), 3); // a Thursday
});

test("rejects impossible dates", () => {
  assert.throws(() => parseDate("2026-02-30"), PlanError);
  assert.throws(() => parseDate("soon"), PlanError);
});

test("rejects sloppy and out of range dates", () => {
  for (const bad of ["+202-10-30", "2026-1-30x", "2026-10-30garbage", "-200-01-01", "0000-01-01", "9999-12-31",
    "1899-12-31", "２０２６-10-30", "2026/10/30", "", "2026-10-30\n"]) {
    assert.throws(() => parseDate(bad), PlanError, `${bad} must be refused`);
  }
  assert.doesNotThrow(() => parseDate("2026-10-30 08:00"));
  assert.doesNotThrow(() => parseDate("1900-01-01"));
  assert.doesNotThrow(() => parseDate("2200-12-31"));
  assert.doesNotThrow(() => parseDate("2024-02-29"));
  assert.throws(() => parseDate("2100-02-29"), PlanError);
});

test("long garbage is cut in the error", () => {
  assert.throws(() => parseDate("x".repeat(5000)), (e) => e.message.length < 200);
});

test("times are strict", () => {
  for (const bad of ["8:00", "08:0", "08:30abc", "24:00", "08:60", "08-30", "2026-10-30", "2026-10-30T8:00", ""]) {
    assert.throws(() => parseMinutes(bad), PlanError, `${bad} must be refused`);
  }
  assert.equal(parseMinutes("2026-10-30T21:05:33"), 1265);
});

test("datetimes are canonicalised", () => {
  assert.equal(parseDatetime("2026-10-30 08:05"), "2026-10-30T08:05");
  assert.equal(parseDatetime("2026-10-30T08:05:59"), "2026-10-30T08:05");
  assert.throws(() => parseDatetime("2026-10-30"), PlanError);
});

test("counts days across a leap day", () => {
  assert.equal(daysUntil(d("2024-02-28"), d("2024-03-01")), 2);
  assert.equal(iso(plusDays(d("2024-02-28"), 2)), "2024-03-01");
  assert.equal(daysUntil(d("2026-10-30"), d("2026-10-29")), -1);
});

test("parses times with and without a date", () => {
  assert.equal(parseMinutes("08:30"), 510);
  assert.equal(parseMinutes("2026-10-30T21:05"), 1265);
  assert.throws(() => parseMinutes("25:00"), PlanError);
});
