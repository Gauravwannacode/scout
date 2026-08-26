import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Alarm } from "../types";

export const pad2 = (n: number): string => (n < 10 ? `0${n}` : String(n));

export function fmtClock(d: Date): { time: string; ampm: string } {
  const h = d.getHours();
  const h12 = h % 12 === 0 ? 12 : h % 12;
  return { time: `${pad2(h12)}:${pad2(d.getMinutes())}`, ampm: h < 12 ? "AM" : "PM" };
}

export function fmtDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}:${pad2(m)}:${pad2(s)}` : `${pad2(m)}:${pad2(s)}`;
}

export const DAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Ticks on a wall clock. `everyMs` of 1000 keeps seconds honest. */
export function useNow(everyMs = 1000): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), everyMs);
    return () => window.clearInterval(id);
  }, [everyMs]);
  return now;
}

export type TimerMode = "focus" | "stopwatch";

export interface TimerState {
  mode: TimerMode;
  running: boolean;
  /** Milliseconds shown on the dial. */
  displayMs: number;
  /** 0..1, only meaningful in focus mode. */
  progress: number;
  finished: boolean;
}

export interface TimerApi extends TimerState {
  start(): void;
  pause(): void;
  reset(): void;
  setMode(m: TimerMode): void;
  setFocusMinutes(min: number): void;
  focusMinutes: number;
}

/**
 * Timestamp-derived timer.
 *
 * The elapsed time is always computed from wall-clock anchors, never
 * accumulated per tick. Background throttling, a suspended machine, or a
 * dropped frame therefore cannot cause drift — on resume the dial is simply
 * correct. The interval exists only to trigger a re-render.
 */
export function useTimer(initialFocusMinutes = 25, onFinish?: () => void): TimerApi {
  const [mode, setModeState] = useState<TimerMode>("focus");
  const [focusMinutes, setFocusMinutes] = useState(initialFocusMinutes);
  const [running, setRunning] = useState(false);
  const [finished, setFinished] = useState(false);

  /** When the current run began. Null while paused. */
  const [startedAt, setStartedAt] = useState<number | null>(null);
  /** Elapsed time banked by previous runs, in ms. */
  const [banked, setBanked] = useState(0);
  const [, forceRender] = useState(0);

  const finishRef = useRef(onFinish);
  finishRef.current = onFinish;

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => forceRender((n) => n + 1), 250);
    return () => window.clearInterval(id);
  }, [running]);

  const elapsed = banked + (startedAt !== null ? Date.now() - startedAt : 0);
  const durationMs = focusMinutes * 60_000;
  const displayMs = mode === "focus" ? Math.max(0, durationMs - elapsed) : elapsed;

  // A focus run that has burned its duration is over, however long the app was
  // asleep for. Checked during render-driven ticks rather than on a timeout,
  // so a suspend that spans the end time still resolves correctly on wake.
  useEffect(() => {
    if (mode !== "focus" || !running || elapsed < durationMs) return;
    setRunning(false);
    setStartedAt(null);
    setBanked(durationMs);
    setFinished(true);
    finishRef.current?.();
  }, [mode, running, elapsed, durationMs]);

  const start = useCallback(() => {
    setFinished(false);
    setStartedAt(Date.now());
    setRunning(true);
  }, []);

  const pause = useCallback(() => {
    setBanked((b) => b + (startedAt !== null ? Date.now() - startedAt : 0));
    setStartedAt(null);
    setRunning(false);
  }, [startedAt]);

  const reset = useCallback(() => {
    setRunning(false);
    setStartedAt(null);
    setBanked(0);
    setFinished(false);
  }, []);

  const setMode = useCallback((m: TimerMode) => {
    setModeState(m);
    setRunning(false);
    setStartedAt(null);
    setBanked(0);
    setFinished(false);
  }, []);

  return {
    mode,
    running,
    displayMs,
    progress: mode === "focus" ? Math.min(1, elapsed / durationMs) : 0,
    finished,
    start,
    pause,
    reset,
    setMode,
    focusMinutes,
    setFocusMinutes,
  };
}

/** The next time this alarm should sound, or null if it never will. */
export function nextOccurrence(alarm: Alarm, from: Date = new Date()): Date | null {
  if (!alarm.enabled) return null;
  const [h, m] = alarm.at.split(":").map(Number);
  if (Number.isNaN(h) || Number.isNaN(m)) return null;

  // A one-shot alarm fires at the next occurrence of that time, today or tomorrow.
  if (alarm.days.length === 0) {
    const t = new Date(from);
    t.setHours(h, m, 0, 0);
    if (t <= from) t.setDate(t.getDate() + 1);
    return t;
  }

  for (let offset = 0; offset < 8; offset++) {
    const t = new Date(from);
    t.setDate(t.getDate() + offset);
    t.setHours(h, m, 0, 0);
    if (t > from && alarm.days.includes(t.getDay())) return t;
  }
  return null;
}

/** Soonest upcoming alarm across the set. */
export function useNextAlarm(alarms: Alarm[], now: Date): { alarm: Alarm; at: Date } | null {
  return useMemo(() => {
    let best: { alarm: Alarm; at: Date } | null = null;
    for (const a of alarms) {
      const at = nextOccurrence(a, now);
      if (at && (!best || at < best.at)) best = { alarm: a, at };
    }
    return best;
  }, [alarms, now]);
}

/** "in 46m", "in 2h 10m", "now". */
export function relativeTo(target: Date, now: Date): string {
  const mins = Math.round((target.getTime() - now.getTime()) / 60_000);
  if (mins <= 0) return "now";
  if (mins < 60) return `in ${mins}m`;
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return m === 0 ? `in ${h}h` : `in ${h}h ${m}m`;
}

export function describeDays(days: number[]): string {
  if (days.length === 0) return "Once";
  if (days.length === 7) return "Every day";
  const sorted = [...days].sort();
  if (sorted.join() === "1,2,3,4,5") return "Weekdays";
  if (sorted.join() === "0,6") return "Weekends";
  return sorted.map((d) => DAY_LABELS[d]).join(", ");
}

export const todayIso = (d: Date = new Date()): string =>
  `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
