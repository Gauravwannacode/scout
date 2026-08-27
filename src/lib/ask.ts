import { invoke } from "@tauri-apps/api/core";
import { isDesktop } from "./sqliteRepo";
import type { Item, Task } from "../types";

export interface Turn {
  role: "user" | "assistant";
  content: string;
}

/** How much of the day the advisor sees. */
const MAX_STORIES = 10;
const MAX_OPENINGS = 10;

const OPENING_KINDS = new Set(["job", "internship", "hackathon", "oss", "grant", "company"]);

function daysUntil(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const days = Math.ceil((d.getTime() - Date.now()) / 86_400_000);
  if (days < 0) return " (closed)";
  if (days === 0) return " (closes today)";
  if (days === 1) return " (closes tomorrow)";
  return ` (closes in ${days} days)`;
}

/**
 * Renders today into the digest the advisor reasons over.
 *
 * Badges are spelled out rather than passed as codes — "big and barely
 * covered" is the judgement the ranking already made, and the advisor should
 * be able to lean on it instead of re-deriving it from two numbers.
 */
export function buildContext(items: Item[], tasks: Task[], brief: string | null): string {
  const stories = items
    .filter((i) => !OPENING_KINDS.has(i.kind))
    .sort((a, b) => b.significance - a.significance)
    .slice(0, MAX_STORIES);

  const openings = items
    .filter((i) => OPENING_KINDS.has(i.kind))
    .sort((a, b) => b.significance - a.significance)
    .slice(0, MAX_OPENINGS);

  const open = tasks.filter((t) => t.status === "open");
  const done = tasks.filter((t) => t.status === "done").length;

  const label = (i: Item) =>
    i.badge === "legendary"
      ? " [big, and barely covered]"
      : i.badge === "worth-knowing"
        ? " [big, widely covered]"
        : "";

  const parts: string[] = [];

  if (brief) parts.push(`TODAY'S BRIEF:\n${brief}`);

  parts.push(
    stories.length
      ? `NEWS (most significant first):\n${stories
          .map((i) => `- ${i.title}${label(i)} — ${i.source}. ${i.whyLine ?? ""}`.trim())
          .join("\n")}`
      : "NEWS: nothing collected yet.",
  );

  parts.push(
    openings.length
      ? `OPENINGS:\n${openings
          .map(
            (i) =>
              `- [${i.kind}] ${i.title}${daysUntil(i.deadlineAt)} — ${i.source}. ${i.whyLine ?? ""}`.trim(),
          )
          .join("\n")}`
      : "OPENINGS: none found yet.",
  );

  parts.push(
    open.length
      ? `HIS OPEN TASKS:\n${open
          .map((t) => `- ${t.title}${t.dueAt ? ` (due ${t.dueAt.slice(0, 10)})` : ""}`)
          .join("\n")}\n(${done} finished recently)`
      : `HIS OPEN TASKS: none. (${done} finished recently)`,
  );

  return parts.join("\n\n");
}

/** Questions worth one tap, phrased the way he would actually ask. */
export const SUGGESTIONS = [
  "What should I spend this week on?",
  "Anything here worth skipping?",
  "Which opening is the best use of my time?",
  "What did I miss today?",
];

export async function askAdvisor(
  question: string,
  context: string,
  history: Turn[],
): Promise<string> {
  if (!isDesktop()) {
    throw new Error("The advisor needs the desktop app.");
  }
  return invoke<string>("ask_advisor", { question, context, history });
}
