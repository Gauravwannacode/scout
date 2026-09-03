import Database from "@tauri-apps/plugin-sql";
import type { Repo } from "./repo";
import type { Alarm, Brief, FocusSession, Item, Profile, Task } from "../types";

/** True when running inside the Tauri shell rather than a plain browser tab. */
export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function uid(): string {
  return Math.random().toString(36).slice(2, 10) + Date.now().toString(36);
}

const csv = {
  parse: (s: string): number[] =>
    s ? s.split(",").filter(Boolean).map(Number).filter((n) => !Number.isNaN(n)) : [],
  stringify: (n: number[]): string => n.join(","),
};

interface AlarmRow {
  id: string;
  at: string;
  label: string;
  days: string;
  enabled: number;
}

interface TaskRow {
  id: string;
  title: string;
  due_at: string | null;
  status: string;
  item_id: string | null;
  created_at: string;
}

interface SessionRow {
  id: string;
  started_at: string;
  ended_at: string | null;
  task_id: string | null;
  mode: string;
  completed: number;
}

interface ItemRow {
  id: string;
  kind: string;
  title: string;
  org: string | null;
  url: string;
  summary: string | null;
  published_at: string | null;
  deadline_at: string | null;
  location: string | null;
  is_online: number | null;
  source: string;
  external_id: string;
  significance: number;
  reach: number;
  badge: string;
  why_line: string | null;
  corroborations: number;
  first_seen_at: string;
}

/**
 * SQLite-backed store, used whenever the app runs in the desktop shell.
 * Same surface as LocalRepo, so no page component knows the difference.
 */
export class SqliteRepo implements Repo {
  private dbPromise: Promise<Database> | null = null;

  private db(): Promise<Database> {
    this.dbPromise ??= Database.load("sqlite:scout.db");
    return this.dbPromise;
  }

  async listTasks(): Promise<Task[]> {
    const db = await this.db();
    const rows = await db.select<TaskRow[]>(
      "SELECT * FROM task ORDER BY status ASC, COALESCE(due_at, '9999') ASC, created_at ASC",
    );
    return rows.map((r) => ({
      id: r.id,
      title: r.title,
      dueAt: r.due_at,
      status: r.status === "done" ? "done" : "open",
      itemId: r.item_id,
      createdAt: r.created_at,
    }));
  }

  async addTask(t: Omit<Task, "id" | "createdAt">): Promise<Task> {
    const db = await this.db();
    const task: Task = { ...t, id: uid(), createdAt: new Date().toISOString() };
    await db.execute(
      "INSERT INTO task (id, title, due_at, status, item_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
      [task.id, task.title, task.dueAt, task.status, task.itemId, task.createdAt],
    );
    return task;
  }

  async updateTask(id: string, patch: Partial<Task>): Promise<void> {
    const db = await this.db();
    if (patch.status !== undefined)
      await db.execute("UPDATE task SET status = $1 WHERE id = $2", [patch.status, id]);
    if (patch.title !== undefined)
      await db.execute("UPDATE task SET title = $1 WHERE id = $2", [patch.title, id]);
    if (patch.dueAt !== undefined)
      await db.execute("UPDATE task SET due_at = $1 WHERE id = $2", [patch.dueAt, id]);
  }

  async removeTask(id: string): Promise<void> {
    const db = await this.db();
    await db.execute("DELETE FROM task WHERE id = $1", [id]);
  }

  async listAlarms(): Promise<Alarm[]> {
    const db = await this.db();
    const rows = await db.select<AlarmRow[]>("SELECT * FROM alarm ORDER BY at ASC");
    return rows.map((r) => ({
      id: r.id,
      at: r.at,
      label: r.label,
      days: csv.parse(r.days),
      enabled: r.enabled === 1,
    }));
  }

  async addAlarm(a: Omit<Alarm, "id">): Promise<Alarm> {
    const db = await this.db();
    const alarm: Alarm = { ...a, id: uid() };
    await db.execute(
      "INSERT INTO alarm (id, at, label, days, enabled) VALUES ($1, $2, $3, $4, $5)",
      [alarm.id, alarm.at, alarm.label, csv.stringify(alarm.days), alarm.enabled ? 1 : 0],
    );
    return alarm;
  }

  async updateAlarm(id: string, patch: Partial<Alarm>): Promise<void> {
    const db = await this.db();
    if (patch.enabled !== undefined)
      await db.execute("UPDATE alarm SET enabled = $1 WHERE id = $2", [patch.enabled ? 1 : 0, id]);
    if (patch.at !== undefined)
      await db.execute("UPDATE alarm SET at = $1 WHERE id = $2", [patch.at, id]);
    if (patch.label !== undefined)
      await db.execute("UPDATE alarm SET label = $1 WHERE id = $2", [patch.label, id]);
    if (patch.days !== undefined)
      await db.execute("UPDATE alarm SET days = $1 WHERE id = $2", [csv.stringify(patch.days), id]);
  }

  async removeAlarm(id: string): Promise<void> {
    const db = await this.db();
    await db.execute("DELETE FROM alarm WHERE id = $1", [id]);
  }

