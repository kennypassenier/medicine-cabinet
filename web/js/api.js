// The data layer the pages call. Reads come from the browser's local database
// and writes go through the kit's queue (see ./store/); the server stays the
// master and answers every write that is replayed. The names and result
// shapes are those of the server's /api/mc routes.

import { dayParam } from "./clock.js";
import { store } from "./store/index.js";

export { ApiError } from "./api-error.js";

export const api = store;

/** The planned stamp of a schedule slot today: "YYYY-MM-DDTHH:MM". @param {string} time "HH:MM" */
export const plannedToday = (time) => `${dayParam()}T${time}`;
