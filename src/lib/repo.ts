import type { Alarm, Brief, FocusSession, Item, Profile, Task } from "../types";
import { SqliteRepo, isDesktop } from "./sqliteRepo";

/**
 * Everything the UI is allowed to know about storage.
 *
 * Step 1 ships `LocalRepo` (browser storage) so the three offline pages work
 * before the Tauri shell exists. `SqliteRepo` lands in step 2 against the same
 * interface, so no page component changes when the backing store does.
 */
export interface Repo {
  listTasks(): Promise<Task[]>;
  addTask(t: Omit<Task, "id" | "createdAt">): Promise<Task>;
  updateTask(id: string, patch: Partial<Task>): Promise<void>;
  removeTask(id: string): Promise<void>;

  listAlarms(): Promise<Alarm[]>;
  addAlarm(a: Omit<Alarm, "id">): Promise<Alarm>;
  updateAlarm(id: string, patch: Partial<Alarm>): Promise<void>;
  removeAlarm(id: string): Promise<void>;

  listSessions(sinceIso: string): Promise<FocusSession[]>;
  addSession(s: Omit<FocusSession, "id">): Promise<FocusSession>;

  listItems(): Promise<Item[]>;
  putItems(items: Item[]): Promise<void>;

  getBrief(date: string): Promise<Brief | null>;
  putBrief(b: Brief): Promise<void>;

  getProfile(): Promise<Profile>;
  putProfile(p: Profile): Promise<void>;
}

const KEY = "scout.v1";

interface Snapshot {
  tasks: Task[];
  alarms: Alarm[];
  sessions: FocusSession[];
  items: Item[];
  briefs: Brief[];
  profile: Profile;
}

const DEFAULT_PROFILE: Profile = {
  bio: "Second-year CS student. Building things, looking for remote work.",
  skills: ["typescript", "react", "python", "c"],
  year: 2,
  goals: "Remote internship or paid gig. Hackathons. First open-source patch.",
  remoteOnly: true,
  noDegreeGate: true,
};

function emptySnapshot(): Snapshot {
  return {
    tasks: [],
    alarms: [],
    sessions: [],
    items: [],
    briefs: [],
    profile: DEFAULT_PROFILE,
  };
}

function uid(): string {
  return Math.random().toString(36).slice(2, 10) + Date.now().toString(36);
}

/**
 * localStorage-backed store. Synchronous underneath, async on the surface so
 * that swapping in SQLite is invisible to callers.
 */
export class LocalRepo implements Repo {
  private read(): Snapshot {
    try {
      const raw = localStorage.getItem(KEY);
      if (!raw) return emptySnapshot();
      return { ...emptySnapshot(), ...(JSON.parse(raw) as Partial<Snapshot>) };
    } catch {
      // A corrupt store should not brick the app; start clean instead.
      return emptySnapshot();
    }
  }

  private write(s: Snapshot): void {
    localStorage.setItem(KEY, JSON.stringify(s));
  }

  async listTasks(): Promise<Task[]> {
    return this.read().tasks;
  }

  async addTask(t: Omit<Task, "id" | "createdAt">): Promise<Task> {
    const s = this.read();
    const task: Task = { ...t, id: uid(), createdAt: new Date().toISOString() };
    s.tasks.push(task);
    this.write(s);
    return task;
  }

  async updateTask(id: string, patch: Partial<Task>): Promise<void> {
    const s = this.read();
    s.tasks = s.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t));
    this.write(s);
  }

  async removeTask(id: string): Promise<void> {
    const s = this.read();
    s.tasks = s.tasks.filter((t) => t.id !== id);
    this.write(s);
  }

  async listAlarms(): Promise<Alarm[]> {
    return this.read().alarms;
  }

  async addAlarm(a: Omit<Alarm, "id">): Promise<Alarm> {
    const s = this.read();
    const alarm: Alarm = { ...a, id: uid() };
    s.alarms.push(alarm);
    s.alarms.sort((x, y) => x.at.localeCompare(y.at));
    this.write(s);
    return alarm;
  }

  async updateAlarm(id: string, patch: Partial<Alarm>): Promise<void> {
    const s = this.read();
    s.alarms = s.alarms.map((a) => (a.id === id ? { ...a, ...patch } : a));
    this.write(s);
  }

  async removeAlarm(id: string): Promise<void> {
    const s = this.read();
    s.alarms = s.alarms.filter((a) => a.id !== id);
    this.write(s);
  }

  async listSessions(sinceIso: string): Promise<FocusSession[]> {
    return this.read().sessions.filter((x) => x.startedAt >= sinceIso);
  }

  async addSession(x: Omit<FocusSession, "id">): Promise<FocusSession> {
    const s = this.read();
    const session: FocusSession = { ...x, id: uid() };
    s.sessions.push(session);
    this.write(s);
    return session;
  }

  async listItems(): Promise<Item[]> {
    return this.read().items;
  }

  async putItems(items: Item[]): Promise<void> {
    const s = this.read();
    const byKey = new Map(s.items.map((i) => [`${i.source}:${i.externalId}`, i]));
    for (const i of items) byKey.set(`${i.source}:${i.externalId}`, i);
    s.items = [...byKey.values()];
    this.write(s);
  }

  async getBrief(date: string): Promise<Brief | null> {
    return this.read().briefs.find((b) => b.date === date) ?? null;
  }

  async putBrief(b: Brief): Promise<void> {
    const s = this.read();
    s.briefs = [b, ...s.briefs.filter((x) => x.date !== b.date)].slice(0, 30);
    this.write(s);
  }

  async getProfile(): Promise<Profile> {
    return this.read().profile;
  }

  async putProfile(p: Profile): Promise<void> {
    const s = this.read();
    s.profile = p;
    this.write(s);
  }
}

/**
 * SQLite in the desktop shell, browser storage when running `npm run dev` in a
 * plain tab. Resolved once at module load so the rest of the app never checks.
 */
export const repo: Repo = isDesktop() ? new SqliteRepo() : new LocalRepo();
