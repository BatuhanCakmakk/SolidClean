import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { version } from "./package.json";

// Tauri geliştirme sunucusunu sabit portta bekler.
export default defineConfig({
  plugins: [react()],
  // "Hakkında" penceresindeki sürüm; MSIX sürümü de buradan (tauri.conf.json ile aynı) gelir.
  define: { __APP_VERSION__: JSON.stringify(version) },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: { target: "es2022" },
});
