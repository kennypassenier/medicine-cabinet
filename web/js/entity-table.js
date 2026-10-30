// A small managed list as the package's data table: sortable columns, rows
// that become cards on a phone, and an actions column where every row has the
// same buttons in the same place. Redrawing keeps the sort the reader chose.

import { attachDataTables, dataTable } from "/static/kp/js/datatable.js";
import { h } from "./dom.js";

/**
 * @typedef {{ title: string, sort?: "text" | "number", numeric?: boolean, actions?: boolean }} Column
 * @typedef {{ key: string, cells: (Node | string | number)[], sortValues?: (string | number | undefined)[] }} Row
 */

/**
 * Put a data table of `rows` into `host` (replacing what was there).
 * @param {HTMLElement} host
 * @param {{ label: string, columns: Column[], rows: Row[], search?: string, pageSize?: number, none: string }} o
 * @returns {() => void} detach
 */
export function drawEntityTable(host, { label, columns, rows, search, pageSize = 8, none }) {
  const old = host.querySelector(".kp-datatable");
  const kept = old instanceof HTMLElement ? dataTable(old)?.view().sorts ?? [] : [];
  const head = h("tr", null, ...columns.map((c) => c.actions
    ? h("th", { scope: "col", class: "mc-actions-head" }, c.title)
    : h("th", { scope: "col", "data-kp-sort": c.sort ?? "text", class: c.numeric ? "kp-numeric" : null }, c.title)));
  const body = h("tbody", null, ...rows.map((r) => h("tr", { "data-kp-row-key": r.key },
    ...r.cells.map((cell, i) => h("td", { class: columns[i].numeric ? "kp-numeric" : columns[i].actions ? "mc-actions" : null }, cell)))));
  const wrapper = h("div", { class: "kp-datatable mc-entity", "data-kp-datatable": "", "data-kp-cards": "", "data-kp-page-size": pageSize, "data-kp-page-sizes": "none", "data-kp-region-label": label },
    search ? h("div", { class: "kp-datatable__bar" }, h("input", { class: "kp-datatable__search", type: "search", "data-kp-datatable-search": "", "aria-label": search, placeholder: search, autocomplete: "off" })) : null,
    h("div", { class: "kp-table-wrap" }, h("table", { class: "kp-table" }, h("thead", null, head), body)),
    h("div", { class: "kp-empty", "data-kp-datatable-empty": "", hidden: true }, none),
    h("div", { class: "kp-datatable__bar" }, h("p", { class: "kp-datatable__status", "data-kp-datatable-status": "", role: "status", "aria-live": "polite" }), h("div", { class: "kp-datatable__pager", "data-kp-datatable-pager": "" })));
  host.replaceChildren(wrapper);
  const detach = attachDataTables(host);
  const handle = dataTable(wrapper);
  const first = columns.findIndex((c) => !c.actions);
  const sorts = kept.length ? kept : [{ column: first, direction: /** @type {const} */ ("ascending") }];
  handle?.sortBy(sorts);
  return detach;
}

/** The two row buttons every managed row carries, in fixed columns. @param {{ label: string, onClick: () => void, look?: string, aria: string }} b */
export function rowButton({ label, onClick, look = "", aria }) {
  return h("button", { type: "button", class: `kp-button kp-button--sm ${look}`.trim(), "aria-label": aria, onclick: onClick }, label);
}
