import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App";
import Mini from "./Mini";

/**
 * Both windows load the same bundle, so the entry point picks which root to
 * mount based on the Tauri window label. Reading it from the URL rather than
 * calling into the API keeps this synchronous — a flash of the full app
 * inside a 260px overlay would be very visible.
 */
function isMiniWindow(): boolean {
  const params = new URLSearchParams(window.location.search);
  if (params.get("window") === "mini") return true;
  return new URLSearchParams(window.location.hash.replace(/^#/, "")).get("window") === "mini";
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isMiniWindow() ? <Mini /> : <App />}</StrictMode>,
);
