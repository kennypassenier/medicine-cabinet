// The "Form" field of the medicine form: one pill per form, at most one
// chosen (a chosen pill pressed again means no form), plus a link to Settings, where the forms are managed.
// Choosing a form fills the stock unit while the unit is empty or was filled this way. A mirror field lets the
// unsaved-changes bar see the choice.

import { h } from "../dom.js";
import { chips } from "../controls.js";
import { linkButton } from "../ui.js";
import { mirror } from "./form-mirror.js";
import { row } from "./form-fields.js";

/**
 * @param {{ forms: any[], formId: number | null, onPick: (form: any | null) => void }} o
 * @returns {{ row: HTMLElement, value: () => number | null }}
 */
export function formPills({ forms, formId, onPick }) {
  let chosen = formId;
  const slot = h("div");
  const bar = mirror({ kind: "form", read: () => (chosen === null ? "" : String(chosen)), write: (v) => { chosen = v ? Number(v) : null; draw(); } });

  function draw() {
    if (chosen !== null && !forms.some((f) => f.id === chosen)) chosen = null;
    slot.replaceChildren(chips({
      label: "Form",
      value: chosen === null ? "" : String(chosen),
      none: true,
      options: forms.map((f) => /** @type {[string, string]} */ ([String(f.id), f.name])),
      onChange: (v) => {
        chosen = v ? Number(v) : null;
        onPick(forms.find((f) => f.id === chosen) ?? null);
        bar.sync();
      },
    }));
  }

  draw();
  const r = row({ label: "Form", control: h("div", null, slot, bar.input), hint: "Pick how it comes. Choosing one fills the unit." });
  r.append(h("div", { class: "kp-formgrid__actions" }, linkButton("Manage forms", "/settings")));
  return { row: r, value: () => chosen };
}
