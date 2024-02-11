import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        about: "./about.html",
        toast: "./toast.html",
        main: "./toolbox.html",
        config: "./config.html"
      }
    }
  },
  server: {
    port: 8124,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] }
  }
}));
