import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [sveltekit()],
  server: { port: 6100 },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("@codemirror") || id.includes("@lezer")) {
            return "codemirror";
          }
          if (id.includes("@xyflow") || id.includes("elkjs")) {
            return "graph";
          }
        },
      },
    },
  },
});
