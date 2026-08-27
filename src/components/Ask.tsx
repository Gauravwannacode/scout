import { useEffect, useRef, useState } from "react";
import { Button, cx } from "./ui";
import { SUGGESTIONS, askAdvisor, buildContext } from "../lib/ask";
import type { Turn } from "../lib/ask";
import type { Store } from "../lib/store";

/**
 * The advisor panel.
 *
 * Grounded on what Scout collected today, which is the whole difference from
 * general chat: it can tell him the biggest story of the day is worth
 * skipping, because it knows what else is on his plate.
 */
export default function Ask({ store, onClose }: { store: Store; onClose: () => void }) {
  const [turns, setTurns] = useState<Turn[]>([]);
  const [draft, setDraft] = useState("");
  const [thinking, setThinking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [turns, thinking]);

  // Escape closes, matching every other overlay in the app.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  async function send(question: string) {
    const q = question.trim();
    if (!q || thinking) return;

    setError(null);
    setDraft("");
    // The history sent is what came *before* this question; the question
    // itself is passed separately.
    const history = turns;
    setTurns([...turns, { role: "user", content: q }]);
    setThinking(true);

    try {
      const answer = await askAdvisor(
        q,
        buildContext(store.items, store.tasks, store.brief),
        history,
      );
      setTurns((t) => [...t, { role: "assistant", content: answer }]);
    } catch (e) {
      // Never a silent failure — say what went wrong and leave the question
      // in view so it can be retried.
      setError(String(e).replace(/^Error:\s*/, ""));
    } finally {
      setThinking(false);
    }
  }

  const empty = turns.length === 0;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-void/80 p-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="mt-8 flex max-h-[82vh] w-full max-w-[620px] flex-col rounded-[18px] border border-line-2 bg-panel"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="flex items-baseline gap-3 border-b border-line px-6 py-4">
          <h2 className="font-serif text-[24px] leading-none tracking-[-0.012em]">Ask</h2>
          <span className="font-mono text-[10px] tracking-[0.1em] text-faint uppercase">
            {store.items.length > 0 ? `grounded on ${store.items.length} items` : "nothing fetched yet"}
          </span>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto cursor-pointer font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-cream"
          >
            Close
          </button>
        </header>

        <div className="flex-1 overflow-y-auto px-6 py-5">
          {empty ? (
            <div className="flex flex-col gap-4">
              <p className="max-w-[52ch] text-[14px] leading-relaxed text-muted">
                Answers come from what Scout actually collected today — the stories, the openings
                and your own to-do list. Not general web chat.
              </p>
              <div className="flex flex-col items-start gap-2">
                {SUGGESTIONS.map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => void send(s)}
                    className="cursor-pointer rounded-full border border-line px-3.5 py-1.5 text-left text-[13px] text-muted transition-colors hover:border-line-2 hover:text-cream"
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-4">
              {turns.map((t, i) => (
                <div
                  key={i}
                  className={cx(
                    "rounded-[12px] px-4 py-3 text-[14px] leading-relaxed",
                    t.role === "user"
                      ? "ml-auto max-w-[80%] border border-line bg-panel-2 text-cream"
                      : "border border-line border-l-[3px] border-l-clay bg-transparent text-cream",
                  )}
                >
                  {t.role === "assistant" && (
                    <span className="mb-1.5 block font-mono text-[9.5px] tracking-[0.16em] text-faint uppercase">
                      Scout
                    </span>
                  )}
                  <p className="whitespace-pre-wrap">{t.content}</p>
                </div>
              ))}
              {thinking && (
                <p className="font-mono text-[11px] text-faint">Reading today…</p>
              )}
            </div>
          )}

          {error && (
            <p className="mt-4 rounded-[12px] border border-clay-deep px-4 py-3 font-mono text-[11px] leading-relaxed text-clay-hot">
              {error}
            </p>
          )}
          <div ref={endRef} />
        </div>

        <footer className="flex items-end gap-2 border-t border-line px-6 py-4">
          <textarea
            ref={inputRef}
            value={draft}
            rows={1}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              // Enter sends; Shift+Enter is a newline, as in every chat box.
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void send(draft);
              }
            }}
            placeholder="What should I spend this week on?"
            className="max-h-[120px] min-h-[38px] flex-1 resize-none rounded-[10px] border border-line bg-panel-2 px-3 py-2.5 text-[13.5px] text-cream outline-none focus:border-line-2"
          />
          <Button
            variant="primary"
            onClick={() => void send(draft)}
            disabled={thinking || draft.trim().length === 0}
          >
            {thinking ? "…" : "Ask"}
          </Button>
        </footer>
      </div>
    </div>
  );
}
