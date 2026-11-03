import test from "node:test";
import assert from "node:assert/strict";
import { parseDate, plusDays, iso } from "../../js/plan/date.js";
import { attentionFor } from "../../js/plan/attention.js";
import { judge } from "../../js/plan/medicine.js";

const today = parseDate("2026-10-30");
const med = (id, name, extra = {}) => ({
  id, name, active: true, as_needed: false, stock: 30, stock_unit: "capsules", low_stock_at: null,
  expires_on: null, opened_on: null, shelf_life_days: null, course_ends_on: null, slots: [],
  effective_expiry: null, freshness: "fresh", stock_level: "ok", course_over: false, ...extra,
});
const kinds = (all) => attentionFor(all, today).map((a) => [a.medicine_id, a.kind]);

test("a healthy medicine needs no attention", () => {
  assert.deepEqual(kinds([med(1, "A")]), []);
});

test("stock alerts distinguish out from low", () => {
  const list = attentionFor([med(1, "A", { stock_level: "out" }), med(2, "B", { stock_level: "low", stock: 2.5 })], today);
  assert.deepEqual(list.map((a) => [a.medicine_id, a.kind]), [[1, "out_of_stock"], [2, "low_stock"]]);
  assert.equal(list[0].detail, "None left.");
  assert.equal(list[1].detail, "Only 2.5 capsules left.");
});

test("expiry alerts need a date to name", () => {
  const list = attentionFor([
    med(1, "A", { freshness: "expired", effective_expiry: "2026-10-01" }),
    med(2, "B", { freshness: "expiring_soon", effective_expiry: "2026-11-10" }),
    med(3, "C", { freshness: "expired" }),
  ], today);
  assert.deepEqual(list.map((a) => [a.medicine_id, a.kind]), [[1, "expired"], [2, "expiring_soon"]]);
  assert.equal(list[0].detail, "Expired on 2026-10-01.");
  assert.equal(list[1].detail, "Good until 2026-11-10.");
});

test("a course alerts from three days out through its last day", () => {
  for (const [offset, alert] of [[-1, false], [0, true], [3, true], [4, false]]) {
    const m = med(1, "A", { course_ends_on: iso(plusDays(today, offset)) });
    assert.equal(kinds([m]).length, alert ? 1 : 0, `offset ${offset}`);
  }
});

test("a course end text names the date", () => {
  const list = attentionFor([med(1, "A", { course_ends_on: "2026-11-01" })], today);
  assert.equal(list[0].detail, "The course ends on 2026-11-01.");
});

test("switched off medicines raise nothing", () => {
  const m = med(1, "A", { active: false, stock_level: "out", freshness: "expired", effective_expiry: "2026-01-01" });
  assert.deepEqual(kinds([m]), []);
});

test("one medicine can raise several alerts in a fixed order", () => {
  const m = med(1, "A", { stock_level: "low", freshness: "expiring_soon", effective_expiry: "2026-11-05", course_ends_on: "2026-11-01" });
  assert.deepEqual(kinds([m]), [[1, "low_stock"], [1, "expiring_soon"], [1, "course_ends_soon"]]);
});

test("an unparseable course date is ignored", () => {
  assert.deepEqual(kinds([med(1, "A", { course_ends_on: "not-a-date" })]), []);
});

// domain/medicine.rs
test("the earlier of printed and opened expiry wins", () => {
  const m = judge(med(1, "A", { expires_on: "2027-01-01", opened_on: "2026-10-01", shelf_life_days: 30 }), today);
  assert.equal(m.effective_expiry, "2026-10-31");
});

test("freshness flags soon and past", () => {
  const f = (expires_on) => judge(med(1, "A", { expires_on }), today).freshness;
  assert.equal(f("2026-10-29"), "expired");
  assert.equal(f("2026-11-20"), "expiring_soon");
  assert.equal(f("2027-06-01"), "fresh");
  assert.equal(f(null), "fresh");
});

test("stock levels", () => {
  const lvl = (stock, low_at) => judge(med(1, "A", { stock, low_stock_at: low_at }), today).stock_level;
  assert.equal(lvl(0, 5), "out");
  assert.equal(lvl(4, 5), "low");
  assert.equal(lvl(9, 5), "ok");
  assert.equal(lvl(9, null), "ok");
});

test("a course is over the day after its end date", () => {
  const over = (end) => judge(med(1, "A", { course_ends_on: end }), today).course_over;
  assert.equal(over("2026-10-29"), true);
  assert.equal(over("2026-10-30"), false);
  assert.equal(over(null), false);
});
