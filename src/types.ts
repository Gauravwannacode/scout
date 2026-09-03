export type ItemKind =
  | "news"
  | "paper"
  | "release"
  | "job"
  | "internship"
  | "hackathon"
  | "oss"
  | "grant"
  | "company";

export type Badge = "legendary" | "worth-knowing" | "radar";

export interface Item {
  id: string;
  kind: ItemKind;
  title: string;
  org: string | null;
  url: string;
  summary: string | null;
  publishedAt: string | null;
  deadlineAt: string | null;
  /** Where it physically happens. Null when the source did not say. */
  location: string | null;
  /** Null is "unknown", which is not the same as "not online". */
  isOnline: boolean | null;
  source: string;
  externalId: string;
  /** 0-100. How big a deal this is. The sort key — never lowered for popularity. */
  significance: number;
  /** 0-100. How widely covered already. Only ever affects the badge. */
  reach: number;
  badge: Badge;
  /** Second-person line: why this matters to him specifically. */
  whyLine: string | null;
  /** How many independent sources carried this story. Drives `reach`. */
  corroborations: number;
  firstSeenAt: string;
}

export type TaskStatus = "open" | "done";

export interface Task {
  id: string;
  title: string;
  dueAt: string | null;
  status: TaskStatus;
  /** Set when the task was created by accepting an opening. */
  itemId: string | null;
  createdAt: string;
}

export interface Alarm {
  id: string;
  /** "HH:MM", 24-hour. */
  at: string;
  label: string;
  /** Days of week this repeats on. 0 = Sunday. Empty means one-shot. */
  days: number[];
  enabled: boolean;
}

export type FocusMode = "focus" | "break";

export interface FocusSession {
  id: string;
  startedAt: string;
  endedAt: string | null;
  taskId: string | null;
  mode: FocusMode;
  completed: boolean;
}

export interface Brief {
  /** ISO date, YYYY-MM-DD. One brief per day. */
  date: string;
  body: string;
  generatedAt: string;
  leadItemId: string | null;
}

export interface Profile {
  bio: string;
  skills: string[];
  year: number;
  goals: string;
  remoteOnly: boolean;
  noDegreeGate: boolean;
}

export interface FetchRun {
  id: string;
  startedAt: string;
  finishedAt: string | null;
  /** Per-source item counts, and the error message for any source that failed. */
  counts: Record<string, number>;
  errors: Record<string, string>;
  /** True when the run was skipped because the machine was offline. */
  skippedOffline: boolean;
}
