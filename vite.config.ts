import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed port and should not have vite obscuring rust errors.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // The rust build directory churns constantly and locks its output DLL
      // while the app runs. Watching it crashes the dev server with EBUSY.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: { target: "chrome110", sourcemap: true },
});
