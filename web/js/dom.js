// Small DOM helpers every screen shares.

/**
 * Make an element: `h('p', { class: 'x', onclick: fn }, 'text', child)`.
 * Attributes starting with `on` become listeners; null/false/undefined are skipped.
 * @param {string} tag
 * @param {Record<string, any> | null} [attrs]
 * @param {...(Node | string | number | null | undefined | false)} children
 * @returns {HTMLElement}
 */
export function h(tag, attrs, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs ?? {})) {
    if (v === null || v === undefined || v === false) continue;
    if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v);
    else el.setAttribute(k, v === true ? "" : String(v));
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "number" ? String(c) : c);
  }
  return el;
}

/** @param {string} sel @param {ParentNode} [root] */
export const $ = (sel, root = document) => /** @type {HTMLElement | null} */ (root.querySelector(sel));

/** @param {string} sel @param {ParentNode} [root] */
export const $$ = (sel, root = document) => /** @type {HTMLElement[]} */ ([...root.querySelectorAll(sel)]);

/**
 * Redraw a container in place without losing the keyboard's place: the focused
 * control is found again by its `data-key` after the redraw (a 60 s refresh
 * must never throw focus to the top of the page).
 * @param {HTMLElement} container
 * @param {() => void} draw fills the container
 */
export function redraw(container, draw) {
  const active = document.activeElement;
  const inside = active instanceof HTMLElement && container.contains(active);
  // A control is found again by its row key, or by its id when it has no key.
  const key = inside ? active.dataset.key : undefined;
  const id = inside && !key ? active.id : "";
  const caret = inside && active instanceof HTMLInputElement && active.type === "text" ? [active.selectionStart, active.selectionEnd] : null;
  const scrollY = window.scrollY;
  draw();
  const again = key
    ? container.querySelector(`[data-key="${CSS.escape(key)}"]`)
    : id ? container.querySelector(`#${CSS.escape(id)}`) : null;
  if (again instanceof HTMLElement) {
    again.focus({ preventScroll: true });
    if (caret && again instanceof HTMLInputElement) again.setSelectionRange(caret[0], caret[1]);
  }
  if (window.scrollY !== scrollY) window.scrollTo({ top: scrollY });
}

/** An svg icon from path data (24 grid, stroked in the current colour). @param {string} d */
export function icon(d) {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  for (const [k, v] of Object.entries({ viewBox: "0 0 24 24", width: "24", height: "24", fill: "none", stroke: "currentColor", "stroke-width": "1.8", "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true", focusable: "false" })) svg.setAttribute(k, v);
  for (const part of d.split("|")) {
    const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
    p.setAttribute("d", part);
    svg.append(p);
  }
  return svg;
}
