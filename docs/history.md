# Historical data: design

How `meteo_ro` (a wasm eframe/egui app on GitHub Pages) browses **past** weather
snapshots without bloating the repo working tree.

## The idea: git history as the time-series DB

`meteoromania.ro` sends no CORS headers, so the wasm build can't fetch it
directly. The [`snapshot.yml`](../.github/workflows/snapshot.yml) workflow already
fetches the 4 endpoints server-side every 3 hours and commits them same-origin
under `data/{name}.json`, which the live app reads from `./data/{name}.json`.

The "200 IQ" move for history: **don't store snapshots in the working tree at
all** beyond the single latest copy. Each commit that touches `data/` already
*is* an immutable snapshot, addressable by its SHA. So the **git commit history
is the time-series database**, and the wasm app reads past versions straight
from the browser via two GitHub URLs that both advertise `access-control-allow-origin: *`:

1. **List snapshots** — the GitHub REST "list commits touching a path" endpoint.
2. **Fetch a snapshot's bytes** — `raw.githubusercontent.com` at a specific SHA.

No proxy, no server, no Pages-side state. The working tree only ever holds the
latest snapshot; everything older lives compressed in git's packfiles on
GitHub's side.

### The 4 endpoints (recap + data-quality caveats)

Base: `https://www.meteoromania.ro/wp-json/meteoapi/v2/{name}` → committed to
`data/{name}.json`.

| name | content | verified shape |
| --- | --- | --- |
| `starea-vremii` | stations / current weather | `object` (`{date, features, success, type}`), ~65 KB |
| `prognoza-orase` | city forecasts | `object`, ~8 KB |
| `avertizari-generale` | general alerts | `object`, up to ~685 KB |
| `avertizari-nowcasting` | nowcasting alerts | often a bare JSON **string** marker (~16 B) when no alerts |

Two real quirks the history reader must tolerate (both observed live, 2026-06):

- **Non-strict JSON.** `avertizari-generale` *intermittently* contains unescaped
  control characters (`U+0000..U+001F` inside strings), which strict RFC-8259
  parsers (`jq`, and `serde_json`) reject. The workflow therefore stores that
  payload **verbatim** when it can't be normalized (it never blanks or re-encodes
  it), so a historical read sees exactly what the live read saw. The existing
  `decode_alerts` already tolerates the second quirk below; the control-char case
  may still surface a `MeteoError::Decode` — handle it, don't panic.
- **String-marker payload.** `avertizari-nowcasting` (and sometimes `-generale`)
  can be a bare JSON string instead of an object. `src/fetch.rs::decode_alerts`
  already special-cases `Value::String(_) => Ok(Vec::new())`; the history reader
  must reuse that same decode path, not a stricter one.

> Because history reads the *committed bytes*, the historical decode path and the
> live decode path must be **the same function**. Reuse `dto` + `domain`, don't
> fork them.

## Read path — the two URLs

### (a) List commits touching a path  →  the snapshot index

```
GET https://api.github.com/repos/{owner}/{repo}/commits?path=data/starea-vremii.json&per_page=100
Accept: application/vnd.github+json
```

Returns a JSON **array**, newest commit first. Only the two fields we need:

```jsonc
[
  {
    "sha": "f0f849d96ff7c1ea57eca1de63a966e8f832baf5",
    "commit": { "committer": { "date": "2026-02-15T07:27:12Z" } }
  }
]
```

