// The as-needed flow, shared by Today and Find: ask the server whether a dose
// is allowed, explain a refusal in plain words, offer an explicit override,
// and offer Undo after the dose is logged.

import { api } from "./api.js";
import { h } from "./dom.js";
import { ask } from "./dialogs.js";
import { duration } from "./clock.js";
import { qty } from "./format.js";
import { reportError } from "./ui.js";
import { undoable } from "./undo.js";

/** Medicines whose "I took one" is being handled: a second tap must not log a second dose. @type {Set<number>} */
const taking = new Set();

/** The dose logged for an as-needed medicine: the schedule has no slot to say, so one unit. */
export const AS_NEEDED_DOSE = 1;

/** The server counts in bare minutes ("0 minutes ago; wait 360 more"); the dialog speaks in hours and minutes. @param {string} reason */
function readable(reason) {
  return reason.replace(/was (\d+) minutes? ago; wait \d+ more\./, (_, n) => (Number(n) === 0 ? "was just now." : `was ${duration(Number(n))} ago.`));
}

/** The sentence under "Too soon", from the guard's own reason and wait. @param {any} med @param {any} check */
function refusal(med, check) {
  const wait = check.wait_minutes ? `Safe again in ${duration(check.wait_minutes)}.` : "";
  return h("div", null,
    h("p", null, check.reason ? readable(check.reason) : `The limit for ${med.name} has been reached.`),
    wait ? h("p", null, h("strong", null, wait)) : null,
    h("p", null, "Taking it anyway is your decision; it will be logged like any other dose."));
}

/** Log one dose and offer Undo. @param {any} med @param {() => void} onChange */
async function log(med, onChange) {
  const { id } = await api.logIntake({ medicineId: med.id, status: "taken", dose: AS_NEEDED_DOSE });
  onChange();
  undoable(`Logged 1 dose of ${med.name}. ${qty(Math.max(0, med.stock - AS_NEEDED_DOSE), med.stock_unit)} left.`, {
    undo: () => api.undoIntake(id),
    failed: `Could not take the dose of ${med.name} back.`,
    after: onChange,
  });
}

/**
 * "I took one" for an as-needed medicine.
 * @param {any} med the medicine
 * @param {() => void} onChange called after the log changes (refresh the screen)
 * @returns {Promise<void>}
 */
export async function takeOne(med, onChange) {
  if (taking.has(med.id)) return;
  taking.add(med.id);
  try {
    const check = await api.doseCheck(med.id);
    if (!check.allowed) {
      const yes = await ask({ title: `Too soon for ${med.name}`, text: refusal(med, check), accept: "Take anyway", cancel: "Not now" });
      if (!yes) return;
    }
    await log(med, onChange);
  } catch (e) {
    reportError(`Could not log ${med.name}.`, e);
  } finally {
    taking.delete(med.id);
  }
}
