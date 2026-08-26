import { invoke } from "@tauri-apps/api/core";
import { isDesktop } from "./sqliteRepo";
import { repo } from "./repo";
import type { Item } from "../types";

/** Mirrors the Rust `PipelineResult`. */
interface PipelineResult {
  items: Item[];
  counts: Record<string, number>;
  errors: Record<string, string>;
  offline: boolean;
  /** Scores came from the heuristic, not the model — the UI must say so. */
  provisionalScores: boolean;
  scoreError: string | null;
  rawCount: number;
  storyCount: number;
}

export interface RefreshOutcome {
  ok: boolean;
  offline: boolean;
  provisional: boolean;
  storyCount: number;
  rawCount: number;
  /** Sources that returned nothing or failed — a dead adapter should be visible. */
  deadSources: string[];
  message: string;
}

const LAST_RUN_KEY = "scout.lastRun";

export function lastRunAt(): Date | null {
  const raw = localStorage.getItem(LAST_RUN_KEY);
  return raw ? new Date(raw) : null;
}

/**
 * Runs the Rust pipeline and stores the result.
 *
 * Offline is a normal outcome, not an error: the cached items stay put and the
 * caller shows a quiet "last updated" line rather than a failure.
 */
export async function refreshNews(): Promise<RefreshOutcome> {
  if (!isDesktop()) {
    return {
      ok: false,
      offline: false,
      provisional: false,
      storyCount: 0,
      rawCount: 0,
      deadSources: [],
      message: "News fetching needs the desktop app — the browser cannot read these sources.",
    };
  }

  let result: PipelineResult;
  try {
    result = await invoke<PipelineResult>("refresh");
  } catch (e) {
    return {
      ok: false,
      offline: false,
      provisional: false,
      storyCount: 0,
      rawCount: 0,
      deadSources: [],
      message: `Refresh failed: ${String(e)}`,
    };
  }

  if (result.offline) {
    return {
      ok: false,
      offline: true,
      provisional: false,
      storyCount: 0,
      rawCount: 0,
      deadSources: [],
      message: "Offline — showing what was saved. Everything else still works.",
    };
  }

  await repo.putItems(result.items);
  localStorage.setItem(LAST_RUN_KEY, new Date().toISOString());

  const deadSources = Object.entries(result.counts)
    .filter(([name, count]) => count === 0 || name in result.errors)
    .map(([name]) => name);

  const parts = [`${result.storyCount} stories from ${result.rawCount} items`];
  if (result.provisionalScores) parts.push("scores are provisional");
  if (deadSources.length) parts.push(`${deadSources.length} source(s) returned nothing`);

  return {
    ok: true,
    offline: false,
    provisional: result.provisionalScores,
    storyCount: result.storyCount,
    rawCount: result.rawCount,
    deadSources,
    message: parts.join(" · "),
  };
}
