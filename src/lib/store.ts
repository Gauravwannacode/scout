import { useCallback, useEffect, useState } from "react";
import { repo } from "./repo";
import type { Alarm, FocusSession, Item, Task } from "../types";
import { todayIso } from "./time";
import { refreshNews } from "./refresh";
import type { RefreshOutcome } from "./refresh";
import { drainPendingItems, onBackgroundRefresh, syncAlarms, syncDeadlines } from "./scheduler";

/** Sensible starting content so a fresh install is not an empty room. */
const SEED_ALARMS: Omit<Alarm, "id">[] = [
  { at: "06:30", label: "Wake", days: [1, 2, 3, 4, 5, 6], enabled: true },
  { at: "12:45", label: "Leave for class", days: [1, 3], enabled: true },
  { at: "22:00", label: "Wind down", days: [0, 1, 2, 3, 4, 5, 6], enabled: false },
];

export interface Store {
  ready: boolean;
  tasks: Task[];
  alarms: Alarm[];
  sessions: FocusSession[];
  items: Item[];
  /** Today's brief, or null when none has been written. */
  brief: string | null;

  addTask(title: string, dueAt?: string | null): Promise<void>;
  toggleTask(id: string): Promise<void>;
  removeTask(id: string): Promise<void>;

  addAlarm(a: Omit<Alarm, "id">): Promise<void>;
  toggleAlarm(id: string): Promise<void>;
  removeAlarm(id: string): Promise<void>;

  logSession(s: Omit<FocusSession, "id">): Promise<void>;

  /** Fetch, cluster and score. Null until a run has happened this session. */
  refresh(): Promise<void>;
  refreshing: boolean;
  lastRefresh: RefreshOutcome | null;
}

/**
 * Seeding must happen exactly once even if two mounts race (StrictMode does
 * this in development, and a remount can do it in production). Sharing a
 * single module-scoped promise serialises them: the second caller awaits the
 * first rather than re-running the check against a store that is still empty.
 */
let seedOnce: Promise<void> | null = null;

function ensureSeeded(): Promise<void> {
  seedOnce ??= (async () => {
    const existing = await repo.listAlarms();
    if (existing.length > 0) return;
    for (const seed of SEED_ALARMS) await repo.addAlarm(seed);
  })();
  return seedOnce;
}

export function useStore(): Store {
  const [ready, setReady] = useState(false);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [alarms, setAlarms] = useState<Alarm[]>([]);
  const [sessions, setSessions] = useState<FocusSession[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [brief, setBrief] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<RefreshOutcome | null>(null);

  const weekAgo = useCallback(() => {
    const d = new Date();
    d.setDate(d.getDate() - 7);
    return d.toISOString();
  }, []);

  useEffect(() => {
    let alive = true;
    (async () => {
      await ensureSeeded();
      const [a, t, s, i, b] = await Promise.all([
        repo.listAlarms(),
        repo.listTasks(),
        repo.listSessions(weekAgo()),
        repo.listItems(),
        repo.getBrief(todayIso()),
      ]);
      if (!alive) return;
      setAlarms(a);
      setTasks(t);
      setSessions(s);
      setItems(i);
      setBrief(b?.body ?? null);
      setReady(true);
    })();
    return () => {
      alive = false;
    };
  }, [weekAgo]);

  // The scheduler fires alarms and deadline reminders from Rust, so its copy
  // has to track the UI's. Syncing on change rather than on every mutation
  // site means a new mutation cannot forget to do it.
  useEffect(() => {
    if (ready) void syncAlarms(alarms);
  }, [ready, alarms]);

  useEffect(() => {
    if (ready) void syncDeadlines(tasks);
  }, [ready, tasks]);

  // A background run may have landed stories while the window was hidden.
  // Drain on mount to catch anything from a previous session, and on each
  // event to catch runs that happen while the app sits in the tray.
  useEffect(() => {
    const pull = async () => {
      if ((await drainPendingItems()) > 0) {
        setItems(await repo.listItems());
        setBrief((await repo.getBrief(todayIso()))?.body ?? null);
      }
    };
    void pull();
    return onBackgroundRefresh(() => {
      void pull();
    });
  }, []);

  const addTask = useCallback(async (title: string, dueAt: string | null = null) => {
    const trimmed = title.trim();
    if (!trimmed) return;
    await repo.addTask({ title: trimmed, dueAt, status: "open", itemId: null });
    setTasks(await repo.listTasks());
  }, []);

  const toggleTask = useCallback(
    async (id: string) => {
      const current = tasks.find((t) => t.id === id);
      if (!current) return;
      await repo.updateTask(id, { status: current.status === "done" ? "open" : "done" });
      setTasks(await repo.listTasks());
    },
    [tasks],
  );

  const removeTask = useCallback(async (id: string) => {
    await repo.removeTask(id);
    setTasks(await repo.listTasks());
  }, []);

  const addAlarm = useCallback(async (a: Omit<Alarm, "id">) => {
    await repo.addAlarm(a);
    setAlarms(await repo.listAlarms());
  }, []);

  const toggleAlarm = useCallback(
    async (id: string) => {
      const current = alarms.find((a) => a.id === id);
      if (!current) return;
      await repo.updateAlarm(id, { enabled: !current.enabled });
      setAlarms(await repo.listAlarms());
    },
    [alarms],
  );

  const removeAlarm = useCallback(async (id: string) => {
    await repo.removeAlarm(id);
    setAlarms(await repo.listAlarms());
  }, []);

  const logSession = useCallback(
    async (s: Omit<FocusSession, "id">) => {
      await repo.addSession(s);
      setSessions(await repo.listSessions(weekAgo()));
    },
    [weekAgo],
  );

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      const outcome = await refreshNews();
      setLastRefresh(outcome);
      // Offline or failed runs leave the cached items alone rather than
      // blanking the page.
      if (outcome.ok) {
        setItems(await repo.listItems());
        setBrief((await repo.getBrief(todayIso()))?.body ?? null);
      }
    } finally {
      setRefreshing(false);
    }
  }, []);

  return {
    ready,
    refresh,
    refreshing,
    lastRefresh,
    tasks,
    alarms,
    sessions,
    items,
    brief,
    addTask,
    toggleTask,
    removeTask,
    addAlarm,
    toggleAlarm,
    removeAlarm,
    logSession,
  };
}

/** Tasks due today or with no date, open ones first. */
export function todaysTasks(tasks: Task[]): Task[] {
  const today = todayIso();
  return tasks
    .filter((t) => !t.dueAt || t.dueAt.slice(0, 10) <= today || t.status === "open")
    .sort((a, b) => {
      if (a.status !== b.status) return a.status === "open" ? -1 : 1;
      return (a.dueAt ?? "9999").localeCompare(b.dueAt ?? "9999");
    });
}
