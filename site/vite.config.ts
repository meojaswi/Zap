import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Vercel serves the site from the domain root. GitHub Pages serves it from /Zap/, so its
// workflow sets SITE_BASE and assets get the repo name as their prefix.
export default defineConfig({
  base: process.env.SITE_BASE ?? "/",
  plugins: [react(), tailwindcss()],
});