  async listSessions(sinceIso: string): Promise<FocusSession[]> {
    const db = await this.db();
    const rows = await db.select<SessionRow[]>(
      "SELECT * FROM focus_session WHERE started_at >= $1 ORDER BY started_at DESC",
      [sinceIso],
    );
    return rows.map((r) => ({
      id: r.id,
      startedAt: r.started_at,
      endedAt: r.ended_at,
      taskId: r.task_id,
      mode: r.mode === "break" ? "break" : "focus",
      completed: r.completed === 1,
    }));
  }

  async addSession(s: Omit<FocusSession, "id">): Promise<FocusSession> {
    const db = await this.db();
    const session: FocusSession = { ...s, id: uid() };
    await db.execute(
      "INSERT INTO focus_session (id, started_at, ended_at, task_id, mode, completed) VALUES ($1, $2, $3, $4, $5, $6)",
      [
        session.id,
        session.startedAt,
        session.endedAt,
        session.taskId,
        session.mode,
        session.completed ? 1 : 0,
      ],
    );
    return session;
  }

  async listItems(): Promise<Item[]> {
    const db = await this.db();
    const rows = await db.select<ItemRow[]>(
      "SELECT * FROM item ORDER BY significance DESC LIMIT 200",
    );
    return rows.map((r) => ({
      id: r.id,
      kind: r.kind as Item["kind"],
      title: r.title,
      org: r.org,
      url: r.url,
      summary: r.summary,
      publishedAt: r.published_at,
      deadlineAt: r.deadline_at,
      location: r.location ?? null,
      // SQLite has no boolean; 1/0/NULL maps to true/false/unknown.
      isOnline: r.is_online === null || r.is_online === undefined ? null : r.is_online === 1,
      source: r.source,
      externalId: r.external_id,
      significance: r.significance,
      reach: r.reach,
      badge: r.badge as Item["badge"],
      whyLine: r.why_line,
      corroborations: r.corroborations,
      firstSeenAt: r.first_seen_at,
    }));
  }

  async putItems(items: Item[]): Promise<void> {
    const db = await this.db();
    for (const i of items) {
      // Re-running a fetch must refresh scores rather than duplicate the row.
      await db.execute(
        `INSERT INTO item (id, kind, title, org, url, summary, published_at, deadline_at,
           location, is_online, source, external_id, significance, reach, badge, why_line,
           corroborations, first_seen_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
         ON CONFLICT (source, external_id) DO UPDATE SET
           significance = excluded.significance,
           reach = excluded.reach,
           badge = excluded.badge,
           why_line = excluded.why_line,
           corroborations = excluded.corroborations,
           deadline_at = excluded.deadline_at,
           location = excluded.location,
           is_online = excluded.is_online`,
        [
          i.id, i.kind, i.title, i.org, i.url, i.summary, i.publishedAt, i.deadlineAt,
          i.location, i.isOnline === null ? null : i.isOnline ? 1 : 0,
          i.source, i.externalId, i.significance, i.reach, i.badge, i.whyLine,
          i.corroborations, i.firstSeenAt,
        ],
      );
    }
  }

  async getBrief(date: string): Promise<Brief | null> {
    const db = await this.db();
    const rows = await db.select<
      { date: string; body: string; generated_at: string; lead_item_id: string | null }[]
    >("SELECT * FROM brief WHERE date = $1", [date]);
    const r = rows[0];
    return r
      ? { date: r.date, body: r.body, generatedAt: r.generated_at, leadItemId: r.lead_item_id }
      : null;
  }

  async putBrief(b: Brief): Promise<void> {
    const db = await this.db();
    await db.execute(
      `INSERT INTO brief (date, body, generated_at, lead_item_id) VALUES ($1,$2,$3,$4)
       ON CONFLICT (date) DO UPDATE SET body = excluded.body,
         generated_at = excluded.generated_at, lead_item_id = excluded.lead_item_id`,
      [b.date, b.body, b.generatedAt, b.leadItemId],
    );
  }

  async getProfile(): Promise<Profile> {
    const db = await this.db();
    const rows = await db.select<
      {
        bio: string;
        skills: string;
        year: number;
        goals: string;
        remote_only: number;
        no_degree_gate: number;
      }[]
    >("SELECT * FROM profile WHERE id = 1");
    const r = rows[0];
    if (!r) {
      return {
        bio: "",
        skills: [],
        year: 2,
        goals: "",
        remoteOnly: true,
        noDegreeGate: true,
      };
    }
    return {
      bio: r.bio,
      skills: r.skills ? r.skills.split(",") : [],
      year: r.year,
      goals: r.goals,
      remoteOnly: r.remote_only === 1,
      noDegreeGate: r.no_degree_gate === 1,
    };
  }

  async putProfile(p: Profile): Promise<void> {
    const db = await this.db();
    await db.execute(
      `INSERT INTO profile (id, bio, skills, year, goals, remote_only, no_degree_gate)
       VALUES (1,$1,$2,$3,$4,$5,$6)
       ON CONFLICT (id) DO UPDATE SET bio = excluded.bio, skills = excluded.skills,
         year = excluded.year, goals = excluded.goals, remote_only = excluded.remote_only,
         no_degree_gate = excluded.no_degree_gate`,
      [p.bio, p.skills.join(","), p.year, p.goals, p.remoteOnly ? 1 : 0, p.noDegreeGate ? 1 : 0],
    );
  }
}
