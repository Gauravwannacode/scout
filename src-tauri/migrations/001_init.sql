-- Scout's local store. Everything the app owns lives here; nothing requires
-- the network to read. Timestamps are ISO-8601 text, which SQLite compares
-- and sorts correctly as strings.

CREATE TABLE IF NOT EXISTS task (
  id          TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  due_at      TEXT,
  status      TEXT NOT NULL DEFAULT 'open',
  item_id     TEXT,
  created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS task_due_idx ON task (status, due_at);

CREATE TABLE IF NOT EXISTS alarm (
  id       TEXT PRIMARY KEY,
  at       TEXT NOT NULL,          -- "HH:MM", 24-hour
  label    TEXT NOT NULL,
  days     TEXT NOT NULL DEFAULT '', -- comma-separated weekdays, 0=Sunday; empty = one-shot
  enabled  INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS focus_session (
  id          TEXT PRIMARY KEY,
  started_at  TEXT NOT NULL,
  ended_at    TEXT,
  task_id     TEXT REFERENCES task (id) ON DELETE SET NULL,
  mode        TEXT NOT NULL DEFAULT 'focus',
  completed   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS focus_started_idx ON focus_session (started_at);

CREATE TABLE IF NOT EXISTS item (
  id              TEXT PRIMARY KEY,
  kind            TEXT NOT NULL,
  title           TEXT NOT NULL,
  org             TEXT,
  url             TEXT NOT NULL,
  summary         TEXT,
  published_at    TEXT,
  deadline_at     TEXT,
  tags            TEXT NOT NULL DEFAULT '',
  source          TEXT NOT NULL,
  external_id     TEXT NOT NULL,
  raw             TEXT,
  -- How big a deal this is. The sort key; never lowered for popularity.
  significance    INTEGER NOT NULL DEFAULT 0,
  -- How widely covered already. Only ever affects the badge.
  reach           INTEGER NOT NULL DEFAULT 0,
  badge           TEXT NOT NULL DEFAULT 'radar',
  why_line        TEXT,
  -- Independent sources carrying this story. Produced by clustering, drives reach.
  corroborations  INTEGER NOT NULL DEFAULT 1,
  first_seen_at   TEXT NOT NULL,
  UNIQUE (source, external_id)
);
CREATE INDEX IF NOT EXISTS item_rank_idx ON item (significance DESC);
CREATE INDEX IF NOT EXISTS item_kind_idx ON item (kind);

CREATE TABLE IF NOT EXISTS brief (
  date          TEXT PRIMARY KEY,   -- YYYY-MM-DD, one per day
  body          TEXT NOT NULL,
  generated_at  TEXT NOT NULL,
  lead_item_id  TEXT REFERENCES item (id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS profile (
  id              INTEGER PRIMARY KEY CHECK (id = 1), -- single row
  bio             TEXT NOT NULL DEFAULT '',
  skills          TEXT NOT NULL DEFAULT '',
  year            INTEGER NOT NULL DEFAULT 2,
  goals           TEXT NOT NULL DEFAULT '',
  remote_only     INTEGER NOT NULL DEFAULT 1,
  no_degree_gate  INTEGER NOT NULL DEFAULT 1
);

-- Per-source counts and errors for each refresh, so a source that silently
-- stops returning results is visible rather than mysterious.
CREATE TABLE IF NOT EXISTS fetch_run (
  id               TEXT PRIMARY KEY,
  started_at       TEXT NOT NULL,
  finished_at      TEXT,
  counts           TEXT NOT NULL DEFAULT '{}',
  errors           TEXT NOT NULL DEFAULT '{}',
  skipped_offline  INTEGER NOT NULL DEFAULT 0
);
