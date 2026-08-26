import { Button, Chip, Empty, cx } from "../components/ui";
import type { Store } from "../lib/store";
import { todaysTasks } from "../lib/store";
import type { Item } from "../types";
import { fmtClock, relativeTo, useNextAlarm, useNow } from "../lib/time";
import type { Page } from "../App";

/**
 * Home is the five-second glance: two squares on top (to-do, clock) and one
 * wide panel below carrying the single most important story.
 */
export default function HomePage({ store, go }: { store: Store; go(p: Page): void }) {
  const now = useNow(1000);
  const { time, ampm } = fmtClock(now);
  const tasks = todaysTasks(store.tasks).slice(0, 3);
  const done = store.tasks.filter((t) => t.status === "done").length;
  const nextAlarm = useNextAlarm(store.alarms, now);
  const enabled = store.alarms.filter((a) => a.enabled).length;
  const lead = pickLead(store.items);

  return (
    <div>
      <div className="mb-5 flex flex-wrap items-baseline gap-3">
        <h1 className="font-serif text-[26px] leading-none tracking-[-0.012em]">
          {now.toLocaleDateString(undefined, { weekday: "long" })}
        </h1>
        <span className="ml-auto font-mono text-[11px] text-faint">
          {time} · {store.tasks.length - done} open
        </span>
      </div>

      {/* two squares */}
      <div className="grid grid-cols-1 gap-3.5 md:grid-cols-2">
        <Square label="To-do" corner={`${done} of ${store.tasks.length} done`}>
          {tasks.length === 0 ? (
            <Empty>Nothing planned yet.</Empty>
          ) : (
            <div className="flex flex-col gap-2">
              {tasks.map((t, i) => (
                <button
                  key={t.id}
                  onClick={() => store.toggleTask(t.id)}
                  className={cx(
                    "flex cursor-pointer items-center gap-3 rounded-[12px] border px-3.5 py-3 text-left transition-colors",
                    i === 0 && t.status === "open"
                      ? "border-clay-deep bg-panel"
                      : "border-line bg-panel hover:border-line-2",
                  )}
                >
                  <span
                    className={cx(
                      "h-[15px] w-[15px] flex-none rounded-[5px] border-[1.5px]",
                      t.status === "done"
                        ? "border-faint bg-faint"
                        : i === 0
                          ? "border-clay"
                          : "border-line-2",
                    )}
                  />
                  <p
                    className={cx(
                      "text-[13.5px] leading-tight",
                      t.status === "done" && "text-faint line-through",
                    )}
                  >
                    {t.title}
                  </p>
                  <span
                    className={cx(
                      "ml-auto flex-none font-mono text-[10px]",
                      i === 0 && t.status === "open" ? "text-clay" : "text-faint",
                    )}
                  >
                    {t.status === "done" ? "done" : i === 0 ? "now" : ""}
                  </span>
                </button>
              ))}
            </div>
          )}
          <button
            onClick={() => go("todo")}
            className="mt-auto cursor-pointer self-start pt-3 font-mono text-[10px] tracking-[0.1em] text-faint uppercase transition-colors hover:text-cream"
          >
            All tasks →
          </button>
        </Square>

        <Square label="Clock" corner={`${enabled} ${enabled === 1 ? "alarm" : "alarms"} on`}>
          <div className="font-serif text-[clamp(44px,7vw,58px)] leading-none tracking-[-0.02em] tabular-nums">
            {time}
            <span className="ml-2 text-[0.36em] tracking-[0.06em] text-faint">{ampm}</span>
          </div>
          <div className="mt-1.5 font-mono text-[10.5px] tracking-[0.1em] text-muted uppercase">
            {now.toLocaleDateString(undefined, { day: "numeric", month: "long" })}
          </div>
          <div className="mt-auto flex items-center gap-2.5 border-t border-line pt-3.5">
            {nextAlarm ? (
              <>
                <em className="font-mono text-[11px] text-clay not-italic">
                  {relativeTo(nextAlarm.at, now)}
                </em>
                <p className="text-[13px] text-muted">{nextAlarm.alarm.label}</p>
              </>
            ) : (
              <p className="text-[13px] text-faint">No alarms set</p>
            )}
          </div>
        </Square>
      </div>

      {/* the wide panel */}
      {lead ? <LeadPanel item={lead} onAdd={() => store.addTask(lead.title, null)} /> : <LeadEmpty />}
    </div>
  );
}

function Square({
  label,
  corner,
  children,
}: {
  label: string;
  corner: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex min-h-[238px] flex-col rounded-[18px] border border-line bg-panel-2 p-5">
      <div className="mb-4 flex items-center gap-2.5">
        <span className="eyebrow">{label}</span>
        <b className="ml-auto font-mono text-[10px] font-normal text-faint">{corner}</b>
      </div>
      {children}
    </div>
  );
}

function LeadPanel({ item, onAdd }: { item: Item; onAdd(): void }) {
  return (
    <article
      className="relative mt-3.5 overflow-hidden rounded-[18px] border border-clay-deep p-6"
      style={{
        background:
          "linear-gradient(120deg, rgba(217,119,87,.15), transparent 54%), var(--color-panel-2)",
      }}
    >
      <div
        className="pointer-events-none absolute -top-25 -right-20 h-65 w-65 rounded-full"
        style={{ background: "radial-gradient(circle, rgba(217,119,87,.18), transparent 66%)" }}
      />
      <Chip tone="clay" pulse>
        {item.badge === "legendary" ? "Legendary" : "Most important today"}
      </Chip>
      <h2 className="relative z-1 mt-4 max-w-[22ch] font-serif text-[clamp(24px,3.3vw,34px)] leading-[1.14] tracking-[-0.012em]">
        {item.title}
      </h2>
      {item.whyLine && (
        <p className="relative z-1 mt-3.5 max-w-[52ch] text-[15px] leading-relaxed text-muted">
          {item.whyLine}
        </p>
      )}
      <div className="relative z-1 mt-5 flex flex-wrap items-center gap-2.5">
        <Chip>
          {item.source} · {item.corroborations === 1 ? "1 source" : `${item.corroborations} sources`}
        </Chip>
        <Button variant="primary" onClick={onAdd}>
          Add to to-do
        </Button>
      </div>
    </article>
  );
}

/**
 * Shown before the first fetch, and on days when nothing cleared the bar.
 * Deliberately calm: an empty panel is information, not a failure.
 */
function LeadEmpty() {
  return (
    <div className="mt-3.5 rounded-[18px] border border-dashed border-line-2 px-6 py-12 text-center">
      <p className="mx-auto max-w-[44ch] text-[14px] leading-relaxed text-faint">
        No story yet. Once the news pipeline is running, the single most important thing in AI and
        software lands here every morning — and stays empty on the days nothing earns it.
      </p>
    </div>
  );
}

/** Highest significance wins. Reach never demotes; it only decided the badge. */
function pickLead(items: Item[]): Item | null {
  if (items.length === 0) return null;
  return [...items].sort((a, b) => b.significance - a.significance)[0];
}
