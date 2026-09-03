import { useCallback, useEffect, useState } from "react";
import { Button, Chip, Empty, Section, cx } from "../components/ui";
import { openStory } from "../lib/open";
import OpeningSearch from "../components/OpeningSearch";
import type { Store } from "../lib/store";
import type { Item } from "../types";

const OPENING_KINDS = new Set(["job", "internship", "hackathon", "oss", "grant", "company"]);

/**
 * Three sections, never a wall.
 *
 * The lead is a reader rather than a single fixed card: he can page through
 * the ranked stories one at a time, which keeps the "one important thing"
 * framing while letting him read past it. The grid below doubles as the
 * picker — clicking a card jumps the reader to it.
 */
export default function NewsPage({ store, city }: { store: Store; city: string }) {
  const stories = store.items
    .filter((i) => !OPENING_KINDS.has(i.kind))
    .sort((a, b) => b.significance - a.significance);

  const [cursor, setCursor] = useState(0);

  // A refresh reorders everything, so holding position would land him on an
  // unrelated story. Going back to the top is the honest behaviour.
  useEffect(() => {
    setCursor(0);
  }, [store.items]);

  const total = stories.length;
  const clamped = Math.min(cursor, Math.max(0, total - 1));
  const current = stories[clamped] ?? null;

  const go = useCallback(
    (delta: number) => {
      setCursor((c) => {
        const next = c + delta;
        if (next < 0 || next >= total) return c;
        return next;
      });
    },
    [total],
  );

  // Arrow keys page the reader, unless he is typing somewhere.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement;
      if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return;
      if (e.key === "ArrowRight") go(1);
      if (e.key === "ArrowLeft") go(-1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go]);

  // Everything except the story being read, so the picker never shows a
  // duplicate of what is already on screen.
  const others = stories.filter((_, i) => i !== clamped).slice(0, 6);

  return (
    <div>
      <div className="mb-5 flex flex-wrap items-center gap-3">
        <h1 className="font-serif text-[26px] leading-none tracking-[-0.012em]">News</h1>
        <Button
          variant="primary"
          onClick={() => store.refresh()}
          className={store.refreshing ? "opacity-60" : undefined}
        >
          {store.refreshing ? "Reading…" : "Refresh"}
        </Button>
        <span className="ml-auto font-mono text-[11px] text-faint">
          {store.items.length === 0 ? "not fetched yet" : `${store.items.length} stories`}
        </span>
      </div>

      {store.lastRefresh && (
        <div
          className={cx(
            "mb-5 rounded-[12px] border px-4 py-3 font-mono text-[11px] leading-relaxed",
            store.lastRefresh.offline
              ? "border-line-2 text-muted"
              : store.lastRefresh.provisional
                ? "border-clay-deep text-clay-hot"
                : "border-line text-faint",
          )}
        >
          {store.lastRefresh.message}
          {store.lastRefresh.provisional && (
            <span className="mt-1 block text-muted">
              Ranking is rough until a working model key is set.
            </span>
          )}
          {store.lastRefresh.deadSources.length > 0 && (
            <span className="mt-1 block text-muted">
              Silent: {store.lastRefresh.deadSources.join(", ")}
            </span>
          )}
        </div>
      )}

      <Section label={clamped === 0 ? "The one" : `Story ${clamped + 1}`}>
        {current ? (
          <article
            className="relative overflow-hidden rounded-[18px] border border-clay-deep p-6"
            style={{
              background:
                "linear-gradient(120deg, rgba(217,119,87,.15), transparent 54%), var(--color-panel-2)",
            }}
          >
            <div className="flex flex-wrap items-center gap-2.5">
              <Chip tone={current.badge === "legendary" ? "clay" : undefined} pulse={clamped === 0}>
                {current.badge === "legendary"
                  ? "Legendary"
                  : clamped === 0
                    ? "Biggest today"
                    : current.badge === "worth-knowing"
                      ? "Worth knowing"
                      : "Under the radar"}
              </Chip>

              <div className="ml-auto flex items-center gap-2">
                <Button onClick={() => go(-1)} disabled={clamped === 0} aria-label="Previous story">
                  ←
                </Button>
                <span className="min-w-[64px] text-center font-mono text-[11px] tabular-nums text-faint">
                  {clamped + 1} of {total}
                </span>
                <Button
                  onClick={() => go(1)}
                  disabled={clamped >= total - 1}
                  aria-label="Next story"
                >
                  →
                </Button>
              </div>
            </div>

            <h2 className="mt-4 max-w-[22ch] font-serif text-[clamp(24px,3.3vw,34px)] leading-[1.14]">
              {current.title}
            </h2>

            {current.whyLine && (
              <p className="mt-3.5 max-w-[52ch] text-[15px] leading-relaxed text-muted">
                {current.whyLine}
              </p>
            )}

            <div className="mt-5 flex flex-wrap items-center gap-2.5">
              <Chip>{current.source}</Chip>
              <Chip>
                {current.corroborations === 1 ? "1 source" : `${current.corroborations} sources`}
              </Chip>
              <Button variant="primary" onClick={() => openStory(current.url)}>
                Read
              </Button>
              <Button onClick={() => store.addTask(current.title, current.deadlineAt, current.id)}>Add to to-do</Button>
            </div>
          </article>
        ) : (
          <Empty>
            Nothing fetched yet. This is where the single most important story of the day will sit.
          </Empty>
        )}
      </Section>

      <Section label="Worth knowing">
        {others.length === 0 ? (
          <Empty>The quieter second tier appears here — usually two or three items.</Empty>
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            {others.map((i) => (
              <StoryCard
                key={i.id}
                item={i}
                onOpen={() => setCursor(stories.findIndex((s) => s.id === i.id))}
              />
            ))}
          </div>
        )}
      </Section>

      <Section label="Openings">
        <OpeningSearch store={store} city={city} />
      </Section>
    </div>
  );
}

function StoryCard({ item, onOpen }: { item: Item; onOpen: () => void }) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="flex cursor-pointer flex-col gap-2.5 rounded-[12px] border border-line bg-panel-2 p-4 text-left transition-colors hover:border-line-2"
    >
      <h4 className="text-[14.5px] leading-snug font-semibold">{item.title}</h4>
      {item.whyLine && <p className="text-[13px] leading-relaxed text-muted">{item.whyLine}</p>}
      <span className="mt-auto font-mono text-[10px] text-faint">
        {item.corroborations === 1 ? "1 source" : `${item.corroborations} sources`} · {item.source}
      </span>
    </button>
  );
}




/** Days remaining reads better than a date when the point is urgency. */
