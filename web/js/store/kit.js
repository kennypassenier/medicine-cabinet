// The seam to the kit: the local database collections, the write queue and the
// translation of the kit's errors into the ApiError the pages already show.
// Nothing here keeps data of its own.

import { ApiError } from "../api-error.js";

export const BASE = "/api/mc";

/** The kit's client; the runtime puts it on the window before any module runs. */
export function kit() {
  const client = /** @type {any} */ (window).chassisData;
  if (!client) throw new ApiError("The offline data client is not loaded.", "Reload the page.", 0);
  return client;
}

/** @param {string} name the collection: medicines, intakes, symptoms, categories, forms */
export const col = (name) => kit().db.collection(name);

/** Any failure as an ApiError. @param {any} e */
export function toApiError(e) {
  if (e instanceof ApiError) return e;
  const body = e?.body;
  if (body && typeof body.error === "string") return new ApiError(body.error, body.remedy ?? "", e.status ?? 0);
  const text = String(e?.message ?? e).replace(/^chassisData(\.db)?: /, "");
  return new ApiError(text, "Reload the page; if it keeps happening, check the connection.", e?.status ?? 0);
}

/** Run `fn`, turning every failure into an ApiError. @template T @param {() => Promise<T>} fn @returns {Promise<T>} */
export async function guard(fn) {
  try {
    return await fn();
  } catch (e) {
    throw toApiError(e);
  }
}

/** Pull what the server changed (the stock moves with an intake, a list count with a symptom). Reported, never fatal. */
export async function refresh() {
  try {
    await kit().db.sync();
  } catch (e) {
    console.warn("The local database could not be refreshed after a write.", e);
  }
}

/**
 * Send a write the way the kit does: at once when the queue is empty and the
 * server answers, queued with its If-Match otherwise.
 * @param {"PUT" | "POST" | "DELETE"} method @param {string} path @param {unknown} [body] @param {number | string} [rev]
 */
export async function send(method, path, body, rev) {
  const options = rev === undefined || rev === null ? {} : { rev };
  const client = kit();
  const result = method === "DELETE" ? await client.delete(path, options) : await client[method.toLowerCase()](path, body, options);
  if (!result.queued) await refresh();
  return result;
}

/**
 * A create that needs the server (it hands out the id). Without a connection it
 * is refused, not queued: nothing local could show it until it was made.
 * @param {string} what "A medicine" @param {string} path @param {object} body
 */
export async function createOnline(what, path, body) {
  const refused = new ApiError(`${what} can only be added with a connection.`, "Connect and try again.", 0);
  if (!navigator.onLine) throw refused;
  const result = await send("POST", path, body);
  if (!result.queued) return result.data;
  const mine = JSON.stringify(body);
  const entry = (await kit().queue()).findLast((/** @type {any} */ e) => e.method === "POST" && e.path === path && JSON.stringify(e.body) === mine);
  if (entry) await kit().drop(entry.id);
  throw refused;
}

/** Queued, not failed, edits (oldest first). */
export async function waiting() {
  return (await kit().queue()).filter((/** @type {any} */ e) => e.state !== "failed");
}

/** Take back edits still waiting in the queue; true when at least one went. @param {(e: any) => boolean} match */
export async function dropWaiting(match) {
  const hits = (await waiting()).filter(match);
  const dropped = await Promise.all(hits.map((e) => kit().drop(e.id)));
  return dropped.some(Boolean);
}

/** Case-insensitive ASCII order, like the server's NOCASE. @param {string} a @param {string} b */
export function nocase(a, b) {
  const x = a.replace(/[A-Z]/g, (c) => c.toLowerCase());
  const y = b.replace(/[A-Z]/g, (c) => c.toLowerCase());
  return x < y ? -1 : x > y ? 1 : 0;
}

/** Numeric ids: the kit's keys are text. @param {any} row */
export const numeric = (row) => ({ ...row, id: Number(row.id) });
