import { useMemo, useState } from "react";
import { Button, Chip, Empty, cx } from "./ui";
import { hostOf, openStory } from "../lib/open";
import { matchesCity } from "../lib/city";
import type { Store } from "../lib/store";
import type { Item } from "../types";

/** Which slice of the openings is on screen. */
type Filter = "all" | "near" | "online" | "flagship" | "closing";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "flagship", label: "Big ones" },
  { id: "near", label: "Near me" },
  { id: "online", label: "Online" },
  { id: "closing", label: "Closing soon" },
];

function daysLeft(iso: string | null): number | null {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  return Math.ceil((d.getTime() - Date.now()) / 86_400_000);
}

function closingText(iso: string | null): string | null {
  const days = daysLeft(iso);
  if (days === null) return null;
  if (days < 0) return "closed";
  if (days === 0) return "closes today";
  if (days === 1) return "closes tomorrow";
  return `closes in ${days} days`;
}


/**
 * Search and filter across every opening Scout has collected.
 *
 * Scout's whole design is "show one thing, not everything" — but that is a
 * rule for the daily read, not for the moment he actively goes looking. This
 * is the deliberate exception: the full list, searchable, with a link out to
 * apply on each one.
 */
export default function OpeningSearch({ store, city }: { store: Store; city: string }) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");

  const openings = useMemo(
    () =>
      store.items
        .filter((i) =>
          ["job", "internship", "hackathon", "oss", "grant", "company"].includes(i.kind),
        )
        .sort((a, b) => b.significance - a.significance),
    [store.items],
  );

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();

    return openings.filter((i) => {
      if (q) {
        const hay =
          `${i.title} ${i.org ?? ""} ${i.summary ?? ""} ${i.location ?? ""} ${i.source}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      switch (filter) {
        case "flagship":
          return i.source === "flagship";
        case "near":
          return matchesCity(i.location, i.isOnline, `${i.title} ${i.summary ?? ""}`, city);
        case "online":
          return i.isOnline === true;
        case "closing": {
          const d = daysLeft(i.deadlineAt);
          return d !== null && d >= 0 && d <= 14;
        }
        default:
          return true;
      }
    });
  }, [openings, query, filter, city]);

  const nearCount = useMemo(
    () =>
      city
        ? openings.filter((i) =>
            matchesCity(i.location, i.isOnline, `${i.title} ${i.summary ?? ""}`, city),
          ).length
        : 0,
    [openings, city],
  );

  return (
    <div className="flex flex-col gap-3.5">
      <div className="flex flex-col gap-2.5">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search hackathons, internships, open source…"
          spellCheck={false}
          className="w-full rounded-[12px] border border-line bg-panel-2 px-4 py-2.5 text-[14px] text-cream outline-none focus:border-line-2"
        />

        <div className="flex flex-wrap gap-2">
          {FILTERS.map((f) => {
            const disabled = f.id === "near" && !city;
            return (
              <button
                key={f.id}
                type="button"
                disabled={disabled}
                onClick={() => setFilter(f.id)}
                title={
                  disabled ? "Set your city in Settings to use this" : undefined
                }
                className={cx(
                  "rounded-full border px-3.5 py-1.5 font-mono text-[10px] tracking-[0.1em] uppercase transition-colors",
                  disabled
                    ? "cursor-not-allowed border-line text-faint opacity-50"
                    : filter === f.id
                      ? "cursor-pointer border-clay bg-clay text-[#1a1210]"
                      : "cursor-pointer border-line-2 text-muted hover:border-cream hover:text-cream",
                )}
              >
                {f.label}
                {f.id === "near" && city ? ` · ${nearCount}` : ""}
              </button>
            );
          })}
        </div>
      </div>

      <p className="font-mono text-[10.5px] text-faint">
        {shown.length} of {openings.length}
        {filter === "near" && city ? ` in and around ${city}` : ""}
      </p>

      {shown.length === 0 ? (
        <Empty>
          {filter === "near" && city
            ? `Nothing offline near ${city} right now. Most listed hackathons are online — try that filter.`
            : query
              ? `Nothing matches "${query}".`
              : "Nothing here yet. Refresh on the News page to collect openings."}
        </Empty>
      ) : (
        <div className="flex flex-col gap-2.5">
          {shown.map((i) => (
            <OpeningRow key={i.id} item={i} store={store} />
          ))}
        </div>
      )}
    </div>
  );
}

function OpeningRow({ item, store }: { item: Item; store: Store }) {
  const closing = closingText(item.deadlineAt);
  const urgent = (daysLeft(item.deadlineAt) ?? 99) <= 3;
  const flagship = item.source === "flagship";

  return (
    <div
      className={cx(
        "flex flex-col gap-2 rounded-[12px] border bg-panel-2 px-4 py-3.5",
        flagship ? "border-clay-deep" : "border-line",
      )}
    >
      <div className="flex flex-wrap items-center gap-2.5">
        {flagship && <Chip tone="clay">Worth planning for</Chip>}
        <h4 className="text-[14.5px] leading-snug font-semibold">{item.title}</h4>
      </div>

      {item.summary && (
        <p className="max-w-[68ch] text-[13px] leading-relaxed text-muted">{item.summary}</p>
      )}

      <div className="flex flex-wrap items-center gap-2">
        {item.location && (
          <span className="font-mono text-[10px] text-muted">
            {item.isOnline === true ? "🌐" : "📍"} {item.location}
          </span>
        )}
        {closing && (
          <span
            className={cx("font-mono text-[10px]", urgent ? "text-clay" : "text-faint")}
          >
            {closing}
          </span>
        )}
        <span className="font-mono text-[10px] text-faint">{item.source}</span>

        <div className="ml-auto flex gap-2">
          <Button
            variant="primary"
            onClick={() => void openStory(item.url)}
            title={`Opens ${hostOf(item.url)} in your browser`}
          >
            Apply
          </Button>
          <Button onClick={() => store.addTask(item.title, item.deadlineAt, item.id)}>
            Save
          </Button>
        </div>
      </div>
    </div>
  );
}
