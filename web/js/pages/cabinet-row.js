// One medicine in the cabinet list: identity, schedule and status, quick
// stock adjust and the on/off switch. Columns continue from row to row.

import { h } from "../dom.js";
import { daysLabel, formatDate } from "../clock.js";
import { duration } from "../clock.js";
import { stepper, toggle } from "../controls.js";
import { statusBadges, specLine } from "../format.js";

/** One line saying when a medicine is taken. @param {any} m */
export function scheduleLine(m) {
  if (m.as_needed) {
    const gap = m.min_gap_minutes ? `at least ${duration(m.min_gap_minutes)} apart` : "";
    const max = m.max_per_day ? `max ${m.max_per_day} a day` : "";
    return ["When needed", gap, max].filter(Boolean).join(" · ");
  }
  if (!m.slots.length) return "No schedule";
  const days = [...new Set(m.slots.map((/** @type {any} */ s) => daysLabel(s.days)))];
  return `${m.slots.map((/** @type {any} */ s) => s.time).sort().join(", ")} · ${days.join(" / ")}`;
}

/** The facts line under the name. @param {any} m */
function facts(m) {
  const parts = [specLine(m), m.purpose].filter(Boolean);
  if (m.course_ends_on) parts.push(`course until ${formatDate(m.course_ends_on)}`);
  return parts.join(" · ");
}

/**
 * @param {any} m
 * @param {{ setActive: (m: any, on: boolean) => void, step: (m: any, v: number) => void }} on
 */
export function medicineRow(m, on) {
  const chips = m.symptoms.length
    ? h("ul", { class: "kp-tag-list mc-row__symptoms", "aria-label": "Helps with" }, ...m.symptoms.map((/** @type {string} */ s) => h("li", { class: "kp-tag" }, h("span", null, s))))
    : null;
  return h("li", { class: "mc-row", "data-off": String(!m.active) },
    h("div", { class: "mc-row__main" },
      h("a", { class: "mc-row__name", href: `/medicine/${m.id}`, "data-key": `open-${m.id}` }, m.name),
      h("span", { class: "mc-row__facts" }, facts(m)),
      chips),
    h("div", { class: "mc-row__when" },
      h("span", { class: "mc-row__schedule" }, scheduleLine(m)),
      h("span", { class: "mc-row__badges" }, ...statusBadges(m))),
    h("div", { class: "mc-row__stock" }, stepper({ label: `Stock of ${m.name}`, value: m.stock, min: 0, unit: m.stock_unit, key: `stock-${m.id}`, onChange: (v) => on.step(m, v) })),
    h("div", { class: "mc-row__switch" }, toggle({ label: "Active", name: `${m.name} active`, checked: m.active, key: `switch-${m.id}`, onChange: (checked) => on.setActive(m, checked) })));
}
