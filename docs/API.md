# JSON API (`/api/mc`)

The routes the browser app talks to. All of them are defined in
`src/api/mod.rs` and mounted with `app.dashboard_routes(...)` in `src/main.rs`.
Every example below was produced by running the debug binary
(`scripts/dev.sh`, demo data) and calling it with `curl`; where a body is
shortened that is said.

## Contents

1. [Calling it](#calling-it): authentication, the `now` convention, errors
2. [Shapes](#shapes)
3. [Medicines](#medicines)
4. [Intakes and the dosing guard](#intakes-and-the-dosing-guard)
5. [Today and search](#today-and-search)
6. [Symptoms, categories, forms](#symptoms-categories-forms)
7. [Merged page endpoints](#merged-page-endpoints)
8. [Behaviour that may surprise you](#behaviour-that-may-surprise-you)

## Calling it

**Authentication.** The routes sit behind the admin login: a browser session
cookie, or the admin token (`MEDICINE_CABINET_TOKEN`) as a bearer:

```bash
curl -H "Authorization: Bearer $MEDICINE_CABINET_TOKEN" "http://127.0.0.1:8099/api/mc/forms"
```

With no or a wrong credential: no token gives `303 See Other` with
`location: /login` (an empty body); a wrong bearer gives `401` and

```json
{"error":"the bearer token is not this service's MEDICINE_CABINET_TOKEN","remedy":"send the value of MEDICINE_CABINET_TOKEN from the service's environment file (`chassis clients` reads it from the variable named by --token-env); a client token cannot manage clients"}
```

Client tokens from the Clients page are for the kit's `api_routes`; this
service has none. I did not try a client token against `/api/mc`.

**Format.** Request and response bodies are JSON. Send
`Content-Type: application/json` on every request with a body. Numbers are JSON
numbers (`stock` and `dose` are floats, `1.0` in responses). Dates are
`YYYY-MM-DD`, times `HH:MM`, and local moments `YYYY-MM-DDTHH:MM` with no zone.

**The `now` convention.** The server has no clock opinion: the browser sends
its own local date and time in the query string as `now=YYYY-MM-DDTHH:MM`
(`web/js/clock.js` `nowParam`). Routes that need "today" require it:

| Needs `?now=` | Does not |
|---|---|
| `GET /medicines`, `GET /medicines/{id}`, `POST /medicines`, `PUT /medicines/{id}`, `POST /medicines/{id}/active`, `/stock`, `/restore`, `GET /medicines/{id}/dose-check`, `GET /today`, `GET /search` | `DELETE /medicines/{id}`, everything under `/intakes`, `/symptoms`, `/categories`, `/forms` |

`dose-check` and `today` use the date and the time; the others only the date
(a trailing time is ignored). Missing or malformed `now`:

```
$ curl -H "$AUTH" "$B/api/mc/medicines"
Failed to deserialize query string: missing field `now`        [400, plain text]
$ curl -H "$AUTH" "$B/api/mc/medicines?now=2026-13-45T10:00"
{"error":"'2026-13-45T10:00' is not a date","remedy":"use the form YYYY-MM-DD, for example 2026-10-30"}   [400]
$ curl -H "$AUTH" "$B/api/mc/today?now=2026-11-02T25:00"
{"error":"'2026-11-02T25:00' is not a time","remedy":"use the form HH:MM"}   [400]
```

**Errors.** An error raised by the app or the kit is
`{"error": <what happened>, "remedy": <what to do>}` with a status from its
kind (`chassis` `Kind::status`):

| Status | Kind | Seen here when |
|---|---|---|
| 400 | invalid | a field fails validation; a duplicate name |
| 404 | not found | the id does not exist (or is soft-deleted) |
| 502 | dependency | the database refused the request (a body reference to an unknown uid is a `400`, not this) |
| 500 | internal | `error` is replaced by `internal error`; the cause is only in the log |

Rejections made before the app code runs are **plain text, not JSON**, and the
browser app shows them as they are: `400 Failed to deserialize query string: ...`,
`400 Failed to parse the request body as JSON: ...`,
`422 Failed to deserialize the JSON body into the target type: missing field `name` ...`,
`415 Expected request with `Content-Type: application/json``.
The kit adds its own limits on top: bodies over 1 MiB are 413, requests over
64 in flight are 503, requests slower than 30 s are 408 (defaults, see the
README's configuration section).

The messages of the app's own errors, exactly as in the source:

| Message (`error`) | `remedy` | Where |
|---|---|---|
| `a medicine needs a name` | `fill in the name field` | `src/domain/medicine.rs` |
| `stock cannot be negative` | `enter 0 or a positive amount` | `src/domain/medicine.rs` |
| `'<text>' is not a date` | `use the form YYYY-MM-DD, for example 2026-10-30` | `src/domain/date.rs` |
| `'<text>' is not a time` | `use the form HH:MM` | `src/domain/date.rs` |
| `a schedule slot needs at least one weekday` | `pick one or more days` | `src/domain/schedule.rs` |
| `a dose must be more than zero` | `enter the amount taken at once` (slot) or `enter the amount that was taken` (intake edit) | `src/domain/schedule.rs`, `src/store/intakes.rs` |
| `status '<s>' is not known` | `use 'taken' or 'skipped'` | `src/store/intakes.rs` |
| `a symptom needs a name` | `fill in the name field` | `src/store/symptoms.rs` |
| `a category needs a name` | `fill in the name field` | `src/store/categories.rs` |
| `a form needs a name` | `fill in the name field` | `src/store/forms.rs` |
| `The symptom '<n>' already exists` | `Pick another name, or edit the existing one.` | `src/store/symptoms.rs` |
| `The category '<n>' already exists` | `Pick another name, or edit the existing one.` | `src/store/categories.rs` |
| `The form '<n>' already exists` | `Pick another name, or edit the existing one.` | `src/store/forms.rs` |
| `that <what> does not exist` (`medicine`, `intake`, `symptom`, `category`, `form`, `deleted medicine`); `the <what> <uid> does not exist` for a uid in a URL | `reload the list; it may have been deleted` | `src/store/mod.rs`, `src/store/refs.rs` |
| `` `<text>` is not a UUID `` | `create the item with an id made by crypto.randomUUID(), e.g. ...` | `src/store/refs.rs` (kit `client_key`) |
| `` `<field>`: no <what> has the id <uid> `` / `` `<field>`: `<text>` is not a UUID `` | `send the id of an existing <what> as `<field>`` | `src/store/refs.rs` |
| `the database refused the request: <sqlite text>` | `check the Storage page for the database state; if it is full or locked, retry in a moment` | `src/store/mod.rs` |

## Identifiers

Medicines, symptoms, categories and forms are known over the API and the sync
feed by a **`uid`**: a UUID text such as `3f2b8c1e-5a4d-4e0a-9b1c-2d7e6f8a9b0c`,
shown in the JSON field **`id`**. The server's integer key never leaves the
server (it stays the primary and foreign key inside the database). Why: a device
must be able to create these while offline, under their final id, and link
them at once (a new medicine with a new form, a new symptom in a new category);
an integer handed out by the server cannot do that.

* Every reference between them is a uid string too: `form_id` of a medicine,
  `category_id` of a symptom, `medicine_id` of an intake, `medicine_id` in the
  Today doses, attention items and search results, and the lists `medicine_ids`
  and `symptom_ids`.
* Route parameters `{id}` of these four kinds are the uid. A malformed one is
  `400` (`` `7` is not a UUID ``), an unknown one `404`.
* A reference in a request body to an unknown uid is `400` and names the field
  (`` `form_id`: no form has the id ... ``).
* Intakes keep their own `uid` and their integer `id`; their routes accept both
  (`/intakes/3` or `/intakes/<uid>`).
* Create routes (`POST /medicines`, `/symptoms`, `/categories`, `/forms`) take
  an optional `id` (a well-formed UUID the device made) and the
  `Idempotency-Key` header. An `id` the server does not hold creates; the same
  `Idempotency-Key` again returns the first answer and changes nothing (no
  second row, no second stock or side effect); an `id` held by a request with
  another key (or none) is `409` `{"code":"exists"}`. Without an `id` the
  server makes one. They answer `{"id": "<uid>", "rev": n}` (a medicine create
  no longer needs `now` and no longer answers the whole Medicine: the device
  reads it from the feed).

## Shapes

**Slot** (a schedule row), in both directions:

```json
{"time":"20:00","days":31,"dose":2.0}
```

`time` is `HH:MM`. `days` is a bitmask, bit 0 = Monday ... bit 6 = Sunday,
`127` = every day, `31` = Monday to Friday; it must be 1 to 127. `dose` must be
above 0. On input `days` defaults to 127 and `dose` to 1.

**Medicine** (response of every medicine route):

| Field | Type | Meaning |
|---|---|---|
| `id` | string (uid) | |
| `rev` | integer | revision, bumped by every write |
| `name`, `purpose`, `strength`, `notes` | string | |
| `form` | string | the form's name, `""` when none |
| `form_id` | string (uid) or null | |
| `active` | boolean | switched on; off hides the schedule everywhere |
| `as_needed` | boolean | taken only when needed (has no schedule use) |
| `min_gap_minutes`, `max_per_day` | integer or null | the as-needed limits |
| `stock` | number | never negative |
| `stock_unit` | string | what stock is counted in; free text, **not** derived from the form |
| `low_stock_at` | number or null | warn at or below this |
| `expires_on`, `opened_on`, `course_ends_on` | `YYYY-MM-DD` or null | |
| `shelf_life_days` | integer or null | days good after `opened_on` |
| `symptoms` | string[] | lowercase names, sorted |
| `slots` | Slot[] | sorted by time |
| `effective_expiry` | date or null | the earlier of `expires_on` and `opened_on` + `shelf_life_days` |
| `freshness` | `fresh`, `expiring_soon` (within 30 days), `expired` | relative to `now` |
| `stock_level` | `ok`, `low` (at or below `low_stock_at`), `out` (0) | |
| `course_over` | boolean | `course_ends_on` is before today |

**Medicine body** (create and replace): `name` (required) and optionally
`purpose`, `form_id`, `strength`, `notes`, `active` (default `true`),
`as_needed` (default `false`), `min_gap_minutes`, `max_per_day`, `stock`
(default 0), `stock_unit`, `low_stock_at`, `expires_on`, `opened_on`,
`shelf_life_days`, `course_ends_on`, `symptoms` (string[]), `slots` (Slot[]).
The server trims the name; rejects an empty name, negative `stock` or
`low_stock_at`; parses the three dates (a blank string counts as none) and
stores them as `YYYY-MM-DD`; trims, lowercases and de-duplicates the symptoms
and creates the ones it does not know.

## Medicines

### `GET /api/mc/medicines?now=...`

All medicines that are not soft-deleted, switched-on first, then by name
(case-insensitive).

```
$ curl -H "$AUTH" "$B/api/mc/medicines/<medicine-uid>?now=2026-11-02T09:30"
{"id":"<medicine-uid>","name":"Ibuprofen","purpose":"Pain and fever","form":"Tablet","form_id":"<form-uid>","strength":"400 mg","notes":"","active":true,"as_needed":true,"min_gap_minutes":360,"max_per_day":3,"stock":12.0,"stock_unit":"tablets","low_stock_at":4.0,"expires_on":"2026-11-22","opened_on":null,"shelf_life_days":null,"course_ends_on":null,"symptoms":["fever","headache","muscle pain","toothache"],"slots":[],"effective_expiry":"2026-11-22","freshness":"expiring_soon","stock_level":"ok","course_over":false}
```

The list returns an array of these.

### `GET /api/mc/medicines/{id}?now=...`

One medicine, same shape. `404` for an unknown or soft-deleted id:
`{"error":"the medicine eeeeeeee-5555-4555-8555-eeeeeeeeeeee does not exist","remedy":"reload the list; it may have been deleted"}`.

### `POST /api/mc/medicines?now=...`

Creates a medicine and its slots in one transaction; answers `{"id": <uid>, "rev": n}`. Body as for `PUT`, plus the optional `id` (see [Identifiers](#identifiers)); `form_id` is the form's uid. `now` is not read.

```
$ curl -H "$AUTH" -H "Content-Type: application/json" -H "Idempotency-Key: 0b9d1a52-7c1e-4d38-8a55-1f0e2d3c4b5a" -X POST "$B/api/mc/medicines" -d @create.json
# create.json:
{"id":"dddddddd-4444-4444-8444-dddddddddddd","name":" Docs Test ","form_id":"aaaaaaaa-1111-4111-8111-aaaaaaaaaaaa","stock":10,"stock_unit":"tablets","low_stock_at":3,"expires_on":"2027-01-31","symptoms":[" Cough ","cough","Sore Throat"],"slots":[{"time":"08:00"},{"time":"20:00","days":31,"dose":2}]}
# answer:
{"id":"dddddddd-4444-4444-8444-dddddddddddd","rev":2}
```

Validation errors (all `400`, JSON): `{"name":"  "}` gives `a medicine needs a name`;
`"stock":-1` gives `stock cannot be negative`; `"expires_on":"31/01/2027"`
gives `'31/01/2027' is not a date`; a slot `{"time":"8am"}` gives `'8am' is not a time`;
`"days":0` gives `a schedule slot needs at least one weekday`; `"dose":0` gives
`a dose must be more than zero`. A body without `name` is the plain-text `422`.

### `PUT /api/mc/medicines/{id}?now=...`

**Replaces** the medicine: same body as create, and every field you leave out
goes back to its default, including `stock_unit`, `form_id`, `symptoms` and
`slots`. The schedule rows are deleted and rewritten from `slots`. Send the
whole medicine.

```
$ ... -X PUT -d '{"name":"Docs Test 2","stock":8,"slots":[{"time":"09:00"}]}' "$B/api/mc/medicines/<medicine-uid>?now=..."
{"id":"<medicine-uid>","name":"Docs Test 2","purpose":"","form":"","form_id":null,...,"stock":8.0,"stock_unit":"","low_stock_at":null,"expires_on":null,...,"symptoms":[],"slots":[{"time":"09:00","days":127,"dose":1.0}],...}
```

(`...` marks fields that were unchanged defaults.) `404` for an unknown id.

### `POST /api/mc/medicines/{id}/active?now=...`

Body `{"active": false}`; switches the medicine on or off and answers the
Medicine. Nothing else changes: the schedule is derived from `active`.

### `POST /api/mc/medicines/{id}/stock?now=...`

Body `{"delta": 5}`: adds to the stock; a negative number corrects downwards.
The result is clamped at 0 (a `delta` of -100 on a stock of 8 leaves 0).
Answers the Medicine. This is the "Add stock" button, the Cabinet stepper
(which sends the difference) and the undo of both.

### `DELETE /api/mc/medicines/{id}`

**Soft delete.** Answers `{"deleted": "<uid>", "rev": n}`. The medicine, schedule, symptom links
and intakes stay for 14 days and are invisible meanwhile (`GET` answers 404).
A second delete answers 404.

### `POST /api/mc/medicines/{id}/restore?now=...`

Brings a soft-deleted medicine back (inside the 14 days) and answers the
Medicine. For a medicine that is not deleted, or already purged:
`{"error":"that deleted medicine does not exist","remedy":"reload the list; it may have been deleted"}` (404).
The 14 days are checked by a purge that the reminder loop runs every 30
seconds, not at restore time.

## Intakes and the dosing guard

An **intake** is one logged dose:

```json
{"id":3,"uid":"8a1f0c52-0d2e-4b7a-9c3d-5e6f7a8b9c0d","rev":1,"medicine_id":"e7eeec8f-f253-4ab1-9021-8fd9be415097","medicine_name":"Amoxicillin","taken_at":"2026-11-02T08:10","planned_for":"2026-11-02T08:00","status":"skipped","dose":1.0,"note":""}
```

`planned_for` is the schedule slot the intake answers (Today's Take/Skip send
it; as-needed and "earlier dose" entries have `null`). `status` is `taken` or
`skipped`.

### `GET /api/mc/intakes?day=...&limit=...`

Newest first (by `taken_at`, as text). `day` is a **prefix** of `taken_at`:
`2026-11-02` is a day, `2026-11` a month, absent means all. `limit` is 1 to
1000 (clamped), default 200. Intakes of soft-deleted medicines are left out.
A non-numeric `limit` is the plain-text `400`.

```
$ curl -H "$AUTH" "$B/api/mc/intakes?day=2026-11-02"
[{"id":3,"uid":"8a1f0c52-0d2e-4b7a-9c3d-5e6f7a8b9c0d","rev":1,"medicine_id":"e7eeec8f-f253-4ab1-9021-8fd9be415097","medicine_name":"Amoxicillin","taken_at":"2026-11-02T08:10","planned_for":"2026-11-02T08:00","status":"skipped","dose":1.0,"note":""},
 {"id":2,"medicine_id":"<uid>","medicine_name":"Vitamin D","taken_at":"2026-11-02T08:05","planned_for":"2026-11-02T08:00","status":"taken","dose":1.0,"note":""},
 {"id":1,"medicine_id":"<uid>","medicine_name":"Paracetamol","taken_at":"2026-11-02T08:00","planned_for":null,"status":"taken","dose":2.0,"note":"headache"}]
```

(One line per entry added here for reading; the server sends one line.)

### `POST /api/mc/intakes`

Body: `medicine_id` (the medicine's uid) and `taken_at` (required), `uid` (optional, made by the device), `planned_for` (optional), `status`
(default `taken`), `dose` (default 1), `note` (default empty). A `taken` intake
subtracts `dose` from the stock (never below 0); `skipped` does not. Answers
`{"id": 7, "uid": "<uid>", "rev": 1}`.

```
$ ... -X POST -d '{"medicine_id":"dddddddd-4444-4444-8444-dddddddddddd","taken_at":"2026-11-02T08:00","dose":2,"note":"headache"}' "$B/api/mc/intakes"
{"id":1,"uid":"99999999-6666-4666-8666-999999999999","rev":1}
```

Errors: unknown or malformed `medicine_id` `400` naming the field; `status` other
than `taken` or `skipped` gives `status 'late' is not known`; a `taken_at` that
does not start with a valid date gives `'yesterday' is not a date`. See
[behaviour that may surprise you](#behaviour-that-may-surprise-you): `dose` and
the time part of `taken_at` are not validated here.

### `PUT /api/mc/intakes/{id}`

Corrects a logged intake. Body: `taken_at`, `dose` (must be above 0) and
optionally `note` (default empty); it **replaces** all three. For a `taken`
intake the stock moves by the difference between the old and new dose.
Answers `{"id": 1}`. `404` for an unknown id.

```
# Paracetamol stock 0.0 after the 2-tablet intake above
$ ... -X PUT -d '{"taken_at":"2026-11-02T08:15","dose":1,"note":"half"}' "$B/api/mc/intakes/1"
{"id":1}
# stock is now 1.0
```

### `DELETE /api/mc/intakes/{id}`

Removes the intake (hard delete); a `taken` one gives its dose back to the
stock. Answers `{"undone": 1}`; a second call is `404`. The app's "Undo" of a
Take uses this; its "Undo" of a removal posts the intake again.

### `GET /api/mc/medicines/{id}/dose-check?now=...`

May an as-needed medicine be taken right now? Looks at the `taken` intakes of
that medicine (the latest 50) that happened before `now`, and applies the
medicine's `min_gap_minutes` first, then `max_per_day` over the last 24 hours
(`src/domain/schedule.rs` `check_dose`). It counts intakes, not amounts: a
dose of 2 tablets is one dose.

```
{"allowed":true,"reason":null,"wait_minutes":null}
{"allowed":false,"reason":"The last dose was 90 minutes ago; wait 150 more.","wait_minutes":150}
{"allowed":false,"reason":"2 doses in the last 24 hours is the maximum (2).","wait_minutes":1230}
```

Answers `200` in all three cases; refusal is only advice (`allowed: false`),
`POST /intakes` does not consult it. In the last example the oldest of the two
doses was 210 minutes ago, so it ages out of the 24 hours in 1230 minutes.
A medicine with neither limit is always allowed. A scheduled medicine can be
checked too; it simply has no limits.

## Today and search

### `GET /api/mc/today?now=...`

The Today screen in one call:

```json
{
  "doses": [
    {"medicine_id":"<uid>","medicine_name":"Amoxicillin","time":"08:00","dose":1.0,"unit":"capsules","state":"overdue","intake_id":null,"minutes_until":-90}
  ],
  "attention": [
    {"medicine_id":"<uid>","medicine_name":"Amoxicillin","kind":"course_ends_soon","detail":"The course ends on 2026-11-04."},
    {"medicine_id":"<uid>","medicine_name":"Cetirizine","kind":"out_of_stock","detail":"None left."},
    {"medicine_id":"<uid>","medicine_name":"Ibuprofen","kind":"expiring_soon","detail":"Good until 2026-11-22."},
    {"medicine_id":"<uid>","medicine_name":"Paracetamol","kind":"low_stock","detail":"Only 2 tablets left."}
  ],
  "as_needed": [ {"id":"<uid>","name":"Cetirizine", "...": "full Medicine objects"} ]
}
```

(Shortened to one dose and with the `as_needed` objects abbreviated; the real
answer lists every dose.)

* `doses`: today's scheduled slots of switched-on, scheduled medicines whose
  course is not over and whose weekday mask includes today, sorted by time then
  name. `state` is `taken` or `skipped` when an intake with the same
  `planned_for` (`<today>T<time>`) exists (`intake_id` is then set), otherwise
  by the clock: `upcoming` (before its time), `due` (up to 60 minutes after),
  `overdue` (later). `minutes_until` is negative once late.
* `attention`: for switched-on medicines, in this order per medicine:
  `out_of_stock` ("None left."), `low_stock` ("Only N unit left."), `expired`
  ("Expired on DATE."), `expiring_soon` ("Good until DATE.", within 30 days),
  `course_ends_soon` ("The course ends on DATE.", 0 to 3 days before the end).
* `as_needed`: switched-on as-needed medicines whose course is not over.

### `GET /api/mc/search?symptom=...&now=...`

Both parameters are required. The symptom is trimmed and lowercased and
matched as a substring of each medicine's symptom names (`ache` matches
`headache`).

```json
{"symptom":"fever","usable":[ ...Medicine ],"unavailable":[ ...Medicine ]}
```

`usable` = switched on, not out of stock, not expired, course not over;
`unavailable` = matches but fails one of those. Called with `symptom=%20Fever%20`
the demo data answered `symptom: "fever"`, `usable`: Amoxicillin, Ibuprofen,
Paracetamol, and an empty `unavailable`. Scheduled medicines are included too.
A missing `symptom` is the plain-text
`400 Failed to deserialize query string: missing field `symptom``.

An empty `symptom=` is not an error: every medicine that lists at least one
symptom matches (verified: 8 of 11 medicines, the three without symptoms were
left out). The browser app never sends an empty one.

## Symptoms, categories, forms

These three lists have the same shape of routes. Names are unique
**case-insensitively**; a duplicate is `400` with the `already exists` message.
Symptom names are trimmed and lowercased; category and form names are trimmed
and keep their case. Create answers `{"id": "<uid>", "rev": n}` (see [Identifiers](#identifiers)); update answers the same.

### Symptoms

`GET /api/mc/symptoms` lists them by name:

```json
{"id":"801b3ca3-e907-4518-8bf2-f2efd644fc5c","rev":2,"name":"fever","category_id":"55aa1d38-366a-4056-9c13-eb84605de26e","category":"General","medicines":3}
```

`medicines` counts the medicines that list the symptom (including soft-deleted
ones; the count does not look at `deleted_at`).

* `POST /api/mc/symptoms` body `{"id": "<uid>", "name": "wheeze", "category_id": "<uid>", "medicine_ids": ["<uid>"]}`.
  `id`, `category_id` and `medicine_ids` are optional. `medicine_ids` links the new
  symptom to those medicines; the app uses it to undo a delete. A uid in
  `category_id` or `medicine_ids` that no row holds is a `400` naming the field.
* `PUT /api/mc/symptoms/{id}` body `{"name": ..., "category_id": ...}`; replaces
  both (send `"category_id": null` for none). `medicine_ids` is ignored.
* `DELETE /api/mc/symptoms/{id}` unlinks it from every medicine and says which:

```
{"deleted":"<uid>","rev":8,"was":{"category_id":null,"medicine_ids":["<uid>"],"name":"wheezing"}}
```

`was` is exactly the body that `POST /api/mc/symptoms` needs to put it back
(under a new `id`, or the old one if the device sends it).

### Categories

`GET /api/mc/categories`, ordered by name: `{"id":"<uid>","rev":2,"name":"Eyes","symptoms":1}`
(`symptoms` is how many symptoms are filed under it).

* `POST /api/mc/categories` body `{"id": "<uid>", "name": "Respiratory", "symptom_ids": ["<uid>"]}`;
  `id` is optional; `symptom_ids` (optional) files those symptoms under the new category.
* `PUT /api/mc/categories/{id}` body `{"name": "Breathing"}` renames.
* `DELETE /api/mc/categories/{id}`: its symptoms stay, uncategorised.
  `{"deleted":"<uid>","rev":8,"was":{"name":"Respiratory","symptom_ids":["<uid>"]}}`.

### Forms

`GET /api/mc/forms`, ordered by name:
`{"id":"<uid>","rev":1,"name":"Capsule","unit":"capsules","medicines":3}`. The first migration
that creates the table adds nine: Tablet (tablets), Capsule (capsules), Syrup
(ml), Drops (drops), Cream (g), Spray (doses), Inhaler (puffs), Patch (patches),
Injection (doses).

* `POST /api/mc/forms` body `{"id": "<uid>", "name": "Lozenge", "unit": "lozenges", "medicine_ids": ["<uid>"]}`;
  `id` is optional; `unit` defaults to empty and is trimmed; `medicine_ids` sets this form on those
  medicines.
* `PUT /api/mc/forms/{id}` body `{"name": ..., "unit": ...}`. The `unit` of a
  form is a suggestion for new medicines; renaming it does not change the
  `stock_unit` of medicines already saved.
* `DELETE /api/mc/forms/{id}`: medicines keep existing and lose their form.
  `{"deleted":"<uid>","rev":7,"was":{"medicine_ids":["<uid>"],"name":"Tablet","unit":"tablets"}}`.

## Merged page endpoints

Kenny's rule (2026-11-04): every resource keeps its own endpoint (all of the
above remain), and a page that uses several resources gets ONE merged endpoint
so it loads with a single call. The browser app uses them when the browser
blocks its local database (a private window); it is the fallback read path.

### `GET /api/mc/pages/{name}?now=...`

`now` is the local time, as for the individual endpoints. Every part is the
JSON its individual endpoint answers for the same query (byte-identical), read
in one read transaction, so the parts agree with each other. An unknown name
answers the kit's `404` JSON.

| `name` | answer | the parts are |
|---|---|---|
| `today` | `{today, medicines}` | `GET /today`, `GET /medicines` |
| `cabinet` | `{medicines, forms, categories}` | `GET /medicines`, `GET /forms`, `GET /categories` |
| `symptoms` | `{symptoms, categories, medicines}` | `GET /symptoms`, `GET /categories`, `GET /medicines` |
| `history` | `{intakes, medicines}` | `GET /intakes` (its default `limit` of 200, every day), `GET /medicines` |
| `settings` | `{forms, categories, symptoms}` | `GET /forms`, `GET /categories`, `GET /symptoms` |

```
$ curl -H "$AUTH" "$B/api/mc/pages/nope?now=2026-11-04T09:00"
{"error":"there is no page endpoint called 'nope'","remedy":"use today, cabinet, symptoms, history or settings"}
```

## Behaviour that may surprise you

Found by calling the running service; none of it is hidden in the app because
the browser app never sends these inputs.

* `PUT /medicines/{id}` replaces everything, see above.
* `POST /intakes` validates only that `taken_at` *starts* with a real date.
  `"taken_at":"2026-11-02"` and `"2026-11-02T99:99"` were stored as given, and
  then break nothing visible except sorting.
* `POST /intakes` does not check `dose`. `"dose": -5` was accepted and **added**
  5 to the stock (59 to 64). It also does not consult the dosing guard.
* `POST /intakes` for a soft-deleted medicine succeeds (the existence check
  ignores `deleted_at`), and so does the stock change of
  `POST /medicines/{id}/stock` on one, although that call then answers `404`
  because it re-reads the medicine. Both changes were still applied and
  showed after a restore.
* A taken intake with a dose larger than the stock leaves the stock at 0, so
  removing that intake afterwards gives back the full dose: stock can end
  higher than it started.
* `DELETE /medicines/{id}`, the intake routes and the three list routes ignore
  `now`; a `now` you send there is not read.