(Verified live against a public repo: the element has top-level `sha` and a
`commit.committer.date` ISO-8601 UTC string. There is also `commit.author.date`;
prefer `committer.date` — it's the time the bot actually committed the snapshot.)

Notes:
- **One file is enough.** Every cron run rewrites all 4 files in one commit, so
  the commit list for `data/starea-vremii.json` is effectively the global
  snapshot timeline. Query just that path.
- **Only real snapshots appear.** Because the workflow commits *only on change*,
  every commit in this list is a genuine new snapshot — no no-op noise.
- **Pagination.** `per_page` max is 100. For deeper history, follow the `Link`
  response header (`rel="next"` / `rel="last"`) or pass `&page=N`. At the 3-hour
  cadence that's 8 commits/day ≈ 240/month, so one 100-entry page ≈ 12 days.
- **Branch.** Add `&sha={branch}` to pin a branch; default is the repo's default
  branch. (Heads-up: this repo's local branch is `master`; GitHub's default for
  new repos is `main`. Make sure the workflow's push target and any `&sha=` agree
  — see "owner/repo & branch" below.)

### (b) Fetch a snapshot's bytes at a SHA

```
GET https://raw.githubusercontent.com/{owner}/{repo}/{sha}/data/starea-vremii.json
```

Verified: returns `200`, `access-control-allow-origin: *`, a strong `etag`, and
`content-type: text/plain` (fine — we parse the bytes ourselves). Content at a
given SHA is **immutable**, so it's safe to cache forever keyed by SHA, and it's
CDN-served (Fastly), so repeated reads are cheap.

## Rate limits & the plan to stay under them

| host | unauth limit | scope | mitigation |
| --- | --- | --- | --- |
| `api.github.com` | **60 requests / hour** | per client IP | only the commit-*list* hits this |
| `raw.githubusercontent.com` | no documented per-hour cap (CDN, immutable-by-SHA) | — | cache per SHA in memory |

Verified headers on the api call: `x-ratelimit-limit: 60`,
`x-ratelimit-remaining: 59`, `x-ratelimit-reset: <epoch>`.

So the **only scarce resource is the commit-list call**, and the budget is
generous because of how we use it:

1. **Fetch the commit list lazily and once.** Don't call the API on startup.
   Call it the first time the user opens *History* mode, then **cache the
   `Vec<Snapshot>` for the session**. One page (100 snapshots ≈ 12 days) is one
   request. Even paging back a full year is ~30 requests — well under 60/hr, and
   only if the user actually scrubs that far.
2. **Scrubbing costs nothing against the api budget.** Moving the slider only
   triggers `raw.githubusercontent.com` reads (not rate-limited the same way),
   and each SHA's bytes are cached after first load.
3. **Respect the headers.** On `403` with `x-ratelimit-remaining: 0`, surface a
   "rate limited, try again at {reset}" state instead of hammering. Optionally
   send `If-None-Match` with the stored `ETag` so an unchanged list returns `304`
   (a `304` does **not** count against the limit).
4. **Optional escape hatch:** a `data/history.json` index served same-origin
   removes the api call entirely for recent history — see the recommendation.

## The `history` module (proposed shape)

Mirror the existing `src/fetch.rs` exactly: an `Arc<Mutex<Vec<…>>>` inbox, a
plain `ehttp::fetch` call with a move-closure that pushes a result and calls
`ctx.request_repaint()`. **No reqwest, no tokio, no new crates.** The snippets
below are illustrative pseudo-Rust (they won't be compiled here); the real
implementation must avoid `unwrap`/`expect` and route every failure through a
named error enum.

### Types

```rust
// src/history.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::Station;
use crate::error::MeteoError;

/// One past snapshot = one commit that touched data/starea-vremii.json.
pub struct Snapshot {
    pub sha: String,
    pub timestamp: String, // ISO-8601 UTC, straight from commit.committer.date
}

/// Async results delivered back to the UI thread (mirrors fetch::Incoming).
pub enum HistoryIncoming {
    /// The snapshot index (already newest-first from the API).
    Index(Result<Vec<Snapshot>, MeteoError>),
    /// Decoded stations for one SHA (carry the sha so we know what we got back).
    Frame { sha: String, stations: Result<Vec<Station>, MeteoError> },
}

pub type HistoryInbox = Arc<Mutex<Vec<HistoryIncoming>>>;

pub fn new_inbox() -> HistoryInbox {
    Arc::new(Mutex::new(Vec::new()))
}

/// Session state for History mode.
pub struct History {
    pub snapshots: Vec<Snapshot>,         // index 0 = newest
    pub selected: usize,                  // which snapshot the slider points at
    pub frames: HashMap<String, Vec<Station>>, // sha -> decoded stations (cache)
    pub index_loaded: bool,
}
```

> Note the error enum: reuse `crate::error::MeteoError`. The existing variants
> (`Transport`, `HttpStatus(u16)`, `Decode(String)`) already cover the failure
> modes; the human may add e.g. `RateLimited { reset: u64 }` or `NoSnapshots` for
> nicer UX. **Do not** introduce a second ad-hoc error type, and **do not**
> `unwrap`/`expect` — propagate `Result<_, MeteoError>` exactly like `fetch.rs`.

### Compile-time config

```rust
// owner/repo, injected by CI, with a fallback for local dev.
const REPO_SLUG: &str = match option_env!("METEO_REPO_SLUG") {
    Some(s) => s,
    None => "ZmoleCristian/numabatevantule",
};

fn commits_url() -> String {
    format!(
        "https://api.github.com/repos/{REPO_SLUG}/commits\
         ?path=data/starea-vremii.json&per_page=100"
    )
}

fn raw_url(sha: &str) -> String {
    format!("https://raw.githubusercontent.com/{REPO_SLUG}/{sha}/data/starea-vremii.json")
}
```

### Fetch flow (same pattern as `fetch.rs`)

```rust
fn push(inbox: &HistoryInbox, msg: HistoryIncoming) {
    if let Ok(mut guard) = inbox.lock() {
        guard.push(msg);
    } // a poisoned mutex is the app's existing concern; don't add a new panic.
}

/// Step 1 — load the snapshot index (call once, lazily, on entering History).
pub fn load_index(ctx: &egui::Context, inbox: &HistoryInbox) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    let mut request = ehttp::Request::get(commits_url());
    // GitHub's REST API wants these. On wasm the browser sets User-Agent itself
    // (and forbids overriding it via fetch), so setting it here is a no-op there
    // and the correct thing on a native build.
    request.headers.insert("Accept", "application/vnd.github+json");
    request.headers.insert("User-Agent", "meteo_ro");
    ehttp::fetch(request, move |result| {
        let outcome = match result {
            Ok(resp) if resp.ok => decode_index(&resp.bytes),
            Ok(resp) => Err(MeteoError::HttpStatus(resp.status)), // 403 => rate limit
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, HistoryIncoming::Index(outcome));
        ctx.request_repaint();
    });
}

/// Step 2 — fetch + decode the stations for one SHA (only if not cached).
pub fn load_frame(ctx: &egui::Context, inbox: &HistoryInbox, sha: String) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    let request = ehttp::Request::get(raw_url(&sha));
    ehttp::fetch(request, move |result| {
        let stations = match result {
            Ok(resp) if resp.ok => decode_stations(&resp.bytes), // REUSE the live path
            Ok(resp) => Err(MeteoError::HttpStatus(resp.status)),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, HistoryIncoming::Frame { sha, stations });
        ctx.request_repaint();
    });
}

// DTO for the api response — only the fields we use.
#[derive(serde::Deserialize)]
struct CommitRow { sha: String, commit: CommitMeta }
#[derive(serde::Deserialize)]
struct CommitMeta { committer: CommitWho }
#[derive(serde::Deserialize)]
struct CommitWho { date: String }

fn decode_index(bytes: &[u8]) -> Result<Vec<Snapshot>, MeteoError> {
    let rows: Vec<CommitRow> = serde_json::from_slice(bytes)
        .map_err(|e| MeteoError::Decode(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| Snapshot { sha: r.sha, timestamp: r.commit.committer.date })
        .collect())
}

// Identical to fetch::decode_stations: dto::CurrentWeather -> domain::stations.
fn decode_stations(bytes: &[u8]) -> Result<Vec<Station>, MeteoError> {
    let parsed: crate::dto::CurrentWeather =
        serde_json::from_slice(bytes).map_err(|e| MeteoError::Decode(e.to_string()))?;
    crate::domain::stations(parsed)
}
```

Draining the inbox happens in `App::update` next to the existing
`fetch::Incoming` drain: on `Index(Ok(v))` store `snapshots`, set
`index_loaded`; on `Frame{sha, Ok(s)}` insert into `frames`.

### Time-slider UX sketch

```
┌──────────────────────────────────────────── History ──────┐
│  [ Live ]  ●━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━○      │
│            older                                    newer  │
│            2026-06-12 06:00Z   ◀ selected ▶   2026-06-27 09:00Z │
└────────────────────────────────────────────────────────────┘
```

- A **History** toggle (a HUD/menu button). Entering it calls `load_index` once.
- An `egui::Slider` (or a custom timeline) over `0..=snapshots.len()-1`. The api
  returns **newest-first**, so map the slider so the **right = newest**: render
  index `snapshots.len()-1 - slider_pos`, or store reversed. Label the handle
  with `snapshots[selected].timestamp`.
- On a change of `selected`: if `frames` lacks that SHA, call `load_frame(sha)`
  and keep showing the previous frame (or a spinner) until it arrives — never
  block. Then re-run the existing map/heatmap render against the historical
  `Vec<Station>` instead of the live one.
- **Live** button = leave History mode and fall back to the normal `./data`
  fetch (treat "live" as distinct from snapshot[0], which can lag by up to one
  cron interval).
- Nice-to-have: prefetch the immediate neighbours of `selected` so scrubbing is
  smooth; they're cached by SHA anyway.

> Scope note: the slider drives the **stations** map. Forecasts/alerts can be
> added later by fetching `data/prognoza-orase.json` etc. at the same SHA — same
> commit, same timeline.

## owner/repo & branch configuration

The app needs `{owner}/{repo}` (and implicitly the branch) to build the URLs.
This repo currently has **no git remote and no commits**, so it can't be derived
locally — it must be configured. Options:

1. **Compile-time `const`** — simplest, but every fork must edit source (and
   `src/` is lint-gated here).
2. **`option_env!("METEO_REPO_SLUG")` injected by CI, `const` fallback** —
   no source edit for the canonical build; forks "just work" because CI passes
   their own slug. **← recommended.**
3. **Runtime discovery via `data/repo.json`** — the workflow writes
   `{"owner","repo","default_branch"}` (it knows `${{ github.repository }}`), the
   app fetches it same-origin at startup. Zero recompile to fork, fully
   data-driven — but adds a startup round-trip and another moving part.

**Recommendation: option 2.** The build job in `snapshot.yml` already exports
`METEO_REPO_SLUG: ${{ github.repository }}` before `trunk build`, so the wasm
binary is stamped with the right `owner/repo` with no manual edit and no extra
fetch. Read it with `option_env!` and keep a `"OWNER/REPO"` fallback const for
`trunk serve` local dev. (If you later want fork-proof + branch-aware config
with no recompile, graduate to option 3 — it composes cleanly with the same-origin
philosophy.)

Branch: SHAs are absolute, so **reads don't care about the branch**. Only the
*default* branch (what `?path=...` lists without `&sha=`) and the workflow's push
target matter — keep them the same. If you rename `master`→`main`, the workflow
pushes to `${{ github.ref_name }}` automatically, so nothing to change there.

## Tradeoffs: pure git history vs. an `INDEX.json` manifest

**A. Pure git history (what's shipped).**
The commits-by-path API is the index; raw-by-SHA is the bytes. The workflow
commits one snapshot per change and nothing else.

- ➕ **Zero working-tree bloat** — only the latest snapshot is checked out; all
  history lives in GitHub-side packfiles (near-identical JSON deltifies well).
- ➕ **No file to maintain** — no append-only growth, no merge conflicts; the
  commit log *is* the manifest, and "commit only on change" keeps it noise-free.
- ➕ **Immutable, CDN-cached reads** keyed by SHA.
- ➖ Enumerating history needs the **api.github.com 60/hr** call (+ pagination).
- ➖ A history-rewrite (squash / force-push / `filter-repo`) would **break old
  SHAs**. Treat the data branch as append-only.
- ➖ One api round-trip before the first scrub (mitigated by session caching).

**B. Also commit an append-only `history/INDEX.json` manifest.**
Each run appends an entry the app can read same-origin in one fetch.

- ➕ **No GitHub api call** for the index — dodges the 60/hr limit and api
  outages; can carry extra metadata (per-endpoint change flags, min/max temp for
  a preview).
- ➖ **Defeats the no-bloat goal**: an *unbounded* manifest grows forever in the
  working tree, gets re-checked-out and re-deployed every run, and is
  merge-conflict-prone.
- ➖ **Redundant** with what git already records.
- ➖ A timestamp-only manifest still can't fetch bytes without a SHA, and a
  snapshot can't cleanly write its *own* SHA into the same commit (chicken/egg).

### Recommendation

**Ship pure git history (option A) as the canonical store — and do NOT keep an
unbounded append-only manifest.** It's the minimal-bloat design the whole idea is
built on, and it's sufficient: commits-by-path gives the `(sha, date)` index and
raw-by-SHA gives immutable bytes, both CORS-open and verified.

If/when the 60/hr limit or api-outage risk actually bites, add **one bounded
hedge** instead of an append-only file: have the workflow regenerate a small,
**rolling** `data/history.json` (e.g. last ~30 days ≈ 240 entries of
`{"sha","ts"}`) from `git log` each run — *bounded, not append-only*, so the tree
stays tiny and deterministic. Mechanism that sidesteps the self-SHA problem:
commit the data first (SHA `S1`), then `git log --format='%H %cI' -- data/starea-vremii.json | head -n 240`
(which now includes `S1`) into `data/history.json`, and commit *that* separately.
Because the index commit doesn't touch `starea-vremii.json`, the commits-by-path
API stays clean (real snapshots only), while the app serves recent history from
one same-origin fetch and falls back to the api for deep scrubbing. Start
pure-git; bolt on the rolling index only if measurements say you need it.
