import test from "node:test";
import assert from "node:assert/strict";
import { overview, doseCheck } from "../../js/plan/index.js";

const med = (id, name, extra = {}) => ({
  id, name, active: true, as_needed: false, stock: 30, stock_unit: "capsules", low_stock_at: null,
  expires_on: null, opened_on: null, shelf_life_days: null, course_ends_on: null,
  slots: [{ time: "08:00", days: 127, dose: 1 }], symptoms: [], min_gap_minutes: null, max_per_day: null, ...extra,
});

test("the overview has the three keys of /api/mc/today", () => {
  const o = overview([], [], "2026-10-30T10:00");
  assert.deepEqual(o, { doses: [], attention: [], as_needed: [] });
});

test("the overview plans, answers and flags in one go", () => {
  const meds = [
    med(1, "Vitamin D", { stock: 2, low_stock_at: 5 }),
    med(2, "Paracetamol", { as_needed: true, slots: [], min_gap_minutes: 240, max_per_day: 4 }),
    med(3, "Old course", { course_ends_on: "2026-10-01" }),
    med(4, "Switched off", { active: false }),
  ];
  const intakes = [{ id: 9, medicine_id: 1, status: "taken", taken_at: "2026-10-30T08:05", planned_for: "2026-10-30T08:00" }];
  const o = overview(meds, intakes, "2026-10-30T10:30");
  assert.deepEqual(o.doses, [{
    medicine_id: 1, medicine_name: "Vitamin D", time: "08:00", dose: 1, unit: "capsules",
    state: "taken", intake_id: 9, minutes_until: -150,
  }]);
  assert.deepEqual(o.attention.map((a) => [a.medicine_id, a.kind]), [[1, "low_stock"]]);
  assert.deepEqual(o.as_needed.map((m) => m.id), [2]);
  assert.equal(o.as_needed[0].course_over, false);
});

test("medicines come in the server order: switched on first, by name", () => {
  const prn = { as_needed: true, slots: [] };
  const o = overview([med(1, "b", prn), med(2, "A", { ...prn, active: false }), med(3, "C", prn), med(4, "a", prn)], [], "2026-10-30T10:00");
  assert.deepEqual(o.as_needed.map((m) => m.id), [4, 1, 3]);
});

test("a bad clock is refused, not guessed", () => {
  assert.throws(() => overview([], [], "2026-10-30"));
  assert.throws(() => overview([], [], "yesterday"));
});

test("the guard answer for the paracetamol example", () => {
  const p = med(2, "Paracetamol", { as_needed: true, slots: [], min_gap_minutes: 240, max_per_day: 4 });
  const v = doseCheck(p, [{ medicine_id: 2, status: "taken", taken_at: "2026-10-30T08:00" }], "2026-10-30T10:30");
  assert.equal(v.allowed, false);
  assert.equal(v.wait_minutes, 90);
});
