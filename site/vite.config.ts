import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Served from https://meojaswi.github.io/Zap/, so assets need the repo name as their base.
export default defineConfig({
  base: "/Zap/",
  plugins: [react(), tailwindcss()],
});
