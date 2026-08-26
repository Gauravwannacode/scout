import { openUrl } from "@tauri-apps/plugin-opener";
import { isDesktop } from "./sqliteRepo";

/**
 * Opens a story in the real browser rather than inside the app window.
 *
 * Scout is a reading desk, not a browser — following a link should hand the
 * page to whatever he already has open, with his sessions and extensions,
 * rather than trapping it in a 1060px webview with no address bar.
 *
 * Falls back to `window.open` so the same call works in the dev server.
 */
export async function openStory(url: string | null | undefined): Promise<void> {
  if (!url) return;

  if (isDesktop()) {
    try {
      await openUrl(url);
      return;
    } catch (e) {
      console.error("could not open the story", e);
    }
  }
  window.open(url, "_blank", "noopener");
}

/** The bare host, for showing where a link actually goes. */
export function hostOf(url: string | null | undefined): string {
  if (!url) return "—";
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "—";
  }
}
