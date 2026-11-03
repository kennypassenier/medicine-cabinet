import test from "node:test";
import assert from "node:assert/strict";
import { parseDate, plusDays } from "../../js/plan/date.js";
import { DUE_WINDOW_MINUTES, daySlots, plan, stateOf } from "../../js/plan/plan.js";

const clock = (minute) => `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
const slot = (minute, answered = null, name = "Vitamin D", dose = 1, unit = "capsules") => ({
  medicine_id: 1, medicine_name: name, minute, time: clock(minute), dose, unit,
  answered: answered ? { id: 7, status: answered } : null,
});

test("states follow the clock", () => {
  assert.equal(stateOf(slot(600), 540), "upcoming");
  assert.equal(stateOf(slot(600), 630), "due");
  assert.equal(stateOf(slot(600), 700), "overdue");
});

test("an answer beats the clock", () => {
  assert.equal(stateOf(slot(600, "taken"), 900), "taken");
  assert.equal(stateOf(slot(600, "skipped"), 540), "skipped");
});

test("plan is sorted by time", () => {
  const p = plan([slot(900), slot(480)], 600);
  assert.equal(p[0].time, "08:00");
  assert.equal(p[1].time, "15:00");
});

test("the dose is due from its minute through sixty minutes later", () => {
  const s = slot(600);
  assert.equal(stateOf(s, 599), "upcoming");
  assert.equal(stateOf(s, 600), "due");
  assert.equal(stateOf(s, 660), "due");
  assert.equal(stateOf(s, 661), "overdue");
  assert.equal(DUE_WINDOW_MINUTES, 60);
});

test("midnight and end of day slots behave", () => {
  assert.equal(stateOf(slot(0), 0), "due");
  assert.equal(stateOf(slot(0), 61), "overdue");
  assert.equal(stateOf(slot(1439), 1438), "upcoming");
  assert.equal(stateOf(slot(1439), 1439), "due");
});

test("any status other than taken counts as skipped", () => {
  assert.equal(stateOf(slot(600, "skipped"), 0), "skipped");
  assert.equal(stateOf(slot(600, "whatever"), 0), "skipped");
});

test("plan carries ids, minutes until and the slot fields", () => {
  const p = plan([slot(600, "taken", "A", 2, "tablets"), slot(540, null, "B", 2, "tablets")], 570);
  assert.deepEqual([p[0].medicine_name, p[0].minutes_until], ["B", -30]);
  assert.equal(p[0].intake_id, null);
  assert.deepEqual([p[1].medicine_name, p[1].minutes_until], ["A", 30]);
  assert.equal(p[1].intake_id, 7);
  assert.equal(p[1].state, "taken");
  assert.deepEqual([p[1].dose, p[1].unit], [2, "tablets"]);
  assert.deepEqual(Object.keys(p[0]), ["medicine_id", "medicine_name", "time", "dose", "unit", "state", "intake_id", "minutes_until"]);
});

test("equal times sort by medicine name", () => {
  const p = plan([slot(480, null, "Zinc"), slot(480, null, "Aspirin"), slot(480, null, "Mg")], 0);
  assert.deepEqual(p.map((x) => x.medicine_name), ["Aspirin", "Mg", "Zinc"]);
});

test("an empty day plans nothing", () => {
  assert.deepEqual(plan([], 600), []);
});

// dayplan.rs
const today = parseDate("2026-10-26"); // a Monday
const med = (id, name, extra = {}) => ({
  id, name, active: true, as_needed: false, course_over: false, stock_unit: "capsules",
  slots: [{ time: "08:00", days: 127, dose: 1 }], ...extra,
});
const intake = (id, medicine_id, planned_for, status) => ({
  id, medicine_id, medicine_name: "", taken_at: "2026-10-26T08:05", planned_for, status, dose: 1, note: "",
});

test("a daily slot becomes one unanswered plan slot", () => {
  const s = daySlots([med(1, "A")], [], today);
  assert.equal(s.length, 1);
  assert.deepEqual([s[0].minute, s[0].time], [480, "08:00"]);
  assert.equal(s[0].answered, null);
  assert.equal(s[0].unit, "capsules");
});

test("a matching planned_for answers the slot with its status", () => {
  const s = daySlots([med(1, "A")], [intake(5, 1, "2026-10-26T08:00", "skipped")], today);
  assert.deepEqual(s[0].answered, { id: 5, status: "skipped" });
});

test("an intake for another medicine, day or slot does not answer", () => {
  const logged = [
    intake(1, 2, "2026-10-26T08:00", "taken"),
    intake(2, 1, "2026-10-25T08:00", "taken"),
    intake(3, 1, "2026-10-26T20:00", "taken"),
    intake(4, 1, null, "taken"),
  ];
  assert.equal(daySlots([med(1, "A")], logged, today)[0].answered, null);
});

test("slots only appear on their weekdays", () => {
  const m = med(1, "A", { slots: [{ time: "08:00", days: 0b10, dose: 1 }] });
  assert.equal(daySlots([m], [], today).length, 0);
  assert.equal(daySlots([m], [], plusDays(today, 1)).length, 1);
});

test("inactive, as-needed and finished courses have no slots", () => {
  const all = [med(1, "A", { active: false }), med(2, "B", { as_needed: true }), med(3, "C", { course_over: true })];
  assert.equal(daySlots(all, [], today).length, 0);
});

test("several slots and medicines all appear with their doses", () => {
  const a = med(1, "A", { slots: [{ time: "08:00", days: 127, dose: 1 }, { time: "20:30", days: 127, dose: 2 }] });
  const s = daySlots([a, med(2, "B")], [], today);
  assert.equal(s.length, 3);
  assert.ok(s.some((x) => x.time === "20:30" && x.dose === 2 && x.minute === 1230));
});
