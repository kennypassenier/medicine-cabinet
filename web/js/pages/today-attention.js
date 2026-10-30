// The attention band: one alert per problem, worst first, each with its fix.

import { setAttention } from "/static/kp/js/attention.js";
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

/** How many problems show before "Show N more": the worst come first, the rest wait one tap away. */
const SHOWN = 3;

/**
 * @param {HTMLElement} wrap the element made by attentionBand()
 * @param {any[]} items the API's attention list
 * @param {Map<number, any>} meds
 * @param {() => void} reload
 */
export function renderAttention(wrap, items, meds, reload) {
  const band = /** @type {HTMLElement} */ (wrap.querySelector(".kp-attention"));
  const more = /** @type {HTMLButtonElement} */ (wrap.querySelector(".mc-attention__more"));
  const expanded = wrap.dataset.expanded === "1";
  const worst = [...items].sort((a, b) => rank(a) - rank(b));
  const shown = expanded ? worst : worst.slice(0, SHOWN);
  more.hidden = worst.length <= SHOWN;
  more.textContent = expanded ? "Show fewer" : `Show ${worst.length - SHOWN} more`;
  more.setAttribute("aria-expanded", String(expanded));
  setAttention(band, shown.map((a) => {
    const med = meds.get(a.medicine_id);
    const stock = a.kind === "out_of_stock" || a.kind === "low_stock";
    return {
      key: `${a.medicine_id}-${a.kind}`,
      severity: SEVERITY[/** @type {keyof typeof SEVERITY} */ (a.kind)] ?? "info",
      title: `${a.medicine_name} ${TITLE[/** @type {keyof typeof TITLE} */ (a.kind)] ?? a.kind}`,
      text: humanize(a.detail),
      action: stock && med
        ? button("Add stock", { look: "kp-button--sm", onClick: () => addStock(med, reload), label: `Add stock to ${a.medicine_name}` })
        : linkButton("Open", `/medicine/${a.medicine_id}`, "kp-button--sm"),
    };
  }));
}

const rank = (/** @type {any} */ a) => ["critical", "warning", "info"].indexOf(SEVERITY[/** @type {keyof typeof SEVERITY} */ (a.kind)] ?? "info");

/** The band with its "show more" button; `reload` redraws after the button is pressed. @param {() => void} redraw */
export function attentionBand(redraw) {
  const more = h("button", { type: "button", class: "kp-button kp-button--ghost mc-attention__more", hidden: true });
  const wrap = h("div", { class: "mc-today__attention" },
    h("div", { class: "kp-attention", role: "region", "aria-label": "Needs attention" }), more);
  more.addEventListener("click", () => {
    wrap.dataset.expanded = wrap.dataset.expanded === "1" ? "0" : "1";
    redraw();
  });
  return wrap;
}
