import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Porta fixa: o `tauri dev` espera o front nela. No navegador (backend
// simulado, `lib/mock.ts`) é a mesma página — o que muda é quem responde.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    // O WebView2 é Chromium: nada de transpilar para navegador velho.
    target: "chrome120",
    outDir: "dist",
    emptyOutDir: true,
  },
});
