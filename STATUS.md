# Scout — build status

Local-first desktop app: daily AI/software news, openings, clock/alarms, to-do.
Plan: `C:\Users\gaura\.claude\plans\lazy-twirling-cherny.md`

## Done

**1. Shell + offline pages** — Tauri v2 + React + Vite + Tailwind. Home (two
squares + wide news panel), Clock (live clock, focus timer, stopwatch, alarms),
To-do (today, deadlines, weekly stats), News (three sections). SQLite via
`tauri-plugin-sql`, seven tables, stored in `%APPDATA%\dev.gaurav.scout\`.
Fonts bundled with `@fontsource` so the app renders identically offline.

**2. Source adapters** — 27 live sources, all verified against real endpoints.
`cargo run --bin probe` sweeps them and exits non-zero if any returns nothing.

**3. Clustering + scoring** — TF-IDF/trigram clustering (no embeddings, works
offline), arithmetic reach, Groq significance scoring with a heuristic fallback.
Badges: legendary / worth-knowing / radar.

**4. Openings** — YC, Launch HN, Show HN, HN hiring, GitHub good-first-issues,
Devpost, Devfolio, Unstop, RemoteOK, Arbeitnow, Himalayas. Rules prefilter drops
senior roles, degree gates, expired deadlines and non-software work — and
reports how many it dropped, per reason.

**5. Always on** — a Rust background task that outlives the window: alarms and
deadline reminders fire as real OS notifications with the app closed to the
tray, news refreshes every 30 min, and a legendary story raises a notification
(nothing else does). Tray icon with Open / Refresh / Quit; closing the window
hides rather than exits. Launch-on-startup available via the autostart plugin.

**6. Reading** — News lead is a pager: prev/next (and arrow keys) walk the
ranked stories, "Worth knowing" cards double as a picker, and openings have a
Details expander showing summary, deadline in days, coverage and link host.
Stories open in the real browser via the opener plugin.

## Two design decisions worth remembering

**Parked results.** The background refresh cannot write to SQLite — the
frontend plugin owns that connection. A run with the window closed writes its
stories to `pending.json`, and the UI drains it on next mount. One writer, and
nothing fetched is ever lost.

**Single instance.** Two Scouts sharing one SQLite file, hard-stopped, leave an
unreplayable WAL that reads as "database disk image is malformed" — this
happened during development once close-to-tray landed. `tauri-plugin-single-
instance` now hands focus to the running copy instead of opening a second.

**7. Deadlines and fit scoring** — Devpost (`"Aug 21 - 25, 2026"` ranges,
including the implied-month form), Devfolio (`ends_at`) and Unstop
(`regnRequirements.end_regn_dt`) now populate `deadline_at`: 69 of 165 openings
carry a real date, and the expiry filter drops what has closed. Openings are
scored by a separate fit prompt — remote, no degree, feasible deadline, small
enough field to place in — instead of the news prompt.

## The scoring budget

Groq's free tier caps at **8000 tokens per minute**, not per day. A 500-story
sweep therefore cannot be model-scored: the first two batches exhausted every
key and 95% of items silently fell back to the heuristic while the run still
reported success.

Two changes fix it. The heuristic ranks everything cheaply, then only the top
`NEWS_CANDIDATES` (60) and `OPENING_CANDIDATES` (40) are sent to the model —
scoring item #400 was always waste, since it is never displayed. Batches are
paced 21s apart, and a 429 backs off once before the key is retired.

`ScoreOutcome.model_scored` now reports how many items the model actually
judged. Partial fallback used to be invisible because `provisional_scores` only
went true when *everything* failed — news succeeding is not evidence that
openings did.

**7. Settings, tray tooltip, and the daily brief** — a settings panel behind
the footer (Groq keys one per line, Gemini key, start-with-Windows toggle); the
tray tooltip now names the next alarm and how far off it is; and the daily
brief is finally written — the "anchor script" that reads the whole day rather
than one item at a time.

## Why the brief nearly shipped broken

`gpt-oss` reasons before answering, and that reasoning counts against
`max_tokens`. At 400 the reasoning consumed the entire budget and `content`
came back empty with `finish_reason: "length"` — an HTTP 200 carrying nothing.
It now runs at 1200 tokens with `reasoning_effort: "low"`.

It was invisible at first because every failure path was a silent
`else { continue }`. `brief_error` now records exactly which key and model
failed and why, the same fix applied earlier to scoring. Groq writes the brief
by default so it works with the keys already configured; a Gemini key, if set,
is preferred for its longer context.

## Known gaps

- **A full sweep takes ~2 minutes** because of the rate-limit pacing. Fine in
  the background; slow if triggered by hand from the Refresh button.
- `/ask` advisor not built — the last item from the original plan.
- **The settings panel is verified structurally, not clicked through.** The
  browser path correctly refuses (desktop-only) and the field names now match
  the Rust struct exactly, but the real fields have not been exercised in the
  Tauri window.
- **Alarm audio is proven non-silent by test, not by ear.** `cargo run --bin
  ring` plays it; the tone's samples are asserted to peak above 0.2.

## Commands

    cargo run --bin probe              # check every source is alive
    cargo run --bin probe -- pipeline  # fetch, cluster, score, rank
    cargo test --lib                   # 36 unit tests
    npx tauri dev                      # run in development
    npx tauri build                    # produce the installer

## Configuration

`%APPDATA%\dev.gaurav.scout\settings.json` holds `groq_api_keys` (an array —
keys are rotated when one hits its daily limit) and `gemini_api_key`.
Environment variables `GROQ_API_KEY` (comma-separated) and `GEMINI_API_KEY`
override the file.

Groq's free catalogue churns: every Llama model 404s as of 2026-08-23. The
model list in `src-tauri/src/pipeline/score.rs` is an ordered fallback for
exactly this reason.
