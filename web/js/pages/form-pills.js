// The "Form" field of the medicine form: one pill per form, exactly one
// chosen (or "No form"), plus the way to manage the list. Choosing a form
// fills the stock unit while the unit is empty or was filled this way.

import { h } from "../dom.js";
import { chips } from "../controls.js";
import { button } from "../ui.js";
import { row } from "./form-fields.js";
import { openFormsManager } from "./forms-manage.js";

/**
 * @param {{ forms: any[], formId: number | null, onPick: (form: any | null) => void, onReloaded: (forms: any[]) => void, reload: () => Promise<any[]> }} o
 * @returns {{ row: HTMLElement, value: () => number | null }}
 */
export function formPills({ forms, formId, onPick, onReloaded, reload }) {
  let list = forms;
  let chosen = formId;
  const slot = h("div", { class: "mc-formpills" });

  function draw() {
    if (chosen !== null && !list.some((f) => f.id === chosen)) chosen = null;
    slot.replaceChildren(chips({
      label: "Form",
      value: chosen === null ? "" : String(chosen),
      options: [["", "No form"], ...list.map((f) => /** @type {[string, string]} */ ([String(f.id), f.name]))],
      onChange: (v) => {
        chosen = v ? Number(v) : null;
        onPick(list.find((f) => f.id === chosen) ?? null);
      },
    }));
  }

  async function manage() {
    if (!(await openFormsManager())) return;
    list = await reload();
    draw();
    onReloaded(list);
  }

  draw();
  const r = row({ label: "Form", control: slot, hint: "Pick how it comes. Choosing one fills the unit." });
  r.append(h("div", { class: "kp-formgrid__actions" }, button("Manage forms", { onClick: () => void manage() })));
  return { row: r, value: () => chosen };
}
