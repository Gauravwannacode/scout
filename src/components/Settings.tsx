import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { Button, cx } from "./ui";
import { isDesktop } from "../lib/sqliteRepo";

/**
 * Mirrors the Rust `Settings` struct exactly.
 *
 * Snake_case on purpose: the struct has no `rename_all`, and these names are
 * also the keys in the on-disk settings.json. Renaming either side would
 * orphan the keys already saved there.
 */
interface AppSettings {
  groq_api_keys: string[];
  gemini_api_key: string;
  city: string;
}

/**
 * Settings live behind the footer rather than in the nav.
 *
 * Four pages was a deliberate constraint, and configuration is not something
 * opened daily — a fifth tab would cost a permanent slot for a rare visit.
 */
export default function Settings({ onClose }: { onClose: () => void }) {
  const [groq, setGroq] = useState("");
  const [gemini, setGemini] = useState("");
  const [city, setCity] = useState("");
  const [autostart, setAutostart] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    if (!isDesktop()) {
      setLoaded(true);
      return;
    }
    void (async () => {
      try {
        const s = await invoke<AppSettings>("get_settings");
        // Several Groq keys are rotated when one hits its daily cap, so the
        // field takes a list. One per line reads better than commas.
        setGroq((s.groq_api_keys ?? []).join("\n"));
        setGemini(s.gemini_api_key ?? "");
        setCity(s.city ?? "");
      } catch (e) {
        console.error("could not read settings", e);
      }
      try {
        setAutostart(await isEnabled());
      } catch {
        // Autostart is unavailable in some environments; the toggle simply
        // stays off rather than the panel failing to open.
      }
      setLoaded(true);
    })();
  }, []);

  async function save() {
    try {
      await invoke("save_settings", {
        value: {
          groq_api_keys: groq
            .split(/[\n,]/)
            .map((k) => k.trim())
            .filter(Boolean),
          gemini_api_key: gemini.trim(),
          city: city.trim(),
        },
      });
      setStatus("Saved.");
    } catch (e) {
      setStatus(`Could not save: ${e}`);
    }
  }

  async function toggleAutostart() {
    try {
      if (autostart) {
        await disable();
        setAutostart(false);
        setStatus("Scout will no longer start with Windows.");
      } else {
        await enable();
        setAutostart(true);
        setStatus("Scout will start with Windows, hidden in the tray.");
      }
    } catch (e) {
      setStatus(`Could not change startup: ${e}`);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-void/80 p-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="mt-10 flex w-full max-w-[520px] flex-col gap-5 rounded-[18px] border border-line-2 bg-panel p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-baseline gap-3">
          <h2 className="font-serif text-[24px] leading-none tracking-[-0.012em]">Settings</h2>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto cursor-pointer font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-cream"
          >
            Close
          </button>
        </div>

        {!loaded ? (
          <p className="font-mono text-[11px] text-faint">Reading…</p>
        ) : !isDesktop() ? (
          <p className="text-[13px] leading-relaxed text-muted">
            Settings are only available in the desktop app.
          </p>
        ) : (
          <>
            <Field
              label="Groq API keys"
              hint="One per line. Scout rotates to the next when a key hits its rate limit."
            >
              <textarea
                value={groq}
                onChange={(e) => setGroq(e.target.value)}
                rows={3}
                spellCheck={false}
                placeholder="gsk_…"
                className="w-full resize-y rounded-[10px] border border-line bg-panel-2 px-3 py-2 font-mono text-[12px] text-cream outline-none focus:border-line-2"
              />
            </Field>

            <Field
              label="Gemini API key"
              hint="Used only for the daily brief. Scoring works without it."
            >
              <input
                value={gemini}
                onChange={(e) => setGemini(e.target.value)}
                spellCheck={false}
                placeholder="AIza… or AQ.…"
                className="w-full rounded-[10px] border border-line bg-panel-2 px-3 py-2 font-mono text-[12px] text-cream outline-none focus:border-line-2"
              />
            </Field>

            <Field
              label="Your city"
              hint="Used to find offline hackathons near you. Devfolio and Unstop name a real city; Devpost lists everything as online."
            >
              <input
                value={city}
                onChange={(e) => setCity(e.target.value)}
                spellCheck={false}
                placeholder="Pune"
                className="w-full rounded-[10px] border border-line bg-panel-2 px-3 py-2 text-[13px] text-cream outline-none focus:border-line-2"
              />
            </Field>

            <div className="flex items-center gap-3 rounded-[12px] border border-line bg-panel-2 px-4 py-3">
              <div className="flex flex-col gap-0.5">
                <span className="text-[13.5px] font-semibold">Start with Windows</span>
                <span className="text-[12px] text-muted">
                  Opens hidden in the tray, so alarms work from boot.
                </span>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={autostart}
                aria-label="Start with Windows"
                onClick={() => void toggleAutostart()}
                className={cx(
                  "relative ml-auto h-[21px] w-[38px] flex-none cursor-pointer rounded-full border transition-colors",
                  autostart ? "border-clay bg-clay/15" : "border-line-2 bg-panel-3",
                )}
              >
                <span
                  className={cx(
                    "absolute top-[2px] block h-[15px] w-[15px] rounded-full transition-all",
                    autostart ? "left-[19px] bg-clay" : "left-[2px] bg-faint",
                  )}
                />
              </button>
            </div>

            <div className="flex items-center gap-3">
              <Button variant="primary" onClick={() => void save()}>
                Save keys
              </Button>
              {status && (
                <span className="font-mono text-[10.5px] leading-relaxed text-muted">{status}</span>
              )}
            </div>

            <p className="border-t border-line pt-4 font-mono text-[10px] leading-relaxed text-faint">
              Keys are stored in plain text at %APPDATA%\dev.gaurav.scout\settings.json. They never
              leave this machine except in requests to Groq and Google.
            </p>
          </>
        )}
      </div>
    </div>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="font-mono text-[9.5px] tracking-[0.17em] text-faint uppercase">{label}</span>
      {children}
      <span className="text-[11.5px] leading-relaxed text-muted">{hint}</span>
    </label>
  );
}
