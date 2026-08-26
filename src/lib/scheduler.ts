import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isDesktop } from "./sqliteRepo";
import { repo } from "./repo";
import type { Alarm, Item, Task } from "../types";

/**
 * Mirrors alarms and deadlines into the Rust scheduler.
 *
 * The scheduler owns firing, not the UI. Anything scheduled in JavaScript
 * stops the moment the window closes, which is precisely when a 06:30 alarm
 * needs to work. Calling this on every change keeps Rust's copy current.
 */
export async function syncAlarms(alarms: Alarm[]): Promise<void> {
  if (!isDesktop()) return;
  try {
    await invoke("sync_alarms", {
      alarms: alarms.map((a) => ({
        id: a.id,
        at: a.at,
        label: a.label,
        days: a.days,
        enabled: a.enabled,
      })),
    });
  } catch (e) {
    console.error("failed to sync alarms to the scheduler", e);
  }
}

export async function syncDeadlines(tasks: Task[]): Promise<void> {
  if (!isDesktop()) return;
  const deadlines = tasks
    .filter((t) => t.status === "open" && t.dueAt)
    .map((t) => ({ id: t.id, title: t.title, dueAt: t.dueAt as string }));
  try {
    await invoke("sync_deadlines", { deadlines });
  } catch (e) {
    console.error("failed to sync deadlines to the scheduler", e);
  }
}

/**
 * Moves stories fetched while the window was closed into the database.
 *
 * A background run cannot write to SQLite itself — the frontend plugin owns
 * that connection — so it parks results on disk and this drains them. Safe to
 * call at any time; there is usually nothing waiting.
 *
 * Returns how many stories were taken.
 */
export async function drainPendingItems(): Promise<number> {
  if (!isDesktop()) return 0;
  try {
    const items = await invoke<Item[]>("take_pending_items");
    if (items.length === 0) return 0;
    await repo.putItems(items);
    return items.length;
  } catch (e) {
    console.error("failed to drain parked stories", e);
    return 0;
  }
}

/** Fires when the background refresh completes, so the UI can update itself. */
export function onBackgroundRefresh(fn: () => void): () => void {
  if (!isDesktop()) return () => {};
  const stop = listen("scout://refreshed", () => fn());
  return () => {
    void stop.then((unlisten) => unlisten());
  };
}

/** Fires when the scheduler sounds an alarm, so the UI can reflect it. */
export function onAlarmFired(fn: (alarmId: string) => void): () => void {
  if (!isDesktop()) return () => {};
  const stop = listen<string>("scout://alarm-fired", (e) => fn(e.payload));
  return () => {
    void stop.then((unlisten) => unlisten());
  };
}
