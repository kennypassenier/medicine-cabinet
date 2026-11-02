// The attention band: one alert per problem, worst first, each with its fix.

import { setAttention } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { addStock } from "../stock.js";
import { humanize } from "../format.js";
import { button, linkButton } from "../ui.js";

const SEVERITY = { out_of_stock: "critical", expired: "critical", low_stock: "warning", expiring_soon: "warning", course_ends_soon: "info" };
const TITLE = {
  out_of_stock: "is out of stock",
  low_stock: "is running low",
  expired: "has expired",
  expiring_soon: "expires soon",
  course_ends_soon: "course ends soon",
};

/** How many problems show before "Show N more": the package folds the rest, worst first. */
const SHOWN = 3;

/**
 * @param {HTMLElement} band the element made by attentionBand()
 * @param {any[]} items the API's attention list
 * @param {Map<number, any>} meds
 * @param {() => void} reload
 */
export function renderAttention(band, items, meds, reload) {
  setAttention(band, items.map((a) => {
    const med = meds.get(a.medicine_id);
    const stock = a.kind === "out_of_stock" || a.kind === "low_stock";
    return {
      key: `${a.medicine_id}-${a.kind}`,
      severity: SEVERITY[/** @type {keyof typeof SEVERITY} */ (a.kind)] ?? "info",
      title: `${a.medicine_name} ${TITLE[/** @type {keyof typeof TITLE} */ (a.kind)] ?? a.kind}`,
      text: humanize(a.detail),
      action: stock && med
        ? addStockButton(med, reload)
        : openLink(a),
    };
  }));
}

/** The "Open" link of a problem; its name says which medicine. @param {any} a */
function openLink(a) {
  const link = linkButton("Open", `/medicine/${a.medicine_id}`, "kp-button--sm");
  link.setAttribute("aria-label", `Open ${a.medicine_name}`);
  return link;
}

/** The "Add stock" button: it opens the popover under itself. @param {any} med @param {() => void} reload */
function addStockButton(med, reload) {
  const btn = button("Add stock", { look: "kp-button--sm", label: `Add stock to ${med.name}` });
  btn.setAttribute("aria-haspopup", "dialog");
  btn.addEventListener("click", () => addStock(med, btn, reload));
  return btn;
}

/** The band: the package caps it at `SHOWN` problems and adds "Show N more" itself. */
export function attentionBand() {
  return h("div", { class: "kp-attention kp-ops__attention", role: "region", "aria-label": "Needs attention", "data-kp-attention-max": SHOWN });
}
