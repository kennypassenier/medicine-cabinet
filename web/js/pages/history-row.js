// One entry of the History log: what was taken, the time and the dose edited in
// place (saved a moment after the last change; Undo puts the old values back),
// and Remove.

import { QUANTITY_EVENT, TIME_EVENT } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { api } from "../api.js";
import { formatDate } from "../clock.js";
import { stepper, stepperValue } from "../controls.js";
import { badge, qty } from "../format.js";
import { undoable } from "../undo.js";
import { button, reportError } from "../ui.js";

const SAVE_DELAY_MS = 600;

/** "Planned for 08:00" style line for an intake. @param {any} i */
function planned(i) {
  if (!i.planned_for) return "As needed";
  const when = i.planned_for.slice(11, 16);
  return i.planned_for.slice(0, 10) === i.taken_at.slice(0, 10) ? `Planned for ${when}` : `Planned for ${formatDate(i.planned_for)} ${when}`;
}

/** The time field of a row. @param {any} i */
const timeField = (i) => h("div", { class: "kp-time", "data-kp-time": "", "data-kp-name": `at-${i.id}`, "data-kp-value": i.taken_at.slice(11, 16), "data-kp-label": `Time of ${i.medicine_name}`, "data-kp-time-step": "5" });

/** Save the row's time and dose together, then offer Undo. @param {any} i @param {{ time: string, dose: number }} next @param {string} unit @param {() => void} after */
async function saveEdit(i, next, unit, after) {
  const was = { taken_at: i.taken_at, dose: i.dose };
  const body = { taken_at: `${i.taken_at.slice(0, 10)}T${next.time}`, dose: next.dose, note: i.note ?? "" };
  try {
    await api.editIntake(i.id, body);
  } catch (e) {
    reportError(`Could not change ${i.medicine_name}.`, e);
    return after(); // the row shows what the server holds again, not the value that was refused
  }
  Object.assign(i, { taken_at: body.taken_at, dose: body.dose });
  after();
  undoable(`${i.medicine_name} changed to ${next.time}${i.status === "taken" ? `, ${qty(next.dose, unit)}` : ""}.`, {
    undo: () => api.editIntake(i.id, { taken_at: was.taken_at, dose: was.dose, note: i.note ?? "" }),
    failed: `Could not put ${i.medicine_name} back.`,
    after,
  });
}

/**
 * @param {any} i the intake @param {Map<number, any>} meds
 * @param {{ remove: (i: any) => void, changed: () => void }} on `changed` runs after a saved edit or its Undo (reload)
 */
export function historyRow(i, meds, on) {
  const unit = meds.get(i.medicine_id)?.stock_unit ?? "";
  const time = i.taken_at.slice(11, 16);
  const taken = i.status === "taken";
  const clock = timeField(i);
  const dose = taken ? stepper({ label: `Dose of ${i.medicine_name}`, value: i.dose, min: 0.25, step: 0.25, unit, key: `dose-${i.id}` }) : null;
  let timer = 0;
  const value = { time, dose: i.dose };
  const schedule = () => {
    clearTimeout(timer);
    timer = window.setTimeout(() => void saveEdit(i, { ...value }, unit, on.changed), SAVE_DELAY_MS);
  };
  clock.addEventListener(TIME_EVENT, (e) => {
    const v = /** @type {CustomEvent} */ (e).detail?.value;
    if (typeof v === "string" && /^\d\d:\d\d$/.test(v) && v !== value.time) { value.time = v; schedule(); }
  });
  dose?.addEventListener(QUANTITY_EVENT, () => { value.dose = stepperValue(dose); schedule(); });
  return h("li", { class: "kp-listrow" },
    h("div", { class: "kp-listrow__identity" },
      h("span", { class: "kp-listrow__name" }, i.medicine_name),
      h("span", { class: "kp-listrow__meta" }, [planned(i), i.note].filter(Boolean).join(" · "))),
    h("div", { class: "kp-listrow__status" }, h("div", { class: "kp-badges" }, taken ? badge("kp-badge--success", "Taken") : badge("", "Skipped")), clock),
    h("div", { class: "kp-listrow__stepper" }, dose ?? h("span", { class: "kp-text-muted" }, "Not taken")),
    h("div", { class: "kp-listrow__switch" }, button("Remove", { look: "kp-button--ghost", key: `remove-${i.id}`, onClick: () => on.remove(i), label: `Remove ${i.medicine_name} at ${time}` })));
}
