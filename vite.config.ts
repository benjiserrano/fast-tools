/// <reference types="vitest" />
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vitest/config";

// Tauri espera un puerto fijo y debe fallar en vez de elegir otro en silencio,
// porque la URL de desarrollo está fijada en tauri.conf.json.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // WebView2 es Chromium evergreen, así que la base puede ser moderna.
    target: "chrome120",
    sourcemap: false,
    minify: "esbuild",
  },
  test: {
    // Por defecto node, para los tests que leen el catálogo de Rust desde disco.
    // Los de componentes piden jsdom con `@vitest-environment jsdom`.
    environment: "node",
    restoreMocks: true,
  },
});
