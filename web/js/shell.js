// The app shell's navigation: the top bar (four areas and a "More" menu on a
// desktop), the phone's tab bar (four tabs and a "More" tab with a drawer) and
// which entry is lit. The kit's own pages (Status, Clients, Storage) come from
// the server's page registry and live under "More" next to this app's Settings.

import { attachMenuButtons, setMenu, attachTabbars, setTabbarCurrent, attachDialogs } from "/static/kp/dist/kp-themes.min.js";
import { h } from "./dom.js";
import { AREAS, MORE, tabMarkup } from "./routes.js";

const nav = /** @type {HTMLElement} */ (document.getElementById("nav"));
const tabbar = /** @type {HTMLElement} */ (document.getElementById("tabbar"));

/** @typedef {{ path: string, title: string, hint?: string }} Place */

/** The "More" places the kit's page registry lists (Settings, Status, Clients, Storage). @returns {Promise<Place[]>} */
async function morePlaces() {
  const own = MORE.map(({ path, title, hint }) => ({ path, title, hint }));
  try {
    const res = await fetch("/api/kit/pages", { credentials: "same-origin" });
    if (!res.ok) throw new Error(`The page registry answered ${res.status}.`);
    const set = await res.json();
    // The kit says which pages live in the one More menu (`more`: their ids, in order).
    const byId = new Map(set.pages.map((/** @type {any} */ p) => [p.id, p]));
    const listed = (set.more ?? []).map((/** @type {string} */ id) => byId.get(id)).filter(Boolean);
    return listed.map((/** @type {any} */ p) => ({ path: p.path, title: p.title, hint: p.hint ?? undefined }));
  } catch (e) {
    console.warn("kit page registry unavailable", e);
    return own;
  }
}

/** The desktop "More" entry of the top bar: a menu button. @param {Place[]} places */
function moreMenu(places) {
  const wrapper = h("div", { class: "kp-menu-button", "data-kp-menu-button": "", "data-kp-key": "nav-more" },
    h("button", { type: "button", class: "kp-nav__link kp-nav__disclosure" }, "More"),
    h("div", { class: "kp-menu kp-menu--rich", role: "menu", "aria-label": "More pages", hidden: true }));
  nav.append(h("li", null, wrapper));
  attachMenuButtons(nav);
  setMenu(wrapper, [{ group: "", items: places.map((p) => ({ label: p.title, hint: p.hint, href: p.path })) }]);
}

/** The phone's "More" tab and the drawer it opens. @param {Place[]} places */
function moreTab(places) {
  const tab = h("button", { type: "button", class: "kp-tabbar__tab kp-tabbar__tab--more", "data-kp-dialog": "more-drawer" },
    h("span", { class: "kp-tabbar__icon kp-tabbar__icon--more", "aria-hidden": "true" }),
    h("span", { class: "kp-tabbar__label" }, "More"));
  const drawer = h("dialog", { class: "kp-drawer kp-dialog", id: "more-drawer", "aria-labelledby": "more-drawer-t" },
    h("header", { class: "kp-drawer__head" },
      h("h2", { class: "kp-dialog__title", id: "more-drawer-t" }, "More"),
      h("button", { type: "button", class: "kp-icon-button kp-dialog__close", "data-kp-dialog-close": "", "aria-label": "Close" }, "×"),
      h("p", { class: "kp-drawer__desc" }, "Settings and the service's own pages.")),
    h("div", { class: "kp-drawer__body" },
      h("div", { class: "kp-menu kp-menu--rich", role: "group", "aria-label": "More pages" },
        ...places.map((p) => h("a", { class: "kp-menu__item", href: p.path },
          h("span", { class: "kp-menu__label" }, p.title), p.hint ? h("span", { class: "kp-menu__hint" }, p.hint) : null)))));
  // The app routes a link in place, so the page never reloads and the drawer must close itself.
  drawer.addEventListener("click", (e) => { if (/** @type {HTMLElement} */ (e.target).closest("a[href]")) drawer.close(); });
  tabbar.append(tab);
  document.body.append(drawer);
  attachDialogs(document);
  attachTabbars(document);
}

/** Light the entry of the current screen in the bar, the tab bar and the More drawer. @param {string} path */
export function markNav(path) {
  for (const a of document.querySelectorAll("#nav a, #more-drawer a")) {
    if (a.getAttribute("href") === path) a.setAttribute("aria-current", "page");
    else a.removeAttribute("aria-current");
  }
  const more = document.querySelector("#nav .kp-menu-button > button");
  const inMore = !!path && !AREAS.some((a) => a.path === path) && !!document.querySelector(`#nav [href="${CSS.escape(path)}"]`);
  if (inMore) more?.setAttribute("aria-current", "page");
  else more?.removeAttribute("aria-current");
  setTabbarCurrent(tabbar, path);
}

/** Draw the bar and the tab bar once; `current` is the path to light afterwards. @param {string} current */
export async function drawShell(current) {
  nav.replaceChildren(...AREAS.map((a) => h("li", null, h("a", { class: "kp-nav__link", href: a.path }, a.title))));
  tabbar.replaceChildren(...AREAS.map(tabMarkup));
  const places = await morePlaces();
  moreMenu(places);
  moreTab(places);
  markNav(current);
}
