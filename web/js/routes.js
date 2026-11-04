// The app's places: one list drives the top bar, the phone's tab bar and the
// router, so a screen is never named twice.

import { icon } from "./dom.js";

/** The four everyday areas, in tab order. `path` is also the bar entry a deeper screen lights up. */
export const AREAS = [
  { id: "today", path: "/", title: "Today", glyph: "M12 21a9 9 0 100-18 9 9 0 000 18z|M12 7v5l3.5 2", load: () => import("./pages/today.js") },
  { id: "cabinet", path: "/cabinet", title: "Cabinet", glyph: "M10.5 20.5a5 5 0 01-7-7l10-10a5 5 0 017 7z|M8.5 8.5l7 7", load: () => import("./pages/cabinet.js") },
  { id: "symptoms", path: "/symptoms", title: "Symptoms", glyph: "M11 19a8 8 0 100-16 8 8 0 000 16z|M21 21l-4.3-4.3", load: () => import("./pages/symptoms.js") },
  { id: "history", path: "/history", title: "History", glyph: "M3 12a9 9 0 109-9 9 9 0 00-6.4 2.6L3 8|M3 3v5h5|M12 7v5l3 2", load: () => import("./pages/history.js") },
];

/** The rarer places of this app, first in the "More" menu (the kit's own pages follow from its registry). */
export const MORE = [
  { id: "settings", path: "/settings", title: "Settings", hint: "Medicine forms and the unit each one counts in", load: () => import("./pages/settings.js") },
];

const FORM = () => import("./pages/medicine-form.js");

/**
 * The screen a path opens: its loader, its parameters and the bar entry it lights up;
 * the old /find address answers with a redirect to /symptoms.
 * @param {string} path @param {string} search
 */
export function route(path, search) {
  const params = new URLSearchParams(search);
  if (path === "/find") return { redirect: `/symptoms${search}` };
  const place = [...AREAS, ...MORE].find((a) => a.path === path);
  if (place) return { load: place.load, params, bar: place.path };
  if (path === "/medicine/new") return { load: FORM, params, bar: "/cabinet" };
  const m = path.match(/^\/medicine\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/i);
  if (m) {
    params.set("id", m[1]);
    return { load: FORM, params, bar: "/cabinet" };
  }
  return null;
}

/** One tab of the phone's bar. @param {typeof AREAS[number]} a */
export function tabMarkup(a) {
  const tab = document.createElement("a");
  tab.className = "kp-tabbar__tab";
  tab.href = a.path;
  tab.dataset.kpTab = a.path;
  const ic = document.createElement("span");
  ic.className = "kp-tabbar__icon";
  ic.append(icon(a.glyph));
  const label = document.createElement("span");
  label.className = "kp-tabbar__label";
  label.textContent = a.title;
  tab.append(ic, label);
  return tab;
}
