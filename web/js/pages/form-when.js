// The "when to take it" card. Both kinds of medicine are always on screen: the
// schedule (times, weekdays, dose) and the as-needed limits (gap, most per day);
// the switch only enables one and disables the other, so the cards below never
// move. The schedule's rows are tracked by a mirror field for the unsaved bar.

import { attachWeekdays, attachTimes, attachRowEditors, TIME_EVENT, QUANTITY_EVENT, WEEKDAYS_EVENT } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { stepper, toggle, wire } from "../controls.js";
import { mirror } from "./form-mirror.js";
import { row, section } from "./form-fields.js";
import { newSlot, readSlots, slotEditor } from "./form-schedule.js";

const FIELDS = "input, button, select, textarea";
const EDITOR_EVENTS = ["kp-rows-change", TIME_EVENT, QUANTITY_EVENT, WEEKDAYS_EVENT];

/** Enable or disable every control inside `host`. @param {HTMLElement} host @param {boolean} off */
function setOff(host, off) {
  for (const f of /** @type {NodeListOf<HTMLInputElement>} */ (host.querySelectorAll(FIELDS))) {
    f.disabled = off;
    f.toggleAttribute("aria-disabled", off);
  }
}

/** How the bar words the schedule: the times and days of each slot. @param {string} value */
export function slotsSummary(value) {
  try {
    return /** @type {{ time: string, dose: number }[]} */ (JSON.parse(value)).map((s) => `${s.time} ×${s.dose}`).join(", ") || "(no times)";
  } catch {
    return value;
  }
}

/** The two as-needed limit rows (minimum gap, most per day). @param {{ minGap: number, maxPerDay: number }} model */
function limitRows(model) {
  const gap = row({ label: "Minimum gap", control: stepper({ label: "Minimum gap in minutes", value: model.minGap, min: 0, step: 30, unit: "min", onChange: (v) => { model.minGap = v; } }), hint: "Between two doses of a medicine taken as needed. 0 means no limit." });
  const most = row({ label: "Most per day", control: stepper({ label: "Most doses per day", value: model.maxPerDay, min: 0, step: 1, unit: "doses", onChange: (v) => { model.maxPerDay = v; } }), hint: "In 24 hours. 0 means no limit." });
  return { gap, most };
}

/** @param {{ asNeeded: boolean, minGap: number, maxPerDay: number, slots: import("./form-schedule.js").Slot[], unit: string }} model mutated as the card changes */
export function createWhen(model) {
  /** @type {(() => void)[]} */
  let detach = [];
  const timesHost = h("div");
  const { gap, most } = limitRows(model);
  const times = row({ label: "Times", control: timesHost, hint: "Used for scheduled medicines. Add a row for each time of the day." });
  /** @type {HTMLElement | null} */
  let editor = null;
  const slots = mirror({
    kind: "slots",
    read: () => JSON.stringify(editor ? readSlots(editor) : model.slots),
    write: (v) => { model.slots = JSON.parse(v); drawEditor(); },
  });
  const applyKind = () => { setOff(timesHost, model.asNeeded); for (const r of [gap, most]) setOff(r, !model.asNeeded); };

  function drawEditor() {
    detach.forEach((d) => d());
    editor = slotEditor(model.slots.length ? model.slots : [newSlot()], model.unit);
    timesHost.replaceChildren(editor);
    wire(timesHost);
    detach = [attachWeekdays(timesHost), attachTimes(timesHost), attachRowEditors(timesHost)];
    for (const type of EDITOR_EVENTS) editor.addEventListener(type, () => slots.sync());
    applyKind();
  }

  const kind = toggle({ label: "Only when needed", checked: model.asNeeded, onChange: (on) => { model.asNeeded = on; applyKind(); } });
  const el = section({ id: "when", title: "When to take it", description: "A fixed schedule, or a guard for medicines you take as needed. The switch picks which half is used.", rows: [
    row({ label: "Kind", control: kind, hint: "Needed medicines have no fixed times; the cabinet guards the safe gap instead." }),
    times, gap, most,
  ] });
  times.querySelector(".kp-formgrid__control")?.append(slots.input);
  drawEditor();
  return {
    el,
    /** The slots to save: what the editor's rows hold now. */
    read: () => (editor ? readSlots(editor) : model.slots),
    stop: () => detach.forEach((d) => d()),
  };
}
