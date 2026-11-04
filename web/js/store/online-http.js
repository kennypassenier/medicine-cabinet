// Plain fetch to /api/mc for the online-only mode. Failures become the same
// ApiError the local path throws, with the server's own sentence and remedy.

import { ApiError } from "../api-error.js";
import { nowParam } from "../clock.js";
import { BASE } from "./kit.js";

/** What to do next, by the kind of refusal. @param {number} status */
const remedyFor = (status) => {
  if (status === 401 || status === 403) return "Log in again.";
  if (status >= 500) return "The server had a problem. Try again in a moment.";
  return "Check the entries and try again.";
};

/** @param {Response} res */
async function failure(res) {
  const text = await res.text();
  try {
    const j = JSON.parse(text);
    if (j && typeof j.error === "string") return new ApiError(j.error, j.remedy ?? "", res.status);
  } catch {
    // Not JSON: the framework's plain-text refusal, shown as it is below.
  }
  // A plain-text body is shown only for a short client refusal; a 5xx body never reaches a toast.
  const plain = res.status < 500 && text.trim().length <= 200 ? text.trim() : "";
  return new ApiError(plain || `The server answered ${res.status}.`, remedyFor(res.status), res.status);
}

/**
 * @param {string} method @param {string} path under /api/mc
 * @param {{ query?: Record<string, string | number | undefined>, body?: unknown, timed?: boolean }} [opts] `timed` adds the browser's `now=`
 */
export async function call(method, path, { query = {}, body, timed = false } = {}) {
  const params = new URLSearchParams();
  if (timed) params.set("now", nowParam());
  for (const [k, v] of Object.entries(query)) if (v !== undefined && v !== "") params.set(k, String(v));
  const qs = params.toString();
  let res;
  try {
    res = await fetch(`${BASE}${path}${qs ? `?${qs}` : ""}`, {
      method,
      credentials: "same-origin",
      headers: body === undefined ? {} : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError("The server could not be reached.", "Check the connection and try again.", 0);
  }
  if (res.redirected && res.url.includes("/login")) {
    location.assign("/login");
    throw new ApiError("Your session has ended.", "Log in again.", 401);
  }
  if (!res.ok) throw await failure(res);
  try {
    return await res.json();
  } catch {
    throw new ApiError("The answer from the server could not be read.", "Reload the page; if it keeps happening, check the connection or log in again.", res.status);
  }
}
