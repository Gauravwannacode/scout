import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { Button, Chip, cx } from "./ui";
import { isDesktop } from "../lib/sqliteRepo";
import type { Item } from "../types";

/** Mirrors the Rust `Draft`. */
interface Draft {
  company: string;
  subject: string;
  body: string;
  template: string;
  researchNote: string;
  understanding: string;
  included: string[];
  dropped: string[];
  resumePath: string | null;
  researchFailed: boolean;
}

/**
 * Builds a cold email for one opening, and hands it to his mail client.
 *
 * Nothing is sent from here. The subject and body are editable before the
 * handoff, because the whole value of a cold email is that a person wrote it —
 * and because he is the one whose name is on it.
 */
export default function Outreach({ item, onClose }: { item: Item; onClose: () => void }) {
  const [draft, setDraft] = useState<Draft | null>(null);
  const [working, setWorking] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Editable copies, so his changes survive a re-read of the draft.
  const [to, setTo] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");

  const company = item.org ?? item.title;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    if (!isDesktop()) {
      setError("Drafting needs the desktop app.");
      setWorking(false);
      return;
    }
    let alive = true;
    void (async () => {
      try {
        const d = await invoke<Draft>("draft_outreach", {
          company,
          url: item.url,
          title: item.title,
          summary: item.summary,
        });
        if (!alive) return;
        setDraft(d);
        setSubject(d.subject);
        setBody(d.body);
      } catch (e) {
        if (alive) setError(String(e).replace(/^Error:\s*/, ""));
      } finally {
        if (alive) setWorking(false);
      }
    })();
    return () => {
      alive = false;
    };
  }, [company, item.url, item.title, item.summary]);

  async function handOff() {
    // mailto has a practical length ceiling in most clients; the body is well
    // under it, but encode properly regardless.
    const url =
      `mailto:${encodeURIComponent(to.trim())}` +
      `?subject=${encodeURIComponent(subject)}` +
      `&body=${encodeURIComponent(body)}`;
    try {
      await openUrl(url);
    } catch (e) {
      setError(`Could not open your mail client: ${e}`);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-void/80 p-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="mt-8 flex w-full max-w-[660px] flex-col gap-4 rounded-[18px] border border-line-2 bg-panel p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-baseline gap-3">
          <h2 className="font-serif text-[24px] leading-none tracking-[-0.012em]">Reach out</h2>
          <span className="truncate font-mono text-[10px] tracking-[0.1em] text-faint uppercase">
            {company}
          </span>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto cursor-pointer font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-cream"
          >
            Close
          </button>
        </div>

        {working && (
          <p className="font-mono text-[11px] text-faint">
            Reading their page, tailoring the resume, writing the email…
          </p>
        )}

        {error && (
          <p className="rounded-[12px] border border-clay-deep px-4 py-3 font-mono text-[11px] leading-relaxed text-clay-hot">
            {error}
          </p>
        )}

        {draft && (
          <>
            {/* What it understood, so a misreading is caught before sending. */}
            <div className="flex flex-col gap-2 rounded-[12px] border border-line bg-panel-2 px-4 py-3">
              <span className="font-mono text-[9.5px] tracking-[0.17em] text-faint uppercase">
                What it understood
              </span>
              <p className="text-[13px] leading-relaxed text-muted">{draft.understanding}</p>
              <p
                className={cx(
                  "font-mono text-[10px]",
                  draft.researchFailed ? "text-clay-hot" : "text-faint",
                )}
              >
                {draft.researchNote}
              </p>
            </div>

            {draft.dropped.length > 0 && (
              // Normally empty. When it is not, the model tried to invent
              // something and the fact base refused it.
              <p className="rounded-[12px] border border-clay-deep px-4 py-2.5 font-mono text-[10.5px] text-clay-hot">
                Dropped {draft.dropped.length} invented{" "}
                {draft.dropped.length === 1 ? "entry" : "entries"}: {draft.dropped.join(", ")}
              </p>
            )}

            <Field label="To">
              <input
                value={to}
                onChange={(e) => setTo(e.target.value)}
                spellCheck={false}
                placeholder="founder@company.com — find it on their site or HN profile"
                className="w-full rounded-[10px] border border-line bg-panel-2 px-3 py-2 text-[13px] text-cream outline-none focus:border-line-2"
              />
            </Field>

            <Field label="Subject">
              <input
                value={subject}
                onChange={(e) => setSubject(e.target.value)}
                className="w-full rounded-[10px] border border-line bg-panel-2 px-3 py-2 text-[13px] text-cream outline-none focus:border-line-2"
              />
            </Field>

            <Field label="Email">
              <textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                rows={9}
                className="w-full resize-y rounded-[10px] border border-line bg-panel-2 px-3 py-2.5 text-[13.5px] leading-relaxed text-cream outline-none focus:border-line-2"
              />
            </Field>

            <div className="flex flex-col gap-2 rounded-[12px] border border-line bg-panel-2 px-4 py-3">
              <div className="flex items-center gap-2.5">
                <Chip tone={draft.template === "bold" ? "clay" : undefined}>
                  {draft.template === "bold" ? "Bold template" : "ATS-safe template"}
                </Chip>
                <span className="font-mono text-[10px] text-faint">
                  {draft.included.length} facts selected
                </span>
                {draft.resumePath && (
                  <button
                    type="button"
                    onClick={() => void revealItemInDir(draft.resumePath!).catch(() => {})}
                    className="ml-auto cursor-pointer font-mono text-[10px] tracking-[0.1em] text-clay uppercase hover:text-clay-hot"
                  >
                    Show resume
                  </button>
                )}
              </div>
              <p className="text-[12px] leading-relaxed text-muted">
                {draft.included.slice(0, 6).join(" · ")}
              </p>
              {!draft.resumePath && (
                <p className="font-mono text-[10px] text-clay-hot">
                  No resume was attached — see the note above.
                </p>
              )}
            </div>

            <div className="flex flex-wrap items-center gap-2">
              <Button variant="primary" onClick={() => void handOff()} disabled={!to.trim()}>
                Open in mail
              </Button>
              <Button onClick={() => void navigator.clipboard.writeText(body)}>Copy email</Button>
              <span className="font-mono text-[10px] leading-relaxed text-faint">
                {draft.resumePath
                  ? "Attach the PDF yourself — mail clients cannot be handed a file this way."
                  : ""}
              </span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="font-mono text-[9.5px] tracking-[0.17em] text-faint uppercase">{label}</span>
      {children}
    </label>
  );
}
